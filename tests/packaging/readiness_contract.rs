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
        "qualification=CANDIDATE",
        "CLROOM_QUALIFICATION_EVIDENCE_DIR",
        "verify-qualification.py",
        "sbom.cdx.json",
        "provenance.intoto.json",
        "shasum -a 256",
    ] {
        assert!(source.contains(required), "readiness gate omits {required}");
    }
    assert!(!source.contains("audit-release"));
    assert!(!source.contains("release-build/verify-source"));
}

#[test]
fn codex_qualification_cannot_regress_to_a_diagnostic_probe() {
    let harness = fs::read_to_string("scripts/release/qualify-real-provider.sh").unwrap();
    let verifier = fs::read_to_string("scripts/release/verify-qualification.py").unwrap();
    assert!(harness.contains("if [[ $provider == codex ]]"));
    assert!(harness.contains("pty.fork()"));
    assert!(harness.contains("[candidate, \"--no-alt-screen\"]"));
    assert!(harness.contains("real-provider-interactive-startup-no-model"));
    assert!(verifier.contains("expected_scope"));
    assert!(verifier.contains("real-provider-interactive-startup-no-model"));
    assert!(verifier.contains("interactive-path"));
}
