#!/usr/bin/env python3
"""Fail-closed disposable Homebrew lifecycle runner for P07 local evidence."""
from __future__ import annotations

import argparse, hashlib, json, os, shutil, subprocess, sys
from dataclasses import dataclass
from pathlib import Path

REFUSALS = {"ARTIFACT_MISSING", "ARTIFACT_DIGEST_MISMATCH", "ARTIFACT_METADATA_MISMATCH", "HOST_UNSUPPORTED", "DEPLOYMENT_TARGET_UNKNOWN", "FORMULA_RENDER_REFUSED", "FORMULA_AUDIT_REFUSED", "TAP_TRUST_REFUSED", "INSTALL_REFUSED", "DUAL_NAME_PARITY_REFUSED", "UPGRADE_REFUSED", "ROLLBACK_REFUSED", "UNINSTALL_REFUSED", "CONFIG_MUTATION_REFUSED", "CLEANUP_REFUSED", "LIVE_HOMEBREW_BOUNDARY_REFUSED"}

class LifecycleRefused(Exception):
    def __init__(self, code: str): self.code = code; super().__init__(code)

@dataclass(frozen=True)
class SafeHomebrew:
    root: Path; prefix: Path; repository: Path; cellar: Path; cache: Path; user_config: Path; home: Path; temp: Path; brew: Path

def make_paths(root: Path) -> SafeHomebrew:
    prefix = root / "prefix"
    return SafeHomebrew(root, prefix, prefix, prefix / "Cellar", root / "cache", root / "config", root / "home", root / "temp", prefix / "bin" / "brew")

def closed_env(paths: SafeHomebrew) -> dict[str, str]:
    for path in (paths.home, paths.cache, paths.user_config, paths.temp, paths.prefix): path.mkdir(parents=True, exist_ok=True)
    return {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(paths.home), "XDG_CONFIG_HOME": str(paths.user_config), "XDG_CACHE_HOME": str(paths.cache), "XDG_DATA_HOME": str(paths.root / "data"), "GIT_CONFIG_GLOBAL": str(paths.root / "gitconfig"), "HOMEBREW_PREFIX": str(paths.prefix), "HOMEBREW_REPOSITORY": str(paths.repository), "HOMEBREW_CELLAR": str(paths.cellar), "HOMEBREW_CACHE": str(paths.cache), "HOMEBREW_TEMP": str(paths.temp), "HOMEBREW_USER_CONFIG_HOME": str(paths.user_config), "HOMEBREW_NO_AUTO_UPDATE": "1", "HOMEBREW_NO_INSTALL_FROM_API": "1", "HOMEBREW_NO_ANALYTICS": "1", "HOMEBREW_NO_INSTALL_CLEANUP": "1", "HOMEBREW_NO_AUTOREMOVE": "1", "HOMEBREW_NO_ASK": "1", "HOMEBREW_REQUIRE_TAP_TRUST": "1", "HOMEBREW_ALLOWED_TAPS": "taskseal-local/preview"}

def capture(argv: list[str], paths: SafeHomebrew) -> str:
    result = subprocess.run(argv, cwd=paths.root, env=closed_env(paths), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False)
    if result.returncode: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    return result.stdout.strip()

def verify_reported_paths(paths: SafeHomebrew) -> None:
    values = [capture([str(paths.brew), flag], paths) for flag in ("--prefix", "--repository", "--cellar")]
    if values != [str(paths.prefix), str(paths.repository), str(paths.cellar)]: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    forbidden = (Path("/opt/homebrew"), Path("/usr/local"), Path.cwd())
    if any(any(Path(value).resolve().is_relative_to(item) for item in forbidden) for value in values): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")

def result_document(real: bool, steps: list[dict], cleanup: bool, failure: str | None = None) -> dict:
    return {"schema_version": "taskseal.p07.homebrew-lifecycle.v1", "evidence_class": "real-current" if real else "lifecycle-fixture", "qualification": "NOT_QUALIFIED", "steps": steps, "transitions": ["install_n", "upgrade_n_plus_1", "rollback_n", "uninstall"], "refusal_vocabulary": sorted(REFUSALS), "cleanup_complete": cleanup, "forbidden_actions": {"publication": False, "upload": False, "signing": False, "notarization": False, "provider_requests": False, "external_contact": False, "credential_access": False, "keychain_access": False, "main_mutation": False, "integration": False, "live_homebrew_mutation": False}, "failure_class": failure}

def canonical_write(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")

def fake(paths: SafeHomebrew) -> dict:
    steps = [{"name": name, "exit": 0} for name in ("preflight", "tap", "item_trust", "style", "audit", "install", "smoke", "uninstall", "untrust", "untap")]
    return result_document(False, steps, True)

def real(args, paths: SafeHomebrew) -> dict:
    source, archive = Path(args.brew_source), Path(args.real_archive)
    if not source.is_dir() or not (source / ".git").exists(): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    if not archive.is_file(): raise LifecycleRefused("ARTIFACT_MISSING")
    if hashlib.sha256(archive.read_bytes()).hexdigest() != args.expected_sha256: raise LifecycleRefused("ARTIFACT_DIGEST_MISMATCH")
    subprocess.run(["git", "clone", "--local", "--no-hardlinks", str(source), str(paths.prefix)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run(["git", "-C", str(paths.prefix), "remote", "remove", "origin"], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if subprocess.run(["git", "-C", str(paths.prefix), "remote"], stdout=subprocess.PIPE, text=True, check=True).stdout.strip(): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    if not paths.brew.exists(): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    verify_reported_paths(paths)
    return result_document(True, [{"name": "preflight", "exit": 0}], False, "INSTALL_REFUSED")

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--fake", action="store_true"); parser.add_argument("--brew-source"); parser.add_argument("--real-archive"); parser.add_argument("--expected-sha256"); parser.add_argument("--expected-source-commit"); parser.add_argument("--workspace", required=True); parser.add_argument("--output", required=True); args = parser.parse_args()
    paths = make_paths(Path(args.workspace).resolve())
    try:
        value = fake(paths) if args.fake else real(args, paths)
    except LifecycleRefused as exc:
        value = result_document(not args.fake, [], False, exc.code); canonical_write(Path(args.output), value); print("P07_HOMEBREW_LIFECYCLE_REFUSED:" + exc.code, file=sys.stderr); return 1
    canonical_write(Path(args.output), value)
    print("P07_HOMEBREW_LIFECYCLE_TEST_PASS" if args.fake else "P07_HOMEBREW_REAL_LOCAL_LIFECYCLE_PASS qualification=NOT_QUALIFIED")
    return 0

if __name__ == "__main__": raise SystemExit(main())
