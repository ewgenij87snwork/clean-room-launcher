# Changelog

All notable changes to Clean Room Launcher will be documented in this file.

The format is based on Keep a Changelog, and this project intends to use
Semantic Versioning after the first public release.

## [Unreleased]

## [0.1.0-alpha.4.2] - 2026-08-24

### Fixed

- Tag CI exposed a concurrent reaper/owner cleanup race; cleanup is idempotent
  only for an absent generated session leaf beneath the exact validated private
  layout, while unsafe ancestors and leaves remain fail-closed.

## [0.1.0-alpha.4.1] - 2026-08-24

### Changed

- Revalidated provider executable identity and version immediately before each
  launch, with a closed allowlisted parent environment and truthful launch
  status when the clean boundary cannot be established.
- Hardened path, symlink, and selected-skill boundary checks for the qualified
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

## [0.1.0-alpha.3] - 2026-08-22

### Added

- `clroom --version` and `clroom -V`, with the package version also visible in
  help and on the launch plaque.
- A conditional plaque card reporting valid project-local skills that remain
  available to Codex.

### Changed

- Help and the pre-launch review use a concise, adaptive presentation.
- The plaque reflects explicit user overrides for apps, hooks and plugins.

### Fixed

- Native Codex skill discovery can enumerate known roots while unselected skill
  contents remain outside the launch boundary.
- Duplicate selected global skills resolve once using Codex root precedence.

### Security

- Discovery access is limited to root metadata and listing; unselected skill
  bodies remain unreadable.

## [0.1.0-alpha.2] - 2026-08-22

### Added

- One `--skill-set=` option for exact global skills, whole namespaces, exact
  `namespace:skill` selectors, reusable named `@sets`, and mixed selections.
- User-owned skill sets from `$XDG_CONFIG_HOME/clroom/skill-sets.yaml` or
  `~/.config/clroom/skill-sets.yaml`; Clean Room Launcher reads this file only
  when an `@set` is requested and never creates or rewrites it.

### Changed

- Project-local skills remain automatic while unselected global skills stay
  outside each launch.
- The launch plaque reports how many global skills were admitted.

### Security

- Unknown, malformed, nested, unsafe-path, and ambiguous selectors fail before
  Codex starts.

## [0.1.0-alpha.1] - 2026-08-21

### Added

- `clroom codex [ARGS...]` for the locally installed Codex CLI on macOS/Apple
  Silicon.
- A macOS Seatbelt boundary that blocks global Codex instructions and known
  ambient skill roots while retaining project access.
- Clean launch defaults for apps, hooks, plugins, developer instructions and
  notifications, with explicit user arguments retaining final priority.
- Deterministic unsigned macOS/arm64 archive and `SHA256SUMS` verification.

### Changed

- The pre-launch screen is a compact boundary status plaque that reports the
  enforced global boundary and temporary Codex defaults without delaying exec.

### Deprecated

### Removed

### Fixed

### Security

- The launcher does not log in, read or copy credentials, retain prompts, or
  modify provider configuration.
