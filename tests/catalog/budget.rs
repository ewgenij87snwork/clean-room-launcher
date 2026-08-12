use super::budget::{BodyMeasurement, CatalogBudgetError, CatalogLimits, budget_catalog};
use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
fn entry() -> LevelAEntry {
    LevelAEntry {
        id: "a".into(),
        name: "a".into(),
        capability: "c".into(),
        trigger_summary: "s".into(),
        source_id: "src".into(),
        body_digest: "a".repeat(64),
    }
}
fn decision(kind: BodyDecision) -> DecisionRecord {
    DecisionRecord {
        id: "a".into(),
        decision: kind,
        reason: "R",
    }
}
#[test]
fn exact_independent_byte_and_token_limits_pass() {
    let a = vec![entry()];
    let d = vec![decision(BodyDecision::LoadNow)];
    let index = serde_json::to_vec(&a).unwrap().len() as u64;
    assert!(
        budget_catalog(
            &a,
            &d,
            &[BodyMeasurement {
                id: "a".into(),
                bytes: 10,
                token_upper_bound: 8
            }],
            CatalogLimits {
                index_bytes: index,
                detail_bytes: 10,
                detail_tokens: 8
            }
        )
        .is_ok()
    )
}
#[test]
fn huge_protected_body_refuses_even_when_record_count_is_one() {
    let a = vec![entry()];
    let d = vec![decision(BodyDecision::LoadNow)];
    assert_eq!(
        budget_catalog(
            &a,
            &d,
            &[BodyMeasurement {
                id: "a".into(),
                bytes: 50_000_000,
                token_upper_bound: 50_000_000
            }],
            CatalogLimits {
                index_bytes: 999,
                detail_bytes: 1024,
                detail_tokens: 1024
            }
        )
        .unwrap_err(),
        CatalogBudgetError::ProtectedDetailBytesExceeded
    )
}

#[test]
fn aggregate_overflow_refuses_deterministically() {
    let mut second = entry();
    second.id = "b".into();
    let mut second_decision = decision(BodyDecision::LoadNow);
    second_decision.id = "b".into();
    assert_eq!(
        budget_catalog(
            &[entry(), second],
            &[decision(BodyDecision::LoadNow), second_decision],
            &[
                BodyMeasurement {
                    id: "a".into(),
                    bytes: u64::MAX,
                    token_upper_bound: 1
                },
                BodyMeasurement {
                    id: "b".into(),
                    bytes: 1,
                    token_upper_bound: 1
                },
            ],
            CatalogLimits {
                index_bytes: u64::MAX,
                detail_bytes: u64::MAX,
                detail_tokens: u64::MAX
            },
        )
        .unwrap_err(),
        CatalogBudgetError::CounterOverflow
    );
}
