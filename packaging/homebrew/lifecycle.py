#!/usr/bin/env python3
"""Fail-closed disposable Homebrew lifecycle runner for P07 local evidence."""
from __future__ import annotations
import argparse, json, os, shutil, subprocess, sys
from dataclasses import dataclass
from pathlib import Path

REFUSALS = {"ARTIFACT_MISSING", "ARTIFACT_DIGEST_MISMATCH", "ARTIFACT_METADATA_MISMATCH", "HOST_UNSUPPORTED", "DEPLOYMENT_TARGET_UNKNOWN", "FORMULA_RENDER_REFUSED", "FORMULA_AUDIT_REFUSED", "TAP_TRUST_REFUSED", "INSTALL_REFUSED", "DUAL_NAME_PARITY_REFUSED", "UPGRADE_REFUSED", "ROLLBACK_REFUSED", "UNINSTALL_REFUSED", "CONFIG_MUTATION_REFUSED", "CLEANUP_REFUSED", "LIVE_HOMEBREW_BOUNDARY_REFUSED"}

class LifecycleRefused(Exception):
    def __init__(self, code: str): self.code = code

@dataclass(frozen=True)
class SafeHomebrew:
    root: Path
    prefix: Path
    repository: Path
    cellar: Path
    cache: Path
    user_config: Path
    home: Path
    temp: Path
    brew: Path

def make_paths(root: Path, brew: Path) -> SafeHomebrew:
    prefix = root / "prefix"
    return SafeHomebrew(root, prefix, prefix, prefix / "Cellar", root / "cache", root / "config", root / "home", root / "temp", brew)

def closed_env(paths: SafeHomebrew) -> dict[str, str]:
    for path in (paths.home, paths.cache, paths.user_config, paths.temp, paths.prefix, paths.cellar): path.mkdir(parents=True, exist_ok=True)
    return {"PATH":"/usr/bin:/bin:/usr/sbin:/sbin", "HOME":str(paths.home), "XDG_CONFIG_HOME":str(paths.user_config), "XDG_CACHE_HOME":str(paths.cache), "XDG_DATA_HOME":str(paths.root / "data"), "GIT_CONFIG_GLOBAL":str(paths.root / "gitconfig"), "HOMEBREW_PREFIX":str(paths.prefix), "HOMEBREW_REPOSITORY":str(paths.repository), "HOMEBREW_CELLAR":str(paths.cellar), "HOMEBREW_CACHE":str(paths.cache), "HOMEBREW_TEMP":str(paths.temp), "HOMEBREW_USER_CONFIG_HOME":str(paths.user_config), "HOMEBREW_NO_AUTO_UPDATE":"1", "HOMEBREW_NO_INSTALL_FROM_API":"1", "HOMEBREW_NO_ANALYTICS":"1", "HOMEBREW_NO_INSTALL_CLEANUP":"1", "HOMEBREW_NO_AUTOREMOVE":"1", "HOMEBREW_NO_ASK":"1", "HOMEBREW_REQUIRE_TAP_TRUST":"1", "HOMEBREW_ALLOWED_TAPS":"taskseal-local/preview", "P07_FAKE_ROOT":str(paths.root)}

def call(paths: SafeHomebrew, args: list[str], code: str) -> None:
    result = subprocess.run([sys.executable, str(paths.brew), *args], cwd=paths.root, env=closed_env(paths), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    if result.returncode: raise LifecycleRefused(code)

def preflight(paths: SafeHomebrew) -> None:
    expected = [str(paths.prefix), str(paths.repository), str(paths.cellar)]
    actual = []
    for flag in ("--prefix", "--repository", "--cellar"):
        result = subprocess.run([sys.executable, str(paths.brew), flag], cwd=paths.root, env=closed_env(paths), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False)
        if result.returncode: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
        actual.append(result.stdout.strip())
    if actual != expected: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")

def document(steps: list[dict], cleanup: bool, failure: str | None) -> dict:
    return {"schema_version":"taskseal.p07.homebrew-lifecycle.v1", "evidence_class":"lifecycle-fixture", "qualification":"NOT_QUALIFIED", "steps":steps, "refusal_vocabulary":sorted(REFUSALS), "cleanup_complete":cleanup, "failure_class":failure, "forbidden_actions":{"publication":False,"upload":False,"signing":False,"notarization":False,"provider_requests":False,"external_contact":False,"credential_access":False,"keychain_access":False,"main_mutation":False,"integration":False,"live_homebrew_mutation":False}}

def atomic_write(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(path.name + ".tmp")
    with open(temp, "x", encoding="utf-8") as out: out.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
    os.chmod(temp, 0o600); os.replace(temp, path)

def cleanup(paths: SafeHomebrew, steps: list[dict]) -> bool:
    ok = True
    for args, code, name in [(["untrust", "--formula", "taskseal-local/preview/taskseal-preview@0.0.1"], "TAP_TRUST_REFUSED", "untrust_versioned"), (["untrust", "--formula", "taskseal-local/preview/taskseal-preview"], "TAP_TRUST_REFUSED", "untrust"), (["untap", "taskseal-local/preview"], "CLEANUP_REFUSED", "untap")]:
        try: call(paths, args, code); steps.append({"name":name,"exit":0})
        except LifecycleRefused: ok = False; steps.append({"name":name,"exit":1})
    for path in (paths.prefix, paths.cache, paths.user_config, paths.home, paths.temp): shutil.rmtree(path, ignore_errors=True)
    return ok and all(not path.exists() for path in (paths.prefix, paths.cache, paths.user_config, paths.home, paths.temp))

def fake(args, paths: SafeHomebrew) -> dict:
    steps: list[dict] = []; failure = None
    sequence = [("tap", ["tap","taskseal-local/preview",str(paths.root / "tap")], "TAP_TRUST_REFUSED"), ("item_trust", ["trust","--formula","taskseal-local/preview/taskseal-preview"], "TAP_TRUST_REFUSED"), ("style", ["style","--formula","taskseal-local/preview/taskseal-preview"], "FORMULA_AUDIT_REFUSED"), ("audit", ["audit","--strict","--formula","taskseal-local/preview/taskseal-preview"], "FORMULA_AUDIT_REFUSED"), ("install_n", ["install","taskseal-local/preview/taskseal-preview"], "INSTALL_REFUSED"), ("test", ["test","taskseal-local/preview/taskseal-preview"], "DUAL_NAME_PARITY_REFUSED"), ("upgrade_n_plus_1", ["upgrade","taskseal-local/preview/taskseal-preview"], "UPGRADE_REFUSED"), ("install_versioned", ["trust","--formula","taskseal-local/preview/taskseal-preview@0.0.1"], "TAP_TRUST_REFUSED"), ("rollback_n", ["install","taskseal-local/preview/taskseal-preview@0.0.1"], "ROLLBACK_REFUSED"), ("unlink", ["unlink","taskseal-local/preview/taskseal-preview"], "ROLLBACK_REFUSED"), ("link", ["link","taskseal-local/preview/taskseal-preview@0.0.1"], "ROLLBACK_REFUSED"), ("uninstall", ["uninstall","taskseal-local/preview/taskseal-preview@0.0.1"], "UNINSTALL_REFUSED"), ("uninstall_current", ["uninstall","taskseal-local/preview/taskseal-preview"], "UNINSTALL_REFUSED")]
    try:
        preflight(paths); steps.append({"name":"preflight","exit":0})
        for name, argv, code in sequence:
            if args.inject_failure == name or (args.inject_failure == "upgrade" and name == "upgrade_n_plus_1"): raise LifecycleRefused(code)
            call(paths, argv, code); steps.append({"name":name,"exit":0})
    except LifecycleRefused as exc:
        failure = exc.code
    complete = cleanup(paths, steps)
    if not complete and failure is None: failure = "CLEANUP_REFUSED"
    return document(steps, complete, failure)

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--fake", action="store_true"); parser.add_argument("--fake-brew"); parser.add_argument("--inject-failure"); parser.add_argument("--workspace", required=True); parser.add_argument("--output", required=True); args = parser.parse_args()
    if not args.fake or not args.fake_brew: print("P07_HOMEBREW_LIFECYCLE_REFUSED:INSTALL_REFUSED", file=sys.stderr); return 1
    paths = make_paths(Path(args.workspace).resolve(), Path(args.fake_brew).resolve())
    value = fake(args, paths); atomic_write(Path(args.output), value)
    if value["failure_class"]: print("P07_HOMEBREW_LIFECYCLE_REFUSED:" + value["failure_class"], file=sys.stderr); return 1
    print("P07_HOMEBREW_LIFECYCLE_TEST_PASS"); return 0

if __name__ == "__main__": raise SystemExit(main())
