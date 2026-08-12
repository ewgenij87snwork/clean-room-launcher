use super::budget::{CatalogBudgetError, CatalogLimits, budget_catalog};
use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
fn entry(id: &str) -> LevelAEntry {
    LevelAEntry {
        id: id.into(),
        name: id.into(),
        capability: "c".into(),
        trigger_summary: "s".into(),
        source_id: "src".into(),
        body_digest: "a".repeat(64),
    }
}
fn decision(id: &str, kind: BodyDecision) -> DecisionRecord {
    DecisionRecord {
        id: id.into(),
        decision: kind,
        reason: "R",
    }
}
#[test]
fn exact_limits_pass_and_dimensions_are_independent() {
    let a = vec![entry("a")];
    let d = vec![decision("a", BodyDecision::LoadOnInvoke)];
    let measured = serde_json::to_vec(&a).unwrap().len() as u64;
    let out = budget_catalog(
        &a,
        &d,
        CatalogLimits {
            index_bytes: measured,
            detail_records: 0,
        },
    )
    .unwrap();
    assert_eq!(out.level_a.len(), 1);
}
#[test]
fn index_and_protected_detail_overflow_refuse_without_truncation() {
    let a = vec![entry("a")];
    let d = vec![decision("a", BodyDecision::LoadNow)];
    assert_eq!(
        budget_catalog(
            &a,
            &d,
            CatalogLimits {
                index_bytes: 1,
                detail_records: 1
            }
        )
        .unwrap_err(),
        CatalogBudgetError::IndexExceeded
    );
    assert_eq!(
        budget_catalog(
            &a,
            &d,
            CatalogLimits {
                index_bytes: 999,
                detail_records: 0
            }
        )
        .unwrap_err(),
        CatalogBudgetError::ProtectedDetailExceeded
    );
}
