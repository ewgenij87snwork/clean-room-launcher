#!/usr/bin/env python3
"""Generate deterministic synthetic lifecycle archives, never release artifacts."""
from __future__ import annotations
import argparse, gzip, hashlib, io, json, tarfile
from pathlib import Path

def make(output: Path, version: str) -> str:
    root = f"taskseal-v{version}-aarch64-apple-darwin"; binary = f"#!/bin/sh\nprintf 'fixture {version}\\n'\n".encode()
    files = {"LICENSE": b"fixture license\n", "NOTICE": b"fixture notice\n", "VERSION": f"version={version}\nsource_commit={'1' * 40}\ntarget=aarch64-apple-darwin\nevidence_class=lifecycle-fixture\nqualification=NOT_QUALIFIED\n".encode(), "bin/taskseal": binary, "bin/tseal": binary, "share/doc/taskseal/CHANGELOG.md": b"fixture changelog\n"}
    with open(output, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz, tarfile.open(fileobj=gz, mode="w") as tar:
        for name in sorted(files):
            data = files[name]; info = tarfile.TarInfo(f"{root}/{name}"); info.size = len(data); info.uid = info.gid = info.mtime = 0; info.mode = 0o755 if name.startswith("bin/") else 0o644; tar.addfile(info, io.BytesIO(data))
    return hashlib.sha256(output.read_bytes()).hexdigest()

def pair(directory: Path) -> dict[str, object]:
    directory.mkdir(parents=True, exist_ok=True); records = []
    for version in ("0.0.1", "0.0.2"):
        name = f"taskseal-v{version}-aarch64-apple-darwin.tar.gz"; records.append({"version": version, "filename": name, "sha256": make(directory / name, version), "evidence_class": "lifecycle-fixture"})
    if records[0]["sha256"] == records[1]["sha256"]: raise RuntimeError("fixture archive collision")
    value = {"schema_version": "taskseal.p07.homebrew-fixtures.v1", "archives": records}; (directory / "fixture-archives.json").write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8"); return value

def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--output-dir", required=True); args = parser.parse_args(); pair(Path(args.output_dir)); print("P07_HOMEBREW_FIXTURE_ARCHIVES_PASS"); return 0
if __name__ == "__main__": raise SystemExit(main())
