# CLROOM Release Contract v1

This document defines the public, executable release contract for Clean Room
Launcher. It is a release-safety contract, not a claim of certification or
formal compliance.

The private Owner/GPT permission flow remains outside the public repository.
This document describes product-facing evidence and fail-closed release
conditions only.

## Goals

A CLROOM release is acceptable only when all of these are true:

1. The complete delta since the previous published release has been reviewed.
2. The exact release candidate source is accepted and current.
3. Security, dependency, documentation, packaging, and release-workflow changes
   are represented in the changelog and public support claims.
4. Current real Codex and Claude versions on the Owner's macOS Apple Silicon
   machine match the exact qualification pins.
5. Provider behavior is exercised manually on the Owner machine.
6. Real-provider automated qualification runs against binaries extracted from
   the release archive, never only against sibling build outputs.
7. A tag is created only after accepted-main evidence is current.
8. The Draft Release is verified before publication, including an Owner-machine
   smoke of the exact draft archive bytes.
9. Publication is a separate explicit gate.
10. Post-publication installation is verified from the public release surface.

## Release state machine

```text
WHOLE-RELEASE REVIEW
  -> PROVIDER REFRESH
  -> RELEASE CANDIDATE ACCEPT
  -> MERGE
  -> ACCEPTED-MAIN LOCAL ARTIFACT SMOKE
  -> TAG GATE
  -> TAG WORKFLOW
  -> DRAFT RELEASE
  -> EXACT DRAFT-ASSET LOCAL SMOKE
  -> DRAFT ACCEPT
  -> PUBLISH GATE
  -> PUBLISH
  -> PUBLIC INSTALL VERIFY
```

No later state implies an earlier one. In particular:

- merged != tagged;
- tagged != draft verified;
- draft verified != published;
- published != public install verified.

## 1. Whole-release review

Review from the previous published release tag to the candidate, not only the
last feature PR.

At minimum inspect:

- every commit;
- every changed file;
- runtime/source changes;
- dependency and lockfile changes;
- CI/release workflow changes;
- packaging/installer changes;
- security-policy changes;
- documentation/public claims;
- changelog coverage;
- provider qualification pins.

Use:

```sh
scripts/release/review-release-delta.sh <previous-tag> [candidate-ref]
```

Material changes absent from the release notes are a release blocker.

## 2. Provider freshness and version truth

Codex and Claude are fast-moving external providers. Stable CLROOM releases
therefore require a fresh Owner-machine check immediately before tag approval.

The public qualification source is `release/qualification.json`. The code,
provider canary provisioning, qualification verification, README, SECURITY,
CHANGELOG, and relevant docs must agree with it.

Run:

```sh
python3 scripts/release/check-provider-version-sync.py
scripts/release/local-release-smoke.sh pretag \
  --plugin-id <qualified-installed-claude-plugin-id>
```

The pre-tag smoke reads the currently installed real provider versions.

If either installed provider version differs from the exact pins, stop. Do not
reinterpret an older qualification as current. Update the exact pins, canary
packages/integrity values, code constants, and all public version claims in a
reviewed PR; rerun CI/release readiness; then rerun the local smoke.

For a stable release, the Claude clean-launch and plugin-activation paths must
both be manually exercised on the exact Claude version claimed by that release.

The smoke must never install, update, downgrade, enable, or disable a provider
or provider plugin. Provider maintenance is an explicit Owner action outside the
release script.

## 3. Release candidate freeze

After release-candidate acceptance, scope is frozen. Only release blockers may
change the candidate:

- correctness;
- security/privacy;
- integration/provider compatibility;
- build/CI/release;
- false public claims;
- broken first-use/install path.

Any source HEAD change invalidates exact-head review and all evidence bound to
the old HEAD.

## 4. Artifact binding

The release archive is the product that users receive.

Automated real-provider qualification MUST execute `clroom-codex` and
`clroom-claude` extracted from the created archive. Qualifying
`target/.../release/*` beside the archive is insufficient release evidence.

The archive must remain bound to:

- release version;
- source commit;
- target;
- Cargo.lock;
- packaging policy;
- NOTICE/SBOM/provenance inputs.

## 5. Accepted-main pre-tag local smoke

Because CLROOM uses squash merge, PR-HEAD bytes are not sufficient evidence for
the final source identity. After merge and post-merge verification, run the
pre-tag local smoke again on accepted `main`.

Required evidence:

- exact accepted-main SHA;
- archive SHA-256;
- macOS Apple Silicon;
- real Codex version;
- real Claude version;
- clean provider startup;
- interactive Codex TUI startup without a model request;
- interactive Claude TUI startup without a model request;
- bounded Claude selected-plugin startup using an already-installed,
  qualification-eligible skill-only plugin;
- selected plugin absent from clean launch and present from selected launch;
- no newly admitted sibling plugin;
- zero plugin load errors;
- persistent provider configuration unchanged.

A failure is a release blocker. A provider-version mismatch returns to the
provider-refresh step.

## 6. Tag gate

Tagging is a separate one-shot release action.

Immediately before tag creation verify:

- accepted `main` has not moved;
- all required checks are green on the exact accepted source;
- pre-tag local evidence is PASS and bound to that source;
- provider versions still match the qualification pins;
- the tag does not already exist;
- changelog date and version identity are correct.

Tag identity is version-first: `vX.Y.Z`.

Protected release tags must not be mutable or deletable through normal project
operation.

## 7. Draft Release gate

The tag workflow builds, verifies, attests, and creates a Draft Release.

Before publish, run the local smoke against the exact Draft Release archive:

```sh
scripts/release/local-release-smoke.sh draft \
  --tag vX.Y.Z \
  --plugin-id <same-qualified-installed-claude-plugin-id>
```

This phase downloads the draft assets with authenticated GitHub CLI, verifies
checksums and attestations when available, extracts the exact archive, and
repeats provider/manual startup checks against those exact public-candidate
bytes.

This is the final protection against a difference between source/build evidence
and the bytes that would be published.

## 8. Publish gate

Publishing is a separate Owner action after Draft verification.

Before publish verify:

- tag, annotated tag message when present, package version, changelog version,
  and GitHub Release title agree;
- Release title starts with the version token;
- expected assets are complete;
- SHA256SUMS passes;
- provenance and SBOM attestations verify;
- exact draft-asset local smoke is PASS;
- release notes match the final whole-release delta;
- the release is still draft/unpublished.

Prefer immutable releases for published CLROOM releases. Draft all assets first,
then publish once.

## 9. Post-publish verification

After publish verify from the public user path, not the workspace:

- `releases/latest` resolves to the intended version;
- public `install.sh` downloads successfully;
- checksums/attestations verify;
- a clean temporary install produces `clroom --version` for the published
  version;
- at least one no-model provider startup succeeds from the installed public
  bytes.

If an immutable published release is wrong, do not rewrite its tag/assets.
Prepare a corrective release and communicate the defect.

## Evidence invalidation

| Change | Evidence invalidated |
| --- | --- |
| source HEAD | exact-head review, build, local smoke, tag readiness |
| Cargo.lock/dependency | SCA, SBOM, build, local smoke |
| packaging/release workflow | artifact, provenance, draft verification |
| provider version/binary | provider qualification, local smoke, version docs |
| qualification pins/docs | version-sync gate, release notes review |
| tag target | tag workflow, draft evidence |
| draft artifact digest/assets | checksums, attestations, draft local smoke |
| published asset/tag identity | corrective release required; do not silently mutate |

Evidence from an invalidated state MUST NOT be carried forward as PASS.

## Evidence storage

Public repository:
- this contract;
- qualification pins;
- executable review/smoke/sync scripts;
- CI/release gates.

Private control plane:
- Owner gates;
- exact accepted source;
- evidence summaries/digests;
- per-release decision ledger;
- no raw provider transcripts, credentials, machine paths, or private logs.

Google Drive planning:
- human-readable planning and continuity handoff;
- not a permission source and not product SSOT.

GitHub Draft/Release:
- archive;
- installer;
- SHA256SUMS;
- CycloneDX SBOM;
- provenance/SBOM Sigstore bundles;
- release notes.

## Minimal release evidence record

A private per-release ledger should record only sanitized facts:

- release version;
- previous published tag;
- accepted source SHA;
- whole-release review PASS;
- CI/release-readiness run identities;
- artifact SHA-256;
- Codex tested version;
- Claude tested version;
- selected benign plugin ID used for activation smoke;
- pre-tag local smoke PASS timestamp;
- tag SHA;
- Draft Release asset digest;
- draft local smoke PASS timestamp;
- publish approval;
- published release identity;
- public install verification.

Never store raw provider session IDs, prompts, transcripts, credentials, home
paths, or unrestricted environment dumps.

## Design references

These are design references, not compliance claims:

- NIST SSDF, especially release archival/provenance and secure testing;
- SLSA build provenance;
- OpenSSF Scorecard branch protection, pinned build dependencies, token
  permissions, vulnerability and CI checks;
- GitHub artifact attestations, protected tags/rulesets, Draft Releases, and
  immutable releases;
- Google SRE release engineering: repeatable processes, canarying, and recovery.
