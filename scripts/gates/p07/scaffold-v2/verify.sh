#!/bin/sh
set -eu

mode=production
if [ "${1:-}" = "--preflight" ]; then mode=preflight; shift; fi
if [ "$#" -ne 0 ]; then
  echo 'P07_EXACT_CURRENT_REFUSED:ARGUMENTS' >&2
  exit 1
fi

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd -P)
export P07_EXACT_CURRENT_ROOT="$ROOT"
export P07_EXACT_CURRENT_MODE="$mode"
python3 - <<'PY'
import hashlib, json, os, subprocess, sys
from pathlib import Path

root = Path(os.environ["P07_EXACT_CURRENT_ROOT"])
refused = "P07_EXACT_CURRENT_REFUSED:"
base = "e28e47309bde88582719a3e1b389667b8dfbc141"
correction_path = "reports/gates/p07/scaffold-v2/source-verifier-correction.json"
implementation_paths = [
  "scripts/gates/p07/scaffold-v2/test-verify.py",
  "scripts/gates/p07/scaffold-v2/verify.sh",
  "scripts/release-build/verify-source.sh",
  "tests/packaging/no_skip.rs",
]
red_command = (
  "sh -c 'CARGO_MANIFEST_DIR=\"$PWD\" rustc --test tests/packaging/no_skip.rs "
  "-o /tmp/p07-negative-no-skip && /tmp/p07-negative-no-skip; rust_exit=$?; "
  "python3 scripts/gates/p07/scaffold-v2/test-verify.py; gate_exit=$?; "
  "printf \"no_skip_exit=%s exact_current_exit=%s\\n\" \"$rust_exit\" \"$gate_exit\"; "
  "test \"$rust_exit\" -eq 0 && test \"$gate_exit\" -eq 0'"
)
green_command = (
  "sh -c 'test -x scripts/gates/p07/scaffold-v2/verify.sh; executable_exit=$?; "
  "CARGO_MANIFEST_DIR=\"$PWD\" rustc --test tests/packaging/no_skip.rs "
  "-o /tmp/p07-negative-no-skip-green && /tmp/p07-negative-no-skip-green; rust_exit=$?; "
  "python3 scripts/gates/p07/scaffold-v2/test-verify.py; gate_exit=$?; "
  "printf \"executable_exit=%s no_skip_exit=%s exact_current_exit=%s\\n\" "
  "\"$executable_exit\" \"$rust_exit\" \"$gate_exit\"; "
  "test \"$executable_exit\" -eq 0 && test \"$rust_exit\" -eq 0 && test \"$gate_exit\" -eq 0'"
)
profiles = {
  1: ("reports/gates/p07/task-1.json", "fd43bb83074e1dd75b5d7f44d9973f790e746a80", "3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06", "correction_implementation_head"),
  2: ("reports/gates/p07/task-2.json", "cee78ac90ae9a4dc3b07518089df26c8d64f68d1", "3f2e59a113bcb38c6a53b14d8ee70c37823a29cdc2ef603d1b26b6da9f1d571a", "implementation_head"),
  3: ("reports/gates/p07/task-3.json", "0d5b7fbc9a079e8816bf4acfef6ee0e5b741a123", "26d05b906e14c6c9aeaf24b392082578ac788689b4cac7a088b80da170e2caef", "implementation_head"),
}
command_names = [
  "workflow-no-skip", "workflow-orchestrator",
  "p02-gate", "p03-gate", "p04-gate", "p05-gate", "p06-gate", "p07-gate",
  "fmt", "clippy", "test", "schema", "golden", "parity", "privacy", "dependency", "license",
]

class Refused(Exception): pass

def strict_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result: raise Refused("JSON_DUPLICATE")
        result[key] = value
    return result

def git(*args):
    process = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True)
    if process.returncode: raise Refused("GIT")
    return process.stdout

def is_ancestor(older, newer):
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", older, newer],
        cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode == 0

def regular(rel):
    path = Path(rel)
    full = root / path
    if path.is_absolute() or path.as_posix() != rel or ".." in path.parts or full.is_symlink() or not full.is_file():
        raise Refused("PATH")
    rows = git("ls-files", "--stage", rel).splitlines()
    if len(rows) != 1 or not (rows[0].startswith("100644 ") or rows[0].startswith("100755 ")):
        raise Refused("PATH")
    return full.read_bytes()

def load(data, stage="JSON"):
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=strict_pairs)
    except (UnicodeDecodeError, json.JSONDecodeError, Refused) as exc:
        raise Refused(stage) from exc
    if not isinstance(value, dict): raise Refused(stage)
    return value

def run(stage, *args, **kwargs):
    process = subprocess.run(args, cwd=root, text=True, capture_output=True, **kwargs)
    if process.returncode: raise Refused(stage)
    return process

def validate_correction():
    receipt_bytes = regular(correction_path)
    receipt = load(receipt_bytes, "CORRECTION_RECEIPT_JSON")
    if set(receipt) != {
        "base_head", "evidence", "forbidden_actions", "implementation_head",
        "implementation_tree", "plan_id", "qualification", "schema_version", "subjects",
    }: raise Refused("CORRECTION_RECEIPT_SHAPE")
    if receipt["schema_version"] != "taskseal.p07.scaffold-negative-source-verifier-receipt.v1" or receipt["plan_id"] != "P07-SCAFFOLD-NEGATIVE-SOURCE-VERIFIER-V1" or receipt["base_head"] != base or receipt["qualification"] != "NOT_QUALIFIED":
        raise Refused("CORRECTION_RECEIPT_IDENTITY")
    expected_evidence = {
        "red": {"command": red_command, "no_skip_exit": 101, "exact_current_exit": 1},
        "green": {"command": green_command, "executable_exit": 0, "no_skip_exit": 0, "exact_current_exit": 0, "marker": "P07_EXACT_CURRENT_TEST_PASS cases=4"},
    }
    if receipt["evidence"] != expected_evidence: raise Refused("CORRECTION_RECEIPT_EVIDENCE")
    if receipt["forbidden_actions"] != {
        "external_contact": False, "integration": False, "main_mutation": False,
        "notarization": False, "provider_requests": False, "publication": False,
        "signing": False, "upload": False,
    }: raise Refused("CORRECTION_RECEIPT_BOUNDARY")

    implementation = receipt["implementation_head"]
    git("cat-file", "-e", f"{base}^{{commit}}")
    git("cat-file", "-e", f"{implementation}^{{commit}}")
    if git("rev-list", "--parents", "-n", "1", implementation).split() != [implementation, base]:
        raise Refused("CORRECTION_IMPLEMENTATION_PARENT")
    changed = sorted(git("diff-tree", "--no-commit-id", "--name-only", "-r", implementation).splitlines())
    if changed != implementation_paths: raise Refused("CORRECTION_IMPLEMENTATION_DIFF")
    if receipt["implementation_tree"] != git("rev-parse", f"{implementation}^{{tree}}").strip():
        raise Refused("CORRECTION_IMPLEMENTATION_TREE")
    subjects = receipt["subjects"]
    if not isinstance(subjects, dict) or sorted(subjects) != implementation_paths:
        raise Refused("CORRECTION_SUBJECTS")
    for rel in implementation_paths:
        blob = git("show", f"{implementation}:{rel}").encode()
        if regular(rel) != blob or subjects[rel] != hashlib.sha256(blob).hexdigest():
            raise Refused("CORRECTION_SUBJECTS")

    head = git("rev-parse", "HEAD").strip()
    candidates = git("rev-list", "--reverse", f"{implementation}..{head}", "--", correction_path).splitlines()
    if len(candidates) != 1: raise Refused("CORRECTION_RECEIPT_COMMIT")
    receipt_commit = candidates[0]
    if git("rev-list", "--parents", "-n", "1", receipt_commit).split() != [receipt_commit, implementation]:
        raise Refused("CORRECTION_RECEIPT_PARENT")
    if git("diff-tree", "--no-commit-id", "--name-only", "-r", receipt_commit).splitlines() != [correction_path]:
        raise Refused("CORRECTION_RECEIPT_DIFF")
    if git("show", f"{receipt_commit}:{correction_path}").encode() != receipt_bytes or not is_ancestor(receipt_commit, head):
        raise Refused("CORRECTION_RECEIPT_DURABILITY")
    return implementation, receipt_commit

def validate_source_result(process, subject):
    if process.returncode != 1: raise Refused("SOURCE_VERIFIER_EXIT")
    value = load(process.stdout.encode(), "SOURCE_VERIFIER_JSON")
    if set(value) != {
        "schema_version", "result", "subject_digest", "commands", "skips_counted_as_pass",
        "p06_qualification", "network_or_provider_spend",
    }: raise Refused("SOURCE_VERIFIER_SHAPE")
    if value != {**value,
        "schema_version": "taskseal.release-source-verification.v2",
        "result": "NOT_QUALIFIED",
        "subject_digest": subject,
        "skips_counted_as_pass": 0,
        "p06_qualification": "NOT_QUALIFIED",
        "network_or_provider_spend": False,
    }: raise Refused("SOURCE_VERIFIER_STATE")
    commands = value["commands"]
    if not isinstance(commands, list) or [item.get("name") if isinstance(item, dict) else None for item in commands] != command_names:
        raise Refused("SOURCE_VERIFIER_COMMANDS")
    for item in commands:
        if set(item) != {"name", "exit", "status", "subject_digest"} or type(item["exit"]) is not int or item["subject_digest"] != subject:
            raise Refused("SOURCE_VERIFIER_COMMANDS")
        if item["status"] not in ("PASS", "NOT_QUALIFIED") or ((item["exit"] == 0) != (item["status"] == "PASS")):
            raise Refused("SOURCE_VERIFIER_COMMANDS")
    by_name = {item["name"]: item for item in commands}
    if by_name["p06-gate"]["exit"] == 0 or by_name["p06-gate"]["status"] != "NOT_QUALIFIED":
        raise Refused("SOURCE_VERIFIER_P06")
    if by_name["p07-gate"] != {"name": "p07-gate", "exit": 0, "status": "PASS", "subject_digest": subject}:
        raise Refused("SOURCE_VERIFIER_P07")
    return value

try:
    if git("status", "--porcelain"): raise Refused("DIRTY")
    branch = git("branch", "--show-current").strip()
    if not branch or branch in ("main", "master"): raise Refused("DETACHED_OR_MAIN")
    implementation, correction_commit = validate_correction()

    manifest = load(regular("reports/gates/p07/scaffold-v2/source-manifest.json"), "MANIFEST_JSON")
    if set(manifest) != {"schema_version", "entries"} or manifest["schema_version"] != "taskseal.p07.scaffold-v2.source-manifest.v1": raise Refused("MANIFEST_SHAPE")
    entries = manifest["entries"]
    if not isinstance(entries, list) or len(entries) != 3: raise Refused("MANIFEST_ENTRIES")
    for task, entry in enumerate(entries, 1):
        path, commit, digest, selector = profiles[task]
        if entry != {"task": task, "path": path, "commit": commit, "sha256": digest, "implementation_selector": selector}: raise Refused("RECEIPT_DIGEST")
        blob = git("show", f"{commit}:{path}").encode()
        if hashlib.sha256(blob).hexdigest() != digest or regular(path) != blob: raise Refused("RECEIPT_DIGEST")
    for task in (1, 3):
        projection = regular(f"reports/gates/p07/scaffold-v2/task-{task}.json")
        process = run(f"PROJECTION_T{task}", sys.executable, "scripts/gates/p07/scaffold-v2/normalize.py", "--root", str(root), "--task", str(task))
        if process.stdout.encode() != projection: raise Refused("PROJECTION")
    task2_projection = regular("reports/gates/p07/scaffold-v2/task-2.json")
    if hashlib.sha256(task2_projection).hexdigest() != "4441ec2f4c727c9252be8d3825ac1aea1157d33e2cccefb5c819eb44925bf4b5" or git("show", "ed79e591c2b6d4b50076f4b9ae389c3375d2ea25:reports/gates/p07/scaffold-v2/task-2.json").encode() != task2_projection:
        raise Refused("PROJECTION_T2_FROZEN")

    compile_env = {**os.environ, "CARGO_MANIFEST_DIR": str(root)}
    for task, (source, output) in enumerate((("tests/packaging/target_matrix.rs", "/tmp/p07-exact-t1"), ("tests/packaging/no_skip.rs", "/tmp/p07-exact-t2"), ("tests/packaging/artifact_layout.rs", "/tmp/p07-exact-t3")), 1):
        run(f"FOCUSED_COMPILE_T{task}", "rustc", "--test", source, "-o", output, env=compile_env)
        run(f"FOCUSED_RUN_T{task}", output)
    verifier = os.environ.get("P07_EXACT_CURRENT_VERIFIER", "scripts/release-build/verify-source.sh")
    subject = git("rev-parse", "HEAD").strip()
    source_process = subprocess.run(
        [verifier, "--workflow", ".github/workflows/release-candidate.yml", "--subject-digest", subject, "--scaffold"],
        cwd=root, text=True, capture_output=True,
    )
    source_result = validate_source_result(source_process, subject)
    result = {
        "correction_receipt_commit": correction_commit,
        "implementation_head": implementation,
        "overall": "3/7",
        "p07": "3/8",
        "qualification": "NOT_QUALIFIED",
        "source_verification": source_result["result"],
        "tasks": [1, 2, 3],
    }
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    print("P07_EXACT_CURRENT_PREFLIGHT_PASS" if os.environ["P07_EXACT_CURRENT_MODE"] == "preflight" else "P07_PACKAGING_SCAFFOLD_EXACT_CURRENT_PASS")
except Refused as exc:
    print(refused + str(exc), file=sys.stderr)
    raise SystemExit(1)
PY
