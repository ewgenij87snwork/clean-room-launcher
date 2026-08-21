# Install v0.1.0-alpha.1

Prerequisites:

- macOS on Apple Silicon;
- a locally installed `codex` CLI that already works on its own.

## Release archive

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

The archive is unsigned and unnotarized. If local macOS policy refuses it,
prefer the source install below; do not disable Gatekeeper globally.

## Cargo from the release tag

```sh
cargo install --git https://github.com/ewgenij87snwork/clean-room-launcher \
  --tag v0.1.0-alpha.1 --locked
```

The alpha is not published to crates.io.

## Verify the installation

```sh
clroom --help
cd your-project
clroom codex features list
```

To remove an archive install, delete `$HOME/.local/bin/clroom`. For Cargo, run
`cargo uninstall clean-room-launcher`. No service or system setting is created.
