use crate::core::decode::{DecodedDocument, DecodedInputs};
use serde_json::json;

fn inputs(ids: &[&str]) -> DecodedInputs {
    DecodedInputs {
        documents: ids
            .iter()
            .map(|id| DecodedDocument::Json {
                logical_path: format!("{id}.l2.json"),
                value: json!({"schema_version":"l2.v1","id":id,"content":"x","digest":"0".repeat(64)}),
            })
            .collect(),
    }
}

#[test]
fn resolves_only_explicit_reachable_scopes_in_parent_first_order() {
    let target = super::Target::new(["child"], [("child", "root")]);
    let resolved = super::resolve_scopes(&inputs(&["sibling", "child", "root"]), &target).unwrap();
    assert_eq!(resolved.ids, ["root", "child"]);
}

#[test]
fn supports_multiple_targets_without_implicit_siblings() {
    let target = super::Target::new(["b", "a"], Vec::<(&str, &str)>::new());
    let resolved = super::resolve_scopes(&inputs(&["unused", "a", "b"]), &target).unwrap();
    assert_eq!(resolved.ids, ["a", "b"]);
}

#[test]
fn refuses_cycles_missing_parents_and_duplicate_links() {
    for (name, target, code) in [
        (
            "cycle",
            super::Target::new(["a"], [("a", "b"), ("b", "a")]),
            "SCOPE_CYCLE",
        ),
        (
            "missing",
            super::Target::new(["a"], [("a", "missing")]),
            "MISSING_SCOPE",
        ),
        (
            "ambiguous",
            super::Target::new(["a"], [("a", "b"), ("a", "c")]),
            "AMBIGUOUS_PARENT",
        ),
    ] {
        let error = super::resolve_scopes(&inputs(&["a", "b", "c"]), &target).unwrap_err();
        assert!(error.to_string().starts_with(code), "{name}: {error}");
    }
}
