use std::{fs, path::{Path, PathBuf}, process::Command};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() { copy_tree(&entry.path(), &target); }
        else { fs::copy(entry.path(), target).unwrap(); }
    }
}

fn fixture(name: &str) -> PathBuf {
    let temp = std::env::temp_dir().join(format!("p08-dossier-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(temp.join("reports/gates")).unwrap();
    for plan in ["p02", "p03", "p04", "p05", "p06", "p07"] {
        copy_tree(&root().join("reports/gates").join(plan), &temp.join("reports/gates").join(plan));
    }
    temp
}

fn collect(fixture: &Path) -> std::process::Output {
    Command::new(root().join("scripts/release/collect-dossier.rb"))
        .args(["collect", "--root", fixture.to_str().unwrap(), "--output", fixture.join("candidate.json").to_str().unwrap(), "--requested-state", "PRIVATE_CANDIDATE"])
        .output().unwrap()
}

#[test]
fn collector_builds_a_closed_private_candidate_and_refuses_bad_receipts() {
    let valid = fixture("valid");
    let result = collect(&valid);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "P08_DOSSIER_COLLECTED state=PRIVATE_CANDIDATE qualification=NOT_QUALIFIED\n");
    let candidate = fs::read_to_string(valid.join("candidate.json")).unwrap();
    for needle in [
        "\"schema_version\":\"taskseal.release-dossier.v1\"",
        "\"requested_release_state\":\"PRIVATE_CANDIDATE\"",
        "\"plan\":\"P02\"", "\"plan\":\"P07\"",
        "\"qualification\":\"NOT_QUALIFIED\"",
        "\"evidence_path\":\"reports/gates/p07/task-3.json\"",
        "\"evidence_pointer\":\"/claims/archive_sha256\"",
    ] { assert!(candidate.contains(needle), "missing {needle}"); }
    for forbidden in ["/Users/", "/home/", "ghp_", "sk-12345678901234567890", "raw_prompt", "prompt_payload"] {
        assert!(!candidate.contains(forbidden), "private data leaked: {forbidden}");
    }

    let missing = fixture("missing");
    fs::remove_file(missing.join("reports/gates/p06/qualification-gate.json")).unwrap();
    let result = collect(&missing);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("RECEIPT_MISSING:P06"));

    let stale = fixture("stale");
    let path = stale.join("reports/gates/p06/qualification-gate.json");
    fs::write(&path, format!("{}\n", fs::read_to_string(&path).unwrap())).unwrap();
    let result = collect(&stale);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("RECEIPT_STALE:P06"));

    let wrong_digest = fixture("wrong-digest");
    let path = wrong_digest.join("reports/gates/p07/task-7.json");
    fs::write(&path, format!("{}\n", fs::read_to_string(&path).unwrap())).unwrap();
    let result = collect(&wrong_digest);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("RECEIPT_DIGEST:P07_TASK_7"));

    let duplicate = fixture("duplicate");
    let path = duplicate.join("reports/gates/p07/supply-chain-gate.json");
    fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"head\":", "\"head\":\"duplicate\",\"head\":")).unwrap();
    let result = collect(&duplicate);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("DUPLICATE_JSON_KEY"));
}
