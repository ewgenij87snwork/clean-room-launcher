use crate::core::decode::{DecodedDocument, DecodedInputs};
use crate::core::scope::{Target, resolve_scopes};
use serde_json::{Value, json};

fn l2(id: &str, parents: &[&str], sections: Value) -> DecodedDocument {
    DecodedDocument::Json {
        logical_path: format!("{id}.l2.json"),
        value: json!({
            "schema_version":"l2.v2","scope_id":id,"scope_kind":"repo","parent_scope_ids":parents,
            "sections":sections,"provenance":[],"approval":{"status":"approved","receipt_id":null},"source_hashes":[]
        }),
    }
}

#[test]
fn applies_six_typed_operations_with_child_override_and_deny_union() {
    let inputs = DecodedInputs {
        documents: vec![
            l2(
                "root",
                &[],
                json!({"replace":{"command":"cargo test"},"append_unique":["rust"],"deny_union":["rm"],"map_override":{"test":"cargo test"},"ordered_chain":["fmt"],"no_inherit":{"socket":"root"}}),
            ),
            l2(
                "child",
                &["root"],
                json!({"replace":{"command":"cargo nextest"},"append_unique":["rust","security"],"deny_union":["network"],"map_override":{"test":"cargo nextest"},"ordered_chain":["clippy"],"no_inherit":{"socket":"child"}}),
            ),
        ],
    };
    let resolved = resolve_scopes(&inputs, &Target::new(["child"])).unwrap();
    let merged = super::merge(&[resolved]).unwrap();
    assert_eq!(merged.replace["command"].value, "cargo nextest");
    assert_eq!(merged.replace["command"].source_scope, "child");
    assert_eq!(
        merged
            .append_unique
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["rust", "security"]
    );
    assert_eq!(
        merged.denies.iter().map(String::as_str).collect::<Vec<_>>(),
        ["network", "rm"]
    );
    assert_eq!(merged.map_override["test"].value, "cargo nextest");
    assert_eq!(merged.ordered_chain, ["fmt", "clippy"]);
    assert_eq!(merged.no_inherit["socket"].value, "child");
}

#[test]
fn conflicting_parent_replacements_and_unknown_sections_refuse() {
    let inputs = DecodedInputs {
        documents: vec![
            l2("a", &[], json!({"replace":{"command":"a"}})),
            l2("b", &[], json!({"replace":{"command":"b"}})),
            l2("child", &["a", "b"], json!({"deny_union":["safe"]})),
        ],
    };
    let resolved = resolve_scopes(&inputs, &Target::new(["child"])).unwrap();
    assert!(
        super::merge(&[resolved])
            .unwrap_err()
            .to_string()
            .starts_with("MERGE_COLLISION")
    );
}

#[test]
fn protected_denies_cannot_be_weakened() {
    let mut merged = super::MergedContext::default();
    merged.denies.insert("network".into());
    assert!(crate::core::policy::enforce_denies(&merged, &["network"]).is_ok());
    assert!(
        crate::core::policy::enforce_denies(&merged, &[])
            .unwrap_err()
            .to_string()
            .starts_with("DENY_WEAKENED")
    );
}
