use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "clroom-minimum-launch-{}-{}",
            std::process::id(),
            SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn interactive_enter_launches_fake_codex_through_the_isolated_boundary() {
    // Break caught: bare Enter returns a placeholder local success instead of
    // entering the already-proven isolated Codex handoff.
    let root = Scratch::new();
    let project = root.0.join("project");
    let home = root.0.join("home");
    let codex_home = root.0.join("codex-home");
    let bin = root.0.join("bin");
    let capture = project.join("capture");
    fs::create_dir_all(project.join("canaries")).unwrap();
    fs::create_dir_all(home.join(".agents/skills/ambient")).unwrap();
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(project.join("canaries/PROJECT.md"), b"project\n").unwrap();
    fs::write(codex_home.join("AGENTS.md"), b"global\n").unwrap();
    fs::write(
        home.join(".agents/skills/ambient/SKILL.md"),
        b"ambient skill\n",
    )
    .unwrap();

    let fake = bin.join("codex");
    fs::write(
        &fake,
        "#!/bin/sh\n\
         /bin/cat \"$CLROOM_PROJECT_CANARY\" >/dev/null || exit 70\n\
         /bin/cat \"$CLROOM_GLOBAL_AGENTS\" >/dev/null 2>&1 && exit 71\n\
         /bin/cat \"$CLROOM_AMBIENT_SKILL\" >/dev/null 2>&1 && exit 72\n\
         printf isolated-enter > \"$CLROOM_CAPTURE_PATH\"\n\
         exit 42\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new("/usr/bin/expect")
        .args([
            "-c",
            concat!(
                "set timeout 5\n",
                "spawn -noecho $env(CLROOM_TEST_BIN)\n",
                "expect {\n",
                "  \"Clean Room Launcher\" {}\n",
                "  timeout { exit 124 }\n",
                "  eof { exit 125 }\n",
                "}\n",
                "send \"\\r\"\n",
                "expect eof\n",
                "set child_status [wait]\n",
                "exit [lindex $child_status 3]\n",
            ),
        ])
        .current_dir(&project)
        .env("CLROOM_TEST_BIN", env!("CARGO_BIN_EXE_clroom"))
        .env("PATH", &bin)
        .env("HOME", &home)
        .env("CODEX_HOME", &codex_home)
        .env("CLROOM_PROJECT_CANARY", project.join("canaries/PROJECT.md"))
        .env("CLROOM_GLOBAL_AGENTS", codex_home.join("AGENTS.md"))
        .env(
            "CLROOM_AMBIENT_SKILL",
            home.join(".agents/skills/ambient/SKILL.md"),
        )
        .env("CLROOM_CAPTURE_PATH", &capture)
        .env("COLUMNS", "80")
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(42),
        "Enter did not propagate the isolated fake-provider result:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(fs::read_to_string(&capture).unwrap(), "isolated-enter");
    let transcript = String::from_utf8(output.stdout).unwrap().replace('\r', "");
    assert!(transcript.contains(&format!(
        "Project  {}",
        project.canonicalize().unwrap().display()
    )));
    assert!(transcript.contains("Boundary active · global AGENTS.md and ambient skills excluded"));
    assert!(transcript.contains("Defaults hooks/plugins off · explicit user overrides win"));
    assert!(transcript.contains(
        "Notice   Codex may show `Operation not permitted` for blocked ambient files; expected"
    ));
    assert!(transcript.contains("Action   Launch Codex"));
    assert!(!transcript.contains("Launch succeeded"));
}

#[test]
fn unimplemented_local_lifecycle_commands_refuse_truthfully_without_a_provider() {
    // Break caught: release-facing lifecycle placeholders report a false
    // success even though no local state or skill decision is produced.
    let root = Scratch::new();
    let bin = root.0.join("bin");
    let capture = root.0.join("provider-capture");
    fs::create_dir_all(&bin).unwrap();
    let fake = bin.join("codex");
    fs::write(
        &fake,
        "#!/bin/sh\nprintf launched > \"$CLROOM_CAPTURE_PATH\"\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();

    for args in [
        &["init"][..],
        &["status"][..],
        &["scan"][..],
        &["prepare"][..],
        &["check"][..],
        &["inspect", "skill:rust"][..],
        &["explain", "skill:rust"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
            .args(args)
            .env("PATH", &bin)
            .env("CLROOM_CAPTURE_PATH", &capture)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "command: {}", args[0]);
        assert!(output.stdout.is_empty(), "command: {}", args[0]);
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!(
                "LOCAL_LIFECYCLE_UNAVAILABLE: {} is not implemented in this build; use clroom codex for the minimum isolated launch\n",
                args[0]
            )
        );
    }

    assert!(
        !capture.exists(),
        "a lifecycle refusal started the provider"
    );
}

#[test]
fn bare_non_interactive_preview_never_starts_the_fake_provider() {
    let root = Scratch::new();
    let bin = root.0.join("bin");
    let capture = root.0.join("provider-capture");
    fs::create_dir_all(&bin).unwrap();
    let fake = bin.join("codex");
    fs::write(
        &fake,
        "#!/bin/sh\nprintf launched > \"$CLROOM_CAPTURE_PATH\"\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .env("PATH", &bin)
        .env("CLROOM_CAPTURE_PATH", &capture)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Preview only · provider not launched")
    );
    assert!(!capture.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn interactive_enter_refuses_locally_when_codex_is_unavailable() {
    let root = Scratch::new();
    let project = root.0.join("project");
    let home = root.0.join("home");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&home).unwrap();

    let output = Command::new("/usr/bin/expect")
        .args([
            "-c",
            concat!(
                "set timeout 5\n",
                "spawn -noecho $env(CLROOM_TEST_BIN)\n",
                "expect \"Clean Room Launcher\"\n",
                "send \"\\r\"\n",
                "expect eof\n",
                "set child_status [wait]\n",
                "exit [lindex $child_status 3]\n",
            ),
        ])
        .current_dir(&project)
        .env("CLROOM_TEST_BIN", env!("CARGO_BIN_EXE_clroom"))
        .env("PATH", root.0.join("empty-bin"))
        .env("HOME", &home)
        .env("COLUMNS", "80")
        .env("TERM", "xterm-256color")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let transcript = String::from_utf8(output.stdout).unwrap().replace('\r', "");
    assert!(
        transcript
            .contains("LOCAL_CODEX_UNAVAILABLE: executable 'codex' not found; continue locally")
    );
    let lower = transcript.to_ascii_lowercase();
    assert!(!lower.contains("login"));
    assert!(!lower.contains("sign in"));
}
