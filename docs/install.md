---
layout: page
title: Install v0.2.0
permalink: /install.html
---

Prerequisites:

- macOS on Apple Silicon;
- Codex CLI `0.147.0+` or Claude Code CLI `2.1.223+` already working on its own.

## One-line install

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://github.com/ewgenij87snwork/clean-room-launcher/releases/latest/download/install.sh | sh
```

The installer downloads the current stable macOS Apple Silicon release from
GitHub Releases, verifies the exact archive against `SHA256SUMS`, extracts only
the `clroom` binary, and installs it to `~/.local/bin/clroom`. It does not use
`sudo`, edit shell startup files, install a service, or change provider state.

If `~/.local/bin` is not already in `PATH`, the installer prints the directory
to add. The release archive is unsigned and unnotarized; do not disable
Gatekeeper globally if local macOS policy refuses it.

## Manual release archive

```sh
VERSION=v0.2.0
ASSET=clean-room-launcher-v0.2.0-aarch64-apple-darwin.tar.gz

curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/$ASSET"
curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/SHA256SUMS"
EXPECTED=$(awk -v asset="$ASSET" '$2 == asset {print $1}' SHA256SUMS)
ACTUAL=$(shasum -a 256 "$ASSET" | awk '{print $1}')
test -n "$EXPECTED" && test "$ACTUAL" = "$EXPECTED"
tar -xOzf "$ASSET" "${ASSET%.tar.gz}/bin/clroom" > clroom
mkdir -p "$HOME/.local/bin"
install -m 0755 clroom "$HOME/.local/bin/clroom"
rm clroom
```

This verifies only the archive you downloaded; `SHA256SUMS` also covers the
other release assets.

## Cargo from the release tag

```sh
cargo install --git https://github.com/ewgenij87snwork/clean-room-launcher \
  --tag v0.2.0 --locked
```

The release is not published to crates.io.

## Verify the installation

Run only the provider command or commands you intend to use:

```sh
clroom --help
cd your-project
clroom codex --help       # if Codex is installed
clroom codex --version
clroom claude --version   # if Claude Code is installed
```

To remove an archive or one-line installation, delete
`$HOME/.local/bin/clroom`. For Cargo, run `cargo uninstall clean-room-launcher`.
No service or system setting is created.
