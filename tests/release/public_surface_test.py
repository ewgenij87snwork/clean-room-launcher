#!/usr/bin/env python3
"""Focused contract for honest local public-security inputs."""

from __future__ import annotations

import json
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]


def body(relative: str) -> str:
    path = ROOT / relative
    if not path.is_file() or path.is_symlink():
        raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:MISSING:" + relative)
    return path.read_text(encoding="utf-8")


def main() -> int:
    security = body("SECURITY.md")
    for needle in (
        "Reporting status: `NOT_YET_AVAILABLE`",
        "private vulnerability report",
        "Do not open a public issue",
        "does not offer a bounty",
    ):
        if needle not in security:
            raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:SECURITY:" + needle)

    codeowners = body(".github/CODEOWNERS")
    if "Status: NOT_YET_ENFORCEABLE" not in codeowners:
        raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:CODEOWNERS_STATUS")
    for protected in ("SECURITY.md", ".github/workflows/", ".github/CODEOWNERS", "scripts/gates/", "packaging/"):
        if protected not in codeowners:
            raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:PROTECTED_PATH:" + protected)
    if any(line.strip() and not line.lstrip().startswith("#") for line in codeowners.splitlines()):
        raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:UNVERIFIED_OWNER_MAPPING")

    threat = body("docs/threat-model.md")
    for pattern in (
        r"protected assets",
        r"trust boundaries",
        r"attacker-controlled inputs",
        r"HOME contamination",
        r"path and symlink escape",
        r"malicious context",
        r"provider drift",
        r"private-data leakage",
        r"receipt forgery",
        r"Mitigations",
        r"Residual risks",
    ):
        if not re.search(pattern, threat, re.IGNORECASE):
            raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:THREAT_MODEL:" + pattern)

    limitations = body("docs/limitations.md")
    for needle in ("NOT_QUALIFIED", "DEFERRED_NOT_ADVERTISED", "croom", "not implemented", "not reserved"):
        if needle not in limitations:
            raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:LIMITATIONS:" + needle)

    namespace = json.loads(body("reports/release/namespace-ownership.json"))
    if namespace != {
        "schema_version": "taskseal.release-namespace-ownership.v1",
        "recorded_at": "2026-08-16T10:51:17Z",
        "public_product_name": "Clean Room Launcher",
        "category": "A clean-room launcher for coding-agent CLIs.",
        "intended_command": "croom",
        "approved_public_name": True,
        "repository_owned": False,
        "package_owned": False,
        "binary_names_owned": False,
        "croom_implemented": False,
        "queries": [
            {"registry": "crates.io", "name": "croom", "http_status": 404, "access": "read-only"},
            {"registry": "Homebrew Formulae", "name": "croom", "http_status": 404, "access": "read-only"},
            {"registry": "local PATH", "name": "croom", "status": "ABSENT", "access": "read-only"},
        ],
        "result": "NOT_OWNED_NOT_RESERVED",
    }:
        raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:NAMESPACE_RECORD")

    audit = body("scripts/release/audit-release.sh")
    for needle in ("PROTECTED_PATH_OWNERSHIP_UNVERIFIED", "NOT_YET_ENFORCEABLE", "NOT_YET_AVAILABLE"):
        if needle not in audit:
            raise SystemExit("P08_PUBLIC_SURFACE_REFUSED:AUDIT_BOUNDARY:" + needle)

    print("P08_PUBLIC_SURFACE_PASS local_security_inputs=4 external_ownership=NOT_QUALIFIED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
