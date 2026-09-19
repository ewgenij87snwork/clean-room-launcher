# Release governance

CLROOM treats a release as the complete delta from the **latest published stable
GitHub Release** to the exact commit that is proposed for the new tag. A feature
PR, release-prep PR, tag, or branch name is not a substitute for that full
release boundary.

## Why

Release risk is cumulative. Runtime changes, dependency updates, CI changes,
documentation claims, discovery metadata, installer changes, and release-system
changes can all enter `main` between two published versions. The release process
must therefore prove what is actually shipping, not what the team remembers
working on.

This contract follows four industry principles:

1. secure-development requirements are part of the lifecycle, not a final
   afterthought;
2. release integrity and provenance are verifiable and retained;
3. machine-enforceable policy is preferred to convention;
4. every release feeds new failure modes and near-misses back into the release
   contract.

## Canonical release boundary

For every release candidate:

1. resolve the latest **published** stable GitHub Release;
2. resolve its tag to the exact commit;
3. compare that commit to the prospective tag HEAD;
4. classify every changed file using `packaging/release-contract.json`;
5. fail closed on any unclassified path;
6. derive the union of required gates from the observed change classes;
7. require the versioned review manifest under `release/reviews/` to match the
   observed classes and gates exactly;
8. run all machine gates;
9. separately complete the private strategic-alignment review against the
   Owner's canonical Prime Directive;
10. require fresh Owner gates for tag and publication.

A new file or component surface that does not fit the current taxonomy is not
silently accepted. It blocks release until the contract is intentionally
expanded or the change is removed.

## Contract evolution gate

Every release must answer: **did this release, its near-misses, upstream changes,
or its distribution path reveal a new class of release risk that the current
contract does not model?**

- If **no**, the versioned release review records `contract_review=confirmed`.
- If **yes**, update the release contract and its tests first, then record
  `contract_review=expanded`.

A failure discovered during release work is therefore not only fixed in product
code. When the failure represents a reusable class of risk, the release system
must gain a durable gate that prevents recurrence.

## Evidence layers

Machine evidence includes locked tests, security negatives, CodeQL, dependency
review/SCA, public-boundary checks, installer self-test, artifact metadata,
SBOM/provenance, baseline real-provider canaries, and the full-delta audit.

Capabilities that cannot be honestly reproduced on hosted CI remain explicit
manual release gates. For v0.4.0, exact draft-artifact Claude plugin activation
on Claude Code 2.1.273 is such a gate: it is performed against the bytes produced
by the tag workflow before publication.

## Promotion model

`accepted main → tag gate → exact tag workflow → guarded Draft Release → draft
artifact verification → publish gate → immutable published release → public
install verification`.

The tag workflow must never publish automatically. Publication is a separate
Owner decision after the draft artifacts and release identity are reviewed.

Published release assets are treated as immutable supply-chain outputs.
