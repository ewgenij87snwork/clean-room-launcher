#!/usr/bin/env python3
"""Fail-closed intake for the exact P07 Homebrew preview archive."""
from __future__ import annotations
import argparse, hashlib, json, os, platform, re, subprocess, sys, tarfile, tempfile
from dataclasses import dataclass
from pathlib import Path

VERSION_KEYS = {"version", "source_commit", "rust_toolchain", "target", "rustc", "cargo", "python", "packaging_script_sha256", "archive_profile", "qualification", "signing", "dependencies"}
MACOS = {"10.15": "catalina", "11": "big_sur", "12": "monterey", "13": "ventura", "14": "sonoma", "15": "sequoia", "26": "tahoe"}
REQUIRED = {"LICENSE", "NOTICE", "VERSION", "bin/clroom", "share/doc/clean-room-launcher/CHANGELOG.md"}

class InputRefused(Exception):
    def __init__(self, code): self.code = code; super().__init__(code)

@dataclass(frozen=True)
class ArchiveLayout:
    root: str; members: list[str]; clroom: bytes; version: bytes; sha256: str; size: int

def refuse(code): raise InputRefused(code)
def strict_key_value_lines(data: bytes) -> dict[str, str]:
    try: lines = data.decode("utf-8").splitlines()
    except UnicodeDecodeError: refuse("ARTIFACT_METADATA_MISMATCH")
    out = {}
    for line in lines:
        if line.count("=") != 1:
            refuse("ARTIFACT_METADATA_MISMATCH")
        key, value = line.split("=", 1)
        if not key or not value or key in out: refuse("ARTIFACT_METADATA_MISMATCH")
        out[key] = value
    return out
def parse_version(data: bytes) -> dict[str, str]:
    fields = strict_key_value_lines(data)
    if set(fields) != VERSION_KEYS or not re.fullmatch(r"[0-9a-f]{40}", fields["source_commit"]) or not re.fullmatch(r"[0-9a-f]{64}", fields["packaging_script_sha256"]): refuse("ARTIFACT_METADATA_MISMATCH")
    if fields["target"] != "aarch64-apple-darwin" or fields["qualification"] != "NOT_QUALIFIED" or fields["signing"] != "unsigned-preview-only": refuse("ARTIFACT_METADATA_MISMATCH")
    return fields
def macos_symbol(version: str) -> str:
    major = ".".join(version.split(".")[:2]) if version.startswith("10.") else version.split(".")[0]
    if major not in MACOS: refuse("DEPLOYMENT_TARGET_UNKNOWN")
    return MACOS[major]
def parse_vtool_build(text: str) -> tuple[str, str]:
    match = re.search(r"(?:architecture|arch)\s+(arm64).*?\bminos\s+([0-9]+(?:\.[0-9]+)*)", text, re.S)
    if match is None:
        # vtool does not print architecture on every SDK; artifact lipo supplies it.
        match = re.search(r"\bminos\s+([0-9]+(?:\.[0-9]+)*)", text)
        if match is None: refuse("DEPLOYMENT_TARGET_UNKNOWN")
        minimum = match.group(1); macos_symbol(minimum); return "arm64", minimum
    minimum = match.group(2); macos_symbol(minimum); return "arm64", minimum
def require_host(system: str, machine: str):
    if system != "Darwin" or machine not in {"arm64", "aarch64"}: refuse("HOST_UNSUPPORTED")
def require_macho(architectures: str):
    if architectures.strip() != "arm64": refuse("ARTIFACT_METADATA_MISMATCH")
def safe_rel(name: str) -> str:
    if name.startswith("/") or ".." in Path(name).parts or name == "": refuse("ARTIFACT_METADATA_MISMATCH")
    return name
def verify_exact_digest_and_layout(path: Path, expected: str) -> ArchiveLayout:
    if not path.is_file(): refuse("ARTIFACT_MISSING")
    raw = path.read_bytes(); digest = hashlib.sha256(raw).hexdigest()
    if digest != expected: refuse("ARTIFACT_DIGEST_MISMATCH")
    return verify_archive_members(path, digest, len(raw))
def verify_archive_members(path: Path, digest: str, size: int | None = None) -> ArchiveLayout:
    try:
        with tarfile.open(path, "r:gz") as tar:
            members = [m for m in tar.getmembers() if m.isfile()]
            roots = {safe_rel(m.name).split("/", 1)[0] for m in members}
            if len(roots) != 1: refuse("ARTIFACT_METADATA_MISMATCH")
            root = next(iter(roots)); rels = [m.name[len(root) + 1:] for m in members if m.name.startswith(root + "/")]
            if set(rels) != REQUIRED: refuse("ARTIFACT_METADATA_MISMATCH")
            by_rel = {m.name[len(root) + 1:]: m for m in members}
            for rel, member in by_rel.items():
                if member.mode != (0o755 if rel == "bin/clroom" else 0o644): refuse("ARTIFACT_METADATA_MISMATCH")
            clroom = tar.extractfile(by_rel["bin/clroom"]).read(); version = tar.extractfile(by_rel["VERSION"]).read()
    except (OSError, tarfile.TarError, KeyError): refuse("ARTIFACT_METADATA_MISMATCH")
    parse_version(version)
    return ArchiveLayout(root, sorted(rels), clroom, version, digest, size if size is not None else path.stat().st_size)
def call(*argv):
    return subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=False).stdout
def inspect_macho(binary: bytes) -> tuple[str, str]:
    with tempfile.TemporaryDirectory(prefix="clroom-homebrew-input-") as temp:
        path = Path(temp) / "clroom"; path.write_bytes(binary); os.chmod(path, 0o700)
        if "Mach-O" not in call("/usr/bin/file", str(path)): refuse("ARTIFACT_METADATA_MISMATCH")
        archs = call("/usr/bin/lipo", "-archs", str(path)); require_macho(archs)
        _, minimum = parse_vtool_build(call("/usr/bin/vtool", "-show-build", str(path)))
        return "arm64", minimum
def canonical_write(path: Path, value: dict):
    if path.exists() and path.is_symlink(): refuse("ARTIFACT_METADATA_MISMATCH")
    path.parent.mkdir(parents=True, exist_ok=True); temp = path.with_name(path.name + ".tmp")
    temp.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8"); os.chmod(temp, 0o600); os.replace(temp, path)
def verify_archive(args):
    require_host(platform.system(), platform.machine())
    layout = verify_exact_digest_and_layout(Path(args.archive), args.expected_sha256)
    fields = parse_version(layout.version)
    if fields["source_commit"] != args.expected_source_commit or fields["target"] != args.expected_target: refuse("ARTIFACT_METADATA_MISMATCH")
    macho_arch, minimum = inspect_macho(layout.clroom); symbol = macos_symbol(minimum)
    return {"schema_version":"taskseal.p07.homebrew-input.v1","evidence_class":"real-current","archive":{"filename":Path(args.archive).name,"sha256":layout.sha256,"size":layout.size},"artifact":{"version":fields["version"],"source_commit":fields["source_commit"],"target":fields["target"],"qualification":fields["qualification"],"signing":fields["signing"],"root":layout.root,"members":layout.members,"clroom_sha256":hashlib.sha256(layout.clroom).hexdigest()},"host":{"system":"Darwin","machine":"arm64","macho_arch":macho_arch,"minimum_macos":minimum,"homebrew_symbol":symbol}}
def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--archive", required=True); parser.add_argument("--expected-sha256", required=True); parser.add_argument("--expected-source-commit", required=True); parser.add_argument("--expected-target", required=True); parser.add_argument("--output", required=True); args = parser.parse_args()
    try:
        result = verify_archive(args); canonical_write(Path(args.output), result)
    except InputRefused as exc:
        print("P07_HOMEBREW_INPUT_REFUSED:" + exc.code, file=sys.stderr); return 1
    print("P07_HOMEBREW_INPUT_PASS sha256=%s minimum_macos=%s" % (result["archive"]["sha256"], result["host"]["minimum_macos"])); return 0
if __name__ == "__main__": raise SystemExit(main())
