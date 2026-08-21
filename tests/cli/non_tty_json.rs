use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(args)
        .output()
        .expect("clroom must run")
}

#[test]
fn leading_json_selector_emits_only_the_versioned_guided_object() {
    // Break caught: automation receives human transcript, ANSI, or more than one JSON document.
    let output = run(&["--output", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("../../fixtures/cli/output/guided.json")
    );
}

#[test]
fn non_interactive_init_refuses_instead_of_waiting_for_a_choice() {
    // Break caught: a piped mutation blocks or silently mutates project state.
    let output = run(&["init"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "LOCAL_LIFECYCLE_UNAVAILABLE: init is not implemented in this build; use clroom codex for the minimum isolated launch\n"
    );
}

#[test]
fn deprecated_json_spelling_refuses_safely() {
    // Break caught: the superseded machine selector silently regains support.
    let output = run(&["--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "UNKNOWN_COMMAND: --json; try help\n"
    );
}
