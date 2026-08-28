---
layout: page
title: Demo
permalink: /demo.html
---

From any project on macOS/Apple Silicon:

```sh
clroom codex
```

Before Codex starts, the launcher prints a compact boundary status plaque. It
shows global instructions and skills blocked, and apps, hooks, plugins,
developer instructions and notifications disabled by default.

Use ordinary Codex commands unchanged:

```sh
clroom codex features list
clroom codex exec "summarize this repository"
```

Explicit arguments retain final priority:

```sh
clroom codex --enable apps --enable hooks --enable plugins
```

The launcher never performs login. If `codex` is missing, it stops locally with
`LOCAL_CODEX_UNAVAILABLE`.
