use std::fs;
use taskseal::contracts::schema::{SchemaKind, validate};

#[test]
fn valid_vectors_pass_for_each_public_schema() {
    for (kind, path) in [
        (SchemaKind::L2, "fixtures/contracts/schema/valid/l2.json"),
        (
            SchemaKind::TaskPacketV2,
            "fixtures/contracts/schema/valid/task-packet-v2.json",
        ),
        (
            SchemaKind::Catalog,
            "fixtures/contracts/schema/valid/catalog.json",
        ),
        (
            SchemaKind::Manifest,
            "fixtures/contracts/schema/valid/manifest.json",
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(validate(kind, &bytes).is_ok(), "{path}");
    }
}

#[test]
fn invalid_vectors_refuse_required_unknown_enum_and_bounds() {
    for (name, path) in [
        (
            "missing-required",
            "fixtures/contracts/schema/invalid/missing-required.json",
        ),
        (
            "unknown-property",
            "fixtures/contracts/schema/invalid/unknown-property.json",
        ),
        (
            "invalid-enum",
            "fixtures/contracts/schema/invalid/invalid-enum.json",
        ),
        (
            "invalid-bounds",
            "fixtures/contracts/schema/invalid/invalid-bounds.json",
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(
            validate(SchemaKind::TaskPacketV2, &bytes).is_err(),
            "{name} accepted"
        );
    }
}

#[test]
fn errors_are_sorted_by_json_pointer_and_schema_ids_are_owned() {
    let bytes = fs::read("fixtures/contracts/schema/invalid/missing-required.json").unwrap();
    let error = validate(SchemaKind::TaskPacketV2, &bytes)
        .unwrap_err()
        .to_string();
    assert!(error.contains("/"), "{error}");
    assert!(!error.contains("taskseal.example"), "{error}");
}

#[test]
fn l2_requires_scope_graph_typed_sections_and_evidence() {
    let valid = serde_json::json!({
        "schema_version":"l2.v2",
        "scope_id":"repo:taskseal",
        "scope_kind":"repo",
        "parent_scope_ids":[],
        "sections":{"deny_union":["no ambient HOME"]},
        "provenance":[{"source_id":"project","source_kind":"project","source_ref":".clroom/scopes/root.json","sha256":"0".repeat(64)}],
        "approval":{"status":"approved","receipt_id":"approval-1"},
        "source_hashes":[{"source_id":"project","sha256":"0".repeat(64)}]
    });
    assert!(validate(SchemaKind::L2, &serde_json::to_vec(&valid).unwrap()).is_ok());

    for field in [
        "scope_id",
        "scope_kind",
        "parent_scope_ids",
        "sections",
        "provenance",
        "approval",
        "source_hashes",
    ] {
        let mut invalid = valid.clone();
        invalid.as_object_mut().unwrap().remove(field);
        assert!(
            validate(SchemaKind::L2, &serde_json::to_vec(&invalid).unwrap()).is_err(),
            "missing {field} accepted"
        );
    }
    let mut recursive = valid.clone();
    recursive["sections"]["generic_deep_merge"] = serde_json::json!({"x":{"y":true}});
    assert!(validate(SchemaKind::L2, &serde_json::to_vec(&recursive).unwrap()).is_err());
}

#[test]
fn truncated_l2_v1_and_invalid_merge_operations_refuse() {
    let legacy = serde_json::json!({"schema_version":"l2.v1","id":"legacy","content":"x","digest":"0".repeat(64)});
    assert!(validate(SchemaKind::L2, &serde_json::to_vec(&legacy).unwrap()).is_err());

    let base = serde_json::json!({
        "schema_version":"l2.v2","scope_id":"repo:a","scope_kind":"repo","parent_scope_ids":[],
        "sections":{"replace":{"command":"cargo test"}},
        "provenance":[{"source_id":"p","source_kind":"project","source_ref":"scope.json","sha256":"0".repeat(64)}],
        "approval":{"status":"approved","receipt_id":null},
        "source_hashes":[{"source_id":"p","sha256":"0".repeat(64)}]
    });
    for poison in [
        serde_json::json!({"replace":[]}),
        serde_json::json!({"append_unique":"x"}),
        serde_json::json!({"deny_union":{"x":true}}),
        serde_json::json!({"ordered_chain":{}}),
        serde_json::json!({"no_inherit":[]}),
    ] {
        let mut invalid = base.clone();
        invalid["sections"] = poison;
        assert!(
            validate(SchemaKind::L2, &serde_json::to_vec(&invalid).unwrap()).is_err(),
            "{invalid}"
        );
    }
}
