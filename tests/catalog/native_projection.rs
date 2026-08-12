use super::level_a::LevelAEntry;
use super::projection::{CapabilityTruth, ProjectionError, project_native};
use crate::contracts::adapter::AdapterDeclaration;
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
fn declaration(qualified: bool) -> AdapterDeclaration {
    AdapterDeclaration {
        provider_id: "codex".into(),
        executable: "codex".into(),
        version_range: ">=0.147.0".into(),
        context_target: "provider_native_context".into(),
        collision_policy: "deny".into(),
        capability_evidence: "fixture".into(),
        qualified,
    }
}
#[test]
fn unsupported_and_tampered_native_seams_refuse() {
    assert_eq!(
        project_native(
            &[entry("a", "alpha", 'a')],
            &declaration(false),
            CapabilityTruth {
                native_progressive_disclosure: false,
                observed_digests: BTreeMap::new()
            }
        )
        .unwrap_err(),
        ProjectionError::UnsupportedNativeSeam
    );
    assert_eq!(
        project_native(
            &[entry("a", "alpha", 'a')],
            &declaration(true),
            CapabilityTruth {
                native_progressive_disclosure: true,
                observed_digests: BTreeMap::from([("a".into(), "b".repeat(64))])
            }
        )
        .unwrap_err(),
        ProjectionError::DigestMismatch("a".into())
    );
}
#[test]
fn multi_skill_projection_is_per_skill_digest_bound_and_body_free() {
    let p = project_native(
        &[entry("a", "alpha", 'a'), entry("b", "beta", 'b')],
        &declaration(true),
        CapabilityTruth {
            native_progressive_disclosure: true,
            observed_digests: BTreeMap::from([
                ("a".into(), "a".repeat(64)),
                ("b".into(), "b".repeat(64)),
            ]),
        },
    )
    .unwrap();
    assert_eq!(p.entries[0].native_link, "skill://codex/a");
    assert_eq!(p.entries.len(), 2);
    assert!(p.startup_bytes.windows(5).all(|w| w != b"BODY:"));
}
