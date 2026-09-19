---
layout: page
title: Release process
description: How CLROOM reviews, qualifies, builds, attests, and publishes a release.
permalink: /release-process.html
---

# Release process

CLROOM treats a release as a product and security decision, not only a successful build.

## Release base

Every release review starts from the **last published GitHub Release**, resolves its exact tag/commit, and reviews the complete delta from that release to the candidate. The previous PR or previous `main` checkpoint is not an acceptable substitute.

The machine-readable release assessment lives in:

- `release/contract.toml` — durable release control classes and evidence requirements;
- `release/reviews/vX.Y.Z.toml` — the candidate's declared change classes, capabilities, contract-evolution decisions, and evidence obligations.

`scripts/release/check-release-contract.py` fails closed if the published base is wrong, a changed path is unclassified, the release assessment does not match the actual delta, release identity/date disagree, or required evidence is omitted.

## Contract evolution

Each release must ask whether the delta, a failure, or a near-miss exposed a class of behavior or risk not covered by the durable release contract.

The release assessment must record one of:

- `EXTEND_CONTRACT:` — add a durable control before release;
- `ALREADY_COVERED:` — identify the existing control that catches the class;
- `DEFER_REMOVE:` — do not ship that behavior;
- `OWNER_EXCEPTION:` — bounded exceptional risk acceptance outside the normal path.

A one-time manual discovery is not considered a durable fix unless the class is converted into a reusable check, negative test, evidence requirement, or explicit fail-closed boundary.

## Behavior-specific evidence

Evidence is not transferable between unrelated behaviors. Baseline provider startup does not qualify a new provider capability. Artifact provenance proves where bytes came from; it does not prove that a feature works.

The v0.4.0 assessment therefore separates:

- Codex and Claude baseline clean-launch qualification;
- Claude 2.1.273 skill-only plugin activation qualification;
- post-tag exact draft-artifact evidence;
- artifact checksum/SBOM/provenance verification;
- installer smoke verification.

## Pipeline

1. Pull-request CI validates the full published-release delta contract.
2. Release readiness reruns locked tests, security/dependency controls, provider baseline canaries, artifact verification, SBOM/provenance generation, and installer checks.
3. The accepted tag must point to the exact accepted default-branch tip.
4. Tag CI rebuilds and verifies the exact release artifact and creates a guarded **Draft** GitHub Release.
5. Required post-tag evidence is verified against the exact draft artifact.
6. Publication is a separate human/Owner gate.
7. After publication, the public installer/download path is verified.

## Local review

With GitHub CLI authenticated:

```sh
git fetch --tags origin
CLROOM_LATEST_PUBLISHED_TAG="$(gh api repos/y-sor/clean-room-launcher/releases/latest --jq .tag_name)" \
  python3 scripts/release/check-release-contract.py --version 0.4.0 --head HEAD
```

For the full local release-readiness gate on the qualified macOS host, use the repository's pinned provider provisioning and `scripts/release/readiness.sh`.

## External framework alignment

This project uses the framework ideas rather than claiming certification:

- NIST Secure Software Development Framework (SSDF), SP 800-218;
- OpenSSF Open Source Project Security Baseline (OSPS Baseline);
- SLSA build provenance concepts;
- GitHub artifact attestations / Sigstore verification.

These references do not replace CLROOM's own threat model or evidence.
