use std::{env, fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }
fn temp() -> PathBuf { let p = env::temp_dir().join(format!("taskseal-live-matrix-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())); fs::create_dir_all(&p).unwrap(); p }
fn digest(n: char) -> String { n.to_string().repeat(64) }
fn run(script: &str, artifact: &Path, receipt: &Path, protected: &[(&str, String)]) -> std::process::Output {
    let checksum = "d1f1ed0e5319bcee0e8819698b8343e6eb30a18fdb0a0bacb9ca76dd9dbe0ba0";
    let mut cmd = Command::new("sh");
    cmd.arg(root().join(script)).args(["--artifact", artifact.to_str().unwrap(), "--sha256", checksum, "--receipt", receipt.to_str().unwrap()]);
    for (key, value) in protected { cmd.env(key, value); }
    cmd.output().unwrap()
}
fn complete_protected() -> Vec<(&'static str, String)> {
    [("TASKSEAL_CONFIG_SHA256", digest('a')), ("TASKSEAL_PROVIDER_SHA256", digest('b')), ("TASKSEAL_GIT_SHA256", digest('c')), ("TASKSEAL_USER_FILES_SHA256", digest('d')), ("TASKSEAL_CONFIG_SHA256_AFTER", digest('a')), ("TASKSEAL_PROVIDER_SHA256_AFTER", digest('b')), ("TASKSEAL_GIT_SHA256_AFTER", digest('c')), ("TASKSEAL_USER_FILES_SHA256_AFTER", digest('d'))].into()
}
fn artifact(temp: &Path) -> PathBuf { let p = temp.join("artifact.tgz"); fs::write(&p, b"produced artifact fixture\n").unwrap(); p }

#[test]
fn wrong_checksum_refuses_before_receipt() {
    let t = temp(); let a = artifact(&t); let r = t.join("receipt.json");
    for script in ["scripts/release/live-macos.sh", "scripts/release/live-ubuntu.sh"] {
        let output = Command::new("sh").arg(root().join(script)).args(["--artifact", a.to_str().unwrap(), "--sha256", &digest('0'), "--receipt", r.to_str().unwrap()]).output().unwrap();
        assert!(!output.status.success()); assert!(String::from_utf8_lossy(&output.stderr).contains("ARTIFACT_CHECKSUM_MISMATCH")); assert!(!r.exists());
    }
}

#[test]
fn missing_before_after_and_mismatch_refuse_without_receipts() {
    let t = temp(); let a = artifact(&t);
    for (label, values, expected) in [
        ("missing-before", Vec::new(), "MISSING_PROTECTED_STATE"),
        ("missing-after", complete_protected().into_iter().take(4).collect(), "MISSING_PROTECTED_STATE"),
        ("mismatch", { let mut v = complete_protected(); v[7].1 = digest('e'); v }, "PROTECTED_STATE_MISMATCH"),
    ] {
        for script in ["scripts/release/live-macos.sh", "scripts/release/live-ubuntu.sh"] {
            let r = t.join(format!("{label}-{script}.json")); let output = run(script, &a, &r, &values);
            assert!(!output.status.success(), "{script} accepted {label}"); assert!(String::from_utf8_lossy(&output.stderr).contains(expected)); assert!(!r.exists());
        }
    }
}

#[test]
fn invalid_prerequisite_digest_refuses_before_receipt() {
    let t = temp(); let a = artifact(&t);
    for prerequisite in ["not-a-digest", "\"invalid"] {
        for script in ["scripts/release/live-macos.sh", "scripts/release/live-ubuntu.sh"] {
            let r = t.join(format!("prerequisite-{script}.json")); let mut values = complete_protected(); values.push(("TASKSEAL_PREREQUISITES_SHA256", prerequisite.into()));
            let output = run(script, &a, &r, &values);
            assert!(!output.status.success()); assert!(String::from_utf8_lossy(&output.stderr).contains("INVALID_PREREQUISITES_SHA256")); assert!(!r.exists());
        }
    }
}

#[test]
fn receipts_parse_bind_distinct_captures_and_hash_their_payload() {
    let t = temp(); let a = artifact(&t); let values = complete_protected();
    for (script, lane) in [("scripts/release/live-macos.sh", "macos"), ("scripts/release/live-ubuntu.sh", "ubuntu")] {
        let r = t.join(format!("{lane}.json")); let output = run(script, &a, &r, &values); assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let check = Command::new("ruby").args(["-rjson", "-rdigest", "-e", "x=JSON.parse(File.read(ARGV[0])); h=x.delete('receipt_sha256'); abort unless x.dig('protected_state_before','config_sha256') == 'a'*64 && x.dig('protected_state_after','user_files_sha256') == 'd'*64 && h == Digest::SHA256.hexdigest(JSON.generate(x));", r.to_str().unwrap()]).output().unwrap();
        assert!(check.status.success(), "{lane}: {}", String::from_utf8_lossy(&check.stderr));
    }
}

#[test]
fn windows_contract_requires_before_after_and_checksum_refusal_without_claiming_execution() {
    let s = fs::read_to_string(root().join("scripts/release/live-windows.ps1")).unwrap();
    for required in ["ARTIFACT_CHECKSUM_MISMATCH", "MISSING_PROTECTED_STATE", "PROTECTED_STATE_MISMATCH", "INVALID_PREREQUISITES_SHA256", "TASKSEAL_PREREQUISITES_SHA256", "TASKSEAL_CONFIG_SHA256_AFTER", "protected_state_before", "protected_state_after", "receipt_sha256", "NOT_QUALIFIED"] { assert!(s.contains(required)); }
    for forbidden in ["git clone", "cargo install", "CARGO_MANIFEST_DIR", "developer checkout"] { assert!(!s.contains(forbidden)); }
}
