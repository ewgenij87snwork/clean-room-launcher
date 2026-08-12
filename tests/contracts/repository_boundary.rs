use std::path::{Path, PathBuf};
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git must run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("git output is UTF-8").trim().to_owned()
}

#[test]
fn repository_is_an_independent_non_main_sibling() {
    let worktree = std::env::current_dir().expect("current worktree");
    let authority: serde_json::Value = serde_json::from_slice(
        &std::fs::read(".taskseal-dev/execution-authority.json").expect("private authority"),
    )
    .expect("valid authority JSON");
    let repository = PathBuf::from(authority["repository_realpath"].as_str().unwrap());
    let checkpoint = PathBuf::from(authority["plan_checkpoint_path"].as_str().unwrap());
    let owner_taskseal_root = checkpoint.parent().unwrap().parent().unwrap();

    assert_eq!(git(&worktree, &["rev-parse", "--show-toplevel"]), worktree.display().to_string());
    assert_ne!(git(&worktree, &["branch", "--show-current"]), "main");
    assert!(!repository.starts_with(owner_taskseal_root));
    assert!(!owner_taskseal_root.starts_with(&repository));
    assert_ne!(git(&worktree, &["rev-parse", "--git-common-dir"]), owner_taskseal_root.display().to_string());
}

#[test]
fn nested_and_main_boundaries_are_refused() {
    assert!(taskseal::contracts::repository::validate_boundary(
        Path::new("/tmp/taskseal"),
        Path::new("/tmp/wisdom"),
        "feat/p02-contracts",
        false,
    ).is_ok());
    assert!(taskseal::contracts::repository::validate_boundary(
        Path::new("/tmp/wisdom/taskseal"),
        Path::new("/tmp/wisdom"),
        "feat/p02-contracts",
        false,
    ).is_err());
    assert!(taskseal::contracts::repository::validate_boundary(
        Path::new("/tmp/taskseal"),
        Path::new("/tmp/wisdom"),
        "main",
        false,
    ).is_err());
    assert!(taskseal::contracts::repository::validate_boundary(
        Path::new("/tmp/taskseal"),
        Path::new("/tmp/wisdom"),
        "feat/p02-contracts",
        true,
    ).is_err());
}
