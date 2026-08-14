//! Focused, dependency-free contract tests for the P07 release target matrix.

use std::{collections::BTreeMap, fs, path::Path};

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect("packaging fixture exists")
}

fn list_value(source: &str, key: &str) -> Vec<String> {
    source
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key} = ")))
        .map(|value| {
            value
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .filter_map(|item| {
                    let item = item.trim().trim_matches('"');
                    (!item.is_empty()).then(|| item.to_owned())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn target_blocks(source: &str) -> Vec<BTreeMap<String, String>> {
    source.split("[[targets]]").skip(1).map(|block| {
        block.lines().filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().trim_matches('"').to_owned()))
        }).collect()
    }).collect()
}

fn matrix_is_honest(source: &str) -> bool {
    let advertised = list_value(source, "advertised_targets");
    let qualified = list_value(source, "qualified_targets");
    if advertised != qualified { return false; }
    let targets = target_blocks(source);
    advertised.iter().all(|name| targets.iter().any(|target| {
        target.get("name") == Some(name)
            && target.get("qualification_status") == Some(&"QUALIFIED".to_owned())
            && target.get("runner").is_some_and(|runner| runner != "unsupported" && runner != "missing-runner")
            && target.get("artifact_name").is_some_and(|artifact| !artifact.is_empty())
    }))
}

fn workflow_is_bound_to_qualified(source: &str) -> bool {
    source.contains("packaging/targets.toml")
        && source.contains("qualified_targets")
        && source.contains("qualification_status")
        && source.contains("rust_target")
        && source.contains("runs-on: ${{ matrix.runner }}")
        && source.contains("matrix:")
        && source.contains("cargo test")
        && !source.contains("continue-on-error")
        && !source.contains("if: always()")
        && !source.contains("cargo fmt --all")
}

#[test]
fn matrix_advertised_targets_exactly_equal_qualified_lanes() {
    let matrix = read("packaging/targets.toml");
    assert!(matrix_is_honest(&matrix));
    assert!(list_value(&matrix, "advertised_targets").is_empty());
    for target in target_blocks(&matrix) {
        for field in ["name", "rust_target", "runner", "qualification_status", "artifact_name", "required_checks", "signing_policy"] {
            assert!(target.contains_key(field), "target is missing {}", field);
        }
        assert!(target["required_checks"].contains("fmt"));
        assert!(target["required_checks"].contains("test"));
        assert!(!target["required_checks"].contains("skip"));
    }
}

#[test]
fn matrix_mutations_refuse_false_claims_and_mismatched_lists() {
    let matrix = read("packaging/targets.toml");
    let advertised_false = matrix.replace("advertised_targets = []", "advertised_targets = [\"alpha-macos-aarch64\"]");
    assert!(!matrix_is_honest(&advertised_false), "non-empty NOT_QUALIFIED claim must fail");
    let lists_mismatch = matrix.replace("qualified_targets = []", "qualified_targets = [\"alpha-macos-aarch64\"]");
    assert!(!matrix_is_honest(&lists_mismatch), "advertised/qualified mismatch must fail");
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
fn workflow_is_structurally_bound_to_qualified_entries() {
    let workflow = read(".github/workflows/ci.yml");
    assert!(workflow_is_bound_to_qualified(&workflow));
    let advertised_binding = workflow.replace("qualified_targets", "advertised_targets");
    assert!(!workflow_is_bound_to_qualified(&advertised_binding));
    let hardcoded_runner = workflow.replace("runs-on: ${{ matrix.runner }}", "runs-on: ubuntu-latest");
    assert!(!workflow_is_bound_to_qualified(&hardcoded_runner));
    let global_fmt = format!("{workflow}\n      - run: cargo fmt --all -- --check\n");
    assert!(!workflow_is_bound_to_qualified(&global_fmt));
}
