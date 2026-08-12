use super::level_a::LevelAEntry;
use super::projection::{CapabilityTruth, ProjectionError, project_native};
fn entry() -> LevelAEntry {
    LevelAEntry {
        id: "a".into(),
        name: "alpha".into(),
        capability: "c".into(),
        trigger_summary: "use alpha".into(),
        source_id: "src".into(),
        body_digest: "a".repeat(64),
    }
}
#[test]
fn unsupported_and_tampered_native_seams_refuse() {
    assert_eq!(
        project_native(
            &[entry()],
            CapabilityTruth {
                native_progressive_disclosure: false,
                observed_digest: None
            }
        )
        .unwrap_err(),
        ProjectionError::UnsupportedNativeSeam
    );
    assert_eq!(
        project_native(
            &[entry()],
            CapabilityTruth {
                native_progressive_disclosure: true,
                observed_digest: Some("b".repeat(64))
            }
        )
        .unwrap_err(),
        ProjectionError::DigestMismatch
    )
}
#[test]
fn projection_contains_names_and_links_but_no_body_api_or_bytes() {
    let p = project_native(
        &[entry()],
        CapabilityTruth {
            native_progressive_disclosure: true,
            observed_digest: Some("a".repeat(64)),
        },
    )
    .unwrap();
    assert_eq!(p.entries[0].name, "alpha");
    assert!(p.startup_bytes.windows(5).all(|w| w != b"BODY:"));
}
