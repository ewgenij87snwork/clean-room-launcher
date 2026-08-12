use std::process::Command;

#[test]
fn bare_tseal_renders_the_approved_inline_pre_confirmation_screen() {
    // Break caught: claiming verified too early, losing the write boundary, or changing the compact menu.
    let output = Command::new(env!("CARGO_BIN_EXE_tseal")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let screen = String::from_utf8(output.stdout).unwrap();
    assert_eq!(screen, include_str!("../../fixtures/cli/first-screen.txt"));
    assert_eq!(screen.lines().filter(|line| !line.is_empty()).count(), 10);
    assert!(screen.lines().all(|line| line.chars().count() <= 80));
    assert!(!screen.contains("verified"));
}

#[test]
fn bare_tseal_stacks_labels_without_horizontal_overflow_when_narrow() {
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .env("COLUMNS", "40")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let screen = String::from_utf8(output.stdout).unwrap();
    assert!(screen.contains("Project\n/Users/alex/work/offerstream"));
    assert!(screen.contains("Writes\n.taskseal/project.json + .taskseal/out/"));
    assert!(screen.contains("Leaves\n~/.codex · ~/.claude · Git/provider"));
    assert!(screen.lines().all(|line| line.chars().count() <= 40));
}
