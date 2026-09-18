use crate::adapters::{
    claude::plugin_state as claude_plugins, codex::plugin_state as codex_plugins,
};
use crate::catalog::plugin_surface::{
    inspect_plugin_surface, PluginComponent, PluginSurfaceError, ProviderPluginSemantics,
};
use crate::catalog::resource::{
    ActivationPolicy, DiscoveryState, EnablementState, InstallationState, QualificationState,
    ResourceId, ResourceInfo, ResourceKind, ResourceOrigin, SelectionState,
};
use std::path::{Path, PathBuf};

pub const CODEX_EXACT: (u64, u64, u64) = (0, 154, 0);
pub const CLAUDE_EXACT: (u64, u64, u64) = (2, 1, 274);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    Codex,
    Claude,
}

impl Provider {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
        }
    }

    pub fn exact_version(self) -> (u64, u64, u64) {
        match self {
            Self::Codex => CODEX_EXACT,
            Self::Claude => CLAUDE_EXACT,
        }
    }

    fn plugin_semantics(self) -> ProviderPluginSemantics {
        match self {
            Self::Codex => ProviderPluginSemantics::Codex,
            Self::Claude => ProviderPluginSemantics::Claude,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInventory {
    pub entry: ResourceInfo,
    pub root: Option<PathBuf>,
    pub declared_components: Vec<PluginComponent>,
    pub effective_components: Vec<PluginComponent>,
    pub conflicts: Vec<String>,
}

pub fn exact_tuple(
    provider: Provider,
    version: (u64, u64, u64),
    os: &str,
    arch: &str,
) -> bool {
    os == "macos" && arch == "aarch64" && version == provider.exact_version()
}

pub fn valid_plugin_key(provider: Provider, value: &str) -> bool {
    let Some((plugin, marketplace)) = value.rsplit_once('@') else {
        return false;
    };
    valid_plugin_segment(plugin, true)
        && valid_plugin_segment(marketplace, matches!(provider, Provider::Claude))
}

pub fn inspect_plugin(
    provider: Provider,
    home: &Path,
    codex_home: Option<&Path>,
    plugin_id: &str,
    tuple_qualified: bool,
) -> PluginInventory {
    let installation = locate_plugin(provider, home, codex_home, plugin_id);
    let (installation_state, discovery, root, mut conflicts, mut reason_code) = match installation {
        LocatedPlugin::Installed(root) => (
            InstallationState::Installed,
            DiscoveryState::Discoverable,
            Some(root),
            Vec::new(),
            "PLUGIN_ACTIVATION_V04",
        ),
        LocatedPlugin::NotInstalled => (
            InstallationState::NotInstalled,
            DiscoveryState::Unknown,
            None,
            vec!["PLUGIN_NOT_INSTALLED".to_owned()],
            "PLUGIN_NOT_INSTALLED",
        ),
        LocatedPlugin::Ambiguous => (
            InstallationState::Installed,
            DiscoveryState::Discoverable,
            None,
            vec!["PLUGIN_INSTALLATION_AMBIGUOUS".to_owned()],
            "PLUGIN_INSTALLATION_AMBIGUOUS",
        ),
        LocatedPlugin::InvalidState => (
            InstallationState::Unknown,
            DiscoveryState::Unknown,
            None,
            vec!["PLUGIN_INSTALLATION_STATE_INVALID".to_owned()],
            "PLUGIN_INSTALLATION_STATE_INVALID",
        ),
    };

    let (declared_components, effective_components) = root
        .as_deref()
        .map(|root| inspect_plugin_surface(provider.plugin_semantics(), root))
        .map(|result| match result {
            Ok(surface) => (surface.declared, surface.effective),
            Err(error) => {
                let blocker = plugin_surface_error_code(error).to_owned();
                reason_code = plugin_surface_error_code(error);
                conflicts.push(blocker);
                (Vec::new(), Vec::new())
            }
        })
        .unwrap_or_default();

    let activation_qualified = provider == Provider::Claude
        && tuple_qualified
        && installation_state == InstallationState::Installed
        && root.is_some()
        && conflicts.is_empty();

    if installation_state == InstallationState::Installed && conflicts.is_empty() && !activation_qualified
    {
        reason_code = if !tuple_qualified {
            "PROVIDER_TUPLE_NOT_QUALIFIED"
        } else {
            "PLUGIN_ACTIVATION_UNAVAILABLE"
        };
        conflicts.push(reason_code.to_owned());
    }

    PluginInventory {
        entry: ResourceInfo {
            resource: ResourceId::new(provider.id(), ResourceKind::Plugin, plugin_id)
                .expect("validated selector target must remain a valid resource id"),
            origin: ResourceOrigin::Unknown,
            discovery,
            installation: installation_state,
            provider_enablement: EnablementState::Unknown,
            selection: if activation_qualified {
                SelectionState::Selectable
            } else {
                SelectionState::NotSelectable
            },
            qualification: if activation_qualified {
                QualificationState::Qualified
            } else {
                QualificationState::Unqualified
            },
            required_dependencies: Vec::new(),
            activation_policy: ActivationPolicy::AtomicBundle,
            reason_code: Some(reason_code.to_owned()),
        },
        root,
        declared_components,
        effective_components,
        conflicts,
    }
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

enum LocatedPlugin {
    Installed(PathBuf),
    NotInstalled,
    Ambiguous,
    InvalidState,
}

fn locate_plugin(
    provider: Provider,
    home: &Path,
    codex_home: Option<&Path>,
    plugin_id: &str,
) -> LocatedPlugin {
    if !valid_plugin_key(provider, plugin_id) {
        return LocatedPlugin::InvalidState;
    }
    match provider {
        Provider::Codex => {
            let codex_home = codex_home
                .map(Path::to_path_buf)
                .unwrap_or_else(|| home.join(".codex"));
            match codex_plugins::locate(&codex_home, plugin_id) {
                codex_plugins::PluginInstallation::Installed(root) => LocatedPlugin::Installed(root),
                codex_plugins::PluginInstallation::NotInstalled => LocatedPlugin::NotInstalled,
                codex_plugins::PluginInstallation::Ambiguous => LocatedPlugin::Ambiguous,
                codex_plugins::PluginInstallation::InvalidState => LocatedPlugin::InvalidState,
            }
        }
        Provider::Claude => match claude_plugins::locate(home, plugin_id) {
            claude_plugins::PluginInstallation::Installed(root) => LocatedPlugin::Installed(root),
            claude_plugins::PluginInstallation::NotInstalled => LocatedPlugin::NotInstalled,
            claude_plugins::PluginInstallation::Ambiguous => LocatedPlugin::Ambiguous,
            claude_plugins::PluginInstallation::InvalidState => LocatedPlugin::InvalidState,
        },
    }
}

fn plugin_surface_error_code(error: PluginSurfaceError) -> &'static str {
    match error {
        PluginSurfaceError::Unavailable => "PLUGIN_SURFACE_UNAVAILABLE",
        PluginSurfaceError::InvalidManifest => "PLUGIN_MANIFEST_INVALID",
        PluginSurfaceError::UnsupportedManifest => "PLUGIN_MANIFEST_UNSUPPORTED",
    }
}

#[cfg(test)]
mod tests {
    use super::{exact_tuple, inspect_plugin, Provider, CLAUDE_EXACT};
    use crate::catalog::resource::{QualificationState, SelectionState};
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "clroom-provider-inventory-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        let plugin = home.join(".claude/plugins/cache/example/superpowers/6.3.0");
        fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
        fs::create_dir_all(home.join(".claude/plugins")).unwrap();
        fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            r#"{"name":"superpowers","version":"6.3.0"}"#,
        )
        .unwrap();
        fs::write(
            home.join(".claude/plugins/installed_plugins.json"),
            format!(
                r#"{{"plugins":{{"superpowers@example":[{{"installPath":"{}"}}]}}}}"#,
                plugin.display()
            ),
        )
        .unwrap();
        (home, plugin)
    }

    #[test]
    fn claude_plugin_selectability_is_bound_to_exact_provider_tuple() {
        let (home, plugin) = fixture();
        let qualified = inspect_plugin(
            Provider::Claude,
            &home,
            None,
            "superpowers@example",
            true,
        );
        let canonical_plugin = fs::canonicalize(&plugin).unwrap();
        assert_eq!(qualified.root.as_deref(), Some(canonical_plugin.as_path()));
        assert_eq!(qualified.entry.selection, SelectionState::Selectable);
        assert_eq!(qualified.entry.qualification, QualificationState::Qualified);
        assert!(qualified.conflicts.is_empty());

        let drifted = inspect_plugin(
            Provider::Claude,
            &home,
            None,
            "superpowers@example",
            false,
        );
        assert_eq!(drifted.entry.selection, SelectionState::NotSelectable);
        assert_eq!(drifted.entry.qualification, QualificationState::Unqualified);
        assert!(
            drifted
                .conflicts
                .iter()
                .any(|reason| reason == "PROVIDER_TUPLE_NOT_QUALIFIED")
        );

        let _ = fs::remove_dir_all(home.parent().unwrap());
    }

    #[test]
    fn exact_tuple_is_platform_and_version_specific() {
        assert!(exact_tuple(Provider::Claude, CLAUDE_EXACT, "macos", "aarch64"));
        assert!(!exact_tuple(
            Provider::Claude,
            (CLAUDE_EXACT.0, CLAUDE_EXACT.1, CLAUDE_EXACT.2 + 1),
            "macos",
            "aarch64"
        ));
        assert!(!exact_tuple(Provider::Claude, CLAUDE_EXACT, "linux", "aarch64"));
    }
}
