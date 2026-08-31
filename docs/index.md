---
layout: home
title: Clean Room Launcher (CLROOM)
description: Clean Room Launcher (CLROOM) is an open-source developer tool for launching Codex and Claude Code with a clean/selective session setup on macOS.
permalink: /
---

Clean Room Launcher (CLROOM) is an open-source developer tool for launching the installed Codex or Claude Code CLI with a session-specific clean/selective setup on supported macOS systems. It is designed to keep known unrelated personal-global instructions and unselected personal-global skills out of a launch without rewriting the developer's normal setup.

## Start here

- [Why CLROOM exists — the 2-minute explanation](why-clroom.md)
- [Problem index: find your symptom or half-remembered term](problem-index.md)
- [Use cases: practical CLROOM workflows](use-cases.md)
- [Skill sets: create, use, combine, and edit reusable groups](skill-sets.md)
- [When to use CLROOM — and when not to](when-to-use-clroom.md)
- [Claude Code and CLROOM](claude-code.md)
- [Codex and CLROOM](codex.md)
- [Configuration matrix](configuration-matrix.md)
- [Frequently asked questions](faq.md)
- [Current limitations](limitations.md)
- [Threat model](threat-model.md)
- [Installation](install.md)

## Start from the problem, not the product name

If you only remember a symptom — old instructions, too many skills, `--safe-mode`, `--bare`, `CODEX_HOME`, a hook firing, a wrong implementation path, or a clean baseline — use the [coding-agent configuration problem index](problem-index.md). It maps common Claude Code and Codex problems to the shortest correct answer and shows where CLROOM fits.

## How these docs are written

These pages separate:

1. what Codex or Claude Code does natively;
2. what current CLROOM source and tests establish;
3. what CLROOM does **not** claim;
4. what still needs runtime verification.

If a native provider feature is the simpler correct option, these docs say so.

Last structured provider-doc review: **2026-08-28**.
