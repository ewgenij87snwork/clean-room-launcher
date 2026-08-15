#!/usr/bin/env python3
"""Stateful, workspace-confined fake brew for P07 process tests only."""
from __future__ import annotations
import json, os, sys
import urllib.request
from pathlib import Path

root = Path(os.environ["P07_FAKE_ROOT"]).resolve(); prefix = root / "prefix"; argv = sys.argv[1:]; scenario = os.environ.get("P07_SCENARIO", "")
credential_words = ("TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "KEY")
if not argv or Path(os.environ.get("HOMEBREW_PREFIX", "")).resolve() != prefix or os.environ.get("HOME") != str(root / "home") or os.environ.get("PATH") != f"{root / 'poison'}:/usr/bin:/bin:/usr/sbin:/sbin" or any(any(word in key.upper() for word in credential_words) for key in os.environ): raise SystemExit(2)
if scenario == "require_portable_ruby" and not (Path(os.environ["HOMEBREW_REPOSITORY"]) / "Library/Homebrew/vendor/portable-ruby/current/bin/ruby").is_file(): raise SystemExit(2)
if scenario == "require_portable_ruby" and ("HOMEBREW_NO_INSTALL_FROM_API" in os.environ or not any((Path(os.environ["HOMEBREW_CACHE"]) / "api/internal").glob("packages.*.jws.json"))): raise SystemExit(2)
if scenario == "require_rendered_formula":
    formula = Path(os.environ["HOMEBREW_ALLOWED_TAPS"]) / "Formula/taskseal-preview.rb"
    if not formula.is_file() or 'url "http://127.0.0.1:49152/taskseal-v0.0.1-aarch64-apple-darwin.tar.gz"' not in formula.read_text(encoding="utf-8") or 'sha256 "' not in formula.read_text(encoding="utf-8"): raise SystemExit(2)
ledger = root / "ledger.jsonl"; ledger.parent.mkdir(parents=True, exist_ok=True)
with ledger.open("a", encoding="utf-8") as out: out.write(json.dumps({"argv": argv}, sort_keys=True, separators=(",", ":")) + "\n")
state_path = root / "state.json"; state = json.loads(state_path.read_text()) if state_path.exists() else {"tap": False, "trusted": [], "installed": []}
formulae = {"taskseal-local/preview/taskseal-preview", "taskseal-local/preview/taskseal-preview@0.0.1"}
if argv in (["--prefix"], ["--repository"]):
    if scenario == "reported_prefix_mismatch" and argv == ["--prefix"] or scenario == "reported_repository_mismatch" and argv == ["--repository"]: print(root / "live")
    else: print(prefix)
elif argv == ["--cellar"]: print(root / "live" if scenario == "reported_cellar_mismatch" else prefix / "Cellar")
elif argv[:2] == ["trust", "--tap"]: raise SystemExit(2)
elif argv[:2] in (["trust", "--formula"], ["untrust", "--formula"]):
    if len(argv) != 3 or argv[2] not in formulae or (argv[0] == "trust" and (os.environ.get("HOMEBREW_REQUIRE_TAP_TRUST") != "1" or os.environ.get("HOMEBREW_ALLOWED_TAPS") != str(root / "tap") or scenario == "missing_item_trust")): raise SystemExit(2)
    state["trusted"] = sorted(set(state["trusted"] + [argv[2]])) if argv[0] == "trust" else [x for x in state["trusted"] if x != argv[2]]
elif argv[0] == "tap":
    if scenario == "tap_clone_failed":
        print("fatal: local clone failed", file=sys.stderr); raise SystemExit(2)
    if argv[:2] != ["tap", "taskseal-local/preview"] or len(argv) != 3 or not Path(argv[2]).resolve().is_relative_to(root) or os.environ.get("HOMEBREW_ALLOWED_TAPS") != argv[2]: raise SystemExit(2)
    state["tap"] = True
elif argv[0] in {"style", "audit", "test", "upgrade", "unlink", "link", "install", "uninstall"}:
    item = argv[-1]
    if scenario == "require_native_install_boundary" and argv[0] in {"install", "test"}:
        if os.environ.get("P07_NETWORK_BOUNDARY") != "homebrew-native-sandbox-loopback-proxy" or "HOMEBREW_AVOID_NESTED_SANDBOXING" in os.environ: raise SystemExit(2)
    if item not in formulae or (argv[0] in {"install", "upgrade"} and item not in state["trusted"]): raise SystemExit(2)
    if scenario == "install_archive_fetch_failed" and argv[0] == "install":
        print("Error: unsupported formula dependency"); raise SystemExit(2)
    if scenario == "style_refusal_cleanup" and argv[0] in {"style", "uninstall"}: raise SystemExit(2)
    if scenario in {"require_loopback_server", "require_native_install_boundary"} and argv[0] == "install":
        body = urllib.request.urlopen("http://127.0.0.1:49152/taskseal-v0.0.1-aarch64-apple-darwin.tar.gz", timeout=1).read()
        if not body: raise SystemExit(2)
    if argv[0] == "install":
        version = "0.0.1" if item.endswith("@0.0.1") else ("0.0.2" if "upgrade" in state.get("events", []) else "0.0.1")
        state["installed"] = sorted(set(state["installed"] + [item])); cell = prefix / "Cellar" / item.rsplit("/", 1)[-1] / version / "bin"; cell.mkdir(parents=True, exist_ok=True)
        payload = b"#!/bin/sh\nprintf 'OUTPUT_UNSUPPORTED_FOR_COMMAND: status; use human output\\n'\n[ \"$1\" = status ] || exit 2\n"
        for name in ("taskseal", "tseal"): (cell / name).write_bytes(payload); (cell / name).chmod(0o755); (prefix / "bin").mkdir(parents=True, exist_ok=True); target = prefix / "bin" / name; target.unlink(missing_ok=True); target.symlink_to(cell / name)
    if argv[0] == "upgrade": state["events"] = state.get("events", []) + ["upgrade"]
    if argv[0] == "uninstall" and scenario != "partial_uninstall": state["installed"] = [x for x in state["installed"] if x != item]; shutil_target = prefix / "Cellar" / item.rsplit("/", 1)[-1];
    if argv[0] == "uninstall" and scenario != "partial_uninstall":
        import shutil; shutil.rmtree(prefix / "Cellar" / item.rsplit("/", 1)[-1], ignore_errors=True)
elif argv == ["untap", "taskseal-local/preview"]:
    if scenario == "cleanup_failure": raise SystemExit(2)
    state["tap"] = False
else: raise SystemExit(2)
if scenario.startswith("sentinel_") or scenario == "config_mutation":
    key = "config" if scenario == "config_mutation" else scenario.removeprefix("sentinel_").removesuffix("_mutation")
    (root / ("gitconfig" if key == "config" else "sentinel-" + key)).write_text("mutated\n", encoding="utf-8")
state_path.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")
