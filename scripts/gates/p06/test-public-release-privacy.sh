#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)
inventory="$root/qualification/public-release-inventory-v1.json"

exec python3 - "$root" "$inventory" <<'PY'
import json
from pathlib import Path
import re
import sys

root = Path(sys.argv[1]).resolve()
inventory_path = Path(sys.argv[2])

if not inventory_path.is_file() or inventory_path.is_symlink():
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INVENTORY_MISSING")

try:
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
except (OSError, UnicodeError, json.JSONDecodeError):
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INVENTORY_INVALID")

expected_keys = {
    "schema_version",
    "result",
    "public_paths",
    "conditional_public_paths",
    "excluded_internal_paths",
    "negative_fixture_paths",
    "generic_fixture_root",
    "history_policy",
}
if set(inventory) != expected_keys:
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INVENTORY_SHAPE")
if inventory["schema_version"] != "taskseal.p06.public-release-inventory.v1" or inventory["result"] != "PASS":
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INVENTORY_IDENTITY")
if inventory["history_policy"] != "IMMUTABLE_INTERNAL_EVIDENCE_EXCLUDED_FROM_PUBLIC_SOURCE":
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:HISTORY_POLICY")
if inventory["generic_fixture_root"] != "/workspace/taskseal":
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:GENERIC_FIXTURE_ROOT")
if inventory["excluded_internal_paths"] != [".taskseal-dev", "reports/gates", "scripts/gates"]:
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INTERNAL_BOUNDARY")
if inventory["negative_fixture_paths"] != ["tests/fixtures", "tests/release/audit_fixtures"]:
    raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:FIXTURE_BOUNDARY")

required = inventory["public_paths"]
conditional = inventory["conditional_public_paths"]
for label, values in (("PUBLIC_PATHS", required), ("CONDITIONAL_PATHS", conditional)):
    if not isinstance(values, list) or values != sorted(set(values)) or not all(isinstance(item, str) and item for item in values):
        raise SystemExit(f"P06_PUBLIC_RELEASE_PRIVACY_REFUSED:{label}")

for forbidden in inventory["excluded_internal_paths"]:
    if any(item == forbidden or item.startswith(forbidden + "/") for item in required + conditional):
        raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:INTERNAL_PATH_INCLUDED")

paths = []
for relative in required:
    path = root / relative
    if not path.exists():
        raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:REQUIRED_PATH_MISSING:" + relative)
    paths.append((relative, path))
for relative in conditional:
    path = root / relative
    if path.exists():
        paths.append((relative, path))

owner_markers = ("/Users/ysorokin/", "/home/ysorokin/", "5-LVL - 2026/Temp in Projects/wisdom/taskseal")
generic_home = re.compile(
    r"(?:/(?:Users|home)/[A-Za-z0-9._-]+(?:/|\b)|[A-Za-z]:\\Users\\[A-Za-z0-9._-]+(?:\\|\b))"
)
fixture_prefixes = tuple(inventory["negative_fixture_paths"])

files = []
for relative, path in paths:
    candidates = [path] if path.is_file() else sorted(path.rglob("*"))
    for candidate in candidates:
        if candidate.is_symlink():
            raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:SYMLINK:" + candidate.relative_to(root).as_posix())
        if candidate.is_file():
            files.append(candidate)

for path in files:
    relative = path.relative_to(root).as_posix()
    try:
        body = path.read_text(encoding="utf-8")
    except UnicodeError:
        continue
    if any(marker in body for marker in owner_markers):
        raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:OWNER_MARKER:" + relative)
    is_negative_fixture = any(relative == prefix or relative.startswith(prefix + "/") for prefix in fixture_prefixes)
    is_test_source = relative == "tests" or relative.startswith("tests/")
    if not is_negative_fixture and not is_test_source and generic_home.search(body):
        raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:ABSOLUTE_HOME_PATH:" + relative)

generic_fixture = root / "fixtures/adapters/generic"
for path in sorted(generic_fixture.rglob("*")):
    if not path.is_file() or path.is_symlink():
        continue
    try:
        body = path.read_text(encoding="utf-8")
    except UnicodeError:
        continue
    if generic_home.search(body) or any(marker in body for marker in owner_markers):
        raise SystemExit("P06_PUBLIC_RELEASE_PRIVACY_REFUSED:GENERIC_FIXTURE_PATH:" + path.relative_to(root).as_posix())

print(f"P06_PUBLIC_RELEASE_PRIVACY_PASS paths={len(paths)} files={len(files)}")
PY
