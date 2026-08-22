# Clean Room Launcher

![Clean Room Launcher keeps ambient global inputs outside while project context, CLI arguments, and selected global skills reach Codex.](docs/assets/clean-room-launcher-hero.png)

A clean-room launcher for coding agents (Codex for now).

## Launch your coding agent without unrelated instructions and skills.

Codex can load global rules, skills from other work, and forgotten instructions
from outside the current project. `clroom` keeps them out of this launch while
your project context stays available.

Your existing setup stays untouched.

Use the automatic clean launch immediately.

## Use the global skills you need without loading the rest.

Project-local skills are available automatically.

Global skills stay outside unless you add them for this launch.

```sh
clroom codex --skill-set=any-my-skill,@any-my-skill-set --approve-for-me
```

Use skill names installed in your own setup.

### Skill sets

Run `clroom --help` to see the exact skill-set file path. It is normally
`~/.config/clroom/skill-sets.yaml`.

Open it and create skill groups you can reuse by name:

```yaml
any-my-skill-set:
  - any-first-skill
  - any-second-skill

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
- Invalid or unknown selectors stop before Codex starts.
- Selections apply to this launch only.
- Repeated and overlapping selectors admit each logical skill once.
- If the same logical skill exists in multiple discovered roots, Codex root
  precedence chooses one.
- Saved groups cannot include other saved groups.
- `clroom` reads the YAML file only when you use an `@set` and never creates or
  rewrites it.

</details>

## See the boundary as Codex starts

Run `clroom codex` from the directory where you want to work.

Before Codex takes over the terminal, Clean Room Launcher shows a compact
summary of the active boundary:

- global Codex `AGENTS.md` files are blocked;
- unselected global skill contents stay blocked;
- apps, hooks, and plugins are off by default;
- developer instructions and notifications are cleared by default.

```text
╓──○──╖ ╭─ CLEAN ROOM ──────────────────╮
║░░░░░║⠒│                               │
║░░░░░║⠒│     Global AGENTS.md  off     │
║░░░░░║⠒│     Global skills    3 on     │
║░░░░░║⠒│     Apps              off     │
║░░░░░║⠒│     Hooks/plugins     off     │
║░░░░░║⠒│     Dev prompt        off     │
║░░░░░║⠒│     Notifications     off     │
║░░░░░║⠒│                               │
╙──○──╜ ╰───────╥───────────────╥───────╯
        ╭───────╨───────────────╨───────╮
        │     Project skills   2 on     │
        ╰────────────── v0.1.0-alpha.3 ─╯
```

The main plaque reports the global boundary and admitted global-skill count.
When project-local skills are present in `.agents/skills`, the separate card
shows how many remain available; with none, the card is omitted. Project
context and explicit Codex arguments remain available.

Codex then starts immediately in the same terminal. There is no menu,
confirmation step, second launch, or artificial delay.

## Install in sixty seconds

You need macOS on Apple Silicon and an already installed, working `codex` CLI.

```sh
VERSION=v0.1.0-alpha.3
ASSET=clean-room-launcher-v0.1.0-alpha.3-aarch64-apple-darwin.tar.gz

curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/$ASSET"
curl -fLO "https://github.com/ewgenij87snwork/clean-room-launcher/releases/download/$VERSION/SHA256SUMS"
shasum -a 256 -c SHA256SUMS
tar -xzf "$ASSET"

mkdir -p "$HOME/.local/bin"
install -m 0755 "clean-room-launcher-v0.1.0-alpha.3-aarch64-apple-darwin/bin/clroom" "$HOME/.local/bin/clroom"
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
  --tag v0.1.0-alpha.3 --locked
```

No crates.io package is published for this alpha.

## Start clean

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

## How it works

1. **Resolve Codex locally.** Clean Room Launcher finds the installed `codex`
   executable through `PATH`. It does not install or replace Codex.

2. **Establish the boundary.** It creates a narrow macOS Seatbelt policy that
   denies reads of global Codex instruction files and unselected ambient skill
   contents.

3. **Admit your selected skills.** Direct global skill selectors and named
   `@sets` composed with `--skill-set=` are readable for this launch only.
   Project-local skills remain available automatically.

4. **Apply clean defaults.** It starts Codex with apps, hooks, and plugins off,
   empty developer instructions, and no notifications.

5. **Show the boundary.** The launcher prints the compact `CLEAN ROOM` status
   plaque, admitted global-skill count, and a project-skill card when local
   skills are present.

6. **Launch the original CLI.** Clean Room Launcher replaces itself with Codex.
   Codex remains the foreground process and retains normal terminal input,
   output, signals, and exit behavior.

Clean Room Launcher makes no model request and performs no provider login before
Codex starts.

## What it changes and what it leaves alone

| For this launch | Left unchanged |
|---|---|
| Global Codex instruction files are unavailable | Those files remain untouched on disk |
| Unselected global skill contents are unavailable | Existing skills remain untouched on disk |
| Selected global skills are readable for one launch | No skill is copied, installed, or enabled permanently |
| Apps, hooks, and plugins are off by default | Explicit user arguments can re-enable them |
| Developer instructions and notifications are cleared by default | Codex configuration is not rewritten |
| The selected project remains available | Project files, Git history, and project instructions remain untouched |
| Codex starts inside the launcher boundary | Codex installation, login, and provider state remain owned by Codex |

Clean Room Launcher does not need to copy authentication data into its own
configuration. It does not open a browser or ask you to sign in.

## Trust boundary

Clean Room Launcher provides a focused context boundary. It is not a virtual
machine, container, network sandbox, complete home-directory sandbox,
permission broker, coding-agent proxy, or hosted coding service.

Its macOS Seatbelt policy blocks the documented global instruction files and
the contents of known global skill roots, while allowing the root listing Codex
needs for discovery. It then admits only the resolved global skill directories
you selected. Other project and host paths remain available unless macOS or
Codex applies another restriction.

The launcher does not make unsafe commands safe and does not replace Codex
sandbox or approval controls.

Codex may display `Operation not permitted` when it probes a blocked global
`AGENTS.md` file. That warning is expected: the clean-room boundary denied the
read. It does not mean Codex failed to start.

If the required macOS isolation boundary cannot be created, Clean Room Launcher
fails instead of silently starting a normal inherited Codex session.

See [the current limitations](docs/limitations.md) and
[security policy](SECURITY.md).

## Coding-agent support

The current alpha supports one path:

| Coding agent | Platform | Status |
|---|---|---|
| Locally installed Codex CLI | macOS / Apple Silicon | Alpha |

Claude, Linux, Windows, Intel macOS, Homebrew, crates.io, signing, and
notarization are not supported by this release.

Additional coding agents and platforms may be considered later, but this README
makes no support claim for them.

## Frequently asked questions

### Does it delete or rewrite my existing setup?

No. Existing instructions, skills, Codex settings, and other projects stay where
they are. The boundary applies only to the launched process.

### Does it remove all context from Codex?

No. Your selected project, project-local skills, and any global skills you
explicitly selected remain available. Codex also keeps its own built-in
behavior. Clean Room Launcher keeps the other specified ambient global inputs
outside this launch.

### Do I need another account or subscription?

No. Clean Room Launcher starts your existing Codex CLI. Codex continues to own
its account, subscription, authentication, and provider connection.

### Does Clean Room Launcher read or copy my credentials?

No. It does not request, inspect, or copy provider credentials. Codex accesses
its existing provider state itself after launch.

### Is this a complete operating-system sandbox?

No. It uses macOS Seatbelt to enforce a narrow filesystem denylist. It does not
block the network, isolate every home-directory file, or replace a VM or
container.

### Can I see what was cleaned?

Yes. The launch plaque shows the active boundary categories, admitted global
skills, and—when present—the project-local skill count before Codex starts.

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
Removing Clean Room Launcher does not modify Codex or its authentication.

## Project status

`v0.1.0-alpha.3` is a public, unsigned, and unnotarized prerelease for macOS
on Apple Silicon.

It supports the locally installed Codex CLI through a focused clean-room
boundary. It is not a stable-support promise.

See the
[GitHub prerelease](https://github.com/ewgenij87snwork/clean-room-launcher/releases/tag/v0.1.0-alpha.3)
for the archive and `SHA256SUMS`.

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
endorsed by OpenAI.
