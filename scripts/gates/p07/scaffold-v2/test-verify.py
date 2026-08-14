#!/usr/bin/env python3
"""Two-case exact-current gate test: baseline and frozen receipt digest refusal."""
from __future__ import annotations
import os, shutil, subprocess, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
GATE = ROOT / "scripts/gates/p07/scaffold-v2/verify.sh"

def fixture() -> Path:
    d = Path(tempfile.mkdtemp(prefix="p07-exact-current-")); repo = d / "repo"
    subprocess.run(["git", "clone", "--no-local", "-q", str(ROOT), str(repo)], check=True)
    target = repo / "scripts/gates/p07/scaffold-v2/verify.sh"; shutil.copy2(GATE, target); target.chmod(0o755)
    (repo / "scripts/gates/p07/scaffold-v2/test-normalize.py").write_text("#!/usr/bin/env python3\nprint('P07_SCAFFOLD_V2_NORMALIZER_MUTATIONS_PASS')\n")
    bindir = repo / "fixture-bin"; bindir.mkdir()
    (bindir / "rustc").write_text("#!/bin/sh\ntest \"${CARGO_MANIFEST_DIR:-}\" = \"$PWD\" || { printf 'fixture rustc missing repository root\\n' >&2; exit 86; }\ntest \"${P07_TEST_FORCE_RUSTC_FAILURE:-0}\" != 1 || { printf 'fixture rustc forced failure\\n' >&2; exit 87; }\nout=\nwhile [ $# -gt 0 ]; do if [ \"$1\" = -o ]; then out=$2; shift 2; else shift; fi; done\nprintf '#!/bin/sh\\nexit 0\\n' > \"$out\"; chmod +x \"$out\"\n")
    (bindir / "verify-source.sh").write_text("#!/bin/sh\nprintf 'P07_SCAFFOLD_VALIDATION_PASS\\n'\n")
    (bindir / "verify-source.sh").chmod(0o755)
    for p in bindir.iterdir(): p.chmod(0o755)
    subprocess.run(["git", "add", "."], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture exact-current gate"], cwd=repo, check=True)
    return repo

def invoke(repo: Path, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "PATH": str(repo / "fixture-bin") + os.pathsep + os.environ["PATH"], "P07_EXACT_CURRENT_VERIFIER": "verify-source.sh"}
    env.update(extra_env or {})
    return subprocess.run([str(repo / "scripts/gates/p07/scaffold-v2/verify.sh"), "--preflight"], cwd=repo, text=True, capture_output=True, env=env)

def main() -> int:
    repo = fixture()
    try:
        p = invoke(repo); out = p.stdout + p.stderr
        assert p.returncode == 0 and "P07_EXACT_CURRENT_PREFLIGHT_PASS" in out, out
        assert "P07_PACKAGING_SCAFFOLD_EXACT_CURRENT_PASS" not in out
        assert '"qualification":"QUALIFIED"' not in out and "release-qualified" not in out and "publication-ready" not in out and "signed" not in out and "notarized" not in out
        p = invoke(repo, {"P07_TEST_FORCE_RUSTC_FAILURE": "1"}); out = p.stdout + p.stderr
        assert p.returncode != 0 and "P07_EXACT_CURRENT_REFUSED:FOCUSED_COMPILE_T1" in out, out
        manifest = repo / "reports/gates/p07/scaffold-v2/source-manifest.json"
        manifest.write_text(manifest.read_text().replace("3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06", "0" * 64, 1))
        subprocess.run(["git", "add", str(manifest)], cwd=repo, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "mutate frozen receipt digest"], cwd=repo, check=True)
        p = invoke(repo); out = p.stdout + p.stderr
        assert p.returncode != 0 and "P07_EXACT_CURRENT_REFUSED:RECEIPT_DIGEST" in out, out
    finally:
        shutil.rmtree(repo.parent)
    print("P07_EXACT_CURRENT_TEST_PASS cases=3")
    return 0

if __name__ == "__main__": raise SystemExit(main())
