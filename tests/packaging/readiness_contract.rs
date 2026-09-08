use std::fs;

#[test]
fn canonical_release_readiness_gate_covers_required_v020_controls() {
    let source = fs::read_to_string("scripts/release/readiness.sh").unwrap();
    for required in [
        "git diff --check",
        "check-public-boundary.sh",
        "shellcheck",
        "cargo test --locked --all-targets",
        "cargo deny --config deny.toml --locked check",
        "qualification=QUALIFIED",
        "sbom.cdx.json",
        "provenance.intoto.json",
        "shasum -a 256",
    ] {
        assert!(source.contains(required), "readiness gate omits {required}");
    }
    assert!(!source.contains("audit-release"));
    assert!(!source.contains("release-build/verify-source"));
}
