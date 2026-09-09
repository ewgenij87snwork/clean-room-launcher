---
layout: page
title: Use cases
description: Practical Clean Room Launcher (CLROOM) workflows for testing Agent Skills and choosing global skills for one Codex or Claude Code launch.
permalink: /use-cases/
nav_title: Use cases
---
## For skill authors: test your skills in clean launches with CLROOM

You built a skill. Test it without unrelated global instructions or skills. Alone, with a skill set you created, or both together:

```sh
clroom codex exec --skill-set=my-skill,@my-skill-set
```

Then test other skills on the same task to compare the results, token use, and time:

```sh
clroom codex exec --skill-set=superpowers
```

Then repeat the test with the other supported coding agent in the same simple way:

```sh
clroom claude --skill-set=@skill-set
```

Your project context stays available. Your normal setup stays untouched.

You configure only this launch.

## Different skills for different workers

Use separate saved groups when a planning, review, or debugging worker needs a
different personal-global skill set:

```sh
clroom codex --skill-set=@planning
clroom codex exec --skill-set=@review "Review the staged diff."
clroom claude --skill-set=@debugging
```

## Cross-provider review workflow

The same runner can qualify one task through both supported interactive provider
paths while keeping each provider's native environment and lifecycle rules.

## Clean worker launched from a script or CI

For headless automation, the qualified path in this release is `clroom codex
exec ...`. A runner that provides a terminal can also start the qualified
interactive Codex or Claude Code path. Keep provider authentication, queues,
worktrees, and session reuse in the system that owns those responsibilities;
CLROOM supplies the per-launch clean/selective layer.
