use clroom::adapters::{claude::plugin_state as claude_plugins, codex::plugin_state as codex_plugins};
use clroom::catalog::plugin_surface::{
    PluginComponent, PluginSurfaceError, ProviderPluginSemantics, inspect_plugin_surface,
};
use clroom::catalog::resource::{
    ActivationPolicy, DiscoveryState, EnablementState, InstallationState, QualificationState,
    ResourceId, ResourceInfo, ResourceKind, ResourceOrigin, SelectionState,
};
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

use super::{output, process};

const SCHEMA_VERSION: &str = "clroom.provider-info.v1";
const CODEX_EXACT: (u64, u64, u64) = (0, 154, 0);
const CLAUDE_EXACT: (u64, u64, u64) = (2, 1, 263);

#[derive(Clone, Copy)]
enum Provider {
    Codex,
    Claude,
}

impl Provider {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    fn display(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
        }
    }

    fn exact_version(self) -> (u64, u64, u64) {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceTarget {
    plugin_id: String,
}

#[derive(Serialize)]
struct ProviderSummary {
    id: &'static str,
    installed: bool,
    version: Option<String>,
    os: &'static str,
    arch: &'static str,
}

#[derive(Serialize)]
struct CleanLaunchSummary {
    qualification: QualificationState,
    exact_target: String,
    reason_code: Option<&'static str>,
}

#[derive(Serialize)]
struct CapabilityInfo {
    id: &'static str,
    discovery: DiscoveryState,
    installation: InstallationState,
    provider_enablement: EnablementState,
    selection: SelectionState,
    qualification: QualificationState,
    reason_code: &'static str,
}

#[derive(Serialize)]
struct ResourceDetail {
    resource: ResourceInfo,
    declared_components: Vec<PluginComponent>,
    effective_components: Vec<PluginComponent>,
    qualified_closure: Vec<String>,
    conflicts: Vec<String>,
}

#[derive(Serialize)]
struct CombinedResourceSummary {
    qualified_closure: Vec<String>,
    conflicts: Vec<String>,
}

#[derive(Serialize)]
struct ProviderInfo {
    schema_version: &'static str,
    provider: ProviderSummary,
    clean_launch: CleanLaunchSummary,
    capabilities: Vec<CapabilityInfo>,
    resources: Vec<ResourceDetail>,
    combined: CombinedResourceSummary,
}

pub fn run(args: &[String], mode: output::Mode) -> Result<String, String> {
    let Some(provider) = args.first().and_then(|value| Provider::parse(value)) else {
        return Err("INFO_PROVIDER_REQUIRED: use info <codex|claude>".to_owned());
    };
    let targets = parse_resource_targets(provider, &args[1..])?;
    let report = inspect(provider, &targets);
    match mode {
        output::Mode::Human => Ok(render_human(provider, &report)),
        output::Mode::Json => serde_json::to_string_pretty(&report)
            .map_err(|_| "INFO_SERIALIZATION_FAILED".to_owned()),
    }
}

fn parse_resource_targets(
    provider: Provider,
    args: &[String],
) -> Result<Vec<ResourceTarget>, String> {
    let mut targets = BTreeSet::new();
    for argument in args {
        let Some(plugin_id) = argument.strip_prefix("plugin:") else {
            return Err(
                "INFO_RESOURCE_TARGET_INVALID: use separate plugin:<provider-resource-id> targets"
                    .to_owned(),
            );
        };
        if !valid_plugin_key(provider, plugin_id) {
            return Err(
                "INFO_RESOURCE_TARGET_INVALID: use separate plugin:<provider-resource-id> targets"
                    .to_owned(),
            );
        }
        targets.insert(ResourceTarget {
            plugin_id: plugin_id.to_owned(),
        });
    }
    Ok(targets.into_iter().collect())
}

fn valid_plugin_key(provider: Provider, value: &str) -> bool {
    let Some((plugin, marketplace)) = value.rsplit_once('@') else {
        return false;
    };
    valid_plugin_segment(plugin, true)
        && valid_plugin_segment(marketplace, matches!(provider, Provider::Claude))
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

fn inspect(provider: Provider, targets: &[ResourceTarget]) -> ProviderInfo {
    let exact = provider.exact_version();
    let exact_target = format!("{} / macOS / Apple Silicon", version_string(exact));
    let resolved = match provider {
        Provider::Codex => process::resolve_codex_executable(),
        Provider::Claude => process::resolve_claude_executable(),
    };

    let (installed, version, clean_qualification, clean_reason, exact_tuple) = match resolved {
        Err(_) => (
            false,
            None,
            QualificationState::Unqualified,
            Some("PROVIDER_NOT_INSTALLED"),
            false,
        ),
        Ok(executable) => {
            let identity = match provider {
                Provider::Codex => process::preflight_codex(&executable, &[]),
                Provider::Claude => process::preflight_claude(&executable),
            };
            match identity {
                Ok(identity) => {
                    let version = version_string(identity.version);
                    let platform_supported = identity.os == "macos" && identity.arch == "aarch64";
                    if !platform_supported {
                        (
                            true,
                            Some(version),
                            QualificationState::Unsupported,
                            Some("PLATFORM_NOT_QUALIFIED"),
                            false,
                        )
                    } else if identity.version == exact {
                        (
                            true,
                            Some(version),
                            QualificationState::Qualified,
                            None,
                            true,
                        )
                    } else {
                        (
                            true,
                            Some(version),
                            QualificationState::Unqualified,
                            Some("PROVIDER_VERSION_NOT_EXACT_TARGET"),
                            false,
                        )
                    }
                }
                Err(_) => (
                    true,
                    None,
                    QualificationState::Unqualified,
                    Some("PROVIDER_IDENTITY_PROBE_FAILED"),
                    false,
                ),
            }
        }
    };

    let known_discovery = if exact_tuple {
        DiscoveryState::Discoverable
    } else {
        DiscoveryState::Unknown
    };
    let unavailable_reason = if exact_tuple {
        None
    } else {
        Some("PROVIDER_TUPLE_NOT_QUALIFIED")
    };

    let resources = targets
        .iter()
        .map(|target| inspect_plugin(provider, target, exact_tuple))
        .collect::<Vec<_>>();
    let conflicts = resources
        .iter()
        .flat_map(|resource| resource.conflicts.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    ProviderInfo {
        schema_version: SCHEMA_VERSION,
        provider: ProviderSummary {
            id: provider.id(),
            installed,
            version,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        clean_launch: CleanLaunchSummary {
            qualification: clean_qualification,
            exact_target,
            reason_code: clean_reason,
        },
        capabilities: vec![
            capability(
                "browser",
                known_discovery,
                if exact_tuple {
                    "BROWSER_E2E_NOT_QUALIFIED"
                } else {
                    unavailable_reason.unwrap_or("PROVIDER_TUPLE_NOT_QUALIFIED")
                },
            ),
            capability(
                "plugins",
                known_discovery,
                if exact_tuple {
                    "PLUGIN_ACTIVATION_V04"
                } else {
                    unavailable_reason.unwrap_or("PROVIDER_TUPLE_NOT_QUALIFIED")
                },
            ),
            capability(
                "mcp",
                known_discovery,
                if exact_tuple {
                    "MCP_ACTIVATION_V04"
                } else {
                    unavailable_reason.unwrap_or("PROVIDER_TUPLE_NOT_QUALIFIED")
                },
            ),
        ],
        resources,
        combined: CombinedResourceSummary {
            qualified_closure: Vec::new(),
            conflicts,
        },
    }
}

fn inspect_plugin(provider: Provider, target: &ResourceTarget, exact_tuple: bool) -> ResourceDetail {
    let installation = locate_plugin(provider, &target.plugin_id);
    let (installation_state, discovery, root, mut conflicts, reason_code) = match installation {
        LocatedPlugin::Installed(root) => (
            InstallationState::Installed,
            DiscoveryState::Discoverable,
            Some(root),
            Vec::new(),
            if exact_tuple {
                "PLUGIN_ACTIVATION_V04"
            } else {
                "PROVIDER_TUPLE_NOT_QUALIFIED"
            },
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

    let (declared_components, effective_components) = if exact_tuple {
        root.as_deref()
            .map(|root| inspect_plugin_surface(provider.plugin_semantics(), root))
            .map(|result| match result {
                Ok(surface) => (surface.declared, surface.effective),
                Err(error) => {
                    let blocker = plugin_surface_error_code(error).to_owned();
                    conflicts.push(blocker);
                    (Vec::new(), Vec::new())
                }
            })
            .unwrap_or_default()
    } else {
        (Vec::new(), Vec::new())
    };

    if installation_state == InstallationState::Installed && conflicts.is_empty() {
        conflicts.push(reason_code.to_owned());
    }

    ResourceDetail {
        resource: ResourceInfo {
            resource: ResourceId::new(provider.id(), ResourceKind::Plugin, &target.plugin_id)
                .expect("validated plugin target"),
            origin: ResourceOrigin::Unknown,
            discovery,
            installation: installation_state,
            provider_enablement: EnablementState::Unknown,
            selection: SelectionState::NotSelectable,
            qualification: QualificationState::Unqualified,
            required_dependencies: Vec::new(),
            activation_policy: ActivationPolicy::AtomicBundle,
            reason_code: Some(reason_code.to_owned()),
        },
        declared_components,
        effective_components,
        qualified_closure: Vec::new(),
        conflicts,
    }
}

enum LocatedPlugin {
    Installed(PathBuf),
    NotInstalled,
    Ambiguous,
    InvalidState,
}

fn locate_plugin(provider: Provider, plugin_id: &str) -> LocatedPlugin {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return LocatedPlugin::InvalidState;
    };
    match provider {
        Provider::Codex => {
            let codex_home = std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"));
            match codex_plugins::locate(&codex_home, plugin_id) {
                codex_plugins::PluginInstallation::Installed(root) => LocatedPlugin::Installed(root),
                codex_plugins::PluginInstallation::NotInstalled => LocatedPlugin::NotInstalled,
                codex_plugins::PluginInstallation::Ambiguous => LocatedPlugin::Ambiguous,
                codex_plugins::PluginInstallation::InvalidState => LocatedPlugin::InvalidState,
            }
        }
        Provider::Claude => match claude_plugins::locate(&home, plugin_id) {
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

fn capability(
    id: &'static str,
    discovery: DiscoveryState,
    reason_code: &'static str,
) -> CapabilityInfo {
    CapabilityInfo {
        id,
        discovery,
        installation: InstallationState::Unknown,
        provider_enablement: EnablementState::Unknown,
        selection: SelectionState::NotSelectable,
        qualification: QualificationState::Unqualified,
        reason_code,
    }
}

fn render_human(provider: Provider, report: &ProviderInfo) -> String {
    let provider_line = match report.provider.version.as_deref() {
        Some(version) => format!("Provider: {} {version}", provider.display()),
        None if report.provider.installed => {
            format!("Provider: {} (version unknown)", provider.display())
        }
        None => format!("Provider: {} (not installed)", provider.display()),
    };
    let clean = qualification_name(report.clean_launch.qualification);
    let mut lines = vec![
        provider_line,
        format!("Clean launch: {clean}"),
        format!("Exact target: {}", report.clean_launch.exact_target),
    ];
    if let Some(reason) = report.clean_launch.reason_code {
        lines.push(format!("Clean launch reason: {reason}"));
    }
    for capability in &report.capabilities {
        lines.push(format!(
            "{}: {} / {} / {} ({})",
            capability_label(capability.id),
            discovery_name(capability.discovery),
            selection_name(capability.selection),
            qualification_name(capability.qualification),
            capability.reason_code,
        ));
    }
    if report.resources.is_empty() {
        lines.push("Resources: no resource targets requested".to_owned());
    } else {
        for detail in &report.resources {
            lines.push(format!("Resource: {}", detail.resource.resource.canonical()));
            lines.push(format!(
                "  installation: {}",
                installation_name(detail.resource.installation)
            ));
            lines.push(format!(
                "  provider enablement: {}",
                enablement_name(detail.resource.provider_enablement)
            ));
            lines.push(format!(
                "  selection: {} / {}",
                selection_name(detail.resource.selection),
                qualification_name(detail.resource.qualification)
            ));
            lines.push(format!(
                "  declared components: {}",
                component_summary(&detail.declared_components)
            ));
            lines.push(format!(
                "  effective components: {}",
                component_summary(&detail.effective_components)
            ));
            lines.push(format!(
                "  qualified closure: {}",
                if detail.qualified_closure.is_empty() {
                    "none".to_owned()
                } else {
                    detail.qualified_closure.join(", ")
                }
            ));
            if !detail.conflicts.is_empty() {
                lines.push(format!("  conflicts: {}", detail.conflicts.join(", ")));
            }
        }
        lines.push(format!(
            "Combined qualified closure: {}",
            if report.combined.qualified_closure.is_empty() {
                "none".to_owned()
            } else {
                report.combined.qualified_closure.join(", ")
            }
        ));
        if !report.combined.conflicts.is_empty() {
            lines.push(format!(
                "Combined conflicts: {}",
                report.combined.conflicts.join(", ")
            ));
        }
    }
    lines.push(format!("Schema: {}", report.schema_version));
    lines.join("\n")
}

fn component_summary(components: &[PluginComponent]) -> String {
    if components.is_empty() {
        return "none or unknown".to_owned();
    }
    components
        .iter()
        .map(|component| format!("{}:{}", resource_kind_name(component.kind), component.id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Skill => "skill",
        ResourceKind::Plugin => "plugin",
        ResourceKind::McpServer => "mcp",
        ResourceKind::HookSet => "hook",
        ResourceKind::Agent => "agent",
        ResourceKind::Browser => "browser",
        ResourceKind::AppConnector => "app",
        ResourceKind::LspServer => "lsp",
        ResourceKind::Monitor => "monitor",
        ResourceKind::PluginExecutable => "bin",
        ResourceKind::SettingsOverlay => "settings",
    }
}

fn capability_label(id: &str) -> &str {
    match id {
        "browser" => "Browser",
        "plugins" => "Plugins",
        "mcp" => "MCP",
        _ => id,
    }
}

fn qualification_name(state: QualificationState) -> &'static str {
    match state {
        QualificationState::Qualified => "qualified",
        QualificationState::Unqualified => "unqualified",
        QualificationState::Unsupported => "unsupported",
    }
}

fn discovery_name(state: DiscoveryState) -> &'static str {
    match state {
        DiscoveryState::Discoverable => "discoverable",
        DiscoveryState::NotDiscoverable => "not discoverable",
        DiscoveryState::Unknown => "unknown",
    }
}

fn installation_name(state: InstallationState) -> &'static str {
    match state {
        InstallationState::Installed => "installed",
        InstallationState::NotInstalled => "not installed",
        InstallationState::Unknown => "unknown",
    }
}

fn enablement_name(state: EnablementState) -> &'static str {
    match state {
        EnablementState::Enabled => "enabled",
        EnablementState::Disabled => "disabled",
        EnablementState::Unknown => "unknown",
    }
}

fn selection_name(state: SelectionState) -> &'static str {
    match state {
        SelectionState::Selectable => "selectable",
        SelectionState::NotSelectable => "not selectable",
        SelectionState::Unknown => "unknown",
    }
}

fn version_string(version: (u64, u64, u64)) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

#[cfg(test)]
mod tests {
    use super::{Provider, inspect};

    #[test]
    fn provider_info_keeps_provider_state_and_clroom_qualification_orthogonal() {
        let report = inspect(Provider::Codex, &[]);
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["schema_version"], "clroom.provider-info.v1");
        assert!(value["provider"].get("installed").is_some());
        assert!(value["clean_launch"].get("qualification").is_some());
        assert!(value["capabilities"].is_array());
        assert!(value["resources"].is_array());
        assert!(value["combined"]["qualified_closure"].is_array());
        assert!(value["combined"]["conflicts"].is_array());
    }
}
