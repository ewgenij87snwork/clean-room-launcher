#[test]
fn consolidated_gate_exists_and_is_executable_contract_surface() {
    let path = std::path::Path::new("scripts/gates/p02/verify.sh");
    if !path.exists() {
        let inventory = std::fs::read_to_string("qualification/public-release-inventory-v1.json")
            .expect("public release inventory exists when internal gates are excluded");
        assert!(inventory.contains("\"scripts/gates\""));
        return;
    }
    let metadata = std::fs::metadata(path).expect("P02 gate exists");
    assert!(
        metadata.permissions().mode() & 0o111 != 0,
        "P02 gate executable"
    );
}

#[test]
fn tag_release_primes_the_full_locked_graph_before_offline_notice_packaging() {
    let source = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
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
fn tag_release_qualifies_the_exact_archive_before_upload() {
    let source = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    let package = source
        .find("./packaging/build-artifacts.sh target/release-artifacts")
        .expect("release workflow must build the canonical archive");
    let provision = source
        .find("name: Provision pinned real provider canaries")
        .expect("release workflow must provision pinned real providers");
    let qualify = source
        .find("name: Qualify exact release archive with real providers")
        .expect("release workflow must qualify the exact release archive");
    let upload = source
        .find("name: Upload verified release bundle")
        .expect("release workflow must upload the verified release bundle");

    assert!(
        package < provision && provision < qualify && qualify < upload,
        "exact-byte provider qualification must happen after packaging and before upload"
    );
    let release_version_guard = format!(
        "test \"$CLROOM_RELEASE_VERSION\" = \"{}\"",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        source.contains(&release_version_guard),
        "provider qualification pins must fail closed unless explicitly reviewed for the packaged release version"
    );
    assert!(source.contains("@openai/codex@0.154.0"));
    assert!(source.contains("@anthropic-ai/claude-code@2.1.272"));
    assert!(source.contains("target/aarch64-apple-darwin/release/clroom-codex"));
    assert!(source.contains("target/aarch64-apple-darwin/release/clroom-claude"));
    assert_eq!(
        source.matches("scripts/release/qualify-real-provider.sh").count(),
        2,
        "both qualified providers must execute against the release-built binaries"
    );
    assert_eq!(
        source.matches("scripts/release/verify-qualification.py").count(),
        2,
        "both qualification records must be rebound to the exact release archive"
    );
}

#[test]
fn browser_e2e_uses_the_unified_claude_route() {
    let source = std::fs::read_to_string("scripts/release/qualify-claude-browser-e2e.sh").unwrap();
    assert!(
        source.contains("\"$candidate\" claude --with=browser -- \"$prompt\""),
        "manual browser qualification must enter the unified launcher through the claude subcommand"
    );
    assert!(
        !source.contains("\"$candidate\" --with=browser -- \"$prompt\""),
        "launcher-owned browser options cannot be parsed before selecting the provider"
    );
}

use std::os::unix::fs::PermissionsExt;
