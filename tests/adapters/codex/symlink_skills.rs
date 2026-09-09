use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use clroom::adapters::codex::isolation::{IsolationError, IsolationInputs, plan, plan_with_skills};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("clroom-{label}-{}-{nonce}", std::process::id()));
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
fn unselected_symlinked_user_skill_target_is_denied() {
    let root = TempRoot::new("unselected-symlink-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let agents_root = home.join(".agents/skills");
    let outside_skill = root.path().join("outside/job-flow");
    let outside_skill_file = outside_skill.join("SKILL.md");

    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(&outside_skill).unwrap();
    fs::write(&outside_skill_file, b"job-flow\n").unwrap();
    symlink(&outside_skill, agents_root.join("job-flow")).unwrap();

    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
    )
    .unwrap();

    assert_eq!(isolation.selected_global_skills, 0);
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/cat \"$1\" >/dev/null 2>&1 && exit 80; exit 0")
        .arg("fixture")
        .arg(&outside_skill_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "unselected symlink target must stay unreadable: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn selected_symlinked_user_skill_is_admitted_and_counted_once() {
    let root = TempRoot::new("selected-symlink-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let agents_root = home.join(".agents/skills");
    let outside_skill = root.path().join("outside/job-flow");
    let outside_skill_file = outside_skill.join("SKILL.md");
    let entry_path = agents_root.join("job-flow");

    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(&outside_skill).unwrap();
    fs::write(&outside_skill_file, b"job-flow\n").unwrap();
    symlink(&outside_skill, &entry_path).unwrap();

    let isolation = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["job-flow".to_owned()],
    )
    .unwrap();

    assert_eq!(isolation.selected_global_skills, 1);
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/cat \"$1\" >/dev/null || exit 80; /bin/cat \"$2\" >/dev/null || exit 81; exit 0")
        .arg("fixture")
        .arg(entry_path.join("SKILL.md"))
        .arg(&outside_skill_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "selected symlink skill must be readable through entry and canonical target: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn selected_native_duplicate_does_not_admit_symlinked_loser_target() {
    let root = TempRoot::new("duplicate-symlink-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let codex_skill = codex_home.join("skills/rev");
    let codex_skill_file = codex_skill.join("SKILL.md");
    let agents_root = home.join(".agents/skills");
    let outside_skill = root.path().join("outside/rev");
    let outside_skill_file = outside_skill.join("SKILL.md");

    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&codex_skill).unwrap();
    fs::create_dir_all(&agents_root).unwrap();
    fs::create_dir_all(&outside_skill).unwrap();
    fs::write(&codex_skill_file, b"codex rev\n").unwrap();
    fs::write(&outside_skill_file, b"agents rev\n").unwrap();
    symlink(&outside_skill, agents_root.join("rev")).unwrap();

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
        .arg(&codex_skill_file)
        .arg(&outside_skill_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "selected native winner must stay readable while symlinked duplicate target remains denied: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn symlinked_codex_skill_target_is_denied_but_not_selectable() {
    let root = TempRoot::new("codex-symlink-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let codex_skills = codex_home.join("skills");
    let outside_skill = root.path().join("outside/escape");
    let outside_skill_file = outside_skill.join("SKILL.md");

    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&codex_skills).unwrap();
    fs::create_dir_all(&outside_skill).unwrap();
    fs::write(&outside_skill_file, b"escape\n").unwrap();
    symlink(&outside_skill, codex_skills.join("escape")).unwrap();

    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs {
            home: home.clone(),
            codex_home: codex_home.clone(),
        },
    )
    .unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/cat \"$1\" >/dev/null 2>&1 && exit 80; exit 0")
        .arg("fixture")
        .arg(&outside_skill_file)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Codex-home symlink target must stay unreadable: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let error = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["escape".to_owned()],
    )
    .unwrap_err();
    assert_eq!(error, IsolationError::UnknownSkillSelector("escape".to_owned()));
}
