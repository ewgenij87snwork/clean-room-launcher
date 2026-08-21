use std::process::Command;

#[test]
fn cargo_exposes_only_the_clroom_executable_target() {
    // Break caught: a legacy public executable alias is accidentally shipped again.
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let names = metadata["packages"][0]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .map(|target| target["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, ["clroom"]);
}
