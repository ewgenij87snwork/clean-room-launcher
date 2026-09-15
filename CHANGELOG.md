# Changelog

All notable changes to Clean Room Launcher will be documented in this file.

The format is based on Keep a Changelog, and this project intends to use
Semantic Versioning after the first public release.

## [Unreleased]

## [0.2.1] - 2026-09-15

### Changed

- Updated the README, demo, and install guidance to present interactive
  `clroom codex` as the primary Codex launch path and the published GitHub
  installer as the normal installation path.

### Fixed

- Repeated Codex launches no longer fail with `CLROOM_CODEX_STATE_DIRTY` after
  supported Codex `0.154.0` creates legitimate provider-owned `cache` or
  `plugins` state inside an initialized CLROOM shadow home.

### Compatibility

- Exact real-provider qualification remains Codex `0.154.0` and Claude Code
  `2.1.263`; documented minimum accepted ranges remain Codex `0.147.0+` and
  Claude Code `2.1.223+`.
- This patch adds no platform expansion: macOS on Apple Silicon remains the
  qualified release platform.

### Security

- Capability-owned Codex state is accepted only inside a valid initialized
  CLROOM shadow home and only as real top-level directories; unknown roots,
  symlinks, and invalid entry types remain fail-closed.
- Apps, hooks, and plugins remain disabled by default. The distributed macOS
  archive remains unsigned and unnotarized.

## [0.2.0] - 2026-09-14

### Added

- Added a persistent clean configuration view for interactive `clroom codex`;
  native `--ignore-user-config` remains an exec-only enhancement.
- Selected symlinked global skills preserve canonical-target isolation and
  duplicate-source safety across the qualified provider paths.
- Codex exec launches with clean user configuration, provider-aware
  selected-skill inventory, and fail-closed filesystem restrictions.
- Drop-in `clroom-codex` and `clroom-claude` provider executables preserve native
  provider arguments, interactive process behavior, and exact `--pass-env=NAME`
  admission, with Claude parity and duplicate/invalid-name refusal.
- The release process binds sanitized real-provider startup evidence to Codex
  `0.154.0` and Claude Code `2.1.263`, alongside canonical release readiness,
  SCA verification, `SHA256SUMS`, a CycloneDX SBOM, provenance, and GitHub
  attestations.
- Added checksum-verified one-line macOS Apple Silicon installation from GitHub
  Releases without `sudo` or shell-configuration mutation.
- Strengthened documentation discovery assets for search engines and AI-facing
  documentation discovery without changing the supported runtime surface.

### Fixed

- Claude Code `2.1.257+` no longer rejects CLROOM's local selected-skill
  projection as a network path when the outer macOS Seatbelt policy is active.
  The fix preserves denial of sibling projection contents, unselected skills,
  provider state, credential roots, and writes to protected skill sources.

### Compatibility

- macOS on Apple Silicon is the qualified platform for `v0.2.0`.
- The exact real-provider qualification targets are Codex `0.154.0` and Claude
  Code `2.1.263`. The documented minimum accepted parser/runtime ranges remain
  Codex `0.147.0+` and Claude Code `2.1.223+`.
- Claude Code project and other ambient MCP configurations are not loaded by the
  default `v0.2.0` Claude launch. CLROOM starts Claude with
  `--strict-mcp-config`; MCP servers are considered only when explicitly
  supplied through Claude's own `--mcp-config` argument.
- Claude Code `-p` response-output semantics are not independently qualified by
  this release.

### Security

- The distributed macOS archive is unsigned and unnotarized.
- The archive records `qualification=CANDIDATE`; runtime qualification is kept
  as separately verified evidence bound to the exact candidate bytes rather
  than being self-asserted by the archive itself.
- Linux, Windows, Intel macOS, Homebrew, crates.io distribution, signing, and
  notarization are not claimed by `v0.2.0`.

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
  contents remain outside the launch restrictions.
- Duplicate selected global skills resolve once using Codex root precedence.

### Security

- Discovery access is limited to root metadata and listing; unselected skill
  bodies remain unreadable.

## [0.1.0-alpha.2] - 2026-08-22

### Added

- One `--skill-set=` option for exact global skills, whole namespaces, exact
  `namespace:skill` skill names, reusable named `@sets`, and mixed selections.
- User-owned skill sets from `$XDG_CONFIG_HOME/clroom/skill-sets.yaml` or
  `~/.config/clroom/skill-sets.yaml`; Clean Room Launcher reads this file only
  when an `@set` is requested and never creates or rewrites it.

### Changed

- Project-local skills remain automatic while unselected global skills stay
  outside each launch.
- The launch plaque reports how many global skills were admitted.

### Security

- Unknown, malformed, nested, unsafe-path, and ambiguous skill choices fail before
  Codex starts.

## [0.1.0-alpha.1] - 2026-08-21

### Added

- `clroom codex [ARGS...]` for the locally installed Codex CLI on macOS/Apple
  Silicon.
- A macOS Seatbelt policy that blocks global Codex instructions and known
  ambient skill roots while retaining project access.
- Clean launch defaults for apps, hooks, plugins, developer instructions and
  notifications, with explicit user arguments retaining final priority.
- Deterministic unsigned macOS/arm64 archive and `SHA256SUMS` verification.

### Changed

- The pre-launch screen is a compact status plaque that reports the enforced
  filesystem restrictions and temporary Codex defaults without delaying exec.

### Deprecated

### Removed

### Fixed

### Security

- The launcher does not log in, read or copy credentials, retain prompts, or
  modify provider configuration.
