use super::budget::{BodyMeasurement, CatalogLimits, budget_catalog};
use super::dependency::{DependencyGraph, close_dependencies};
use super::inventory::inventory_skills;
use super::level_a::build_level_a;
use super::level_b::{DecisionRecord, SkillSignals, decide_bodies};
use super::manifest::CatalogManifest;
use super::sources::SkillSource;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct ValidatedCatalog {
    manifest: CatalogManifest,
}

impl ValidatedCatalog {
    pub fn manifest(&self) -> &CatalogManifest {
        &self.manifest
    }

    #[cfg(test)]
    pub(crate) fn from_manifest_for_test(manifest: CatalogManifest) -> Self {
        Self { manifest }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineError {
    Inventory,
    LevelA,
    Decisions,
    Dependencies,
    Budget,
    Manifest,
}

pub fn build_validated_catalog(
    sources: &[SkillSource],
    signals: &[SkillSignals],
    graph: &DependencyGraph,
    measurements: &[BodyMeasurement],
    limits: CatalogLimits,
    unavailable_sources: Vec<String>,
) -> Result<ValidatedCatalog, PipelineError> {
    let records = inventory_skills(sources).map_err(|_| PipelineError::Inventory)?;
    let level_a = build_level_a(&records).map_err(|_| PipelineError::LevelA)?;
    let decisions = decide_bodies(signals).map_err(|_| PipelineError::Decisions)?;
    let original: BTreeMap<_, _> = decisions.iter().map(|d| (d.id.as_str(), d)).collect();
    let closed = close_dependencies(&decisions, graph).map_err(|_| PipelineError::Dependencies)?;
    let closed_decisions: Vec<_> = closed
        .into_iter()
        .map(|item| DecisionRecord {
            id: item.id.clone(),
            decision: item.decision,
            reason: if original
                .get(item.id.as_str())
                .is_some_and(|d| d.decision == item.decision)
            {
                original[item.id.as_str()].reason
            } else {
                "REQUIRED_DEPENDENCY"
            },
        })
        .collect();
    let budgeted = budget_catalog(&level_a, &closed_decisions, measurements, limits)
        .map_err(|_| PipelineError::Budget)?;
    let manifest = CatalogManifest::new(budgeted.level_a, budgeted.decisions, unavailable_sources)
        .map_err(|_| PipelineError::Manifest)?;
    Ok(ValidatedCatalog { manifest })
}
