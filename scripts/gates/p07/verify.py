#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path


def refuse(code: str) -> None:
    raise SystemExit("P07_SUPPLY_CHAIN_GATE_REFUSED:" + code)


def strict_json(path: Path) -> dict:
    def pairs(values):
        result = {}
        for key, value in values:
            if key in result: refuse("DUPLICATE_JSON_KEY")
            result[key] = value
        return result
    try: value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=pairs)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError): refuse("JSON_INVALID")
    if not isinstance(value, dict): refuse("JSON_SHAPE")
    return value


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(root: Path, *argv: str) -> str:
    env = dict(os.environ); env["CARGO_MANIFEST_DIR"] = str(root)
    result = subprocess.run(argv, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    if result.returncode: refuse("CHECK_FAILED_" + Path(argv[0]).name.upper().replace("-", "_"))
    return result.stdout


def git(root: Path, *argv: str) -> str:
    return command(root, "git", *argv).strip()


def committed_digest(root: Path, commit: str, relative: str) -> str:
    result = subprocess.run(["git", "show", f"{commit}:{relative}"], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    if result.returncode: refuse("SUBJECT_OBJECT_MISSING")
    return hashlib.sha256(result.stdout).hexdigest()


def main() -> int:
    root = Path(sys.argv[1]).resolve()
    if git(root, "rev-parse", "--show-toplevel") != str(root): refuse("WRONG_ROOT")
    branch = git(root, "branch", "--show-current")
    if not branch or branch == "main": refuse("MAIN_OR_DETACHED")
    if git(root, "status", "--porcelain", "--untracked-files=all"): refuse("DIRTY")
    head = git(root, "rev-parse", "HEAD")
    receipt_digests = {}
    forbidden = {"publication", "upload", "signing", "notarization", "provider_requests", "external_contact", "credential_access", "credentials", "keychain_access", "main_mutation", "integration", "network"}
    for task in range(1, 8):
        path = root / f"reports/gates/p07/task-{task}.json"
        receipt = strict_json(path)
        if receipt.get("task") != task or receipt.get("result") != "accepted" or receipt.get("qualification") != "NOT_QUALIFIED" or not str(receipt.get("plan_id", "")).startswith("P07"):
            refuse(f"TASK_{task}_IDENTITY")
        subjects = receipt.get("subjects")
        if not isinstance(subjects, dict) or not subjects: refuse(f"TASK_{task}_SUBJECTS")
        implementation = receipt.get("binding", {}).get("implementation_head")
        subject_commit = implementation or head
        for relative, expected in subjects.items():
            if committed_digest(root, subject_commit, relative) != expected: refuse(f"TASK_{task}_SUBJECT_DIGEST")
        controls = receipt.get("controls")
        if not isinstance(controls, dict): refuse(f"TASK_{task}_CONTROLS")
        if any(controls.get(key) is True for key in forbidden): refuse(f"TASK_{task}_FORBIDDEN_CLAIM")
        if implementation:
            exists = subprocess.run(["git", "cat-file", "-e", implementation + "^{commit}"], cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
            ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", implementation, head], cwd=root).returncode == 0
            if not exists or not ancestor: refuse(f"TASK_{task}_ANCESTRY")
        receipt_digests[str(task)] = digest(path)
    lifecycle_path = root / "reports/gates/p07/homebrew-v1/lifecycle-result.json"
    lifecycle = strict_json(lifecycle_path)
    expected_steps = ["preflight", "clone_local", "origin_removed", "tap_git_ready", "formula_syntax", "tap", "item_trust", "install_current", "uninstall_current", "untrust", "untap"]
    if lifecycle.get("schema_version") != "taskseal.p07.homebrew-lifecycle.v1" or lifecycle.get("evidence_class") != "real-current" or lifecycle.get("qualification") != "NOT_QUALIFIED": refuse("LIFECYCLE_IDENTITY")
    if lifecycle.get("failure_class") is not None or lifecycle.get("cleanup_complete") is not True: refuse("LIFECYCLE_RESULT")
    if lifecycle.get("network_boundary") != "homebrew-native-sandbox-loopback-proxy": refuse("LIFECYCLE_NETWORK_BOUNDARY")
    if lifecycle.get("checks") != {"dual_executable_parity": True, "poison_provider_absent": True, "selector_refusal": True, "status_paths": True}: refuse("LIFECYCLE_CHECKS")
    if any(lifecycle.get("forbidden_actions", {}).values()): refuse("LIFECYCLE_FORBIDDEN_ACTION")
    steps = lifecycle.get("steps")
    if not isinstance(steps, list) or [step.get("name") for step in steps] != expected_steps or any(step.get("exit") != 0 for step in steps): refuse("LIFECYCLE_STEPS")
    checks = [
        ("target_matrix", "tests/packaging/target_matrix.rs"),
        ("no_skip", "tests/packaging/no_skip.rs"),
        ("artifact_layout", "tests/packaging/artifact_layout.rs"),
        ("homebrew", "tests/packaging/homebrew.rs"),
        ("cargo_install", "tests/packaging/cargo_install.rs"),
        ("supply_chain", "tests/packaging/supply_chain.rs"),
        ("signing", "tests/packaging/signing.rs"),
    ]
    evidence = []
    for label, source in checks:
        binary = f"/tmp/p07-final-{label}-{os.getpid()}"
        command(root, "rustc", "--test", source, "-o", binary)
        output = command(root, binary)
        evidence.append({"check": label, "output_sha256": hashlib.sha256(output.encode()).hexdigest()})
        Path(binary).unlink(missing_ok=True)
    command(root, "python3", "tests/packaging/homebrew_contract_test.py", "--section", "input")
    command(root, "python3", "tests/packaging/homebrew_contract_test.py", "--section", "formula")
    result = {"schema_version": "taskseal.p07.supply-chain-gate.v1", "marker": "P07_SUPPLY_CHAIN_GATE_PASS", "head": head, "qualification": "NOT_QUALIFIED", "progress": {"overall": "3/7", "p07": "8/8"}, "receipts": receipt_digests, "lifecycle_result_sha256": digest(lifecycle_path), "checks": evidence, "forbidden_actions": {key: False for key in sorted(forbidden)}}
    output = root / "reports/gates/p07/supply-chain-gate.json"
    output.write_text(json.dumps(result, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    print("P07_SUPPLY_CHAIN_GATE_PASS qualification=NOT_QUALIFIED p07=8/8 overall=3/7")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
