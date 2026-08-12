use std::process::Command;

#[test]
fn bare_tseal_renders_the_conservative_pre_qualification_screen() {
    // Break caught: claiming a clean launch before P06's exact tuple qualification.
    let output = Command::new(env!("CARGO_BIN_EXE_tseal")).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let screen = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        screen,
        include_str!("../../fixtures/cli/first-screen-unqualified.txt")
    );
    assert_eq!(screen.lines().filter(|line| !line.is_empty()).count(), 10);
    assert!(screen.lines().all(|line| line.chars().count() <= 80));
    assert!(!screen.contains("verified"));
    assert!(screen.contains("P06_REQUIRED"));
    assert!(!screen.contains("Start Codex with clean context"));
}

#[test]
fn bare_tseal_stacks_labels_without_horizontal_overflow_when_narrow() {
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .env("COLUMNS", "40")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let screen = String::from_utf8(output.stdout).unwrap();
    assert!(screen.contains("Provider\nCodex · P06_REQUIRED · nothing launched"));
    assert!(screen.contains("Project\n/Users/alex/work/offerstream"));
    assert!(screen.contains("Writes\n.taskseal/project.json + .taskseal/out/"));
    assert!(screen.contains("Leaves\n~/.codex · ~/.claude · Git/provider"));
    assert!(screen.lines().all(|line| line.chars().count() <= 40));
}
