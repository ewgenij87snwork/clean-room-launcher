#!/usr/bin/env python3
"""Project the three historical P07 receipts into one closed V2 contract."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


SOURCE_PROFILES = {
    1: {
        "path": "reports/gates/p07/task-1.json",
        "commit": "fd43bb83074e1dd75b5d7f44d9973f790e746a80",
        "sha256": "3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06",
        "input_head": "ea551a35b058b19e402071cfc07d34862ec9216b",
        "implementation_head": "b08387b5bc060148ad0ffecbdb889f7f50fc2ba0",
        "selector": "correction_implementation_head",
    },
    2: {
        "path": "reports/gates/p07/task-2.json",
        "commit": "cee78ac90ae9a4dc3b07518089df26c8d64f68d1",
        "sha256": "3f2e59a113bcb38c6a53b14d8ee70c37823a29cdc2ef603d1b26b6da9f1d571a",
        "input_head": "fd43bb83074e1dd75b5d7f44d9973f790e746a80",
        "implementation_head": "aab71de37a6130600d60a57c6097340abec530ea",
        "selector": "implementation_head",
    },
    3: {
        "path": "reports/gates/p07/task-3.json",
        "commit": "0d5b7fbc9a079e8816bf4acfef6ee0e5b741a123",
        "sha256": "26d05b906e14c6c9aeaf24b392082578ac788689b4cac7a088b80da170e2caef",
        "input_head": "cee78ac90ae9a4dc3b07518089df26c8d64f68d1",
        "implementation_head": "01ad1d894aabe265b08d61d67d39da1a29cad9e4",
        "selector": "implementation_head",
    },
}
MANIFEST_KEYS = {"schema_version", "entries"}
ENTRY_KEYS = {"task", "path", "commit", "sha256", "implementation_selector"}
CANONICAL_KEYS = {
    "schema_version", "plan_id", "task", "source_receipt", "binding",
    "acceptance_id", "evidence_ids", "subjects", "qualification",
}


class Refused(Exception):
    pass


def strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise Refused(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=strict_pairs)
    except (UnicodeDecodeError, json.JSONDecodeError, Refused) as exc:
        raise Refused(f"invalid JSON {label}: {exc}") from exc
    if not isinstance(value, dict):
        raise Refused(f"JSON object required: {label}")
    return value


def git(root: Path, *args: str, check: bool = True) -> bytes:
    proc = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if check and proc.returncode:
        raise Refused(f"git failed: {' '.join(args)}")
    return proc.stdout


def tracked_regular(root: Path, rel: str) -> bytes:
    path = Path(rel)
    if path.is_absolute() or rel != path.as_posix() or ".." in path.parts:
        raise Refused(f"path alias: {rel}")
    full = root / path
    if not full.is_file() or full.is_symlink():
        raise Refused(f"non-regular path: {rel}")
    entries = git(root, "ls-files", "--stage", rel).decode().splitlines()
    if len(entries) != 1 or not (entries[0].startswith("100644 ") or entries[0].startswith("100755 ")):
        raise Refused(f"untracked or non-regular tracked path: {rel}")
    return full.read_bytes()


def verify_lineage(root: Path, profile: dict[str, str]) -> None:
    source = profile["commit"]
    implementation = profile["implementation_head"]
    for commit in (profile["input_head"], implementation, source):
        git(root, "cat-file", "-e", f"{commit}^{{commit}}")
    if subprocess.run(["git", "merge-base", "--is-ancestor", profile["input_head"], implementation], cwd=root).returncode:
        raise Refused("implementation is not descendant of input")
    if subprocess.run(["git", "merge-base", "--is-ancestor", implementation, source], cwd=root).returncode:
        raise Refused("receipt is not descendant of implementation")
    commits = git(root, "rev-list", "--reverse", f"{implementation}..{source}").decode().splitlines()
    for commit in commits:
        parents = git(root, "rev-list", "--parents", "-n", "1", commit).decode().split()
        if len(parents) != 2:
            raise Refused("merge in receipt lineage")
        changed = git(root, "diff-tree", "--no-commit-id", "--name-only", "-r", commit).decode().splitlines()
        if changed != [profile["path"]]:
            raise Refused("non-receipt change in receipt lineage")


def validate_manifest(root: Path) -> None:
    data = load_json(tracked_regular(root, "reports/gates/p07/scaffold-v2/source-manifest.json"), "manifest")
    if set(data) != MANIFEST_KEYS or data["schema_version"] != "taskseal.p07.scaffold-v2.source-manifest.v1":
        raise Refused("manifest shape")
    entries = data["entries"]
    if not isinstance(entries, list) or [e.get("task") for e in entries] != [1, 2, 3]:
        raise Refused("manifest tasks")
    seen_paths: set[str] = set()
    for task, entry in enumerate(entries, 1):
        if not isinstance(entry, dict) or set(entry) != ENTRY_KEYS:
            raise Refused("manifest entry shape")
        profile = SOURCE_PROFILES[task]
        if entry != {"task": task, "path": profile["path"], "commit": profile["commit"], "sha256": profile["sha256"], "implementation_selector": profile["selector"]}:
            raise Refused("manifest identity")
        if entry["path"] in seen_paths:
            raise Refused("duplicate manifest path")
        seen_paths.add(entry["path"])


def normalize(root: Path, task: int) -> dict[str, Any]:
    if task not in SOURCE_PROFILES:
        raise Refused("unknown task")
    validate_manifest(root)
    profile = SOURCE_PROFILES[task]
    source_bytes = git(root, "show", f"{profile['commit']}:{profile['path']}")
    if hashlib.sha256(source_bytes).hexdigest() != profile["sha256"]:
        raise Refused("source blob SHA mismatch")
    current = tracked_regular(root, profile["path"])
    if current != source_bytes or hashlib.sha256(current).hexdigest() != profile["sha256"]:
        raise Refused("current source receipt differs from Git blob")
    receipt = load_json(current, profile["path"])
    expected_top = {"schema_version", "plan_id", "task", "result", "acceptance", "binding", "evidence", "subjects", "qualification", "controls"}
    if task in (1, 3):
        expected_top.add("claims")
    if set(receipt) != expected_top or receipt.get("schema_version") != "taskseal.p07.task-receipt.v1" or receipt.get("task") != task or receipt.get("plan_id") != "P07-PACKAGING-SCAFFOLD-V1":
        raise Refused("source receipt shape")
    acceptance = receipt.get("acceptance")
    binding = receipt.get("binding")
    if not isinstance(acceptance, dict) or set(acceptance) != {"id", "operator_result", "evidence_ids"}:
        raise Refused("acceptance shape")
    expected_binding = {"input_head", "receipt_seal_role", "implementation_files"}
    if not isinstance(binding, dict):
        raise Refused("binding shape")
    if task == 1:
        expected_binding |= {"accepted_initial_head", "continuation_head", "correction_implementation_head", "receipt_commit_parent"}
        if binding.get("correction_implementation_head") != profile["implementation_head"]:
            raise Refused("wrong implementation selector")
    else:
        expected_binding.add("implementation_head")
    if set(binding) != expected_binding:
        raise Refused("binding shape")
    if binding["input_head"] != profile["input_head"] or binding.get("implementation_head", binding.get("correction_implementation_head")) != profile["implementation_head"]:
        raise Refused("binding identity")
    if task == 1 and binding["receipt_commit_parent"] != profile["implementation_head"]:
        raise Refused("receipt parent identity")
    if not isinstance(binding["implementation_files"], list) or len(set(binding["implementation_files"])) != len(binding["implementation_files"]):
        raise Refused("implementation files")
    evidence = receipt.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        raise Refused("evidence missing")
    if any(not isinstance(e, dict) or set(e) < {"id"} for e in evidence):
        raise Refused("malformed evidence entry")
    evidence_objects = evidence
    actual_ids = [e["id"] for e in evidence_objects]
    if len(actual_ids) != len(set(actual_ids)):
        raise Refused("duplicate evidence ID")
    acceptance_ids = acceptance["evidence_ids"]
    if not isinstance(acceptance_ids, list) or len(acceptance_ids) != len(set(acceptance_ids)) or not acceptance_ids or not set(acceptance_ids).issubset(actual_ids):
        raise Refused("acceptance evidence IDs")
    subjects = receipt.get("subjects")
    if not isinstance(subjects, dict) or len(subjects) != len(binding["implementation_files"]):
        raise Refused("subjects shape")
    for rel in binding["implementation_files"]:
        if rel not in subjects or hashlib.sha256(tracked_regular(root, rel)).hexdigest() != subjects[rel]:
            raise Refused(f"subject SHA mismatch: {rel}")
    if receipt.get("qualification") != "NOT_QUALIFIED":
        raise Refused("qualification boundary")
    verify_lineage(root, profile)
    result = {
        "schema_version": "taskseal.p07.consolidated-task-receipt.v2",
        "plan_id": "P07-PACKAGING-SCAFFOLD-V1",
        "task": task,
        "source_receipt": {"path": profile["path"], "commit": profile["commit"], "sha256": profile["sha256"]},
        "binding": {"input_head": profile["input_head"], "implementation_head": profile["implementation_head"], "first_receipt_commit": profile["commit"], "current_receipt_commit": profile["commit"], "implementation_files": binding["implementation_files"]},
        "acceptance_id": acceptance["id"],
        "evidence_ids": acceptance_ids,
        "subjects": subjects,
        "qualification": receipt["qualification"],
    }
    projection_path = f"reports/gates/p07/scaffold-v2/task-{task}.json"
    projection_bytes = tracked_regular(root, projection_path)
    if load_json(projection_bytes, projection_path) != result:
        raise Refused("canonical projection mismatch")
    if projection_bytes != (json.dumps(result, sort_keys=True, separators=(",", ":")) + "\n").encode():
        raise Refused("canonical projection bytes")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--task", type=int, required=True)
    args = parser.parse_args()
    value = normalize(args.root.resolve(), args.task)
    print(json.dumps(value, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Refused as exc:
        print(f"P07_SCAFFOLD_V2_NORMALIZER_REFUSED:{exc}", file=sys.stderr)
        raise SystemExit(1)
