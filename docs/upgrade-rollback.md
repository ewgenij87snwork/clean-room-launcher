---
layout: page
title: Upgrade, roll back and remove
permalink: /upgrade-rollback.html
---

The archive install is one executable. It does not install a daemon, service,
account or system-wide configuration.

Before upgrading, keep the current binary:

```sh
cp "$HOME/.local/bin/clroom" "$HOME/.local/bin/clroom.previous"
```

Download the new archive, verify `SHA256SUMS`, extract it, then replace only
`$HOME/.local/bin/clroom`.

To roll back:

```sh
mv "$HOME/.local/bin/clroom.previous" "$HOME/.local/bin/clroom"
```

To remove:

```sh
rm "$HOME/.local/bin/clroom"
```

These operations do not modify Codex authentication, configuration, projects
or the Codex installation.
