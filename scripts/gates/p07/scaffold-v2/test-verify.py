#!/usr/bin/env python3
"""Clean committed-fixture mutation suite for the P07 V2 gate."""
from __future__ import annotations
import os, shutil, subprocess, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
GATE = ROOT / "scripts/gates/p07/scaffold-v2/verify.sh"
PASS = "P07_SCAFFOLD_V2_GATE_MUTATIONS_PASS"
REFUSED = "P07_SCAFFOLD_V2_REFUSED:"

def fixture() -> Path:
    d = Path(tempfile.mkdtemp(prefix="p07-v2-gate-")); repo = d / "repo"
    subprocess.run(["git", "clone", "--no-local", "-q", str(ROOT), str(repo)], check=True)
    target = repo / "scripts/gates/p07/scaffold-v2/verify.sh"; shutil.copy2(GATE, target)
    (repo / "scripts/gates/p07/scaffold-v2/test-normalize.py").write_text("#!/usr/bin/env python3\nprint('P07_SCAFFOLD_V2_NORMALIZER_MUTATIONS_PASS mutations=26')\n")
    bindir = repo / "fixture-bin"; bindir.mkdir()
    (bindir / "rustc").write_text("#!/bin/sh\nout=\nwhile [ $# -gt 0 ]; do if [ \"$1\" = -o ]; then out=$2; shift 2; else shift; fi; done\nprintf '#!/bin/sh\\nexit 0\\n' > \"$out\"\nchmod +x \"$out\"\n")
    (bindir / "verify-source.sh").write_text("#!/bin/sh\nprintf 'P07_SCAFFOLD_VALIDATION_PASS\\n'\n")
    for p in bindir.iterdir(): p.chmod(0o755)
    subprocess.run(["git", "add", "."], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture gate"], cwd=repo, check=True)
    return repo

def invoke(repo: Path) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "PATH": str(repo / "fixture-bin") + os.pathsep + os.environ["PATH"], "P07_V2_VERIFY_SOURCE":"verify-source.sh"}
    return subprocess.run([str(repo / "scripts/gates/p07/scaffold-v2/verify.sh"), "--root", str(repo)], text=True, capture_output=True, env=env)

def mutate_commit(repo: Path, label: str, fn) -> None:
    fn(repo); subprocess.run(["git", "add", "-A"], cwd=repo, check=True); subprocess.run(["git", "commit", "-q", "-m", label], cwd=repo, check=True)

def assert_stage(repo: Path, label: str, stage: str) -> None:
    p = invoke(repo); out = p.stdout + p.stderr
    assert p.returncode != 0 and REFUSED + stage in out, f"{label}: expected {stage}, got {out}"
    assert "Traceback" not in out

def main() -> int:
    base = fixture()
    try:
        p = invoke(base); assert p.returncode == 0 and "P07_PACKAGING_SCAFFOLD_V2_PASS" in p.stdout, p.stderr
    finally: shutil.rmtree(base.parent)
    cases = [
      ("wrong source commit", "SOURCE_IDENTITY", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace("fd43bb83074e1dd75b5d7f44d9973f790e746a80", "0"*40, 1))),
      ("wrong parent/non-ancestor", "SOURCE_BLOB", lambda r: (r / "reports/gates/p07/task-1.json").write_text((r / "reports/gates/p07/task-1.json").read_text().replace("b08387b5bc060148ad0ffecbdb889f7f50fc2ba0", "0"*40, 1))),
      ("duplicate task", "MANIFEST_ENTRIES", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace('"task": 2', '"task": 1', 1))),
      ("duplicate path", "MANIFEST_DUPLICATE", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace('reports/gates/p07/task-2.json', 'reports/gates/p07/task-1.json', 1))),
      ("source Git blob/current", "SOURCE_BLOB", lambda r: (r / "reports/gates/p07/task-1.json").write_text((r / "reports/gates/p07/task-1.json").read_text() + "\n")),
      ("canonical subject", "PROJECTION", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text((r / "reports/gates/p07/scaffold-v2/task-1.json").read_text().replace('NOT_QUALIFIED', 'QUALIFIED', 1))),
      ("canonical duplicate JSON", "DUPLICATE_JSON_KEY", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text('{"schema_version":"x","schema_version":"y"}\n')),
      ("implementation subject", "SUBJECT_SHA", lambda r: (r / "packaging/targets.toml").write_text((r / "packaging/targets.toml").read_text() + "\nsubject mutation\n")),
      ("source subject/evidence", "SOURCE_BLOB", lambda r: (r / "reports/gates/p07/task-1.json").write_text((r / "reports/gates/p07/task-1.json").read_text().replace('EVD-P07-T1-GREEN', 'EVD-DUPLICATE', 1))),
      ("symlink projection", "PATH", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").unlink() or os.symlink('../../task-1.json', r / "reports/gates/p07/scaffold-v2/task-1.json")),
      ("untracked projection", "PATH", lambda r: subprocess.run(["git", "rm", "-q", "reports/gates/p07/scaffold-v2/task-1.json"], cwd=r, check=True)),
      ("QUALIFIED", "PROJECTION", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text((r / "reports/gates/p07/scaffold-v2/task-1.json").read_text().replace('NOT_QUALIFIED', 'QUALIFIED', 1))),
      ("full P07_PASS", "OUTPUT_BOUNDARY", lambda r: (r / "scripts/gates/p07/scaffold-v2/verify.sh").write_text((r / "scripts/gates/p07/scaffold-v2/verify.sh").read_text().replace('P07_PACKAGING_SCAFFOLD_V2_PASS', 'P07_PASS'))),
      ("detached HEAD", "DETACHED_OR_MAIN", lambda r: subprocess.run(["git", "checkout", "-q", "--detach", "HEAD"], cwd=r, check=True)),
      ("dirty tracked", "DIRTY", lambda r: (r / "Cargo.toml").write_text((r / "Cargo.toml").read_text() + "\nmutation\n")),
      ("receipt merge", "SOURCE_IDENTITY", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace('"commit": "fd43bb83074e1dd75b5d7f44d9973f790e746a80"', '"commit": "' + '0'*40 + '"', 1))),
      ("receipt other-file diff", "SOURCE_IDENTITY", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace('"commit": "fd43bb83074e1dd75b5d7f44d9973f790e746a80"', '"commit": "' + '1'*40 + '"', 1))),
    ]
    for label, stage, fn in cases:
        repo = fixture()
        try:
            if label == "dirty tracked": fn(repo); assert_stage(repo, label, stage)
            elif label == "detached HEAD": fn(repo); assert_stage(repo, label, stage)
            else: mutate_commit(repo, label, fn); assert_stage(repo, label, stage)
        finally: shutil.rmtree(repo.parent)
    print(f"{PASS} mutations={len(cases)}")
    return 0

if __name__ == "__main__": raise SystemExit(main())
