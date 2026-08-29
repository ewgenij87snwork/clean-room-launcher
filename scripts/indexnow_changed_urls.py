#!/usr/bin/env python3
"""Find canonical Pages URLs changed in a push and optionally notify IndexNow."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

BASE_URL = "https://ewgenij87snwork.github.io/clean-room-launcher"
HOST = "ewgenij87snwork.github.io"
INDEXNOW_ENDPOINT = "https://api.indexnow.org/indexnow"
EXCLUDED = {"llms.txt"}
SITE_WIDE_PREFIXES = ("docs/_config.yml", "docs/_includes/", "docs/_layouts/", "docs/_sass/")


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True)


def frontmatter(root: Path, revision: str, path: str) -> str | None:
    try:
        body = git(root, "show", f"{revision}:{path}")
    except subprocess.CalledProcessError:
        return None
    if not body.startswith("---\n"):
        return None
    end = body.find("\n---\n", 4)
    if end < 0:
        return None
    for line in body[4:end].splitlines():
        if line.startswith("permalink:"):
            value = line.split(":", 1)[1].strip().strip('"\'')
            return value
    return None


def public_url(permalink: str, base_url: str) -> str | None:
    if not permalink.startswith("/"):
        return None
    url = base_url.rstrip("/") + (permalink if permalink != "/" else "/")
    if not url.startswith(base_url.rstrip("/") + "/"):
        return None
    return url


def canonical_pages(root: Path, revision: str, base_url: str) -> set[str]:
    paths = git(root, "ls-tree", "-r", "--name-only", revision, "docs").splitlines()
    result = set()
    for path in paths:
        if not path.endswith(".md") or Path(path).name in EXCLUDED:
            continue
        permalink = frontmatter(root, revision, path)
        if permalink:
            url = public_url(permalink, base_url)
            if url:
                result.add(url)
    return result


def changed_urls(root: Path, before: str, after: str, base_url: str, bootstrap: bool = False) -> list[str]:
    if bootstrap:
        return sorted(canonical_pages(root, after, base_url))
    rows = git(root, "diff", "--name-status", before, after, "--", "docs").splitlines()
    urls: set[str] = set()
    site_wide = False
    for row in rows:
        fields = row.split("\t")
        status, path = fields[0], fields[-1]
        if any(path.startswith(prefix) for prefix in SITE_WIDE_PREFIXES):
            site_wide = True
            continue
        if not path.endswith(".md") or Path(path).name in EXCLUDED:
            continue
        revision = before if status.startswith("D") else after
        permalink = frontmatter(root, revision, path)
        if permalink:
            url = public_url(permalink, base_url)
            if url:
                urls.add(url)
    if site_wide:
        urls.update(canonical_pages(root, after, base_url))
    return sorted(urls)


def get_key(key_file: Path) -> str:
    key = key_file.read_text(encoding="utf-8")
    normalized = key.rstrip("\r\n")
    if normalized != key.rstrip("\r\n") or not normalized:
        raise ValueError("invalid key file")
    if any(ch not in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-" for ch in normalized):
        raise ValueError("invalid key characters")
    return normalized


def request(url: str, method: str = "GET", payload: bytes | None = None) -> tuple[int, bytes]:
    request_obj = urllib.request.Request(url, data=payload, method=method)
    if payload is not None:
        request_obj.add_header("Content-Type", "application/json; charset=utf-8")
    try:
        with urllib.request.urlopen(request_obj, timeout=30) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--before", required=True)
    parser.add_argument("--after", required=True)
    parser.add_argument("--key-file", type=Path, required=True)
    parser.add_argument("--bootstrap", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    key_file = args.key_file if args.key_file.is_absolute() else root / args.key_file
    key = get_key(key_file)
    key_name = key_file.name.removesuffix(".txt")
    key_url = f"{BASE_URL}/{key_name}.txt"
    urls = changed_urls(root, args.before, args.after, BASE_URL, args.bootstrap)
    if not urls:
        print("NO_INDEXABLE_URL_CHANGES")
        return 0
    print(f"INDEXABLE_URL_COUNT={len(urls)}")
    for url in urls:
        print(f"INDEXABLE_URL={url}")
    if args.dry_run:
        return 0
    status, body = request(key_url)
    if status != 200 or body.decode("utf-8").rstrip("\r\n") != key:
        print(f"KEY_PREFLIGHT_FAILED_HTTP={status}", file=sys.stderr)
        return 1
    payload = json.dumps({"host": HOST, "key": key, "keyLocation": key_url, "urlList": urls}).encode()
    status, _ = request(INDEXNOW_ENDPOINT, "POST", payload)
    print(f"INDEXNOW_HTTP={status}")
    if status not in (200, 202):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
