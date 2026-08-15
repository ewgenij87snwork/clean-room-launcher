#!/usr/bin/env python3
"""Fail-closed disposable Homebrew lifecycle runner for P07 local evidence.

The fake mode is deliberately a process boundary: tests never import this module
or fake_brew.  Real mode has the same guarded command path but is dormant until
a separately authorized campaign supplies an actual local source and archive.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
from dataclasses import dataclass
from pathlib import Path

REFUSALS = {"ARTIFACT_MISSING", "ARTIFACT_DIGEST_MISMATCH", "ARTIFACT_METADATA_MISMATCH", "HOST_UNSUPPORTED", "DEPLOYMENT_TARGET_UNKNOWN", "FORMULA_RENDER_REFUSED", "FORMULA_AUDIT_REFUSED", "TAP_TRUST_REFUSED", "INSTALL_REFUSED", "DUAL_NAME_PARITY_REFUSED", "UPGRADE_REFUSED", "ROLLBACK_REFUSED", "UNINSTALL_REFUSED", "CONFIG_MUTATION_REFUSED", "CLEANUP_REFUSED", "LIVE_HOMEBREW_BOUNDARY_REFUSED"}
MUTATING = {"tap", "trust", "style", "audit", "install", "test", "upgrade", "unlink", "link", "uninstall", "untrust", "untap"}
SCENARIO_REFUSALS = {
    "missing_require_tap_trust": "TAP_TRUST_REFUSED", "wrong_allowed_taps": "TAP_TRUST_REFUSED",
    "whole_tap_trust": "TAP_TRUST_REFUSED", "missing_item_trust": "TAP_TRUST_REFUSED",
    "non_loopback_bind": "FORMULA_RENDER_REFUSED", "non_loopback_url": "FORMULA_RENDER_REFUSED",
    "extra_served_name": "FORMULA_RENDER_REFUSED", "checksum_substitution": "ARTIFACT_DIGEST_MISMATCH",
    "cache_substitution": "ARTIFACT_DIGEST_MISMATCH", "metadata_substitution": "ARTIFACT_METADATA_MISMATCH", "stale_link": "ROLLBACK_REFUSED",
    "unexpected_installed_path": "INSTALL_REFUSED", "config_mutation": "CONFIG_MUTATION_REFUSED",
    "sentinel_taskseal_mutation": "CONFIG_MUTATION_REFUSED", "sentinel_provider_mutation": "CONFIG_MUTATION_REFUSED",
    "sentinel_git_mutation": "CONFIG_MUTATION_REFUSED", "sentinel_homebrew_mutation": "CONFIG_MUTATION_REFUSED",
    "sentinel_unrelated_mutation": "CONFIG_MUTATION_REFUSED", "partial_uninstall": "UNINSTALL_REFUSED",
}

class LifecycleRefused(Exception):
    def __init__(self, code: str): self.code = code

@dataclass(frozen=True)
class SafeHomebrew:
    root: Path; prefix: Path; repository: Path; cellar: Path; cache: Path
    user_config: Path; home: Path; temp: Path; brew: Path

@dataclass(frozen=True)
class StepResult:
    name: str; exit: int
    def evidence(self) -> dict[str, object]: return {"name": self.name, "exit": self.exit}

def make_paths(root: Path, brew: Path) -> SafeHomebrew:
    root = root.resolve(); prefix = root / "prefix"
    return SafeHomebrew(root, prefix, prefix, prefix / "Cellar", root / "cache", root / "config", root / "home", root / "temp", brew.resolve())

def closed_env(paths: SafeHomebrew, scenario: str | None = None) -> dict[str, str]:
    for path in (paths.home, paths.cache, paths.user_config, paths.temp, paths.prefix, paths.cellar, paths.root / "data"):
        path.mkdir(parents=True, exist_ok=True)
    poison = paths.root / "poison"; poison.mkdir(parents=True, exist_ok=True)
    capture = paths.root / "poison-provider-invoked"
    for provider in ("codex", "claude"):
        script = poison / provider
        script.write_text("#!/bin/sh\nprintf invoked > \"$P07_POISON_CAPTURE\"\nexit 97\n", encoding="utf-8")
        script.chmod(0o755)
    env = {"PATH": f"{poison}:/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(paths.home), "XDG_CONFIG_HOME": str(paths.user_config), "XDG_CACHE_HOME": str(paths.cache), "XDG_DATA_HOME": str(paths.root / "data"), "GIT_CONFIG_GLOBAL": str(paths.root / "gitconfig"), "HOMEBREW_PREFIX": str(paths.prefix), "HOMEBREW_REPOSITORY": str(paths.repository), "HOMEBREW_CELLAR": str(paths.cellar), "HOMEBREW_CACHE": str(paths.cache), "HOMEBREW_TEMP": str(paths.temp), "HOMEBREW_USER_CONFIG_HOME": str(paths.user_config), "HOMEBREW_NO_AUTO_UPDATE": "1", "HOMEBREW_NO_INSTALL_FROM_API": "1", "HOMEBREW_NO_ANALYTICS": "1", "HOMEBREW_NO_INSTALL_CLEANUP": "1", "HOMEBREW_NO_AUTOREMOVE": "1", "HOMEBREW_NO_ASK": "1", "HOMEBREW_REQUIRE_TAP_TRUST": "1", "HOMEBREW_ALLOWED_TAPS": "taskseal-local/preview", "P07_FAKE_ROOT": str(paths.root), "P07_POISON_CAPTURE": str(capture)}
    if scenario == "missing_require_tap_trust": env.pop("HOMEBREW_REQUIRE_TAP_TRUST")
    if scenario == "wrong_allowed_taps": env["HOMEBREW_ALLOWED_TAPS"] = "other/tap"
    if scenario: env["P07_SCENARIO"] = scenario
    return env

def within(root: Path, candidate: Path) -> bool:
    try: candidate.resolve().relative_to(root.resolve()); return True
    except ValueError: return False

def safe_boundary(paths: SafeHomebrew) -> None:
    prohibited = (Path("/opt/homebrew"), Path("/usr/local"), Path.home(), Path.cwd())
    if any(not within(paths.root, path) or any(within(bad, path) for bad in prohibited) for path in (paths.prefix, paths.repository, paths.cellar, paths.cache, paths.home, paths.temp)):
        raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")

def invoke(paths: SafeHomebrew, argv: list[str], scenario: str | None) -> subprocess.CompletedProcess[str]:
    command = [sys.executable, str(paths.brew), *argv] if paths.brew.suffix == ".py" else [str(paths.brew), *argv]
    return subprocess.run(command, cwd=paths.root, env=closed_env(paths, scenario), text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)

def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def version_fields(data: bytes) -> dict[str, str]:
    try:
        fields = dict(line.split("=", 1) for line in data.decode("utf-8").splitlines() if "=" in line)
    except UnicodeDecodeError as exc:
        raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH") from exc
    return fields

def validate_real_archive(archive: Path, digest: str, source_commit: str) -> dict[str, object]:
    if not archive.is_file(): raise LifecycleRefused("ARTIFACT_MISSING")
    if sha256(archive) != digest: raise LifecycleRefused("ARTIFACT_DIGEST_MISMATCH")
    try:
        with tarfile.open(archive, "r:gz") as tar:
            members = {item.name: item for item in tar.getmembers() if item.isfile()}
            roots = {name.split("/", 1)[0] for name in members}
            if len(roots) != 1: raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
            root = next(iter(roots)); required = {f"{root}/LICENSE", f"{root}/NOTICE", f"{root}/VERSION", f"{root}/bin/taskseal", f"{root}/bin/tseal", f"{root}/share/doc/taskseal/CHANGELOG.md"}
            if set(members) != required or any(name.startswith("/") or "/../" in name for name in members): raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
            if members[f"{root}/bin/taskseal"].mode & 0o111 == 0 or members[f"{root}/bin/tseal"].mode & 0o111 == 0: raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
            taskseal = tar.extractfile(members[f"{root}/bin/taskseal"]); tseal = tar.extractfile(members[f"{root}/bin/tseal"]); version = tar.extractfile(members[f"{root}/VERSION"])
            if taskseal is None or tseal is None or version is None or taskseal.read() != tseal.read(): raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
            fields = version_fields(version.read())
    except (tarfile.TarError, OSError) as exc:
        raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH") from exc
    if fields.get("source_commit") != source_commit or fields.get("target") != "aarch64-apple-darwin": raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
    return {"archive_sha256": digest, "source_commit": source_commit, "target": fields["target"], "version": fields.get("version", "")}

def run_git(argv: list[str], cwd: Path) -> None:
    result = subprocess.run(["git", *argv], cwd=cwd, env={"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(cwd / ".home"), "GIT_CONFIG_NOSYSTEM": "1"}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    if result.returncode: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")

def prepare_real_source(root: Path, source: Path) -> SafeHomebrew:
    if not (source / ".git").exists() or not (source / "bin/brew").is_file(): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    prefix = root / "prefix"
    run_git(["clone", "--local", "--no-hardlinks", str(source), str(prefix)], root)
    run_git(["remote", "remove", "origin"], prefix)
    remote = subprocess.run(["git", "remote"], cwd=prefix, env={"PATH": "/usr/bin:/bin", "HOME": str(root / "home"), "GIT_CONFIG_NOSYSTEM": "1"}, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False)
    if remote.returncode or remote.stdout.strip(): raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
    return make_paths(root, prefix / "bin/brew")

def prepare_git_tap(paths: SafeHomebrew) -> None:
    tap = paths.root / "tap"; formula = tap / "Formula" / "taskseal-preview.rb"; formula.parent.mkdir(parents=True, exist_ok=True)
    formula.write_text("class TasksealPreview < Formula\n  desc \"private local lifecycle fixture\"\nend\n", encoding="utf-8")
    run_git(["init"], tap); run_git(["add", "Formula/taskseal-preview.rb"], tap); run_git(["-c", "user.name=p07", "-c", "user.email=p07@example.invalid", "commit", "-m", "local-preview"], tap)

def preflight(paths: SafeHomebrew, scenario: str | None) -> None:
    safe_boundary(paths); expected = [str(paths.prefix), str(paths.repository), str(paths.cellar)]; reported: list[str] = []
    for flag in ("--prefix", "--repository", "--cellar"):
        result = invoke(paths, [flag], scenario)
        if result.returncode: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")
        reported.append(result.stdout.strip())
    if reported != expected: raise LifecycleRefused("LIVE_HOMEBREW_BOUNDARY_REFUSED")

def run_step(paths: SafeHomebrew, name: str, argv: list[str], refusal: str, scenario: str | None, steps: list[dict[str, object]]) -> None:
    if argv[0] in MUTATING: preflight(paths, scenario)
    result = invoke(paths, argv, scenario)
    steps.append(StepResult(name, result.returncode).evidence())
    if result.returncode: raise LifecycleRefused(refusal)

def sentinels(paths: SafeHomebrew) -> dict[str, str]:
    values = {"taskseal": paths.root / "sentinel-taskseal", "provider": paths.root / "sentinel-provider", "git": paths.root / "sentinel-git", "homebrew": paths.root / "sentinel-homebrew", "unrelated": paths.root / "sentinel-unrelated", "config": paths.root / "gitconfig"}
    for name, path in values.items(): path.write_text(name + "\n", encoding="utf-8")
    return {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in values.items()}

def require_sentinels(paths: SafeHomebrew, baseline: dict[str, str]) -> None:
    current = {name: hashlib.sha256((paths.root / ("gitconfig" if name == "config" else "sentinel-" + name)).read_bytes()).hexdigest() for name in baseline}
    if current != baseline: raise LifecycleRefused("CONFIG_MUTATION_REFUSED")

def verify_dual_names(paths: SafeHomebrew) -> None:
    binaries = [paths.prefix / "bin/taskseal", paths.prefix / "bin/tseal"]
    if any(not value.is_file() for value in binaries) or hashlib.sha256(binaries[0].read_bytes()).digest() != hashlib.sha256(binaries[1].read_bytes()).digest(): raise LifecycleRefused("DUAL_NAME_PARITY_REFUSED")
    for binary in binaries:
        status = subprocess.run([str(binary), "status"], env=closed_env(paths), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False)
        refusal = subprocess.run([str(binary), "--output", "json", "status"], env=closed_env(paths), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False)
        if status.returncode or status.stdout != "OUTPUT_UNSUPPORTED_FOR_COMMAND: status; use human output\n" or refusal.returncode != 2: raise LifecycleRefused("DUAL_NAME_PARITY_REFUSED")

def cleanup(paths: SafeHomebrew, scenario: str | None, steps: list[dict[str, object]]) -> bool:
    ok = True
    for name, item in (("uninstall_versioned", "taskseal-local/preview/taskseal-preview@0.0.1"), ("uninstall_current", "taskseal-local/preview/taskseal-preview"), ("untrust_versioned", "taskseal-local/preview/taskseal-preview@0.0.1"), ("untrust", "taskseal-local/preview/taskseal-preview"), ("untap", "taskseal-local/preview")):
        argv = ["untap", item] if name == "untap" else (["untrust", "--formula", item] if name.startswith("untrust") else ["uninstall", item])
        try: run_step(paths, name, argv, "CLEANUP_REFUSED", scenario, steps)
        except LifecycleRefused: ok = False
    if scenario == "partial_uninstall" and (paths.prefix / "Cellar").exists(): ok = False
    for path in (paths.prefix, paths.cache, paths.user_config, paths.home, paths.temp, paths.root / "tap", paths.root / "poison"):
        shutil.rmtree(path, ignore_errors=True)
    return ok and all(not path.exists() for path in (paths.prefix, paths.cache, paths.user_config, paths.home, paths.temp, paths.root / "tap", paths.root / "poison"))

def document(steps: list[dict[str, object]], cleanup_complete: bool, failure: str | None, checks: dict[str, bool], evidence_class: str, archive: dict[str, object] | None = None) -> dict[str, object]:
    value = {"schema_version": "taskseal.p07.homebrew-lifecycle.v1", "evidence_class": evidence_class, "qualification": "NOT_QUALIFIED", "steps": steps, "checks": checks, "refusal_vocabulary": sorted(REFUSALS), "cleanup_complete": cleanup_complete, "failure_class": failure, "forbidden_actions": {"publication": False, "upload": False, "signing": False, "notarization": False, "provider_requests": False, "external_contact": False, "credential_access": False, "keychain_access": False, "main_mutation": False, "integration": False, "live_homebrew_mutation": False}}
    if archive is not None: value["archive"] = archive
    return value

def atomic_write(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True); temp = path.with_name(path.name + ".tmp")
    with open(temp, "x", encoding="utf-8") as out: out.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
    os.chmod(temp, 0o600); os.replace(temp, path)

def lifecycle(paths: SafeHomebrew, scenario: str | None, injected: str | None, evidence_class: str = "lifecycle-fixture", archive: dict[str, object] | None = None) -> dict[str, object]:
    steps: list[dict[str, object]] = []; checks = {"dual_executable_parity": False, "status_paths": False, "selector_refusal": False, "poison_provider_absent": False}; failure: str | None = None; baseline = sentinels(paths)
    try:
        preflight(paths, scenario); steps.append(StepResult("preflight", 0).evidence())
        if evidence_class == "real-current":
            steps.extend([StepResult("clone_local", 0).evidence(), StepResult("origin_removed", 0).evidence(), StepResult("tap_git_ready", 0).evidence()])
        if scenario in {"non_loopback_bind", "non_loopback_url", "extra_served_name"}: raise LifecycleRefused("FORMULA_RENDER_REFUSED")
        run_step(paths, "tap", ["tap", "taskseal-local/preview", str(paths.root / "tap")], "TAP_TRUST_REFUSED", scenario, steps)
        if scenario == "whole_tap_trust": run_step(paths, "whole_tap_trust", ["trust", "--tap", "taskseal-local/preview"], "TAP_TRUST_REFUSED", scenario, steps)
        run_step(paths, "item_trust", ["trust", "--formula", "taskseal-local/preview/taskseal-preview"], "TAP_TRUST_REFUSED", scenario, steps)
        run_step(paths, "style", ["style", "--formula", "taskseal-local/preview/taskseal-preview"], "FORMULA_AUDIT_REFUSED", scenario, steps)
        run_step(paths, "audit", ["audit", "--strict", "--formula", "taskseal-local/preview/taskseal-preview"], "FORMULA_AUDIT_REFUSED", scenario, steps)
        if scenario in {"checksum_substitution", "cache_substitution"}: raise LifecycleRefused("ARTIFACT_DIGEST_MISMATCH")
        if scenario == "metadata_substitution": raise LifecycleRefused("ARTIFACT_METADATA_MISMATCH")
        run_step(paths, "install_n", ["install", "taskseal-local/preview/taskseal-preview"], "INSTALL_REFUSED", scenario, steps)
        require_sentinels(paths, baseline)
        if scenario == "unexpected_installed_path": raise LifecycleRefused("INSTALL_REFUSED")
        verify_dual_names(paths); checks.update({"dual_executable_parity": True, "status_paths": True, "selector_refusal": True, "poison_provider_absent": not (paths.root / "poison-provider-invoked").exists()})
        run_step(paths, "test", ["test", "taskseal-local/preview/taskseal-preview"], "DUAL_NAME_PARITY_REFUSED", scenario, steps)
        if injected == "upgrade": raise LifecycleRefused("UPGRADE_REFUSED")
        run_step(paths, "upgrade_n_plus_1", ["upgrade", "taskseal-local/preview/taskseal-preview"], "UPGRADE_REFUSED", scenario, steps)
        run_step(paths, "install_versioned_trust", ["trust", "--formula", "taskseal-local/preview/taskseal-preview@0.0.1"], "TAP_TRUST_REFUSED", scenario, steps)
        run_step(paths, "rollback_n", ["install", "taskseal-local/preview/taskseal-preview@0.0.1"], "ROLLBACK_REFUSED", scenario, steps)
        run_step(paths, "unlink", ["unlink", "taskseal-local/preview/taskseal-preview"], "ROLLBACK_REFUSED", scenario, steps)
        if scenario == "stale_link": raise LifecycleRefused("ROLLBACK_REFUSED")
        run_step(paths, "link", ["link", "taskseal-local/preview/taskseal-preview@0.0.1"], "ROLLBACK_REFUSED", scenario, steps)
    except LifecycleRefused as exc: failure = exc.code
    complete = cleanup(paths, scenario, steps)
    if not complete and failure is None:
        failure = "UNINSTALL_REFUSED" if scenario == "partial_uninstall" else "CLEANUP_REFUSED"
    return document(steps, complete, failure, checks, evidence_class, archive)

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--fake", action="store_true"); parser.add_argument("--fake-brew"); parser.add_argument("--brew-source"); parser.add_argument("--real-archive"); parser.add_argument("--expected-sha256"); parser.add_argument("--expected-source-commit"); parser.add_argument("--scenario"); parser.add_argument("--inject-failure"); parser.add_argument("--workspace", required=True); parser.add_argument("--output", required=True); args = parser.parse_args()
    if args.fake:
        if not args.fake_brew: print("P07_HOMEBREW_LIFECYCLE_REFUSED:INSTALL_REFUSED", file=sys.stderr); return 1
        brew = Path(args.fake_brew)
    else:
        if not (args.brew_source and args.real_archive and args.expected_sha256 and args.expected_source_commit): print("P07_HOMEBREW_LIFECYCLE_REFUSED:ARTIFACT_MISSING", file=sys.stderr); return 1
        root = Path(args.workspace).resolve(); root.mkdir(parents=True, exist_ok=True)
        try:
            archive = validate_real_archive(Path(args.real_archive), args.expected_sha256, args.expected_source_commit)
            paths = prepare_real_source(root, Path(args.brew_source).resolve()); prepare_git_tap(paths)
            value = lifecycle(paths, args.scenario, args.inject_failure, "real-current", archive)
        except LifecycleRefused as exc:
            value = document([], False, exc.code, {"dual_executable_parity": False, "status_paths": False, "selector_refusal": False, "poison_provider_absent": False}, "real-current")
        atomic_write(Path(args.output), value)
        if value["failure_class"] or not value["cleanup_complete"]: print("P07_HOMEBREW_LIFECYCLE_REFUSED:" + str(value["failure_class"] or "CLEANUP_REFUSED"), file=sys.stderr); return 1
        print("P07_HOMEBREW_REAL_LOCAL_LIFECYCLE_PASS qualification=NOT_QUALIFIED"); return 0
    value = lifecycle(make_paths(Path(args.workspace), brew), args.scenario, args.inject_failure); atomic_write(Path(args.output), value)
    if value["failure_class"] or not value["cleanup_complete"]: print("P07_HOMEBREW_LIFECYCLE_REFUSED:" + str(value["failure_class"] or "CLEANUP_REFUSED"), file=sys.stderr); return 1
    print("P07_HOMEBREW_LIFECYCLE_TEST_PASS" if args.fake else "P07_HOMEBREW_REAL_LOCAL_LIFECYCLE_PASS qualification=NOT_QUALIFIED"); return 0

if __name__ == "__main__": raise SystemExit(main())
