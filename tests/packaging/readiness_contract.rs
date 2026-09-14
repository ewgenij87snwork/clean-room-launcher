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
fn tag_release_primes_the_full_locked_graph_before_offline_notice_packaging() {
    let source = fs::read_to_string(".github/workflows/release.yml").unwrap();
    let fetch = source
        .find("cargo fetch --locked")
        .expect("release workflow must prime the full locked dependency graph");
    let offline_probe = source
        .find("cargo metadata --locked --offline --format-version 1 >/dev/null")
        .expect("release workflow must prove the full graph is available offline");
    let target_metadata = source
        .find("--filter-platform aarch64-apple-darwin")
        .expect("release workflow must keep the target-filtered SBOM metadata");
    let package = source
        .find("./packaging/build-artifacts.sh target/release-artifacts")
        .expect("release workflow must invoke the canonical artifact builder");

    assert!(
        fetch < offline_probe && offline_probe < target_metadata && target_metadata < package,
        "full-graph prefetch and offline proof must precede NOTICE packaging"
    );
    assert!(
        !source.contains("cargo fetch --locked --target"),
        "target-scoped fetch cannot satisfy the full locked NOTICE census"
    );
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
