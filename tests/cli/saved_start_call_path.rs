use super::state::{AccessClass, SavedStart, StateStore};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("taskseal-call-path-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn save(home: &Path, access_class: AccessClass) {
    StateStore::at(home.join("Library/Application Support/TaskSeal"))
        .save(SavedStart {
            provider: "codex".to_owned(),
            argv: vec!["--model".to_owned(), "private-model".to_owned()],
            project_digest: "a".repeat(64),
            access_class,
            qualification_digest: "b".repeat(64),
        })
        .unwrap();
}

#[test]
fn starts_command_reads_private_store_without_printing_argv() {
    let home = scratch("starts");
    save(&home, AccessClass::Standard);
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .arg("starts")
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("1. codex · STANDARD"));
    assert!(!stdout.contains("private-model"));
}

#[test]
fn standard_start_requires_explicit_approval_then_stops_before_provider_birth() {
    let home = scratch("standard");
    save(&home, AccessClass::Standard);
    let cancelled = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(["start", "1"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(cancelled.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(cancelled.stderr).unwrap(),
        "CONSENT_CANCELLED\n"
    );

    let approved = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(["start", "1", "--approve"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(approved.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(approved.stderr).unwrap(),
        "P06_REQUIRED: provider launch is not qualified\n"
    );
}

#[test]
fn full_access_start_never_uses_the_standard_approval_path() {
    let home = scratch("full-access");
    save(&home, AccessClass::FullAccess);
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(["start", "1", "--approve"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "FULL ACCESS: FULL_ACCESS_CHOOSER_REQUIRED\n"
    );
}
