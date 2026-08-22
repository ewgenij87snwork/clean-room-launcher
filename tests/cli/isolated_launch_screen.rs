#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project =
            std::env::temp_dir().join(format!("clroom-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&project).unwrap();
        Self(project)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn isolated_preview_renders_the_accepted_plain_launch_receipt() {
    // Break caught: the approved receipt geometry, spacing, or boundary claims drift.
    let output = screen::render_isolated_preview_for(
        Path::new("/private/tmp/clroom-preview-project"),
        0,
        screen::PlaqueFeatureState::default(),
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    assert_eq!(
        output,
        "\n\n\n\
╓──○──╖ ╭─ CLEAN ROOM ──────────────────╮\n\
║░░░░░║⠒│                               │\n\
║░░░░░║⠒│     Global AGENTS.md  off     │\n\
║░░░░░║⠒│     Global skills     off     │\n\
║░░░░░║⠒│     Apps              off     │\n\
║░░░░░║⠒│     Hooks/plugins     off     │\n\
║░░░░░║⠒│     Dev prompt        off     │\n\
║░░░░░║⠒│     Notifications     off     │\n\
║░░░░░║⠒│                               │\n\
╙──○──╜ ╰────────────── v0.1.0-alpha.3 ─╯\n"
    );
}

#[test]
fn isolated_preview_styles_only_the_visual_hierarchy() {
    // Break caught: ANSI styling changes the receipt text or its five-cell inset.
    let output = screen::render_isolated_preview_for(
        Path::new("/tmp/project"),
        0,
        screen::PlaqueFeatureState::default(),
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: false,
        },
    )
    .join("\n");

    assert!(output.starts_with("\n\n\n\u{1b}[2m╓──○──╖\u{1b}[0m "));
    assert!(output.contains("\u{1b}[1;36mCLEAN ROOM\u{1b}[0m"));
    assert!(output.contains("\u{1b}[2m╙──○──╜ ╰────────────── v0.1.0-alpha.3 ─╯\u{1b}[0m"));
    assert!(
        output.contains("\u{1b}[2m║░░░░░║⠒│\u{1b}[0m     \u{1b}[1mGlobal AGENTS.md\u{1b}[0m  off")
    );
    assert!(!output.contains("/tmp/project"));
}

#[test]
fn isolated_preview_reports_the_number_of_selected_global_skills() {
    // Break caught: the plaque claims every global skill is off even though
    // this one launch admitted an explicit selection.
    let output = screen::render_isolated_preview_for(
        Path::new("/tmp/project"),
        12,
        screen::PlaqueFeatureState::default(),
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    assert!(output.contains("     Global skills   12 on     "));
    assert!(!output.contains("Global skills     off"));
}

#[test]
fn isolated_preview_adds_an_untitled_card_for_valid_project_skills_only() {
    // Break caught: project-local skills remain available but the launch receipt
    // either hides that value or counts arbitrary directories as skills.
    let project = TempProject::new("project-skills-card");
    for name in ["planning", "review"] {
        let skill = project.path().join(".agents/skills").join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
    }
    fs::create_dir_all(project.path().join(".agents/skills/not-a-skill")).unwrap();

    let output = screen::render_isolated_preview_for(
        project.path(),
        0,
        screen::PlaqueFeatureState::default(),
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    let expected_attachment = [
        "╙──○──╜ ╰───────╥───────────────╥───────╯",
        "        ╭───────╨───────────────╨───────╮",
        "        │     Project skills   2 on     │",
        "        ╰────────────── v0.1.0-alpha.3 ─╯",
    ]
    .join("\n");
    assert!(
        output.contains(&expected_attachment),
        "unexpected project-skills plaque:\n{output}"
    );
    assert!(!output.contains("╭─ Project"));
}

#[test]
fn isolated_preview_reports_explicit_apps_hooks_and_plugins_overrides() {
    // Break caught: the plaque keeps claiming clean defaults after the owner
    // explicitly re-enables one or more native Codex features for this launch.
    let cases = [
        (&["--enable", "apps"][..], "on", "off"),
        (&["--enable", "hooks"][..], "off", "on/off"),
        (&["--enable", "plugins"][..], "off", "off/on"),
        (
            &["--enable", "hooks", "--enable", "plugins"][..],
            "off",
            "on",
        ),
    ];

    for (args, apps, hooks_plugins) in cases {
        let args = args
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        let state = screen::PlaqueFeatureState::from_provider_args(&args);
        let output = screen::render_isolated_preview_for(
            Path::new("/tmp/project"),
            0,
            state,
            screen::RenderContext {
                width: 100,
                interactive: true,
                plain: true,
            },
        )
        .join("\n");

        assert!(output.contains(&format!("     Apps{apps:>17}     ")));
        assert!(output.contains(&format!("     Hooks/plugins{hooks_plugins:>8}     ")));
    }
}

#[test]
fn isolated_preview_uses_the_last_override_and_stops_at_double_dash() {
    // Break caught: stale earlier flags or prompt text after `--` change the
    // visible receipt instead of the effective explicit launch controls.
    let args = [
        "--enable",
        "apps",
        "--disable",
        "apps",
        "--enable=hooks",
        "--disable=hooks",
        "--",
        "--enable",
        "plugins",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let state = screen::PlaqueFeatureState::from_provider_args(&args);
    let output = screen::render_isolated_preview_for(
        Path::new("/tmp/project"),
        0,
        state,
        screen::RenderContext {
            width: 100,
            interactive: true,
            plain: true,
        },
    )
    .join("\n");

    assert!(output.contains("     Apps              off     "));
    assert!(output.contains("     Hooks/plugins     off     "));
}
