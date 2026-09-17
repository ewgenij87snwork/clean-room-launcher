#!/usr/bin/env python3
"""Stateful, workspace-confined fake brew for CLROOM_PACKAGING process tests only."""
from __future__ import annotations
import json, os, sys
import urllib.request
from pathlib import Path

FORMULA_CURRENT = "clroom-local/preview/clroom-preview"
FORMULA_VERSIONED = "clroom-local/preview/clroom-preview@0.0.1"


def canonical_ledger_token(value: str, tap_path: str) -> str:
    if value == "--prefix": return "--prefix"
    if value == "--repository": return "--repository"
    if value == "--cellar": return "--cellar"
    if value == "trust": return "trust"
    if value == "untrust": return "untrust"
    if value == "--tap": return "--tap"
    if value == "--formula": return "--formula"
    if value == "tap": return "tap"
    if value == "clroom-local/preview": return "clroom-local/preview"
    if value == "style": return "style"
    if value == "audit": return "audit"
    if value == "--strict": return "--strict"
    if value == "test": return "test"
    if value == "upgrade": return "upgrade"
    if value == "unlink": return "unlink"
    if value == "link": return "link"
    if value == "--overwrite": return "--overwrite"
    if value == "install": return "install"
    if value == "uninstall": return "uninstall"
    if value == "untap": return "untap"
    if value == FORMULA_CURRENT: return FORMULA_CURRENT
    if value == FORMULA_VERSIONED: return FORMULA_VERSIONED
    if value == tap_path: return "<workspace-tap>"
    raise SystemExit(2)


def default_state() -> dict[str, bool]:
    return {
        "tap": False,
        "trusted_current": False,
        "trusted_versioned": False,
        "installed_current": False,
        "installed_versioned": False,
        "upgraded": False,
    }


def load_state(path: Path) -> dict[str, bool]:
    if not path.exists():
        return default_state()
    value = json.loads(path.read_text(encoding="utf-8"))
    expected = set(default_state())
    if not isinstance(value, dict) or set(value) != expected:
        raise SystemExit(2)
    if any(type(value[key]) is not bool for key in expected):
        raise SystemExit(2)
    return {key: bool(value[key]) for key in expected}


def formula_kind(value: str) -> str:
    if value == FORMULA_CURRENT:
        return "current"
    if value == FORMULA_VERSIONED:
        return "versioned"
    raise SystemExit(2)


root = Path(os.environ["CLROOM_PACKAGING_FAKE_ROOT"]).resolve(); prefix = root / "prefix"; argv = sys.argv[1:]; scenario = os.environ.get("CLROOM_PACKAGING_SCENARIO", "")
credential_words = ("TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "KEY")
if not argv or Path(os.environ.get("HOMEBREW_PREFIX", "")).resolve() != prefix or os.environ.get("HOME") != str(root / "home") or os.environ.get("PATH") != f"{root / 'poison'}:/usr/bin:/bin:/usr/sbin:/sbin" or any(any(word in key.upper() for word in credential_words) for key in os.environ): raise SystemExit(2)
if scenario == "require_portable_ruby" and not (Path(os.environ["HOMEBREW_REPOSITORY"]) / "Library/Homebrew/vendor/portable-ruby/current/bin/ruby").is_file(): raise SystemExit(2)
if scenario == "require_portable_ruby" and ("HOMEBREW_NO_INSTALL_FROM_API" in os.environ or not any((Path(os.environ["HOMEBREW_CACHE"]) / "api/internal").glob("packages.*.jws.json"))): raise SystemExit(2)
if scenario == "require_rendered_formula":
    formula = Path(os.environ["HOMEBREW_ALLOWED_TAPS"]) / "Formula/clroom-preview.rb"
    if not formula.is_file() or 'url "http://127.0.0.1:49152/clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz"' not in formula.read_text(encoding="utf-8") or 'sha256 "' not in formula.read_text(encoding="utf-8"): raise SystemExit(2)
ledger = root / "ledger.jsonl"; ledger.parent.mkdir(parents=True, exist_ok=True)
safe_argv = [canonical_ledger_token(value, str(root / "tap")) for value in argv]
with ledger.open("a", encoding="utf-8") as out: out.write(json.dumps({"argv": safe_argv}, sort_keys=True, separators=(",", ":")) + "\n")
state_path = root / "state.json"; state = load_state(state_path)
if argv in (["--prefix"], ["--repository"]):
    if scenario == "reported_prefix_mismatch" and argv == ["--prefix"] or scenario == "reported_repository_mismatch" and argv == ["--repository"]: print(root / "live")
    else: print(prefix)
elif argv == ["--cellar"]: print(root / "live" if scenario == "reported_cellar_mismatch" else prefix / "Cellar")
elif argv[:2] == ["trust", "--tap"]: raise SystemExit(2)
elif argv[:2] in (["trust", "--formula"], ["untrust", "--formula"]):
    if len(argv) != 3: raise SystemExit(2)
    kind = formula_kind(argv[2])
    if argv[0] == "trust" and (os.environ.get("HOMEBREW_REQUIRE_TAP_TRUST") != "1" or os.environ.get("HOMEBREW_ALLOWED_TAPS") != str(root / "tap") or scenario == "missing_item_trust"): raise SystemExit(2)
    if kind == "current": state["trusted_current"] = argv[0] == "trust"
    else: state["trusted_versioned"] = argv[0] == "trust"
elif argv[0] == "tap":
    if scenario == "tap_clone_failed":
        print("fatal: local clone failed", file=sys.stderr); raise SystemExit(2)
    if argv[:2] != ["tap", "clroom-local/preview"] or len(argv) != 3 or not Path(argv[2]).resolve().is_relative_to(root) or os.environ.get("HOMEBREW_ALLOWED_TAPS") != argv[2]: raise SystemExit(2)
    state["tap"] = True
elif argv[0] in {"style", "audit", "test", "upgrade", "unlink", "link", "install", "uninstall"}:
    kind = formula_kind(argv[-1])
    trusted = state["trusted_current"] if kind == "current" else state["trusted_versioned"]
    if scenario == "require_native_install_boundary" and argv[0] in {"install", "upgrade", "test"}:
        if os.environ.get("CLROOM_PACKAGING_NETWORK_BOUNDARY") != "homebrew-native-sandbox-loopback-proxy" or "HOMEBREW_AVOID_NESTED_SANDBOXING" in os.environ: raise SystemExit(2)
        if argv[0] == "test": print("Error: metadata network unavailable", file=sys.stderr); raise SystemExit(2)
    if argv[0] in {"install", "upgrade"} and not trusted: raise SystemExit(2)
    if scenario == "install_archive_fetch_failed" and argv[0] == "install":
        print("Error: unsupported formula dependency"); raise SystemExit(2)
    if scenario == "smoke_refusal" and argv[0] == "test":
        print("Error: formula test sandbox failed", file=sys.stderr); raise SystemExit(2)
    if scenario == "style_refusal_cleanup" and argv[0] in {"style", "uninstall"}: raise SystemExit(2)
    if scenario in {"require_loopback_server", "require_native_install_boundary"} and argv[0] == "install":
        body = urllib.request.urlopen("http://127.0.0.1:49152/clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz", timeout=1).read()
        if not body: raise SystemExit(2)
    if argv[0] == "install":
        version = "0.0.1" if kind == "versioned" else ("0.0.2" if state["upgraded"] else "0.0.1")
        cell_name = "clroom-preview@0.0.1" if kind == "versioned" else "clroom-preview"
        if kind == "current": state["installed_current"] = True
        else: state["installed_versioned"] = True
        cell = prefix / "Cellar" / cell_name / version / "bin"; cell.mkdir(parents=True, exist_ok=True)
        payload = b'''#!/bin/sh
if [ "$1" = status ]; then printf 'clroom: command accepted\\n'; exit 0; fi
if [ "$1" = --output ] && [ "$2" = json ] && [ "$3" = status ]; then printf 'OUTPUT_UNSUPPORTED_FOR_COMMAND: status; use human output\\n' >&2; exit 2; fi
exit 2
'''
        name = "clroom"; (cell / name).write_bytes(payload); (cell / name).chmod(0o755); (prefix / "bin").mkdir(parents=True, exist_ok=True); target = prefix / "bin" / name; target.unlink(missing_ok=True); target.symlink_to(cell / name)
    if argv[0] == "upgrade": state["upgraded"] = True
    if argv[0] == "uninstall" and scenario != "partial_uninstall":
        cell_name = "clroom-preview@0.0.1" if kind == "versioned" else "clroom-preview"
        if kind == "current": state["installed_current"] = False
        else: state["installed_versioned"] = False
        import shutil; shutil.rmtree(prefix / "Cellar" / cell_name, ignore_errors=True)
elif argv == ["untap", "clroom-local/preview"]:
    if scenario == "cleanup_failure": raise SystemExit(2)
    state["tap"] = False
else: raise SystemExit(2)
if scenario.startswith("sentinel_") or scenario == "config_mutation":
    key = "config" if scenario == "config_mutation" else scenario.removeprefix("sentinel_").removesuffix("_mutation")
    (root / ("gitconfig" if key == "config" else "sentinel-" + key)).write_text("mutated\n", encoding="utf-8")
state_path.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")
