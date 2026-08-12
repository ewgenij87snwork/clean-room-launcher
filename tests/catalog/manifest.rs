use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
use super::manifest::{CatalogManifest, ManifestError};
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
#[test]
fn census_categories_and_evidence_are_closed() {
    let m = CatalogManifest::new(
        vec![entry("a"), entry("b")],
        vec![
            DecisionRecord {
                id: "a".into(),
                decision: BodyDecision::LoadNow,
                reason: "PROTECTED",
                reason_chain: vec!["a".into()],
            },
            DecisionRecord {
                id: "b".into(),
                decision: BodyDecision::LoadOnInvoke,
                reason: "DEFERRED",
                reason_chain: vec!["b".into()],
            },
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(m.counts.loaded_now, 1);
    assert_eq!(m.counts.load_on_invoke, 1);
    assert_eq!(m.counts.total, 2);
    let view = m.inspect("a").unwrap();
    assert_eq!(view.reason, "PROTECTED");
    assert_eq!(view.body_digest, "a".repeat(64));
    assert_eq!(view.source_id, "src")
}
#[test]
fn missing_or_duplicate_decision_refuses() {
    assert_eq!(
        CatalogManifest::new(vec![entry("a")], vec![], vec![]).unwrap_err(),
        ManifestError::CensusMismatch
    )
}

#[test]
fn unavailable_sources_do_not_corrupt_skill_denominator() {
    let m = CatalogManifest::new(
        vec![entry("a")],
        vec![DecisionRecord {
            id: "a".into(),
            decision: BodyDecision::LoadOnInvoke,
            reason: "D",
            reason_chain: vec!["a".into()],
        }],
        vec!["outside".into()],
    )
    .unwrap();
    assert_eq!(m.counts.total, 1);
    assert_eq!(m.counts.unavailable, 0);
    assert_eq!(m.counts.unavailable_source_count, 1);
}
