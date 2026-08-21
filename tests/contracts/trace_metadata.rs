use std::fs;
use std::process::Command;
use taskseal::contracts::trace::{load_control_map, validate_control_map};

#[test]
fn canonical_map_has_446_explicit_rows_and_reciprocal_fields() {
    let map = load_control_map("controls/v0.1-execution-map.tsv").unwrap();
    assert_eq!(map.len(), 446);
    assert!(validate_control_map(&map).is_ok());
}

#[test]
fn missing_conflicting_unreviewed_and_stale_rows_refuse() {
    for fixture in [
        "missing.tsv",
        "conflicting.tsv",
        "unreviewed.tsv",
        "stale.tsv",
    ] {
        let path = format!("fixtures/contracts/requirements/{fixture}");
        let map = load_control_map(&path).unwrap();
        assert!(validate_control_map(&map).is_err(), "{fixture} accepted");
    }
}

#[test]
fn canonical_checker_is_the_executable_446_gate() {
    let output = Command::new("ruby")
        .arg("scripts/check-control-coverage.rb")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("446/446"));
}

#[test]
fn previous_445_row_map_refuses_in_rust_and_the_executable_gate() {
    let current = fs::read_to_string("controls/v0.1-execution-map.tsv").unwrap();
    let previous = current
        .lines()
        .filter(|line| !line.starts_with("AUTH-01\t"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let fixture = std::env::temp_dir().join(format!(
        "taskseal-trace-metadata-previous-445-{}.tsv",
        std::process::id()
    ));
    fs::write(&fixture, previous).unwrap();

    let old_map = load_control_map(fixture.to_str().unwrap()).unwrap();
    let ruby = Command::new("ruby")
        .arg("scripts/check-control-coverage.rb")
        .arg(&fixture)
        .output()
        .unwrap();
    fs::remove_file(&fixture).unwrap();

    assert_eq!(validate_control_map(&old_map), Err("CONTROL_COUNT"));
    assert!(!ruby.status.success(), "previous 445-row map was accepted");
    assert_eq!(String::from_utf8_lossy(&ruby.stderr), "CONTROL_COUNT\n");
}
