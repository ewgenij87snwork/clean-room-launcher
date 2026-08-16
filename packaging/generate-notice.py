#!/usr/bin/env python3
"""Generate deterministic third-party notices from the exact locked Cargo graph."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "packaging/dependency-notice-policy.json"
LICENSE_FILE = re.compile(r"^(?:LICENSE|COPYING|NOTICE)", re.IGNORECASE)
HEX64 = re.compile(r"[0-9a-f]{64}\Z")


def refuse(code: str) -> None:
    raise SystemExit("P07_DEPENDENCY_NOTICE_REFUSED:" + code)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def strict_json(path: Path) -> dict:
    def unique(items):
        value = {}
        for key, item in items:
            if key in value:
                refuse("DUPLICATE_JSON_KEY")
            value[key] = item
        return value

    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    except (OSError, UnicodeError, json.JSONDecodeError):
        refuse("POLICY_JSON")
    if not isinstance(value, dict):
        refuse("POLICY_SHAPE")
    return value


def parse_output(values: list[str]) -> Path:
    if len(values) != 2 or values[0] != "--output":
        refuse("USAGE")
    output = Path(values[1]).resolve()
    if output.exists() and (output.is_symlink() or not output.is_file()):
        refuse("OUTPUT")
    return output


def metadata() -> dict:
    environment = dict(os.environ)
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode:
        refuse("CARGO_METADATA")
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError:
        refuse("CARGO_METADATA")
    if not isinstance(value, dict) or not isinstance(value.get("packages"), list):
        refuse("CARGO_METADATA")
    return value


def load_fallbacks() -> dict[str, dict]:
    policy = strict_json(POLICY_PATH)
    if set(policy) != {"schema_version", "canonical_fallbacks"} or policy["schema_version"] != "taskseal.p07.dependency-notice-policy.v1":
        refuse("POLICY_IDENTITY")
    fallbacks = policy["canonical_fallbacks"]
    if not isinstance(fallbacks, list):
        refuse("POLICY_SHAPE")
    result = {}
    expected_keys = {"package", "declared_expression", "selected_license", "source_path", "source_url", "sha256"}
    for item in fallbacks:
        if not isinstance(item, dict) or set(item) != expected_keys:
            refuse("FALLBACK_SHAPE")
        key = item["package"]
        if key in result or not isinstance(key, str):
            refuse("FALLBACK_DUPLICATE")
        if item["selected_license"] != "MIT" or item["source_url"] != "https://spdx.org/licenses/MIT.txt" or not HEX64.fullmatch(item["sha256"]):
            refuse("FALLBACK_SOURCE")
        source = ROOT / item["source_path"]
        if not source.is_file() or source.is_symlink() or digest(source.read_bytes()) != item["sha256"]:
            refuse("FALLBACK_DIGEST")
        result[key] = item
    if list(result) != sorted(result):
        refuse("FALLBACK_ORDER")
    return result


def normalized_text(path: Path) -> str:
    try:
        body = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        refuse("LICENSE_TEXT")
    return body.replace("\r\n", "\n").rstrip("\n") + "\n"


def generate(output: Path) -> int:
    graph = metadata()
    root_id = graph.get("resolve", {}).get("root")
    if not isinstance(root_id, str):
        refuse("ROOT_PACKAGE")
    packages = sorted(
        (package for package in graph["packages"] if package.get("id") != root_id),
        key=lambda package: (package.get("name", ""), package.get("version", "")),
    )
    if not packages:
        refuse("COMPONENTS")
    fallbacks = load_fallbacks()
    used_fallbacks = set()
    text_groups: dict[str, dict] = {}
    component_lines = []
    fallback_lines = []

    for package in packages:
        name = package.get("name")
        version = package.get("version")
        expression = package.get("license")
        manifest = Path(package.get("manifest_path", ""))
        if not all(isinstance(value, str) and value for value in (name, version, expression)) or not manifest.is_file():
            refuse("COMPONENT_METADATA")
        key = f"{name}@{version}"
        component_lines.append(f"- {name} {version} | {expression}")
        directory = manifest.parent
        license_paths = sorted(
            (path for path in directory.iterdir() if path.is_file() and not path.is_symlink() and LICENSE_FILE.match(path.name)),
            key=lambda path: (path.name.lower(), path.name),
        )
        sources: list[tuple[str, Path]] = [(f"{key}:{path.name}", path) for path in license_paths]
        if not sources:
            fallback = fallbacks.get(key)
            if fallback is None or fallback["declared_expression"] != expression or fallback["selected_license"] not in expression:
                refuse("MISSING_LICENSE_TEXT:" + key)
            source = ROOT / fallback["source_path"]
            sources = [(f"SPDX-{fallback['selected_license']}.txt", source)]
            used_fallbacks.add(key)
            fallback_lines.append(f"Canonical fallback for: {key} | {fallback['selected_license']} | {fallback['source_url']}")
        for label, path in sources:
            text = normalized_text(path)
            text_sha = digest(text.encode("utf-8"))
            group = text_groups.setdefault(text_sha, {"text": text, "sources": []})
            if group["text"] != text:
                refuse("LICENSE_DIGEST_COLLISION")
            group["sources"].append(label)

    if used_fallbacks != set(fallbacks):
        refuse("UNUSED_FALLBACK")
    lock_sha = digest((ROOT / "Cargo.lock").read_bytes())
    lines = [
        "Third-party dependency notices",
        "",
        f"Cargo.lock SHA-256: {lock_sha}",
        f"Component count: {len(packages)}",
        "Census command: cargo metadata --locked --offline --format-version 1",
        "License expressions are Cargo package SPDX declarations; packaged license texts follow.",
        "",
        *fallback_lines,
        "",
        "Component census",
        "",
        *component_lines,
        "",
        "License and notice texts",
        "",
    ]
    for text_sha in sorted(text_groups):
        group = text_groups[text_sha]
        sources = sorted(set(group["sources"]))
        lines.extend(
            [
                f"===== Text SHA-256: {text_sha} =====",
                "Sources: " + ", ".join(sources),
                "",
                group["text"].rstrip("\n"),
                "",
            ]
        )
    body = "\n".join(lines).rstrip("\n") + "\n"
    if any(marker in body for marker in ("/Users/", "/home/", "Documents/it", "ghp_", "sk-")):
        refuse("PRIVATE_DATA")
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".notice-", dir=output.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(body)
        os.chmod(temporary, 0o644)
        os.replace(temporary, output)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    print(f"P07_DEPENDENCY_NOTICE_PASS components={len(packages)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(generate(parse_output(sys.argv[1:])))
