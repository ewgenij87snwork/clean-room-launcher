---
layout: page
title: Provider support in v0.2.0
permalink: /providers.html
---

The release has two supported provider paths:

| Coding-agent CLI | Platform | Status |
| --- | --- | --- |
| Codex CLI 0.147.0+ | macOS / Apple Silicon | Interactive and `exec` qualified |
| Claude Code CLI 2.1.223+ | macOS / Apple Silicon | Interactive qualified |

Qualified examples:

```sh
clroom codex
clroom codex exec [CODEX_EXEC_ARGS]
clroom claude
```

For qualified Codex diagnostics, use the top-level forms:

```sh
clroom codex --help
clroom codex --version
```

Clean Room Launcher resolves `codex` from `PATH`, builds the macOS isolation
profile, prints the filesystem-restriction summary, then starts Codex inside
`sandbox-exec`. The `exec` path additionally injects native
`--ignore-user-config`. Terminal streams, signals and exit status remain native.
Interactive `clroom codex` uses the existing CLROOM isolation path without that
exec-only enhancement.

For Claude, the launcher creates one private session-scoped skill projection,
binds it to the real Claude consumer process, and removes it on normal exit or
after a later launch proves the owner dead. Live or unknown sessions are kept.
The v0.2.0 release qualifies the interactive Claude path. A Claude Code `-p`
launch reached the provider and exited successfully during v0.2.0 release
qualification, but its response-output semantics are not independently qualified here.

The launcher does not install either provider, create an account, perform
browser login, inspect provider authentication state, or copy provider
credentials. Existing authentication is used by the selected CLI itself and
left untouched.

Linux and Windows are `NOT_QUALIFIED`; Intel macOS is not supported by this
release.
