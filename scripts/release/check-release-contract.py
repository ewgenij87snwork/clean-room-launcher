#!/usr/bin/env python3
"""Fail-closed release contract checker for the full published-release delta."""

from __future__ import annotations

import argparse
import fnmatch
import os
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "release" / "contract.toml"


def fail(code: str, detail: str = "") -> "NoReturn":
    suffix = f":{detail}" if detail else ""
    raise SystemExit(f"RELEASE_CONTRACT_BLOCKED:{code}{suffix}")


def run(*args: str) -> str:
    try:
        return subprocess.check_output(args, cwd=ROOT, text=True, stderr=subprocess.STDOUT).strip()
    except subprocess.CalledProcessError as exc:
        fail("COMMAND", f"{' '.join(args)} => {exc.output.strip()}")


def published_tag_from_github() -> str:
    explicit = os.environ.get("CLROOM_LATEST_PUBLISHED_TAG", "").strip()
    if explicit:
        return explicit
    repo = os.environ.get("GITHUB_REPOSITORY", "y-sor/clean-room-launcher")
    try:
        value = subprocess.check_output(
            ["gh", "api", f"repos/{repo}/releases/latest", "--jq", ".tag_name"],
            cwd=ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        fail("LATEST_PUBLISHED_RELEASE_UNKNOWN", "set CLROOM_LATEST_PUBLISHED_TAG or pass --published-tag")
    if not value:
        fail("LATEST_PUBLISHED_RELEASE_UNKNOWN")
    return value


def load_toml(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        fail("TOML", f"{path.relative_to(ROOT)}:{exc}")


def changelog_section(version: str) -> tuple[str, str]:
    lines = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8").splitlines()
    prefix = f"## [{version}] - "
    start = None
    date = None
    body: list[str] = []
    for index, line in enumerate(lines):
        if line.startswith(prefix):
            start = index + 1
            date = line[len(prefix):]
            continue
        if start is not None and line.startswith("## ["):
            break
        if start is not None:
            body.append(line)
    if start is None or date is None:
        fail("CHANGELOG_SECTION", version)
    return date, "\n".join(body)


def classify(paths: list[str], contract: dict) -> tuple[set[str], list[str]]:
    classes: set[str] = set()
    unclassified: list[str] = []
    definitions = contract.get("change_classes", {})
    for path in paths:
        matched = False
        for name, definition in definitions.items():
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in definition.get("patterns", [])):
                classes.add(name)
                matched = True
        if not matched:
            unclassified.append(path)
    return classes, unclassified


def required_evidence(contract: dict, classes: set[str], capabilities: set[str]) -> tuple[set[str], set[str]]:
    pre = set(contract.get("required_pre_tag_evidence", []))
    post = set(contract.get("required_post_tag_evidence", []))
    for name in classes:
        definition = contract["change_classes"][name]
        pre.update(definition.get("pre_tag_evidence", []))
        post.update(definition.get("post_tag_evidence", []))
    capability_defs = contract.get("capabilities", {})
    for name in capabilities:
        if name not in capability_defs:
            fail("UNKNOWN_CAPABILITY", name)
        pre.update(capability_defs[name].get("pre_tag_evidence", []))
        post.update(capability_defs[name].get("post_tag_evidence", []))
    return pre, post


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--published-tag")
    args = parser.parse_args()

    contract = load_toml(CONTRACT)
    if contract.get("schema_version") != 1:
        fail("CONTRACT_SCHEMA")

    review_path = ROOT / "release" / "reviews" / f"v{args.version}.toml"
    review = load_toml(review_path)
    if review.get("schema_version") != 1 or review.get("version") != args.version:
        fail("REVIEW_SCHEMA_OR_VERSION")

    published_tag = (args.published_tag or published_tag_from_github()).strip()
    if review.get("base_tag") != published_tag:
        fail("PUBLISHED_BASE_MISMATCH", f"review={review.get('base_tag')} published={published_tag}")

    run("git", "rev-parse", "--verify", f"{published_tag}^{{commit}}")
    head = run("git", "rev-parse", args.head)
    base = run("git", "rev-parse", f"{published_tag}^{{commit}}")
    try:
        subprocess.check_call(
            ["git", "merge-base", "--is-ancestor", base, head],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        fail("PUBLISHED_BASE_NOT_ANCESTOR")

    manifest = load_toml(ROOT / "Cargo.toml")
    if manifest.get("package", {}).get("version") != args.version:
        fail("CARGO_VERSION")

    release_date, changelog = changelog_section(args.version)
    if review.get("release_date") != release_date:
        fail("RELEASE_DATE_MISMATCH", f"review={review.get('release_date')} changelog={release_date}")

    for section in contract.get("required_changelog_sections", []):
        if f"### {section}" not in changelog:
            fail("CHANGELOG_SECTION_REQUIRED", section)

    names = run("git", "diff", "--name-only", f"{published_tag}...{head}").splitlines()
    names = [name for name in names if name]
    if not names:
        fail("EMPTY_PUBLISHED_DELTA")

    computed_classes, unclassified = classify(names, contract)
    if unclassified:
        fail("UNCLASSIFIED_PATHS", ",".join(unclassified))

    declared_classes = set(review.get("change_classes", []))
    if computed_classes != declared_classes:
        missing = sorted(computed_classes - declared_classes)
        stale = sorted(declared_classes - computed_classes)
        fail("CHANGE_CLASS_MISMATCH", f"missing={missing};stale={stale}")

    capabilities = set(review.get("capabilities", []))
    if "runtime" in computed_classes and not capabilities:
        fail("RUNTIME_CAPABILITY_DECLARATION_REQUIRED")

    dispositions = review.get("contract_evolution", [])
    allowed = ("EXTEND_CONTRACT:", "ALREADY_COVERED:", "DEFER_REMOVE:", "OWNER_EXCEPTION:")
    if not dispositions or any(not item.startswith(allowed) for item in dispositions):
        fail("CONTRACT_EVOLUTION_DISPOSITION")

    required_pre, required_post = required_evidence(contract, computed_classes, capabilities)
    declared_pre = set(review.get("pre_tag_evidence", []))
    declared_post = set(review.get("post_tag_required_evidence", []))
    missing_pre = sorted(required_pre - declared_pre)
    missing_post = sorted(required_post - declared_post)
    if missing_pre:
        fail("PRE_TAG_EVIDENCE_MISSING", ",".join(missing_pre))
    if missing_post:
        fail("POST_TAG_EVIDENCE_MISSING", ",".join(missing_post))

    commits = run("git", "rev-list", "--count", f"{published_tag}..{head}")
    print(f"RELEASE_DELTA_BASE={published_tag}")
    print(f"RELEASE_DELTA_BASE_SHA={base}")
    print(f"RELEASE_DELTA_HEAD_SHA={head}")
    print(f"RELEASE_DELTA_COMMITS={commits}")
    print(f"RELEASE_DELTA_FILES={len(names)}")
    print("RELEASE_CHANGE_CLASSES=" + ",".join(sorted(computed_classes)))
    print("RELEASE_CAPABILITIES=" + ",".join(sorted(capabilities)))
    print("RELEASE_PRE_TAG_EVIDENCE=" + ",".join(sorted(declared_pre)))
    print("RELEASE_POST_TAG_REQUIRED_EVIDENCE=" + ",".join(sorted(declared_post)))
    print("RELEASE_CONTRACT_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
