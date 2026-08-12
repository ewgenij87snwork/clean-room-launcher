use super::budget::{BodyMeasurement, CatalogLimits};
use super::dependency::DependencyGraph;
use super::inventory::inventory_skills;
use super::level_b::SkillSignals;
use super::pipeline::build_validated_catalog;
use super::sources::{SkillSourceAuthority, SkillSourceConfig, enumerate_sources};
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
        &[BodyMeasurement {
            id: record.id,
            bytes: 128,
            token_upper_bound: 128,
        }],
        CatalogLimits {
            index_bytes: 4096,
            detail_bytes: 4096,
            detail_tokens: 4096,
        },
        vec![],
    )
    .unwrap();
    assert_eq!(validated.manifest().counts.total, 1);
    assert_eq!(validated.manifest().counts.load_on_invoke, 1);
}
