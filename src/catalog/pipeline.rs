use super::budget::{BodyMeasurement, CatalogLimits, budget_catalog};
use super::dependency::{DependencyGraph, close_dependencies};
use super::inventory::inventory_skills;
use super::level_a::build_level_a;
use super::level_b::{DecisionRecord, SkillSignals, decide_bodies};
use super::manifest::CatalogManifest;
use super::protection::apply_baseline_protection;
use super::sources::SkillSource;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const P04_ACCEPTANCE_CONTROLS: [&str; 11] = [
    "SKL-01", "SKL-02", "SKL-03", "SKL-04", "SKL-05", "SKL-06", "SKL-07", "SKL-09", "SKL-12",
    "SKL-14", "SKL-15",
];

#[derive(Debug)]
pub struct ValidatedCatalog {
    manifest: CatalogManifest,
    body_paths: BTreeMap<String, PathBuf>,
}

impl ValidatedCatalog {
    pub fn manifest(&self) -> &CatalogManifest {
        &self.manifest
    }
    pub(crate) fn body_path(&self, id: &str) -> Option<&Path> {
        self.body_paths.get(id).map(PathBuf::as_path)
    }

    #[cfg(test)]
    pub(crate) fn from_manifest_for_test(
        manifest: CatalogManifest,
        body_paths: BTreeMap<String, PathBuf>,
    ) -> Self {
        Self {
            manifest,
            body_paths,
        }
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
    limits: CatalogLimits,
) -> Result<ValidatedCatalog, PipelineError> {
    let records = inventory_skills(sources).map_err(|_| PipelineError::Inventory)?;
    let source_roots: BTreeMap<_, _> = sources
        .iter()
        .map(|source| (source.id.as_str(), source.root.as_path()))
        .collect();
    let body_paths = records
        .iter()
        .map(|record| {
            let relative = record
                .body_path
                .strip_prefix(&format!("{}/", record.source_id))
                .ok_or(PipelineError::Inventory)?;
            let root = source_roots
                .get(record.source_id.as_str())
                .ok_or(PipelineError::Inventory)?;
            Ok((record.id.clone(), root.join(relative)))
        })
        .collect::<Result<_, _>>()?;
    let level_a = build_level_a(&records).map_err(|_| PipelineError::LevelA)?;
    let measurements: Vec<_> = records
        .iter()
        .map(|record| BodyMeasurement {
            id: record.id.clone(),
            bytes: record.body_bytes,
            token_upper_bound: record.body_bytes,
        })
        .collect();
    let mut effective_signals = signals.to_vec();
    apply_baseline_protection(&mut effective_signals, &records);
    let decisions = decide_bodies(&effective_signals).map_err(|_| PipelineError::Decisions)?;
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
            reason_chain: item.reason_chain,
        })
        .collect();
    let budgeted = budget_catalog(&level_a, &closed_decisions, &measurements, limits)
        .map_err(|_| PipelineError::Budget)?;
    let unavailable_sources = sources
        .iter()
        .filter(|source| !source.admitted)
        .map(|source| source.id.clone())
        .collect();
    let manifest = CatalogManifest::new(budgeted.level_a, budgeted.decisions, unavailable_sources)
        .map_err(|_| PipelineError::Manifest)?;
    Ok(ValidatedCatalog {
        manifest,
        body_paths,
    })
}
