use super::dependency::{DependencyError, DependencyGraph, close_dependencies};
use super::level_b::{BodyDecision, DecisionRecord};
use super::resource::{QualificationState, ResourceId, ResourceInfo, ResourceKind, SelectionState};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionTarget {
    Browser,
    All,
    Exact { kind: ResourceKind, id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionRequest {
    pub includes: BTreeSet<SelectionTarget>,
    pub excludes: BTreeSet<SelectionTarget>,
}

impl SelectionRequest {
    pub fn include_value(&mut self, value: &str) -> Result<(), SelectionError> {
        self.includes.extend(parse_value(value)?);
        Ok(())
    }

    pub fn exclude_value(&mut self, value: &str) -> Result<(), SelectionError> {
        self.excludes.extend(parse_value(value)?);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.includes.is_empty() && self.excludes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedResource {
    pub id: String,
    pub reason_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPlan {
    pub selected: Vec<PlannedResource>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    InvalidSelector,
    AllUnavailableInV03,
    UnknownTarget(String),
    DuplicateResource(String),
    NotSelectable(String),
    InvalidGraph(DependencyError),
    RequiredDependencyRefused(String),
}

impl SelectionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSelector => "CLROOM_RESOURCE_SELECTOR_INVALID",
            Self::AllUnavailableInV03 => "CLROOM_RESOURCE_ALL_UNAVAILABLE_IN_V0_3",
            Self::UnknownTarget(_) => "CLROOM_RESOURCE_SELECTOR_UNKNOWN",
            Self::DuplicateResource(_) | Self::InvalidGraph(_) => "CLROOM_RESOURCE_GRAPH_INVALID",
            Self::NotSelectable(_) => "CLROOM_RESOURCE_NOT_SELECTABLE",
            Self::RequiredDependencyRefused(_) => "CLROOM_RESOURCE_DEPENDENCY_REFUSED",
        }
    }
}

pub fn plan_selection(
    provider: &str,
    request: &SelectionRequest,
    resources: &[ResourceInfo],
) -> Result<SelectionPlan, SelectionError> {
    if request
        .includes
        .iter()
        .chain(request.excludes.iter())
        .any(|target| matches!(target, SelectionTarget::All))
    {
        return Err(SelectionError::AllUnavailableInV03);
    }

    let mut by_id = BTreeMap::new();
    for resource in resources
        .iter()
        .filter(|resource| resource.resource.provider == provider)
    {
        let canonical = resource.resource.canonical();
        if by_id.insert(canonical.clone(), resource).is_some() {
            return Err(SelectionError::DuplicateResource(canonical));
        }
    }

    let includes = resolve_targets(provider, &request.includes, &by_id)?;
    let excludes = resolve_targets(provider, &request.excludes, &by_id)?;

    let mut decisions = Vec::with_capacity(by_id.len());
    for (id, resource) in &by_id {
        let excluded = excludes.contains(id);
        let requested = includes.contains(id);
        let decision = if excluded {
            BodyDecision::Refuse
        } else if requested {
            if resource.selection != SelectionState::Selectable
                || resource.qualification != QualificationState::Qualified
            {
                return Err(SelectionError::NotSelectable(id.clone()));
            }
            BodyDecision::LoadNow
        } else if resource.selection == SelectionState::Selectable
            && resource.qualification == QualificationState::Qualified
        {
            BodyDecision::LoadOnInvoke
        } else {
            BodyDecision::Refuse
        };
        decisions.push(DecisionRecord {
            id: id.clone(),
            decision,
            reason: if excluded {
                "EXPLICIT_EXCLUSION"
            } else if requested {
                "EXPLICIT_SELECTION"
            } else if decision == BodyDecision::LoadOnInvoke {
                "QUALIFIED_DEFERRED"
            } else {
                "UNQUALIFIED_OR_UNSELECTABLE"
            },
            reason_chain: vec![id.clone()],
        });
    }

    let graph = DependencyGraph::from_edges(by_id.iter().map(|(id, resource)| {
        (id.clone(), resource.required_dependencies.clone())
    }));
    let closed = close_dependencies(&decisions, &graph).map_err(|error| match error {
        DependencyError::RequiredDependencyRefused(id) => {
            SelectionError::RequiredDependencyRefused(id)
        }
        other => SelectionError::InvalidGraph(other),
    })?;

    let selected = closed
        .into_iter()
        .filter(|record| record.decision == BodyDecision::LoadNow)
        .map(|record| PlannedResource {
            id: record.id,
            reason_chain: record.reason_chain,
        })
        .collect();

    Ok(SelectionPlan {
        selected,
        excluded: excludes.into_iter().collect(),
    })
}

fn resolve_targets(
    provider: &str,
    targets: &BTreeSet<SelectionTarget>,
    resources: &BTreeMap<String, &ResourceInfo>,
) -> Result<BTreeSet<String>, SelectionError> {
    let mut resolved = BTreeSet::new();
    for target in targets {
        let canonical = match target {
            SelectionTarget::Browser => ResourceId::new(provider, ResourceKind::Browser, "browser")
                .map_err(|_| SelectionError::InvalidSelector)?
                .canonical(),
            SelectionTarget::Exact { kind, id } => ResourceId::new(provider, *kind, id)
                .map_err(|_| SelectionError::InvalidSelector)?
                .canonical(),
            SelectionTarget::All => return Err(SelectionError::AllUnavailableInV03),
        };
        if !resources.contains_key(&canonical) {
            return Err(SelectionError::UnknownTarget(canonical));
        }
        resolved.insert(canonical);
    }
    Ok(resolved)
}

fn parse_value(value: &str) -> Result<Vec<SelectionTarget>, SelectionError> {
    match value {
        "browser" => return Ok(vec![SelectionTarget::Browser]),
        "all" => return Ok(vec![SelectionTarget::All]),
        _ => {}
    }

    let (kind, members) = if let Some(members) = value.strip_prefix("plugin:") {
        (ResourceKind::Plugin, members)
    } else if let Some(members) = value.strip_prefix("mcp:") {
        (ResourceKind::McpServer, members)
    } else {
        return Err(SelectionError::InvalidSelector);
    };

    if members.is_empty() {
        return Err(SelectionError::InvalidSelector);
    }
    members
        .split(',')
        .map(|member| {
            ResourceId::new("selector", kind, member)
                .map_err(|_| SelectionError::InvalidSelector)?;
            Ok(SelectionTarget::Exact {
                kind,
                id: member.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        SelectionError, SelectionRequest, SelectionTarget, plan_selection,
    };
    use crate::catalog::resource::{
        ActivationPolicy, DiscoveryState, EnablementState, InstallationState,
        QualificationState, ResourceId, ResourceInfo, ResourceKind, ResourceOrigin,
        SelectionState,
    };

    fn resource(
        kind: ResourceKind,
        id: &str,
        dependencies: &[&str],
        selectable: bool,
    ) -> ResourceInfo {
        ResourceInfo {
            resource: ResourceId::new("codex", kind, id).unwrap(),
            origin: ResourceOrigin::ProviderBundled,
            discovery: DiscoveryState::Discoverable,
            installation: InstallationState::Installed,
            provider_enablement: EnablementState::Enabled,
            selection: if selectable {
                SelectionState::Selectable
            } else {
                SelectionState::NotSelectable
            },
            qualification: if selectable {
                QualificationState::Qualified
            } else {
                QualificationState::Unqualified
            },
            required_dependencies: dependencies.iter().map(|value| (*value).to_owned()).collect(),
            activation_policy: ActivationPolicy::ProviderNative,
            reason_code: None,
        }
    }

    #[test]
    fn grammar_parses_portable_and_exact_targets_deterministically() {
        let mut request = SelectionRequest::default();
        request.include_value("browser").unwrap();
        request.include_value("plugin:chrome@bundled,computer-use@bundled").unwrap();
        request.exclude_value("mcp:ambient").unwrap();
        assert!(request.includes.contains(&SelectionTarget::Browser));
        assert_eq!(request.includes.len(), 3);
        assert_eq!(request.excludes.len(), 1);
    }

    #[test]
    fn grammar_rejects_empty_unknown_and_whitespace_members() {
        for value in ["", "plugin:", "mcp:a,,b", "plugin:two words", "hook:x"] {
            let mut request = SelectionRequest::default();
            assert_eq!(
                request.include_value(value),
                Err(SelectionError::InvalidSelector)
            );
        }
    }

    #[test]
    fn dependency_closure_reuses_existing_resolver() {
        let plugin = "codex:plugin:chrome@openai-bundled";
        let resources = vec![
            resource(ResourceKind::Plugin, "chrome@openai-bundled", &[], true),
            resource(ResourceKind::Browser, "browser", &[plugin], true),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("browser").unwrap();

        let plan = plan_selection("codex", &request, &resources).unwrap();
        assert_eq!(
            plan.selected
                .iter()
                .map(|resource| resource.id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex:browser:browser", plugin]
        );
        assert_eq!(
            plan.selected[1].reason_chain,
            vec!["codex:browser:browser", plugin]
        );
    }

    #[test]
    fn exclusion_wins_but_required_exclusion_fails_closed() {
        let plugin = "codex:plugin:chrome@openai-bundled";
        let resources = vec![
            resource(ResourceKind::Plugin, "chrome@openai-bundled", &[], true),
            resource(ResourceKind::Browser, "browser", &[plugin], true),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("browser").unwrap();
        request
            .exclude_value("plugin:chrome@openai-bundled")
            .unwrap();

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::RequiredDependencyRefused(plugin.to_owned()))
        );
    }

    #[test]
    fn unqualified_requested_resource_never_degrades_to_launch() {
        let resources = vec![resource(ResourceKind::Browser, "browser", &[], false)];
        let mut request = SelectionRequest::default();
        request.include_value("browser").unwrap();

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::NotSelectable(
                "codex:browser:browser".to_owned()
            ))
        );
    }

    #[test]
    fn all_is_modeled_but_not_activated_in_v03() {
        let mut request = SelectionRequest::default();
        request.include_value("all").unwrap();
        assert_eq!(
            plan_selection("codex", &request, &[]),
            Err(SelectionError::AllUnavailableInV03)
        );
    }
}
