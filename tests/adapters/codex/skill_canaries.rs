#[test]
fn native_skill_fixture_inventory_is_present_and_body_markers_are_not_startup_metadata() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/adapters/codex/skill-canaries");
    for name in ["needed", "unused", "protected", "outside-boundary", "tampered"] {
        let body = std::fs::read_to_string(root.join(name).join("SKILL.md")).unwrap();
        assert!(body.contains("TASKSEAL_SKILL_BODY_"));
        assert!(body.contains("name:"));
    }
    let startup_metadata = std::fs::read_to_string(root.join("startup-metadata.json")).unwrap();
    assert!(startup_metadata.contains("needed"));
    assert!(startup_metadata.contains("unused"));
    assert!(!startup_metadata.contains("TASKSEAL_SKILL_BODY_"));
}

#[test]
fn taskseal_projection_path_is_metadata_only_and_accounts_for_every_admitted_name() {
    let projection = include_str!("../../../src/catalog/projection.rs");
    assert!(!projection.contains("read_to_string"));
    assert!(!projection.contains("TASKSEAL_SKILL_BODY_"));

    let metadata = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/adapters/codex/skill-canaries/startup-metadata.json"),
    )
    .unwrap();
    for name in [
        "taskseal-needed",
        "taskseal-unused",
        "taskseal-protected",
        "taskseal-outside-boundary",
        "taskseal-tampered",
    ] {
        assert!(metadata.contains(name), "unaccounted admitted name: {name}");
    }
}

#[test]
fn tampered_fixture_digest_is_not_admitted_as_its_recorded_digest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/adapters/codex/skill-canaries");
    let actual = taskseal::core::inventory::sha256_hex(
        &std::fs::read(root.join("tampered/SKILL.md")).unwrap(),
    );
    assert_ne!(actual, "0000000000000000000000000000000000000000000000000000000000000000");
}
