# Limitations

- `v0.1.0-alpha.4.1` is an unsigned, unnotarized prerelease, not a stable release.
- Only macOS on Apple Silicon with Codex CLI `0.147.0+` or Claude Code CLI
  `2.1.223+` is supported.
- The boundary is a narrow macOS filesystem denylist, not a VM, container,
  network sandbox or complete home-directory isolation.
- A provider may visibly warn that reading a blocked global instruction is not
  permitted. This is expected and does not mean the provider itself failed.
- User arguments are intentionally last. Explicit overrides can re-enable apps,
  hooks or plugins and therefore reduce the clean defaults.
- The project directory and other host paths remain available unless macOS or
  the selected provider applies an additional restriction.
- The launcher depends on the undocumented longevity of macOS `sandbox-exec`;
  it fails closed if the boundary cannot be created.
- No bounty program exists.
- Claude cleanup preserves live, unknown, corrupt, and legacy projection state;
  only a recognized session whose recorded owner is proven dead is reaped.
- Linux and Windows are `NOT_QUALIFIED`. Intel macOS, Homebrew, crates.io,
  signing and notarization are not claimed.
