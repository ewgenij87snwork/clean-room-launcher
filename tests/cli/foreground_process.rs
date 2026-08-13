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
fn foreground_provider_is_refused_before_normal_or_signal_execution() {
    // Break caught: a former generic foreground route still reaches child birth.
    let provider = fake_provider();
    let capture = provider.parent().unwrap().join("capture");
    for provider_arg in ["--exit-42", "--abort"] {
        let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
            .args(["--", provider.to_str().unwrap(), provider_arg])
            .env("TASKSEAL_CAPTURE_PATH", &capture)
            .output()
            .expect("tseal must run");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally\n"
        );
        assert!(!capture.exists());
    }
}
