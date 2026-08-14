use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> String {
    fs::read_to_string(root().join(".github/workflows/release-candidate.yml"))
        .expect("release candidate workflow must exist")
}

#[test]
fn workflow_has_no_skip_or_unbound_required_gate() {
    let text = workflow();
    assert!(!text.contains("continue-on-error:"), "required workflow may not continue on error");
    assert!(!text.contains("if: false"), "required workflow may not conditionally skip gates");
    assert!(!text.contains("if: ${{"), "required gates may not be conditionally skipped");
    for gate in ["p02", "p03", "p04", "p05", "p06", "p07"] {
        assert!(text.contains(&format!("{gate}-gate")), "missing {} consolidated gate", gate);
        assert!(text.contains("TASKSEAL_SUBJECT_DIGEST"), "gate digest is not propagated");
    }
}

#[test]
fn source_verifier_closes_every_required_check() {
    let text = fs::read_to_string(root().join("scripts/release-build/verify-source.sh"))
        .expect("source verifier must exist");
    for check in ["fmt", "clippy", "test", "schema", "golden", "parity", "privacy", "dependency", "license"] {
        assert!(text.contains(check), "missing required check: {}", check);
    }
    assert!(text.contains("NOT_QUALIFIED"));
    assert!(text.contains("TASKSEAL_SUBJECT_DIGEST"));
    assert!(text.contains("P06"));
}

#[test]
fn poisoned_workflow_fixtures_are_rejected() {
    let source = fs::read_to_string(root().join("scripts/release-build/verify-source.sh")).unwrap();
    for fixture in ["continue-on-error", "conditional-skip", "missing-gate"] {
        let path = root().join(format!("tests/packaging/fixtures/{fixture}.yml"));
        let status = Command::new("sh")
            .arg(root().join("scripts/release-build/verify-source.sh"))
            .arg("--workflow")
            .arg(&path)
            .arg("--scaffold")
            .status()
            .expect("run verifier");
        assert!(!status.success(), "poisoned fixture {} was accepted", fixture);
    }
    assert!(source.contains("--workflow"));
}

#[test]
fn subject_is_exact_existing_checked_out_commit() {
    let script = root().join("scripts/release-build/verify-source.sh");
    for subject in [
        "abc",
        "0000000000000000000000000000000000000000",
        "ffffffffffffffffffffffffffffffffffffffff",
    ] {
        let status = Command::new("sh")
            .arg(&script)
            .arg("--workflow")
            .arg(root().join(".github/workflows/release-candidate.yml"))
            .arg("--subject-digest")
            .arg(subject)
            .arg("--scaffold")
            .status()
            .expect("run verifier");
        assert!(!status.success(), "invalid subject was accepted: {}", subject);
    }
}

#[test]
fn every_gate_is_attempted_and_missing_gate_is_not_qualified() {
    let output = Command::new("sh")
        .arg(root().join("scripts/release-build/verify-source.sh"))
        .arg("--gate-dir")
        .arg(root().join("tests/packaging/fixtures/gates"))
        .arg("--scaffold")
        .output()
        .expect("run fixture gate orchestrator");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    for gate in ["p02-gate", "p03-gate", "p04-gate", "p05-gate", "p06-gate", "p07-gate"] {
        assert!(stdout.contains(gate), "gate was not recorded: {}", gate);
    }
    assert!(stdout.contains("\"name\": \"p06-gate\""));
    assert!(stdout.contains("\"exit\": 127"));
    assert!(stdout.contains("\"status\": \"NOT_QUALIFIED\""));
    assert!(stdout.contains("\"name\": \"p02-gate\""));
    assert!(stdout.contains("\"exit\": 17"));
}
