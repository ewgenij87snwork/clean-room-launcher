use super::level_b::{BodyDecision, DecisionError, SkillSignals, decide_bodies};

fn signals(id: &str) -> SkillSignals {
    SkillSignals::new(id)
}

#[test]
fn explicit_safety_and_start_signals_load_now() {
    let decisions = decide_bodies(&[
        signals("protected").protected(),
        signals("required").task_required(),
        signals("pinned").pinned(),
        signals("project").project_required(),
    ])
    .unwrap();
    assert!(
        decisions
            .iter()
            .all(|record| record.decision == BodyDecision::LoadNow)
    );
    assert_eq!(
        decisions
            .iter()
            .find(|r| r.id == "protected")
            .unwrap()
            .reason,
        "PROTECTED"
    );
    assert_eq!(
        decisions.iter().find(|r| r.id == "project").unwrap().reason,
        "PROJECT_REQUIRED"
    );
}

#[test]
fn ordinary_skills_defer_but_unknown_stale_and_changed_refuse() {
    let decisions = decide_bodies(&[
        signals("ordinary"),
        signals("unknown").unknown(),
        signals("stale").stale(),
        signals("changed").changed(),
    ])
    .unwrap();
    assert_eq!(
        decisions
            .iter()
            .find(|r| r.id == "ordinary")
            .unwrap()
            .decision,
        BodyDecision::LoadOnInvoke
    );
    assert!(
        decisions
            .iter()
            .filter(|r| r.id != "ordinary")
            .all(|r| r.decision == BodyDecision::Refuse)
    );
}

#[test]
fn relevance_filter_is_not_authorized() {
    assert_eq!(
        decide_bodies(&[signals("cut").relevance_excluded()]).unwrap_err(),
        DecisionError::RelevanceFilterNotAuthorized
    );
}
