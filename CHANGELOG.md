# Changelog

All notable changes to Clean Room Launcher will be documented in this file.

The format is based on Keep a Changelog, and this project intends to use
Semantic Versioning after the first public release.

## [Unreleased]

## [0.2.0] - 2026-09-07

### Added

- Qualified Codex exec launches with clean user configuration, provider-aware
  selected-skill inventory, and fail-closed filesystem restrictions.
- Canonical release readiness and SCA verification with SHA256SUMS, CycloneDX
  SBOM, and GitHub attestations.
- Checksum-verified one-line macOS Apple Silicon installation from the current
  stable GitHub Release, without `sudo` or shell-configuration mutation.

### Security

- Qualified for macOS on Apple Silicon; release artifacts remain unsigned and
  unnotarized.

## [0.1.0-alpha.4.2] - 2026-08-24

### Fixed

- Tag CI exposed a concurrent reaper/owner cleanup race; cleanup is idempotent
  only for an absent generated session leaf beneath the exact validated private
  layout, while unsafe ancestors and leaves remain fail-closed.

## [0.1.0-alpha.4.1] - 2026-08-24

### Changed

- Revalidated provider executable identity and version immediately before each
  launch, with a closed allowlisted parent environment and truthful launch
  status when the clean filesystem restrictions cannot be established.
- Hardened path, symlink, and selected-skill filesystem checks for the qualified
  macOS provider paths.

### Fixed

- Hardened Claude session projection ownership, stale cleanup, process exit,
  and signal handling; concurrent cleanup is idempotent only for a missing
  residue and remains fail-closed for other errors.

### Security

- This patch release adds no new operating-system or provider qualification:
  the supported claim remains macOS on Apple Silicon with Codex and Claude.

## [0.1.0-alpha.4] - 2026-08-23

### Added

- A `clroom claude` launch path with the same explicit one-launch skill
  selection used by Codex.
- Private, session-scoped Claude skill projections with normal-exit cleanup,
  proven-dead crash reaping, and parallel-session isolation.
- A Claude-specific launch plaque covering global instructions, selected global
  skills, user settings, auto memory, and project-local skills.

### Changed

- The project-skills card supports are centered beneath both provider plaques.
- Help, install guidance, provider support, and limitations now describe both
  qualified macOS provider paths.

### Fixed

- Abrupt terminal closure no longer creates indefinitely accumulating Claude
  projections: the next launch removes only residues whose owner is proven dead.

### Security

- Claude projections and owner markers use private modes, selected source skills
  remain read-only, and live, unknown, corrupt, or legacy state is never reaped.
- Linux and Windows remain explicitly `NOT_QUALIFIED`; this release makes no
  cross-platform isolation claim beyond macOS on Apple Silicon.
