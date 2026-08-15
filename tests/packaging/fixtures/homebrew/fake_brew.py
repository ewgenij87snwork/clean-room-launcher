#!/usr/bin/env python3
"""Small fake brew used only by P07 lifecycle process tests."""
from __future__ import annotations
import json, os, sys
from pathlib import Path

root = Path(os.environ["P07_FAKE_ROOT"]); ledger = root / "ledger.jsonl"; ledger.parent.mkdir(parents=True, exist_ok=True)
argv = sys.argv[1:]
with ledger.open("a", encoding="utf-8") as out: out.write(json.dumps(argv, separators=(",", ":")) + "\n")
if argv == ["--prefix"] or argv == ["--repository"]: print(root / "prefix")
elif argv == ["--cellar"]: print(root / "prefix" / "Cellar")
elif argv[:2] == ["trust", "--tap"]: raise SystemExit(2)
else: print("fake-brew-ok")
