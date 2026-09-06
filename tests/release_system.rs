use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn release_workflow_is_guarded_draft_only_and_sha_pinned() {
    let workflow = fs::read_to_string(root().join(".github/workflows/release.yml")).unwrap();
    for required in [
        "git fetch --no-tags --depth=1 origin main",
        "release tag must point to the exact accepted main tip",
        "cargo test --locked --all-targets --target aarch64-apple-darwin",
        "CLROOM_ARTIFACT_QUALIFICATION: NOT_QUALIFIED",
        "python3 packaging/verify-artifact.py",
        "python3 packaging/generate-release-sbom.py",
        "shasum -a 256 -c SHA256SUMS",
        "actions/attest@",
        "subject-path:",
        "sbom-path:",
        "args=(release create",
        "--draft",
        "refusing to modify an already-published release",
        "attestations: write",
        "id-token: write",
        "artifact-metadata: write",
        "persist-credentials: false",
    ] {
        assert!(workflow.contains(required), "missing release control: {required}");
    }
    for forbidden in [
        "actions/checkout@v",
        "actions/upload-artifact@v",
        "actions/download-artifact@v",
        "actions/attest@v",
        "actions/attest-build-provenance@",
        "--latest",
        "--draft=false",
        "release publish",
    ] {
        assert!(!workflow.contains(forbidden), "unsafe release workflow token: {forbidden}");
    }
}

#[test]
fn release_sbom_is_subject_bound_private_path_free_and_metadata_driven() {
    let temp = std::env::temp_dir().join(format!("clroom-release-sbom-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let artifact = temp.join("clean-room-launcher-v0.2.0-aarch64-apple-darwin.tar.gz");
    let metadata = temp.join("cargo-metadata.json");
    let sbom = temp.join("sbom.cdx.json");
    fs::write(&artifact, b"release-artifact-bytes").unwrap();
    fs::write(
        &metadata,
        r#"{
          "packages": [
            {
              "id": "path+file:///workspace/clean-room-launcher#0.2.0",
              "name": "clean-room-launcher",
              "version": "0.2.0",
              "license": "MPL-2.0"
            },
            {
              "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.229",
              "name": "serde",
              "version": "1.0.229",
              "license": "MIT OR Apache-2.0"
            }
          ],
          "resolve": {
            "root": "path+file:///workspace/clean-room-launcher#0.2.0",
            "nodes": [
              {
                "id": "path+file:///workspace/clean-room-launcher#0.2.0",
                "dependencies": ["registry+https://github.com/rust-lang/crates.io-index#serde@1.0.229"]
              },
              {
                "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.229",
                "dependencies": []
              }
            ]
          }
        }"#,
    )
    .unwrap();

    let output = Command::new("python3")
        .arg(root().join("packaging/generate-release-sbom.py"))
        .args([
            "--artifact",
            artifact.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            sbom.to_str().unwrap(),
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let text = fs::read_to_string(&sbom).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["bomFormat"], "CycloneDX");
    assert_eq!(value["specVersion"], "1.7");
    assert_eq!(value["metadata"]["component"]["name"], "clean-room-launcher");
    assert_eq!(value["components"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["metadata"]["properties"][0]["value"],
        artifact.file_name().unwrap().to_str().unwrap()
    );
    assert!(!text.contains("/Users/"));
    assert!(!text.contains("/home/"));
    assert!(!text.contains("ghp_"));
    assert!(!text.contains("sk-"));

    fs::remove_dir_all(&temp).unwrap();
}
