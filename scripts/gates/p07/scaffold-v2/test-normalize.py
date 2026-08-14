#!/usr/bin/env python3
"""Focused RED/GREEN contract tests for the canonical P07 V2 adapter."""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
NORMALIZE = ROOT / "scripts/gates/p07/scaffold-v2/normalize.py"
MARKER = "P07_SCAFFOLD_V2_NORMALIZER_MUTATIONS_PASS"


def run(task: int, root: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(NORMALIZE), "--root", str(root), "--task", str(task)],
        text=True,
        capture_output=True,
        env={**os.environ, "PYTHONHASHSEED": "0"},
    )


def main() -> int:
    if not NORMALIZE.exists():
        print("P07_V2_NORMALIZER_MISSING")
        return 1

    outputs = []
    for task in (1, 2, 3):
        first = run(task)
        assert first.returncode == 0, first.stderr
        second = run(task)
        assert second.returncode == 0, second.stderr
        assert first.stdout == second.stdout
        value = json.loads(first.stdout)
        assert list(value) == [
            "acceptance_id", "binding", "evidence_ids", "plan_id", "qualification",
            "schema_version", "source_receipt", "subjects", "task",
        ]
        assert value["schema_version"] == "taskseal.p07.consolidated-task-receipt.v2"
        assert value["plan_id"] == "P07-PACKAGING-SCAFFOLD-V1"
        assert value["task"] == task
        assert value["qualification"] == "NOT_QUALIFIED"
        outputs.append(first.stdout.encode())

    destination = ROOT / "reports/gates/p07/scaffold-v2"
    for task, expected in zip((1, 2, 3), outputs):
        actual = (destination / f"task-{task}.json").read_bytes()
        assert actual == expected

    manifest = json.loads((destination / "source-manifest.json").read_text())
    assert list(manifest) == ["schema_version", "entries"]
    assert manifest["schema_version"] == "taskseal.p07.scaffold-v2.source-manifest.v1"
    assert [entry["task"] for entry in manifest["entries"]] == [1, 2, 3]
    assert len({entry["path"] for entry in manifest["entries"]}) == 3
    print(MARKER)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"P07_SCAFFOLD_V2_NORMALIZER_REFUSED:{exc}")
        raise SystemExit(1)
