#!/usr/bin/env python3
"""Fail-closed verifier for locally generated TaskSeal preview archives."""
import gzip, hashlib, os, re, sys, tarfile

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
            if rel in ("bin/taskseal", "bin/tseal"):
                if not m.isfile(): fail("binary is not a regular file")
        rel_set = set(rels)
        if not REQUIRED.issubset(rel_set): fail("missing required member")
        if any(rel.startswith("bin/") and rel not in ("bin/taskseal", "bin/tseal") for rel in rels): fail("wrong binary name")
        taskseal = tar.extractfile(root + "/bin/taskseal").read()
        tseal = tar.extractfile(root + "/bin/tseal").read()
        if taskseal != tseal: fail("tseal is not byte-identical")
        version = tar.extractfile(root + "/VERSION").read().decode("utf-8")
        if "qualification=NOT_QUALIFIED\n" not in version or "source_commit=" not in version: fail("unbound qualification metadata")
        for field in ("notice_generator_sha256", "license_policy_sha256", "notice_policy_sha256", "cargo_lock_sha256"):
            if not re.search(rf"^{field}=[0-9a-f]{{64}}$", version, re.MULTILINE): fail("unbound notice metadata")
        notice = tar.extractfile(root + "/NOTICE").read().decode("utf-8")
        if not notice.startswith("Third-party dependency notices\n"): fail("dependency notice identity")
        count = re.search(r"^Component count: ([1-9][0-9]*)$", notice, re.MULTILINE)
        if count is None or notice.count("\n- ") != int(count.group(1)): fail("dependency notice component census")
        if not re.search(r"^Cargo\.lock SHA-256: [0-9a-f]{64}$", notice, re.MULTILINE): fail("dependency notice lock binding")
        if any(marker in notice for marker in ("/Users/", "/home/", "Documents/it", "ghp_", "sk-")): fail("dependency notice private data")
except (tarfile.TarError, OSError, UnicodeError) as exc:
    fail(str(exc))
print("ARTIFACT_VALID sha256=" + hashlib.sha256(open(archive, "rb").read()).hexdigest())
