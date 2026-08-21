use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const CODEX_CLEAN_DEFAULTS: &str = "-c\0features.hooks=false\0-c\0features.plugins=false\0-c\0developer_instructions=\"\"\0-c\0notify=[]\0";

fn fake_codex() -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "clroom-codex-launch-{}-{}",
        std::process::id(),
        SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let executable = dir.join("codex");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli/fake-provider.rs");
    let output = Command::new("rustc")
        .args([source, PathBuf::from("-o"), executable.clone()])
        .output()
        .expect("rustc must start");
    assert!(output.status.success(), "fake provider must compile");
    let capture = dir.join("capture");
    (executable, capture)
}

#[test]
fn direct_codex_command_launches_literal_local_child_and_returns_status() {
    let (codex, capture) = fake_codex();
    let path = codex.parent().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["codex", "--exit-42", "safe-value"])
        .env("PATH", path)
        .env("CLROOM_CAPTURE_PATH", &capture)
        .env("CLROOM_INHERITED_MARKER", "inherited")
        .output()
        .expect("clroom must run");
    assert_eq!(output.status.code(), Some(42));
    assert_eq!(
        fs::read_to_string(capture).unwrap(),
        format!("{CODEX_CLEAN_DEFAULTS}--exit-42\0safe-value\0inherited")
    );
}

#[test]
fn codex_without_legacy_boundary_forwards_native_help() {
    let (codex, capture) = fake_codex();
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["codex", "--help"])
        .env("PATH", codex.parent().unwrap())
        .env("CLROOM_CAPTURE_PATH", &capture)
        .output()
        .expect("clroom must run");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(capture).unwrap(),
        format!("{CODEX_CLEAN_DEFAULTS}--help\0")
    );
}

#[test]
fn codex_sensitive_tail_refuses_before_child_construction() {
    let (codex, capture) = fake_codex();
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["codex", "--api-key", "must-not-be-read"])
        .env("PATH", codex.parent().unwrap())
        .env("CLROOM_CAPTURE_PATH", &capture)
        .output()
        .expect("clroom must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "ZERO_AUTH_ARGUMENT_REFUSAL: sensitive argument refused before dispatch; continue locally\n"
    );
    assert!(!capture.exists());
}

#[test]
fn codex_unavailable_is_local_status_not_login_flow() {
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(["codex", "--help"])
        .env("PATH", "/definitely/not-a-clroom-command-path")
        .output()
        .expect("clroom must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "LOCAL_CODEX_UNAVAILABLE: executable 'codex' not found; continue locally\n"
    );
}
