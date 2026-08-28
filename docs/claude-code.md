---
layout: page
title: Claude Code and CLROOM
description: How Clean Room Launcher (CLROOM) relates to Claude Code user, project, local and managed settings, CLAUDE.md, skills, safe mode, bare mode, and setting sources.
permalink: /claude-code/
nav_title: Claude Code
---
CLROOM does not replace Claude Code. It launches the installed `claude` CLI.

This page focuses on one question: **which Claude Code configuration sources are relevant to a CLROOM launch, and what should you use natively instead when CLROOM is unnecessary?**

## Claude Code has multiple configuration scopes

Anthropic documents user, project, project-local, and managed organization settings. Those scopes are not interchangeable.

A simplified view:

| Claude Code scope | Examples | Current CLROOM direction |
| --- | --- | --- |
| User | `~/.claude/settings.json`, user `CLAUDE.md`, user rules/skills | Ordinary user settings source omitted; known personal-global instruction/skill roots restricted |
| Project | project `CLAUDE.md`, `.claude/settings.json`, project rules/skills | Retained |
| Project local | `CLAUDE.local.md`, `.claude/settings.local.json` | Retained |
| Managed / organization | managed settings delivered through supported admin mechanisms | Must remain authoritative |

Current CLROOM source launches Claude with `--setting-sources project,local`, `--strict-mcp-config`, fail-closed sandbox settings, disabled auto-memory, and additional filesystem controls for known personal-global roots.

Selected personal-global skills are exposed through a private temporary projection and `--add-dir`.

## Does CLROOM remove every Claude global or provider-owned input?

**No.**

Anthropic documents some global configuration/state outside the ordinary filesystem settings-source model, including keys in `~/.claude.json`. Current CLROOM does not blanket-block that sibling file.

Do not translate “ordinary user settings are omitted” into “all Claude global configuration is gone.”

CLROOM also does not claim complete home-directory isolation.

## Does CLROOM bypass organization-managed Claude Code policy?

**No. It must not.**

Anthropic documents managed settings as the highest normal settings tier, with specific exceptions that can only make security-sensitive values stricter.

CLROOM's product invariant is that organization-managed policy remains authoritative.

There is still a runtime-verification gap around every possible interaction between managed customization restrictions and a selected skill exposed through an additional directory. Until that matrix is proven, do not claim universal enterprise-policy compatibility.

## `claude --safe-mode` vs CLROOM

Anthropic documents `--safe-mode` as a troubleshooting mode that disables a broad set of customizations, including `CLAUDE.md`, skills, plugins, hooks, MCP servers, commands/agents, styles/workflows, and auto-memory. Managed settings policy still applies, with provider-documented exceptions for which managed customizations do or do not load.

Use it when broad clean troubleshooting is what you want.

CLROOM targets a narrower selective workflow: keep relevant project/local work, omit ordinary personal-global sources, and deliberately admit selected personal-global skills.

## `claude --bare` vs CLROOM

Anthropic documents `--bare` as a minimal mode for faster scripted calls. It skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and `CLAUDE.md`.

That is a strong native option for minimal invocation.

CLROOM is intended for a different workflow: a repeatable selective session that does not require rewriting the developer's normal setup.

## `--setting-sources project,local` vs CLROOM

This native Claude flag is part of CLROOM's current implementation, but you can use it directly yourself.

Use the native flag alone if it completely solves the problem.

CLROOM additionally applies its provider-specific launch controls and selected-skill workflow. Those are implementation details, not a reason to use CLROOM when the native flag is already sufficient.

## Why `--add-dir` matters for selected skills

Anthropic documents skills and commands as an exception to the usual additional-directory rule: `.claude/skills/` and `.claude/commands/` in an added directory can be discovered automatically.

CLROOM uses that provider behavior for its private selected-skill projection.

This is also why managed-policy interactions around selected skills require careful runtime testing rather than assumptions.

## Official Anthropic sources

- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Claude Code skills](https://code.claude.com/docs/en/skills)
- [Claude Code documentation index](https://code.claude.com/docs/llms.txt)

Last verified against current Anthropic documentation: **2026-08-28**.
