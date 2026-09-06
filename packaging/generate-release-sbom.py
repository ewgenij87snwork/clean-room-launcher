#!/usr/bin/env python3
"""Generate a minimal deterministic CycloneDX SBOM for one CLROOM release archive."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[1]
PRIVATE_MARKERS = ("/Users/", "/home/", "ghp_", "sk-", "AKIA")


def fail(message: str) -> "NoReturn":
    print(f"SBOM_INVALID: {message}", file=sys.stderr)
    raise SystemExit(1)


def sha256(path: Path) -> str:
    if not path.is_file() or path.is_symlink():
        fail("artifact must be a regular file")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def cargo_metadata() -> dict:
    env = dict(os.environ)
    env["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        fail("cargo metadata failed")
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError:
        fail("cargo metadata was not valid JSON")
    if not isinstance(value, dict):
        fail("cargo metadata shape")
    return value


def license_expression(package: dict) -> str:
    value = package.get("license")
    if not isinstance(value, str) or not value.strip():
        fail(f"missing license metadata for {package.get('name', '<unknown>')}")
    return value.strip()


def component(package: dict, *, application: bool = False, artifact_sha: str | None = None) -> dict:
    name = package.get("name")
    version = package.get("version")
    if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
        fail("package identity")
    item = {
        "type": "application" if application else "library",
        "name": name,
        "version": version,
        "bom-ref": f"pkg:cargo/{name}@{version}",
        "purl": f"pkg:cargo/{name}@{version}",
        "licenses": [{"expression": license_expression(package)}],
    }
    if artifact_sha is not None:
        item["hashes"] = [{"alg": "SHA-256", "content": artifact_sha}]
    return item


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    artifact = Path(args.artifact).resolve()
    output = Path(args.output).resolve()
    artifact_sha = sha256(artifact)
    metadata = cargo_metadata()

    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        fail("cargo metadata packages/resolve")
    root_id = resolve.get("root")
    if not isinstance(root_id, str) or not root_id:
        fail("cargo metadata root")

    by_id = {package.get("id"): package for package in packages if isinstance(package, dict)}
    root_package = by_id.get(root_id)
    if not isinstance(root_package, dict) or root_package.get("name") != "clean-room-launcher":
        fail("unexpected root package")

    dependencies = [
        component(package)
        for package_id, package in by_id.items()
        if package_id != root_id and isinstance(package, dict)
    ]
    dependencies.sort(key=lambda item: (item["name"], item["version"], item["bom-ref"]))

    serial = str(uuid.UUID(artifact_sha[:32]))
    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.7",
        "serialNumber": f"urn:uuid:{serial}",
        "version": 1,
        "metadata": {
            "component": component(root_package, application=True, artifact_sha=artifact_sha),
            "properties": [
                {"name": "clroom:artifact", "value": artifact.name},
                {"name": "clroom:artifact-sha256", "value": artifact_sha},
            ],
        },
        "components": dependencies,
    }

    encoded = json.dumps(sbom, sort_keys=True, separators=(",", ":")) + "\n"
    if any(marker in encoded for marker in PRIVATE_MARKERS):
        fail("private or secret-like data in SBOM")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(encoded, encoding="utf-8")
    output.chmod(0o644)
    print(f"SBOM_VALID sha256={hashlib.sha256(encoded.encode()).hexdigest()}")


if __name__ == "__main__":
    main()
