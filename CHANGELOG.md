# Changelog

All notable changes to Clean Room Launcher will be documented in this file.

The format is based on Keep a Changelog, and this project intends to use
Semantic Versioning after the first public release.

## [Unreleased]

## [0.1.0-alpha.1] - 2026-08-21

### Added

- `clroom codex [ARGS...]` for the locally installed Codex CLI on macOS/Apple
  Silicon.
- A macOS Seatbelt boundary that blocks global Codex instructions and known
  ambient skill roots while retaining project access.
- Clean launch defaults for hooks, plugins, developer instructions and
  notifications, with explicit user arguments retaining final priority.
- Deterministic unsigned macOS/arm64 archive and `SHA256SUMS` verification.

### Changed

- The pre-launch screen now explains the active boundary and the expected
  blocked-file warning from Codex.

### Deprecated

### Removed

### Fixed

### Security

- The launcher does not log in, read or copy credentials, retain prompts, or
  modify provider configuration.
