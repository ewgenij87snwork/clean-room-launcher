---
layout: page
title: Skill sets
description: Create, edit, use, and combine reusable Clean Room Launcher (CLROOM) skill sets for Codex and Claude Code launches.
permalink: /skill-sets/
nav_title: Skill sets
---
CLROOM skill sets are user-created groups of global skill selectors. They let you reuse a selection without rewriting your normal provider setup.

## Create a skill set

Run:

```sh
clroom --help
```

It shows the exact skill-set file path for your setup. The normal path is:

```text
~/.config/clroom/skill-sets.yaml
```

Create or edit that YAML file and define reusable groups:

```yaml
review:
  - my-review-skill
  - my-security-review

bugfix:
  - superpowers:systematic-debugging
  - my-regression-check

brainstorming:
  - my-design-skill
  - superpowers:brainstorming
```

The `my-*` names are placeholders for global skills you installed or created. The `superpowers:*` examples assume those skills are already installed in a global skill location CLROOM can discover. CLROOM groups selectors; it does not install skills.

For the built-in selector example, run:

```sh
clroom help skill-set
```

## Use a skill set

Reference a saved group with `@set-name`:

```sh
clroom codex --skill-set=@review

clroom codex --skill-set=@bugfix

clroom claude --skill-set=@brainstorming
```

The same user-created set mechanism is available through both CLROOM launchers. Codex and Claude Code remain different provider environments.

## Combine skills and skill sets

Use more than one saved set in the same launch:

```sh
clroom codex --skill-set=@review,@bugfix
```

Or combine a direct global skill with a saved set:

```sh
clroom codex --skill-set=my-skill,@review
```

Selectors are comma-separated. Repeated and overlapping selectors are deduplicated, and the selection applies only to this launch.

## Edit a skill set

Open the same YAML file, add or remove selectors under the set name, and keep using the same `@set-name`.

CLROOM reads the file when an `@set` is used. It does not create or rewrite the file for you.

## Selector rules

- A bare `name` selects the logical global skill with that name, or every skill in a namespace with that name.
- `namespace:skill` selects one specific skill from a namespace.
- `@set-name` selects a user-created saved group from the YAML file.
- Direct skills and saved sets can be combined in one comma-separated `--skill-set` value.
- Multiple saved sets can be combined in one launch.
- Repeated and overlapping selectors are deduplicated.
- Saved sets cannot contain other `@sets`.
- Invalid or unknown selections stop before the provider starts.
- The YAML file is read only when an `@set` is used.
- CLROOM never creates or rewrites the YAML file.
- Selections apply only to the current launch.
- Project-local skills remain available automatically.
