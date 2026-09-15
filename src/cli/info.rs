use clroom::catalog::resource::{
    DiscoveryState, EnablementState, InstallationState, QualificationState, ResourceInfo,
    SelectionState,
};
use serde::Serialize;

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
struct ProviderInfo {
    schema_version: &'static str,
    provider: ProviderSummary,
    clean_launch: CleanLaunchSummary,
    capabilities: Vec<CapabilityInfo>,
    resources: Vec<ResourceInfo>,
}

pub fn run(args: &[String], mode: output::Mode) -> Result<String, String> {
    let Some(provider) = args.first().and_then(|value| Provider::parse(value)) else {
        return Err("INFO_PROVIDER_REQUIRED: use info <codex|claude>".to_owned());
    };
    if args.len() > 1 {
        return Err(
            "INFO_RESOURCE_TARGETS_UNAVAILABLE: this build supports provider-level info only"
                .to_owned(),
        );
    }

    let report = inspect(provider);
    match mode {
        output::Mode::Human => Ok(render_human(provider, &report)),
        output::Mode::Json => serde_json::to_string_pretty(&report)
            .map_err(|_| "INFO_SERIALIZATION_FAILED".to_owned()),
    }
}

fn inspect(provider: Provider) -> ProviderInfo {
    let exact = provider.exact_version();
    let exact_target = format!(
        "{} / macOS / Apple Silicon",
        version_string(exact)
    );
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
        resources: Vec::new(),
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
        None if report.provider.installed => format!("Provider: {} (version unknown)", provider.display()),
        None => format!("Provider: {} (not installed)", provider.display()),
    };
    let clean = state_name(report.clean_launch.qualification);
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
            state_name(capability.qualification),
            capability.reason_code,
        ));
    }
    lines.push("Resources: provider-level report only; detailed inventory not inspected".to_owned());
    lines.push(format!("Schema: {}", report.schema_version));
    lines.join("\n")
}

fn capability_label(id: &str) -> &str {
    match id {
        "browser" => "Browser",
        "plugins" => "Plugins",
        "mcp" => "MCP",
        _ => id,
    }
}

fn state_name(state: QualificationState) -> &'static str {
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
        let report = inspect(Provider::Codex);
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["schema_version"], "clroom.provider-info.v1");
        assert!(value["provider"].get("installed").is_some());
        assert!(value["clean_launch"].get("qualification").is_some());
        assert!(value["capabilities"].is_array());
        assert!(value["resources"].is_array());
    }
}
