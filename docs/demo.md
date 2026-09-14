---
layout: page
title: Clean-launch walkthrough
permalink: /demo.html
---

From any project on macOS/Apple Silicon, start Codex normally:

```sh
clroom codex
```

Before Codex takes over the terminal, the launcher prints a compact
filesystem-restriction status plaque. It shows global instructions and skills
blocked, and apps, hooks, plugins, developer instructions and notifications
disabled by default.

Add selected global skills to the same interactive launch when needed:

```sh
clroom codex --skill-set=my-skill
```

For a non-interactive task, use Codex `exec`:

```sh
clroom codex exec "summarize this repository"
```

Non-interactive `codex exec` arguments pass through unchanged. Explicit
arguments retain final priority:

```sh
clroom codex exec --enable apps --enable hooks --enable plugins
```

The launcher never performs login. If `codex` is missing, it stops locally with
`LOCAL_CODEX_UNAVAILABLE`.
