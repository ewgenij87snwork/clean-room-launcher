use super::inventory::{InventoryError, inventory_skills};
use super::sources::{SkillSourceAuthority, SkillSourceConfig, enumerate_sources};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/catalog/inventory")
        .join(name)
}

fn admitted(name: &str) -> Vec<super::sources::SkillSource> {
    let root = fixture(name);
    enumerate_sources(
        &SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]),
        &[root],
    )
    .unwrap()
}

#[test]
fn inventories_sorted_metadata_and_body_digests_without_body_text() {
    let records = inventory_skills(&admitted("stable-multi-source")).unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].name, "alpha");
    assert_eq!(records[1].name, "zeta");
    assert!(
        records
            .iter()
            .all(|record| record.body_path.ends_with("SKILL.md"))
    );
    assert!(records.iter().all(|record| record.body_digest.len() == 64));
    assert!(
        records
            .iter()
            .all(|record| record.metadata_digest.len() == 64)
    );
    let debug = format!("{records:?}");
    assert!(!debug.contains("SECRET_BODY_CANARY"));
}

#[test]
fn refuses_duplicate_names_with_stable_reason() {
    let error = inventory_skills(&admitted("duplicate-names")).unwrap_err();
    assert_eq!(error, InventoryError::DuplicateName("same".to_owned()));
}

#[test]
fn refuses_malformed_metadata_and_missing_body() {
    assert_eq!(
        inventory_skills(&admitted("malformed-metadata")).unwrap_err(),
        InventoryError::MalformedMetadata
    );
    assert_eq!(
        inventory_skills(&admitted("missing-body")).unwrap_err(),
        InventoryError::MissingBody
    );
}
