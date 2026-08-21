# Limitations

- `v0.1.0-alpha.1` is an unsigned, unnotarized prerelease, not a stable release.
- Only macOS on Apple Silicon with a locally installed Codex CLI is supported.
- The boundary is a narrow macOS filesystem denylist, not a VM, container,
  network sandbox or complete home-directory isolation.
- Codex may visibly warn that reading a blocked global `AGENTS.md` is not
  permitted. This is expected and does not mean Codex itself failed.
- User arguments are intentionally last. Explicit overrides can re-enable apps,
  hooks or plugins and therefore reduce the clean defaults.
- The project directory and other host paths remain available unless macOS or
  Codex applies an additional restriction.
- The launcher depends on the undocumented longevity of macOS `sandbox-exec`;
  it fails closed if the boundary cannot be created.
- No bounty program exists.
- Claude, Linux, Windows, Intel macOS, Homebrew, crates.io, signing and
  notarization are not claimed.
