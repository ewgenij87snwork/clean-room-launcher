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
        "gh release create",
        "--draft",
        "refusing to modify an already-published release",
        "attestations: write",
        "id-token: write",
    ] {
        assert!(workflow.contains(required), "missing release control: {required}");
    }
    for forbidden in [
        "actions/checkout@v",
        "actions/upload-artifact@v",
        "actions/download-artifact@v",
        "actions/attest-build-provenance@v",
        "gh release create \"$tag\" --latest",
        "gh release edit \"$tag\" --draft=false",
    ] {
        assert!(!workflow.contains(forbidden), "unsafe release workflow token: {forbidden}");
    }
}

#[test]
fn release_sbom_is_subject_bound_and_private_path_free() {
    let temp = std::env::temp_dir().join(format!("clroom-release-sbom-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let artifact = temp.join("clean-room-launcher-v0.2.0-aarch64-apple-darwin.tar.gz");
    let sbom = temp.join("sbom.cdx.json");
    fs::write(&artifact, b"release-artifact-bytes").unwrap();

    let output = Command::new("python3")
        .arg(root().join("packaging/generate-release-sbom.py"))
        .args(["--artifact", artifact.to_str().unwrap(), "--output", sbom.to_str().unwrap()])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let text = fs::read_to_string(&sbom).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["bomFormat"], "CycloneDX");
    assert_eq!(value["specVersion"], "1.7");
    assert_eq!(value["metadata"]["component"]["name"], "clean-room-launcher");
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
