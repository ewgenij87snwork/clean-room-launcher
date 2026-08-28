---
layout: page
title: Coding-agent configuration problem index — Claude Code, Codex, Agent Skills and clean sessions
description: Common Claude Code and Codex problems around global instructions, Agent Skills, clean sessions, safe mode, bare mode, CODEX_HOME, CLAUDE.md, AGENTS.md, hooks, plugins and reproducibility — with where Clean Room Launcher (CLROOM) fits and where native tools are better.
permalink: /problem-index/
nav_title: Problem index
---
People rarely start by searching for **Clean Room Launcher (CLROOM)**. They start with a symptom or a half-remembered term:

- “Why is Codex following old instructions?”
- “How do I test one Agent Skill without the others?”
- “What is Claude `--safe-mode` or `--bare`?”
- “Can I keep my normal setup but run one cleaner session?”

This page routes those problems to the shortest correct answer. It also says when a native Claude Code or Codex feature is the better tool.

> CLROOM is not a replacement for Claude Code or Codex. It launches the installed provider CLI with a session-specific clean/selective setup on supported macOS systems.

If you want the human explanation before the provider details, read [Why Clean Room Launcher (CLROOM) exists](why-clroom.md).

<a id="clean-or-fresh-session"></a>
## Need a clean or fresh coding-agent session?

**Common ways people ask this:**

- `clean coding agent session`
- `clean agent session`
- `start Claude clean`
- `isolated agent session`
- `how do I test with a clean agent setup`

Use a native provider clean/minimal mode when you truly want broad customization off. Use Clean Room Launcher (CLROOM) when you want a repeatable clean/selective launch without deleting the setup you normally use.

**Go deeper:** [When To Use Clroom](when-to-use-clroom.md) · [Claude Code](claude-code.md) · [Codex](codex.md)

<a id="global-instructions-vs-current-work"></a>
## Are global instructions affecting the work in front of you?

**Common ways people ask this:**

- `disable global instructions for one session`
- `ignore global instructions temporarily`
- `global instructions leaking into another project`
- `ignore ~/.claude CLAUDE.md`
- `unrelated instructions loaded by Codex`

Claude Code and Codex both have persistent configuration/instruction layers. CLROOM is designed to keep known unrelated personal-global instruction inputs out of its launch while retaining the relevant project-side context it is designed to keep. Provider-specific limits matter, so use the Claude/Codex pages for the exact scope.

**Go deeper:** [Claude Code](claude-code.md) · [Codex](codex.md) · [Configuration Matrix](configuration-matrix.md)

<a id="only-selected-skills"></a>
## Do you want only selected global skills for this launch?

**Common ways people ask this:**

- `run with only selected skills`
- `choose skills per session`
- `test Agent Skill cleanly`
- `skills from other projects interfering`
- `test whether a skill actually improved output`

That is a core CLROOM use case. A developer can select a global skill or reusable skill set for one launch instead of letting every personal-global skill remain eligible by default.

**Go deeper:** [When To Use Clroom](when-to-use-clroom.md) · [Claude Code](claude-code.md) · [Codex](codex.md) · [Faq](faq.md)

<a id="many-skills-and-token-concerns"></a>
## Do many installed skills mean too many tokens or too much context?

**Common ways people ask this:**

- `many skills tokens`
- `too many skills tokens`
- `skill descriptions context cost`
- `too many skills overwhelming agent`
- `skill discovery overhead`

Not necessarily. Providers can use progressive disclosure and metadata rather than loading every skill body at once. CLROOM should not promise a fixed token saving. Its stronger value is controlling which personal-global skills can participate and reducing unrelated or conflicting instruction paths.

**Go deeper:** [Faq](faq.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="wrong-path-or-rework"></a>
## Did the agent go down the wrong implementation path?

**Common ways people ask this:**

- `coding agent went down wrong path`
- `agent chose wrong implementation path`
- `agent behaves differently between projects`
- `same prompt different result because config`
- `agent setup causes rework`

Many causes are possible: repository instructions, local configuration, provider state, tools, models, or personal-global inputs. CLROOM gives you a cleaner comparison point for testing whether personal-global instructions or skills were part of the problem.

**Go deeper:** [Faq](faq.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="context-noise-or-contamination"></a>
## Are you searching for “context pollution”, “prompt pollution”, or “agent context noise”?

**Common ways people ask this:**

- `context pollution coding agent`
- `context contamination coding agent`
- `instruction pollution`
- `reduce coding agent context noise`
- `configuration contamination coding agent`

Those phrases are useful symptoms, but CLROOM makes a narrower technical claim. It controls known personal-global instruction and skill inputs for its launch; it does not claim that every kind of model context or provider state disappears.

**Go deeper:** [Configuration Matrix](configuration-matrix.md) · [Faq](faq.md) · [Limitations](limitations.md)

<a id="claude-safe-mode"></a>
## Should you use Claude Code `--safe-mode` instead?

**Common ways people ask this:**

- `claude --safe-mode`
- `Claude Code safe mode`
- `Claude disable all customizations`
- `Claude Code broken config`
- `what does Claude safe mode disable`

Often, yes. If you want broad customization disabled for troubleshooting, Claude's native safe mode is the simpler answer. CLROOM targets the selective case where relevant project/local configuration can remain while ordinary personal-global inputs stay out and chosen global skills can be admitted.

**Go deeper:** [Claude Code](claude-code.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="claude-bare-mode"></a>
## Should you use Claude Code `--bare` instead?

**Common ways people ask this:**

- `claude --bare`
- `Claude Code bare mode`
- `bare vs CLROOM`
- `Claude scripted minimal mode`
- `when to use --bare Claude`

Use native `--bare` when its minimal/script-oriented behavior matches the job. CLROOM is not the only clean-launch option; it is for repeatable selective composition without rewriting the normal setup.

**Go deeper:** [Claude Code](claude-code.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="claude-setting-sources"></a>
## Can Claude Code `--setting-sources` solve this natively?

**Common ways people ask this:**

- `Claude --setting-sources`
- `setting-sources project local`
- `Claude user project local settings`
- `keep CLAUDE.md project but ignore global CLAUDE.md`
- `Claude Code configuration layers`

Sometimes. Claude can choose user/project/local filesystem setting sources directly. CLROOM uses provider-native controls plus its own launch behavior; if `--setting-sources` alone solves the problem, prefer the native flag.

**Go deeper:** [Claude Code](claude-code.md) · [Configuration Matrix](configuration-matrix.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="claude-config-dir"></a>
## Would `CLAUDE_CONFIG_DIR` be simpler?

**Common ways people ask this:**

- `CLAUDE_CONFIG_DIR clean config`
- `temporary CLAUDE_CONFIG_DIR`
- `empty Claude config directory`
- `Claude alternate config directory`
- `Claude config home`

Use an alternate Claude config directory when you want a persistent alternate configuration, but do not assume `CLAUDE_CONFIG_DIR` alone isolates every Claude input. CLROOM is aimed at session-specific clean/selective launches while the normal configuration remains in place.

**Go deeper:** [Claude Code](claude-code.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="claude-project-and-local-configuration"></a>
## What happens to Claude project and local configuration?

**Common ways people ask this:**

- `CLAUDE.md vs CLAUDE.local.md`
- `Claude project vs local instructions`
- `Claude settings.json vs settings.local.json`
- `Claude project hooks vs user hooks`
- `does CLROOM keep project CLAUDE.md`

Current CLROOM intentionally retains Claude project and project-local setting sources. That includes the provider's project/local layering CLROOM is configured to keep; read the provider page and limitations before assuming every customization behaves identically.

**Go deeper:** [Claude Code](claude-code.md) · [Configuration Matrix](configuration-matrix.md) · [Limitations](limitations.md)

<a id="codex-agents-md"></a>
## Why is Codex reading global `AGENTS.md`?

**Common ways people ask this:**

- `Codex without global AGENTS.md`
- `Codex AGENTS.md global project`
- `Codex instruction hierarchy`
- `temporary disable AGENTS.md`
- `Codex user instructions project instructions`

OpenAI documents global instructions under `CODEX_HOME` plus project instruction discovery. CLROOM's Codex path is designed to block the known global `AGENTS.md` / `AGENTS.override.md` inputs for its clean launch while retaining project instruction context.

**Go deeper:** [Codex](codex.md) · [Configuration Matrix](configuration-matrix.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="codex-home-and-profiles"></a>
## Should you use `CODEX_HOME` or a Codex profile instead?

**Common ways people ask this:**

- `CODEX_HOME`
- `temporary CODEX_HOME`
- `multiple Codex configs`
- `Codex alternate config`
- `Codex without default config`

Use native homes/profiles when you want a persistent alternate Codex setup or reusable config values. CLROOM is useful when the problem is per-launch control over known personal-global instructions and skills without maintaining another normal home.

**Go deeper:** [Codex](codex.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="codex-skill-scopes"></a>
## How do Codex user, repository, admin, and system skills relate to CLROOM?

**Common ways people ask this:**

- `Codex global skills`
- `Codex user skills`
- `Codex skill locations`
- `Codex .agents/skills`
- `Codex skill precedence`

Codex skill scopes are separate from `AGENTS.md`. CLROOM's selected-skill workflow concerns personal-global skills admitted for one launch. It should not be described as controlling every administrator or system skill mechanism.

**Go deeper:** [Codex](codex.md) · [Configuration Matrix](configuration-matrix.md) · [Limitations](limitations.md)

<a id="hooks-plugins-mcp-and-apps"></a>
## Are hooks, plugins, MCP servers, or apps changing agent behavior?

**Common ways people ask this:**

- `disable coding agent hooks temporarily`
- `disable plugins one session`
- `Claude MCP conflict`
- `clean launch without hooks`
- `which plugin is affecting my agent`

They can, but Claude Code and Codex do not expose one universal scope model for all of them. CLROOM has provider-specific clean defaults; use the provider pages for what is off by default, what native controls exist, and what CLROOM does not claim.

**Go deeper:** [Claude Code](claude-code.md) · [Codex](codex.md) · [Configuration Matrix](configuration-matrix.md)

<a id="different-projects-and-workflows"></a>
## Does one personal agent setup fit every project or workflow?

**Common ways people ask this:**

- `different coding agent rules per project`
- `global agent setup doesn't fit every project`
- `different skills per project`
- `planning skills vs debugging skills`
- `personal global rules conflict with repository rules`

Often it does not. CLROOM is useful when instructions or skills that help one kind of work should not automatically participate in another, while reusable selected skill sets can still be brought in for the launch that needs them.

**Go deeper:** [When To Use Clroom](when-to-use-clroom.md) · [Faq](faq.md)

<a id="testing-and-reproducibility"></a>
## Are you trying to reproduce a bug or test whether a skill changed the result?

**Common ways people ask this:**

- `reproduce Claude Code bug clean environment`
- `reproduce Codex bug clean environment`
- `test prompt without my config`
- `reproducible skill testing`
- `debug coding agent customizations`

A clean/selective launch can provide a more repeatable baseline without destructive renaming or editing of the normal setup. It does not make model output deterministic, but it can remove known personal-global variables from the comparison.

**Go deeper:** [When To Use Clroom](when-to-use-clroom.md) · [Faq](faq.md) · [Configuration Matrix](configuration-matrix.md)

<a id="managed-enterprise-policy"></a>
## Can CLROOM override company-managed Claude Code or Codex policy?

**Common ways people ask this:**

- `Claude managed settings vs CLROOM`
- `Codex managed configuration vs CLROOM`
- `organization managed Claude settings`
- `coding agent MDM settings`
- `managed hooks MCP Claude`

No. CLROOM must never be positioned as a policy bypass. If organization policy already removes personal customization, CLROOM may add little to ordinary daily work. Managed-policy interactions should remain explicit and conservative.

**Go deeper:** [Claude Code](claude-code.md) · [Codex](codex.md) · [Configuration Matrix](configuration-matrix.md) · [Limitations](limitations.md)

<a id="keep-normal-setup-untouched"></a>
## Can you test a cleaner session without deleting or renaming your normal config?

**Common ways people ask this:**

- `test coding agent without changing config`
- `clean session without deleting config`
- `don't rename ~/.claude`
- `temporary agent configuration`
- `switch agent setup without reconfiguring`

Yes. That is a core CLROOM property: the change is launch-specific. The normal configuration remains on disk for other work.

**Go deeper:** [When To Use Clroom](when-to-use-clroom.md) · [Faq](faq.md)

<a id="development-cost-tokens-and-rework"></a>
## Can configuration conflict increase development cost?

**Common ways people ask this:**

- `coding agent wasting tokens because instructions`
- `conflicting instructions token cost`
- `reduce coding agent wasted tokens`
- `coding agent instruction conflicts cost`
- `coding agent configuration overhead`

Yes, through the work it causes rather than through a guaranteed fixed context bill: an irrelevant or conflicting instruction can influence a decision, tool call, implementation path, rework, and another review/fix cycle. CLROOM's value claim should stay on that mechanism, not a universal token-saving number.

**Go deeper:** [Faq](faq.md) · [When To Use Clroom](when-to-use-clroom.md)

<a id="what-loaded-into-the-session"></a>
## How do you know what configuration or skills were active?

**Common ways people ask this:**

- `what config did Claude load`
- `what settings sources Claude loaded`
- `show active Codex instructions`
- `which AGENTS.md loaded`
- `what influenced this agent session`

Use provider-native status or inspection tools where they exist, and CLROOM's launch summary for the controls CLROOM owns. No tool should claim it can enumerate every influence on a model response.

**Go deeper:** [Claude Code](claude-code.md) · [Codex](codex.md) · [Faq](faq.md)

<a id="agent-skills-basics-and-testing"></a>
## Are you learning or authoring Agent Skills?

**Common ways people ask this:**

- `what are Agent Skills`
- `how do Agent Skills work`
- `skill discovery activation execution`
- `test my new Agent Skill`
- `skill compatibility Claude Codex`

Start with the provider/Agent Skills documentation for the skill model itself. CLROOM becomes relevant when the next question is how to test a skill cleanly, compare with/without it, or keep unrelated personal-global skills out of the test launch.

**Go deeper:** [Faq](faq.md) · [When To Use Clroom](when-to-use-clroom.md) · [Claude Code](claude-code.md) · [Codex](codex.md)

## If your wording is different

You do not need to know the provider's exact terminology before using these docs. Start with the symptom: old rules, too many skills, a wrong implementation path, a clean baseline, a profile, a hook, MCP, or a setting you cannot place.

Search engines and AI systems can connect synonyms and related meanings; these docs therefore keep one useful answer per real problem instead of generating near-duplicate pages for every wording variation.

If the problem is still not answered, open an issue in the [CLROOM repository](https://github.com/ewgenij87snwork/clean-room-launcher). A real unanswered question is more useful than another synthetic keyword page.

---

## Want the product explanation instead of another configuration detail?

Read [Why Clean Room Launcher (CLROOM) exists](why-clroom.md), then use [When to use CLROOM — and when not to](when-to-use-clroom.md) for the decision against native alternatives.
