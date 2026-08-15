#!/usr/bin/env python3
"""Render the closed, private P07 local Homebrew Formula."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import urllib.parse
from pathlib import Path

ALLOWED_FORMULA = re.compile(r"taskseal-preview(?:@[0-9]+(?:\.[0-9]+){1,2})?\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
VERSION = re.compile(r"[0-9]+(?:\.[0-9]+){1,2}\Z")
ARCHIVE = re.compile(r"taskseal-v[0-9]+(?:\.[0-9]+){1,2}-aarch64-apple-darwin\.tar\.gz\Z")
MACOS = {"catalina", "big_sur", "monterey", "ventura", "sonoma", "sequoia", "tahoe"}

class FormulaRefused(Exception):
    def __init__(self, code: str = "FORMULA_RENDER_REFUSED"):
        self.code = code
        super().__init__(code)

def refuse() -> None:
    raise FormulaRefused()

def load_contract(path: Path) -> dict:
    try:
        def no_duplicates(items):
            answer = {}
            for key, value in items:
                if key in answer:
                    refuse()
                answer[key] = value
            return answer
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=no_duplicates)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        refuse()
    return validate_contract(value)

def validate_contract(value: object) -> dict:
    if not isinstance(value, dict) or set(value) != {"schema_version", "evidence_class", "archive", "artifact", "host"}:
        refuse()
    archive, artifact, host = value["archive"], value["artifact"], value["host"]
    if value["schema_version"] != "taskseal.p07.homebrew-input.v1" or value["evidence_class"] != "real-current":
        refuse()
    if not isinstance(archive, dict) or set(archive) != {"filename", "sha256", "size"}:
        refuse()
    if not isinstance(artifact, dict) or set(artifact) != {"version", "source_commit", "target", "qualification", "signing", "root", "members", "taskseal_sha256", "tseal_sha256"}:
        refuse()
    if not isinstance(host, dict) or set(host) != {"system", "machine", "macho_arch", "minimum_macos", "homebrew_symbol"}:
        refuse()
    if not isinstance(archive["filename"], str) or not ARCHIVE.fullmatch(archive["filename"]):
        refuse()
    if not isinstance(archive["sha256"], str) or not HEX64.fullmatch(archive["sha256"]) or not isinstance(archive["size"], int) or archive["size"] < 1:
        refuse()
    for key in ("taskseal_sha256", "tseal_sha256"):
        if not isinstance(artifact[key], str) or not HEX64.fullmatch(artifact[key]):
            refuse()
    if artifact["taskseal_sha256"] != artifact["tseal_sha256"] or not isinstance(artifact["version"], str) or not VERSION.fullmatch(artifact["version"]):
        refuse()
    if artifact["target"] != "aarch64-apple-darwin" or artifact["qualification"] != "NOT_QUALIFIED" or artifact["signing"] != "unsigned-preview-only":
        refuse()
    if host["system"] != "Darwin" or host["machine"] != "arm64" or host["macho_arch"] != "arm64" or host["homebrew_symbol"] not in MACOS:
        refuse()
    if not isinstance(host["minimum_macos"], str) or not VERSION.fullmatch(host["minimum_macos"]):
        refuse()
    return value

def validate_loopback_url(value: str, archive_name: str) -> str:
    parsed = urllib.parse.urlsplit(value)
    if parsed.scheme != "http" or parsed.hostname != "127.0.0.1" or parsed.username or parsed.password or parsed.query or parsed.fragment or parsed.port is None or not (1 <= parsed.port <= 65535) or parsed.path != "/" + archive_name:
        refuse()
    return value

def ruby_class_name(formula_id: str) -> str:
    if not ALLOWED_FORMULA.fullmatch(formula_id):
        refuse()
    if "@" not in formula_id:
        return "TasksealPreview"
    version = formula_id.split("@", 1)[1].replace(".", "")
    return "TasksealPreviewAT" + version

def render(contract: dict, formula_id: str, url: str) -> bytes:
    contract = validate_contract(contract)
    archive, artifact, host = contract["archive"], contract["artifact"], contract["host"]
    validate_loopback_url(url, archive["filename"])
    klass = ruby_class_name(formula_id)
    text = f'''class {klass} < Formula
  desc "Private local lifecycle fixture for an unsigned TaskSeal preview"
  homepage "https://taskseal-preview.invalid/"
  url "{url}"
  version "{artifact["version"]}"
  sha256 "{archive["sha256"]}"
  depends_on arch: :arm64
  depends_on macos: :{host["homebrew_symbol"]}

  def install
    bin.install "bin/taskseal"
    bin.install "bin/tseal"
    (share/"taskseal-preview").install "LICENSE", "NOTICE", "VERSION"
  end

  test do
    taskseal_status = shell_output("#{'{'}bin{'}'}/taskseal status")
    tseal_status = shell_output("#{'{'}bin{'}'}/tseal status")
    assert_equal taskseal_status, tseal_status
    expected = "OUTPUT_UNSUPPORTED_FOR_COMMAND: status; use human output\\n"
    assert_equal expected, shell_output("#{'{'}bin{'}'}/taskseal --output json status 2>&1", 2)
    assert_equal expected, shell_output("#{'{'}bin{'}'}/tseal --output json status 2>&1", 2)
  end
end
'''
    return text.encode("utf-8")

def atomic_write(path: Path, value: bytes) -> None:
    if path.exists() and path.is_symlink():
        refuse()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    if temporary.exists() and temporary.is_symlink():
        refuse()
    try:
        with open(temporary, "xb") as handle:
            handle.write(value)
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
    except FileExistsError:
        refuse()

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-contract", required=True)
    parser.add_argument("--formula-id", required=True)
    parser.add_argument("--artifact-url", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        rendered = render(load_contract(Path(args.input_contract)), args.formula_id, args.artifact_url)
        atomic_write(Path(args.output), rendered)
    except FormulaRefused as exc:
        print("P07_HOMEBREW_FORMULA_REFUSED:" + exc.code, file=sys.stderr)
        return 1
    print("P07_HOMEBREW_FORMULA_PASS sha256=" + hashlib.sha256(rendered).hexdigest())
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
