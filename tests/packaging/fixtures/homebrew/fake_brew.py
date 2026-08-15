#!/usr/bin/env python3
"""Stateful, workspace-confined fake brew for P07 process tests only."""
from __future__ import annotations
import json, os, sys
from pathlib import Path

root = Path(os.environ["P07_FAKE_ROOT"]).resolve()
prefix = root / "prefix"
ledger = root / "ledger.jsonl"
argv = sys.argv[1:]
if not argv or Path(os.environ.get("HOMEBREW_PREFIX", "")).resolve() != prefix:
    raise SystemExit(2)
ledger.parent.mkdir(parents=True, exist_ok=True)
with ledger.open("a", encoding="utf-8") as out:
    out.write(json.dumps(argv, separators=(",", ":")) + "\n")
state_path = root / "state.json"
state = json.loads(state_path.read_text()) if state_path.exists() else {"tap": False, "trusted": [], "installed": []}
formulae = {"taskseal-local/preview/taskseal-preview", "taskseal-local/preview/taskseal-preview@0.0.1"}
if argv in (["--prefix"], ["--repository"]): print(prefix)
elif argv == ["--cellar"]: print(prefix / "Cellar")
elif argv[:2] == ["trust", "--tap"]: raise SystemExit(2)
elif argv[:2] in (["trust", "--formula"], ["untrust", "--formula"]):
    if len(argv) != 3 or argv[2] not in formulae: raise SystemExit(2)
    if argv[0] == "trust": state["trusted"] = sorted(set(state["trusted"] + [argv[2]]))
    else: state["trusted"] = [item for item in state["trusted"] if item != argv[2]]
elif argv[0] == "tap":
    if argv[:2] != ["tap", "taskseal-local/preview"] or len(argv) != 3 or not Path(argv[2]).resolve().is_relative_to(root): raise SystemExit(2)
    state["tap"] = True
elif argv[0] in {"style", "audit", "install", "test", "upgrade", "unlink", "link", "uninstall"}:
    item = argv[-1]
    if item not in formulae or (argv[0] in {"install", "upgrade"} and item not in state["trusted"]): raise SystemExit(2)
    if argv[0] == "install": state["installed"] = sorted(set(state["installed"] + [item]))
    if argv[0] == "uninstall": state["installed"] = [value for value in state["installed"] if value != item]
elif argv == ["untap", "taskseal-local/preview"]: state["tap"] = False
else: raise SystemExit(2)
state_path.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")
