use std::{fs, path::PathBuf, process::Command};

fn fake_provider() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("taskseal-foreground-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let executable = dir.join("fake-provider");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli/fake-provider.rs");
    let output = Command::new("rustc")
        .args([source, PathBuf::from("-o"), executable.clone()])
        .output()
        .expect("rustc must start");
    assert!(output.status.success(), "fake provider must compile");
    executable
}

#[test]
fn foreground_provider_preserves_normal_exit_and_reports_signal_termination() {
    // Break caught: status conversion hides an abnormal provider termination as an ordinary exit.
    let provider = fake_provider();
    let normal = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(["--", provider.to_str().unwrap(), "--exit-42"])
        .output()
        .expect("tseal must run");
    assert_eq!(normal.status.code(), Some(42));
    assert!(normal.stderr.is_empty());

    let aborted = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(["--", provider.to_str().unwrap(), "--abort"])
        .output()
        .expect("tseal must run");
    assert_eq!(aborted.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(aborted.stderr).unwrap(),
        format!("PROVIDER_TERMINATED_BY_SIGNAL: {}\n", provider.display())
    );
}
