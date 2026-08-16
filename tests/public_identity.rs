use std::{
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("clroom-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn clroom() -> &'static Path {
    Path::new(option_env!("CARGO_BIN_EXE_clroom").expect("clroom executable target is required"))
}

fn fake_codex() -> (PathBuf, PathBuf) {
    let root = scratch("public-identity-codex");
    let executable = root.join("codex");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/cli/fake-provider.rs");
    let output = Command::new("rustc")
        .args([source.as_os_str(), "-o".as_ref(), executable.as_os_str()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (executable, root.join("capture"))
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(clroom()).args(args).output().unwrap()
}

#[test]
fn clroom_is_the_only_public_identity_and_preserves_the_native_codex_process() {
    // Break caught: the shipped launcher is born under a stale name, rewrites native
    // Codex argv/stdio/status/signals, or exposes a login/credential acquisition path.
    let bare = run(&[]);
    assert!(bare.status.success());
    let bare_stdout = String::from_utf8(bare.stdout).unwrap();
    assert!(bare_stdout.contains("Clean Room Launcher"));
    assert!(bare_stdout.contains("A clean-room launcher for coding-agent CLIs."));
    assert!(bare_stdout.contains("clroom codex"));
    for stale in ["TaskSeal", "taskseal", "tseal", "croom"] {
        assert!(
            !bare_stdout.contains(stale),
            "stale public identity {stale:?}: {bare_stdout}"
        );
    }

    let help = run(&["--help"]);
    assert!(help.status.success());
    let help_stdout = String::from_utf8(help.stdout).unwrap();
    assert!(
        help_stdout
            .starts_with("Clean Room Launcher\nA clean-room launcher for coding-agent CLIs.\n")
    );
    assert!(help_stdout.contains("Usage: clroom"));

    let (codex, capture) = fake_codex();
    let provider_path = codex.parent().unwrap();
    let native = |args: &[&str]| {
        Command::new(clroom())
            .arg("codex")
            .args(args)
            .env("PATH", provider_path)
            .env("CLROOM_CAPTURE_PATH", &capture)
            .env("CLROOM_INHERITED_MARKER", "inherited")
            .output()
            .unwrap()
    };

    let no_args = native(&[]);
    assert!(no_args.status.success());
    assert_eq!(fs::read_to_string(&capture).unwrap(), "\0inherited");

    let forwarded = native(&["--help", "--approve-for-me", "--yolo"]);
    assert!(forwarded.status.success());
    assert_eq!(
        fs::read_to_string(&capture).unwrap(),
        "--help\0--approve-for-me\0--yolo\0inherited"
    );

    let mut stdio = Command::new(clroom())
        .args(["codex", "--stdio"])
        .env("PATH", provider_path)
        .env("CLROOM_CAPTURE_PATH", &capture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    stdio
        .stdin
        .take()
        .unwrap()
        .write_all(b"foreground-input")
        .unwrap();
    let stdio = stdio.wait_with_output().unwrap();
    assert!(stdio.status.success());
    assert_eq!(stdio.stdout, b"stdout:foreground-input\n");
    assert_eq!(stdio.stderr, b"stderr:foreground-input\n");

    let exited = native(&["--exit-42"]);
    assert_eq!(exited.status.code(), Some(42));
    let signalled = native(&["--abort"]);
    assert!(
        signalled.status.signal().is_some(),
        "launcher hid the provider signal: {signalled:?}"
    );

    for refused in [&["login"][..], &["--api-key", "must-not-be-retained"]] {
        let _ = fs::remove_file(&capture);
        let output = native(refused);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!capture.exists(), "refused provider child was born");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("must-not-be-retained"));
        assert!(stderr.contains("ZERO_AUTH"));
    }

    let home = scratch("public-state");
    let starts = Command::new(clroom())
        .arg("starts")
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(starts.status.success());
    assert!(
        home.join("Library/Application Support/Clean Room Launcher")
            .is_dir()
    );
    assert!(!home.join("Library/Application Support/TaskSeal").exists());

    fs::set_permissions(provider_path, fs::Permissions::from_mode(0o700)).unwrap();
    let _ = fs::remove_dir_all(provider_path);
    let _ = fs::remove_dir_all(home);
}
