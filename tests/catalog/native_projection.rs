use super::level_a::LevelAEntry;
use super::projection::{
    ProjectionError, QualificationReceipt, declaration_digest, project_native,
};
use crate::contracts::adapter::parse_declaration;
use std::collections::BTreeMap;
fn entry(id: &str, name: &str, digest: char) -> LevelAEntry {
    LevelAEntry {
        id: id.into(),
        name: name.into(),
        capability: "c".into(),
        trigger_summary: format!("use {name}"),
        source_id: "src".into(),
        body_digest: digest.to_string().repeat(64),
    }
}
fn declaration() -> crate::contracts::adapter::AdapterDeclaration {
    parse_declaration(include_str!("../../adapters/declarations/codex.toml")).unwrap()
}
fn receipt(ok: bool) -> QualificationReceipt {
    let d = declaration();
    QualificationReceipt {
        provider_id: "codex".into(),
        declaration_digest: declaration_digest(&d),
        native_progressive_disclosure: ok,
        projection_candidate: ok,
        observed_digests: BTreeMap::new(),
    }
}
#[test]
fn real_unqualified_declaration_requires_bound_qualification_receipt() {
    let d = declaration();
    assert!(!d.qualified);
    assert_eq!(
        project_native(&[entry("a", "alpha", 'a')], &d, receipt(false)).unwrap_err(),
        ProjectionError::UnsupportedNativeSeam
    )
}
#[test]
fn multi_skill_projection_is_per_skill_digest_bound_and_body_free() {
    let d = declaration();
    let mut r = receipt(true);
    r.observed_digests =
        BTreeMap::from([("a".into(), "a".repeat(64)), ("b".into(), "b".repeat(64))]);
    let p = project_native(&[entry("a", "alpha", 'a'), entry("b", "beta", 'b')], &d, r).unwrap();
    assert_eq!(p.entries[0].native_link, "skill://codex/a");
    assert_eq!(p.entries.len(), 2);
    assert!(p.startup_bytes.windows(5).all(|w| w != b"BODY:"))
}
#[test]
fn tampered_receipt_or_body_digest_refuses() {
    let d = declaration();
    let mut r = receipt(true);
    r.declaration_digest = "0".repeat(64);
    assert_eq!(
        project_native(&[entry("a", "alpha", 'a')], &d, r).unwrap_err(),
        ProjectionError::ReceiptBindingMismatch
    );
    let mut r = receipt(true);
    r.observed_digests.insert("a".into(), "b".repeat(64));
    assert_eq!(
        project_native(&[entry("a", "alpha", 'a')], &d, r).unwrap_err(),
        ProjectionError::DigestMismatch("a".into())
    )
}
