use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(root().join("packaging/supply-chain/generate.sh"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn metadata_is_subject_bound_complete_and_secret_free() {
    let script = root().join("packaging/supply-chain/generate.sh");
    assert!(script.is_file(), "supply-chain generator is missing");
    let temp = std::env::temp_dir().join(format!("p07-supply-chain-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let artifact = temp.join("taskseal-v0.1.0-aarch64-apple-darwin.tar.gz");
    let output = temp.join("metadata");
    fs::write(&artifact, b"taskseal-artifact-v1").unwrap();

    let generate = run(&[
        "generate", "--artifact", artifact.to_str().unwrap(), "--source-commit",
        "1111111111111111111111111111111111111111", "--target", "aarch64-apple-darwin",
        "--builder-id", "local://taskseal/p07", "--output", output.to_str().unwrap(),
    ]);
    assert!(generate.status.success(), "{}", String::from_utf8_lossy(&generate.stderr));
    assert_eq!(String::from_utf8(generate.stdout).unwrap(), "P07_SUPPLY_CHAIN_GENERATE_PASS qualification=NOT_QUALIFIED\n");

    let verify = || run(&["verify", "--artifact", artifact.to_str().unwrap(), "--output", output.to_str().unwrap()]);
    let accepted = verify();
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert_eq!(String::from_utf8(accepted.stdout).unwrap(), "P07_SUPPLY_CHAIN_VERIFY_PASS qualification=NOT_QUALIFIED\n");

    let combined = fs::read_to_string(output.join("sbom.cdx.json")).unwrap()
        + &fs::read_to_string(output.join("provenance.intoto.json")).unwrap();
    assert!(!combined.contains("/Users/") && !combined.contains("ghp_") && !combined.contains("sk-"));

    fs::write(&artifact, b"tampered").unwrap();
    assert!(!verify().status.success(), "changed subject was accepted");
    fs::write(&artifact, b"taskseal-artifact-v1").unwrap();

    let mutate = |path: &Path, expression: &str| {
        let result = Command::new("python3").args(["-c", expression, path.to_str().unwrap()]).status().unwrap();
        assert!(result.success());
    };
    mutate(&output.join("sbom.cdx.json"), "import json,sys; p=sys.argv[1]; x=json.load(open(p)); x['components']=[]; open(p,'w').write(json.dumps(x,separators=(',',':'))+'\\n')");
    assert!(!verify().status.success(), "component omission was accepted");

    assert!(run(&["generate", "--artifact", artifact.to_str().unwrap(), "--source-commit", "1111111111111111111111111111111111111111", "--target", "aarch64-apple-darwin", "--builder-id", "local://taskseal/p07", "--output", output.to_str().unwrap()]).status.success());
    mutate(&output.join("sbom.cdx.json"), "import json,sys; p=sys.argv[1]; x=json.load(open(p)); x['serialNumber']='not-a-urn'; open(p,'w').write(json.dumps(x,separators=(',',':'))+'\\n')");
    assert!(!verify().status.success(), "CycloneDX schema violation was accepted");

    assert!(run(&["generate", "--artifact", artifact.to_str().unwrap(), "--source-commit", "1111111111111111111111111111111111111111", "--target", "aarch64-apple-darwin", "--builder-id", "local://taskseal/p07", "--output", output.to_str().unwrap()]).status.success());
    mutate(&output.join("provenance.intoto.json"), "import json,sys; p=sys.argv[1]; x=json.load(open(p)); x['predicate']['runDetails']['builder']['id']='https://unknown.invalid'; open(p,'w').write(json.dumps(x,separators=(',',':'))+'\\n')");
    assert!(!verify().status.success(), "unknown builder was accepted");

    assert!(run(&["generate", "--artifact", artifact.to_str().unwrap(), "--source-commit", "1111111111111111111111111111111111111111", "--target", "aarch64-apple-darwin", "--builder-id", "local://taskseal/p07", "--output", output.to_str().unwrap()]).status.success());
    mutate(&output.join("provenance.intoto.json"), "import json,sys; p=sys.argv[1]; x=json.load(open(p)); del x['predicate']['buildDefinition']['buildType']; open(p,'w').write(json.dumps(x,separators=(',',':'))+'\\n')");
    assert!(!verify().status.success(), "incomplete SLSA provenance was accepted");

    fs::remove_dir_all(&temp).unwrap();
}
