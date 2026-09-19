#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import json
import pathlib
import subprocess
import sys
import tomllib
from dataclasses import dataclass


class ContractError(RuntimeError):
    pass


@dataclass(frozen=True)
class FileChange:
    status: str
    path: str


def git(*args: str) -> str:
    proc = subprocess.run(
        ["git", *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if proc.returncode != 0:
        raise ContractError(proc.stderr.strip() or f"git {' '.join(args)} failed")
    return proc.stdout


def load_json(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ContractError(f"{path} must contain a JSON object")
    return value


def package_version() -> str:
    with open("Cargo.toml", "rb") as handle:
        value = tomllib.load(handle)
    return str(value["package"]["version"])


def changes(baseline: str, candidate: str) -> list[FileChange]:
    raw = git("diff", "--name-status", "--find-renames", f"{baseline}..{candidate}")
    result: list[FileChange] = []
    for line in raw.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        status = parts[0]
        if status.startswith("R"):
            path = parts[-1]
        elif len(parts) >= 2:
            path = parts[1]
        else:
            raise ContractError(f"unexpected git diff row: {line}")
        result.append(FileChange(status=status, path=path))
    return result


def matching_domain(path: str, contract: dict) -> str | None:
    for rule in contract.get("path_rules", []):
        if not isinstance(rule, dict):
            continue
        domain = rule.get("domain")
        for pattern in rule.get("patterns", []):
            if isinstance(pattern, str) and fnmatch.fnmatchcase(path, pattern):
                return str(domain)
    return None


def triggered_checks(paths: list[str], domains: set[str], contract: dict) -> set[str]:
    result: set[str] = set()
    domain_specs = contract.get("domains", {})
    for domain in domains:
        spec = domain_specs.get(domain)
        if isinstance(spec, dict) and isinstance(spec.get("trigger"), str):
            result.add(spec["trigger"])
    for trigger in contract.get("global_triggers", []):
        if not isinstance(trigger, dict) or not isinstance(trigger.get("id"), str):
            continue
        patterns = [p for p in trigger.get("patterns", []) if isinstance(p, str)]
        if any(any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns) for path in paths):
            result.add(trigger["id"])
    return result


def validate(baseline: str, candidate: str, contract_path: pathlib.Path, review_path: pathlib.Path) -> dict:
    contract = load_json(contract_path)
    review = load_json(review_path)

    if contract.get("schema_version") != 1:
        raise ContractError("unsupported technical release contract schema")
    if review.get("schema_version") != 1:
        raise ContractError("unsupported release review schema")

    version = package_version()
    if review.get("release_version") != version:
        raise ContractError(
            f"release review version {review.get('release_version')!r} != Cargo.toml {version!r}"
        )
    if review.get("baseline_release") != baseline:
        raise ContractError(
            f"release review baseline {review.get('baseline_release')!r} != published baseline {baseline!r}"
        )

    git("rev-parse", "--verify", f"{baseline}^{{commit}}")
    git("rev-parse", "--verify", f"{candidate}^{{commit}}")

    file_changes = changes(baseline, candidate)
    if not file_changes:
        raise ContractError("release delta is empty")

    classified: dict[str, str] = {}
    unknown: list[str] = []
    for change in file_changes:
        domain = matching_domain(change.path, contract)
        if domain is None:
            unknown.append(change.path)
        else:
            classified[change.path] = domain
    if unknown:
        raise ContractError(
            "UNCLASSIFIED_RELEASE_DELTA: " + ", ".join(sorted(unknown))
        )

    domains = set(classified.values())
    declared_domains = set(review.get("changed_domains", []))
    if domains != declared_domains:
        missing = sorted(domains - declared_domains)
        stale = sorted(declared_domains - domains)
        raise ContractError(
            f"release review domain mismatch missing={missing} stale={stale}"
        )

    triggers = triggered_checks(list(classified), domains, contract)
    allowed = set(contract.get("allowed_dispositions", []))
    assessments = review.get("assessments", [])
    if not isinstance(assessments, list):
        raise ContractError("release review assessments must be a list")

    by_trigger: dict[str, dict] = {}
    for item in assessments:
        if not isinstance(item, dict) or not isinstance(item.get("trigger"), str):
            raise ContractError("every assessment must have a trigger")
        trigger = item["trigger"]
        if trigger in by_trigger:
            raise ContractError(f"duplicate assessment for trigger {trigger}")
        disposition = item.get("disposition")
        if disposition not in allowed:
            raise ContractError(f"invalid disposition {disposition!r} for {trigger}")
        evidence = item.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            raise ContractError(f"assessment {trigger} requires non-empty evidence")
        for raw_path in evidence:
            if not isinstance(raw_path, str) or not pathlib.Path(raw_path).exists():
                raise ContractError(f"assessment {trigger} references missing evidence {raw_path!r}")
        if not isinstance(item.get("note"), str) or not item["note"].strip():
            raise ContractError(f"assessment {trigger} requires a note")
        by_trigger[trigger] = item

    missing_assessments = sorted(triggers - set(by_trigger))
    stale_assessments = sorted(set(by_trigger) - triggers)
    if missing_assessments or stale_assessments:
        raise ContractError(
            f"contract assessment mismatch missing={missing_assessments} stale={stale_assessments}"
        )

    followups = [
        trigger for trigger, item in by_trigger.items()
        if item.get("disposition") == "FOLLOW_UP"
    ]

    commits = [
        line for line in git("log", "--format=%h %s", f"{baseline}..{candidate}").splitlines()
        if line.strip()
    ]

    return {
        "schema_version": 1,
        "baseline": baseline,
        "candidate": git("rev-parse", candidate).strip(),
        "release_version": version,
        "commit_count": len(commits),
        "file_count": len(file_changes),
        "domains": sorted(domains),
        "triggers": sorted(triggers),
        "followups": sorted(followups),
        "changes": [
            {"status": change.status, "path": change.path, "domain": classified[change.path]}
            for change in file_changes
        ],
        "commits": commits,
    }


def render(report: dict) -> str:
    lines = [
        f"RELEASE_DELTA_PASS version={report['release_version']} baseline={report['baseline']} candidate={report['candidate']}",
        f"commits={report['commit_count']} files={report['file_count']}",
        "domains=" + ",".join(report["domains"]),
        "triggers=" + ",".join(report["triggers"]),
    ]
    if report["followups"]:
        lines.append("followups=" + ",".join(report["followups"]))
    lines.append("")
    lines.append("Changed files:")
    for item in report["changes"]:
        lines.append(f"  {item['status']:>3}  {item['domain']:<20} {item['path']}")
    lines.append("")
    lines.append("Commits:")
    for commit in report["commits"]:
        lines.append(f"  {commit}")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate whole-release delta against the public technical release contract.")
    parser.add_argument("--baseline", required=True, help="Last published stable release tag")
    parser.add_argument("--candidate", default="HEAD", help="Candidate git ref/SHA")
    parser.add_argument("--contract", default="release/technical-release-contract.json")
    parser.add_argument("--review", required=True, help="Per-release technical contract review JSON")
    parser.add_argument("--json-output", help="Optional JSON report path")
    args = parser.parse_args()

    try:
        report = validate(
            args.baseline,
            args.candidate,
            pathlib.Path(args.contract),
            pathlib.Path(args.review),
        )
    except ContractError as exc:
        print(f"RELEASE_DELTA_BLOCKED:{exc}", file=sys.stderr)
        return 1

    print(render(report))
    if args.json_output:
        pathlib.Path(args.json_output).write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
