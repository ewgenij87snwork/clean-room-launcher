use super::inventory::SkillRecord;
use super::level_a::{LevelAError, build_level_a};

fn record(name: &str, summary: &str) -> SkillRecord {
    SkillRecord {
        id: format!("skill-{name}"),
        name: name.to_owned(),
        capability: format!("cap-{name}"),
        trigger_summary: summary.to_owned(),
        source_id: "skill-source-test".to_owned(),
        body_path: format!("skill-source-test/{name}/SKILL.md"),
        body_digest: "a".repeat(64),
        body_bytes: 1,
        metadata_digest: "b".repeat(64),
        native_frontmatter: true,
    }
}

#[test]
fn emits_every_name_in_stable_order_without_body_path_or_text() {
    let entries = build_level_a(&[
        record("zeta", "Use for zeta work"),
        record("alpha", "Use for alpha work"),
    ])
    .unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "alpha");
    assert_eq!(entries[1].name, "zeta");
    assert_eq!(entries[0].body_digest, "a".repeat(64));
    let debug = format!("{entries:?}");
    assert!(!debug.contains("SKILL.md"));
    assert!(!debug.contains("SECRET_BODY_CANARY"));
}

#[test]
fn refuses_missing_overlong_and_unsafe_static_summaries() {
    assert_eq!(
        build_level_a(&[record("empty", "")]).unwrap_err(),
        LevelAError::MissingSummary
    );
    assert_eq!(
        build_level_a(&[record("long", &"x".repeat(1025))]).unwrap_err(),
        LevelAError::SummaryTooLong
    );
    assert_eq!(
        build_level_a(&[record("unsafe", "line one\nline two")]).unwrap_err(),
        LevelAError::UnsafeSummary
    );
}

#[test]
fn snapshot_size_is_compact_and_deterministic() {
    let entries = build_level_a(&[record("alpha", "Use for alpha work")]).unwrap();
    let bytes = serde_json::to_vec(&entries).unwrap();

    assert!(bytes.len() <= 256);
    assert_eq!(bytes, serde_json::to_vec(&entries).unwrap());
}
