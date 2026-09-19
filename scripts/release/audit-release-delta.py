#!/usr/bin/env python3
"""Fail-closed audit of the complete delta from the last published release."""

from __future__ import annotations

import argparse
import fnmatch
import json
import pathlib
import subprocess
import sys
from dataclasses import dataclass


class AuditError(RuntimeError):
    pass


def run_git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        raise AuditError(
            f"git {' '.join(args)} failed: {result.stderr.strip() or result.stdout.strip()}"
        )
    return result.stdout.strip()


def load_json(path: pathlib.Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise AuditError(f"invalid json {path}: {exc}") from exc


def matches(path: str, pattern: str) -> bool:
    return fnmatch.fnmatchcase(path, pattern)


@dataclass(frozen=True)
class ClassifiedFile:
    path: str
    classes: tuple[str, ...]


def classify(paths: list[str], contract: dict) -> list[ClassifiedFile]:
    rules = contract.get("rules")
    if not isinstance(rules, list) or not rules:
        raise AuditError("release contract has no rules")

    result: list[ClassifiedFile] = []
    for path in paths:
        classes = []
        for rule in rules:
            if any(matches(path, pattern) for pattern in rule.get("patterns", [])):
                classes.append(rule["id"])
        if not classes:
            raise AuditError(f"UNCLASSIFIED_RELEASE_DELTA:{path}")
        result.append(ClassifiedFile(path=path, classes=tuple(sorted(set(classes)))))
    return result


def required_gates(classified: list[ClassifiedFile], contract: dict) -> list[str]:
    gates = set(contract.get("always_required_gates", []))
    active = {item for entry in classified for item in entry.classes}
    for rule in contract["rules"]:
        if rule["id"] in active:
            gates.update(rule.get("required_gates", []))
    return sorted(gates)


def self_test() -> int:
    contract = {
        "always_required_gates": ["always"],
        "rules": [
            {"id": "runtime", "patterns": ["src/**"], "required_gates": ["tests"]},
            {"id": "docs", "patterns": ["README.md"], "required_gates": ["claims"]},
        ],
    }
    classified = classify(["src/main.rs", "README.md"], contract)
    assert {c for item in classified for c in item.classes} == {"runtime", "docs"}
    assert required_gates(classified, contract) == ["always", "claims", "tests"]
    try:
        classify(["brand-new-surface/file"], contract)
    except AuditError as exc:
        assert "UNCLASSIFIED_RELEASE_DELTA" in str(exc)
    else:
        raise AssertionError("unclassified file must fail closed")
    print("RELEASE_DELTA_AUDIT_SELF_TEST_PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract")
    parser.add_argument("--review")
    parser.add_argument("--base-ref")
    parser.add_argument("--head-ref", default="HEAD")
    parser.add_argument("--version")
    parser.add_argument("--output-json")
    parser.add_argument("--output-markdown")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    required = [
        args.contract,
        args.review,
        args.base_ref,
        args.version,
        args.output_json,
        args.output_markdown,
    ]
    if any(value is None for value in required):
        parser.error("all audit arguments are required unless --self-test is used")

    contract = load_json(pathlib.Path(args.contract))
    review = load_json(pathlib.Path(args.review))
    if contract.get("schema_version") != "clroom.release-contract.v1":
        raise AuditError("unsupported release contract schema")
    if review.get("schema_version") != "clroom.release-review.v1":
        raise AuditError("unsupported release review schema")
    if review.get("version") != args.version:
        raise AuditError("release review version mismatch")
    if review.get("baseline_release") != args.base_ref:
        raise AuditError("release review baseline mismatch")

    base = run_git("rev-parse", f"{args.base_ref}^{{commit}}")
    head = run_git("rev-parse", f"{args.head_ref}^{{commit}}")
    merge_base = run_git("merge-base", base, head)
    if merge_base != base:
        raise AuditError("published release baseline is not an ancestor of candidate")

    raw_paths = run_git("diff", "--name-only", f"{base}..{head}")
    paths = [line for line in raw_paths.splitlines() if line]
    if not paths:
        raise AuditError("release delta is empty")

    classified = classify(paths, contract)
    classes = sorted({item for entry in classified for item in entry.classes})
    gates = required_gates(classified, contract)

    expected_classes = sorted(review.get("reviewed_change_classes", []))
    expected_gates = sorted(review.get("reviewed_required_gates", []))
    if classes != expected_classes:
        raise AuditError(
            "release review change classes are stale: "
            + f"computed={classes} reviewed={expected_classes}"
        )
    if gates != expected_gates:
        raise AuditError(
            "release review required gates are stale: "
            + f"computed={gates} reviewed={expected_gates}"
        )
    if review.get("contract_review") not in {"confirmed", "expanded"}:
        raise AuditError("release contract review decision missing")
    if review.get("strategic_alignment_gate") != "required_before_tag":
        raise AuditError("strategic alignment gate must remain required before tag")

    commits_raw = run_git(
        "log",
        "--reverse",
        "--format=%H%x09%s",
        f"{base}..{head}",
    )
    commits = []
    for line in commits_raw.splitlines():
        if not line:
            continue
        sha, subject = line.split("\t", 1)
        commits.append({"sha": sha, "subject": subject})

    report = {
        "schema_version": "clroom.release-delta.v1",
        "version": args.version,
        "baseline_release": args.base_ref,
        "baseline_commit": base,
        "candidate_commit": head,
        "commit_count": len(commits),
        "file_count": len(paths),
        "change_classes": classes,
        "required_gates": gates,
        "files": [
            {"path": entry.path, "classes": list(entry.classes)}
            for entry in classified
        ],
        "commits": commits,
        "contract_review": review["contract_review"],
        "strategic_alignment_gate": review["strategic_alignment_gate"],
    }

    output_json = pathlib.Path(args.output_json)
    output_markdown = pathlib.Path(args.output_markdown)
    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_markdown.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(
        json.dumps(report, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )

    markdown = [
        "# Release delta audit",
        "",
        f"- Version: `{args.version}`",
        f"- Baseline: `{args.base_ref}` (`{base}`)",
        f"- Candidate: `{head}`",
        f"- Commits: {len(commits)}",
        f"- Files: {len(paths)}",
        f"- Contract review: `{review['contract_review']}`",
        "- Strategic alignment: **required before tag**",
        "",
        "## Change classes",
        "",
    ]
    markdown.extend(f"- `{item}`" for item in classes)
    markdown.extend(["", "## Required gates", ""])
    markdown.extend(f"- `{item}`" for item in gates)
    markdown.extend(["", "## Commits", ""])
    markdown.extend(f"- `{item['sha'][:12]}` {item['subject']}" for item in commits)
    markdown.extend(["", "## Files", ""])
    markdown.extend(
        f"- `{entry.path}` — {', '.join(f'`{c}`' for c in entry.classes)}"
        for entry in classified
    )
    output_markdown.write_text("\n".join(markdown) + "\n", encoding="utf-8")

    print(
        "RELEASE_DELTA_AUDIT_PASS "
        f"version={args.version} baseline={args.base_ref} "
        f"commits={len(commits)} files={len(paths)}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AuditError as exc:
        print(f"RELEASE_DELTA_AUDIT_BLOCKED:{exc}", file=sys.stderr)
        raise SystemExit(1)
