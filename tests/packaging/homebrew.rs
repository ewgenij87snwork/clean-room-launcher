use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

#[test]
fn lifecycle_fake_contract_covers_isolation_and_transitions() {
    let temp = std::env::temp_dir().join(format!("p07-homebrew-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let result = temp.join("result.json");
    let output = Command::new("python3")
        .current_dir(root())
        .args(["packaging/homebrew/lifecycle.py", "--fake", "--workspace", temp.to_str().unwrap(), "--output", result.to_str().unwrap()])
        .output()
        .expect("lifecycle executable");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json = fs::read_to_string(&result).unwrap();
    for needle in ["taskseal.p07.homebrew-lifecycle.v1", "install_n", "upgrade_n_plus_1", "rollback_n", "cleanup_complete", "LIVE_HOMEBREW_BOUNDARY_REFUSED"] {
        assert!(json.contains(needle), "missing {needle}");
    }
    assert!(!json.contains("/Users/"), "owner path leaked");
    assert!(!json.contains("raw_output"), "raw process output leaked");
    println!("P07_HOMEBREW_LIFECYCLE_TEST_PASS");
}
