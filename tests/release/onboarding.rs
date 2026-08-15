use std::{fs, path::{Path, PathBuf}, process::Command};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

fn fixture(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("p08-onboarding-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("onboarding.json"), body).unwrap();
    path
}

fn check(path: &Path) -> std::process::Output {
    Command::new(root().join("scripts/release/check-onboarding.rb"))
        .args(["check", "--report", path.join("onboarding.json").to_str().unwrap()])
        .output().unwrap()
}

const VALID: &str = r#"{
  "schema_version":"taskseal.onboarding-readiness.v1",
  "result":"PREPARED_NOT_QUALIFIED",
  "internal_fixture":{"kind":"DETERMINISTIC_STATE_MACHINE","result":"PASS","active_seconds":42,"user_wait_seconds":11,"help_events":0,"error_events":0,"states":["ARTIFACT_RECEIVED","DIGEST_VERIFIED","PUBLISHED_DOCS_ONLY","ONE_COMMAND_STARTED","CLEAN_CODEX_VERIFIED","CLEANUP_VERIFIED"]},
  "external_observation":{"status":"NOT_RUN","reason":"OWNER_GATE_REQUIRED_EXTERNAL_USER"},
  "setup_time_claim":{"status":"UNAVAILABLE","reason":"NO_OBSERVED_DISTRIBUTION"},
  "comprehension_checks":{"changed":"REQUIRED_NOT_OBSERVED","unchanged":"REQUIRED_NOT_OBSERVED","exit":"REQUIRED_NOT_OBSERVED","rollback":"REQUIRED_NOT_OBSERVED"},
  "privacy":{"sanitized":true,"raw_user_data_retained":false},
  "cleanup":{"required":true,"verified_by_fixture":true}
}"#;

#[test]
fn checker_accepts_only_a_closed_sanitized_internal_readiness_fixture() {
    let valid = fixture("valid", VALID);
    let output = check(&valid);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "P08_ONBOARDING_READY internal=PASS external=NOT_RUN setup_time=UNAVAILABLE\n");

    for (name, from, to, refusal) in [
        ("missing-digest", "\"DIGEST_VERIFIED\",", "", "ARTIFACT_OR_DIGEST"),
        ("collapsed-time", "\"user_wait_seconds\":11", "\"user_wait_seconds\":0", "TIME_COLLAPSED"),
        ("coaching", "\"PUBLISHED_DOCS_ONLY\"", "\"COACHED\"", "COACHING"),
        ("private", "\"sanitized\":true", "\"private_path\":\"/Users/owner\",\"sanitized\":true", "PRIVATE_DATA"),
        ("unsupported-tuple", "\"CLEAN_CODEX_VERIFIED\"", "\"UNSUPPORTED_TUPLE\"", "UNSUPPORTED_TUPLE"),
        ("missing-cleanup", "\"verified_by_fixture\":true", "\"verified_by_fixture\":false", "CLEANUP"),
        ("fake-human", "\"kind\":\"DETERMINISTIC_STATE_MACHINE\"", "\"kind\":\"HUMAN_OBSERVATION\"", "FAKE_HUMAN_PROMOTION"),
        ("marketing-copy", "\"status\":\"UNAVAILABLE\"", "\"status\":\"BEST_RUN_42_SECONDS\"", "SETUP_TIME_CLAIM"),
    ] {
        let case = fixture(name, &VALID.replacen(from, to, 1));
        let output = check(&case);
        assert!(!output.status.success(), "{name} was accepted");
        assert!(String::from_utf8_lossy(&output.stderr).contains(refusal), "{}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn protocol_states_the_unassisted_boundary_and_comprehension_without_human_claims() {
    let protocol = fs::read_to_string(root().join("tests/release/onboarding-protocol.md")).expect("onboarding protocol exists");
    for required in [
        "checksum-bound artifact/link receipt", "verified clean Codex start", "published docs", "one-command path",
        "active time", "user waiting", "help", "errors", "what changed", "what did not", "exit", "rollback",
        "OWNER_GATE_REQUIRED_EXTERNAL_USER", "NOT_RUN", "not a human observation",
    ] { assert!(protocol.contains(required), "protocol missing {required}"); }
}
