use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
#[derive(Debug, Clone, Copy)]
pub struct CatalogLimits {
    pub index_bytes: u64,
    pub detail_records: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CatalogBudgetError {
    IndexExceeded,
    ProtectedDetailExceeded,
}
#[derive(Debug)]
pub struct CatalogOutput {
    pub level_a: Vec<LevelAEntry>,
    pub decisions: Vec<DecisionRecord>,
    pub index_bytes: u64,
    pub detail_records: u64,
}
pub fn budget_catalog(
    level_a: &[LevelAEntry],
    decisions: &[DecisionRecord],
    limits: CatalogLimits,
) -> Result<CatalogOutput, CatalogBudgetError> {
    let index_bytes = serde_json::to_vec(level_a).expect("serializable").len() as u64;
    if index_bytes > limits.index_bytes {
        return Err(CatalogBudgetError::IndexExceeded);
    }
    let detail_records = decisions
        .iter()
        .filter(|d| d.decision == BodyDecision::LoadNow)
        .count() as u64;
    if detail_records > limits.detail_records {
        return Err(CatalogBudgetError::ProtectedDetailExceeded);
    }
    Ok(CatalogOutput {
        level_a: level_a.to_vec(),
        decisions: decisions.to_vec(),
        index_bytes,
        detail_records,
    })
}
