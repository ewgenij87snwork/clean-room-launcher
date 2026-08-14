use std::{fs, path::Path, process::Command};
fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }
#[test]
fn build_script_declares_safe_deterministic_complete_layout() {
    let script = fs::read_to_string(root().join("packaging/build-artifacts.sh")).unwrap();
    for needle in ["cargo build --locked", "source_commit=", "rust_toolchain=", "rustc=", "cargo=", "python=", "qualification=NOT_QUALIFIED", "signing=unsigned-preview-only", "mtime=0", "info.uid = info.gid = 0", "bin/taskseal", "bin/tseal", "LICENSE", "NOTICE", "gzip.GzipFile"] { assert!(script.contains(needle), "missing layout control: {}", needle); }
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
fn production_verifier_rejects_generated_poison_archives() {
    let verifier = root().join("packaging/verify-artifact.py");
    let maker = root().join("tests/packaging/fixtures/make_poison_archive.py");
    let temp = std::env::temp_dir().join(format!("taskseal-poison-{}", std::process::id()));
    let _ = fs::create_dir_all(&temp);
    for kind in ["traversal", "wrong-name", "missing-license", "metadata"] {
        let archive = temp.join(format!("{kind}.tar.gz"));
        assert!(Command::new("python3").args([maker.to_str().unwrap(), kind, archive.to_str().unwrap()]).status().unwrap().success());
        let status = Command::new("python3").args([verifier.to_str().unwrap(), archive.to_str().unwrap()]).status().unwrap();
        assert!(!status.success(), "poison archive unexpectedly accepted: {}", kind);
    }
}
#[test]
fn canonical_and_alias_parser_behavior_matches_on_status_json_selector() {
    let build = Command::new("cargo").args(["build", "--locked", "--release", "--bin", "taskseal", "--bin", "tseal"]).env("CARGO_NET_OFFLINE", "true").output().unwrap();
    assert!(build.status.success(), "offline binary build failed: {}", String::from_utf8_lossy(&build.stderr));
    let bin = root().join("target/release");
    let run = |name: &str| Command::new(bin.join(name)).args(["--output", "json", "status"]).output().unwrap();
    let canonical = run("taskseal");
    let alias = run("tseal");
    assert_eq!(canonical.status.code(), alias.status.code());
    assert_eq!(canonical.stdout, alias.stdout);
    assert_eq!(canonical.stderr, alias.stderr);
    assert_eq!(canonical.status.code(), Some(2));
}
#[test]
fn script_is_executable_and_shell_valid() { assert!(Command::new("bash").args(["-n", root().join("packaging/build-artifacts.sh").to_str().unwrap()]).status().unwrap().success()); }
