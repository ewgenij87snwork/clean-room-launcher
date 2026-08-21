use super::{ContractSet, decode};
use crate::core::inventory::{self, AdmittedRoot, SourceRecord};
use std::fs;
use std::path::PathBuf;

fn records(name: &str, files: &[(&str, &[u8])]) -> Vec<SourceRecord> {
    let root =
        std::env::temp_dir().join(format!("taskseal-p03-decode-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for (path, bytes) in files {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    inventory::inventory(&[AdmittedRoot::new(root, "contracts")]).unwrap()
}

#[test]
fn decode_accepts_schema_valid_json_and_adapter_toml() {
    let valid = include_bytes!("../../fixtures/contracts/schema/valid/l2.json");
    let adapter = include_bytes!("../../adapters/declarations/codex.toml");
    let decoded = decode(
        &records(
            "valid",
            &[("scope.l2.json", valid), ("codex.adapter.toml", adapter)],
        ),
        &ContractSet::standard(),
    )
    .unwrap();
    assert_eq!(decoded.documents.len(), 2);
}

#[test]
fn decode_refuses_malformed_utf8_json_toml_and_unknown_fields() {
    for (name, path, bytes, code) in [
        ("utf8", "scope.l2.json", &[0xff][..], "INVALID_UTF8"),
        ("json", "scope.l2.json", b"{".as_slice(), "JSON_PARSE"),
        (
            "toml",
            "codex.adapter.toml",
            b"provider_id".as_slice(),
            "TOML_SYNTAX",
        ),
        (
            "unknown",
            "packet.task-packet.json",
            br#"{"schema_version":"taskseal.task-packet.v2","unknown":true}"#.as_slice(),
            "SCHEMA_INVALID",
        ),
    ] {
        let error = decode(&records(name, &[(path, bytes)]), &ContractSet::standard()).unwrap_err();
        assert!(error.to_string().starts_with(code), "{name}: {error}");
    }
}

#[test]
fn decode_refuses_unknown_versions_duplicate_ids_and_oversize_records() {
    let unknown_version = br#"{"schema_version":"taskseal.l2.v999"}"#;
    let error = decode(
        &records("version", &[("scope.l2.json", unknown_version)]),
        &ContractSet::standard(),
    )
    .unwrap_err();
    assert!(error.to_string().starts_with("SCHEMA_INVALID"));

    let duplicate = include_bytes!("../../fixtures/contracts/schema/valid/l2.json");
    let error = decode(
        &records(
            "duplicates",
            &[("a.l2.json", duplicate), ("b.l2.json", duplicate)],
        ),
        &ContractSet::standard(),
    )
    .unwrap_err();
    assert!(error.to_string().starts_with("DUPLICATE_ID"));

    let error = decode(
        &records("oversize", &[("scope.l2.json", &vec![b' '; 1_048_577])]),
        &ContractSet::standard(),
    )
    .unwrap_err();
    assert!(error.to_string().starts_with("INPUT_TOO_LARGE"));
}

#[allow(dead_code)]
fn _path_type(_: PathBuf) {}
