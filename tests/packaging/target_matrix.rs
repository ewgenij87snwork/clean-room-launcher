//! Focused contract tests for the P07 release target declaration.
//!
//! This test intentionally uses only the standard library so it can be run
//! before the workspace has a packaging parser dependency.

use std::{collections::BTreeMap, fs, path::Path};

fn read(path: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(Path::new(root).join(path)).expect("packaging fixture exists")
}

fn target_blocks(toml: &str) -> Vec<BTreeMap<String, String>> {
    toml.split("[[targets]]").skip(1).map(|block| {
        block.lines().filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"').to_owned();
            Some((key.trim().to_owned(), value))
        }).collect()
    }).collect()
}

#[test]
fn matrix_declares_complete_target_interface_and_honest_claims() {
    let matrix = read("packaging/targets.toml");
    let blocks = target_blocks(&matrix);
    assert!(!blocks.is_empty(), "at least the alpha target is required");

    for target in &blocks {
        for field in ["name", "rust_target", "runner", "qualification_status", "artifact_name", "required_checks", "signing_policy"] {
            assert!(target.contains_key(field), "target is missing {}", field);
        }
        assert!(target["required_checks"].contains("fmt"));
        assert!(target["required_checks"].contains("test"));
        assert!(!target["required_checks"].contains("skip"));
    }

    let advertised = matrix.lines().find(|line| line.starts_with("advertised_targets"))
        .expect("advertised_targets is explicit");
    assert!(!advertised.contains("unsupported"));
    assert!(!advertised.contains("missing-runner"));
    assert!(!advertised.contains("rc-"), "P06 has no qualified receipt for RC");
}

#[test]
fn fixtures_distinguish_supported_unsupported_and_missing_runner() {
    let matrix = read("packaging/targets.toml");
    assert!(matrix.contains("fixture_supported"));
    assert!(matrix.contains("fixture_unsupported"));
    assert!(matrix.contains("fixture_missing_runner"));
    assert!(matrix.contains("NOT_QUALIFIED"));
    assert!(matrix.contains("unsupported targets are excluded"));
}

#[test]
fn workflow_builds_only_declared_qualified_lanes_without_skips() {
    let workflow = read(".github/workflows/ci.yml");
    assert!(workflow.contains("packaging/targets.toml"));
    assert!(workflow.contains("cargo test"));
    assert!(workflow.contains("cargo fmt --all -- --check"));
    assert!(!workflow.contains("continue-on-error"));
    assert!(!workflow.contains("if: always()"));
}
