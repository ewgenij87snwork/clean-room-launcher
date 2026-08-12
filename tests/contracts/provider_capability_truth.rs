use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static PROBE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn repository_root() -> PathBuf {
    std::env::current_dir().expect("test runs from repository root")
}

fn run_probe(extra_args: &[&str]) -> std::process::Output {
    let root = repository_root();
    let sequence = PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_root = std::env::temp_dir().join(format!(
        "taskseal-provider-test-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&temp_root).expect("create isolated probe temp root");
    let output = Command::new(root.join("scripts/probe/provider-capabilities.sh"))
        .arg("--root")
        .arg(&root)
        .args(extra_args)
        .env("TMPDIR", &temp_root)
        .output()
        .expect("provider capability probe must be executable");
    let residue = std::fs::read_dir(&temp_root)
        .expect("read isolated probe temp root")
        .count();
    std::fs::remove_dir_all(&temp_root).expect("remove isolated test temp root");
    assert_eq!(residue, 0, "provider probe left temporary state behind");
    output
}

#[test]
fn codex_fixture_produces_closed_capability_truth_without_a_clean_overclaim() {
    let output = run_probe(&["--provider", "codex", "--fixture", "qualified-home"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
    for required in [
        "\"executable_digest\"",
        "\"version\"",
        "\"discovery_roots\"",
        "\"native_metadata_lifecycle\"",
        "\"runtime_filter\"",
        "\"auth_dependencies\"",
        "\"ambient_sources\"",
        "\"projection_candidate\"",
        "\"state\"",
    ] {
        assert!(
            report.contains(required),
            "missing field {required}: {report}"
        );
    }
    assert!(report.contains("\"state\":\"narrowed\""), "{report}");
    assert!(
        report.contains("\"metadata_at_start\":\"qualified\""),
        "{report}"
    );
    assert!(
        report.contains("\"body_on_invocation\":\"unsupported\""),
        "{report}"
    );
    assert!(
        report.contains("\"projection_candidate\":false"),
        "boolean field encoded incorrectly: {report}"
    );
    assert!(
        report.contains("\"persistent_state_unchanged\":true"),
        "{report}"
    );
    assert!(
        !report.contains("TASKSEAL_CANARY_BODY_7E5B1E21"),
        "body leaked: {report}"
    );
}

#[test]
fn absent_native_isolation_refuses_a_requested_clean_claim() {
    let output = run_probe(&[
        "--provider",
        "codex",
        "--fixture",
        "no-native-isolation",
        "--require-clean-claim",
    ]);
    assert!(
        !output.status.success(),
        "unsupported clean claim was accepted"
    );
    let error = String::from_utf8(output.stderr).expect("probe error is UTF-8");
    assert!(error.contains("UNSUPPORTED_CLEAN_CLAIM"), "{error}");
}

#[test]
fn wrong_version_and_poisoned_ambient_source_cannot_qualify() {
    for fixture in ["wrong-version", "poisoned-home"] {
        let output = run_probe(&["--provider", "codex", "--fixture", fixture]);
        assert!(
            output.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
        assert!(
            report.contains("\"state\":\"unsupported\"")
                || report.contains("\"state\":\"narrowed\""),
            "{fixture}: {report}"
        );
        assert!(
            !report.contains("TASKSEAL_POISON_BODY_933BF642"),
            "{fixture}: body leaked"
        );
    }
}

#[test]
fn claude_evidence_is_no_spend_and_never_runtime_qualified() {
    let output = run_probe(&["--provider", "claude", "--fixture", "no-spend"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
    assert!(report.contains("\"provider\":\"claude\""), "{report}");
    assert!(report.contains("\"state\":\"unsupported\""), "{report}");
    assert!(report.contains("\"model_invoked\":false"), "{report}");
}
