use std::path::{Path, PathBuf};
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git must run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

#[test]
fn repository_context_matches_public_or_private_boundary_contract() {
    let worktree = std::env::current_dir().expect("current worktree");
    assert_eq!(
        git(&worktree, &["rev-parse", "--show-toplevel"]),
        worktree.display().to_string()
    );

    if !Path::new("AGENTS.md").exists() {
        // Public CI intentionally runs on main. The non-main execution-worktree
        // invariant belongs to the private authority-backed path below.
        let inventory: serde_json::Value = serde_json::from_slice(
            &std::fs::read("qualification/public-release-inventory-v1.json")
                .expect("public release inventory exists"),
        )
        .expect("public release inventory parses");
        assert!(
            inventory["excluded_internal_paths"]
                .as_array()
                .expect("excluded internal paths are listed")
                .iter()
                .any(|path| path == ".clroom-dev")
        );
        return;
    }

    let authority: serde_json::Value = serde_json::from_slice(
        &std::fs::read(".clroom-dev/execution-authority.json").expect("private authority"),
    )
    .expect("valid authority JSON");
    let repository = PathBuf::from(authority["repository_realpath"].as_str().unwrap());
    let checkpoint = PathBuf::from(authority["plan_checkpoint_path"].as_str().unwrap());
    let owner_clroom_root = checkpoint.parent().unwrap().parent().unwrap();

    assert_ne!(git(&worktree, &["branch", "--show-current"]), "main");
    assert!(!repository.starts_with(owner_clroom_root));
    assert!(!owner_clroom_root.starts_with(&repository));
    assert_ne!(
        git(&worktree, &["rev-parse", "--git-common-dir"]),
        owner_clroom_root.display().to_string()
    );
}

#[test]
fn nested_and_main_boundaries_are_refused() {
    assert!(
        clroom::contracts::repository::validate_boundary(
            Path::new("/tmp/clroom"),
            Path::new("/tmp/wisdom"),
            "feat/p02-contracts",
            false,
        )
        .is_ok()
    );
    assert!(
        clroom::contracts::repository::validate_boundary(
            Path::new("/tmp/wisdom/clroom"),
            Path::new("/tmp/wisdom"),
            "feat/p02-contracts",
            false,
        )
        .is_err()
    );
    assert!(
        clroom::contracts::repository::validate_boundary(
            Path::new("/tmp/clroom"),
            Path::new("/tmp/wisdom"),
            "main",
            false,
        )
        .is_err()
    );
    assert!(
        clroom::contracts::repository::validate_boundary(
            Path::new("/tmp/clroom"),
            Path::new("/tmp/wisdom"),
            "feat/p02-contracts",
            true,
        )
        .is_err()
    );
}
