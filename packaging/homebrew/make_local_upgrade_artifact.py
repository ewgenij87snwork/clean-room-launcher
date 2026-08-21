#!/usr/bin/env python3
"""Deterministically repack an accepted local artifact as a lifecycle-only N+1."""
from __future__ import annotations
import argparse, gzip, hashlib, io, re, tarfile
from pathlib import Path

VERSION = re.compile(r"[0-9]+(?:\.[0-9]+){2}\Z")

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True); parser.add_argument("--version", required=True); parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if not VERSION.fullmatch(args.version): raise SystemExit("P07_LOCAL_UPGRADE_REFUSED:VERSION")
    source, output = Path(args.source), Path(args.output)
    with tarfile.open(source, "r:gz") as archive:
        files = {item.name: (item, archive.extractfile(item).read()) for item in archive.getmembers() if item.isfile()}
    roots = {name.split("/", 1)[0] for name in files}
    if len(roots) != 1: raise SystemExit("P07_LOCAL_UPGRADE_REFUSED:LAYOUT")
    old_root = next(iter(roots)); new_root = re.sub(r"-v[0-9]+(?:\.[0-9]+){2}-", f"-v{args.version}-", old_root)
    if new_root == old_root: raise SystemExit("P07_LOCAL_UPGRADE_REFUSED:LAYOUT")
    version_name = old_root + "/VERSION"
    lines = files[version_name][1].decode("utf-8").splitlines()
    values = dict(line.split("=", 1) for line in lines)
    values["version"] = args.version
    values["archive_profile"] = "normalized-local-lifecycle-upgrade"
    version_bytes = ("\n".join(f"{key}={values[key]}" for key in [line.split("=", 1)[0] for line in lines]) + "\n").encode()
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as zipped, tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as target:
        for old_name in sorted(files):
            old, data = files[old_name]
            name = new_root + old_name[len(old_root):]
            if old_name == version_name: data = version_bytes
            info = tarfile.TarInfo(name); info.size = len(data); info.mode = old.mode; info.uid = info.gid = 0; info.uname = info.gname = ""; info.mtime = 0
            target.addfile(info, io.BytesIO(data))
    print("P07_LOCAL_UPGRADE_ARTIFACT_PASS sha256=" + hashlib.sha256(output.read_bytes()).hexdigest())
    return 0

if __name__ == "__main__": raise SystemExit(main())
