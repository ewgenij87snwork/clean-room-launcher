use super::dependency::{close_dependencies, DependencyError, DependencyGraph};
use super::level_b::{BodyDecision, DecisionRecord};
use super::resource::{
    ActivationPolicy, EnablementState, InstallationState, QualificationState, ResourceId,
    ResourceInfo, ResourceKind, ResourceOrigin, SelectionState,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectionTarget {
    All,
    Exact { kind: ResourceKind, id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionRequest {
    pub includes: BTreeSet<SelectionTarget>,
    pub excludes: BTreeSet<SelectionTarget>,
    global_skill_ids: Option<BTreeSet<String>>,
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

    pub fn narrow_global_skills(&mut self, ids: impl IntoIterator<Item = String>) {
        self.global_skill_ids = Some(ids.into_iter().collect());
    }

    pub fn is_empty(&self) -> bool {
        self.includes.is_empty() && self.excludes.is_empty() && self.global_skill_ids.is_none()
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
    UnknownTarget(String),
    DuplicateResource(String),
    NotSelectable(String),
    AllEffectiveMemberUnqualified(String),
    AtomicExclusionConflict(String),
    InvalidGraph(DependencyError),
    RequiredDependencyRefused(String),
}

impl SelectionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSelector => "CLROOM_RESOURCE_SELECTOR_INVALID",
            Self::UnknownTarget(_) => "CLROOM_RESOURCE_SELECTOR_UNKNOWN",
            Self::DuplicateResource(_) | Self::InvalidGraph(_) => "CLROOM_RESOURCE_GRAPH_INVALID",
            Self::NotSelectable(_) => "CLROOM_RESOURCE_NOT_SELECTABLE",
            Self::AllEffectiveMemberUnqualified(_) => "CLROOM_RESOURCE_ALL_INCOMPLETE",
            Self::AtomicExclusionConflict(_) => "CLROOM_RESOURCE_EXCLUSION_CONFLICT",
            Self::RequiredDependencyRefused(_) => "CLROOM_RESOURCE_DEPENDENCY_REFUSED",
        }
    }
}

pub fn plan_selection(
    provider: &str,
    request: &SelectionRequest,
    resources: &[ResourceInfo],
) -> Result<SelectionPlan, SelectionError> {
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

    let excludes = resolve_targets(
        provider,
        &request.excludes,
        &by_id,
        false,
        &BTreeSet::new(),
        request.global_skill_ids.as_ref(),
    )?;
    let includes = resolve_targets(
        provider,
        &request.includes,
        &by_id,
        true,
        &excludes,
        request.global_skill_ids.as_ref(),
    )?;

    for included in &includes {
        let resource = by_id
            .get(included)
            .expect("resolved selection must reference known resource");
        if resource.activation_policy == ActivationPolicy::AtomicBundle {
            for dependency in &resource.required_dependencies {
                if excludes.contains(dependency) {
                    return Err(SelectionError::AtomicExclusionConflict(
                        dependency.clone(),
                    ));
                }
            }
        }
    }

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

    let graph = DependencyGraph::from_edges(
        by_id
            .iter()
            .map(|(id, resource)| (id.clone(), resource.required_dependencies.clone())),
    );
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
    validate_all_qualification: bool,
    skipped: &BTreeSet<String>,
    global_skill_ids: Option<&BTreeSet<String>>,
) -> Result<BTreeSet<String>, SelectionError> {
    let mut resolved = BTreeSet::new();
    for target in targets {
        if matches!(target, SelectionTarget::All) {
            for (canonical, resource) in resources {
                if skipped.contains(canonical)
                    || !all_effective_member(resource, global_skill_ids)
                {
                    continue;
                }
                if validate_all_qualification
                    && (resource.selection != SelectionState::Selectable
                        || resource.qualification != QualificationState::Qualified)
                {
                    return Err(SelectionError::AllEffectiveMemberUnqualified(
                        canonical.clone(),
                    ));
                }
                resolved.insert(canonical.clone());
            }
            continue;
        }

        let canonical = match target {
            SelectionTarget::Exact { kind, id } => ResourceId::new(provider, *kind, id)
                .map_err(|_| SelectionError::InvalidSelector)?
                .canonical(),
            SelectionTarget::All => unreachable!(),
        };
        if !resources.contains_key(&canonical) {
            return Err(SelectionError::UnknownTarget(canonical));
        }
        resolved.insert(canonical);
    }
    Ok(resolved)
}

fn all_effective_member(
    resource: &ResourceInfo,
    global_skill_ids: Option<&BTreeSet<String>>,
) -> bool {
    if resource.installation != InstallationState::Installed
        || resource.provider_enablement != EnablementState::Enabled
        || matches!(resource.origin, ResourceOrigin::Project | ResourceOrigin::Managed)
    {
        return false;
    }

    if resource.resource.kind == ResourceKind::Skill
        && resource.origin == ResourceOrigin::User
        && global_skill_ids
            .is_some_and(|ids| !ids.contains(&resource.resource.id))
    {
        return false;
    }

    matches!(
        resource.activation_policy,
        ActivationPolicy::Standalone
            | ActivationPolicy::AtomicBundle
            | ActivationPolicy::ProviderNative
    )
}

fn parse_value(value: &str) -> Result<Vec<SelectionTarget>, SelectionError> {
    if value == "all" {
        return Ok(vec![SelectionTarget::All]);
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
    use super::{plan_selection, SelectionError, SelectionRequest, SelectionTarget};
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
    fn grammar_parses_exact_targets_deterministically() {
        let mut request = SelectionRequest::default();
        request
            .include_value("plugin:alpha@bundled,beta@bundled")
            .unwrap();
        request.exclude_value("mcp:ambient").unwrap();
        assert_eq!(request.includes.len(), 2);
        assert_eq!(request.excludes.len(), 1);
    }

    #[test]
    fn grammar_rejects_capability_aliases_and_malformed_members() {
        for value in [
            "",
            "browser",
            "plugin:",
            "mcp:a,,b",
            "plugin:two words",
            "hook:x",
        ] {
            let mut request = SelectionRequest::default();
            assert_eq!(
                request.include_value(value),
                Err(SelectionError::InvalidSelector)
            );
        }
    }

    #[test]
    fn dependency_closure_reuses_existing_resolver() {
        let dependency = "codex:mcp:helper";
        let resources = vec![
            resource(ResourceKind::McpServer, "helper", &[], true),
            resource(ResourceKind::Plugin, "compound@market", &[dependency], true),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("plugin:compound@market").unwrap();

        let plan = plan_selection("codex", &request, &resources).unwrap();
        let ids = plan
            .selected
            .iter()
            .map(|resource| resource.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"codex:plugin:compound@market"));
        assert!(ids.contains(&dependency));
        let dependency_record = plan
            .selected
            .iter()
            .find(|resource| resource.id == dependency)
            .unwrap();
        assert_eq!(
            dependency_record.reason_chain,
            vec!["codex:plugin:compound@market", dependency]
        );
    }

    #[test]
    fn exclusion_wins_but_required_exclusion_fails_closed() {
        let dependency = "codex:mcp:helper";
        let resources = vec![
            resource(ResourceKind::McpServer, "helper", &[], true),
            resource(ResourceKind::Plugin, "compound@market", &[dependency], true),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("plugin:compound@market").unwrap();
        request.exclude_value("mcp:helper").unwrap();

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::RequiredDependencyRefused(
                dependency.to_owned()
            ))
        );
    }

    #[test]
    fn atomic_bundle_internal_exclusion_refuses_instead_of_rewriting_bundle() {
        let hook = "codex:hook:session-start";
        let mut plugin = resource(ResourceKind::Plugin, "compound@market", &[hook], true);
        plugin.activation_policy = ActivationPolicy::AtomicBundle;
        let hook_resource = resource(ResourceKind::HookSet, "session-start", &[], true);
        let resources = vec![plugin, hook_resource];

        let mut request = SelectionRequest::default();
        request.includes.insert(SelectionTarget::Exact {
            kind: ResourceKind::Plugin,
            id: "compound@market".to_owned(),
        });
        request.excludes.insert(SelectionTarget::Exact {
            kind: ResourceKind::HookSet,
            id: "session-start".to_owned(),
        });

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::AtomicExclusionConflict(hook.to_owned()))
        );
    }

    #[test]
    fn unqualified_requested_resource_never_degrades_to_launch() {
        let resources = vec![resource(ResourceKind::Plugin, "demo@market", &[], false)];
        let mut request = SelectionRequest::default();
        request.include_value("plugin:demo@market").unwrap();

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::NotSelectable(
                "codex:plugin:demo@market".to_owned()
            ))
        );
    }

    #[test]
    fn all_expands_only_effective_installed_global_activation_resources() {
        let mut global = resource(ResourceKind::Plugin, "active@market", &[], true);
        global.activation_policy = ActivationPolicy::ProviderNative;

        let mut disabled = resource(ResourceKind::Plugin, "disabled@market", &[], true);
        disabled.provider_enablement = EnablementState::Disabled;

        let mut project = resource(ResourceKind::Skill, "project", &[], true);
        project.origin = ResourceOrigin::Project;
        project.activation_policy = ActivationPolicy::Standalone;

        let mut observation = resource(ResourceKind::HookSet, "observed", &[], true);
        observation.activation_policy = ActivationPolicy::ObservationOnly;

        let mut request = SelectionRequest::default();
        request.include_value("all").unwrap();
        let plan = plan_selection(
            "codex",
            &request,
            &[global, disabled, project, observation],
        )
        .unwrap();

        assert_eq!(
            plan.selected
                .iter()
                .map(|resource| resource.id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex:plugin:active@market"]
        );
    }

    #[test]
    fn all_skill_narrowing_affects_only_standalone_global_skills() {
        let mut arrow = resource(ResourceKind::Skill, "arrow", &[], true);
        arrow.origin = ResourceOrigin::User;
        arrow.activation_policy = ActivationPolicy::Standalone;

        let mut other = resource(ResourceKind::Skill, "other", &[], true);
        other.origin = ResourceOrigin::User;
        other.activation_policy = ActivationPolicy::Standalone;

        let mut plugin_skill = resource(ResourceKind::Skill, "plugin-skill", &[], true);
        plugin_skill.origin = ResourceOrigin::Plugin;
        plugin_skill.activation_policy = ActivationPolicy::AtomicBundle;

        let mut project_skill = resource(ResourceKind::Skill, "project-skill", &[], true);
        project_skill.origin = ResourceOrigin::Project;
        project_skill.activation_policy = ActivationPolicy::Standalone;

        let mut request = SelectionRequest::default();
        request.include_value("all").unwrap();
        request.narrow_global_skills(["arrow".to_owned()]);

        let plan = plan_selection(
            "codex",
            &request,
            &[arrow, other, plugin_skill, project_skill],
        )
        .unwrap();
        assert_eq!(
            plan.selected
                .iter()
                .map(|resource| resource.id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex:skill:arrow", "codex:skill:plugin-skill"]
        );
    }

    #[test]
    fn all_fails_closed_on_effective_unqualified_member() {
        let resources = vec![
            resource(ResourceKind::Plugin, "active@market", &[], true),
            resource(ResourceKind::Plugin, "ambient@market", &[], false),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("all").unwrap();

        assert_eq!(
            plan_selection("codex", &request, &resources),
            Err(SelectionError::AllEffectiveMemberUnqualified(
                "codex:plugin:ambient@market".to_owned()
            ))
        );
    }

    #[test]
    fn without_wins_over_all_including_unqualified_members() {
        let resources = vec![
            resource(ResourceKind::Plugin, "active@market", &[], true),
            resource(ResourceKind::Plugin, "ambient@market", &[], false),
        ];
        let mut request = SelectionRequest::default();
        request.include_value("all").unwrap();
        request.exclude_value("plugin:ambient@market").unwrap();

        let plan = plan_selection("codex", &request, &resources).unwrap();
        assert_eq!(
            plan.selected
                .iter()
                .map(|resource| resource.id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex:plugin:active@market"]
        );
        assert_eq!(plan.excluded, vec!["codex:plugin:ambient@market"]);
    }
}
