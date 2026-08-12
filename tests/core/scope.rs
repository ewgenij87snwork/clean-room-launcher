use crate::core::decode::{DecodedDocument, DecodedInputs};
use serde_json::json;

fn inputs(scopes: &[(&str, &[&str])]) -> DecodedInputs {
    DecodedInputs { documents: scopes.iter().map(|(id, parents)| DecodedDocument::Json {
        logical_path: format!("{id}.l2.json"),
        value: json!({"schema_version":"l2.v2","scope_id":id,"scope_kind":"repo","parent_scope_ids":parents,"sections":{"replace":{"x":"y"}},"provenance":[],"approval":{"status":"approved","receipt_id":null},"source_hashes":[]}),
    }).collect() }
}

#[test]
fn resolves_only_schema_declared_reachable_scopes_parent_first() {
    let graph = inputs(&[("sibling", &[]), ("child", &["root"]), ("root", &[])]);
    let resolved = super::resolve_scopes(&graph, &super::Target::new(["child"])).unwrap();
    assert_eq!(resolved.ids, ["root", "child"]);
}

#[test]
fn supports_multiple_targets_and_multi_parent_dag_without_siblings() {
    let graph = inputs(&[
        ("unused", &[]),
        ("root-a", &[]),
        ("root-b", &[]),
        ("child", &["root-b", "root-a"]),
    ]);
    let resolved = super::resolve_scopes(&graph, &super::Target::new(["child"])).unwrap();
    assert_eq!(resolved.ids, ["root-b", "root-a", "child"]);
}

#[test]
fn refuses_cycles_missing_parents_and_duplicate_scope_ids() {
    for (name, graph, code) in [
        (
            "cycle",
            inputs(&[("a", &["b"]), ("b", &["a"])]),
            "SCOPE_CYCLE",
        ),
        ("missing", inputs(&[("a", &["missing"])]), "MISSING_SCOPE"),
        (
            "duplicate",
            inputs(&[("a", &[]), ("a", &[])]),
            "DUPLICATE_SCOPE",
        ),
    ] {
        let error = super::resolve_scopes(&graph, &super::Target::new(["a"])).unwrap_err();
        assert!(error.to_string().starts_with(code), "{name}: {error}");
    }
}
