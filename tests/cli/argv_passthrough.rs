use std::{fs, path::PathBuf, process::Command};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("taskseal-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn fake_provider(name: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    let executable = dir.join(name);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli/fake-provider.rs");
    let output = Command::new("rustc")
        .args([source, PathBuf::from("-o"), executable.clone()])
        .output()
        .expect("rustc must start");
    assert!(output.status.success(), "fake provider must compile");
    let capture = dir.join("argv.txt");
    (executable, capture)
}

#[test]
fn generic_boundary_forwards_every_argument_without_shell_evaluation() {
    // Break caught: joined/shell-evaluated argv loses empty/Unicode values or executes an injection string.
    let (generic, generic_capture) = fake_provider("generic-provider");
    let generic_args = ["--model", "opus", "", "--dangerously-skip-permissions"];
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .arg("--")
        .arg(&generic)
        .args(generic_args)
        .env("TASKSEAL_CAPTURE_PATH", &generic_capture)
        .output()
        .expect("tseal must run");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(generic_capture).unwrap(),
        generic_args.join("\0")
    );
}

#[test]
fn unqualified_provider_route_refuses_before_ambient_path_can_spawn() {
    let (codex, capture) = fake_provider("codex");
    let before = fs::read(&codex).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .arg("codex")
        .arg("--version")
        .env("PATH", codex.parent().unwrap())
        .env("TASKSEAL_CAPTURE_PATH", &capture)
        .output()
        .expect("tseal must run");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "P06_REQUIRED: provider tuple is not qualified\n"
    );
    assert!(!capture.exists());
    assert_eq!(fs::read(codex).unwrap(), before);
}

#[test]
fn generic_boundary_without_an_executable_refuses_safely() {
    // Break caught: an empty generic boundary panics or invokes an ambient shell.
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .arg("--")
        .output()
        .expect("tseal must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "GENERIC_EXECUTABLE_REQUIRED: use tseal -- <executable> [args...]\n"
    );
}
