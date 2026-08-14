#!/bin/sh
set -eu
ROOT=.
if [ "${1:-}" = "--root" ]; then
  ROOT=${2:?missing root}
  shift 2
fi
if [ "$#" -ne 0 ]; then
  echo "P07_SCAFFOLD_V2_REFUSED:ARGUMENTS" >&2
  exit 1
fi
export P07_V2_ROOT="$ROOT"
export P07_V2_GATE_SCRIPT="$0"
python3 - <<'PY'
import hashlib, json, os, subprocess, sys
from pathlib import Path

root = Path(os.environ["P07_V2_ROOT"]).resolve()
refused = "P07_SCAFFOLD_V2_REFUSED:"
profiles = {
  1: ("reports/gates/p07/task-1.json", "fd43bb83074e1dd75b5d7f44d9973f790e746a80", "3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06"),
  2: ("reports/gates/p07/task-2.json", "cee78ac90ae9a4dc3b07518089df26c8d64f68d1", "3f2e59a113bcb38c6a53b14d8ee70c37823a29cdc2ef603d1b26b6da9f1d571a"),
  3: ("reports/gates/p07/task-3.json", "0d5b7fbc9a079e8816bf4acfef6ee0e5b741a123", "26d05b906e14c6c9aeaf24b392082578ac788689b4cac7a088b80da170e2caef"),
}
receipt_profiles = {
  1: "ea551a35b058b19e402071cfc07d34862ec9216b",
  2: "fd43bb83074e1dd75b5d7f44d9973f790e746a80",
  3: "cee78ac90ae9a4dc3b07518089df26c8d64f68d1",
}
class Refused(Exception): pass
def git(*args):
  p = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True)
  if p.returncode: raise Refused("GIT")
  return p.stdout
def pairs(items):
  out = {}
  for k,v in items:
    if k in out: raise Refused("DUPLICATE_JSON_KEY")
    out[k] = v
  return out
def load(data, label):
  try: return json.loads(data.decode(), object_pairs_hook=pairs)
  except Exception as e: raise Refused("JSON_" + label) from e
def regular(rel):
  p = root / rel
  if p.is_symlink() or not p.is_file() or Path(rel).is_absolute() or Path(rel).as_posix() != rel or ".." in Path(rel).parts:
    raise Refused("PATH")
  rows = git("ls-files", "--stage", rel).splitlines()
  if len(rows) != 1 or not (rows[0].startswith("100644 ") or rows[0].startswith("100755 ")):
    raise Refused("PATH")
  return p.read_bytes()
def check():
  if not root.is_dir(): raise Refused("ROOT")
  if git("status", "--porcelain"):
    raise Refused("DIRTY")
  branch = git("branch", "--show-current").strip()
  if not branch or branch in ("main", "master"):
    raise Refused("DETACHED_OR_MAIN")
  manifest = load(regular("reports/gates/p07/scaffold-v2/source-manifest.json"), "MANIFEST")
  if set(manifest) != {"schema_version", "entries"} or manifest["schema_version"] != "taskseal.p07.scaffold-v2.source-manifest.v1":
    raise Refused("MANIFEST_SHAPE")
  entries = manifest["entries"]
  if not isinstance(entries, list) or len(entries) != 3 or any(not isinstance(x, dict) for x in entries):
    raise Refused("MANIFEST_ENTRIES")
  paths = []
  for task, entry in enumerate(entries, 1):
    if set(entry) != {"task", "path", "commit", "sha256", "implementation_selector"} or entry["task"] != task:
      raise Refused("MANIFEST_ENTRIES")
    if entry["path"] in paths: raise Refused("MANIFEST_DUPLICATE")
    profile = profiles[task]
    if (entry["path"], entry["commit"], entry["sha256"]) != profile:
      raise Refused("SOURCE_IDENTITY")
    paths.append(entry["path"])
    blob = git("show", f"{entry['commit']}:{entry['path']}").encode()
    if hashlib.sha256(blob).hexdigest() != entry["sha256"] or regular(entry["path"]) != blob:
      raise Refused("SOURCE_BLOB")
    receipt = load(blob, f"SOURCE_{task}")
    binding = receipt.get("binding")
    if not isinstance(binding, dict): raise Refused("BINDING")
    implementation = binding.get("correction_implementation_head", binding.get("implementation_head"))
    input_head = binding.get("input_head")
    if not implementation or not input_head: raise Refused("BINDING")
    for commit in (input_head, implementation, entry["commit"]):
      git("cat-file", "-e", f"{commit}^{{commit}}")
    if subprocess.run(["git", "merge-base", "--is-ancestor", input_head, implementation], cwd=root).returncode:
      raise Refused("IMPLEMENTATION_ANCESTRY")
    if subprocess.run(["git", "merge-base", "--is-ancestor", implementation, entry["commit"]], cwd=root).returncode:
      raise Refused("RECEIPT_ANCESTRY")
    files = set(binding.get("implementation_files", []))
    changed = set(git("diff", "--name-only", f"{input_head}..{implementation}").splitlines())
    if not files or not files.issubset(changed): raise Refused("IMPLEMENTATION_FILES")
    commits = git("rev-list", "--reverse", f"{implementation}..{entry['commit']}").splitlines()
    for commit in commits:
      parents = git("rev-list", "--parents", "-n", "1", commit).split()
      changed_receipt = git("diff-tree", "--no-commit-id", "--name-only", "-r", commit).splitlines()
      if len(parents) != 2 or changed_receipt != [entry["path"]]:
        raise Refused("RECEIPT_LINEAGE")
  normalizer = root / "scripts/gates/p07/scaffold-v2/normalize.py"
  for task in (1, 2, 3):
    projection = regular(f"reports/gates/p07/scaffold-v2/task-{task}.json")
    proc = subprocess.run([sys.executable, str(normalizer), "--root", str(root), "--task", str(task)], text=True, capture_output=True)
    if proc.returncode or proc.stdout.encode() != projection:
      raise Refused("PROJECTION")
    value = load(projection, f"PROJECTION_{task}")
    if value.get("qualification") != "NOT_QUALIFIED" or value.get("task") != task:
      raise Refused("BOUNDARY")
    if not value.get("evidence_ids") or len(value["evidence_ids"]) != len(set(value["evidence_ids"])):
      raise Refused("ACCEPTANCE_EVIDENCE")
  gate_source = Path(os.environ.get("P07_V2_GATE_SCRIPT", "scripts/gates/p07/scaffold-v2/verify.sh")).read_text()
  if "P07_PACKAGING_SCAFFOLD_V2_PASS" not in gate_source or "P07_" + "PASS" in gate_source:
    raise Refused("OUTPUT_BOUNDARY")
  focused = (
    ("tests/packaging/target_matrix.rs", "/tmp/p07-v2-t1"),
    ("tests/packaging/no_skip.rs", "/tmp/p07-v2-t2"),
    ("tests/packaging/artifact_layout.rs", "/tmp/p07-v2-t3"),
  )
  for source, output in focused:
    if subprocess.run(["rustc", "--test", source, "-o", output], cwd=root).returncode:
      raise Refused("FOCUSED_COMPILE")
    if subprocess.run([output], cwd=root).returncode:
      raise Refused("FOCUSED_RUN")
  verifier = os.environ.get("P07_V2_VERIFY_SOURCE", "scripts/release-build/verify-source.sh")
  scaffold = subprocess.run([verifier, "--scaffold", "--subject-digest", git("rev-parse", "HEAD").strip()], cwd=root, text=True, capture_output=True)
  if scaffold.returncode or "P07_SCAFFOLD_VALIDATION_PASS" not in scaffold.stdout:
    raise Refused("SCAFFOLD_STRUCTURAL")
  return {"qualification":"NOT_QUALIFIED", "p07":"3/8", "overall":"3/7", "tasks":[1,2,3]}
try:
  result = check()
  print(json.dumps(result, sort_keys=True, separators=(",", ":")))
  print("P07_PACKAGING_SCAFFOLD_V2_PASS")
except Refused as e:
  print(refused + str(e), file=sys.stderr)
  raise SystemExit(1)
PY
