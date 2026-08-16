# Install the local preview

Clean Room Launcher is currently a macOS/arm64 local preview. It is not yet a
published package or a public release.

Given an exact `clean-room-launcher-*.tar.gz` artifact and its SHA-256, verify
it before extracting it:

```sh
shasum -a 256 clean-room-launcher-*.tar.gz
tar -xzf clean-room-launcher-*.tar.gz
```

The extracted archive contains one executable, `bin/clroom`.

```sh
./clean-room-launcher-*/bin/clroom --help
./clean-room-launcher-*/bin/clroom status
./clean-room-launcher-*/bin/clroom codex --help
```

`clroom codex …` passes ordinary Codex arguments through the launcher. On the
currently observed macOS tuple it starts Codex under the clean-room filesystem
boundary. It does not ask you to log in, copy credentials, or change your
existing configuration.

To remove the preview, delete only the extracted archive directory. It does
not install a service, background process, account, or system-wide setting.
