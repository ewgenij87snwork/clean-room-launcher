---
layout: page
title: Limitations
permalink: /limitations.html
---

- `v0.2.0` is unsigned and unnotarized; qualification is limited to the
  documented macOS Apple Silicon release path.
- Only macOS on Apple Silicon is supported. The minimum accepted versions are
  Codex CLI `0.147.0` and Claude Code CLI `2.1.223`; independent v0.2.0
  qualification is limited to Codex `0.154.0` and Claude Code `2.1.263`.
- The protection is a narrow macOS filesystem denylist, not a VM, container,
  network sandbox or complete home-directory isolation.
- A provider may visibly warn that reading a blocked global instruction is not
  permitted. This is expected and does not mean the provider itself failed.
- User arguments are intentionally last. Explicit overrides can re-enable apps,
  hooks or plugins and therefore reduce the clean defaults.
- The project directory and other host paths remain available unless macOS or
  the selected provider applies an additional restriction.
- CLROOM controls each top-level launch. Provider-owned Claude Code teammates
  and subagents follow Claude's own inheritance and scoping rules.
- Claude Code `-p` reached the provider and exited successfully in the v0.2.0
  qualification canary, but response-output semantics are not independently
  qualified by this release.
- The launcher depends on the undocumented longevity of macOS `sandbox-exec`;
  it fails closed if the protection cannot be created.
- No bounty program exists.
- Claude cleanup preserves live, unknown, corrupt, and legacy projection state;
  only a recognized session whose recorded owner is proven dead is reaped.
- Linux and Windows are `NOT_QUALIFIED`. Intel macOS, Homebrew, crates.io,
  signing and notarization are not claimed.
