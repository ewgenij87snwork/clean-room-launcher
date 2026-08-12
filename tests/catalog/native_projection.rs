use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
use super::manifest::CatalogManifest;
use super::pipeline::ValidatedCatalog;
use super::projection::{ProjectionError, parse_qualification_receipt, project_native};
use crate::contracts::adapter::parse_declaration;
use std::process::Command;
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
fn catalog(entries: Vec<LevelAEntry>) -> ValidatedCatalog {
    let decisions = entries
        .iter()
        .map(|entry| DecisionRecord {
            id: entry.id.clone(),
            decision: BodyDecision::LoadOnInvoke,
            reason: "DEFERRED_NATIVE",
        })
        .collect();
    ValidatedCatalog::from_manifest_for_test(
        CatalogManifest::new(entries, decisions, vec![]).unwrap(),
    )
}
fn declaration() -> crate::contracts::adapter::AdapterDeclaration {
    parse_declaration(include_str!("../../adapters/declarations/codex.toml")).unwrap()
}
fn fixture_receipt() -> super::projection::QualificationReceipt {
    let probe = include_bytes!("../../fixtures/catalog/native-projection/provider-native-probe.sh");
    parse_qualification_receipt(
        include_bytes!("../../fixtures/catalog/native-projection/codex-qualified-receipt.json"),
        &declaration(),
        probe,
        probe,
    )
    .unwrap()
}
#[test]
fn real_unqualified_declaration_requires_bound_qualification_receipt() {
    let d = declaration();
    assert!(!d.qualified);
    let bytes =
        include_str!("../../fixtures/catalog/native-projection/codex-qualified-receipt.json")
            .replace(
                "\"native_progressive_disclosure\": true",
                "\"native_progressive_disclosure\": false",
            );
    let probe = include_bytes!("../../fixtures/catalog/native-projection/provider-native-probe.sh");
    let receipt = parse_qualification_receipt(bytes.as_bytes(), &d, probe, probe).unwrap();
    assert_eq!(
        project_native(
            &catalog(vec![entry("a", "alpha", 'a'), entry("b", "beta", 'b')]),
            &d,
            receipt
        )
        .unwrap_err(),
        ProjectionError::UnsupportedNativeSeam
    )
}
#[test]
fn multi_skill_projection_is_per_skill_digest_bound_and_body_free() {
    let d = declaration();
    let r = fixture_receipt();
    let p = project_native(
        &catalog(vec![entry("a", "alpha", 'a'), entry("b", "beta", 'b')]),
        &d,
        r,
    )
    .unwrap();
    assert_eq!(p.entries[0].native_link, "skill://codex/a");
    assert_eq!(p.entries.len(), 2);
    assert!(p.startup_bytes.windows(5).all(|w| w != b"BODY:"))
}

#[test]
fn qualified_native_fixture_executes_canary_and_matches_bound_receipt() {
    let probe = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/catalog/native-projection/provider-native-probe.sh"
    );
    let output = Command::new("sh").arg(probe).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "a=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "b=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n"
        )
    );
    let projected = project_native(
        &catalog(vec![entry("a", "alpha", 'a'), entry("b", "beta", 'b')]),
        &declaration(),
        fixture_receipt(),
    )
    .unwrap();
    assert_eq!(projected.entries.len(), 2);
}
#[test]
fn immutable_receipt_parser_rejects_unbound_or_unknown_evidence() {
    let d = declaration();
    let probe = include_bytes!("../../fixtures/catalog/native-projection/provider-native-probe.sh");
    let bad =
        include_bytes!("../../fixtures/catalog/native-projection/codex-qualified-receipt.json")
            .as_slice()
            .to_vec();
    let text = String::from_utf8(bad).unwrap();
    let unbound = text.replace("980c03af", "080c03af");
    assert_eq!(
        parse_qualification_receipt(unbound.as_bytes(), &d, probe, probe).unwrap_err(),
        ProjectionError::ReceiptBindingMismatch
    );
    let unknown = text.replace("\n}", ",\n  \"ambient_claim\": true\n}");
    assert_eq!(
        parse_qualification_receipt(unknown.as_bytes(), &d, probe, probe).unwrap_err(),
        ProjectionError::InvalidReceipt
    );
    let duplicate = text.replace(
        "\"a\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        "\"a\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n    \"a\": \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"",
    );
    assert_eq!(
        parse_qualification_receipt(duplicate.as_bytes(), &d, probe, probe).unwrap_err(),
        ProjectionError::InvalidReceipt
    );
}
#[test]
fn tampered_receipt_or_body_digest_refuses() {
    let d = declaration();
    let probe = include_bytes!("../../fixtures/catalog/native-projection/provider-native-probe.sh");
    let fixture =
        include_str!("../../fixtures/catalog/native-projection/codex-qualified-receipt.json");
    let bad_binding = fixture.replace("980c03af", "080c03af");
    assert_eq!(
        parse_qualification_receipt(bad_binding.as_bytes(), &d, probe, probe).unwrap_err(),
        ProjectionError::ReceiptBindingMismatch
    );
    let bad_body = fixture.replace(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    );
    let bad_body = bad_body.replace(
        "a22932d6dd21d960a6ed4837c1b486536ebb4415c024bca16024ec1e581c69a8",
        "a2f6abba8cbb51691b0a094cc60024b5aee08df6ef821878cfee4fd1cddd7108",
    );
    let r = parse_qualification_receipt(bad_body.as_bytes(), &d, probe, probe).unwrap();
    assert_eq!(
        project_native(
            &catalog(vec![entry("a", "alpha", 'a'), entry("b", "beta", 'b')]),
            &d,
            r
        )
        .unwrap_err(),
        ProjectionError::DigestMismatch("a".into())
    )
}
