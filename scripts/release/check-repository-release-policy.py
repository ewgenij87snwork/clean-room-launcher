#!/usr/bin/env python3
"""Verify the external GitHub tag ruleset required by the CLROOM release contract."""

from __future__ import annotations

import json
import os
import subprocess

REPO = os.environ.get("GITHUB_REPOSITORY", "y-sor/clean-room-launcher")
TAG_PATTERN = "refs/tags/v*"


def fail(message: str) -> None:
    raise SystemExit("RELEASE_REPOSITORY_POLICY_BLOCKED:" + message)


def gh_json(endpoint: str):
    proc = subprocess.run(
        ["gh", "api", endpoint],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if proc.returncode != 0:
        fail("github-api:" + proc.stderr.strip())
    try:
        return json.loads(proc.stdout)
    except ValueError as exc:
        fail(f"github-json:{exc}")


def main() -> None:
    rulesets = gh_json(f"repos/{REPO}/rulesets")
    if not isinstance(rulesets, list):
        fail("rulesets-response")

    candidates = [
        item
        for item in rulesets
        if isinstance(item, dict)
        and item.get("target") == "tag"
        and item.get("enforcement") == "active"
    ]
    if not candidates:
        fail("active-tag-ruleset-missing")

    qualifying = []
    for summary in candidates:
        ruleset_id = summary.get("id")
        if not isinstance(ruleset_id, int):
            continue
        detail = gh_json(f"repos/{REPO}/rulesets/{ruleset_id}")
        includes = detail.get("conditions", {}).get("ref_name", {}).get("include", [])
        rule_types = {
            rule.get("type")
            for rule in detail.get("rules", [])
            if isinstance(rule, dict)
        }
        bypass = detail.get("bypass_actors")
        if (
            TAG_PATTERN in includes
            and {"update", "deletion"}.issubset(rule_types)
            and isinstance(bypass, list)
            and not bypass
        ):
            qualifying.append(detail)

    if len(qualifying) != 1:
        fail(f"exact-protective-tag-ruleset-count:{len(qualifying)}")

    ruleset = qualifying[0]
    print(
        "RELEASE_REPOSITORY_POLICY_PASS "
        f"repo={REPO} tag_ruleset={ruleset.get('id')} pattern={TAG_PATTERN}"
    )


if __name__ == "__main__":
    main()
