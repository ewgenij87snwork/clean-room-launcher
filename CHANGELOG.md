# Changelog

All notable changes to Clean Room Launcher will be documented in this file.

The format is based on Keep a Changelog, and this project intends to use
Semantic Versioning after the first public release.

## [Unreleased]

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
