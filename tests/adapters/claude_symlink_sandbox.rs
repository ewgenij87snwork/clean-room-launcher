#[cfg(target_os = "macos")]
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "macos")]
fn scratch() -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "clroom-claude-symlink-sandbox-{}-{nonce}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[cfg(target_os = "macos")]
fn plan_for(
    project: &Path,
    home: &Path,
    projection: &clroom::adapters::claude::projection::Projection,
) -> clroom::adapters::claude::isolation::IsolationPlan {
    clroom::adapters::claude::isolation::plan(
        project,
        Path::new("/bin/sh"),
        home,
        projection.storage_root(),
        &projection.add_dir,
        projection.denied_source_paths(),
        projection.allowed_source_paths(),
    )
    .unwrap()
}

#[cfg(target_os = "macos")]
#[test]
fn claude_symlink_canonical_target_policy_is_enforced_by_sandbox() {
    let root = scratch();
    let project = root.join("project");
    let home = root.join("home");
    let skill_root = home.join(".claude/skills");
    let external = root.join("shared/review");
    let external_skill = external.join("SKILL.md");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&skill_root).unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::write(&external_skill, b"review\n").unwrap();
    symlink(&external, skill_root.join("review")).unwrap();

    let unselected = clroom::adapters::claude::projection::project(&home, &[]).unwrap();
    let unselected_plan = plan_for(&project, &home, &unselected);
    let output = Command::new("/usr/bin/sandbox-exec")
        .args([
            "-p",
            unselected_plan.profile.as_str(),
            "--",
            "/bin/sh",
            "-c",
        ])
        .arg("/bin/cat \"$1\" >/dev/null 2>&1 && exit 80; exit 0")
        .arg("fixture")
        .arg(&external_skill)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "unselected Claude symlink target must be unreadable: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(unselected);

    let selected =
        clroom::adapters::claude::projection::project(&home, &["review".to_owned()]).unwrap();
    assert_eq!(selected.selected_global_skills, 1);
    let selected_plan = plan_for(&project, &home, &selected);
    let projected_skill = selected.add_dir.join(".claude/skills/review/SKILL.md");
    let output = Command::new("/usr/bin/sandbox-exec")
        .args([
            "-p",
            selected_plan.profile.as_str(),
            "--",
            "/bin/sh",
            "-c",
        ])
        .arg("/bin/cat \"$1\" >/dev/null || exit 80; /bin/cat \"$2\" >/dev/null || exit 81; exit 0")
        .arg("fixture")
        .arg(&external_skill)
        .arg(&projected_skill)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "selected Claude symlink target must be readable only through the admitted path: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    drop(selected);
    let _ = fs::remove_dir_all(root);
}
