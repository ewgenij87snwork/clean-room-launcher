# Clean Room Launcher demo

This candidate is a local macOS/arm64 preview. It is not published and remains
`NOT_QUALIFIED` for public release.

The archive contains one executable:

```text
bin/clroom
```

After extracting the exact `clean-room-launcher` archive, inspect the launcher
without starting a coding-agent CLI:

```sh
./bin/clroom --help
./bin/clroom status
```

To start the locally available coding-agent CLI through the clean-room launch
boundary, pass its ordinary arguments unchanged:

```sh
./bin/clroom codex --help
```

On macOS, `clroom` starts Codex under its filesystem boundary. The launcher
does not log in, copy credentials, change existing configuration, publish
anything, or make an unsupported provider claim.

The exact candidate artifact and its SHA-256 are recorded in
`reports/release/candidate.json`. Public availability, namespace ownership,
an external clean install, and a fully qualified clean Codex launch remain
open release conditions.
