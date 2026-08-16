# Provider support

The local preview has one observed launch path:

| Coding-agent CLI | Platform | Status |
| --- | --- | --- |
| Local Codex CLI | macOS / arm64 | Local launch boundary observed |

Use it as:

```sh
clroom codex [ordinary Codex arguments]
```

Clean Room Launcher starts the locally available CLI. It does not replace the
CLI, create an account, perform browser login, or copy provider credentials.

Other providers and operating systems are not advertised by this preview.
Their support requires their own observed artifact and launch evidence.
