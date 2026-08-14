#!/usr/bin/env python3
"""Real disposable-Git mutation suite for the P07 V2 normalizer."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
NORMALIZE = ROOT / "scripts/gates/p07/scaffold-v2/normalize.py"
MARKER = "P07_SCAFFOLD_V2_NORMALIZER_MUTATIONS_PASS"
REFUSED = "P07_SCAFFOLD_V2_NORMALIZER_REFUSED:"


def clone() -> Path:
    path = Path(tempfile.mkdtemp(prefix="p07-v2-fixture-")) / "repo"
    subprocess.run(["git", "clone", "--no-local", "-q", str(ROOT), str(path)], check=True)
    return path


def write_json(root: Path, rel: str, value: object) -> None:
    (root / rel).write_text(json.dumps(value, indent=2) + "\n")


def source(root: Path, task: int = 1) -> Path:
    return root / f"reports/gates/p07/task-{task}.json"


def manifest(root: Path) -> Path:
    return root / "reports/gates/p07/scaffold-v2/source-manifest.json"


def run(root: Path, task: int = 1) -> subprocess.CompletedProcess[str]:
    return subprocess.run([sys.executable, str(NORMALIZE), "--root", str(root), "--task", str(task)], text=True, capture_output=True)


def assert_refused(root: Path, task: int = 1, label: str = "") -> None:
    result = run(root, task)
    assert result.returncode != 0, f"{label}: mutation unexpectedly accepted"
    combined = result.stdout + result.stderr
    assert REFUSED in combined, f"{label}: missing standardized refusal: {combined}"
    assert "Traceback" not in combined, f"{label}: traceback leaked: {combined}"


def mutate_manifest(root: Path, mutate) -> None:
    value = json.loads(manifest(root).read_text())
    mutate(value)
    write_json(root, str(manifest(root).relative_to(root)), value)


def main() -> int:
    if not NORMALIZE.exists():
        print("P07_V2_NORMALIZER_MISSING")
        return 1
    baseline = clone()
    try:
        for task in (1, 2, 3):
            first, second = run(baseline, task), run(baseline, task)
            assert first.returncode == second.returncode == 0, first.stderr + second.stderr
            assert first.stdout == second.stdout
            assert json.loads(first.stdout)["task"] == task

        mutations = []
        def m(label, fn, task=1):
            mutations.append((label, fn, task))

        m("wrong source SHA", lambda r: mutate_manifest(r, lambda x: x["entries"][0].__setitem__("sha256", "0" * 64)))
        m("wrong source commit", lambda r: mutate_manifest(r, lambda x: x["entries"][0].__setitem__("commit", "0" * 40)))
        m("wrong task", lambda r: None, 4)
        m("missing manifest key", lambda r: mutate_manifest(r, lambda x: x["entries"][0].pop("sha256")))
        m("extra manifest key", lambda r: mutate_manifest(r, lambda x: x["entries"][0].__setitem__("extra", True)))
        m("duplicate manifest task", lambda r: mutate_manifest(r, lambda x: x["entries"][1].__setitem__("task", 1)))
        m("duplicate manifest path", lambda r: mutate_manifest(r, lambda x: x["entries"][1].__setitem__("path", x["entries"][0]["path"])))
        m("null manifest entry", lambda r: mutate_manifest(r, lambda x: x["entries"].__setitem__(1, None)))
        m("scalar manifest entry", lambda r: mutate_manifest(r, lambda x: x["entries"].__setitem__(1, "task-2")))
        m("wrong selector", lambda r: mutate_manifest(r, lambda x: x["entries"][0].__setitem__("implementation_selector", "implementation_head")))
        m("source receipt subject mutation", lambda r: source(r).write_text(source(r).read_text().replace("392a949", "492a949", 1)))
        m("source receipt task mutation", lambda r: source(r).write_text(source(r).read_text().replace('"task": 1', '"task": 2', 1)))
        m("canonical subject mutation", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text((r / "reports/gates/p07/scaffold-v2/task-1.json").read_text().replace("392a949", "492a949", 1)))
        m("canonical duplicate JSON key", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text('{"schema_version":"x","schema_version":"y"}\n'))
        m("nested duplicate JSON key", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text('{"schema_version":"x","binding":{"input_head":"a","input_head":"b"}}\n'))
        m("canonical missing key", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").write_text('{}\n'))
        m("canonical symlink", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").unlink() or os.symlink("../../task-1.json", r / "reports/gates/p07/scaffold-v2/task-1.json"))
        m("canonical non-regular", lambda r: (r / "reports/gates/p07/scaffold-v2/task-1.json").unlink() or (r / "reports/gates/p07/scaffold-v2/task-1.json").mkdir())
        m("canonical untracked", lambda r: subprocess.run(["git", "rm", "--cached", "-q", "reports/gates/p07/scaffold-v2/task-1.json"], cwd=r, check=True))
        m("source symlink", lambda r: (source(r).unlink() or os.symlink("task-2.json", source(r))))
        m("source non-regular", lambda r: (source(r).unlink() or source(r).mkdir()))
        m("source untracked", lambda r: subprocess.run(["git", "rm", "--cached", "-q", "reports/gates/p07/task-1.json"], cwd=r, check=True))
        m("duplicate evidence IDs", lambda r: (source(r).write_text(source(r).read_text().replace('EVD-P07-T1-GREEN', 'EVD-P07-T1-RED', 1))))
        m("malformed evidence entry", lambda r: (source(r).write_text(source(r).read_text().replace('{\n      "id":', '{\n      "bad":', 1))))
        m("wrong implementation head", lambda r: source(r).write_text(source(r).read_text().replace("b08387b5bc060148ad0ffecbdb889f7f50fc2ba0", "0" * 40, 1)))
        m("non-ancestor implementation", lambda r: source(r).write_text(source(r).read_text().replace("b08387b5bc060148ad0ffecbdb889f7f50fc2ba0", "25f579b25ed154569234b3683cd9bd67d594cc4d", 1)))
        for label, mutation, task in mutations:
            fixture = clone()
            try:
                mutation(fixture)
                assert_refused(fixture, task, label)
            finally:
                shutil.rmtree(fixture.parent)
        spec = importlib.util.spec_from_file_location("p07_normalize", NORMALIZE)
        module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        spec.loader.exec_module(module)
        lineage = clone()
        try:
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=lineage, text=True).strip()
            subprocess.run(["git", "checkout", "-q", "-b", "side"], cwd=lineage, check=True)
            (lineage / "reports/gates/p07/task-1.json").write_text((lineage / "reports/gates/p07/task-1.json").read_text() + "\n")
            subprocess.run(["git", "add", "reports/gates/p07/task-1.json"], cwd=lineage, check=True)
            subprocess.run(["git", "commit", "-q", "-m", "fixture side"], cwd=lineage, check=True)
            side = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=lineage, text=True).strip()
            subprocess.run(["git", "checkout", "-q", "-"], cwd=lineage, check=True)
            (lineage / "reports/gates/p07/task-1.json").write_text((lineage / "reports/gates/p07/task-1.json").read_text() + "\n\n")
            subprocess.run(["git", "add", "reports/gates/p07/task-1.json"], cwd=lineage, check=True)
            subprocess.run(["git", "commit", "-q", "-m", "fixture main"], cwd=lineage, check=True)
            subprocess.run(["git", "merge", "-s", "ours", "--no-ff", "-q", "side", "-m", "fixture merge"], cwd=lineage, check=True)
            merge = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=lineage, text=True).strip()
            try:
                module.verify_lineage(lineage, {"input_head": base, "implementation_head": base, "commit": merge, "path": "reports/gates/p07/task-1.json"})
                raise AssertionError("receipt-lineage merge unexpectedly accepted")
            except module.Refused:
                pass
            subprocess.run(["git", "checkout", "-q", base], cwd=lineage, check=True)
            (lineage / "README.md").write_text("unexpected receipt-lineage file\n")
            subprocess.run(["git", "add", "README.md"], cwd=lineage, check=True)
            subprocess.run(["git", "commit", "-q", "-m", "fixture non receipt"], cwd=lineage, check=True)
            unexpected = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=lineage, text=True).strip()
            try:
                module.verify_lineage(lineage, {"input_head": base, "implementation_head": base, "commit": unexpected, "path": "reports/gates/p07/task-1.json"})
                raise AssertionError("non-receipt lineage unexpectedly accepted")
            except module.Refused:
                pass
        finally:
            shutil.rmtree(lineage.parent)
        print(f"{MARKER} mutations={len(mutations)}")
        return 0
    finally:
        shutil.rmtree(baseline.parent)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"{REFUSED}{exc}")
        raise SystemExit(1)
