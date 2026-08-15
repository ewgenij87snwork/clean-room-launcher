use std::{env, fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }

fn temporary_dir() -> PathBuf {
    let path = env::temp_dir().join(format!("taskseal-live-matrix-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn run_shell(script: &str, artifact: &Path, checksum: &str, receipt: &Path) -> std::process::Output {
    Command::new("sh").arg(root().join(script)).arg("--artifact").arg(artifact).arg("--sha256").arg(checksum).arg("--receipt").arg(receipt).output().unwrap()
}

#[test]
fn wrong_checksum_refuses_before_any_os_lifecycle_or_receipt_write() {
    let temp = temporary_dir();
    let artifact = temp.join("taskseal-artifact.tgz");
    let receipt = temp.join("receipt.json");
    fs::write(&artifact, b"produced artifact fixture\n").unwrap();

    for script in ["scripts/release/live-macos.sh", "scripts/release/live-ubuntu.sh"] {
        let output = run_shell(script, &artifact, &"0".repeat(64), &receipt);
        assert!(!output.status.success(), "{script} accepted a wrong checksum");
        assert!(String::from_utf8_lossy(&output.stderr).contains("ARTIFACT_CHECKSUM_MISMATCH"));
        assert!(!receipt.exists(), "{script} wrote a receipt before checksum refusal");
    }
}

#[test]
fn locally_unqualified_lanes_emit_artifact_bound_not_qualified_receipts() {
    let temp = temporary_dir();
    let artifact = temp.join("taskseal-artifact.tgz");
    fs::write(&artifact, b"produced artifact fixture\n").unwrap();
    let checksum = "d1f1ed0e5319bcee0e8819698b8343e6eb30a18fdb0a0bacb9ca76dd9dbe0ba0";

    for (script, lane) in [("scripts/release/live-macos.sh", "macos"), ("scripts/release/live-ubuntu.sh", "ubuntu")] {
        let receipt = temp.join(format!("{lane}.json"));
        let output = run_shell(script, &artifact, checksum, &receipt);
        assert!(output.status.success(), "{script}: {}", String::from_utf8_lossy(&output.stderr));
        let source = fs::read_to_string(&receipt).unwrap();
        for required in [
            "\"schema_version\":\"taskseal.live-os-receipt.v1\"",
            &format!("\"lane\":\"{lane}\""),
            "\"qualification\":\"NOT_QUALIFIED\"",
            &format!("\"artifact_sha256\":\"{checksum}\""),
            "\"clean_image\":{", "\"prerequisites\":{", "\"protected_state_before\":{",
            "\"protected_state_after\":{", "\"config_sha256\":", "\"provider_sha256\":",
            "\"git_sha256\":", "\"user_files_sha256\":", "\"receipt_sha256\":"
        ] { assert!(source.contains(required), "{script} receipt missing {required}: {source}"); }
        assert!(!source.contains(&root().display().to_string()), "developer checkout leaked into receipt");
    }
}

#[test]
fn windows_harness_is_artifact_only_and_unavailable_execution_is_not_qualified() {
    let script = root().join("scripts/release/live-windows.ps1");
    assert!(script.is_file(), "Windows harness is missing");
    let source = fs::read_to_string(script).unwrap();
    for required in ["ARTIFACT_CHECKSUM_MISMATCH", "NOT_QUALIFIED", "artifact_sha256", "clean_image", "protected_state_before", "protected_state_after", "receipt_sha256"] {
        assert!(source.contains(required), "Windows harness missing contract {required}");
    }
    for forbidden in ["git clone", "cargo install", "CARGO_MANIFEST_DIR", "developer checkout"] {
        assert!(!source.contains(forbidden), "Windows harness has checkout fallback: {forbidden}");
    }
}
