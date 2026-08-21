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
pub struct BudgetRefusal {
    pub dimension: &'static str,
    pub observed: u64,
    pub limit: u64,
    pub preserved_names: Vec<String>,
    pub search_path: &'static str,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CatalogBudgetError {
    Exceeded(BudgetRefusal),
    MissingMeasurement(String),
    CounterOverflow,
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
        .map_err(|_| CatalogBudgetError::CounterOverflow)?
        .len() as u64;
    if index_bytes > limits.index_bytes {
        return Err(refusal(
            "index_bytes",
            index_bytes,
            limits.index_bytes,
            level_a,
            "catalog/index",
        ));
    }
    let by_id: BTreeMap<_, _> = measurements.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut detail_bytes: u64 = 0;
    let mut detail_tokens: u64 = 0;
    for decision in decisions
        .iter()
        .filter(|d| d.decision == BodyDecision::LoadNow)
    {
        let m = by_id
            .get(decision.id.as_str())
            .ok_or_else(|| CatalogBudgetError::MissingMeasurement(decision.id.clone()))?;
        detail_bytes = detail_bytes
            .checked_add(m.bytes)
            .ok_or(CatalogBudgetError::CounterOverflow)?;
        detail_tokens = detail_tokens
            .checked_add(m.token_upper_bound)
            .ok_or(CatalogBudgetError::CounterOverflow)?;
    }
    if detail_bytes > limits.detail_bytes {
        return Err(refusal(
            "detail_bytes",
            detail_bytes,
            limits.detail_bytes,
            level_a,
            "catalog/detail",
        ));
    }
    if detail_tokens > limits.detail_tokens {
        return Err(refusal(
            "detail_tokens",
            detail_tokens,
            limits.detail_tokens,
            level_a,
            "catalog/detail",
        ));
    }
    Ok(CatalogOutput {
        level_a: level_a.to_vec(),
        decisions: decisions.to_vec(),
        index_bytes,
        detail_bytes,
        detail_tokens,
    })
}

fn refusal(
    dimension: &'static str,
    observed: u64,
    limit: u64,
    level_a: &[LevelAEntry],
    search_path: &'static str,
) -> CatalogBudgetError {
    CatalogBudgetError::Exceeded(BudgetRefusal {
        dimension,
        observed,
        limit,
        preserved_names: level_a.iter().map(|entry| entry.name.clone()).collect(),
        search_path,
    })
}
