use super::state::{AccessClass, SavedStart, StateStore};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("clroom-call-path-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn final_zero_auth_ingestion_saved_start_refuses_before_argv_hashing() {
    let home = scratch("sensitive-prehash");
    let store = StateStore::at(home.join("Library/Application Support/Clean Room Launcher"));
    fs::create_dir_all(store.root()).unwrap();
    fs::set_permissions(store.root(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        store.state_path(),
        format!(
            "{{\"schema_version\":\"taskseal.saved-start.v1\",\"starts\":[{{\"provider\":\"codex\",\"argv\":[\"--with-access-token\",\"must-not-be-hashed\"],\"project_digest\":\"{}\",\"access_class\":\"standard\",\"qualification_digest\":\"{}\"}}]}}",
            "a".repeat(64),
            "b".repeat(64)
        ),
    )
    .unwrap();
    fs::set_permissions(store.state_path(), fs::Permissions::from_mode(0o600)).unwrap();
    let before = fs::read(store.state_path()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["start", "1", "--approve"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "SAVED_START_SENSITIVE_ARGUMENT_REFUSED\n"
    );
    assert_eq!(fs::read(store.state_path()).unwrap(), before);
}

fn save(home: &Path, access_class: AccessClass) {
    StateStore::at(home.join("Library/Application Support/Clean Room Launcher"))
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
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
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
    let cancelled = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["start", "1"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(cancelled.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(cancelled.stderr).unwrap(),
        "CONSENT_CANCELLED\n"
    );

    let approved = Command::new(env!("CARGO_BIN_EXE_clroom"))
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
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
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
