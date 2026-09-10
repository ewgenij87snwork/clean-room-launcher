---
layout: page
title: CLROOM configuration matrix
description: Conservative Codex and Claude Code scope matrix showing what Clean Room Launcher (CLROOM) retains, excludes, or does not claim.
permalink: /configuration-matrix/
---
This table is deliberately conservative.

“Excluded” means current CLROOM has a specific mechanism for the known input listed in that row. It does **not** mean every possible provider-owned input is gone.

| Provider | Input / scope | Current CLROOM direction | Confidence |
| --- | --- | --- | --- |
| Claude Code | interactive top-level launch | Isolated and qualified | v0.2.0 qualification canary |
| Claude Code | `-p` non-interactive launch | Launch path exercised; response-output semantics not claimed as qualified | Provider exited 0, but the expected textual canary was not observed |
| Claude Code | ordinary user settings source | Omitted through `--setting-sources project,local`, with additional controls for known personal-global roots | Confirmed from current CLROOM source |
| Claude Code | project settings source | Retained | Confirmed from current CLROOM source |
| Claude Code | project-local settings source | Retained | Confirmed from current CLROOM source |
| Claude Code | `~/.claude.json` | Not blanket-blocked | Known limitation |
| Claude Code | managed / organization policy | Must remain authoritative | Product invariant; detailed combinations continue to require tests |
| Claude Code | selected personal-global skill | Admitted through a private temporary projection | Confirmed from current CLROOM source |
| Codex | global `AGENTS.md` / `AGENTS.override.md` | Known global instruction inputs blocked for the CLROOM launch | Confirmed from current CLROOM source |
| Codex | interactive top-level launch | Existing isolation path retained | v0.2.0 qualification canary |
| Codex | `exec` non-interactive launch | Existing isolation plus exec-only `--ignore-user-config` | v0.2.0 qualification canary |
| Codex | project instruction chain | Retained | Confirmed from current CLROOM source |
| Codex | unselected personal-global skill contents | Known personal-global skill roots restricted | Confirmed from current CLROOM source |
| Codex | selected personal-global skills | Admitted for the launch | Confirmed from current CLROOM source |
| Both | selected symlinked personal-global skill | Admitted once when supported; canonical target is explicitly bounded | Focused security tests plus macOS sandbox enforcement tests |
| Codex | apps, hooks, plugins | Clean defaults off; explicit supported user arguments can re-enable them | Version-qualified |
| Both | complete home directory | **Not** claimed to be completely isolated | Explicit non-claim |
| Both | provider authentication | Existing provider authentication remains provider-owned | Confirmed product direction |

## Qualified skill source maps

CLROOM inventories only provider-qualified local sources. For Codex this is
the repository `.agents/skills` chain, personal `$HOME/.agents/skills` and
`$CODEX_HOME/skills`, and provider-owned admin/system behavior. Codex plugin
activation remains provider-owned: CLROOM does not infer activation from
cached package residue or invent an `installed_plugins.json` authority. For
Claude Code this is project/enterprise/personal skill locations and an active
cached plugin version gated by its provider-supported `installed_plugins.json`
contract.

Provider-managed synced/remote state is not guessed from filesystem residue.
Cached package directories without an active install record are stale and
remain unavailable. Selecting a package skill admits only that skill
directory and its supporting files; package hooks, MCP, agents, executables,
settings, and notifications are not activated.

For support limits and security scope, read [Limitations](limitations.md) and the [Threat model](threat-model.md).
