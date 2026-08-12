use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn guard(root: &Path) -> Output {
    Command::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/check-public-boundary.sh"))
        .arg("--root")
        .arg(root)
        .output()
        .expect("public boundary guard must exist and be executable")
}

#[test]
fn clean_public_inventory_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/public-boundary/clean");
    let output = guard(&root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = guard(repository);
    assert!(
        output.status.success(),
        "real repository: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn poisoned_public_inventory_fails_with_a_stable_reason() {
    let fixtures = [
        ("praxis-role", "PRIVATE_PRAXIS_ROLE"),
        ("home-path", "ABSOLUTE_HOME_PATH"),
        ("credential", "CREDENTIAL_TOKEN"),
        ("transcript", "TRANSCRIPT_FRAGMENT"),
    ];
    for (fixture, reason) in fixtures {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/public-boundary")
            .join(fixture);
        let output = guard(&root);
        assert!(!output.status.success(), "{fixture} unexpectedly passed");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            reason,
            "{fixture}"
        );
    }

    let symlink_root = std::env::temp_dir().join(format!(
        "taskseal-public-symlink-test-{}",
        std::process::id()
    ));
    std::fs::create_dir(&symlink_root).expect("create symlink fixture root");
    std::os::unix::fs::symlink("/etc/hosts", symlink_root.join("README.md"))
        .expect("create symlink fixture");
    let output = guard(&symlink_root);
    std::fs::remove_dir_all(&symlink_root).expect("remove symlink fixture root");
    assert!(
        !output.status.success(),
        "symlink escape unexpectedly passed"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "SYMLINK_ESCAPE"
    );
}

#[test]
fn accepted_p05_surface_is_allowlisted_but_adjacent_surface_is_not() {
    let root = std::env::temp_dir().join(format!("taskseal-p05-public-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("fixtures/cli")).unwrap();
    std::fs::write(root.join("fixtures/cli/fake-provider.rs"), "fn main() {}\n").unwrap();
    assert!(guard(&root).status.success());
    std::fs::create_dir_all(root.join("fixtures/cli-extra")).unwrap();
    std::fs::write(root.join("fixtures/cli-extra/unowned.rs"), "fn main() {}\n").unwrap();
    let output = guard(&root);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "UNALLOWLISTED_PUBLIC_PATH:fixtures/cli-extra/unowned.rs"
    );
}
