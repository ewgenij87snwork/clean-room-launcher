use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use taskseal::adapters::codex::isolation::{IsolationInputs, plan, plan_with_skills};

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

#[test]
fn selected_duplicate_logical_skill_prefers_codex_root_and_denies_agents_body() {
    // Break caught: admitting every native duplicate produces duplicate rows in
    // Codex's picker instead of one deterministic selected skill.
    let root = TempRoot::new("duplicate-native-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let agents_root = home.join(".agents/skills");
    let codex_root = codex_home.join("skills");
    let agents_skill = agents_root.join("rev/SKILL.md");
    let codex_skill = codex_root.join("rev/SKILL.md");
    fs::create_dir_all(agents_skill.parent().unwrap()).unwrap();
    fs::create_dir_all(codex_skill.parent().unwrap()).unwrap();
    fs::write(&agents_skill, b"agents rev\n").unwrap();
    fs::write(&codex_skill, b"codex rev\n").unwrap();
    fs::create_dir_all(&project).unwrap();

    let isolation = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["rev".to_owned()],
    )
    .unwrap();

    assert_eq!(isolation.selected_global_skills, 1);
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/cat \"$1\" >/dev/null || exit 80; /bin/cat \"$2\" >/dev/null 2>&1 && exit 81; exit 0")
        .arg("fixture")
        .arg(&codex_skill)
        .arg(&agents_skill)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Codex-local rev must be readable and the duplicate agents rev must remain denied: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn selected_skill_roots_are_listable_but_unselected_skill_bodies_stay_denied() {
    let root = TempRoot::new("skill-root-discovery");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let skills_root = home.join(".agents/skills");
    let selected = skills_root.join("arrow/SKILL.md");
    let unselected = skills_root.join("ambient/SKILL.md");
    fs::create_dir_all(selected.parent().unwrap()).unwrap();
    fs::create_dir_all(unselected.parent().unwrap()).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(&selected, b"selected\n").unwrap();
    fs::write(&unselected, b"unselected\n").unwrap();

    let isolation = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["arrow".to_owned()],
    )
    .unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/ls \"$1\" >/dev/null || exit 80; /usr/bin/stat \"$4\" >/dev/null || exit 81; /bin/realpath \"$4\" >/dev/null || exit 82; /bin/ls \"$2\" >/dev/null || exit 83; /bin/cat \"$3\" >/dev/null || exit 84; /bin/cat \"$5\" >/dev/null 2>&1 && exit 85; exit 0")
        .arg("fixture")
        .arg(&skills_root)
        .arg(selected.parent().unwrap())
        .arg(&selected)
        .arg(unselected.parent().unwrap())
        .arg(&unselected)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "native discovery needs root listing and child metadata while unselected bodies stay denied: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
