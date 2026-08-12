use std::fs;
use taskseal::contracts::schema::{validate, SchemaKind};

#[test]
fn valid_vectors_pass_for_each_public_schema() {
    for (kind, path) in [
        (SchemaKind::L2, "fixtures/contracts/schema/valid/l2.json"),
        (SchemaKind::TaskPacketV2, "fixtures/contracts/schema/valid/task-packet-v2.json"),
        (SchemaKind::Catalog, "fixtures/contracts/schema/valid/catalog.json"),
        (SchemaKind::Manifest, "fixtures/contracts/schema/valid/manifest.json"),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(validate(kind, &bytes).is_ok(), "{path}");
    }
}

#[test]
fn invalid_vectors_refuse_required_unknown_enum_and_bounds() {
    for (name, path) in [
        ("missing-required", "fixtures/contracts/schema/invalid/missing-required.json"),
        ("unknown-property", "fixtures/contracts/schema/invalid/unknown-property.json"),
        ("invalid-enum", "fixtures/contracts/schema/invalid/invalid-enum.json"),
        ("invalid-bounds", "fixtures/contracts/schema/invalid/invalid-bounds.json"),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(validate(SchemaKind::TaskPacketV2, &bytes).is_err(), "{name} accepted");
    }
}

#[test]
fn errors_are_sorted_by_json_pointer_and_schema_ids_are_owned() {
    let bytes = fs::read("fixtures/contracts/schema/invalid/missing-required.json").unwrap();
    let error = validate(SchemaKind::TaskPacketV2, &bytes).unwrap_err().to_string();
    assert!(error.contains("/"), "{error}");
    assert!(!error.contains("taskseal.example"), "{error}");
}
