use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginInstallation {
    Installed(PathBuf),
    NotInstalled,
    Ambiguous,
    InvalidState,
}

pub fn locate(codex_home: &Path, plugin_key: &str) -> PluginInstallation {
    let Some((plugin_name, marketplace_name)) = parse_plugin_key(plugin_key) else {
        return PluginInstallation::InvalidState;
    };
    let cache_root = codex_home.join("plugins/cache");
    let Some(cache_root) = canonical_nonsymlink_directory(&cache_root) else {
        return PluginInstallation::NotInstalled;
    };
    let base = cache_root.join(marketplace_name).join(plugin_name);
    let Ok(base_metadata) = fs::symlink_metadata(&base) else {
        return PluginInstallation::NotInstalled;
    };
    if base_metadata.file_type().is_symlink() || !base_metadata.is_dir() {
        return PluginInstallation::InvalidState;
    }

    let Ok(entries) = fs::read_dir(&base) else {
        return PluginInstallation::InvalidState;
    };
    let mut versions = Vec::new();
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let Some(version) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !valid_version_segment(&version) {
            continue;
        }
        let Some(root) = canonical_nonsymlink_directory(&entry.path()) else {
            continue;
        };
        if !root.starts_with(&cache_root) {
            continue;
        }
        if version == "local" {
            return PluginInstallation::Installed(root);
        }
        versions.push(root);
    }
    versions.sort();
    versions.dedup();
    match versions.len() {
        0 => PluginInstallation::NotInstalled,
        1 => PluginInstallation::Installed(versions.remove(0)),
        _ => PluginInstallation::Ambiguous,
    }
}

fn parse_plugin_key(value: &str) -> Option<(&str, &str)> {
    let (plugin, marketplace) = value.rsplit_once('@')?;
    valid_plugin_segment(plugin, true)
        .then_some(())
        .and_then(|_| valid_plugin_segment(marketplace, false).then_some((plugin, marketplace)))
}

fn valid_plugin_segment(value: &str, allow_dots: bool) -> bool {
    if value.is_empty() || allow_dots && matches!(value, "." | "..") {
        return false;
    }
    if allow_dots && (value.starts_with('.') || value.ends_with('.') || value.contains("..")) {
        return false;
    }
    value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || matches!(character, '-' | '_')
            || allow_dots && character == '.'
    })
}

fn valid_version_segment(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
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

    fn root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "clroom-codex-plugin-state-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("plugins/cache")).unwrap();
        root
    }

    #[test]
    fn single_cached_version_is_installed() {
        let home = root();
        let plugin = home.join("plugins/cache/openai-bundled/chrome/0.154.0");
        fs::create_dir_all(&plugin).unwrap();
        assert!(matches!(
            locate(&home, "chrome@openai-bundled"),
            PluginInstallation::Installed(path) if path == fs::canonicalize(&plugin).unwrap()
        ));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn local_version_wins_without_guessing_other_version_order() {
        let home = root();
        let local = home.join("plugins/cache/local-market/example/local");
        fs::create_dir_all(&local).unwrap();
        fs::create_dir_all(home.join("plugins/cache/local-market/example/9.0.0")).unwrap();
        assert!(matches!(
            locate(&home, "example@local-market"),
            PluginInstallation::Installed(path) if path == fs::canonicalize(&local).unwrap()
        ));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn multiple_nonlocal_versions_are_ambiguous_instead_of_guessed() {
        let home = root();
        fs::create_dir_all(home.join("plugins/cache/market/example/1.9.0")).unwrap();
        fs::create_dir_all(home.join("plugins/cache/market/example/1.10.0")).unwrap();
        assert_eq!(locate(&home, "example@market"), PluginInstallation::Ambiguous);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn invalid_plugin_key_cannot_escape_cache_layout() {
        let home = root();
        for id in ["missing-market", "../evil@market", "evil@../market", "@market"] {
            assert_eq!(locate(&home, id), PluginInstallation::InvalidState);
        }
        let _ = fs::remove_dir_all(home);
    }
}
