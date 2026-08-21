use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(args)
        .output()
        .expect("clroom must run")
}

#[test]
fn top_help_aliases_render_the_same_concise_index() {
    // Break caught: help aliases drift or expose a provider-option wall.
    let expected = include_str!("../../fixtures/cli/help/top.txt");
    for args in [&["help"][..], &["--help"], &["-h"]] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn command_help_aliases_render_the_same_inspect_card() {
    // Break caught: `help inspect` and command-suffix help use different contracts.
    let expected = include_str!("../../fixtures/cli/help/inspect.txt");
    for args in [
        &["help", "inspect"][..],
        &["inspect", "--help"],
        &["inspect", "-h"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn command_card_uses_its_own_usage_instead_of_a_skill_placeholder() {
    // Break caught: a generic card tells users to pass `<skill>` to `doctor`.
    let output = run(&["help", "doctor"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("../../fixtures/cli/help/doctor.txt")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn unknown_help_topic_is_stderr_only_and_actionable() {
    // Break caught: unknown help executes a command or loses the smallest safe action.
    let output = run(&["help", "not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "UNKNOWN_HELP_TOPIC: not-a-command; try \"clroom help\"\n"
    );
}
