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

def input_contract():
    return {
        "schema_version": "taskseal.p07.homebrew-input.v1", "evidence_class": "real-current",
        "archive": {"filename": "taskseal-v0.1.0-aarch64-apple-darwin.tar.gz", "sha256": "6" * 64, "size": 42},
        "artifact": {"version": "0.1.0", "source_commit": "01ad1d894aabe265b08d61d67d39da1a29cad9e4", "target": "aarch64-apple-darwin", "qualification": "NOT_QUALIFIED", "signing": "unsigned-preview-only", "root": "taskseal-v0.1.0-aarch64-apple-darwin", "members": ["LICENSE", "NOTICE", "VERSION", "bin/taskseal", "bin/tseal", "share/doc/taskseal/CHANGELOG.md"], "taskseal_sha256": "a" * 64, "tseal_sha256": "a" * 64},
        "host": {"system": "Darwin", "machine": "arm64", "macho_arch": "arm64", "minimum_macos": "13.0", "homebrew_symbol": "ventura"},
    }

def run_formula():
    spec = importlib.util.spec_from_file_location("render_formula", ROOT / "packaging/homebrew/render_formula.py")
    if spec is None or spec.loader is None:
        raise AssertionError("renderer missing")
    renderer = importlib.util.module_from_spec(spec); sys.modules[spec.name] = renderer; spec.loader.exec_module(renderer)
    contract = input_contract()
    url = "http://127.0.0.1:49152/taskseal-v0.1.0-aarch64-apple-darwin.tar.gz"
    with tempfile.TemporaryDirectory() as temp:
        output = Path(temp) / "Formula" / "taskseal-preview.rb"
        first = renderer.render(contract, "taskseal-preview", url)
        second = renderer.render(contract, "taskseal-preview", url)
        assert first == second and b"class TasksealPreview < Formula" in first
        assert b"depends_on arch:" not in first
        assert b"depends_on macos: :ventura" in first
        assert b"keg_only :versioned_formula" not in first
        versioned = renderer.render(contract, "taskseal-preview@0.0.1", url)
        assert b"class TasksealPreviewAT001 < Formula" in versioned
        assert b"keg_only :versioned_formula" in versioned
        renderer.atomic_write(output, first)
        assert output.read_bytes() == first and (output.stat().st_mode & 0o777) == 0o644
        assert b"post_install" not in first and b'system "curl"' not in first
        assert b"https://taskseal-preview.invalid/" in first and b"127.0.0.1:49152" in first
        assert b"provider" not in first.lower() and b"login" not in first.lower()
        assert __import__("subprocess").run(["ruby", "-c", str(output)], stdout=__import__("subprocess").PIPE, stderr=__import__("subprocess").PIPE).returncode == 0
        for bad_url in ["https://127.0.0.1:49152/taskseal-v0.1.0-aarch64-apple-darwin.tar.gz", "http://localhost:49152/taskseal-v0.1.0-aarch64-apple-darwin.tar.gz", "http://127.0.0.1:49152/other.tar.gz", "http://user@127.0.0.1:49152/taskseal-v0.1.0-aarch64-apple-darwin.tar.gz", url + "?x=1"]:
            expect_formula_refusal(lambda bad_url=bad_url: renderer.render(contract, "taskseal-preview", bad_url), renderer)
        for bad_id in ["taskseal", "taskseal-preview; system('x')", "taskseal-preview@bad"]:
            expect_formula_refusal(lambda bad_id=bad_id: renderer.render(contract, bad_id, url), renderer)
        malformed = dict(contract); malformed["extra"] = True
        expect_formula_refusal(lambda: renderer.render(malformed, "taskseal-preview", url), renderer)
        unknown = json.loads(json.dumps(contract)); unknown["host"]["homebrew_symbol"] = "unknown"
        expect_formula_refusal(lambda: renderer.render(unknown, "taskseal-preview", url), renderer)
        link = Path(temp) / "link.rb"; link.symlink_to(output)
        expect_formula_refusal(lambda: renderer.atomic_write(link, first), renderer)

def expect_formula_refusal(fn, renderer):
    try:
        fn()
    except renderer.FormulaRefused as exc:
        assert exc.code == "FORMULA_RENDER_REFUSED"
    else:
        raise AssertionError("expected FORMULA_RENDER_REFUSED")

def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--section", required=True); args = parser.parse_args()
    if args.section == "input": run_input(); run_archive()
    elif args.section == "formula": run_formula(); print("P07_HOMEBREW_FORMULA_TEST_PASS"); return
    else: raise SystemExit("unknown section")
    print("P07_HOMEBREW_INPUT_TEST_PASS")

if __name__ == "__main__": main()
