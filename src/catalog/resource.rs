use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Skill,
    Plugin,
    McpServer,
    HookSet,
    Agent,
    AppConnector,
    LspServer,
    Monitor,
    PluginExecutable,
    SettingsOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryState {
    Discoverable,
    NotDiscoverable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationState {
    Installed,
    NotInstalled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnablementState {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionState {
    Selectable,
    NotSelectable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationState {
    Qualified,
    Unqualified,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceOrigin {
    User,
    Project,
    Managed,
    ProviderBundled,
    Plugin,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationPolicy {
    Standalone,
    AtomicBundle,
    ProviderNative,
    ObservationOnly,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ResourceId {
    pub provider: String,
    pub kind: ResourceKind,
    pub id: String,
}

impl ResourceId {
    pub fn new(provider: &str, kind: ResourceKind, id: &str) -> Result<Self, ResourceIdError> {
        if !valid_component(provider) || !valid_component(id) {
            return Err(ResourceIdError);
        }
        Ok(Self {
            provider: provider.to_owned(),
            kind,
            id: id.to_owned(),
        })
    }

    pub fn canonical(&self) -> String {
        format!("{}:{}:{}", self.provider, kind_name(self.kind), self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceIdError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceInfo {
    pub resource: ResourceId,
    pub origin: ResourceOrigin,
    pub discovery: DiscoveryState,
    pub installation: InstallationState,
    pub provider_enablement: EnablementState,
    pub selection: SelectionState,
    pub qualification: QualificationState,
    pub required_dependencies: Vec<String>,
    pub activation_policy: ActivationPolicy,
    pub reason_code: Option<String>,
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().all(|character| {
            character.is_ascii_graphic() && !matches!(character, ',' | ':')
        })
}

fn kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Skill => "skill",
        ResourceKind::Plugin => "plugin",
        ResourceKind::McpServer => "mcp",
        ResourceKind::HookSet => "hook",
        ResourceKind::Agent => "agent",
        ResourceKind::AppConnector => "app",
        ResourceKind::LspServer => "lsp",
        ResourceKind::Monitor => "monitor",
        ResourceKind::PluginExecutable => "bin",
        ResourceKind::SettingsOverlay => "settings",
    }
}

#[cfg(test)]
mod tests {
    use super::{ResourceId, ResourceKind};

    #[test]
    fn canonical_ids_are_typed_and_stable() {
        let id = ResourceId::new(
            "codex",
            ResourceKind::Plugin,
            "chrome@openai-bundled",
        )
        .unwrap();
        assert_eq!(id.canonical(), "codex:plugin:chrome@openai-bundled");
    }

    #[test]
    fn ids_refuse_whitespace_control_and_selector_delimiters() {
        for invalid in ["", "two words", "bad,member", "bad:member", "line\nbreak"] {
            assert!(ResourceId::new("codex", ResourceKind::Plugin, invalid).is_err());
        }
    }

    #[test]
    fn catalog_does_not_model_browser_capability_as_a_resource_kind() {
        let kinds = [
            ResourceKind::Skill,
            ResourceKind::Plugin,
            ResourceKind::McpServer,
            ResourceKind::HookSet,
            ResourceKind::Agent,
            ResourceKind::AppConnector,
            ResourceKind::LspServer,
            ResourceKind::Monitor,
            ResourceKind::PluginExecutable,
            ResourceKind::SettingsOverlay,
        ];
        let encoded = serde_json::to_value(kinds).unwrap();
        assert_eq!(encoded.as_array().unwrap().len(), 10);
    }
}
