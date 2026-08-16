# Threat model

## Scope and protected assets

This model covers local launch preparation, provider process birth, generated
context, release artifacts and evidence. Protected assets are provider
authentication state, user and project files, Git state, source skills,
generated runtime state, artifact identity and release receipts.

## Trust boundaries and attacker-controlled inputs

Trust boundaries exist between the parent environment and the isolated runtime,
between source skills and generated context, between TaskSeal and a provider
CLI, and between a built artifact and its release evidence. Attacker-controlled
inputs include project files, skill metadata, command arguments, environment
variables, archive members, paths and symlinks, provider output and edited
receipts.

## Threats

- HOME contamination can expose parent configuration, unrelated skills or
  private files to a supposedly clean launch.
- Path and symlink escape can redirect placement, reads, writes or cleanup
  outside TaskSeal-owned runtime roots.
- Malicious context can use admitted files or skill metadata to influence a
  provider outside the intended task boundary.
- Adapter or provider drift can make evidence from one executable, version,
  operating system or architecture appear valid for another tuple.
- Private-data leakage can place secrets, absolute user paths, prompts or
  transcripts in source exports, artifacts, logs, SBOMs or receipts.
- Digest or receipt forgery can detach a successful observation from the bytes,
  process, state baseline or cleanup result that produced it.
- Unsafe archive entries or process construction can escape extraction roots or
  introduce shell interpretation.
- Incomplete cleanup can leave generated context or launcher-owned state that
  affects a later provider start.

## Mitigations

- Resolve executable, version, OS, architecture, source commit and artifact
  digests before provider birth; unsupported or stale tuples remain
  `NOT_QUALIFIED`.
- Build an allowlisted environment, refuse collisions before placement, reject
  symlinks and path traversal, and clean only digest-bound TaskSeal-owned state.
- Preserve provider-native skill loading and record only bounded observations;
  never retain raw provider output, prompts, credentials or transcripts.
- Require normalized archives, locked dependencies, checksums, SBOM and
  provenance bound to one artifact digest, plus deterministic receipt subjects.
- Scan public source inputs separately from immutable internal gate evidence and
  keep planted negative fixtures outside promotion evidence.

## Residual risks

- The current candidate is unsigned and has not been independently installed by
  an external user or separate external machine.
- Public repository, package and command namespaces are not reserved or owned.
- A verified private security-reporting route and enforceable CODEOWNERS mapping
  do not exist until the public repository is created by the owner.
- Provider behavior may change after the exact observed version; evidence never
  transfers automatically to a new tuple.
