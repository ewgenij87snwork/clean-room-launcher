use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::symlink;

use clroom::adapters::codex::isolation::{IsolationInputs, plan, plan_with_skills};

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
fn plan_allows_project_canary_but_sandbox_denies_documented_ambient_canaries() {
    // Break caught: removing an ambient deny rule would let the fixture read its canary.
    let root = TempRoot::new("isolation");
    let project = root.path().join("project");
    let fake_home = root.path().join("home");
    let fake_codex_home = root.path().join("codex-home");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(fake_home.join(".agents/skills/ambient")).unwrap();
    for root in [".ssh", ".aws", ".config/gcloud", ".azure"] {
        fs::create_dir_all(fake_home.join(root)).unwrap();
        fs::write(fake_home.join(root).join("canary"), b"synthetic\n").unwrap();
    }
    fs::create_dir_all(fake_codex_home.join("skills/ambient")).unwrap();
    fs::create_dir_all(fake_codex_home.join("plugins/cache/ambient")).unwrap();
    let project_canary = project.join("PROJECT.md");
    let global_agents = fake_codex_home.join("AGENTS.md");
    let ambient_skill = fake_home.join(".agents/skills/ambient/SKILL.md");
    let ambient_plugin = fake_codex_home.join("plugins/cache/ambient/plugin.json");
    fs::write(&project_canary, b"admitted\n").unwrap();
    fs::write(&global_agents, b"ambient instruction\n").unwrap();
    fs::write(&ambient_skill, b"ambient skill\n").unwrap();
    fs::write(&ambient_plugin, b"{}\n").unwrap();
    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs {
            home: fake_home.clone(),
            codex_home: fake_codex_home,
        },
    )
    .unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("cat \"$1\" >/dev/null || exit 70; printf 'project-write\\n' >>\"$1\" || exit 71; for path in \"$2\" \"$3\" \"$4\" \"$5\" \"$6\" \"$7\" \"$8\"; do cat \"$path\" >/dev/null 2>&1 && exit 72; printf 'blocked\\n' >>\"$path\" 2>/dev/null && exit 73; done; exit 0")
        .arg("fixture")
        .arg(&project_canary)
        .arg(&global_agents)
        .arg(&ambient_skill)
        .arg(&ambient_plugin)
        .arg(fake_home.join(".ssh/canary"))
        .arg(fake_home.join(".aws/canary"))
        .arg(fake_home.join(".config/gcloud/canary"))
        .arg(fake_home.join(".azure/canary"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "sandbox fixture must read/write the project and reject ambient read/write canaries: status={:?} stdout={} stderr={}",
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
fn codex_provider_files_allow_metadata_but_deny_contents() {
    // Break caught: allowing no provider-file metadata makes --ignore-user-config
    // fail before launch; allowing file data would re-admit ambient config.
    let root = TempRoot::new("provider-file-metadata");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = home.join(".codex");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&codex_home).unwrap();
    let config = codex_home.join("config.toml");
    fs::write(&config, b"ambient-canary = true\n").unwrap();

    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
    )
    .unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/usr/bin/stat \"$1\" >/dev/null || exit 80; /bin/cat \"$1\" >/dev/null 2>&1 && exit 81; exit 0")
        .arg("fixture")
        .arg(&config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "provider config metadata must be visible while its contents stay denied: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
fn codex_plugin_cache_is_not_inventoried_without_provider_activation_state() {
    let root = TempRoot::new("active-plugin-skill");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let plugin = codex_home.join("plugins/cache/superpowers/6.3.0");
    fs::create_dir_all(project.as_path()).unwrap();
    fs::create_dir_all(plugin.join(".codex-plugin")).unwrap();
    fs::create_dir_all(plugin.join("skills/brainstorming/references")).unwrap();
    fs::write(
        plugin.join(".codex-plugin/plugin.json"),
        br#"{"name":"superpowers"}"#,
    )
    .unwrap();
    fs::write(
        plugin.join("skills/brainstorming/SKILL.md"),
        b"brainstorming\n",
    )
    .unwrap();
    fs::write(
        plugin.join("skills/brainstorming/references/guide.md"),
        b"guide\n",
    )
    .unwrap();
    fs::create_dir_all(plugin.join("hooks")).unwrap();
    fs::write(plugin.join("hooks/hooks.json"), b"{}\n").unwrap();
    let error = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["superpowers:brainstorming".to_owned()],
    )
    .unwrap_err();

    assert_eq!(
        error,
        clroom::adapters::codex::isolation::IsolationError::UnknownSkillSelector(
            "superpowers:brainstorming".to_owned()
        )
    );
}

#[test]
fn provider_owned_system_and_admin_skill_roots_remain_readable_but_not_writable() {
    let root = TempRoot::new("provider-owned-skill-roots");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let system_skill = codex_home.join("skills/.system/skill-creator/SKILL.md");
    let admin_skill = root.path().join("private/etc/codex/skills/admin/SKILL.md");
    fs::create_dir_all(system_skill.parent().unwrap()).unwrap();
    fs::create_dir_all(admin_skill.parent().unwrap()).unwrap();
    fs::write(&system_skill, b"system\n").unwrap();
    fs::write(&admin_skill, b"admin\n").unwrap();
    fs::create_dir_all(&project).unwrap();

    let isolation = plan(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
    )
    .unwrap();
    assert!(isolation.profile.contains("/private/etc/codex/skills"));
    assert!(isolation.profile.contains("skills/.system"));

    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &isolation.profile, "--", "/bin/sh", "-c"])
        .arg("/bin/cat \"$1\" >/dev/null || exit 80; printf 'blocked\\n' >>\"$1\" 2>/dev/null && exit 81; exit 0")
        .arg("fixture")
        .arg(&system_skill)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "provider SYSTEM skill must be readable but not writable: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stale_codex_plugin_cache_is_not_a_skill_source() {
    let root = TempRoot::new("stale-plugin-cache");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let plugin = codex_home.join("plugins/cache/stale/1.0.0");
    fs::create_dir_all(project.as_path()).unwrap();
    fs::create_dir_all(plugin.join(".codex-plugin")).unwrap();
    fs::create_dir_all(plugin.join("skills/ghost")).unwrap();
    fs::write(plugin.join("skills/ghost/SKILL.md"), b"ghost\n").unwrap();
    fs::write(
        plugin.join(".codex-plugin/plugin.json"),
        br#"{"name":"stale"}"#,
    )
    .unwrap();

    let error = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["stale:ghost".to_owned()],
    )
    .unwrap_err();

    assert_eq!(
        error,
        clroom::adapters::codex::isolation::IsolationError::UnknownSkillSelector(
            "stale:ghost".to_owned()
        )
    );
}

#[cfg(unix)]
#[test]
fn symlinked_codex_skill_root_is_not_an_inventory_source() {
    let root = TempRoot::new("symlinked-skill-root");
    let project = root.path().join("project");
    let home = root.path().join("home");
    let codex_home = root.path().join("codex-home");
    let outside = root.path().join("outside/escape");
    fs::create_dir_all(outside.as_path()).unwrap();
    fs::write(outside.join("SKILL.md"), b"outside\n").unwrap();
    fs::create_dir_all(codex_home.join("skills")).unwrap();
    symlink(&outside, codex_home.join("skills/escape")).unwrap();
    fs::create_dir_all(&project).unwrap();

    let error = plan_with_skills(
        &project,
        Path::new("/bin/sh"),
        &IsolationInputs { home, codex_home },
        &["escape".to_owned()],
    )
    .unwrap_err();
    assert_eq!(
        error,
        clroom::adapters::codex::isolation::IsolationError::UnknownSkillSelector(
            "escape".to_owned()
        )
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
