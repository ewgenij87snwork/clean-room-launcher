---
layout: page
title: Upgrade, roll back and remove
description: Upgrade, roll back, or remove the three CLROOM command-line binaries without changing provider authentication, configuration, or projects.
permalink: /upgrade-rollback.html
---

The archive install provides three binaries: `clroom`, `clroom-codex`, and
`clroom-claude`. It does not install a daemon, service, account, or system-wide
configuration.

Before upgrading, keep the currently installed binaries:

```sh
for name in clroom clroom-codex clroom-claude; do
  cp "$HOME/.local/bin/$name" "$HOME/.local/bin/$name.previous"
done
```

Download the new archive, verify `SHA256SUMS`, extract it, then replace the same
three binaries under `~/.local/bin`.

To roll back:

```sh
for name in clroom clroom-codex clroom-claude; do
  mv "$HOME/.local/bin/$name.previous" "$HOME/.local/bin/$name"
done
```

To remove an archive or one-line installation:

```sh
rm "$HOME/.local/bin/clroom" "$HOME/.local/bin/clroom-codex" "$HOME/.local/bin/clroom-claude"
```

These operations do not modify Codex or Claude Code authentication,
configuration, projects, or provider installations.
