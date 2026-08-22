# Provider support in v0.1.0-alpha.2

The alpha has one supported path:

| Coding-agent CLI | Platform | Status |
| --- | --- | --- |
| Locally installed Codex CLI | macOS / Apple Silicon | Alpha |

Use it as:

```sh
clroom codex [ordinary Codex arguments]
```

Clean Room Launcher resolves `codex` from `PATH`, builds the macOS isolation
profile, prints the boundary summary, then replaces itself with
`sandbox-exec … codex`. Terminal streams, signals and exit status remain native.

The launcher does not install Codex, create an account, perform browser login,
read provider state, or copy provider credentials. Existing provider state is
used by Codex itself and left untouched.

Claude, Linux, Windows and Intel macOS are not supported by this alpha.
