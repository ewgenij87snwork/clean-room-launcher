use serde_json::Value;
use std::{fs, path::{Path, PathBuf}};

const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginInstallation {
    Installed(PathBuf),
    NotInstalled,
    Ambiguous,
    InvalidState,
}

pub fn locate(home: &Path, plugin_key: &str) -> PluginInstallation {
    if !valid_plugin_key(plugin_key) {
        return PluginInstallation::InvalidState;
    }
    let Some(cache_root) = canonical_nonsymlink_directory(&home.join(".claude/plugins/cache")) else {
        return PluginInstallation::NotInstalled;
    };
    let registry = home.join(".claude/plugins/installed_plugins.json");
    let Ok(metadata) = fs::symlink_metadata(&registry) else {
        return PluginInstallation::NotInstalled;
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_REGISTRY_BYTES
    {
        return PluginInstallation::InvalidState;
    }
    let Ok(bytes) = fs::read(&registry) else {
        return PluginInstallation::InvalidState;
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return PluginInstallation::InvalidState;
    };
    let Some(installations) = value
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get(plugin_key))
        .and_then(Value::as_array)
    else {
        return PluginInstallation::NotInstalled;
    };

    let mut roots = Vec::new();
    let mut invalid = false;
    for installation in installations {
        let Some(path) = installation.get("installPath").and_then(Value::as_str) else {
            invalid = true;
            continue;
        };
        let path = PathBuf::from(path);
        let Some(root) = canonical_nonsymlink_directory(&path) else {
            invalid = true;
            continue;
        };
        if !root.starts_with(&cache_root) {
            invalid = true;
            continue;
        }
        roots.push(root);
    }
    roots.sort();
    roots.dedup();
    match roots.len() {
        0 if invalid => PluginInstallation::InvalidState,
        0 => PluginInstallation::NotInstalled,
        1 if !invalid => PluginInstallation::Installed(roots.remove(0)),
        1 => PluginInstallation::InvalidState,
        _ => PluginInstallation::Ambiguous,
    }
}

fn valid_plugin_key(value: &str) -> bool {
    let Some((plugin, marketplace)) = value.rsplit_once('@') else {
        return false;
    };
    valid_segment(plugin) && valid_segment(marketplace)
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        && !matches!(value, "." | "..")
        && !value.contains("..")
        && !value.starts_with('.')
        && !value.ends_with('.')
}

fn canonical_nonsymlink_directory(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use super::{PluginInstallation, locate};
    use std::{fs, sync::atomic::{AtomicU64, Ordering}};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "clroom-claude-plugin-state-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        let plugin = home.join(".claude/plugins/cache/example/superpowers/6.3.0");
        fs::create_dir_all(&plugin).unwrap();
        fs::create_dir_all(home.join(".claude/plugins")).unwrap();
        (home, plugin)
    }

    #[test]
    fn registry_named_cache_root_is_installed() {
        let (home, plugin) = fixture();
        fs::write(
            home.join(".claude/plugins/installed_plugins.json"),
            format!(
                r#"{{"plugins":{{"superpowers@example":[{{"installPath":"{}"}}]}}}}"#,
                plugin.display()
            ),
        )
        .unwrap();
        assert!(matches!(
            locate(&home, "superpowers@example"),
            PluginInstallation::Installed(path) if path == fs::canonicalize(&plugin).unwrap()
        ));
        let _ = fs::remove_dir_all(home.parent().unwrap());
    }

    #[test]
    fn stale_cache_without_registry_authority_is_not_installed() {
        let (home, _) = fixture();
        fs::write(home.join(".claude/plugins/installed_plugins.json"), r#"{"plugins":{}}"#)
            .unwrap();
        assert_eq!(
            locate(&home, "superpowers@example"),
            PluginInstallation::NotInstalled
        );
        let _ = fs::remove_dir_all(home.parent().unwrap());
    }

    #[test]
    fn registry_path_outside_cache_is_invalid_state() {
        let (home, _) = fixture();
        let outside = home.parent().unwrap().join("outside/plugin");
        fs::create_dir_all(&outside).unwrap();
        fs::write(
            home.join(".claude/plugins/installed_plugins.json"),
            format!(
                r#"{{"plugins":{{"superpowers@example":[{{"installPath":"{}"}}]}}}}"#,
                outside.display()
            ),
        )
        .unwrap();
        assert_eq!(
            locate(&home, "superpowers@example"),
            PluginInstallation::InvalidState
        );
        let _ = fs::remove_dir_all(home.parent().unwrap());
    }

    #[test]
    fn multiple_distinct_install_roots_are_ambiguous() {
        let (home, first) = fixture();
        let second = home.join(".claude/plugins/cache/example/superpowers/6.3.1");
        fs::create_dir_all(&second).unwrap();
        fs::write(
            home.join(".claude/plugins/installed_plugins.json"),
            format!(
                r#"{{"plugins":{{"superpowers@example":[{{"installPath":"{}"}},{{"installPath":"{}"}}]}}}}"#,
                first.display(),
                second.display()
            ),
        )
        .unwrap();
        assert_eq!(
            locate(&home, "superpowers@example"),
            PluginInstallation::Ambiguous
        );
        let _ = fs::remove_dir_all(home.parent().unwrap());
    }
}
