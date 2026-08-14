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
profiles = {
  1: ("reports/gates/p07/task-1.json", "fd43bb83074e1dd75b5d7f44d9973f790e746a80", "3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06", "correction_implementation_head"),
  2: ("reports/gates/p07/task-2.json", "cee78ac90ae9a4dc3b07518089df26c8d64f68d1", "3f2e59a113bcb38c6a53b14d8ee70c37823a29cdc2ef603d1b26b6da9f1d571a", "implementation_head"),
  3: ("reports/gates/p07/task-3.json", "0d5b7fbc9a079e8816bf4acfef6ee0e5b741a123", "26d05b906e14c6c9aeaf24b392082578ac788689b4cac7a088b80da170e2caef", "implementation_head"),
}

class Refused(Exception): pass
def git(*args):
    p = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True)
    if p.returncode: raise Refused("GIT")
    return p.stdout
def regular(rel):
    p = root / rel
    if p.is_symlink() or not p.is_file() or Path(rel).is_absolute() or Path(rel).as_posix() != rel or ".." in Path(rel).parts:
        raise Refused("PATH")
    rows = git("ls-files", "--stage", rel).splitlines()
    if len(rows) != 1 or not (rows[0].startswith("100644 ") or rows[0].startswith("100755 ")):
        raise Refused("PATH")
    return p.read_bytes()
def load(data):
    try: return json.loads(data.decode(), object_pairs_hook=lambda pairs: dict(pairs))
    except Exception as exc: raise Refused("JSON") from exc
def run(stage, *args, **kw):
    p = subprocess.run(args, cwd=root, text=True, capture_output=True, **kw)
    if p.returncode: raise Refused(stage)
    return p

try:
    if git("status", "--porcelain"): raise Refused("DIRTY")
    branch = git("branch", "--show-current").strip()
    if not branch or branch in ("main", "master"): raise Refused("DETACHED_OR_MAIN")
    manifest = load(regular("reports/gates/p07/scaffold-v2/source-manifest.json"))
    if set(manifest) != {"schema_version", "entries"} or manifest["schema_version"] != "taskseal.p07.scaffold-v2.source-manifest.v1": raise Refused("MANIFEST_SHAPE")
    entries = manifest["entries"]
    if not isinstance(entries, list) or len(entries) != 3: raise Refused("MANIFEST_ENTRIES")
    for task, entry in enumerate(entries, 1):
        path, commit, digest, selector = profiles[task]
        if entry != {"task": task, "path": path, "commit": commit, "sha256": digest, "implementation_selector": selector}: raise Refused("RECEIPT_DIGEST")
        blob = git("show", f"{commit}:{path}").encode()
        if hashlib.sha256(blob).hexdigest() != digest or regular(path) != blob: raise Refused("RECEIPT_DIGEST")
    for task in (1, 2, 3):
        projection = regular(f"reports/gates/p07/scaffold-v2/task-{task}.json")
        p = run(f"PROJECTION_T{task}", sys.executable, "scripts/gates/p07/scaffold-v2/normalize.py", "--root", str(root), "--task", str(task))
        if p.stdout.encode() != projection: raise Refused("PROJECTION")
    p = run("NORMALIZER_FOCUSED", sys.executable, "scripts/gates/p07/scaffold-v2/test-normalize.py")
    if "P07_SCAFFOLD_V2_NORMALIZER_MUTATIONS_PASS" not in p.stdout: raise Refused("NORMALIZER")
    compile_env = {**os.environ, "CARGO_MANIFEST_DIR": str(root)}
    for task, (source, output) in enumerate((("tests/packaging/target_matrix.rs", "/tmp/p07-exact-t1"), ("tests/packaging/no_skip.rs", "/tmp/p07-exact-t2"), ("tests/packaging/artifact_layout.rs", "/tmp/p07-exact-t3")), 1):
        run(f"FOCUSED_COMPILE_T{task}", "rustc", "--test", source, "-o", output, env=compile_env)
        run(f"FOCUSED_RUN_T{task}", output)
    verifier = os.environ.get("P07_EXACT_CURRENT_VERIFIER", "scripts/release-build/verify-source.sh")
    run("SOURCE_VERIFIER", verifier, "--workflow", ".github/workflows/release-candidate.yml", "--subject-digest", git("rev-parse", "HEAD").strip(), "--scaffold")
    result = {"qualification":"NOT_QUALIFIED", "p07":"3/8", "overall":"3/7", "tasks":[1,2,3]}
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    print("P07_EXACT_CURRENT_PREFLIGHT_PASS" if os.environ["P07_EXACT_CURRENT_MODE"] == "preflight" else "P07_PACKAGING_SCAFFOLD_EXACT_CURRENT_PASS")
except Refused as exc:
    print(refused + str(exc), file=sys.stderr)
    raise SystemExit(1)
PY
