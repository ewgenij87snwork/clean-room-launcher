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
fn sandbox_probe(profile: &str, script: &str, paths: &[&Path]) -> std::process::Output {
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .args(["-p", profile, "--", "/bin/sh", "-c", script, "fixture"])
        .args(paths);
    command.output().unwrap()
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

#[cfg(target_os = "macos")]
#[test]
fn claude_projection_metadata_seam_preserves_read_and_write_isolation() {
    let root = scratch();
    let project = root.join("project");
    let home = root.join("home");
    let selected_root = home.join(".claude/skills/review");
    let hidden_root = home.join(".claude/skills/hidden");
    let selected_skill = selected_root.join("SKILL.md");
    let hidden_skill = hidden_root.join("SKILL.md");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&selected_root).unwrap();
    fs::create_dir_all(&hidden_root).unwrap();
    fs::write(&selected_skill, b"selected synthetic skill\n").unwrap();
    fs::write(&hidden_skill, b"unselected synthetic skill\n").unwrap();

    let projection =
        clroom::adapters::claude::projection::project(&home, &["review".to_owned()]).unwrap();
    let sibling = projection
        .storage_root()
        .join("active/session-sibling-fixture");
    fs::create_dir_all(&sibling).unwrap();
    let sibling_secret = sibling.join("sibling-secret");
    fs::write(&sibling_secret, b"synthetic sibling content\n").unwrap();
    let storage_canary = projection.storage_root().join("storage-canary");
    fs::write(&storage_canary, b"synthetic shared storage content\n").unwrap();
    let owner_marker = projection.owner_marker_path();
    let release_marker = projection.release_marker_path();
    fs::write(&owner_marker, b"synthetic owner marker\n").unwrap();
    fs::write(&release_marker, b"synthetic release marker\n").unwrap();

    let provider_settings = home.join(".claude/settings.json");
    let ambient_agent_skill = home.join(".agents/skills/ambient/SKILL.md");
    let codex_skill = home.join(".codex/skills/ambient/SKILL.md");
    let codex_plugin = home.join(".codex/plugins/cache/ambient/plugin.json");
    let ssh_key = home.join(".ssh/identity");
    let aws_credentials = home.join(".aws/credentials");
    let gcloud_credentials = home.join(".config/gcloud/application_default_credentials.json");
    let azure_profile = home.join(".azure/profile.json");
    for path in [
        &provider_settings,
        &ambient_agent_skill,
        &codex_skill,
        &codex_plugin,
        &ssh_key,
        &aws_credentials,
        &gcloud_credentials,
        &azure_profile,
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"synthetic isolation canary\n").unwrap();
    }

    let plan = plan_for(&project, &home, &projection);
    let projected_skill = projection.add_dir.join(".claude/skills/review/SKILL.md");
    let readable = sandbox_probe(
        &plan.profile,
        "/bin/cat \"$1\" >/dev/null && /bin/cat \"$2\" >/dev/null",
        &[&selected_skill, &projected_skill],
    );
    assert!(
        readable.status.success(),
        "selected canonical and projected skills must remain readable"
    );

    let denied_reads: [&Path; 13] = [
        &hidden_skill,
        &sibling_secret,
        &storage_canary,
        &owner_marker,
        &release_marker,
        &provider_settings,
        &ambient_agent_skill,
        &codex_skill,
        &codex_plugin,
        &ssh_key,
        &aws_credentials,
        &gcloud_credentials,
        &azure_profile,
    ];
    let unreadable = sandbox_probe(
        &plan.profile,
        "for path do if /bin/cat \"$path\" >/dev/null 2>&1; then exit 81; fi; done; exit 0",
        &denied_reads,
    );
    assert!(
        unreadable.status.success(),
        "unselected, sibling, provider-state, and credential canaries must remain unreadable"
    );

    let session_root = projection.add_dir.parent().unwrap();
    let active_root = projection.storage_root().join("active");
    let listing_denied = sandbox_probe(
        &plan.profile,
        "for path do if /bin/ls \"$path\" >/dev/null 2>&1; then exit 82; fi; done; exit 0",
        &[
            session_root,
            &sibling,
            &active_root,
            projection.storage_root(),
        ],
    );
    assert!(
        listing_denied.status.success(),
        "session, sibling, active, and shared storage listings must remain denied"
    );

    let writes_denied = sandbox_probe(
        &plan.profile,
        "for path do if /bin/sh -c 'printf x >> \"$1\"' fixture \"$path\" 2>/dev/null; then exit 83; fi; done; exit 0",
        &[&projected_skill, &selected_skill],
    );
    assert!(
        writes_denied.status.success(),
        "projection and selected source writes must remain denied"
    );

    drop(projection);
    let _ = fs::remove_dir_all(root);
}
