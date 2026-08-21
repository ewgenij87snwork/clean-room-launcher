# Clean Room Launcher

A small macOS launcher that starts your existing Codex CLI with ambient agent
instructions, skills, apps, hooks, plugins and notifications out of the way.

![Clean Room Launcher alpha demo](docs/assets/clroom-alpha.gif)

`clroom` keeps your current project and provider login, but gives each launch a
cleaner starting point:

- blocks reads of global Codex `AGENTS.md` files and ambient skill roots with
  the built-in macOS sandbox;
- starts Codex with apps, hooks and plugins off, empty developer instructions
  and no notifications;
- appends your Codex arguments last, so an explicit user override still wins;
- leaves provider authentication and configuration untouched.

This is an early, unsigned and unnotarized macOS/Apple Silicon alpha. It is a
focused launch boundary, not a VM, container or complete host sandbox.

## Install in sixty seconds

You need macOS on Apple Silicon and an already installed, working `codex` CLI.

```sh
VERSION=v0.1.0-alpha.1
ASSET=clean-room-launcher-v0.1.0-alpha.1-aarch64-apple-darwin.tar.gz

curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/$ASSET"
curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/SHA256SUMS"
shasum -a 256 -c SHA256SUMS
tar -xzf "$ASSET"

mkdir -p "$HOME/.local/bin"
install -m 0755 "clean-room-launcher-v0.1.0-alpha.1-aarch64-apple-darwin/bin/clroom" "$HOME/.local/bin/clroom"
```

Make sure `$HOME/.local/bin` is on `PATH`, then launch from the project you want
Codex to work in:

```sh
cd your-project
clroom codex
```

Codex may print `Operation not permitted` when it probes a blocked global
`AGENTS.md` file. That warning is expected: the clean-room boundary denied the
read. The launch plaque marks global `AGENTS.md` as off before Codex starts.

## Install with Cargo

Rust users can build the same alpha from the public tag:

```sh
cargo install --git https://github.com/ewgenij87snwork/clean-room-launcher \
  --tag v0.1.0-alpha.1 --locked
```

No crates.io package is published for this alpha.

## Defaults and overrides

`clroom codex [ARGS...]` starts Codex with these defaults first:

```text
-c features.apps=false
-c features.hooks=false
-c features.plugins=false
-c developer_instructions=""
-c notify=[]
```

Your arguments come after them. For example, this deliberately opts back in:

```sh
clroom codex --enable apps --enable hooks --enable plugins
```

## Security boundary

The macOS Seatbelt profile denies reads of global Codex instruction files and
the known Codex/agent skill roots while leaving the selected project available.
It does not block the network, hide every file in your home directory, inspect
prompts, copy credentials or replace Codex. See [the threat model](docs/threat-model.md),
[limitations](docs/limitations.md) and [security policy](SECURITY.md).

## Remove

```sh
rm "$HOME/.local/bin/clroom"
```

There is no daemon, service, account or system-wide configuration to remove.

## Status

`v0.1.0-alpha.1` supports one path: local Codex on macOS/Apple Silicon. Claude,
Linux, Windows, Homebrew, crates.io, signing and notarization are not claimed.

Licensed under [MPL-2.0](LICENSE).
