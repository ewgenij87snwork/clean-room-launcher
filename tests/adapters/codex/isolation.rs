use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use taskseal::adapters::codex::isolation::{IsolationInputs, plan};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("taskseal-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn plan_allows_project_canary_but_sandbox_denies_documented_ambient_canaries() {
    // Break caught: removing an ambient deny rule would let the fixture read its canary.
    let root = TempRoot::new("isolation");
    let project = root.path().join("project");
    let fake_home = root.path().join("home");
    let fake_codex_home = root.path().join("codex-home");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(fake_home.join(".agents/skills/ambient")).unwrap();
    fs::create_dir_all(fake_codex_home.join("skills/ambient")).unwrap();
    let project_canary = project.join("PROJECT.md");
    let global_agents = fake_codex_home.join("AGENTS.md");
    let ambient_skill = fake_home.join(".agents/skills/ambient/SKILL.md");
    fs::write(&project_canary, b"admitted\n").unwrap();
    fs::write(&global_agents, b"ambient instruction\n").unwrap();
    fs::write(&ambient_skill, b"ambient skill\n").unwrap();
    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs {
            home: fake_home,
            codex_home: fake_codex_home,
        },
    )
    .unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("cat \"$1\" >/dev/null; first=$?; cat \"$2\" >/dev/null 2>&1; second=$?; cat \"$3\" >/dev/null 2>&1; third=$?; printf '%s/%s/%s' \"$first\" \"$second\" \"$third\"; test \"$first/$second/$third\" = 0/1/1")
        .arg("fixture")
        .arg(&project_canary)
        .arg(&global_agents)
        .arg(&ambient_skill)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "sandbox fixture must read project canary and reject both ambient canaries: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn plan_renders_denies_for_nonexistent_synthetic_codex_home_without_inspecting_it() {
    // Break caught: metadata/canonicalization of a provider-home input would reject this path.
    let root = TempRoot::new("isolation-nonexistent-home");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let missing_codex_home = root.path().join("does-not-exist");
    fs::create_dir_all(&project).unwrap();

    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs {
            home,
            codex_home: missing_codex_home.clone(),
        },
    )
    .unwrap();

    assert!(
        isolation
            .profile
            .contains(missing_codex_home.to_str().unwrap())
    );
}
