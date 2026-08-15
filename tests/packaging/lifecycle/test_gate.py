#!/usr/bin/env python3
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
GATE = ROOT / "scripts/gates/p07/verify.sh"


def run(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run([str(root / "scripts/gates/p07/verify.sh")], cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)


def clone() -> Path:
    root = Path(tempfile.mkdtemp(prefix="p07-final-gate-")) / "repo"
    subprocess.run(["git", "clone", "--quiet", "--no-hardlinks", str(ROOT), str(root)], check=True)
    for relative in ("packaging", "tests/packaging", "scripts/gates/p07", "reports/gates/p07"):
        shutil.copytree(ROOT / relative, root / relative, dirs_exist_ok=True)
    subprocess.run(["git", "config", "user.name", "p07-test"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.email", "p07-test@example.invalid"], cwd=root, check=True)
    subprocess.run(["git", "add", "packaging", "tests/packaging", "scripts/gates/p07", "reports/gates/p07"], cwd=root, check=True)
    subprocess.run(["git", "commit", "--quiet", "-m", "fixture baseline"], cwd=root, check=True)
    return root


def main() -> int:
    if not GATE.is_file():
        raise SystemExit("P07_FINAL_GATE_EXPECTED_RED: verifier missing")
    baseline = run(clone())
    if baseline.returncode or "P07_SUPPLY_CHAIN_GATE_PASS" not in baseline.stdout:
        raise SystemExit("P07_FINAL_GATE_EXPECTED_RED: baseline refused: " + baseline.stdout[-1200:])
    cases = {
        "lifecycle_failure": ("reports/gates/p07/homebrew-v1/lifecycle-result.json", lambda value: value.__setitem__("failure_class", "INSTALL_REFUSED")),
        "cleanup_false": ("reports/gates/p07/homebrew-v1/lifecycle-result.json", lambda value: value.__setitem__("cleanup_complete", False)),
        "qualification_pass": ("reports/gates/p07/task-7.json", lambda value: value.__setitem__("qualification", "PASS")),
        "upgrade_omitted": ("reports/gates/p07/homebrew-v1/lifecycle-result.json", lambda value: value.__setitem__("steps", [step for step in value["steps"] if step["name"] != "upgrade_n_plus_1"])),
        "same_artifact": ("reports/gates/p07/homebrew-v1/lifecycle-result.json", lambda value: value["archive"].__setitem__("n_plus_1", value["archive"]["n"])),
    }
    for name, (relative, mutate) in cases.items():
        root = clone()
        path = root / relative
        value = json.loads(path.read_text(encoding="utf-8")); mutate(value)
        path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
        subprocess.run(["git", "add", relative], cwd=root, check=True)
        subprocess.run(["git", "commit", "--quiet", "-m", "poison " + name], cwd=root, check=True)
        result = run(root)
        if result.returncode == 0:
            raise SystemExit(f"P07_FINAL_GATE_EXPECTED_REFUSAL_MISSING:{name}")
    print("P07_FINAL_GATE_MUTATIONS_PASS mutations=5")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
