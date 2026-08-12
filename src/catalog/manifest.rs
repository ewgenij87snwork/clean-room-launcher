use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonView {
    pub id: String,
    pub reason: &'static str,
    pub body_digest: String,
    pub source_id: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    CensusMismatch,
    DuplicateId,
    UnknownId,
}
#[derive(Debug, Clone, Default)]
pub struct CensusCounts {
    pub loaded_now: u64,
    pub load_on_invoke: u64,
    pub unavailable: u64,
    pub refused: u64,
    pub total: u64,
}
#[derive(Debug, Clone)]
pub struct CatalogManifest {
    pub level_a: Vec<LevelAEntry>,
    pub decisions: Vec<DecisionRecord>,
    pub unavailable_sources: Vec<String>,
    pub counts: CensusCounts,
}
impl CatalogManifest {
    pub fn new(
        level_a: Vec<LevelAEntry>,
        decisions: Vec<DecisionRecord>,
        unavailable_sources: Vec<String>,
    ) -> Result<Self, ManifestError> {
        let ids: BTreeSet<_> = level_a.iter().map(|e| e.id.as_str()).collect();
        if ids.len() != level_a.len() {
            return Err(ManifestError::DuplicateId);
        }
        let decision_ids: BTreeSet<_> = decisions.iter().map(|d| d.id.as_str()).collect();
        if decision_ids.len() != decisions.len() {
            return Err(ManifestError::DuplicateId);
        }
        if ids != decision_ids {
            return Err(ManifestError::CensusMismatch);
        }
        let mut counts = CensusCounts::default();
        for d in &decisions {
            match d.decision {
                BodyDecision::LoadNow => counts.loaded_now += 1,
                BodyDecision::LoadOnInvoke => counts.load_on_invoke += 1,
                BodyDecision::Unavailable => counts.unavailable += 1,
                BodyDecision::Refuse => counts.refused += 1,
            }
        }
        counts.unavailable += unavailable_sources.len() as u64;
        counts.total =
            counts.loaded_now + counts.load_on_invoke + counts.unavailable + counts.refused;
        Ok(Self {
            level_a,
            decisions,
            unavailable_sources,
            counts,
        })
    }
    pub fn explain(&self, id: &str) -> Result<ReasonView, ManifestError> {
        self.view(id)
    }
    pub fn inspect(&self, id: &str) -> Result<ReasonView, ManifestError> {
        self.view(id)
    }
    fn view(&self, id: &str) -> Result<ReasonView, ManifestError> {
        let entries: BTreeMap<_, _> = self.level_a.iter().map(|e| (e.id.as_str(), e)).collect();
        let e = entries.get(id).ok_or(ManifestError::UnknownId)?;
        let d = self
            .decisions
            .iter()
            .find(|d| d.id == id)
            .ok_or(ManifestError::UnknownId)?;
        Ok(ReasonView {
            id: id.into(),
            reason: d.reason,
            body_digest: e.body_digest.clone(),
            source_id: e.source_id.clone(),
        })
    }
}
