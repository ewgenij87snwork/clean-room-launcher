use std::{fs, process::Command};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
mod screen;

fn ready() -> screen::PrepareReady {
    screen::PrepareReady {
        provider: "Codex",
        project: "project://offerstream",
        preview: "18 KB at start · 43 KB less (read-only measurement)",
        skills: "5 summaries now · 38 load on use · 4 unavailable",
    }
}

fn transcript(context: screen::RenderContext) -> String {
    format!(
        "{}\n",
        screen::render_unqualified_for(ready(), context).join("\n")
    )
}

fn assert_zero_auth_actions(screen: &str) {
    let lower = screen.to_ascii_lowercase();
    for prohibited in [
        "login",
        "log in",
        "sign in",
        "setup",
        "settings",
        "configure",
        "configuration",
        "preferences",
        "api key",
        "api-key",
        "documentation",
    ] {
        assert!(
            !lower.contains(prohibited),
            "unqualified transcript exposed prohibited CTA fragment {prohibited:?}:\n{screen}"
        );
    }
    assert!(screen.contains("Continue locally"));
}

#[cfg(target_os = "macos")]
#[test]
fn real_tty_enter_dispatches_the_taskseal_owned_local_continuation() {
    // Break caught: the interactive CTA is only rendered and the real CLI exits before Enter.
    let output = Command::new("/usr/bin/expect")
        .args([
            "-c",
            concat!(
                "set timeout 5\n",
                "spawn -noecho $env(TSEAL_TEST_BIN)\n",
                "expect {\n",
                "  \"Enter continue locally\" {}\n",
                "  timeout { exit 124 }\n",
                "  eof { exit 125 }\n",
                "}\n",
                "send \"\\r\"\n",
                "expect eof\n",
            ),
        ])
        .env("TSEAL_TEST_BIN", env!("CARGO_BIN_EXE_tseal"))
        .env("COLUMNS", "80")
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let transcript = String::from_utf8(output.stdout).unwrap().replace('\r', "");
    let screen_start = transcript
        .find("TaskSeal · provider qualification required")
        .expect("real PTY output must select the interactive screen");
    let session = &transcript[screen_start..];
    assert!(
        session.starts_with(include_str!(
            "../../fixtures/cli/first-screen-unqualified-tty.txt"
        )),
        "real PTY transcript did not match the exact TTY fixture:\n{session}"
    );
    assert!(
        session.contains("tseal: command accepted\n"),
        "real Enter did not reach the TaskSeal-owned local dispatcher:\n{session}"
    );
    assert_zero_auth_actions(session);
}

#[cfg(target_os = "macos")]
#[test]
fn real_tty_second_choice_launches_the_local_codex_child() {
    let root = std::env::temp_dir().join(format!("taskseal-screen-codex-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let capture = root.join("capture");
    let codex = root.join("codex");
    fs::write(&codex, "#!/bin/sh\nprintf launched > \"$TASKSEAL_CAPTURE_PATH\"\n").unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new("/usr/bin/expect")
        .args([
            "-c",
            concat!(
                "set timeout 5\n",
                "spawn -noecho $env(TSEAL_TEST_BIN)\n",
                "expect \"2 launch Codex\"\n",
                "send \"2\\r\"\n",
                "expect eof\n",
            ),
        ])
        .env("TSEAL_TEST_BIN", env!("CARGO_BIN_EXE_tseal"))
        .env("TASKSEAL_CAPTURE_PATH", &capture)
        .env("PATH", &root)
        .env("COLUMNS", "80")
        .env("TERM", "xterm-256color")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&capture).unwrap(), "launched");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn captured_non_tty_run_emits_exact_local_continuity_without_a_prompt() {
    // Break caught: automation receives an interactive or credential-acquisition route.
    let output = Command::new(env!("CARGO_BIN_EXE_tseal")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let screen = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        screen,
        include_str!("../../fixtures/cli/first-screen-unqualified-non-tty.txt")
    );
    assert!(screen.lines().all(|line| line.chars().count() <= 80));
    assert!(screen.contains("local launcher ready"));
    assert!(!screen.contains("Enter"));
    assert!(!screen.contains('›'));
    assert_zero_auth_actions(&screen);
}

#[test]
fn interactive_tty_has_the_exact_continue_locally_transcript() {
    // Break caught: the unqualified default action starts or configures a provider.
    let actual = transcript(screen::RenderContext {
        width: 80,
        interactive: true,
        plain: false,
    });
    assert_eq!(
        actual,
        include_str!("../../fixtures/cli/first-screen-unqualified-tty.txt")
    );
    assert_zero_auth_actions(&actual);
}

#[test]
fn narrow_interactive_tty_has_an_exact_non_overflowing_transcript() {
    // Break caught: local continuity or provider state disappears during narrow reflow.
    let actual = transcript(screen::RenderContext {
        width: 40,
        interactive: true,
        plain: false,
    });
    assert_eq!(
        actual,
        include_str!("../../fixtures/cli/first-screen-unqualified-narrow.txt")
    );
    assert!(actual.lines().all(|line| line.chars().count() <= 40));
    assert_zero_auth_actions(&actual);
}

#[test]
fn plain_interactive_tty_has_exact_numbered_local_choices() {
    // Break caught: accessibility/plain mode retains cursor-painted selection or changes meaning.
    let actual = transcript(screen::RenderContext {
        width: 80,
        interactive: true,
        plain: true,
    });
    assert_eq!(
        actual,
        include_str!("../../fixtures/cli/first-screen-unqualified-plain.txt")
    );
    assert!(!actual.contains('›'));
    assert_zero_auth_actions(&actual);
}

#[test]
fn launch_codex_requires_the_explicit_second_choice() {
    assert_eq!(
        screen::parse_unqualified_action("2"),
        screen::UnqualifiedAction::LaunchCodex
    );
    assert_eq!(
        screen::parse_unqualified_action(""),
        screen::UnqualifiedAction::ContinueLocally
    );
}
