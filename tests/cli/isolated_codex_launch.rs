use std::{
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const CODEX_CLEAN_DEFAULTS: &[&str] = &[
    "-c",
    "features.apps=false",
    "-c",
    "features.hooks=false",
    "-c",
    "features.plugins=false",
    "-c",
    "developer_instructions=\"\"",
    "-c",
    "notify=[]",
];

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "clroom-isolated-launch-{}-{}",
            std::process::id(),
            SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
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

fn isolated_fixture() -> (Scratch, PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = Scratch::new();
    let project = root.join("project");
    let home = root.join("home");
    let codex_home = root.join("codex-home");
    let bin = root.join("bin");
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
         printf '%s\\0' \"$@\" > \"$CLROOM_CAPTURE_PATH\"\n\
         for argument in \"$@\"; do\n\
           if [ \"$argument\" = \"--stdio\" ]; then\n\
             input=$(/bin/cat)\n\
             printf 'stdout:%s\\n' \"$input\"\n\
             printf 'stderr:%s\\n' \"$input\" >&2\n\
             exit 0\n\
           fi\n\
         done\n\
         exit 42\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    (root, project, home, codex_home, bin)
}

fn command(project: &Path, home: &Path, codex_home: &Path, bin: &Path, capture: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clroom"));
    command
        .current_dir(project)
        .env("PATH", bin)
        .env("HOME", home)
        .env("CODEX_HOME", codex_home)
        .env("CLROOM_PROJECT_CANARY", project.join("canaries/PROJECT.md"))
        .env("CLROOM_GLOBAL_AGENTS", codex_home.join("AGENTS.md"))
        .env(
            "CLROOM_AMBIENT_SKILL",
            home.join(".agents/skills/ambient/SKILL.md"),
        )
        .env("CLROOM_CAPTURE_PATH", capture);
    command
}

fn expected_argv(user_args: &[&str]) -> Vec<u8> {
    CODEX_CLEAN_DEFAULTS
        .iter()
        .chain(user_args)
        .flat_map(|argument| argument.as_bytes().iter().copied().chain([0]))
        .collect()
}

#[test]
fn codex_handoff_preserves_literal_argv_exit_and_stdio_inside_the_isolated_boundary() {
    // Break caught: direct Codex execution lets the fake provider read ambient context.
    let (_root, project, home, codex_home, bin) = isolated_fixture();
    let capture = bin.join("capture");
    let output = command(&project, &home, &codex_home, &bin, &capture)
        .args(["codex", "--exit-42", "literal value"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(42));
    assert_eq!(
        fs::read(&capture).unwrap(),
        expected_argv(&["--exit-42", "literal value"])
    );

    let mut child = command(&project, &home, &codex_home, &bin, &capture)
        .args(["codex", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"native streams\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status, std::process::ExitStatus::from_raw(0));
    assert_eq!(output.stdout, b"stdout:native streams\n");
    assert_eq!(output.stderr, b"stderr:native streams\n");
    assert_eq!(fs::read(&capture).unwrap(), expected_argv(&["--stdio"]));
}

#[test]
fn codex_handoff_keeps_explicit_user_overrides_after_clean_defaults() {
    let (_root, project, home, codex_home, bin) = isolated_fixture();
    let capture = bin.join("override-capture");
    let user_args = [
        "--enable",
        "apps",
        "--enable",
        "hooks",
        "--enable",
        "plugins",
        "-c",
        "developer_instructions=\"owner\"",
        "-c",
        "notify=[\"owner\"]",
        "features",
        "list",
    ];

    let output = command(&project, &home, &codex_home, &bin, &capture)
        .arg("codex")
        .args(user_args)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(42));
    assert_eq!(fs::read(&capture).unwrap(), expected_argv(&user_args));
}

#[test]
fn codex_handoff_refuses_when_home_is_not_supplied() {
    // Break caught: a missing isolation input silently falls back to ambient Codex.
    let (_root, project, home, codex_home, bin) = isolated_fixture();
    let capture = bin.join("missing-home-capture");
    let output = Command::new(env!("CARGO_BIN_EXE_clroom"))
        .current_dir(&project)
        .args(["codex", "--help"])
        .env("PATH", &bin)
        .env_remove("HOME")
        .env("CODEX_HOME", &codex_home)
        .env("CLROOM_PROJECT_CANARY", project.join("canaries/PROJECT.md"))
        .env("CLROOM_GLOBAL_AGENTS", codex_home.join("AGENTS.md"))
        .env(
            "CLROOM_AMBIENT_SKILL",
            home.join(".agents/skills/ambient/SKILL.md"),
        )
        .env("CLROOM_CAPTURE_PATH", &capture)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "CLROOM_ISOLATION_INVALID: HOME is unavailable; continue locally\n"
    );
    assert!(!capture.exists());
    assert!(!home.join("unused").exists());
}
