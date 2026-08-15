use std::{env, fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }
fn temp() -> PathBuf {
    let path = env::temp_dir().join(format!("taskseal-codex-acceptance-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}
fn digest(byte: char) -> String { byte.to_string().repeat(64) }
fn write(path: &Path, bytes: &str) { fs::write(path, bytes).unwrap(); }
fn run(args: &[&str]) -> std::process::Output {
    Command::new("sh").arg(root().join("scripts/release/accept-codex.sh")).args(args).output().unwrap()
}

fn fixture() -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let dir = temp();
    let artifact = dir.join("taskseal-macos-aarch64.tgz"); write(&artifact, "artifact bytes\n");
    let p06 = dir.join("p06.json");
    write(&p06, &format!("{{\"schema_version\":\"taskseal.codex-state-preservation-receipt.v1\",\"task\":11,\"result\":\"accepted\",\"qualification\":\"NOT_QUALIFIED\",\"tuple\":{{\"provider_id\":\"codex\",\"artifact_digest\":\"{}\",\"version\":[0,147,0],\"os\":\"macos\",\"arch\":\"aarch64\"}},\"provider_launch\":false,\"protected_state_unchanged\":true}}", digest('a')));
    let p04 = dir.join("p04.json");
    write(&p04, "{\"schema_version\":\"taskseal.p04.acceptance-evidence.v1\",\"census\":{\"admitted\":2,\"loaded_now\":0},\"context_bytes\":{\"full_bodies_at_startup\":0}}");
    let capture = dir.join("capture.json");
    write(&capture, &format!("{{\"capture_mode\":\"DETERMINISTIC_FAKE\",\"terminal\":true,\"argv\":[\"tseal\",\"codex\",\"--safe\"],\"startup_context_sha256\":\"{}\",\"catalog\":{{\"needed_name_visible\":true,\"unused_body_present\":false,\"invoked_body_available\":true}},\"protected_before_sha256\":\"{}\",\"protected_after_sha256\":\"{}\",\"cleanup\":{{\"exit\":\"NOT_RUN\",\"relaunch\":\"NOT_RUN\",\"uninstall\":\"NOT_RUN\"}}}}", digest('b'), digest('c'), digest('c')));
    let output = dir.join("codex-alpha.json");
    (dir, artifact, p06, p04, capture, output)
}

#[test]
fn deterministic_capture_prepares_the_exact_live_action_without_qualifying_it() {
    let (_dir, artifact, p06, p04, capture, output) = fixture();
    let artifact_sha = Command::new("shasum").args(["-a", "256", artifact.to_str().unwrap()]).output().unwrap();
    let artifact_sha = String::from_utf8(artifact_sha.stdout).unwrap().split_whitespace().next().unwrap().to_owned();
    write(&p06, &fs::read_to_string(&p06).unwrap().replace(&digest('a'), &artifact_sha));
    let result = run(&["--fixture", "--artifact", artifact.to_str().unwrap(), "--artifact-sha256", &artifact_sha, "--p06", p06.to_str().unwrap(), "--p04", p04.to_str().unwrap(), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap()]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "P08_CODEX_ACCEPTANCE_PREPARED_NOT_QUALIFIED\n");
    let receipt = fs::read_to_string(output).unwrap();
    for required in [
        "\"schema_version\":\"taskseal.codex-clean-launch-acceptance.v1\"",
        "\"result\":\"PREPARED_NOT_QUALIFIED\"", "\"live_observation\":\"NOT_RUN\"",
        "\"reason\":\"OWNER_GATE_REQUIRED_PROVIDER_PROCESS\"", "\"capture_mode\":\"DETERMINISTIC_FAKE\"",
        "\"command\":[\"tseal\",\"codex\",\"--safe\"]", "\"unused_body_present\":false",
        "\"protected_mutation\":false", "\"cleanup\":{\"exit\":\"NOT_RUN\",\"relaunch\":\"NOT_RUN\",\"uninstall\":\"NOT_RUN\"}",
        "\"output_sha256\":\"",
    ] { assert!(receipt.contains(required), "missing {required}: {receipt}"); }
    for forbidden in ["artifact bytes", "/Users/", "raw_prompt", "credential", "private_path"] { assert!(!receipt.contains(forbidden), "retained sensitive data: {forbidden}"); }
}

#[test]
fn harness_refuses_non_terminal_and_unexpected_unused_body_without_output() {
    let (_dir, artifact, p06, p04, capture, output) = fixture();
    let artifact_sha = Command::new("shasum").args(["-a", "256", artifact.to_str().unwrap()]).output().unwrap();
    let artifact_sha = String::from_utf8(artifact_sha.stdout).unwrap().split_whitespace().next().unwrap().to_owned();
    write(&p06, &fs::read_to_string(&p06).unwrap().replace(&digest('a'), &artifact_sha));
    let source = fs::read_to_string(&capture).unwrap().replace("\"terminal\":true", "\"terminal\":false"); write(&capture, &source);
    let result = run(&["--fixture", "--artifact", artifact.to_str().unwrap(), "--artifact-sha256", &artifact_sha, "--p06", p06.to_str().unwrap(), "--p04", p04.to_str().unwrap(), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap()]);
    assert!(!result.status.success()); assert!(String::from_utf8_lossy(&result.stderr).contains("NON_TERMINAL_EXECUTION")); assert!(!output.exists());
    let source = fs::read_to_string(&capture).unwrap().replace("\"terminal\":false", "\"terminal\":true").replace("\"unused_body_present\":false", "\"unused_body_present\":true"); write(&capture, &source);
    let result = run(&["--fixture", "--artifact", artifact.to_str().unwrap(), "--artifact-sha256", &artifact_sha, "--p06", p06.to_str().unwrap(), "--p04", p04.to_str().unwrap(), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap()]);
    assert!(!result.status.success()); assert!(String::from_utf8_lossy(&result.stderr).contains("UNEXPECTED_BODY_VISIBILITY")); assert!(!output.exists());
}

#[test]
fn committed_alpha_receipt_is_explicitly_prepared_and_never_a_live_claim() {
    let receipt = fs::read_to_string(root().join("reports/release/codex-alpha.json")).expect("committed Codex alpha receipt is missing");
    for required in [
        "\"schema_version\":\"taskseal.codex-clean-launch-acceptance.v1\"",
        "\"result\":\"PREPARED_NOT_QUALIFIED\"", "\"live_observation\":\"NOT_RUN\"",
        "\"reason\":\"OWNER_GATE_REQUIRED_PROVIDER_PROCESS\"", "\"p06_receipt\":{",
        "\"p04_canary_evidence\":{", "\"live_action\":\"tseal codex <normal owner-selected safe args>\"",
    ] { assert!(receipt.contains(required), "missing {required}"); }
    for forbidden in ["raw_prompt", "credential", "/Users/", "provider_response"] { assert!(!receipt.contains(forbidden), "unsafe retained evidence: {forbidden}"); }
}

#[test]
fn harness_refuses_a_right_shaped_fabricated_p06_receipt() {
    let (_dir, artifact, p06, p04, capture, output) = fixture();
    let artifact_sha = Command::new("shasum").args(["-a", "256", artifact.to_str().unwrap()]).output().unwrap();
    let artifact_sha = String::from_utf8(artifact_sha.stdout).unwrap().split_whitespace().next().unwrap().to_owned();
    write(&p06, &fs::read_to_string(&p06).unwrap().replace(&digest('a'), &artifact_sha));
    let result = run(&["--artifact", artifact.to_str().unwrap(), "--artifact-sha256", &artifact_sha, "--p06", p06.to_str().unwrap(), "--p04", p04.to_str().unwrap(), "--capture", capture.to_str().unwrap(), "--output", output.to_str().unwrap()]);
    assert!(!result.status.success(), "fabricated right-shaped P06 evidence was accepted");
    assert!(String::from_utf8_lossy(&result.stderr).contains("P06_PIN_MISMATCH"));
    assert!(!output.exists());
}
