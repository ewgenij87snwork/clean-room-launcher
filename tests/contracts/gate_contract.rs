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
    let provisioner =
        std::fs::read_to_string("scripts/release/provision-provider-canaries.sh").unwrap();
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
    assert!(source.contains(
        "./scripts/release/provision-provider-canaries.sh \"$RUNNER_TEMP/clroom-providers\" \"$GITHUB_ENV\""
    ));
    assert!(!source.contains("npm install"));
    let pins: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("release/qualification.json").unwrap(),
    )
    .unwrap();
    let codex = pins["providers"]["codex"]["clean_exact"].as_str().unwrap();
    let claude = pins["providers"]["claude"]["clean_exact"].as_str().unwrap();
    assert!(provisioner.contains(&format!("@openai/codex@{codex}")));
    assert!(provisioner.contains(&format!("@openai/codex@{codex}-darwin-arm64")));
    assert!(provisioner.contains(&format!("@anthropic-ai/claude-code@{claude}")));
    assert!(provisioner.contains(
        "HP/vJCH/t2hB9Kg6hotN9UglClJ6/z584fal5lEP14C9gNAgAQS4/kTQC7l5V+BA3TqwDPwINSjul28cX8AYXg=="
    ));
    assert!(!provisioner.contains("npm install"));

    assert!(source.contains("qualification_extract=\"$RUNNER_TEMP/clroom-release-archive\""));
    assert!(source.contains("tar -xzf \"$artifact\" -C \"$qualification_extract\""));
    assert!(source.contains("candidate_dir=\"$archive_root/bin\""));
    assert!(source.contains("--candidate \"$candidate_dir/clroom-codex\""));
    assert!(source.contains("--candidate \"$candidate_dir/clroom-claude\""));
    assert!(
        !source.contains("target/aarch64-apple-darwin/release/clroom-codex")
            && !source.contains("target/aarch64-apple-darwin/release/clroom-claude"),
        "provider qualification must not use sibling target build outputs"
    );
    assert_eq!(
        source.matches("scripts/release/qualify-real-provider.sh").count(),
        2,
        "both qualified providers must execute against binaries extracted from the archive"
    );
    assert_eq!(
        source.matches("scripts/release/verify-qualification.py").count(),
        2,
        "both qualification records must be rebound to the exact release archive"
    );
}


#[test]
fn release_contract_binds_latest_stable_annotated_tags_and_inference_free_local_smoke() {
    let candidate = std::fs::read_to_string(".github/workflows/release-candidate.yml").unwrap();
    let release = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    let smoke = std::fs::read_to_string("scripts/release/local-release-smoke.sh").unwrap();
    let attestation =
        std::fs::read_to_string("scripts/release/check-attestation-contract.sh").unwrap();

    assert!(
        candidate.contains("gh api \"repos/$GITHUB_REPOSITORY/releases/latest\" --jq .tag_name"),
        "release candidate must resolve the authoritative latest published stable release"
    );
    assert!(candidate.contains("python3 scripts/release/check-release-review.py"));

    assert!(
        release.contains("git cat-file -t \"refs/tags/$tag\"") && release.contains("= tag"),
        "release workflow must reject lightweight release tags"
    );
    assert!(release.contains("%(taggerdate:short)"));
    assert!(
        release.contains("gh api \"repos/$GITHUB_REPOSITORY/releases/latest\" --jq .tag_name")
    );

    assert!(
        smoke.contains("claude --init-only")
            && smoke.contains("--with=\"plugin:$plugin_id\" --init-only"),
        "Claude release smoke must use inference-free clean and selected startup"
    );
    assert!(!smoke.contains("Reply exactly UNUSED"));
    assert!(!smoke.contains("--output-format stream-json"));

    assert!(attestation.contains("--bundle \"$provenance\""));
    assert!(attestation.contains("--bundle \"$sbom\""));

    let pins: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("release/qualification.json").unwrap())
            .unwrap();
    let claude_clean = pins["providers"]["claude"]["clean_exact"].as_str().unwrap();
    let claude_plugin = pins["providers"]["claude"]["plugin_activation_exact"]
        .as_str()
        .unwrap();
    assert_ne!(claude_clean, claude_plugin);
    assert!(smoke.contains("$claude_version\" != \"$claude_plugin_pin"));
    assert!(!smoke.contains(
        "$claude_version\" != \"$claude_clean_pin\" || \"$claude_version\" != \"$claude_plugin_pin"
    ));

    let post_publish =
        std::fs::read_to_string("scripts/release/post-publish-smoke.sh").unwrap();
    assert!(post_publish.contains("releases/latest/download/install.sh"));
    assert!(post_publish.contains("LATEST_INSTALLER_BYTES_MISMATCH"));
    assert!(post_publish.contains("sh \"$latest_installer\""));
    assert!(post_publish.contains("--source-digest \"$source_head\""));
    assert!(post_publish.contains("--source-ref \"refs/tags/$tag\""));

    let tag_push = std::fs::read_to_string("scripts/release/push-release-tag.sh").unwrap();
    let repo_policy =
        std::fs::read_to_string("scripts/release/check-repository-release-policy.py").unwrap();
    assert!(
        std::fs::metadata("scripts/release/push-release-tag.sh")
            .unwrap()
            .permissions()
            .mode()
            & 0o111
            != 0,
        "tag push helper must be executable before it is documented as a direct command"
    );
    assert!(tag_push.contains("git cat-file -t \"refs/tags/$tag\""));
    assert!(tag_push.contains("git push origin \"refs/tags/$tag:refs/tags/$tag\""));
    assert!(tag_push.contains("REMOTE_TAG_ALREADY_EXISTS"));
    assert!(tag_push.contains("python3 scripts/release/check-repository-release-policy.py"));
    assert!(repo_policy.contains("refs/tags/v*"));
    assert!(repo_policy.contains("{\"update\", \"deletion\"}.issubset(rule_types)"));
    assert!(repo_policy.contains("not bypass"));
    assert!(smoke.contains("--source-digest \"$source_head\""));
    assert!(smoke.contains("--source-ref \"refs/tags/$tag\""));
    assert!(smoke.contains("[[ \"$codex_tui_rc\" -eq 0 ]] || fail \"CODEX_TUI_EXIT\""));
    assert!(smoke.contains("[[ \"$claude_clean_tui_rc\" -eq 0 ]] || fail \"CLAUDE_CLEAN_TUI_EXIT\""));
    assert!(smoke.contains("[[ \"$claude_tui_rc\" -eq 0 ]] || fail \"CLAUDE_SELECTED_TUI_EXIT\""));
}

#[test]
fn whole_release_review_is_fail_closed_and_declared() {
    let checker = std::fs::read_to_string("scripts/release/check-release-review.py").unwrap();
    let declaration: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("release/review.json").unwrap()).unwrap();

    assert_eq!(declaration["schema_version"], "clroom.release-review.v1");
    assert!(
        declaration["strategic_product_outcome"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
    );
    assert_eq!(declaration["contract_evolution"]["reviewed"], true);

    for invariant in [
        "unclassified-paths:",
        "undeclared-change-classes:",
        "missing-required-evidence:",
        "published-baseline:",
        "contract-evolution-review",
        "contract-change-requires-expansion-review",
        "scripts/release/local-release-smoke.sh",
        "scripts/release/post-publish-smoke.sh",
        ".github/workflows/release.yml",
    ] {
        assert!(checker.contains(invariant), "missing fail-closed invariant {invariant}");
    }
}

use std::os::unix::fs::PermissionsExt;
