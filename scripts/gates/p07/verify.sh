#!/bin/sh
set -eu

root=${P07_GATE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd -P)}
cd "$(CDPATH= cd -- "$root" && pwd -P)"

# This is the Tasks 1–3 scaffold gate.  It deliberately does not inspect or
# execute any later packaging/signing/release task.
python3 - "$root" <<'PY'
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
if subprocess.check_output(["git", "rev-parse", "--show-toplevel"], cwd=root, text=True).strip() != str(root):
    raise SystemExit("P07_GATE_WRONG_ROOT")
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()

def git(*args, check=True):
    p = subprocess.run(["git", *args], cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if check and p.returncode:
        raise SystemExit(f"P07_GATE_GIT_FAILURE {' '.join(args)}: {p.stderr.strip()}")
    return p

def is_commit(oid):
    return len(oid) == 40 and all(c in "0123456789abcdef" for c in oid) and git("cat-file", "-e", oid + "^{commit}", check=False).returncode == 0

def ancestor(old, new=head):
    return git("merge-base", "--is-ancestor", old, new, check=False).returncode == 0

def tree_paths(commit):
    out = git("diff-tree", "--root", "--no-commit-id", "--name-only", "-r", commit).stdout.splitlines()
    return out

def first_receipt_commit(implementation, receipt):
    commits = git("rev-list", "--reverse", "--ancestry-path", f"{implementation}..{head}", "--", receipt).stdout.splitlines()
    if not commits:
        raise ValueError(f"no receipt descendant for {receipt}")
    return commits[0]

def sha256_file(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def require_hex(value, length, label):
    if not isinstance(value, str) or len(value) != length or any(c not in "0123456789abcdef" for c in value):
        raise ValueError(f"{label} must be lowercase {length}-hex")

required_top = {
    1: {"schema_version", "plan_id", "task", "result", "acceptance", "binding", "evidence", "subjects", "qualification", "claims", "controls"},
    2: {"schema_version", "plan_id", "task", "result", "acceptance", "binding", "evidence", "subjects", "qualification", "controls"},
    3: {"schema_version", "plan_id", "task", "result", "acceptance", "binding", "evidence", "subjects", "qualification", "claims", "controls"},
}
expected_inputs = {1: "ea551a35b058b19e402071cfc07d34862ec9216b", 2: "fd43bb83074e1dd75b5d7f44d9973f790e746a80", 3: "cee78ac90ae9a4dc3b07518089df26c8d64f68d1"}
expected_acceptance = {1: "ACC-P07-T1", 2: "ACC-P07-T2", 3: "ACC-P07-T3"}
focused = {
    1: ["tests/packaging/target_matrix.rs"],
    2: ["tests/packaging/no_skip.rs"],
    3: ["tests/packaging/artifact_layout.rs"],
}
results = []

for task in (1, 2, 3):
    receipt_path = root / f"reports/gates/p07/task-{task}.json"
    try:
        data = json.loads(receipt_path.read_text())
        if set(data) != required_top[task]:
            raise ValueError("receipt top-level schema is not exact")
        if data["schema_version"] != "taskseal.p07.task-receipt.v1" or data["plan_id"] != "P07-PACKAGING-SCAFFOLD-V1" or data["task"] != task or data["result"] != "accepted":
            raise ValueError("receipt identity/result mismatch")
        acceptance = data["acceptance"]
        if acceptance.get("id") != expected_acceptance[task] or not isinstance(acceptance.get("operator_result"), str) or not acceptance["operator_result"].strip() or not isinstance(acceptance.get("evidence_ids"), list) or not acceptance["evidence_ids"]:
            raise ValueError("acceptance/evidence presence invalid")
        binding = data["binding"]
        for key in ("input_head", "implementation_head", "receipt_seal_role", "implementation_files"):
            if key not in binding:
                raise ValueError(f"binding missing {key}")
        require_hex(binding["input_head"], 40, "input_head")
        require_hex(binding["implementation_head"], 40, "implementation_head")
        if binding["input_head"] != expected_inputs[task] or not is_commit(binding["input_head"]):
            raise ValueError("input head is not the accepted predecessor")
        if not is_commit(binding["implementation_head"]) or not ancestor(binding["implementation_head"]):
            raise ValueError("implementation head is absent or not an ancestor")
        if binding["receipt_seal_role"] != "receipt-only-child" or not isinstance(binding["implementation_files"], list) or not binding["implementation_files"]:
            raise ValueError("receipt seal/write-set declaration invalid")
        subjects = data["subjects"]
        if not isinstance(subjects, dict) or not subjects:
            raise ValueError("subjects missing")
        for path, digest in subjects.items():
            require_hex(digest, 64, f"subject {path}")
            tracked = git("ls-files", "--error-unmatch", path, check=False)
            if tracked.returncode or sha256_file(root / path) != digest:
                raise ValueError(f"subject digest mismatch: {path}")
        implementation_paths = tree_paths(binding["implementation_head"])
        if sorted(implementation_paths) != sorted(binding["implementation_files"]):
            raise ValueError("implementation commit changed paths outside declared write-set")
        receipt_commit = first_receipt_commit(binding["implementation_head"], f"reports/gates/p07/task-{task}.json")
        parents = git("rev-list", "--parents", "-n", "1", receipt_commit).stdout.split()
        if len(parents) != 2 or parents[1] != binding["implementation_head"]:
            raise ValueError("receipt is not the immediate single-parent child of implementation")
        if tree_paths(receipt_commit) != [f"reports/gates/p07/task-{task}.json"]:
            raise ValueError("receipt seal commit is not receipt-only")
        if not ancestor(receipt_commit):
            raise ValueError("current HEAD does not descend from receipt seal")
        results.append({"task": task, "receipt": str(receipt_path.relative_to(root)), "implementation_head": binding["implementation_head"], "receipt_commit": receipt_commit, "subjects": len(subjects), "status": "validated"})
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        raise SystemExit(f"P07_GATE_TASK_{task}_REFUSED: {exc}")

def run(label, argv):
    p = subprocess.run(argv, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    results.append({"check": label, "command": " ".join(argv), "exit": p.returncode, "output": p.stdout[-4000:]})
    if p.returncode:
        print(p.stdout, end="", file=sys.stderr)
        raise SystemExit(f"P07_GATE_CHECK_FAILED {label} exit={p.returncode}")

for task, paths in focused.items():
    source = paths[0]
    binary = f"/tmp/p07-scaffold-task-{task}-{os.getpid()}"
    run(f"task-{task}-focused", ["rustc", "--test", source, "-o", binary])
    run(f"task-{task}-focused-run", [binary])
run("task-2-scaffold-structure", ["scripts/release-build/verify-source.sh", "--workflow", ".github/workflows/release-candidate.yml", "--subject-digest", head, "--scaffold"])

print(json.dumps({"marker": "P07_PACKAGING_SCAFFOLD_V1_PASS", "result": "NOT_QUALIFIED", "plan_progress": "3/8", "overall_progress": "3/7", "head": head, "checks": results}, sort_keys=True))
print("P07_PACKAGING_SCAFFOLD_V1_PASS")
PY
