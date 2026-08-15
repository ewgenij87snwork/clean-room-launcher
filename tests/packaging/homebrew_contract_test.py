#!/usr/bin/env python3
"""Focused, dependency-free P07 Homebrew contract tests."""
import argparse
import hashlib
import importlib.util
import io
import json
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("verify_input", ROOT / "packaging/homebrew/verify_input.py")
if SPEC is None or SPEC.loader is None:
    raise SystemExit("P07_HOMEBREW_INPUT_TEST_REFUSED:IMPLEMENTATION_MISSING")
mod = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mod
SPEC.loader.exec_module(mod)

def expect_refusal(fn, expected):
    try:
        fn()
    except mod.InputRefused as exc:
        assert exc.code == expected, (exc.code, expected)
    else:
        raise AssertionError("expected refusal " + expected)

def version(**changes):
    fields = {
        "version": "0.1.0", "source_commit": "01ad1d894aabe265b08d61d67d39da1a29cad9e4",
        "rust_toolchain": "1.97.1", "target": "aarch64-apple-darwin", "rustc": "rustc 1.97.1",
        "cargo": "cargo 1.97.1", "python": "Python 3.11.15", "packaging_script_sha256": "a" * 64,
        "archive_profile": "normalized-local-toolchain", "qualification": "NOT_QUALIFIED",
        "signing": "unsigned-preview-only", "dependencies": "cargo-lock",
    }
    fields.update(changes)
    return "".join(f"{key}={value}\n" for key, value in fields.items()).encode()

def run_input():
    assert mod.parse_version(version())["target"] == "aarch64-apple-darwin"
    expect_refusal(lambda: mod.parse_version(b"version=0.1.0\nversion=0.1.0\n"), "ARTIFACT_METADATA_MISMATCH")
    expect_refusal(lambda: mod.parse_version(version(qualification="PASS")), "ARTIFACT_METADATA_MISMATCH")
    arm64 = (ROOT / "tests/packaging/fixtures/homebrew/vtool-arm64.txt").read_text()
    assert mod.parse_vtool_build(arm64) == ("arm64", "13.0")
    unknown = (ROOT / "tests/packaging/fixtures/homebrew/vtool-unknown.txt").read_text()
    expect_refusal(lambda: mod.parse_vtool_build(unknown), "DEPLOYMENT_TARGET_UNKNOWN")
    assert mod.macos_symbol("13.0") == "ventura"
    expect_refusal(lambda: mod.macos_symbol("16"), "DEPLOYMENT_TARGET_UNKNOWN")
    expect_refusal(lambda: mod.require_host("Linux", "x86_64"), "HOST_UNSUPPORTED")
    expect_refusal(lambda: mod.require_macho("x86_64"), "ARTIFACT_METADATA_MISMATCH")

def archive(path, equal=True):
    data = b"mach-o-test"; alias = data if equal else b"different"
    with tarfile.open(path, "w:gz") as tar:
        for name, body, mode in [
            ("taskseal-v0.1.0-aarch64-apple-darwin/bin/taskseal", data, 0o755),
            ("taskseal-v0.1.0-aarch64-apple-darwin/bin/tseal", alias, 0o755),
            ("taskseal-v0.1.0-aarch64-apple-darwin/LICENSE", b"license", 0o644),
            ("taskseal-v0.1.0-aarch64-apple-darwin/NOTICE", b"notice", 0o644),
            ("taskseal-v0.1.0-aarch64-apple-darwin/VERSION", version(), 0o644),
            ("taskseal-v0.1.0-aarch64-apple-darwin/share/doc/taskseal/CHANGELOG.md", b"changes", 0o644),
        ]:
            info = tarfile.TarInfo(name); info.size = len(body); info.mode = mode
            tar.addfile(info, io.BytesIO(body))

def run_archive():
    with tempfile.TemporaryDirectory() as temp:
        path = Path(temp) / "fixture.tar.gz"; archive(path)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        layout = mod.verify_exact_digest_and_layout(path, digest)
        assert layout.root == "taskseal-v0.1.0-aarch64-apple-darwin"
        expect_refusal(lambda: mod.verify_exact_digest_and_layout(path, "0" * 64), "ARTIFACT_DIGEST_MISMATCH")
        bad = Path(temp) / "bad.tar.gz"; archive(bad, equal=False)
        expect_refusal(lambda: mod.verify_archive_members(bad, hashlib.sha256(bad.read_bytes()).hexdigest()), "DUAL_NAME_PARITY_REFUSED")

def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--section", required=True); args = parser.parse_args()
    if args.section == "input": run_input(); run_archive()
    elif args.section == "formula":
        # Renderer cases become available in Task 2 while keeping this one focused suite.
        print("P07_HOMEBREW_FORMULA_TEST_PENDING")
        return
    else: raise SystemExit("unknown section")
    print("P07_HOMEBREW_INPUT_TEST_PASS")

if __name__ == "__main__": main()
