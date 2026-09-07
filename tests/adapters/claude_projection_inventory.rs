use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn scratch() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "clroom-claude-inventory-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn claude_projection_uses_only_active_cached_plugin_versions() {
    let root = scratch();
    let home = root.join("home");
    let plugin = home.join(".claude/plugins/cache/example/superpowers/6.3.0");
    fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
    fs::create_dir_all(plugin.join("skills/brainstorming/references")).unwrap();
    fs::write(
        plugin.join(".claude-plugin/plugin.json"),
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
    fs::create_dir_all(home.join(".claude/plugins")).unwrap();
    fs::write(
        home.join(".claude/plugins/installed_plugins.json"),
        format!(
            r#"{{"plugins":{{"superpowers@plugin":[{{"installPath":"{}"}}]}}}}"#,
            plugin.display()
        ),
    )
    .unwrap();

    let projection = taskseal::adapters::claude::projection::project(
        &home,
        &["superpowers:brainstorming".to_owned()],
    )
    .unwrap_or_else(|error| {
        panic!(
            "{error:?} root={} registry={} plugin={}",
            root.display(),
            home.join(".claude/plugins/installed_plugins.json")
                .display(),
            plugin.display()
        )
    });

    assert_eq!(projection.selected_global_skills, 1);
    assert!(
        projection
            .add_dir
            .join(".claude/skills/brainstorming/references/guide.md")
            .is_file()
    );
    assert!(!projection.add_dir.join("hooks").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn claude_stale_cached_plugin_version_is_not_selectable() {
    let root = scratch();
    let home = root.join("home");
    let plugin = home.join(".claude/plugins/cache/example/stale/1.0.0");
    fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
    fs::create_dir_all(plugin.join("skills/ghost")).unwrap();
    fs::write(
        plugin.join(".claude-plugin/plugin.json"),
        br#"{"name":"stale"}"#,
    )
    .unwrap();
    fs::write(plugin.join("skills/ghost/SKILL.md"), b"ghost\n").unwrap();

    let error = taskseal::adapters::claude::projection::project(&home, &["stale:ghost".to_owned()])
        .unwrap_err();

    assert_eq!(
        error,
        taskseal::adapters::claude::projection::ProjectionError::UnknownSelector(
            "stale:ghost".to_owned()
        )
    );
    let _ = fs::remove_dir_all(root);
}
