use std::{fs, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}};

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
    fs::create_dir_all(temp.join("reports/release")).unwrap();
    for plan in ["p02", "p03", "p04", "p05", "p06", "p07"] {
        copy_tree(&root().join("reports/gates").join(plan), &temp.join("reports/gates").join(plan));
    }
    fs::copy(
        root().join("reports/release/codex-live-observation.json"),
        temp.join("reports/release/codex-live-observation.json"),
    ).unwrap();
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
        "\"candidate_commit\":\"c6f9d8f1cfa65c63bf34595eee056ff5d9202b25\"",
        "\"evidence_path\":\"reports/gates/p06/privacy-release-boundary-v1.json\"",
        "\"evidence_path\":\"reports/gates/p07/legal-candidate-correction-v1.json\"",
        "\"evidence_path\":\"reports/gates/p07/artifact-privacy-correction-v1.json\"",
        "\"evidence_path\":\"reports/release/codex-live-observation.json\"",
        "\"evidence_pointer\":\"/artifact/sha256\"",
        "\"sha256\":\"49a685a98df13fa59766d9032dca7229cfa3e996b1b9b92ec59be8da1b899cf7\"",
        "\"status\":\"CLOSED\"",
        "\"status\":\"BLOCKER\"",
    ] { assert!(candidate.contains(needle), "missing {needle}"); }
    for stale in [
        "656f8701e84e0d7a72c4dbdb62d8ad20733e5743b602ff0fd6447c711a211d33",
        "ea8e60d2b4097ce766758bd70543628d0c15e9c7ab0ebc0d29d76c59da896b0c",
        "\"evidence_path\":\"reports/gates/p07/terminal-review.json\"",
    ] { assert!(!candidate.contains(stale), "stale candidate input retained: {stale}"); }
    for forbidden in ["/Users/", "/home/", "ghp_", "sk-12345678901234567890", "raw_prompt", "prompt_payload"] {
        assert!(!candidate.contains(forbidden), "private data leaked: {forbidden}");
    }
    let audit = fs::read_to_string(root().join("scripts/release/audit-release.sh")).unwrap();
    for needle in [
        "qualification/public-release-inventory-v1.json",
        "IMMUTABLE_INTERNAL_EVIDENCE_EXCLUDED_FROM_PUBLIC_SOURCE",
        "packaging/license-policy.toml",
        "--offline\", \"check\", \"licenses",
    ] { assert!(audit.contains(needle), "audit does not bind corrected input: {needle}"); }

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

    let duplicate = fixture("duplicate");
    let path = duplicate.join("reports/gates/p07/supply-chain-gate.json");
    fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"head\":", "\"head\":\"duplicate\",\"head\":")).unwrap();
    let result = collect(&duplicate);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("DUPLICATE_JSON_KEY"));

    for (label, relative) in [
        ("P06_PRIVACY", "reports/gates/p06/privacy-release-boundary-v1.json"),
        ("P07_LEGAL_CORRECTION", "reports/gates/p07/legal-candidate-correction-v1.json"),
        ("P07_ARTIFACT_PRIVACY", "reports/gates/p07/artifact-privacy-correction-v1.json"),
        ("CODEX_LIVE_OBSERVATION", "reports/release/codex-live-observation.json"),
    ] {
      for mutation in ["missing", "stale", "private", "duplicate", "unsupported"] {
        let name = format!("support-{label}-{mutation}");
        let support = fixture(&name);
        let path = support.join(relative);
        match mutation {
            "missing" => fs::remove_file(&path).unwrap(),
            "stale" => fs::write(&path, format!("{}\n", fs::read_to_string(&path).unwrap())).unwrap(),
            "private" => fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"schema_version\":", "\"private\":\"/Users/owner\",\"schema_version\":")).unwrap(),
            "duplicate" => fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"schema_version\":", "\"schema_version\":\"duplicate\",\"schema_version\":")).unwrap(),
            "unsupported" => fs::write(&path, fs::read_to_string(&path).unwrap().replace("\"schema_version\":", "\"unsupported\":true,\"schema_version\":")).unwrap(),
            _ => unreachable!(),
        }
        let result = collect(&support);
        let expected = match mutation {
            "missing" => format!("EVIDENCE_MISSING:{label}"),
            "private" => format!("PRIVATE_DATA:{label}"),
            "duplicate" => "DUPLICATE_JSON_KEY".to_owned(),
            _ => format!("EVIDENCE_STALE:{label}"),
        };
        assert!(!result.status.success(), "support evidence {name} was accepted");
        assert!(String::from_utf8_lossy(&result.stderr).contains(&expected), "{}", String::from_utf8_lossy(&result.stderr));
        assert!(!support.join("candidate.json").exists(), "refused support evidence wrote a dossier");
      }
    }
}

fn git(args: &[&str]) -> String {
    let output = Command::new("git").current_dir(root()).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn json_string(source: &str, key: &str) -> String {
    let marker = format!("\"{key}\":\"");
    source.split(&marker).nth(1).unwrap().split('"').next().unwrap().to_owned()
}

fn sha256_at_commit(commit: &str, file: &str) -> String {
    let bytes = Command::new("git").current_dir(root()).args(["show", &format!("{commit}:{file}")]).output().unwrap();
    assert!(bytes.status.success(), "{}", String::from_utf8_lossy(&bytes.stderr));
    let mut digest = Command::new("shasum").args(["-a", "256"]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    digest.stdin.as_mut().unwrap().write_all(&bytes.stdout).unwrap();
    let output = digest.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().split_whitespace().next().unwrap().to_owned()
}

fn validate_subject_sha256(source: &str, implementation: &str) -> Result<(), String> {
    let section = source.split("\"subject_sha256\":{").nth(1).ok_or("missing subject_sha256")?;
    for file in ["reports/release/candidate.json", "schemas/release/release-dossier.schema.json", "scripts/release/collect-dossier.rb", "tests/release/dossier.rs"] {
        let expected = json_string(section, file);
        let actual = sha256_at_commit(implementation, file);
        if actual != expected { return Err(format!("subject_sha256 mismatch: {file}")); }
    }
    Ok(())
}

#[test]
fn task_one_receipt_is_a_durable_historical_receipt_that_allows_continuation() {
    const RECEIPT: &str = "reports/gates/p08/task-1.json";
    let path = root().join(RECEIPT);
    let source = fs::read_to_string(&path).expect("Task 1 receipt is missing");
    for needle in [
        "\"schema_version\":\"taskseal.p08.task-receipt.v1\"",
        "\"plan_id\":\"P08\"", "\"task\":1", "\"acceptance_id\":\"ACC-P08-T1\"",
        "\"input_head\":\"487f4105cc56bc147783a12e30dd7c1338716284\"",
        "\"receipt_seal_role\":\"receipt-only-child\"",
    ] { assert!(source.contains(needle), "missing receipt field: {needle}"); }
    let implementation = json_string(&source, "implementation_head");
    let parent = json_string(&source, "receipt_commit_parent");
    assert_eq!(implementation, parent);
    for ancestor in ["04df6e065d569c7d3169df1adb8070c23eab57b4", "04230aa9e6a030109e235a266b0484e4ab2779d5", &implementation] {
        assert_eq!(git(&["merge-base", "--is-ancestor", ancestor, "HEAD"]), "");
    }
    let receipts = git(&["rev-list", "--reverse", &format!("{implementation}..HEAD"), "--", RECEIPT]);
    let receipt_commit = receipts.lines().collect::<Vec<_>>();
    assert_eq!(receipt_commit.len(), 1, "receipt must have one seal child: {receipts}");
    assert_eq!(git(&["rev-list", "--parents", "-n", "1", receipt_commit[0]]), format!("{} {}", receipt_commit[0], implementation));
    assert_eq!(git(&["diff-tree", "--no-commit-id", "--name-only", "-r", receipt_commit[0]]), RECEIPT);
    validate_subject_sha256(&source, &implementation).unwrap();
    let expected = json_string(source.split("\"subject_sha256\":{").nth(1).unwrap(), "reports/release/candidate.json");
    let tampered = source.replacen(&expected, &"0".repeat(64), 1);
    assert!(validate_subject_sha256(&tampered, &implementation).is_err(), "self-consistent subject_sha256 mutation was accepted");
}
