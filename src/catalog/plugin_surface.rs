use super::resource::{NativeKind, ResourceKind};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const MAX_MANIFEST_BYTES: u64 = 128 * 1024;
const MAX_COMPONENT_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SKILL_ENTRIES: usize = 512;
const AGENT_PLUGIN_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPluginSemantics {
    Codex,
    Claude,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PluginComponent {
    /// Provider/manifest-owned component kind. This is open native data, not a
    /// closed CLROOM ontology.
    pub kind: ResourceKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginSurface {
    pub declared: Vec<PluginComponent>,
    pub effective: Vec<PluginComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSurfaceError {
    Unavailable,
    InvalidManifest,
    UnsupportedManifest,
}

pub fn inspect_plugin_surface(
    provider: ProviderPluginSemantics,
    root: &Path,
) -> Result<PluginSurface, PluginSurfaceError> {
    let root = canonical_nonsymlink_directory(root).ok_or(PluginSurfaceError::Unavailable)?;
    match provider {
        ProviderPluginSemantics::Codex => inspect_codex(&root),
        ProviderPluginSemantics::Claude => inspect_claude(&root),
    }
}

fn inspect_codex(root: &Path) -> Result<PluginSurface, PluginSurfaceError> {
    let (_manifest_path, manifest) = codex_manifest(root)?;
    let mut declared = BTreeSet::new();
    let mut effective = BTreeSet::new();

    let skill_paths = manifest
        .get("skills")
        .and_then(manifest_paths)
        .filter(|paths| !paths.is_empty())
        .unwrap_or_else(|| vec!["./skills".to_owned()]);
    for skill in skill_components(root, &skill_paths) {
        declared.insert(skill.clone());
        effective.insert(skill);
    }

    match manifest.get("hooks").cloned() {
        Some(Value::Object(object)) => {
            for event in hook_events_from_value(&Value::Object(object)) {
                let component = component(ResourceKind::HookSet, &event);
                declared.insert(component.clone());
                effective.insert(component);
            }
        }
        Some(value @ Value::String(_)) | Some(value @ Value::Array(_)) => {
            let paths = manifest_paths(&value).unwrap_or_default();
            if paths.is_empty() {
                add_hook_file(root, "./hooks/hooks.json", &mut declared, &mut effective);
            } else {
                for path in paths {
                    add_hook_file(root, &path, &mut declared, &mut effective);
                }
            }
        }
        Some(_) | None => {
            add_hook_file(root, "./hooks/hooks.json", &mut declared, &mut effective);
        }
    }

    add_mcp_components(root, manifest.get("mcpServers"), &mut declared, &mut effective);
    add_app_components(root, manifest.get("apps"), &mut declared, &mut effective);

    Ok(surface(declared, effective))
}

fn inspect_claude(root: &Path) -> Result<PluginSurface, PluginSurfaceError> {
    let manifest_path = root.join(".claude-plugin/plugin.json");
    let manifest = read_json(&manifest_path, MAX_MANIFEST_BYTES)?;
    let mut declared = BTreeSet::new();
    let mut effective = BTreeSet::new();

    for skill in skill_components(root, &["./skills".to_owned()]) {
        declared.insert(skill.clone());
        effective.insert(skill);
    }
    add_root_skill(root, &manifest, &mut declared, &mut effective);
    add_flat_markdown_components(
        root,
        "./commands",
        ResourceKind::Skill,
        &mut declared,
        &mut effective,
    );
    add_flat_markdown_components(
        root,
        "./agents",
        ResourceKind::Agent,
        &mut declared,
        &mut effective,
    );

    if let Some(hooks) = manifest.get("hooks") {
        for event in hook_events_from_value(hooks) {
            let component = component(ResourceKind::HookSet, &event);
            declared.insert(component.clone());
            effective.insert(component);
        }
    }
    add_hook_file(root, "./hooks/hooks.json", &mut declared, &mut effective);
    add_mcp_components(root, None, &mut declared, &mut effective);
    add_named_json_components(
        root,
        "./.lsp.json",
        ResourceKind::LspServer,
        &mut declared,
        &mut effective,
    );
    add_monitor_components(root, &mut declared, &mut effective);
    add_plugin_executables(root, &mut declared, &mut effective);

    // The v0.4 activation qualifier must fail closed on every provider component
    // class whose runtime behavior is not proven by the read-only skill seam.
    // Custom manifest paths can replace default locations, so field presence is
    // itself meaningful even when this bounded inventory does not enumerate the
    // custom file's contents.
    for (field, kind) in [
        ("agents", ResourceKind::Agent),
        ("hooks", ResourceKind::HookSet),
        ("mcpServers", ResourceKind::McpServer),
        ("lspServers", ResourceKind::LspServer),
        ("workflows", NativeKind::new("workflow").expect("static kind")),
        ("outputStyles", NativeKind::new("output_style").expect("static kind")),
        ("userConfig", ResourceKind::SettingsOverlay),
        ("channels", NativeKind::new("channel").expect("static kind")),
        ("dependencies", NativeKind::new("dependency").expect("static kind")),
    ] {
        if manifest.get(field).is_some() {
            let item = component(kind, "manifest");
            declared.insert(item.clone());
            effective.insert(item);
        }
    }
    if manifest
        .get("experimental")
        .and_then(Value::as_object)
        .is_some_and(|experimental| experimental.contains_key("monitors"))
    {
        let item = component(ResourceKind::Monitor, "manifest");
        declared.insert(item.clone());
        effective.insert(item);
    }
    if manifest
        .get("experimental")
        .and_then(Value::as_object)
        .is_some_and(|experimental| experimental.contains_key("themes"))
    {
        let item = component(NativeKind::new("theme").expect("static kind"), "manifest");
        declared.insert(item.clone());
        effective.insert(item);
    }

    for (relative, kind) in [
        ("./workflows", NativeKind::new("workflow").expect("static kind")),
        (
            "./output-styles",
            NativeKind::new("output_style").expect("static kind"),
        ),
        ("./themes", NativeKind::new("theme").expect("static kind")),
    ] {
        if resolve_relative(root, relative).is_some() {
            let item = component(kind, "default");
            declared.insert(item.clone());
            effective.insert(item);
        }
    }

    if root.join("settings.json").is_file() || manifest.get("settings").is_some() {
        let component = component(ResourceKind::SettingsOverlay, "default");
        declared.insert(component.clone());
        effective.insert(component);
    }

    Ok(surface(declared, effective))
}

fn codex_manifest(root: &Path) -> Result<(PathBuf, Value), PluginSurfaceError> {
    let root_manifest = root.join("plugin.json");
    if let Ok(value) = read_json(&root_manifest, MAX_MANIFEST_BYTES) {
        if value.get("$schema").and_then(Value::as_str) == Some(AGENT_PLUGIN_SCHEMA) {
            return Err(PluginSurfaceError::UnsupportedManifest);
        }
        if value
            .get("$schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| schema.starts_with("https://agent-plugins.org/schemas/"))
        {
            return Err(PluginSurfaceError::UnsupportedManifest);
        }
    }

    for relative in [
        ".codex-plugin/plugin.json",
        ".claude-plugin/plugin.json",
        ".cursor-plugin/plugin.json",
    ] {
        let path = root.join(relative);
        match read_json(&path, MAX_MANIFEST_BYTES) {
            Ok(value) => return Ok((path, value)),
            Err(PluginSurfaceError::Unavailable) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(PluginSurfaceError::InvalidManifest)
}

fn skill_components(root: &Path, paths: &[String]) -> Vec<PluginComponent> {
    let mut names = BTreeSet::new();
    let mut visited = 0usize;
    for relative in paths {
        let Some(skill_root) = resolve_relative(root, relative) else {
            continue;
        };
        collect_skills(root, &skill_root, &mut names, &mut visited, 0);
        if visited >= MAX_SKILL_ENTRIES {
            break;
        }
    }
    names
        .into_iter()
        .map(|name| component(ResourceKind::Skill, &name))
        .collect()
}

fn collect_skills(
    plugin_root: &Path,
    directory: &Path,
    names: &mut BTreeSet<String>,
    visited: &mut usize,
    depth: usize,
) {
    if *visited >= MAX_SKILL_ENTRIES || depth > 6 {
        return;
    }
    let Some(directory) = canonical_nonsymlink_directory(directory) else {
        return;
    };
    if !directory.starts_with(plugin_root) {
        return;
    }
    if safe_regular_file(&directory.join("SKILL.md"), MAX_COMPONENT_FILE_BYTES).is_some()
        && let Some(name) = directory.file_name().and_then(|name| name.to_str())
        && valid_public_id(name)
    {
        names.insert(name.to_owned());
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        *visited += 1;
        if *visited >= MAX_SKILL_ENTRIES {
            break;
        }
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_skills(plugin_root, &entry.path(), names, visited, depth + 1);
        }
    }
}

fn add_hook_file(
    root: &Path,
    relative: &str,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let Some(path) = resolve_relative(root, relative) else {
        return;
    };
    let Ok(value) = read_json(&path, MAX_COMPONENT_FILE_BYTES) else {
        return;
    };
    for event in hook_events_from_value(&value) {
        let component = component(ResourceKind::HookSet, &event);
        declared.insert(component.clone());
        effective.insert(component);
    }
}

fn hook_events_from_value(value: &Value) -> Vec<String> {
    value
        .get("hooks")
        .and_then(Value::as_object)
        .or_else(|| value.as_object())
        .into_iter()
        .flat_map(|hooks| hooks.keys())
        .filter(|event| valid_public_id(event))
        .cloned()
        .collect()
}

fn add_root_skill(
    root: &Path,
    manifest: &Value,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    if safe_regular_file(&root.join("SKILL.md"), MAX_COMPONENT_FILE_BYTES).is_none() {
        return;
    }
    let id = manifest
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| valid_public_id(name))
        .unwrap_or("root");
    let skill = component(ResourceKind::Skill, id);
    declared.insert(skill.clone());
    effective.insert(skill);
}

fn add_flat_markdown_components(
    root: &Path,
    relative: &str,
    kind: ResourceKind,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let Some(directory) = resolve_relative(root, relative) else {
        return;
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|file_type| file_type.is_file())
            || path.extension().and_then(|value| value.to_str()) != Some("md")
        {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if !valid_public_id(id) {
            continue;
        }
        let item = component(kind, id);
        declared.insert(item.clone());
        effective.insert(item);
    }
}

fn add_named_json_components(
    root: &Path,
    relative: &str,
    kind: ResourceKind,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let Some(path) = resolve_relative(root, relative) else {
        return;
    };
    let Ok(value) = read_json(&path, MAX_COMPONENT_FILE_BYTES) else {
        return;
    };
    let Some(entries) = value.as_object() else {
        return;
    };
    for id in entries.keys().filter(|id| valid_public_id(id)) {
        let item = component(kind, id);
        declared.insert(item.clone());
        effective.insert(item);
    }
}

fn add_monitor_components(
    root: &Path,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let Some(path) = resolve_relative(root, "./monitors/monitors.json") else {
        return;
    };
    let Ok(value) = read_json(&path, MAX_COMPONENT_FILE_BYTES) else {
        return;
    };
    let Some(entries) = value.as_array() else {
        return;
    };
    for entry in entries {
        let Some(id) = entry
            .get("name")
            .and_then(Value::as_str)
            .filter(|id| valid_public_id(id))
        else {
            continue;
        };
        let item = component(ResourceKind::Monitor, id);
        declared.insert(item.clone());
        effective.insert(item);
    }
}

fn add_plugin_executables(
    root: &Path,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let Some(directory) = resolve_relative(root, "./bin") else {
        return;
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|file_type| file_type.is_file()) {
            continue;
        }
        let name = entry.file_name();
        let Some(id) = name.to_str().map(str::to_owned) else {
            continue;
        };
        if !valid_public_id(&id) {
            continue;
        }
        let item = component(ResourceKind::PluginExecutable, &id);
        declared.insert(item.clone());
        effective.insert(item);
    }
}

fn add_mcp_components(
    root: &Path,
    declared_value: Option<&Value>,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let value = match declared_value {
        Some(Value::Object(object)) => Some(Value::Object(object.clone())),
        Some(Value::String(path)) => resolve_relative(root, path)
            .and_then(|path| read_json(&path, MAX_COMPONENT_FILE_BYTES).ok()),
        Some(_) => None,
        None => read_json(&root.join(".mcp.json"), MAX_COMPONENT_FILE_BYTES).ok(),
    };
    let Some(value) = value else {
        return;
    };
    let servers = value
        .get("mcpServers")
        .and_then(Value::as_object)
        .or_else(|| value.as_object());
    let Some(servers) = servers else {
        return;
    };
    for name in servers.keys().filter(|name| valid_public_id(name)) {
        let component = component(ResourceKind::McpServer, name);
        declared.insert(component.clone());
        effective.insert(component);
    }
}

fn add_app_components(
    root: &Path,
    declared_value: Option<&Value>,
    declared: &mut BTreeSet<PluginComponent>,
    effective: &mut BTreeSet<PluginComponent>,
) {
    let value = match declared_value.and_then(Value::as_str) {
        Some(path) => resolve_relative(root, path)
            .and_then(|path| read_json(&path, MAX_COMPONENT_FILE_BYTES).ok()),
        None => read_json(&root.join(".app.json"), MAX_COMPONENT_FILE_BYTES).ok(),
    };
    let Some(value) = value else {
        return;
    };
    let candidates: Vec<&Value> = match &value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(object) => object
            .get("apps")
            .and_then(Value::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_else(|| vec![&value]),
        _ => Vec::new(),
    };
    for candidate in candidates {
        let id = candidate
            .get("id")
            .or_else(|| candidate.get("connectorId"))
            .and_then(Value::as_str);
        if let Some(id) = id.filter(|id| valid_public_id(id)) {
            let component = component(ResourceKind::AppConnector, id);
            declared.insert(component.clone());
            effective.insert(component);
        }
    }
}

fn manifest_paths(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::String(path) if valid_relative_manifest_path(path) => Some(vec![path.clone()]),
        Value::Array(paths) => Some(
            paths
                .iter()
                .filter_map(Value::as_str)
                .filter(|path| valid_relative_manifest_path(path))
                .map(str::to_owned)
                .collect(),
        ),
        _ => None,
    }
}

fn resolve_relative(root: &Path, relative: &str) -> Option<PathBuf> {
    if !valid_relative_manifest_path(relative) {
        return None;
    }
    let candidate = root.join(relative.strip_prefix("./")?);
    let canonical = fs::canonicalize(candidate).ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn valid_relative_manifest_path(value: &str) -> bool {
    value.starts_with("./")
        && value.len() > 2
        && !value.split(['/', '\\']).any(|component| component == "..")
}

fn read_json(path: &Path, max_bytes: u64) -> Result<Value, PluginSurfaceError> {
    let Some(bytes) = safe_regular_file(path, max_bytes) else {
        return Err(PluginSurfaceError::Unavailable);
    };
    serde_json::from_slice(&bytes).map_err(|_| PluginSurfaceError::InvalidManifest)
}

fn safe_regular_file(path: &Path, max_bytes: u64) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        return None;
    }
    fs::read(path).ok()
}

fn canonical_nonsymlink_directory(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(path).ok()
}

fn component(kind: ResourceKind, id: &str) -> PluginComponent {
    PluginComponent {
        kind,
        id: id.to_owned(),
    }
}

fn surface(
    declared: BTreeSet<PluginComponent>,
    effective: BTreeSet<PluginComponent>,
) -> PluginSurface {
    PluginSurface {
        declared: declared.into_iter().collect(),
        effective: effective.into_iter().collect(),
    }
}

fn valid_public_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|character| character.is_ascii_graphic() && !matches!(character, ',' | ':'))
}

#[cfg(test)]
mod tests {
    use super::{inspect_plugin_surface, PluginComponent, ProviderPluginSemantics};
    use crate::catalog::resource::ResourceKind;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "clroom-superpowers-6-3-0-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::create_dir_all(root.join(".claude-plugin")).unwrap();
        fs::create_dir_all(root.join("skills/brainstorming")).unwrap();
        fs::create_dir_all(root.join("hooks")).unwrap();
        fs::create_dir_all(root.join("agents")).unwrap();
        fs::write(root.join("skills/brainstorming/SKILL.md"), "fixture\n").unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            r#"{"name":"superpowers","version":"6.3.0","skills":"./skills/","hooks":{}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude-plugin/plugin.json"),
            r#"{"name":"superpowers","version":"6.3.0","hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"fixture-inline"}]}]}}"#,
        )
        .unwrap();
        fs::write(
            root.join("hooks/hooks.json"),
            r#"{"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"fixture"}]}]}}"#,
        )
        .unwrap();
        fs::write(root.join("agents/reviewer.md"), "fixture\n").unwrap();
        fs::write(root.join(".lsp.json"), r#"{"rust":{"command":"rust-analyzer"}}"#).unwrap();
        root
    }

    fn has(surface: &[PluginComponent], kind: ResourceKind, id: &str) -> bool {
        surface
            .iter()
            .any(|component| component.kind == kind && component.id == id)
    }

    #[test]
    fn superpowers_6_3_provider_semantics_do_not_flatten_package_contents() {
        let root = fixture();
        let codex = inspect_plugin_surface(ProviderPluginSemantics::Codex, &root).unwrap();
        let claude = inspect_plugin_surface(ProviderPluginSemantics::Claude, &root).unwrap();

        assert!(has(&codex.effective, ResourceKind::Skill, "brainstorming"));
        assert!(!has(
            &codex.effective,
            ResourceKind::HookSet,
            "SessionStart"
        ));
        assert!(has(
            &claude.effective,
            ResourceKind::Skill,
            "brainstorming"
        ));
        assert!(has(
            &claude.effective,
            ResourceKind::HookSet,
            "SessionStart"
        ));
        assert!(has(
            &claude.effective,
            ResourceKind::HookSet,
            "PostToolUse"
        ));
        assert!(has(&claude.effective, ResourceKind::Agent, "reviewer"));
        assert!(has(&claude.effective, ResourceKind::LspServer, "rust"));
        assert!(claude
            .effective
            .iter()
            .any(|item| item.kind == ResourceKind::HookSet && item.id == "manifest"));
        assert!(!codex
            .effective
            .iter()
            .any(|item| matches!(item.kind.as_str(), "mcp" | "app")));
        assert!(!claude
            .effective
            .iter()
            .any(|item| matches!(item.kind.as_str(), "mcp" | "app")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn codex_absent_hooks_manifest_field_uses_default_hook_file() {
        let root = fixture();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            r#"{"name":"superpowers","version":"6.3.0","skills":"./skills/"}"#,
        )
        .unwrap();
        let codex = inspect_plugin_surface(ProviderPluginSemantics::Codex, &root).unwrap();
        assert!(has(
            &codex.effective,
            ResourceKind::HookSet,
            "PostToolUse"
        ));
        let _ = fs::remove_dir_all(root);
    }
}
