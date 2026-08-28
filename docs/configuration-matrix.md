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
| Claude Code | ordinary user settings source | Omitted through `--setting-sources project,local`, with additional controls for known personal-global roots | Confirmed from current CLROOM source |
| Claude Code | project settings source | Retained | Confirmed from current CLROOM source |
| Claude Code | project-local settings source | Retained | Confirmed from current CLROOM source |
| Claude Code | `~/.claude.json` | Not blanket-blocked | Known limitation |
| Claude Code | managed / organization policy | Must remain authoritative | Product invariant; detailed combinations continue to require tests |
| Claude Code | selected personal-global skill | Admitted through a private temporary projection | Confirmed from current CLROOM source |
| Codex | global `AGENTS.md` / `AGENTS.override.md` | Known global instruction inputs blocked for the CLROOM launch | Confirmed from current CLROOM source |
| Codex | project instruction chain | Retained | Confirmed from current CLROOM source |
| Codex | unselected personal-global skill contents | Known personal-global skill roots restricted | Confirmed from current CLROOM source |
| Codex | selected personal-global skills | Admitted for the launch | Confirmed from current CLROOM source |
| Codex | apps, hooks, plugins | Clean defaults off; explicit supported user arguments can re-enable them | Version-qualified |
| Both | complete home directory | **Not** claimed to be completely isolated | Explicit non-claim |
| Both | provider authentication | Existing provider authentication remains provider-owned | Confirmed product direction |

For support limits and security scope, read [Limitations](limitations.md) and the [Threat model](threat-model.md).
