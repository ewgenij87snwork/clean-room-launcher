use super::level_a::LevelAEntry;
use super::level_b::{BodyDecision, DecisionRecord};
use super::manifest::CatalogManifest;
use super::pipeline::ValidatedCatalog;
use super::projection::{
    ProjectionError, parse_qualification_receipt, project_native, verify_native_observation,
};
use crate::contracts::adapter::parse_declaration;
use std::process::Command;
const DIGEST_A: &str = "5f07933adc98d602398359e57783d348658eaa60fa7907da263b53291f8184b1";
const DIGEST_B: &str = "32eea1ee666999e53a16feb944d355ef49bcff8ec379a3e02ee690172752f55e";
fn entry(id: &str, name: &str, digest: &str) -> LevelAEntry {
    LevelAEntry {
        id: id.into(),
        name: name.into(),
        capability: "c".into(),
        trigger_summary: format!("use {name}"),
        source_id: "src".into(),
        body_digest: digest.to_owned(),
        native_frontmatter: true,
    }
}
fn catalog(entries: Vec<LevelAEntry>) -> ValidatedCatalog {
    let decisions = entries
        .iter()
        .map(|entry| (entry.id.clone(), BodyDecision::LoadOnInvoke))
        .collect();
    catalog_with_decisions(entries, decisions)
}
fn catalog_with_decisions(
    entries: Vec<LevelAEntry>,
    decisions: Vec<(String, BodyDecision)>,
) -> ValidatedCatalog {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/catalog/native-projection");
    let body_paths = entries
        .iter()
        .map(|entry| (entry.id.clone(), root.join(&entry.id).join("SKILL.md")))
        .collect();
    let decisions = decisions
        .into_iter()
        .map(|(id, decision)| DecisionRecord {
            id: id.clone(),
            decision,
            reason: "TEST",
            reason_chain: vec![id],
        })
        .collect();
    ValidatedCatalog::from_manifest_for_test(
        CatalogManifest::new(entries, decisions, vec![]).unwrap(),
        body_paths,
    )
}
#[test]
fn refused_skills_remain_discoverable_but_have_no_native_body_path() {
    let entries = vec![entry("a", "alpha", DIGEST_A), entry("b", "beta", DIGEST_B)];
    let catalog = catalog_with_decisions(
        entries,
        vec![
            ("a".into(), BodyDecision::Refuse),
            ("b".into(), BodyDecision::LoadNow),
        ],
    );
    let fixture =
        include_str!("../../fixtures/catalog/native-projection/codex-qualified-receipt.json");
    let receipt = fixture
        .replace(
            "\"catalog_digest\": \"5b6eecee06e584ba621fb7d7286cc41f04bda4f90e02ca26c9272170a6d66031\"",
            "\"catalog_digest\": \"9bc52e36d121fcdde786d9905d4f68ed416a4aff3bc4c02093ce975531fb7f12\"",
        )
        .replace(
            &format!("\"a\": \"{DIGEST_A}\",\n    "),
            "",
        );
    let probe = include_bytes!("../../fixtures/catalog/native-projection/provider-native-probe.sh");
    let projection = project_native(
        &catalog,
        &declaration(),
        parse_qualification_receipt(receipt.as_bytes(), &declaration(), probe, probe).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog.manifest().counts.refused, 1);
    assert_eq!(projection.entries.len(), 1);
    assert_eq!(projection.entries[0].id, "b");
    assert!(projection.entries[0].load_at_start);
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
            &catalog(vec![
                entry("a", "alpha", DIGEST_A),
                entry("b", "beta", DIGEST_B)
            ]),
            &d,
            receipt,
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
        &catalog(vec![
            entry("a", "alpha", DIGEST_A),
            entry("b", "beta", DIGEST_B),
        ]),
        &d,
        r,
    )
    .unwrap();
    assert!(p.entries[0].native_path.ends_with("a/SKILL.md"));
    assert_eq!(p.entries.len(), 2);
    assert!(p.startup_bytes.windows(5).all(|w| w != b"BODY:"))
}

#[test]
fn qualified_native_fixture_executes_canary_and_matches_bound_receipt() {
    let projected = project_native(
        &catalog(vec![
            entry("a", "alpha", DIGEST_A),
            entry("b", "beta", DIGEST_B),
        ]),
        &declaration(),
        fixture_receipt(),
    )
    .unwrap();
    let selected = &projected.entries[0];
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/catalog/native-projection/provider-native-probe.sh"
    );
    let output = Command::new("sh")
        .arg(script)
        .arg(format!(
            "{}={}",
            selected.id,
            selected.native_path.display()
        ))
        .output()
        .unwrap();
    assert!(output.status.success());
    verify_native_observation(&projected, &selected.id, &output.stdout).unwrap();
    assert!(!String::from_utf8(output.stdout).unwrap().contains("b="));
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
        &format!("\"a\": \"{DIGEST_A}\""),
        &format!("\"a\": \"{DIGEST_A}\",\n    \"a\": \"{DIGEST_B}\""),
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
    let r = fixture_receipt();
    assert_eq!(
        project_native(
            &catalog(vec![
                entry("a", "alpha", &"c".repeat(64)),
                entry("b", "beta", DIGEST_B),
            ]),
            &d,
            r,
        )
        .unwrap_err(),
        ProjectionError::DigestMismatch("a".into())
    )
}
