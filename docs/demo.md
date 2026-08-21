# Demo

From any project on macOS/Apple Silicon:

```sh
clroom codex
```

Before Codex starts, the launcher names the project, confirms that the
clean-room boundary is active, lists its temporary defaults, and explains the
expected blocked-file warning.

Use ordinary Codex commands unchanged:

```sh
clroom codex features list
clroom codex exec "summarize this repository"
```

Explicit arguments retain final priority:

```sh
clroom codex --enable hooks --enable plugins
```

The launcher never performs login. If `codex` is missing, it stops locally with
`LOCAL_CODEX_UNAVAILABLE`.
