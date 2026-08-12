use super::inventory::{InventoryError, inventory_skills, inventory_skills_with_mutation};
use super::sources::{SkillSourceAuthority, SkillSourceConfig, enumerate_sources};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/catalog/inventory")
        .join(name)
}

#[test]
fn changing_body_during_read_refuses_with_stable_reason() {
    let root = std::env::temp_dir().join(format!("taskseal-changing-{}", std::process::id()));
    let skill = root.join("alpha");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("skill.json"),
        br#"{"name":"alpha","capability":"c","trigger_summary":"s"}"#,
    )
    .unwrap();
    std::fs::write(skill.join("SKILL.md"), b"stable original body").unwrap();
    let sources = enumerate_sources(
        &SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]),
        std::slice::from_ref(&root),
    )
    .unwrap();
    let target = root.join("alpha/SKILL.md");
    let mut changed = false;
    let error = inventory_skills_with_mutation(&sources[0], &mut |path| {
        if !changed && path.ends_with("alpha/SKILL.md") {
            std::fs::write(&target, b"mutate original body").unwrap();
            changed = true
        }
    })
    .unwrap_err();
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(error, InventoryError::ChangedDuringRead)
}

#[cfg(unix)]
#[test]
fn unreadable_body_refuses_instead_of_disappearing() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = std::env::temp_dir().join(format!("taskseal-unreadable-{}", std::process::id()));
    let skill = scratch.join("skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("skill.json"),
        br#"{"name":"x","capability":"x","trigger_summary":"x"}"#,
    )
    .unwrap();
    let body = skill.join("SKILL.md");
    std::fs::write(
        &body,
        b"---\nname: x\ndescription: unreadable canary\n---\nsecret",
    )
    .unwrap();
    std::fs::set_permissions(&body, std::fs::Permissions::from_mode(0o000)).unwrap();
    let config = SkillSourceConfig::new(vec![(scratch.clone(), SkillSourceAuthority::Project)]);
    let sources = enumerate_sources(&config, std::slice::from_ref(&scratch)).unwrap();
    let result = inventory_skills(&sources);
    std::fs::set_permissions(&body, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::remove_dir_all(scratch).unwrap();
    assert!(
        matches!(result, Err(InventoryError::SourceRefused)),
        "unexpected unreadable result: {result:?}"
    )
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
fn inventories_provider_native_skill_md_frontmatter_without_sidecar() {
    let records = inventory_skills(&admitted("native-frontmatter")).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "native-example");
    assert_eq!(records[0].capability, "Use for native provider discovery.");
    assert_eq!(records[0].trigger_summary, records[0].capability);
    assert_eq!(records[0].body_digest.len(), 64);
    assert_eq!(records[0].metadata_digest.len(), 64);
}

#[test]
fn native_frontmatter_handles_quotes_blocks_bom_crlf_and_enforces_directory_name() {
    let root = std::env::temp_dir().join(format!("taskseal-frontmatter-{}", std::process::id()));
    let skill = root.join("quoted-skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "\u{feff}---\r\nname: \"quoted-skill\"\r\ndescription: >-\r\n  first line\r\n  second line\r\nallowed-tools: [Read]\r\n---\r\nbody",
    )
    .unwrap();
    let config = SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]);
    let sources = enumerate_sources(&config, std::slice::from_ref(&root)).unwrap();
    let records = inventory_skills(&sources).unwrap();
    assert_eq!(records[0].trigger_summary, "first line second line");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: wrong-name\ndescription: mismatch\n---\nbody",
    )
    .unwrap();
    assert_eq!(
        inventory_skills(&sources).unwrap_err(),
        InventoryError::MalformedMetadata
    );
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: quoted-skill\nname: duplicate\ndescription: bad\n---\nbody",
    )
    .unwrap();
    assert_eq!(
        inventory_skills(&sources).unwrap_err(),
        InventoryError::MalformedMetadata
    );
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: quoted-skill\ndescription: 42\n---\nbody",
    )
    .unwrap();
    assert_eq!(
        inventory_skills(&sources).unwrap_err(),
        InventoryError::MalformedMetadata
    );
    std::fs::remove_dir_all(root).unwrap();
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
