#!/usr/bin/env python3
"""Generate a deterministic CycloneDX SBOM for one CLROOM release archive."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import uuid

PRIVATE_MARKERS = ("/Users/", "/home/", "ghp_", "sk-", "AKIA")


def fail(message: str) -> "NoReturn":
    print(f"SBOM_INVALID: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular(path: Path) -> bool:
    return path.is_file() and not path.is_symlink()


def sha256(path: Path) -> str:
    if not regular(path):
        fail("artifact must be a regular file")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> dict:
    if not regular(path):
        fail("metadata must be a regular file")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        fail("metadata was not valid JSON")
    if not isinstance(value, dict):
        fail("metadata shape")
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


def reachable_package_ids(resolve: dict, root_id: str) -> set[str]:
    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        fail("metadata resolve nodes")
    dependencies: dict[str, list[str]] = {}
    for node in nodes:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            fail("metadata resolve node")
        values = node.get("dependencies")
        if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
            fail("metadata resolve dependencies")
        dependencies[node["id"]] = values
    if root_id not in dependencies:
        fail("metadata root node")
    seen: set[str] = set()
    pending = [root_id]
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        pending.extend(dependencies.get(package_id, []))
    return seen


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True)
    parser.add_argument("--metadata", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    artifact = Path(args.artifact).resolve()
    metadata_path = Path(args.metadata).resolve()
    output = Path(args.output).resolve()
    artifact_sha = sha256(artifact)
    metadata = load_json(metadata_path)

    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        fail("metadata packages/resolve")
    root_id = resolve.get("root")
    if not isinstance(root_id, str) or not root_id:
        fail("metadata root")

    by_id = {
        package.get("id"): package
        for package in packages
        if isinstance(package, dict) and isinstance(package.get("id"), str)
    }
    root_package = by_id.get(root_id)
    if not isinstance(root_package, dict) or root_package.get("name") != "clean-room-launcher":
        fail("unexpected root package")
    version = root_package.get("version")
    if not isinstance(version, str) or not artifact.name.startswith(f"clean-room-launcher-v{version}-"):
        fail("artifact/version mismatch")

    reachable = reachable_package_ids(resolve, root_id)
    if not reachable.issubset(by_id):
        fail("metadata dependency package missing")
    dependencies = [
        component(by_id[package_id])
        for package_id in reachable
        if package_id != root_id
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
