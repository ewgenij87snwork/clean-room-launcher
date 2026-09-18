use clroom::catalog::plugin_surface::PluginComponent;
use clroom::catalog::provider_inventory::{self, Provider};
use clroom::catalog::resource::{
    DiscoveryState, EnablementState, InstallationState, QualificationState, ResourceInfo,
    SelectionState,
};
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

use super::{output, process};

const SCHEMA_VERSION: &str = "clroom.provider-info.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NativeTarget {
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
struct NativeEntryDetail {
    #[serde(flatten)]
    entry: ResourceInfo,
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
    native_entries: Vec<NativeEntryDetail>,
    combined: CombinedResourceSummary,
}

pub fn run(args: &[String], mode: output::Mode) -> Result<String, String> {
    let Some(provider) = args.first().and_then(|value| Provider::parse(value)) else {
        return Err("INFO_PROVIDER_REQUIRED: use info <codex|claude>".to_owned());
    };
    let targets = parse_native_targets(provider, &args[1..])?;
    let report = inspect(provider, &targets);
    match mode {
        output::Mode::Human => Ok(render_human(provider, &report)),
        output::Mode::Json => serde_json::to_string_pretty(&report)
            .map_err(|_| "INFO_SERIALIZATION_FAILED".to_owned()),
    }
}

fn parse_native_targets(provider: Provider, args: &[String]) -> Result<Vec<NativeTarget>, String> {
    let mut targets = BTreeSet::new();
    for argument in args {
        let Some(plugin_id) = argument.strip_prefix("plugin:") else {
            return Err(
                "INFO_NATIVE_TARGET_INVALID: use separate plugin:<provider-native-id> targets"
                    .to_owned(),
            );
        };
        if !provider_inventory::valid_plugin_key(provider, plugin_id) {
            return Err(
                "INFO_NATIVE_TARGET_INVALID: use separate plugin:<provider-native-id> targets"
                    .to_owned(),
            );
        }
        targets.insert(NativeTarget {
            plugin_id: plugin_id.to_owned(),
        });
    }
    Ok(targets.into_iter().collect())
}

fn inspect(provider: Provider, targets: &[NativeTarget]) -> ProviderInfo {
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
                    } else if provider_inventory::exact_tuple(
                        provider,
                        identity.version,
                        &identity.os,
                        &identity.arch,
                    ) {
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

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let codex_home = if provider == Provider::Codex {
        home.as_ref().map(|home| {
            std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"))
        })
    } else {
        None
    };
    let native_entries = targets
        .iter()
        .map(|target| {
            inspect_plugin_detail(
                provider,
                target,
                home.as_deref(),
                codex_home.as_deref(),
                exact_tuple,
            )
        })
        .collect::<Vec<_>>();
    let conflicts = native_entries
        .iter()
        .flat_map(|entry| entry.conflicts.iter().cloned())
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
        native_entries,
        combined: CombinedResourceSummary {
            qualified_closure: Vec::new(),
            conflicts,
        },
    }
}

fn inspect_plugin_detail(
    provider: Provider,
    target: &NativeTarget,
    home: Option<&std::path::Path>,
    codex_home: Option<&std::path::Path>,
    tuple_qualified: bool,
) -> NativeEntryDetail {
    let inventory = provider_inventory::inspect_plugin_with_home(
        provider,
        home,
        codex_home,
        &target.plugin_id,
        tuple_qualified,
    );
    NativeEntryDetail {
        entry: inventory.entry,
        declared_components: inventory.declared_components,
        effective_components: inventory.effective_components,
        qualified_closure: Vec::new(),
        conflicts: inventory.conflicts,
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
    if report.native_entries.is_empty() {
        lines.push("Native entries: no targets requested".to_owned());
    } else {
        for detail in &report.native_entries {
            lines.push(format!("Native entry: {}", detail.entry.resource.canonical()));
            lines.push(format!(
                "  installation: {}",
                installation_name(detail.entry.installation)
            ));
            lines.push(format!(
                "  provider enablement: {}",
                enablement_name(detail.entry.provider_enablement)
            ));
            lines.push(format!(
                "  selection: {} / {}",
                selection_name(detail.entry.selection),
                qualification_name(detail.entry.qualification)
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
        .map(|component| format!("{}:{}", component.kind.as_str(), component.id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn capability_label(id: &str) -> &str {
    match id {
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
    use super::{inspect, Provider};

    #[test]
    fn provider_info_keeps_provider_state_and_clroom_qualification_orthogonal() {
        let report = inspect(Provider::Codex, &[]);
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["schema_version"], "clroom.provider-info.v1");
        assert!(value["provider"].get("installed").is_some());
        assert!(value["clean_launch"].get("qualification").is_some());
        assert!(value["capabilities"].is_array());
        assert!(value["native_entries"].is_array());
        assert!(value["combined"]["qualified_closure"].is_array());
        assert!(value["combined"]["conflicts"].is_array());
        assert!(value["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .all(|capability| capability["id"] != "browser"));
    }
}
