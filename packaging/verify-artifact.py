#!/usr/bin/env python3
"""Fail-closed verifier for locally generated TaskSeal preview archives."""
import gzip, hashlib, os, sys, tarfile

REQUIRED = {"LICENSE", "NOTICE", "VERSION", "bin/taskseal", "bin/tseal", "share/doc/taskseal/CHANGELOG.md"}
def fail(message):
    print("ARTIFACT_INVALID: " + message, file=sys.stderr); raise SystemExit(1)
if len(sys.argv) != 2: fail("usage: verify-artifact.py ARCHIVE")
archive = sys.argv[1]
try:
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        if not members: fail("empty archive")
        roots = {m.name.split("/", 1)[0] for m in members}
        if len(roots) != 1: fail("multiple archive roots")
        root = next(iter(roots))
        rels = [m.name[len(root)+1:] for m in members if m.name != root]
        if rels != sorted(rels, key=lambda x: (x.count("/"), x)):
            fail("member order is not canonical")
        for m, rel in zip([m for m in members if m.name != root], rels):
            if m.name.startswith("/") or ".." in m.name.split("/"): fail("path traversal")
            if m.uid != 0 or m.gid != 0 or m.mtime != 0: fail("non-normalized ownership or timestamp")
            expected_mode = 0o755 if m.isdir() or rel in ("bin/taskseal", "bin/tseal") else 0o644
            if m.mode != expected_mode: fail("non-normalized mode")
            if rel.endswith("/taskseal") or rel.endswith("/tseal"):
                if not m.isfile(): fail("binary is not a regular file")
        rel_set = set(rels)
        if not REQUIRED.issubset(rel_set): fail("missing required member")
        if any(rel.startswith("bin/") and rel not in ("bin/taskseal", "bin/tseal") for rel in rels): fail("wrong binary name")
        taskseal = tar.extractfile(root + "/bin/taskseal").read()
        tseal = tar.extractfile(root + "/bin/tseal").read()
        if taskseal != tseal: fail("tseal is not byte-identical")
        version = tar.extractfile(root + "/VERSION").read().decode("utf-8")
        if "qualification=NOT_QUALIFIED\n" not in version or "source_commit=" not in version: fail("unbound qualification metadata")
except (tarfile.TarError, OSError, UnicodeError) as exc:
    fail(str(exc))
print("ARTIFACT_VALID sha256=" + hashlib.sha256(open(archive, "rb").read()).hexdigest())
