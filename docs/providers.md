# Provider support in v0.1.0-alpha.4.2

The alpha has two supported paths:

| Coding-agent CLI | Platform | Status |
| --- | --- | --- |
| Codex CLI 0.147.0+ | macOS / Apple Silicon | Alpha |
| Claude Code CLI 2.1.223+ | macOS / Apple Silicon | Alpha |

Use it as:

```sh
clroom codex [ordinary Codex arguments]
clroom claude [ordinary Claude Code arguments]
```

Clean Room Launcher resolves `codex` from `PATH`, builds the macOS isolation
profile, prints the boundary summary, then replaces itself with
`sandbox-exec … codex`. Terminal streams, signals and exit status remain native.

For Claude, the launcher creates one private session-scoped skill projection,
binds it to the real Claude consumer process, and removes it on normal exit or
after a later launch proves the owner dead. Live or unknown sessions are kept.

The launcher does not install either provider, create an account, perform
browser login, inspect provider authentication state, or copy provider
credentials. Existing authentication is used by the selected CLI itself and
left untouched.

Linux and Windows are `NOT_QUALIFIED`; Intel macOS is not supported by this
alpha.
