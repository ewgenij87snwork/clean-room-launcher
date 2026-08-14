#!/usr/bin/env python3
"""Fixture-only mutation suite for the successor P07 V2 gate."""
from __future__ import annotations
import json, os, shutil, subprocess, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
GATE = ROOT / "scripts/gates/p07/scaffold-v2/verify.sh"
PASS = "P07_SCAFFOLD_V2_GATE_MUTATIONS_PASS"
REFUSED = "P07_SCAFFOLD_V2_REFUSED:"

def fixture() -> Path:
    d = Path(tempfile.mkdtemp(prefix="p07-v2-gate-"))
    repo = d / "repo"
    subprocess.run(["git", "clone", "--no-local", "-q", str(ROOT), str(repo)], check=True)
    target = repo / "scripts/gates/p07/scaffold-v2/verify.sh"
    shutil.copy2(GATE, target)
    subprocess.run(["git", "add", str(target.relative_to(repo))], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture gate"], cwd=repo, check=True)
    return repo

def invoke(repo: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run([str(repo / "scripts/gates/p07/scaffold-v2/verify.sh"), "--root", str(repo)], text=True, capture_output=True, env={**os.environ, "P07_V2_FIXTURE_MODE":"1"})

def refuse(repo: Path, label: str) -> None:
    p = invoke(repo)
    assert p.returncode != 0, f"{label}: accepted"
    out = p.stdout + p.stderr
    assert REFUSED in out, f"{label}: no refusal marker: {out}"
    assert "Traceback" not in out, f"{label}: traceback: {out}"

def main() -> int:
    assert "scripts/gates/p07/verify.sh" not in GATE.read_text()
    base = fixture()
    try:
        good = invoke(base)
        assert good.returncode == 0, good.stderr
        assert "P07_PACKAGING_SCAFFOLD_V2_PASS" in good.stdout
    finally:
        shutil.rmtree(base.parent)
    mutations = []
    def add(label, fn): mutations.append((label, fn))
    add("altered projection", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text("{}\n"))
    add("altered source receipt", lambda r: (r / "reports/gates/p07/task-1.json").write_text((r / "reports/gates/p07/task-1.json").read_text() + "\n"))
    add("symlink projection", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").unlink() or os.symlink("../../task-1.json", r / "reports/gates/p07/scaffold-v2/task-1.json"))
    add("untracked projection", lambda r: subprocess.run(["git", "rm", "--cached", "-q", "reports/gates/p07/scaffold-v2/task-1.json"], cwd=r, check=True))
    add("duplicate manifest task", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text((r / "reports/gates/p07/scaffold-v2/source-manifest.json").read_text().replace('"task": 2', '"task": 1', 1)))
    add("duplicate JSON key", lambda r: (r / "reports/gates/p07/scaffold-v2/source-manifest.json").write_text('{"schema_version":"x","schema_version":"y","entries":[]}\n'))
    add("qualification boundary", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text((r / "reports/gates/p07/scaffold-v2/task-1.json").read_text().replace('NOT_QUALIFIED', 'QUALIFIED', 1)))
    add("full P07 marker", lambda r: (r / "scripts/gates/p07/scaffold-v2/verify.sh").write_text((r / "scripts/gates/p07/scaffold-v2/verify.sh").read_text().replace('P07_PACKAGING_SCAFFOLD_V2_PASS', 'P07_PASS')))
    add("dirty tracked status", lambda r: (r / "Cargo.toml").write_text((r / "Cargo.toml").read_text() + "\nmutation\n"))
    add("removed acceptance evidence", lambda r: (r / "reports/gates/p07/task-1.json").write_text((r / "reports/gates/p07/scaffold-v2/task-1.json").read_text().replace('EVD-P07-T1-RED', 'EVD-MISSING', 1)))
    add("detached HEAD", lambda r: subprocess.run(["git", "checkout", "-q", "--detach", "HEAD"], cwd=r, check=True))
    for label, fn in mutations:
        repo = fixture()
        try:
            fn(repo)
            refuse(repo, label)
        finally:
            shutil.rmtree(repo.parent)
    print(f"{PASS} mutations={len(mutations)}")
    return 0

if __name__ == "__main__":
    try: raise SystemExit(main())
    except AssertionError as e:
        print(REFUSED + str(e))
        raise SystemExit(1)
