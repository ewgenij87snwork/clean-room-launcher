use std::{fs, path::Path, process::Command};
fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }
#[test]
fn build_script_declares_safe_deterministic_complete_layout() {
    let script = fs::read_to_string(root().join("packaging/build-artifacts.sh")).unwrap();
    for needle in ["cargo build --locked", "source_commit=", "rust_toolchain=", "qualification=NOT_QUALIFIED", "signing=unsigned-preview-only", "mtime=0", "info.uid = info.gid = 0", "bin/taskseal", "bin/tseal", "LICENSE", "NOTICE", "gzip.GzipFile"] { assert!(script.contains(needle), "missing layout control: {}", needle); }
}
#[test]
fn archive_fixture_names_are_path_safe_and_complete() {
    let fixture = fs::read_to_string(root().join("tests/packaging/fixtures/archive-layout.txt")).unwrap();
    for name in fixture.lines().filter(|line| !line.is_empty()) { assert!(!name.starts_with('/') && !name.split('/').any(|part| part == ".."), "unsafe path {}", name); }
    for required in ["taskseal-v0.1.0/bin/taskseal", "taskseal-v0.1.0/bin/tseal", "taskseal-v0.1.0/LICENSE", "taskseal-v0.1.0/NOTICE", "taskseal-v0.1.0/VERSION"] { assert!(fixture.lines().any(|line| line == required), "missing {}", required); }
}
#[test]
fn poisoned_fixtures_are_rejected_by_contract() {
    for (file, needle) in [("archive-path-traversal.txt", "../"), ("archive-wrong-binary.txt", "bin/taskseal-old"), ("archive-missing-license.txt", "NOTICE"), ("archive-nondeterministic-metadata.txt", "mtime=now")] {
        let text = fs::read_to_string(root().join("tests/packaging/fixtures").join(file)).unwrap(); assert!(text.contains(needle));
        if file.contains("path-traversal") { assert!(text.lines().any(|l| l.starts_with("../"))); }
        if file.contains("wrong-binary") { assert!(!text.lines().any(|l| l == "bin/taskseal")); }
        if file.contains("missing-license") { assert!(!text.lines().any(|l| l == "LICENSE")); }
        if file.contains("nondeterministic") { assert!(text.contains("mtime=now")); }
    }
}
#[test]
fn script_is_executable_and_shell_valid() { assert!(Command::new("bash").args(["-n", root().join("packaging/build-artifacts.sh").to_str().unwrap()]).status().unwrap().success()); }
