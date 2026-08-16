# Optional local Cargo installation

This is a local source-build path for developers. It is not a published
package channel and does not inherit the checksum, binary provenance,
signature, notarization, or platform qualification of a release archive.
The evidence class is `local-source-build` and the result remains
`NOT_QUALIFIED`.

The repository currently declares `publish = false`; Clean Room Launcher must not be
advertised as installable from crates.io. From an explicitly trusted local
checkout, with dependencies already present locally, install the locked
source without network access:

```sh
CARGO_NET_OFFLINE=true cargo install \
  --path /absolute/path/to/clean-room-launcher \
  --locked --offline --root /absolute/private/install-root
```

The install creates only the package-owned `clroom` executable below the
selected Cargo root. Remove the source-built package with:

```sh
cargo uninstall --root /absolute/private/install-root clean-room-launcher
```

Cargo may retain its documented bookkeeping files inside that explicitly
selected root; uninstall must remove the Clean Room Launcher executable and must
not touch provider, Git, user configuration, or files outside the root.
