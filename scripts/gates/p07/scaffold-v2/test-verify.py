#!/usr/bin/env python3
"""Focused mutation suite for the exact-current negative scaffold gate."""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
GATE = ROOT / "scripts/gates/p07/scaffold-v2/verify.sh"
BASE = "e28e47309bde88582719a3e1b389667b8dfbc141"
CORRECTION = "reports/gates/p07/scaffold-v2/source-verifier-correction.json"
IMPLEMENTATION_PATHS = [
    "scripts/gates/p07/scaffold-v2/test-verify.py",
    "scripts/gates/p07/scaffold-v2/verify.sh",
    "scripts/release-build/verify-source.sh",
    "tests/packaging/no_skip.rs",
]
RED_COMMAND = (
    "sh -c 'CARGO_MANIFEST_DIR=\"$PWD\" rustc --test tests/packaging/no_skip.rs "
    "-o /tmp/p07-negative-no-skip && /tmp/p07-negative-no-skip; rust_exit=$?; "
    "python3 scripts/gates/p07/scaffold-v2/test-verify.py; gate_exit=$?; "
    "printf \"no_skip_exit=%s exact_current_exit=%s\\n\" \"$rust_exit\" \"$gate_exit\"; "
    "test \"$rust_exit\" -eq 0 && test \"$gate_exit\" -eq 0'"
)
GREEN_COMMAND = (
    "sh -c 'test -x scripts/gates/p07/scaffold-v2/verify.sh; executable_exit=$?; "
    "CARGO_MANIFEST_DIR=\"$PWD\" rustc --test tests/packaging/no_skip.rs "
    "-o /tmp/p07-negative-no-skip-green && /tmp/p07-negative-no-skip-green; rust_exit=$?; "
    "python3 scripts/gates/p07/scaffold-v2/test-verify.py; gate_exit=$?; "
    "printf \"executable_exit=%s no_skip_exit=%s exact_current_exit=%s\\n\" "
    "\"$executable_exit\" \"$rust_exit\" \"$gate_exit\"; "
    "test \"$executable_exit\" -eq 0 && test \"$rust_exit\" -eq 0 && test \"$gate_exit\" -eq 0'"
)
COMMAND_NAMES = [
    "workflow-no-skip",
    "workflow-orchestrator",
    "p02-gate",
    "p03-gate",
    "p04-gate",
    "p05-gate",
    "p06-gate",
    "p07-gate",
    "fmt",
    "clippy",
    "test",
    "schema",
    "golden",
    "parity",
    "privacy",
    "dependency",
    "license",
]


def run_git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def correction_receipt(repo: Path, implementation: str) -> dict[str, object]:
    return {
        "base_head": BASE,
        "evidence": {
            "green": {
                "command": GREEN_COMMAND,
                "exact_current_exit": 0,
                "executable_exit": 0,
                "marker": "P07_EXACT_CURRENT_TEST_PASS cases=4",
                "no_skip_exit": 0,
            },
            "red": {
                "command": RED_COMMAND,
                "exact_current_exit": 1,
                "no_skip_exit": 101,
            },
        },
        "forbidden_actions": {
            "external_contact": False,
            "integration": False,
            "main_mutation": False,
            "notarization": False,
            "provider_requests": False,
            "publication": False,
            "signing": False,
            "upload": False,
        },
        "implementation_head": implementation,
        "implementation_tree": run_git(repo, "rev-parse", f"{implementation}^{{tree}}"),
        "plan_id": "P07-SCAFFOLD-NEGATIVE-SOURCE-VERIFIER-V1",
        "qualification": "NOT_QUALIFIED",
        "schema_version": "taskseal.p07.scaffold-negative-source-verifier-receipt.v1",
        "subjects": {rel: sha(repo / rel) for rel in IMPLEMENTATION_PATHS},
    }


def write_fake_verifier(path: Path) -> None:
    path.write_text(
        """#!/usr/bin/env python3
import json, os, sys

args = sys.argv[1:]
subject = args[args.index("--subject-digest") + 1]
mode = os.environ.get("P07_TEST_SOURCE_MODE", "negative")
names = %r
commands = []
for name in names:
    code, status = (1, "NOT_QUALIFIED") if name == "p06-gate" else (0, "PASS")
    commands.append({"name": name, "exit": code, "status": status, "subject_digest": subject})
if mode == "missing-p07":
    commands = [item for item in commands if item["name"] != "p07-gate"]
elif mode == "failed-p07":
    for item in commands:
        if item["name"] == "p07-gate": item.update(exit=1, status="NOT_QUALIFIED")
if mode == "malformed":
    print("{")
    raise SystemExit(1)
value = {
    "schema_version": "taskseal.release-source-verification.v2",
    "result": "PASS" if mode == "qualified" else "NOT_QUALIFIED",
    "subject_digest": ("0" * 40) if mode == "wrong-subject" else subject,
    "commands": commands,
    "skips_counted_as_pass": 0,
    "p06_qualification": "QUALIFIED" if mode == "qualified" else "NOT_QUALIFIED",
    "network_or_provider_spend": False,
}
print(json.dumps(value, sort_keys=True, separators=(",", ":")))
raise SystemExit(0 if mode == "qualified" else 1)
""" % COMMAND_NAMES
    )
    path.chmod(0o755)


def fixture() -> tuple[Path, Path]:
    directory = Path(tempfile.mkdtemp(prefix="p07-exact-current-"))
    repo = directory / "repo"
    subprocess.run(["git", "clone", "--no-local", "-q", str(ROOT), str(repo)], check=True)
    subprocess.run(["git", "config", "user.name", "TaskSeal Fixture"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.email", "fixture@invalid"], cwd=repo, check=True)
    assert run_git(repo, "rev-parse", "HEAD") == BASE
    for rel in IMPLEMENTATION_PATHS:
        shutil.copy2(ROOT / rel, repo / rel)
    subprocess.run(["git", "add", "--", *IMPLEMENTATION_PATHS], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture source verifier implementation"], cwd=repo, check=True)
    implementation = run_git(repo, "rev-parse", "HEAD")
    write_json(repo / CORRECTION, correction_receipt(repo, implementation))
    subprocess.run(["git", "add", "--", CORRECTION], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", "fixture correction receipt"], cwd=repo, check=True)

    tools = directory / "fixture-bin"
    tools.mkdir()
    rustc = tools / "rustc"
    rustc.write_text(
        "#!/bin/sh\n"
        "test \"${CARGO_MANIFEST_DIR:-}\" = \"$PWD\" || { printf 'fixture rustc missing repository root\\n' >&2; exit 86; }\n"
        "test \"${P07_TEST_FORCE_RUSTC_FAILURE:-0}\" != 1 || { printf 'fixture rustc forced failure\\n' >&2; exit 87; }\n"
        "out=\n"
        "while [ $# -gt 0 ]; do if [ \"$1\" = -o ]; then out=$2; shift 2; else shift; fi; done\n"
        "printf '#!/bin/sh\\nexit 0\\n' > \"$out\"; chmod +x \"$out\"\n"
    )
    rustc.chmod(0o755)
    write_fake_verifier(tools / "verify-source.py")
    return repo, tools


def invoke(
    repo: Path, tools: Path, extra_env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    env = {
        **os.environ,
        "PATH": str(tools) + os.pathsep + os.environ["PATH"],
        "P07_EXACT_CURRENT_VERIFIER": str(tools / "verify-source.py"),
    }
    env.update(extra_env or {})
    return subprocess.run(
        ["sh", str(repo / "scripts/gates/p07/scaffold-v2/verify.sh"), "--preflight"],
        cwd=repo,
        text=True,
        capture_output=True,
        env=env,
    )


def combined(process: subprocess.CompletedProcess[str]) -> str:
    return process.stdout + process.stderr


def main() -> int:
    repo, tools = fixture()
    try:
        process = invoke(repo, tools)
        output = combined(process)
        assert process.returncode == 0 and "P07_EXACT_CURRENT_PREFLIGHT_PASS" in output, output
        assert "P07_PACKAGING_SCAFFOLD_EXACT_CURRENT_PASS" not in output
        for forbidden in (
            '"qualification":"QUALIFIED"',
            "release-qualified",
            "publication-ready",
            "signed",
            "notarized",
        ):
            assert forbidden not in output

        process = invoke(repo, tools, {"P07_TEST_FORCE_RUSTC_FAILURE": "1"})
        output = combined(process)
        assert process.returncode != 0 and "P07_EXACT_CURRENT_REFUSED:FOCUSED_COMPILE_T1" in output, output

        for mode in ("qualified", "malformed", "wrong-subject", "missing-p07", "failed-p07"):
            process = invoke(repo, tools, {"P07_TEST_SOURCE_MODE": mode})
            output = combined(process)
            assert process.returncode != 0 and "P07_EXACT_CURRENT_REFUSED:SOURCE_VERIFIER" in output, f"{mode}: {output}"

        manifest = repo / "reports/gates/p07/scaffold-v2/source-manifest.json"
        manifest.write_text(
            manifest.read_text().replace(
                "3d06a78a25f463405d8ee3ce8a31e344f38d698fce3b3d303a73e5dcd2b35f06",
                "0" * 64,
                1,
            )
        )
        subprocess.run(["git", "add", str(manifest)], cwd=repo, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "mutate frozen receipt digest"], cwd=repo, check=True)
        process = invoke(repo, tools)
        output = combined(process)
        assert process.returncode != 0 and "P07_EXACT_CURRENT_REFUSED:RECEIPT_DIGEST" in output, output
    finally:
        shutil.rmtree(repo.parent)
    print("P07_EXACT_CURRENT_TEST_PASS cases=4")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"P07_EXACT_CURRENT_TEST_REFUSED:{exc}", file=sys.stderr)
        raise SystemExit(1)
