use std::process::Command;

#[path = "../../src/cli/screen.rs"]
#[allow(dead_code)]
#[allow(clippy::duplicate_mod)]
mod screen;

fn ready() -> screen::PrepareReady {
    screen::PrepareReady { provider: "Codex" }
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
    assert!(
        screen.contains("Launch Codex")
            || screen.contains("clroom codex [ARGS...] to launch explicitly")
    );
}

#[test]
fn captured_non_tty_run_emits_exact_local_continuity_without_a_prompt() {
    // Break caught: automation receives an interactive or credential-acquisition route.
    let output = Command::new(env!("CARGO_BIN_EXE_clroom")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let screen = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        screen,
        include_str!("../../fixtures/cli/first-screen-unqualified-non-tty.txt")
    );
    assert!(screen.lines().all(|line| line.chars().count() <= 80));
    assert!(screen.contains("already installed local CLI"));
    assert!(!screen.contains("Enter"));
    assert!(!screen.contains('›'));
    assert_zero_auth_actions(&screen);
}

#[test]
fn interactive_tty_has_the_exact_launch_confirmation_transcript() {
    // Break caught: the guided action stops being an explicit isolated launch confirmation.
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
fn enter_confirms_launch_and_every_other_input_stops() {
    assert_eq!(
        screen::parse_unqualified_action(""),
        screen::UnqualifiedAction::LaunchCodex
    );
    assert_eq!(
        screen::parse_unqualified_action("q"),
        screen::UnqualifiedAction::Stop
    );
}
