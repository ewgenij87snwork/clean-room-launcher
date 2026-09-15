#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "clroom-browser-qualification-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(version: &str) -> (Scratch, PathBuf, PathBuf, PathBuf) {
    let root = Scratch::new();
    let home = root.join("home");
    let project = root.join("project");
    let bin = root.join("bin");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&bin).unwrap();

    let executable = bin.join("claude");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '{}\\n'; exit 0; fi\nprintf '%s\\0' \"$@\" > \"$PWD/provider-argv\"\nexit 42\n",
            version
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

    (root, home, project, bin)
}

fn command(home: &Path, project: &Path, bin: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clroom"));
    command
        .current_dir(project)
        .env("HOME", home)
        .env("PATH", bin);
    command
}

#[test]
fn normalized_browser_selector_refuses_unqualified_claude_before_provider_birth() {
    let (_root, home, project, bin) = fixture("2.1.223");
    let capture = project.join("provider-argv");

    let output = command(&home, &project, &bin)
        .args(["claude", "--with=browser"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("CLROOM_RESOURCE_UNQUALIFIED"));
    assert!(stderr.contains("Claude Code 2.1.263"));
    assert!(!capture.exists(), "unqualified Claude reached provider launch");
}

#[test]
fn normalized_browser_selector_reaches_exact_qualified_claude_tuple() {
    let (_root, home, project, bin) = fixture("2.1.263");
    let capture = project.join("provider-argv");

    let output = command(&home, &project, &bin)
        .args(["claude", "--with=browser"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(42));
    let argv = fs::read(&capture).unwrap();
    assert!(argv.windows(b"--chrome\0".len()).any(|window| window == b"--chrome\0"));
}

#[test]
fn raw_provider_chrome_flag_keeps_existing_passthrough_semantics() {
    let (_root, home, project, bin) = fixture("2.1.223");
    let capture = project.join("provider-argv");

    let output = command(&home, &project, &bin)
        .args(["claude", "--chrome"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(42));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CLROOM_RESOURCE_UNQUALIFIED"));
    let argv = fs::read(&capture).unwrap();
    assert!(argv.windows(b"--chrome\0".len()).any(|window| window == b"--chrome\0"));
}
