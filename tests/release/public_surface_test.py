#!/usr/bin/env python3
"""Focused contract for honest local public-security inputs."""

from __future__ import annotations

from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]


def body(relative: str) -> str:
    path = ROOT / relative
    if not path.is_file() or path.is_symlink():
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:MISSING:" + relative)
    return path.read_text(encoding="utf-8")


def main() -> int:
    security = body("SECURITY.md")
    security_contract = " ".join(security.split())
    for pattern in (
        r"Security\s*→\s*Report a vulnerability",
        r"private reporting button is unavailable,? do not put exploit details, credentials, private paths, prompts or user context in a public issue",
        r"Open a minimal public issue asking the maintainer to enable a private channel",
        r"does not offer a bounty",
    ):
        if not re.search(pattern, security_contract, re.IGNORECASE):
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:SECURITY:" + pattern)

    codeowners = body(".github/CODEOWNERS")
    if "Status: OWNER_MAPPING_PRESENT" not in codeowners:
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:CODEOWNERS_STATUS")
    if "Review enforcement: NOT_ENABLED" not in codeowners:
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:CODEOWNERS_ENFORCEMENT_STATUS")
    if "@ewgenij87snwork" not in codeowners:
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:CODEOWNERS_OWNER")
    active_codeowners = [line for line in codeowners.splitlines() if line.strip() and not line.lstrip().startswith("#")]
    if not active_codeowners:
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:CODEOWNERS_MAPPING")
    for protected in ("SECURITY.md", ".github/workflows/", ".github/CODEOWNERS", "scripts/gates/", "scripts/release/", "packaging/", "qualification/", "reports/gates/", "reports/release/"):
        if protected not in codeowners:
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:PROTECTED_PATH:" + protected)

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
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:THREAT_MODEL:" + pattern)

    limitations = " ".join(body("docs/limitations.md").split())
    for pattern in (
        r"`v0\.2\.0` is unsigned and unnotarized",
        r"unsigned.*unnotarized",
        r"Only macOS on Apple Silicon",
        r"Linux and Windows are `NOT_QUALIFIED`",
        r"not a VM, container, network sandbox or complete home-directory isolation",
        r"Explicit overrides can re-enable .* reduce the clean defaults",
        r"No bounty program exists",
    ):
        if not re.search(pattern, limitations, re.IGNORECASE):
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:LIMITATIONS:" + pattern)

    readiness = body("scripts/release/readiness.sh")
    for needle in ("CLROOM_RELEASE_VERSION", "CLROOM_ARTIFACT_QUALIFICATION=QUALIFIED", "RELEASE_READINESS_BLOCKED"):
        if needle not in readiness:
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:READINESS:" + needle)

    release_workflow = body(".github/workflows/release.yml")
    for needle in (
        "CLROOM_ARTIFACT_QUALIFICATION: QUALIFIED",
        "grep -qx 'qualification=QUALIFIED'",
    ):
        if needle not in release_workflow:
            raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:RELEASE_WORKFLOW:" + needle)

    readme = body("README.md")
    if "clean-room-launcher-hero.gif" in readme:
        raise SystemExit("CLROOM_RELEASE_PUBLIC_SURFACE_REFUSED:STALE_HERO_GIF")

    print("CLROOM_RELEASE_PUBLIC_SURFACE_PASS local_security_inputs=3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
