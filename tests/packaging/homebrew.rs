use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

fn run(name: &str, injected: Option<&str>) -> (bool, String, String) {
    let temp = std::env::temp_dir().join(format!("p07-homebrew-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let result = temp.join("result.json");
    let fake = root().join("tests/packaging/fixtures/homebrew/fake_brew.py");
    let mut command = Command::new("python3");
    command.current_dir(root()).args(["packaging/homebrew/lifecycle.py", "--fake", "--fake-brew", fake.to_str().unwrap(), "--workspace", temp.to_str().unwrap(), "--output", result.to_str().unwrap()]);
    if let Some(step) = injected { command.args(["--inject-failure", step]); }
    let output = command.output().expect("lifecycle executable");
    (output.status.success(), fs::read_to_string(result).unwrap(), fs::read_to_string(temp.join("ledger.jsonl")).unwrap_or_default())
}

#[test]
fn fake_lifecycle_executes_each_approved_transition_and_cleans_up() {
    let (ok, json, ledger) = run("success", None);
    assert!(ok, "{}", json);
    for needle in ["tap", "item_trust", "style", "audit", "install_n", "upgrade_n_plus_1", "rollback_n", "unlink", "link", "uninstall", "untrust", "untap"] { assert!(json.contains(needle), "missing step {}", needle); }
    for command in ["[\"tap\"", "[\"trust\",\"--formula\"", "[\"upgrade\"", "[\"unlink\"", "[\"link\"", "[\"untrust\",\"--formula\""] { assert!(ledger.contains(command), "missing ledger command {}: {}", command, ledger); }
    assert!(json.contains("\"cleanup_complete\":true"));
    assert!(!json.contains("raw_output") && !json.contains("/Users/"));
}

#[test]
fn fake_lifecycle_runs_finally_cleanup_after_injected_failure() {
    let (ok, json, ledger) = run("failure", Some("upgrade"));
    assert!(!ok);
    assert!(json.contains("\"failure_class\":\"UPGRADE_REFUSED\""));
    assert!(json.contains("\"cleanup_complete\":true"));
    assert!(ledger.contains("[\"untrust\",\"--formula\"") && ledger.contains("[\"untap\""));
}
