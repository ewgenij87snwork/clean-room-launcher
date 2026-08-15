#!/usr/bin/env python3
"""Generate deterministic synthetic lifecycle archives, never release artifacts."""
from __future__ import annotations
import gzip, hashlib, io, tarfile
from pathlib import Path

def make(output: Path, version: str) -> str:
    root = f"taskseal-v{version}-aarch64-apple-darwin"
    files = {
        "LICENSE": b"fixture license\n", "NOTICE": b"fixture notice\n",
        "VERSION": f"version={version}\nevidence_class=lifecycle-fixture\nqualification=NOT_QUALIFIED\n".encode(),
        "bin/taskseal": f"fixture-taskseal-{version}\n".encode(), "bin/tseal": f"fixture-taskseal-{version}\n".encode(),
    }
    with open(output, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz, tarfile.open(fileobj=gz, mode="w") as tar:
        for name in sorted(files):
            data = files[name]; info = tarfile.TarInfo(f"{root}/{name}"); info.size = len(data); info.uid = info.gid = info.mtime = 0; info.mode = 0o755 if name.startswith("bin/") else 0o644
            tar.addfile(info, io.BytesIO(data))
    return hashlib.sha256(output.read_bytes()).hexdigest()
