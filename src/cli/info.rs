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
struct InventoryLayer {
    status: &'static str,
    resources: Vec<String>,
}

#[derive(Serialize)]
struct InventoryLayers {
    declared: InventoryLayer,
    effective: InventoryLayer,
    qualified: InventoryLayer,
}

#[derive(Serialize)]
struct Qualification {
    status: &'static str,
    exact_target: String,
    platform_target: &'static str,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ProviderInfo {
    schema_version: &'static str,
    provider: &'static str,
    installed: bool,
    version: Option<String>,
    qualification: Qualification,
    inventory: InventoryLayers,
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
    let exact_target = version_string(exact);
    let resolved = match provider {
        Provider::Codex => process::resolve_codex_executable(),
        Provider::Claude => process::resolve_claude_executable(),
    };

    let (installed, version, qualification_status, reason) = match resolved {
        Err(_) => (false, None, "absent", None),
        Ok(executable) => {
            let identity = match provider {
                Provider::Codex => process::preflight_codex(&executable, &[]),
                Provider::Claude => process::preflight_claude(&executable),
            };
            match identity {
                Ok(identity) => {
                    let version = version_string(identity.version);
                    let exact_platform = identity.os == "macos" && identity.arch == "aarch64";
                    let status = if exact_platform && identity.version == exact {
                        "exact_qualified"
                    } else {
                        "unqualified"
                    };
                    let reason = (status == "unqualified").then(|| {
                        "installed provider tuple does not match the exact qualified target"
                            .to_owned()
                    });
                    (true, Some(version), status, reason)
                }
                Err(message) => (true, None, "unqualified", Some(message)),
            }
        }
    };

    ProviderInfo {
        schema_version: SCHEMA_VERSION,
        provider: provider.id(),
        installed,
        version,
        qualification: Qualification {
            status: qualification_status,
            exact_target,
            platform_target: "macOS / Apple Silicon",
            reason,
        },
        inventory: InventoryLayers {
            declared: InventoryLayer {
                status: "not_inspected",
                resources: Vec::new(),
            },
            effective: InventoryLayer {
                status: "not_inspected",
                resources: Vec::new(),
            },
            qualified: InventoryLayer {
                status: "not_inspected",
                resources: Vec::new(),
            },
        },
    }
}

fn render_human(provider: Provider, report: &ProviderInfo) -> String {
    let version = report.version.as_deref().unwrap_or("unknown");
    let installed = if report.installed { "yes" } else { "no" };
    let mut lines = vec![
        format!("Provider: {}", provider.display()),
        format!("Installed: {installed}"),
        format!("Version: {version}"),
        format!("CLROOM qualification: {}", report.qualification.status),
        format!(
            "Exact target: {} ({})",
            report.qualification.exact_target, report.qualification.platform_target
        ),
    ];
    if let Some(reason) = &report.qualification.reason {
        lines.push(format!("Qualification detail: {reason}"));
    }
    lines.extend([
        "Resource inventory:".to_owned(),
        "  declared: not inspected".to_owned(),
        "  effective: not inspected".to_owned(),
        "  qualified: not inspected".to_owned(),
        format!("Schema: {}", report.schema_version),
    ]);
    lines.join("\n")
}

fn version_string(version: (u64, u64, u64)) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

#[cfg(test)]
mod tests {
    use super::{
        InventoryLayer, InventoryLayers, ProviderInfo, Qualification, SCHEMA_VERSION,
    };

    #[test]
    fn json_keeps_declared_effective_and_qualified_separate() {
        let report = ProviderInfo {
            schema_version: SCHEMA_VERSION,
            provider: "codex",
            installed: true,
            version: Some("0.154.0".to_owned()),
            qualification: Qualification {
                status: "exact_qualified",
                exact_target: "0.154.0".to_owned(),
                platform_target: "macOS / Apple Silicon",
                reason: None,
            },
            inventory: InventoryLayers {
                declared: InventoryLayer {
                    status: "not_inspected",
                    resources: vec![],
                },
                effective: InventoryLayer {
                    status: "not_inspected",
                    resources: vec![],
                },
                qualified: InventoryLayer {
                    status: "not_inspected",
                    resources: vec![],
                },
            },
        };
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["schema_version"], "clroom.provider-info.v1");
        assert_eq!(value["inventory"]["declared"]["status"], "not_inspected");
        assert_eq!(value["inventory"]["effective"]["status"], "not_inspected");
        assert_eq!(value["inventory"]["qualified"]["status"], "not_inspected");
    }
}
