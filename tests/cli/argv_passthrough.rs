use std::{
    cell::Cell,
    ffi::OsString,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

use super::cli_entry;

const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally\n";

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

fn assert_zero_auth_refusal(args: Vec<OsString>, provider_dir: &Path, capture: &Path) {
    let _ = fs::remove_file(capture);
    let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
        .args(args)
        .env("PATH", provider_dir)
        .env("TASKSEAL_CAPTURE_PATH", capture)
        .output()
        .expect("tseal must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), ZERO_AUTH_REFUSAL);
    assert!(!capture.exists(), "external child must not be born");
}

#[test]
fn named_and_generic_auth_routes_share_one_pre_birth_zero_auth_refusal() {
    // Break caught: an auth spelling reaches provider birth or a raw-input/browser fallback.
    let (codex, capture) = fake_provider("codex");
    let provider_dir = codex.parent().unwrap();
    let renamed = provider_dir.join("renamed-provider");
    fs::copy(&codex, &renamed).unwrap();
    let symlinked = provider_dir.join("provider-link");
    symlink(&codex, &symlinked).unwrap();
    let device_provider = provider_dir.join("device-provider");
    fs::copy(&codex, &device_provider).unwrap();
    let browser_helper = provider_dir.join("browser-helper");
    fs::copy(&codex, &browser_helper).unwrap();

    let cases = [
        vec!["codex".into(), "login".into()],
        vec![
            "codex".into(),
            "login".into(),
            "--with-access-token".into(),
            "named-token-value-must-not-be-read".into(),
        ],
        vec![
            "--".into(),
            "codex".into(),
            "login".into(),
            "--api-key".into(),
            "generic-key-value-must-not-be-read".into(),
        ],
        vec![
            "--".into(),
            device_provider.into_os_string(),
            "device-flow".into(),
        ],
        vec![
            "--".into(),
            browser_helper.into_os_string(),
            "browser-oauth".into(),
        ],
        vec!["--".into(), renamed.into_os_string(), "login".into()],
        vec!["--".into(), symlinked.into_os_string(), "login".into()],
    ];
    for args in cases {
        assert_zero_auth_refusal(args, provider_dir, &capture);
    }
}

struct PoisonTail {
    prefix: std::vec::IntoIter<String>,
    next_calls: Rc<Cell<usize>>,
    unread_tail: Vec<String>,
}

impl Iterator for PoisonTail {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_calls.set(self.next_calls.get() + 1);
        if let Some(value) = self.prefix.next() {
            return Some(value);
        }
        panic!(
            "TaskSeal consumed a prohibited provider tail containing {} unread values",
            self.unread_tail.len()
        );
    }
}

#[test]
fn provider_and_generic_refusal_do_not_consume_credential_shaped_tails() {
    // Break caught: collecting or cloning argv reads API-key/token values before zero-auth refusal.
    for (prefix, unread_tail, expected_calls) in [
        (
            vec!["codex".to_owned()],
            vec![
                "--api-key".to_owned(),
                "named-key-value-must-not-be-read".to_owned(),
            ],
            1,
        ),
        (
            vec!["--".to_owned(), "codex".to_owned()],
            vec![
                "--access-token".to_owned(),
                "generic-token-value-must-not-be-read".to_owned(),
            ],
            2,
        ),
    ] {
        let next_calls = Rc::new(Cell::new(0));
        let exit = cli_entry::run(
            "tseal",
            PoisonTail {
                prefix: prefix.into_iter(),
                next_calls: Rc::clone(&next_calls),
                unread_tail,
            },
        );
        assert_eq!(exit, std::process::ExitCode::from(2));
        assert_eq!(next_calls.get(), expected_calls);
    }
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
    assert_eq!(String::from_utf8(output.stderr).unwrap(), ZERO_AUTH_REFUSAL);
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

#[test]
fn taskseal_owned_local_commands_remain_available() {
    // Break caught: closing external execution accidentally disables local-only operations.
    for command in ["status", "scan", "prepare", "check"] {
        let output = Command::new(env!("CARGO_BIN_EXE_tseal"))
            .arg(command)
            .output()
            .expect("tseal must run");
        assert_eq!(output.status.code(), Some(0), "{command}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "tseal: command accepted\n",
            "{command}"
        );
        assert!(output.stderr.is_empty(), "{command}");
    }
}
