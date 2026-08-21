use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn shell_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            shell_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "sh") {
            files.push(path);
        }
    }
}

#[test]
fn every_task7_forbidden_class_is_detected_by_the_release_audit() {
    let script = root().join("scripts/release/audit-release.sh");
    assert!(script.is_file(), "Task 7 release audit is missing");

    let output_dir = std::env::temp_dir().join(format!(
        "taskseal-p08-audit-fixtures-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&output_dir);
    fs::create_dir_all(&output_dir).unwrap();
    let report = output_dir.join("audit.json");

    let result = Command::new(&script)
        .current_dir(root())
        .args([
            "--fixture-root",
            root()
                .join("tests/release/audit_fixtures/corpus")
                .to_str()
                .unwrap(),
            "--output",
            report.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "fixture audit failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "P08_TASK7_FIXTURE_DETECTIONS_PASS classes=6 blockers=15\n"
    );

    let report = fs::read_to_string(report).unwrap();
    for blocker in [
        "PRIVATE_SECRET",
        "PRIVATE_ABSOLUTE_PATH",
        "UNSAFE_PROCESS_SHELL",
        "UNSAFE_FILESYSTEM_ROOT_DELETE",
        "DEPENDENCY_LICENSE_NOTICE_MISSING",
        "DEPENDENCY_ADVISORY",
        "NAMESPACE_OWNERSHIP_UNPROVEN",
        "UNSUPPORTED_RELEASE_CLAIM",
        "SECURITY_REPORTING_ROUTE_MISSING",
        "BOUNTY_STATUS_MISSING",
        "THREAT_MODEL_INCOMPLETE",
        "PROTECTED_PATH_OWNERSHIP_MISSING",
        "TELEMETRY_OR_BACKEND_PRESENT",
        "PUBLIC_PROSE_NOT_ENGLISH",
        "PUBLIC_RELEASE_COPY_MISSING",
    ] {
        assert!(report.contains(blocker), "missing blocker {blocker}");
    }

    let correction_proof = "\"scanner_corrections\":{\"official_schema\":{\"accepted\":0,\"filtered\":2,\"raw\":2},\"shellcheck\":{\"accepted\":0,\"filtered_intentional_empty_assignments\":26,\"raw\":26}}";
    let correction_proven = report.contains(correction_proof);

    let mut checked_shell_files = Vec::new();
    shell_files(&root().join("scripts/release"), &mut checked_shell_files);
    shell_files(&root().join("packaging"), &mut checked_shell_files);
    checked_shell_files.sort();
    let shellcheck = Command::new("shellcheck")
        .args(["--format=json", "--severity=warning"])
        .args(&checked_shell_files)
        .output()
        .unwrap();
    assert!(matches!(shellcheck.status.code(), Some(0 | 1)));
    let shellcheck_json = String::from_utf8(shellcheck.stdout).unwrap();
    let has_sc2046 = shellcheck_json.contains("\"code\":2046")
        || shellcheck_json.contains("\"code\": 2046");
    assert!(
        correction_proven && !has_sc2046,
        "scanner correction proof present={correction_proven}; shellcheck={shellcheck_json}"
    );

    fs::remove_dir_all(output_dir).unwrap();
}
