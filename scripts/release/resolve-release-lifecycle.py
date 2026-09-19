#!/usr/bin/env python3
import argparse
import re


CANDIDATE_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-rc\.(\d+))?$")
PUBLISHED_TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


def resolve(candidate_version: str, published_tag: str) -> str:
    candidate = CANDIDATE_RE.fullmatch(candidate_version)
    published = PUBLISHED_TAG_RE.fullmatch(published_tag)
    if candidate is None:
        raise ValueError("CANDIDATE_VERSION")
    if published is None:
        raise ValueError("PUBLISHED_TAG")

    published_version = published_tag[1:]
    if candidate_version == published_version:
        return "POST_PUBLISH"

    candidate_core = tuple(int(candidate.group(i)) for i in range(1, 4))
    published_core = tuple(int(published.group(i)) for i in range(1, 4))
    if candidate_core > published_core:
        return "ACTIVE_CANDIDATE"

    raise ValueError("CANDIDATE_NOT_ADVANCED")


def self_test() -> None:
    assert resolve("0.4.0", "v0.4.0") == "POST_PUBLISH"
    assert resolve("0.4.1", "v0.4.0") == "ACTIVE_CANDIDATE"
    assert resolve("0.5.0", "v0.4.0") == "ACTIVE_CANDIDATE"
    assert resolve("0.4.1-rc.1", "v0.4.0") == "ACTIVE_CANDIDATE"

    blocked = (
        ("0.3.9", "v0.4.0"),
        ("0.4.0-rc.1", "v0.4.0"),
        ("0.4.0", "0.4.0"),
        ("next", "v0.4.0"),
    )
    for candidate, published in blocked:
        try:
            resolve(candidate, published)
        except ValueError:
            continue
        raise SystemExit(
            f"RELEASE_LIFECYCLE_SELF_TEST_FAIL candidate={candidate} published={published}"
        )
    print("RELEASE_LIFECYCLE_SELF_TEST_PASS")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate-version")
    parser.add_argument("--published-tag")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return
    if not args.candidate_version or not args.published_tag:
        parser.error("--candidate-version and --published-tag are required")

    try:
        print(resolve(args.candidate_version, args.published_tag))
    except ValueError as exc:
        raise SystemExit(f"RELEASE_LIFECYCLE_BLOCKED:{exc}") from exc


if __name__ == "__main__":
    main()
