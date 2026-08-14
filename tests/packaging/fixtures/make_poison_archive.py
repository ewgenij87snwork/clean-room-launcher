import io, os, sys, tarfile, gzip

kind, output = sys.argv[1:]
root = "taskseal-v0.1.0"
payload = {"LICENSE": b"license\n", "NOTICE": b"notice\n", "VERSION": b"source_commit=abc\nqualification=NOT_QUALIFIED\n", "bin/taskseal": b"same", "bin/tseal": b"same", "share/doc/taskseal/CHANGELOG.md": b"change\n"}
if kind == "traversal": payload["../outside"] = b"bad"
elif kind == "wrong-name": payload["bin/not-taskseal"] = payload.pop("bin/taskseal")
elif kind == "missing-license": payload.pop("LICENSE")
elif kind == "metadata": pass
else: raise SystemExit("unknown poison kind")
items = [(root + "/" + name, data) for name, data in payload.items()]
if kind == "metadata": items[0] = (items[0][0], items[0][1])
with open(output, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz, tarfile.open(fileobj=gz, mode="w") as tar:
    for name, data in items:
        info = tarfile.TarInfo(name); info.size = len(data); info.uid = info.gid = 0; info.mtime = 123 if kind == "metadata" else 0; info.mode = 0o644
        tar.addfile(info, io.BytesIO(data))
