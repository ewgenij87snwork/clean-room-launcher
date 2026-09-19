#!/usr/bin/env python3
"""Fail-closed whole-release delta classifier for CLROOM."""
from __future__ import annotations

import argparse
import datetime as dt
import pathlib
import re
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]

DOMAIN_RULES = [
    ("runtime", re.compile(r"^src/")),
    ("tests", re.compile(r"^(tests/|fixtures/)")),
    ("dependencies", re.compile(r"^(Cargo\.toml|Cargo\.lock|deny\.toml)$")),
    ("ci", re.compile(r"^\.github/workflows/(?!release(?:-candidate)?\.yml$)")),
    ("release", re.compile(r"^(release/|scripts/release/|packaging/|install\.sh$|\.github/workflows/release(?:-candidate)?\.yml$)")),
    ("docs", re.compile(r"^(README\.md|CHANGELOG\.md|SECURITY\.md|CONTRIBUTING\.md|GOVERNANCE\.md|docs/)")),
]

RISK_RULES = [
    ("provider_activation", re.compile(r"^(src/adapters/claude/activation\.rs|src/catalog/provider_inventory\.rs|src/catalog/plugin_surface\.rs|src/cli/resource_options\.rs)")),
    ("process_sandbox", re.compile(r"^(src/adapters/.*/isolation\.rs|src/cli/process\.rs|src/cli/launch_contract\.rs|src/cli/mod\.rs)")),
    ("runtime_behavior", re.compile(r"^src/")),
    ("dependencies", re.compile(r"^(Cargo\.toml|Cargo\.lock|deny\.toml)$")),
    ("ci_pipeline", re.compile(r"^\.github/workflows/")),
    ("release_pipeline", re.compile(r"^(release/|scripts/release/|packaging/|install\.sh$|\.github/workflows/release(?:-candidate)?\.yml$)")),
    ("public_claims", re.compile(r"^(README\.md|CHANGELOG\.md|SECURITY\.md|docs/)")),
    ("verification", re.compile(r"^(tests/|fixtures/|scripts/release/)")),
]


def fail(code: str, detail: str = "") -> "None":
    suffix = f":{detail}" if detail else ""
    print(f"RELEASE_DELTA_BLOCKED:{code}{suffix}", file=sys.stderr)
    raise SystemExit(1)


def git(*args: str) -> str:
    proc = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail("GIT", " ".join(args))
    return proc.stdout.strip()


def classify(path: str, rules: list[tuple[str, re.Pattern[str]]]) -> set[str]:
    return {name for name, pattern in rules if pattern.search(path)}


def release_section(changelog: str, version: str) -> tuple[str, str]:
    pattern = re.compile(
        rf"^## \[{re.escape(version)}\] - (\d{{4}}-\d{{2}}-\d{{2}})$",
        re.MULTILINE,
    )
    match = pattern.search(changelog)
    if not match:
        fail("CHANGELOG_RELEASE_ENTRY", version)
    start = match.end()
    next_heading = re.search(r"^## \[", changelog[start:], re.MULTILINE)
    end = start + next_heading.start() if next_heading else len(changelog)
    return match.group(1), changelog[start:end]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="release/review.toml")
    parser.add_argument("--candidate", default="HEAD")
    parser.add_argument("--previous-tag")
    parser.add_argument("--release-date")
    args = parser.parse_args()

    config_path = ROOT / args.config
    if not config_path.is_file():
        fail("REVIEW_CONFIG_MISSING", args.config)
    with config_path.open("rb") as handle:
        config = tomllib.load(handle)
    if config.get("schema_version") != 1:
        fail("REVIEW_SCHEMA")

    version = config.get("version")
    previous = args.previous_tag or config.get("previous_published_release")
    if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail("VERSION")
    if not isinstance(previous, str) or not re.fullmatch(r"v\d+\.\d+\.\d+", previous):
        fail("PREVIOUS_RELEASE")

    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if manifest.get("package", {}).get("version") != version:
        fail("CARGO_VERSION", version)

    candidate = git("rev-parse", f"{args.candidate}^{{commit}}")
    previous_commit = git("rev-parse", f"{previous}^{{commit}}")
    if subprocess.run(
        ["git", "merge-base", "--is-ancestor", previous_commit, candidate],
        cwd=ROOT,
    ).returncode:
        fail("PREVIOUS_RELEASE_NOT_ANCESTOR", previous)

    changed = [
        line
        for line in git("diff", "--name-only", f"{previous_commit}..{candidate}").splitlines()
        if line
    ]
    if not changed:
        fail("EMPTY_DELTA")

    domains: set[str] = set()
    risks: set[str] = set()
    unknown: list[str] = []
    for path in changed:
        path_domains = classify(path, DOMAIN_RULES)
        if not path_domains:
            unknown.append(path)
        domains.update(path_domains)
        risks.update(classify(path, RISK_RULES))
    if unknown:
        fail("UNCLASSIFIED_PATH", ",".join(unknown))

    declared_domains = set(config.get("declared_domains", []))
    declared_risks = set(config.get("declared_risk_classes", []))
    if domains != declared_domains:
        fail(
            "DOMAIN_CONTRACT",
            f"computed={','.join(sorted(domains))};declared={','.join(sorted(declared_domains))}",
        )
    if risks != declared_risks:
        fail(
            "RISK_CONTRACT",
            f"computed={','.join(sorted(risks))};declared={','.join(sorted(declared_risks))}",
        )

    required = set(config.get("required_evidence", []))
    minimum = {
        "full_locked_tests",
        "dependency_sca",
        "public_boundary",
        "artifact_metadata",
        "sbom",
        "provenance",
        "draft_release",
    }
    if not minimum.issubset(required):
        fail("EVIDENCE_CONTRACT")
    if "provider_activation" in risks and "claude_2_1_273_plugin_activation" not in required:
        fail("PLUGIN_ACTIVATION_EVIDENCE_MISSING")
    if "process_sandbox" in risks and not {
        "codex_0_154_0_baseline",
        "claude_2_1_272_baseline",
    }.issubset(required):
        fail("BASELINE_PROVIDER_EVIDENCE_MISSING")

    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    date_text, section = release_section(changelog, version)
    try:
        release_date = dt.date.fromisoformat(date_text)
    except ValueError:
        fail("CHANGELOG_DATE")
    requested_date = args.release_date
    if requested_date:
        try:
            expected_date = dt.date.fromisoformat(requested_date)
        except ValueError:
            fail("EXPECTED_RELEASE_DATE")
        if release_date != expected_date:
            fail("RELEASE_DATE_MISMATCH", f"{release_date}!={expected_date}")

    if {"dependencies", "process_sandbox", "provider_activation"} & risks:
        if "### Security" not in section:
            fail("SECURITY_CHANGELOG_MISSING")
    if "### Compatibility" not in section:
        fail("COMPATIBILITY_CHANGELOG_MISSING")

    commit_count = int(git("rev-list", "--count", f"{previous_commit}..{candidate}"))
    print(
        "RELEASE_DELTA_PASS "
        f"version={version} previous={previous} candidate={candidate} "
        f"commits={commit_count} files={len(changed)} "
        f"domains={','.join(sorted(domains))} "
        f"risks={','.join(sorted(risks))}"
    )


if __name__ == "__main__":
    main()
