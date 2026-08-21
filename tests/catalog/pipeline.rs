use super::budget::CatalogLimits;
use super::dependency::DependencyGraph;
use super::inventory::inventory_skills;
use super::level_b::SkillSignals;
use super::pipeline::{P04_ACCEPTANCE_CONTROLS, build_validated_catalog};
use super::sources::{SkillSourceAuthority, SkillSourceConfig, enumerate_sources};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn production_pipeline_enforces_every_catalog_stage_before_projection() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/catalog/inventory/native-frontmatter");
    let sources = enumerate_sources(
        &SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]),
        std::slice::from_ref(&root),
    )
    .unwrap();
    let record = inventory_skills(&sources).unwrap().remove(0);
    let graph = DependencyGraph::new([(record.id.as_str(), vec![])]);
    let validated = build_validated_catalog(
        &sources,
        &[SkillSignals::new(&record.id)],
        &graph,
        CatalogLimits {
            index_bytes: 4096,
            detail_bytes: 4096,
            detail_tokens: 4096,
        },
    )
    .unwrap();
    assert_eq!(validated.manifest().counts.total, 1);
    assert_eq!(validated.manifest().counts.load_on_invoke, 1);
}

#[test]
fn production_pipeline_loads_baseline_protected_categories_without_caller_flag() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/catalog/protection-baseline");
    let sources = enumerate_sources(
        &SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]),
        std::slice::from_ref(&root),
    )
    .unwrap();
    let record = inventory_skills(&sources).unwrap().remove(0);
    let graph = DependencyGraph::new([(record.id.as_str(), vec![])]);
    let validated = build_validated_catalog(
        &sources,
        &[SkillSignals::new(&record.id)],
        &graph,
        CatalogLimits {
            index_bytes: 4096,
            detail_bytes: 4096,
            detail_tokens: 4096,
        },
    )
    .unwrap();
    assert_eq!(validated.manifest().counts.loaded_now, 1);
    assert_eq!(validated.manifest().decisions[0].reason, "PROTECTED");
}

#[test]
fn committed_acceptance_evidence_is_derived_from_production_pipeline() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/catalog/native-projection");
    let sources = enumerate_sources(
        &SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]),
        std::slice::from_ref(&root),
    )
    .unwrap();
    let records = inventory_skills(&sources).unwrap();
    let signals: Vec<_> = records
        .iter()
        .map(|record| SkillSignals::new(&record.id))
        .collect();
    let graph = DependencyGraph::from_ids(records.iter().map(|record| record.id.clone()));
    let validated = build_validated_catalog(
        &sources,
        &signals,
        &graph,
        CatalogLimits {
            index_bytes: 8192,
            detail_bytes: 8192,
            detail_tokens: 8192,
        },
    )
    .unwrap();
    let evidence: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("reports/gates/p04/acceptance-evidence.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let manifest = validated.manifest();
    assert_eq!(evidence["census"]["total"], manifest.counts.total);
    assert_eq!(
        evidence["census"]["load_on_invoke"],
        manifest.counts.load_on_invoke
    );
    assert_eq!(
        evidence["context_bytes"]["full_body_baseline"],
        records.iter().map(|record| record.body_bytes).sum::<u64>()
    );
    let startup = serde_json::to_vec(&manifest.level_a).unwrap();
    assert_eq!(evidence["context_bytes"]["startup_level_a"], startup.len());
    assert!(
        !startup
            .windows(22)
            .any(|window| window == b"ORIGINAL_BODY_CANARY_")
    );
    assert_eq!(evidence["context_bytes"]["full_bodies_at_startup"], 0);
    let inventory_digests: BTreeMap<_, _> = records
        .iter()
        .map(|record| (record.name.as_str(), record.body_digest.as_str()))
        .collect();
    assert_eq!(
        evidence["inventory_body_digests"],
        serde_json::to_value(inventory_digests).unwrap()
    );
    assert_eq!(
        evidence["controls"],
        serde_json::to_value(P04_ACCEPTANCE_CONTROLS).unwrap()
    );
    assert_eq!(
        evidence["command"],
        serde_json::json!({
            "name": "cargo test --lib catalog::pipeline_tests::committed_acceptance_evidence_is_derived_from_production_pipeline",
            "environment": "locked-offline-production-path",
            "exit": 0
        })
    );
    for decision in &manifest.decisions {
        let name = &manifest
            .level_a
            .iter()
            .find(|entry| entry.id == decision.id)
            .unwrap()
            .name;
        assert_eq!(evidence["reasons"][name], decision.reason);
    }
}
