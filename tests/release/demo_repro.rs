use std::{fs, path::{Path, PathBuf}, process::Command};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }
fn sha256(path: &Path) -> String {
    let output = Command::new("shasum").args(["-a", "256", path.to_str().unwrap()]).output().unwrap();
    String::from_utf8(output.stdout).unwrap().split_whitespace().next().unwrap().to_owned()
}
fn fixture(name: &str, receipt: &str, output: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("p08-demo-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir); fs::create_dir_all(&dir).unwrap();
    let artifact = dir.join("candidate.tgz"); fs::write(&artifact, b"sealed candidate bytes\n").unwrap();
    fs::write(dir.join("fixture.json"), receipt).unwrap(); fs::write(dir.join("capture.json"), output).unwrap();
    (dir.clone(), artifact, dir.join("fixture.json"))
}
fn run(dir: &Path, artifact: &Path, receipt: &Path, capture: &Path, result: &Path) -> std::process::Output {
    Command::new(root().join("scripts/release/run-demo.sh")).args([
        "--artifact", artifact.to_str().unwrap(), "--artifact-sha256", "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33",
        "--fixture", receipt.to_str().unwrap(), "--fixture-sha256", &sha256(receipt),
        "--capture", capture.to_str().unwrap(), "--output", result.to_str().unwrap(), "--test-only-fixture",
    ]).env("TASKSEAL_TEST_ONLY_FIXTURE", "1").current_dir(dir).output().unwrap()
}
fn run_with_digest(dir: &Path, artifact: &Path, receipt: &Path, capture: &Path, result: &Path, digest: &str) -> std::process::Output {
    Command::new(root().join("scripts/release/run-demo.sh")).args([
        "--artifact", artifact.to_str().unwrap(), "--artifact-sha256", digest,
        "--fixture", receipt.to_str().unwrap(), "--fixture-sha256", &sha256(receipt),
        "--capture", capture.to_str().unwrap(), "--output", result.to_str().unwrap(), "--test-only-fixture",
    ]).env("TASKSEAL_TEST_ONLY_FIXTURE", "1").current_dir(dir).output().unwrap()
}
fn reproduced_clean_install() -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("p08-demo-real-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir); fs::create_dir_all(dir.join("source")).unwrap(); fs::create_dir_all(dir.join("target")).unwrap(); fs::create_dir_all(dir.join("out")).unwrap();
    assert!(Command::new("sh").args(["-c", "git archive --format=tar 01ad1d894aabe265b08d61d67d39da1a29cad9e4 | tar -xf - -C \"$1\"", "--", dir.join("source").to_str().unwrap()]).current_dir(root()).status().unwrap().success());
    let built = Command::new(dir.join("source/packaging/build-artifacts.sh")).arg(dir.join("out")).env("TASKSEAL_SOURCE_COMMIT", "01ad1d894aabe265b08d61d67d39da1a29cad9e4").env("CARGO_TARGET_DIR", dir.join("target")).env("CARGO_NET_OFFLINE", "true").env("LC_ALL", "C").env("TZ", "UTC").env("SOURCE_DATE_EPOCH", "0").current_dir(dir.join("source")).status().unwrap();
    assert!(built.success()); let artifact=fs::read_dir(dir.join("out")).unwrap().map(|e|e.unwrap().path()).find(|p|p.extension().is_some_and(|x|x=="gz")).unwrap();
    assert_eq!(sha256(&artifact), "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33");
    let install=dir.join("install"); fs::create_dir_all(&install).unwrap(); assert!(Command::new("tar").args(["-xzf", artifact.to_str().unwrap(), "-C", install.to_str().unwrap()]).status().unwrap().success());
    let installed=fs::read_dir(&install).unwrap().next().unwrap().unwrap().path().join("bin/tseal"); (dir, artifact, installed)
}

const FIXTURE: &str = r#"{"schema_version":"taskseal.demo-fixture.v1","mode":"TEST_ONLY_REPLAY","promotion_eligible":false,"p07_source_evidence":"reports/gates/p07/task-3.json#/claims/source_commit","p07_artifact_evidence":"reports/gates/p07/task-3.json#/claims/archive_sha256","p08_task3_contract":"reports/release/codex-alpha.json","claims":["compact_catalog","deferred_body"],"catalog_census":{"admitted":2,"loaded_now":0},"semantic_fields":["claims","catalog_census","result_digest"],"redaction":{"declared_non_semantic":["recorded_at"],"raw_prompt_retained":false,"credential_retained":false,"private_path_retained":false}}"#;
const CAPTURE: &str = r#"{"schema_version":"taskseal.demo-capture.v1","mode":"TEST_ONLY_REPLAY","promotion_eligible":false,"commands":["tseal catalog"],"results":["catalog: 2 skills"],"claims":["compact_catalog","deferred_body"],"catalog_census":{"admitted":2,"loaded_now":0},"cleanup":{"completed":true},"recorded_at":"REDACTED_NON_SEMANTIC"}"#;

#[test]
fn test_only_replay_is_closed_sanitized_and_reproducible_but_not_promotable() {
    let (dir, artifact, installed) = reproduced_clean_install();
    let fixture_path = dir.join("fixture.json"); let capture = dir.join("capture.json"); let output = dir.join("demo.json"); fs::write(&fixture_path, FIXTURE).unwrap(); fs::write(&capture, CAPTURE).unwrap();
    let first = Command::new(root().join("scripts/release/run-demo.sh")).args(["--artifact", artifact.to_str().unwrap(), "--artifact-sha256", "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33", "--installed-tseal", installed.to_str().unwrap(), "--fixture", fixture_path.to_str().unwrap(), "--fixture-sha256", &sha256(&fixture_path), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap(), "--test-only-fixture"]).env("TASKSEAL_TEST_ONLY_FIXTURE", "1").current_dir(&dir).output().unwrap();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert!(String::from_utf8_lossy(&first.stdout).contains("P08_DEMO_PREPARED_NOT_QUALIFIED"));
    let first_bytes = fs::read_to_string(&output).unwrap();
    assert!(first_bytes.contains("\"result\":\"PREPARED_NOT_QUALIFIED\""));
    assert!(first_bytes.contains("\"live_observation\":\"NOT_RUN\""));
    assert!(first_bytes.contains("\"promotion_eligible\":false"));
    let replay = Command::new(root().join("scripts/release/run-demo.sh")).args(["--artifact", artifact.to_str().unwrap(), "--artifact-sha256", "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33", "--installed-tseal", installed.to_str().unwrap(), "--fixture", fixture_path.to_str().unwrap(), "--fixture-sha256", &sha256(&fixture_path), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap(), "--test-only-fixture"]).env("TASKSEAL_TEST_ONLY_FIXTURE", "1").current_dir(&dir).output().unwrap();
    assert!(replay.status.success(), "{}", String::from_utf8_lossy(&replay.stderr));
    assert_eq!(first_bytes, fs::read_to_string(&output).unwrap());
    let verified = Command::new(root().join("scripts/release/run-demo.sh")).args(["--verify-output", output.to_str().unwrap()]).output().unwrap();
    assert!(verified.status.success());
    let mutate_schema = Command::new("ruby").args(["-rjson", "-rdigest", "-e", "x=JSON.parse(File.read(ARGV[0])); x[\"unexpected\"]=true; x.delete(\"output_sha256\"); x[\"output_sha256\"]=Digest::SHA256.hexdigest(JSON.generate(x)); File.write(ARGV[0], JSON.generate(x)+\"\\n\")", output.to_str().unwrap()]).status().unwrap();
    assert!(mutate_schema.success());
    let malformed_schema = Command::new(root().join("scripts/release/run-demo.sh")).args(["--verify-output", output.to_str().unwrap()]).output().unwrap();
    assert!(!malformed_schema.status.success()); assert!(String::from_utf8_lossy(&malformed_schema.stderr).contains("OUTPUT_SCHEMA"));
    fs::write(&output, first_bytes.replacen("PREPARED_NOT_QUALIFIED", "PASS", 1)).unwrap();
    let edited = Command::new(root().join("scripts/release/run-demo.sh")).args(["--verify-output", output.to_str().unwrap()]).output().unwrap();
    assert!(!edited.status.success()); assert!(String::from_utf8_lossy(&edited.stderr).contains("OUTPUT_SCHEMA"));
}

#[test]
fn arbitrary_artifact_digest_cannot_replace_committed_p07_digest() {
    let (dir, artifact, receipt) = fixture("p07-digest", FIXTURE, CAPTURE);
    let output = run_with_digest(&dir, &artifact, &receipt, &dir.join("capture.json"), &dir.join("out.json"), &sha256(&artifact));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("P07_ARTIFACT_DIGEST"));
}

#[test]
fn production_and_developer_checkout_are_never_demo_success_paths() {
    let (dir, artifact, receipt) = fixture("production", FIXTURE, CAPTURE);
    let production = Command::new(root().join("scripts/release/run-demo.sh")).args(["--artifact", artifact.to_str().unwrap(), "--artifact-sha256", &sha256(&artifact), "--fixture", receipt.to_str().unwrap(), "--fixture-sha256", &sha256(&receipt), "--capture", dir.join("capture.json").to_str().unwrap(), "--output", dir.join("out.json").to_str().unwrap()]).output().unwrap();
    assert!(!production.status.success()); assert!(String::from_utf8_lossy(&production.stderr).contains("PRODUCTION_NOT_RUN"));
    let checkout_artifact = root().join("reports/release/demo.json");
    let checkout = run(&dir, &checkout_artifact, &receipt, &dir.join("capture.json"), &dir.join("checkout.json"));
    assert!(!checkout.status.success()); assert!(String::from_utf8_lossy(&checkout.stderr).contains("DEVELOPER_CHECKOUT_REFUSED"));
}

#[test]
fn stale_wrong_unsafe_or_edited_demo_inputs_refuse() {
    for (name, fixture_body, capture_body, expected) in [
        ("wrong-fixture", FIXTURE.replacen("\"promotion_eligible\":false", "\"promotion_eligible\":true", 1), CAPTURE.to_owned(), "FIXTURE_NON_PROMOTABLE"),
        ("private-path", FIXTURE.to_owned(), CAPTURE.replacen("\"recorded_at\":\"REDACTED_NON_SEMANTIC\"", "\"private_path\":\"/Users/owner\",\"recorded_at\":\"REDACTED_NON_SEMANTIC\"", 1), "PRIVATE_PATH"),
        ("raw-prompt", FIXTURE.to_owned(), CAPTURE.replacen("\"commands\":[", "\"raw_prompt\":\"secret\",\"commands\":[", 1), "RAW_PROMPT"),
        ("credential", FIXTURE.to_owned(), CAPTURE.replacen("\"commands\":[", "\"credential\":\"secret\",\"commands\":[", 1), "CREDENTIAL"),
        ("missing-cleanup", FIXTURE.to_owned(), CAPTURE.replacen("\"completed\":true", "\"completed\":false", 1), "CLEANUP"),
        ("edited-output", FIXTURE.to_owned(), CAPTURE.replacen("catalog: 2 skills", "edited success", 1), "CAPTURE_RESULT"),
    ] {
        let (dir, artifact, receipt) = fixture(name, &fixture_body, &capture_body);
        let output = run(&dir, &artifact, &receipt, &dir.join("capture.json"), &dir.join("out.json"));
        assert!(!output.status.success(), "{name} accepted");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected), "{name}: {}", String::from_utf8_lossy(&output.stderr));
    }
}
