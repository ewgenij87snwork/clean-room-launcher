<p align="center">
  <img src="docs/assets/brand/clroom-logo.png" width="80" alt="CLROOM logo">
</p>

<h1 align="center">Clean Room Launcher (CLROOM)</h1>

<p align="center">
  <a href="https://ewgenij87snwork.github.io/clean-room-launcher/">Documentation →</a>
</p>

<p align="center">
  <a
    href="#use-the-global-skills-you-need-without-loading-the-rest"
  >Choose skills</a> ·
  <a href="#see-the-boundary-as-codex-starts">See the boundary</a> ·
  <a href="#install-in-sixty-seconds">Install</a> ·
  <a href="#launch">Launch</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#trust-boundary">Trust boundary</a> ·
  <a href="#frequently-asked-questions">FAQ</a> ·
  <a href="#remove">Remove</a>
</p>

## What?

> **A clean-room launcher for Codex and Claude Code on macOS.**

## Why?

> **One word instead of many long parameters.**<br>
> **Bring only the skills you need into your empty place.**

## Launch your coding agent without unrelated instructions and skills.

Coding-agent CLIs can load global rules, skills from other work, and forgotten
instructions from outside the current project. `clroom` keeps them out of this
launch while your project context stays available.

Your existing setup stays untouched.

Use the automatic clean launch immediately.

## Use the global skills you need without loading the rest.

Project-local skills are available automatically.

Global skills stay outside unless you add them for this launch.

```sh
clroom codex --skill-set=my-skill,@my-skill-set --approve-for-me

clroom claude --skill-set=my-skill,@my-skill-set
```

Use skill names installed in your own setup.

### Skill sets

For the full practical guide, see [Skill sets](docs/skill-sets.md).

Run `clroom help skill-set` for selector and saved-set examples.

Run `clroom --help` to see the exact skill-set file path. It is normally
`~/.config/clroom/skill-sets.yaml`.

Open it and create skill groups you can reuse by name:

```yaml
my-skill-set:
  - my-first-skill
  - my-second-skill

feature-planning:
  - superpowers:brainstorming
  - superpowers:writing-plans
```

<details>
<summary>Exact selector behavior</summary>

- A bare `name` admits the logical global skill with that name, or every skill
  in a namespace with that name.
- `namespace:skill` admits one specific skill from a namespace.
- `@set-name` admits a saved group from the YAML file.
- Invalid or unknown selectors stop before the selected provider starts.
- Selections apply to this launch only.
- Repeated and overlapping selectors admit each logical skill once.
- If the same logical skill exists in multiple discovered roots, the provider's
  documented root precedence chooses one.
- Saved groups cannot include other saved groups.
- `clroom` reads the YAML file only when you use an `@set` and never creates or
  rewrites it.

</details>

## See the boundary as Codex starts

Run `clroom codex --skill-set=my-skill,@my-skill-set` from the directory where
you want to work.

Before Codex takes over the terminal, Clean Room Launcher shows a compact
summary of the active boundary:

- global Codex `AGENTS.md` files are blocked;
- unselected global skill contents stay blocked;
- apps, hooks, and plugins are off by default;
- developer instructions and notifications are cleared by default.

```text
╓──○──╖ ╭─ CLEAN ROOM ─ v0.1.0-alpha.4.2 ─╮
║░░░░░║⠒│                               │
║░░░░░║⠒│     Global AGENTS.md  off     │
║░░░░░║⠒│     Global skills    3 on     │
║░░░░░║⠒│     Apps              off     │
║░░░░░║⠒│     Hooks/plugins     off     │
║░░░░░║⠒│     Dev prompt        off     │
║░░░░░║⠒│     Notifications     off     │
║░░░░░║⠒│                               │
╙──○──╜ ╰───────────╥───────╥───────────╯
        ╭───────────╨───────╨───────────╮
        │     Project skills   2 on     │
        ╰───────────────────────────────╯
```

The main plaque reports the global boundary and admitted global-skill count.
When project-local skills are present in `.agents/skills`, the separate card
shows how many remain available; with none, the card is omitted. Project
context and explicit Codex arguments remain available.

Codex then starts immediately in the same terminal. There is no menu,
confirmation step, second launch, or artificial delay.

## Install in sixty seconds

You need macOS on Apple Silicon and at least one already working provider:
Codex CLI `0.147.0+` or Claude Code CLI `2.1.223+`.

```sh
VERSION=v0.1.0-alpha.4.2
ASSET=clean-room-launcher-v0.1.0-alpha.4.2-aarch64-apple-darwin.tar.gz

curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/$ASSET"
curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/SHA256SUMS"
shasum -a 256 -c SHA256SUMS
tar -xzf "$ASSET"

mkdir -p "$HOME/.local/bin"
install -m 0755 "clean-room-launcher-v0.1.0-alpha.4.2-aarch64-apple-darwin/bin/clroom" "$HOME/.local/bin/clroom"
export PATH="$HOME/.local/bin:$PATH"
clroom --help
```

The `export` makes `clroom` available in the current terminal. To keep it
available in new terminals, add `$HOME/.local/bin` to your shell's `PATH`.

The archive is unsigned and unnotarized. If local macOS policy refuses it,
prefer the Cargo installation below. Do not disable Gatekeeper globally.

## Install with Cargo

Rust users can build the same alpha from the public tag:

```sh
cargo install --git https://github.com/ewgenij87snwork/clean-room-launcher \
  --tag v0.1.0-alpha.4.2 --locked
```

No crates.io package is published for this alpha.

## Launch

### Codex

For a low-friction Codex launch with eligible approval requests handled by
Codex Auto-review:

```sh
cd your-project
clroom codex --approve-for-me
```

`--approve-for-me` is a Codex option. It keeps the Codex workspace sandbox and
routes eligible approval requests through its automatic reviewer. Availability
and reviewer behavior are controlled by the installed Codex version and
account.

If you prefer to review approval requests yourself:

```sh
clroom codex
```

Ordinary Codex arguments pass through unchanged:

```sh
clroom codex --help
clroom codex --enable apps --enable hooks --enable plugins
```

### Claude Code

Start Claude Code with the same clean boundary:

```sh
cd your-project
clroom claude
```

Project-local Claude skills remain available automatically. Add selected
global skills for this launch with the same selector:

```sh
clroom claude --skill-set=my-skill,@my-skill-set
```

## How it works

1. **Resolve the provider locally.** Clean Room Launcher finds the installed
   `codex` or `claude` executable through `PATH`. It does not install or replace
   either CLI.

2. **Establish the boundary.** It creates a narrow macOS Seatbelt policy that
   denies reads of the provider's global instruction files and unselected
   ambient skill contents.

3. **Admit your selected skills.** Direct global skill selectors and named
   `@sets` composed with `--skill-set=` are readable for this launch only.
   Project-local skills remain available automatically.

4. **Apply clean defaults.** Codex starts with apps, hooks, and plugins off,
   empty developer instructions, and no notifications. Claude starts without
   global `CLAUDE.md`, user settings, or auto memory.

5. **Show the boundary.** The launcher prints the compact `CLEAN ROOM` status
   plaque, admitted global-skill count, and a project-skill card when local
   skills are present.

6. **Launch the original CLI.** Codex retains native foreground terminal
   behavior. Claude is supervised only long enough to bind its private skill
   projection to the real consumer process and clean it safely on exit.

<details>
<summary><strong>Crash-safe Claude skill cleanup</strong></summary>

Each Claude launch gets a private, session-scoped skill projection. On a normal
exit, `clroom` moves that projection to quarantine and removes it.
If cleanup is interrupted, a later Claude launch retries only recognized
`clroom` state whose recorded consumer process is confirmed dead. Live,
unknown, malformed, and legacy state is left untouched. Cleanup removes
projection links, not your installed skill sources.

</details>

Clean Room Launcher makes no model request and performs no provider login before
the selected CLI starts.

## What it changes and what it leaves alone

| For this launch | Left unchanged |
|---|---|
| Global provider instruction files are unavailable | Those files remain untouched on disk |
| Unselected global skill contents are unavailable | Existing skills remain untouched on disk |
| Selected global skills are readable for one launch | No skill is copied, installed, or enabled permanently |
| Apps, hooks, and plugins are off by default | Explicit user arguments can re-enable them |
| Provider-specific ambient settings are disabled by default | Provider configuration is not rewritten |
| The selected project remains available | Project files, Git history, and project instructions remain untouched |
| The provider starts inside the launcher boundary | Installation, login, and provider state remain provider-owned |

Clean Room Launcher does not need to copy authentication data into its own
configuration. It does not open a browser or ask you to sign in.

## Trust boundary

Clean Room Launcher provides a focused context boundary. It is not a virtual
machine, container, network sandbox, complete home-directory sandbox,
permission broker, coding-agent proxy, or hosted coding service.

Its macOS Seatbelt policy blocks the documented global instruction files and
the contents of known global skill roots, while allowing the root listing the
provider needs for discovery. It then admits only the resolved skill directories
you selected. Other project and host paths remain available unless macOS or the
provider applies another restriction.

The launcher does not make unsafe commands safe and does not replace provider
sandbox or approval controls.

Codex may display `Operation not permitted` when it probes a blocked global
`AGENTS.md` file. That warning is expected: the clean-room boundary denied the
read. It does not mean Codex failed to start.

If the required macOS isolation boundary cannot be created, Clean Room Launcher
fails instead of silently starting a normal inherited Codex session.

See [the current limitations](docs/limitations.md) and
[security policy](SECURITY.md).

<details>
<summary><strong>Why not just use provider flags or profiles?</strong></summary>

Native provider controls may be the better fit when you only need one
provider's own configuration. `clroom` gives Codex and Claude Code one
repeatable way to launch: project-local context stays available, the documented
global inputs stay outside, and only the global skills you select are admitted
for that launch. It does not replace either provider or rewrite its saved
configuration.

See the official [Claude Code CLI reference][claude-cli-reference] and
[Codex configuration reference][codex-config-reference].

</details>

## Coding-agent support

The current alpha supports two macOS paths:

| Coding agent | Platform | Status |
|---|---|---|
| Codex CLI 0.147.0+ | macOS / Apple Silicon | Alpha |
| Claude Code CLI 2.1.223+ | macOS / Apple Silicon | Alpha |

Linux and Windows are `NOT_QUALIFIED`. Intel macOS, Homebrew, crates.io,
signing, and notarization are not supported by this release.

Additional coding agents and platforms may be considered later, but this README
makes no support claim for them.

## Documentation

For exact provider behavior, native alternatives, current limitations, and common problem wording:

- [Why CLROOM exists](docs/why-clroom.md)
- [Coding-agent configuration problem index](docs/problem-index.md)
- [When to use Clean Room Launcher (CLROOM) — and when not to](docs/when-to-use-clroom.md)
- [Use cases](docs/use-cases.md)
- [Skill sets](docs/skill-sets.md)
- [Claude Code and CLROOM](docs/claude-code.md)
- [Codex and CLROOM](docs/codex.md)
- [Configuration matrix](docs/configuration-matrix.md)
- [FAQ](docs/faq.md)
- [Limitations](docs/limitations.md)
- [Threat model](docs/threat-model.md)

The documentation intentionally recommends native provider features when they are the simpler correct option.

## Frequently asked questions

### Does it delete or rewrite my existing setup?

No. Existing instructions, skills, provider settings, and other projects stay where
they are. The boundary applies only to the launched process.

### Does it remove all context from Codex?

No. Your selected project, project-local skills, and any global skills you
explicitly selected remain available. The provider also keeps its own built-in
behavior. Clean Room Launcher keeps the other specified ambient global inputs
outside this launch.

### Do I need another account or subscription?

No. Clean Room Launcher starts your existing CLI. Codex or Claude continues to
own its account, subscription, authentication, and provider connection.

### Does Clean Room Launcher read or copy my credentials?

No. It does not request, inspect, or copy provider credentials. The selected CLI
accesses its existing provider state itself after launch.

### Is this a complete operating-system sandbox?

No. It uses macOS Seatbelt to enforce a narrow filesystem denylist. It does not
block the network, isolate every home-directory file, or replace a VM or
container.

### Can I see what was cleaned?

Yes. The launch plaque shows the active boundary categories, admitted global
skills, and—when present—the project-local skill count before the provider starts.

This alpha does not yet provide a per-file review interface or compiled-context
manifest.

### Can I override the clean defaults?

Yes. Explicit Codex arguments can re-enable apps, hooks, and plugins or replace
the cleared Codex configuration values.

They do not disable the launcher's filesystem boundary around global
instructions and unselected skill roots. Use `--skill-set=` to admit direct
skill selectors or named `@sets` for one launch.

### Why does Codex show `Operation not permitted`?

Codex may probe a global `AGENTS.md` file during startup. The warning proves
that the clean-room boundary blocked the read. Review other errors normally.

## Remove

For an archive installation:

```sh
rm "$HOME/.local/bin/clroom"
```

For a Cargo installation:

```sh
cargo uninstall clean-room-launcher
```

There is no daemon, service, account, or system-wide configuration to remove.
Removing Clean Room Launcher does not modify either provider or its authentication.

## Project status

`v0.1.0-alpha.4.2` is a public, unsigned, and unnotarized prerelease for macOS
on Apple Silicon.

It supports Codex CLI `0.147.0+` and Claude Code CLI `2.1.223+` through a
focused clean-room boundary. It is not a stable-support promise.

See the
[GitHub prerelease](https://github.com/ewgenij87snwork/clean-room-launcher/releases/tag/v0.1.0-alpha.4.2)
for the archive and `SHA256SUMS`.

## Help improve Clean Room Launcher

If Clean Room Launcher makes your coding-agent sessions easier to trust,
[star the repository](https://github.com/ewgenij87snwork/clean-room-launcher).
It helps other Codex and Claude Code users find it.

Report bugs or request features in
[GitHub Issues](https://github.com/ewgenij87snwork/clean-room-launcher/issues).
For vulnerability reports, follow the private-reporting instructions below.

## Support CLROOM

If CLROOM belongs in your workflow, you can support continued development and testing:

- [Patreon](https://www.patreon.com/CLROOM)
- [Direct support](https://send.monobank.ua/jar/9UUyaEo717)


## Security

Please do not put credentials, private instructions, prompts, transcripts,
private paths, or exploit details in a public issue.

Use **Security → Report a vulnerability** in the GitHub repository. If private
reporting is unavailable, open a minimal public issue asking the maintainer to
provide a private channel.

This project does not offer a vulnerability bounty.

## License

Clean Room Launcher is open-source software under the
[Mozilla Public License 2.0](LICENSE).

Clean Room Launcher is an independent project and is not affiliated with or
endorsed by OpenAI or Anthropic.

[claude-cli-reference]: https://code.claude.com/docs/en/cli-reference
[codex-config-reference]: https://developers.openai.com/codex/config-reference
