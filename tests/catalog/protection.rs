use super::inventory::SkillRecord;
use super::level_b::SkillSignals;
use super::protection::{PROTECTED_PROCESS_CATEGORIES, apply_baseline_protection};

fn record(id: &str, capability: &str) -> SkillRecord {
    SkillRecord {
        id: id.into(),
        name: id.into(),
        capability: capability.into(),
        trigger_summary: capability.into(),
        source_id: "fixture".into(),
        body_path: format!("fixture/{id}/SKILL.md"),
        body_digest: "a".repeat(64),
        body_bytes: 1,
        metadata_digest: "b".repeat(64),
        native_frontmatter: true,
    }
}

#[test]
fn baseline_categories_are_protected_before_decisioning() {
    assert_eq!(
        PROTECTED_PROCESS_CATEGORIES,
        [
            "verification",
            "debugging",
            "security-privacy",
            "planning",
            "evidence",
            "recovery",
        ]
    );
    let records = vec![
        record("verify", "verification"),
        record("optional", "writing"),
    ];
    let mut signals = vec![SkillSignals::new("verify"), SkillSignals::new("optional")];
    apply_baseline_protection(&mut signals, &records);
    assert!(signals[0].signals.protected);
    assert!(!signals[1].signals.protected);
}
