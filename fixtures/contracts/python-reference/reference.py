#!/usr/bin/env python3
"""Privacy-clean stdlib reference for the public TaskSeal v0.1 core contract."""

import hashlib
import json
import sys


def frame(value):
    return len(value).to_bytes(8, "big") + value


def refuse(code):
    print(json.dumps({"status": "refused", "code": code}, separators=(",", ":")))
    return 0


def main():
    request = json.load(sys.stdin)
    layers = request.get("layers")
    if not isinstance(layers, list) or len(layers) != 3 or not all(
        isinstance(value, str) for value in layers
    ):
        return refuse("MISSING_RENDER_LAYER")

    raw = [value.encode("utf-8") for value in layers]
    byte_count = sum(len(value) for value in raw)
    records = request.get("records")
    limits = request.get("limits", {})
    for dimension, measured in (
        ("bytes", byte_count),
        ("records", records),
        ("tokens", byte_count),
    ):
        if not isinstance(measured, int) or measured > limits.get(dimension, -1):
            code = "PROTECTED_BUDGET_EXCEEDED" if request.get("protected") else "BUDGET_EXCEEDED"
            return refuse(code)

    context = b"".join(
        heading + value.rstrip(b"\r\n") + b"\n"
        for heading, value in zip((b"# L0\n", b"# L2\n", b"# L3\n"), raw)
    )
    inputs = sorted(set(request.get("inputs", [])), key=lambda value: value.encode("utf-8"))
    digest_input = bytearray(b"taskseal-generation-v1\0")
    for value in inputs:
        digest_input.extend(frame(value.encode("utf-8")))
    digest_input.append(0xFF)
    digest_input.extend(frame(b"context.md"))
    digest_input.extend(frame(context))
    digest = hashlib.sha256(digest_input).hexdigest()
    manifest = {
        "schema_version": "manifest.v1",
        "inputs": inputs,
        "outputs": ["context.md"],
        "digest": digest,
    }
    print(
        json.dumps(
            {"status": "ok", "context_hex": context.hex(), "manifest": manifest},
            separators=(",", ":"),
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
