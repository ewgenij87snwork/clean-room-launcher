use super::level_b::{BodyDecision, DecisionRecord};
use super::manifest::{CatalogManifest, ManifestError};
#[test]
fn census_is_closed_and_explain_matches_inspect() {
    let m = CatalogManifest::new(
        2,
        vec![
            DecisionRecord {
                id: "a".into(),
                decision: BodyDecision::LoadNow,
                reason: "PROTECTED",
            },
            DecisionRecord {
                id: "b".into(),
                decision: BodyDecision::LoadOnInvoke,
                reason: "DEFERRED_NATIVE",
            },
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(
        m.explain("a").unwrap().reason,
        m.inspect("a").unwrap().reason
    );
    assert_eq!(m.accounted(), 2)
}
#[test]
fn wrong_denominator_refuses() {
    assert_eq!(
        CatalogManifest::new(2, vec![], vec![]).unwrap_err(),
        ManifestError::CensusMismatch
    )
}
