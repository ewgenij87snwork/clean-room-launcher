use std::process::Command;
use taskseal::contracts::trace::{load_control_map, validate_control_map};

#[test]
fn canonical_map_has_445_explicit_rows_and_reciprocal_fields() {
    let map = load_control_map("controls/v0.1-execution-map.tsv").unwrap();
    assert_eq!(map.len(), 445);
    assert!(validate_control_map(&map).is_ok());
}

#[test]
fn missing_conflicting_unreviewed_and_stale_rows_refuse() {
    for fixture in ["missing.tsv", "conflicting.tsv", "unreviewed.tsv", "stale.tsv"] {
        let path = format!("fixtures/contracts/requirements/{fixture}");
        let map = load_control_map(&path).unwrap();
        assert!(validate_control_map(&map).is_err(), "{fixture} accepted");
    }
}

#[test]
fn canonical_checker_is_the_executable_445_gate() {
    let output = Command::new("ruby").arg("scripts/check-control-coverage.rb").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("445/445"));
}
