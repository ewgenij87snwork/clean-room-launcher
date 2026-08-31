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
clroom codex --skill-set=my-skill,@my-skill-set
```

Then test other skills on the same task to compare the results, token use, and time:

```sh
clroom codex --skill-set=superpowers
```

Then repeat the test with the other supported coding agent in the same simple way:

```sh
clroom claude --skill-set=@skill-set
```

Your project context stays available. Your normal setup stays untouched.

You configure only this launch.
