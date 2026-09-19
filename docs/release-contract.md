# Release contract

CLROOM treats a release as the complete change from the **last published stable
GitHub Release** to an exact candidate, not as only the headline feature or the
latest pull request.

The public technical release contract has three machine-readable pieces:

- `release/technical-release-contract.json` — path/domain classification and
  contract-expansion triggers;
- `release/reviews/vX.Y.Z.json` — the release-specific disposition of every
  triggered contract question;
- `scripts/release/check-release-delta.py` — fail-closed validation and a
  deterministic whole-release report.

Any changed file that does not map to a known domain blocks release readiness
with `UNCLASSIFIED_RELEASE_DELTA`. Every triggered domain must have exactly one
release-review disposition: `CONTRACT_EXPAND`, `COVERED`,
`NOT_APPLICABLE`, or `FOLLOW_UP`, with repository evidence.

This is intentionally a technical contract. Private planning, prompts, Owner
reasoning, and strategic control do not belong in the public repository.

## Local release audit

From a clean candidate checkout with GitHub CLI authentication:

```sh
scripts/release/local-release-audit.sh
```

The script resolves the last published stable GitHub Release, reports every
commit and changed file since that release, validates the release contract,
runs canonical release readiness, builds the candidate archive, lists its exact
contents, and prints its checksum. It does not tag, publish, install, modify
provider state, or make a model request.

To audit against an explicitly known published baseline without GitHub API
resolution:

```sh
CLROOM_RELEASE_BASELINE_TAG=v0.3.1 scripts/release/local-release-audit.sh
```

The release workflow independently resolves the last published GitHub Release
again before accepting a tag. A tag alone is not treated as proof that a version
was published.
