use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyMeasurement {
    pub id: String,
    pub bytes: u64,
    pub token_upper_bound: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct CatalogLimits {
    pub index_bytes: u64,
    pub detail_bytes: u64,
    pub detail_tokens: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CatalogBudgetError {
    IndexExceeded,
    MissingMeasurement(String),
    ProtectedDetailBytesExceeded,
    ProtectedDetailTokensExceeded,
}
#[derive(Debug)]
pub struct CatalogOutput {
    pub level_a: Vec<LevelAEntry>,
    pub decisions: Vec<DecisionRecord>,
    pub index_bytes: u64,
    pub detail_bytes: u64,
    pub detail_tokens: u64,
}
pub fn budget_catalog(
    level_a: &[LevelAEntry],
    decisions: &[DecisionRecord],
    measurements: &[BodyMeasurement],
    limits: CatalogLimits,
) -> Result<CatalogOutput, CatalogBudgetError> {
    let index_bytes = serde_json::to_vec(level_a)
        .map_err(|_| CatalogBudgetError::IndexExceeded)?
        .len() as u64;
    if index_bytes > limits.index_bytes {
        return Err(CatalogBudgetError::IndexExceeded);
    }
    let by_id: BTreeMap<_, _> = measurements.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut detail_bytes = 0;
    let mut detail_tokens = 0;
    for decision in decisions
        .iter()
        .filter(|d| d.decision == BodyDecision::LoadNow)
    {
        let m = by_id
            .get(decision.id.as_str())
            .ok_or_else(|| CatalogBudgetError::MissingMeasurement(decision.id.clone()))?;
        detail_bytes += m.bytes;
        detail_tokens += m.token_upper_bound;
    }
    if detail_bytes > limits.detail_bytes {
        return Err(CatalogBudgetError::ProtectedDetailBytesExceeded);
    }
    if detail_tokens > limits.detail_tokens {
        return Err(CatalogBudgetError::ProtectedDetailTokensExceeded);
    }
    Ok(CatalogOutput {
        level_a: level_a.to_vec(),
        decisions: decisions.to_vec(),
        index_bytes,
        detail_bytes,
        detail_tokens,
    })
}
