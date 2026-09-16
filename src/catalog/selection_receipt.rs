use super::resource::{ResourceInfo, ResourceKind};
use super::selection::SelectionPlan;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    InstructionContext,
    ProviderExtension,
    ExternalToolServer,
    LifecycleAutomation,
    DelegatedAgent,
    AuthorizedConnector,
    LanguageServerProcess,
    BackgroundProcess,
    ExecutableSurface,
    ProviderSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectionReceipt {
    pub selected_ids: Vec<String>,
    pub side_effect_classes: Vec<SideEffectClass>,
    /// Deterministic correlation fingerprint for the resolved set.
    /// This is intentionally not a cryptographic integrity or attestation primitive.
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionReceiptError {
    DuplicateResource(String),
    UnknownSelectedResource(String),
}

pub fn build_selection_receipt(
    plan: &SelectionPlan,
    resources: &[ResourceInfo],
) -> Result<SelectionReceipt, SelectionReceiptError> {
    let mut by_id = BTreeMap::new();
    for resource in resources {
        let canonical = resource.resource.canonical();
        if by_id.insert(canonical.clone(), resource).is_some() {
            return Err(SelectionReceiptError::DuplicateResource(canonical));
        }
    }

    let mut selected_ids = plan
        .selected
        .iter()
        .map(|resource| resource.id.clone())
        .collect::<Vec<_>>();
    selected_ids.sort();
    selected_ids.dedup();

    let mut side_effect_classes = BTreeSet::new();
    for selected in &selected_ids {
        let resource = by_id
            .get(selected)
            .ok_or_else(|| SelectionReceiptError::UnknownSelectedResource(selected.clone()))?;
        side_effect_classes.insert(side_effect_class(resource.resource.kind));
    }

    Ok(SelectionReceipt {
        digest: resolved_set_digest(&selected_ids),
        selected_ids,
        side_effect_classes: side_effect_classes.into_iter().collect(),
    })
}

fn side_effect_class(kind: ResourceKind) -> SideEffectClass {
    match kind {
        ResourceKind::Skill => SideEffectClass::InstructionContext,
        ResourceKind::Plugin => SideEffectClass::ProviderExtension,
        ResourceKind::McpServer => SideEffectClass::ExternalToolServer,
        ResourceKind::HookSet => SideEffectClass::LifecycleAutomation,
        ResourceKind::Agent => SideEffectClass::DelegatedAgent,
        ResourceKind::AppConnector => SideEffectClass::AuthorizedConnector,
        ResourceKind::LspServer => SideEffectClass::LanguageServerProcess,
        ResourceKind::Monitor => SideEffectClass::BackgroundProcess,
        ResourceKind::PluginExecutable => SideEffectClass::ExecutableSurface,
        ResourceKind::SettingsOverlay => SideEffectClass::ProviderSettings,
    }
}

fn resolved_set_digest(selected_ids: &[String]) -> String {
    // FNV-1a is used only to give an architecture-independent stable identifier
    // without adding a runtime dependency. The namespace explicitly prevents this
    // value from being mistaken for a cryptographic digest.
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001b3;
    let mut state = OFFSET;
    for byte in b"clroom.resolved-set.fnv1a64.v1\0" {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(PRIME);
    }
    for id in selected_ids {
        for byte in (id.len() as u64).to_le_bytes() {
            state ^= u64::from(byte);
            state = state.wrapping_mul(PRIME);
        }
        for byte in id.as_bytes() {
            state ^= u64::from(*byte);
            state = state.wrapping_mul(PRIME);
        }
    }
    format!("fnv1a64-v1:{state:016x}")
}

#[cfg(test)]
mod tests {
    use super::{build_selection_receipt, SelectionReceiptError, SideEffectClass};
    use crate::catalog::resource::{
        ActivationPolicy, DiscoveryState, EnablementState, InstallationState,
        QualificationState, ResourceId, ResourceInfo, ResourceKind, ResourceOrigin,
        SelectionState,
    };
    use crate::catalog::selection::{PlannedResource, SelectionPlan};

    fn resource(kind: ResourceKind, id: &str) -> ResourceInfo {
        ResourceInfo {
            resource: ResourceId::new("codex", kind, id).unwrap(),
            origin: ResourceOrigin::ProviderBundled,
            discovery: DiscoveryState::Discoverable,
            installation: InstallationState::Installed,
            provider_enablement: EnablementState::Enabled,
            selection: SelectionState::Selectable,
            qualification: QualificationState::Qualified,
            required_dependencies: Vec::new(),
            activation_policy: ActivationPolicy::ProviderNative,
            reason_code: None,
        }
    }

    fn plan(ids: &[&str]) -> SelectionPlan {
        SelectionPlan {
            selected: ids
                .iter()
                .map(|id| PlannedResource {
                    id: (*id).to_owned(),
                    reason_chain: vec![(*id).to_owned()],
                })
                .collect(),
            excluded: Vec::new(),
        }
    }

    #[test]
    fn receipt_is_stable_across_input_and_plan_order() {
        let mcp = resource(ResourceKind::McpServer, "browser-bridge");
        let plugin = resource(ResourceKind::Plugin, "demo@market");
        let first = build_selection_receipt(
            &plan(&["codex:plugin:demo@market", "codex:mcp:browser-bridge"]),
            &[mcp.clone(), plugin.clone()],
        )
        .unwrap();
        let second = build_selection_receipt(
            &plan(&["codex:mcp:browser-bridge", "codex:plugin:demo@market"]),
            &[plugin, mcp],
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first.selected_ids,
            vec!["codex:mcp:browser-bridge", "codex:plugin:demo@market"]
        );
        assert_eq!(
            first.side_effect_classes,
            vec![SideEffectClass::ProviderExtension, SideEffectClass::ExternalToolServer]
        );
        assert!(first.digest.starts_with("fnv1a64-v1:"));
        assert_eq!(first.digest.len(), "fnv1a64-v1:".len() + 16);
    }

    #[test]
    fn receipt_refuses_selected_ids_without_inventory_evidence() {
        let error = build_selection_receipt(&plan(&["codex:mcp:missing"]), &[]).unwrap_err();
        assert_eq!(
            error,
            SelectionReceiptError::UnknownSelectedResource("codex:mcp:missing".to_owned())
        );
    }
}
