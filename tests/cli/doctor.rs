use std::process::Command;

#[test]
fn doctor_reports_unqualified_adapters_without_claiming_provider_support_or_mutating_root() {
    let root = env!("CARGO_MANIFEST_DIR");
    let before = std::fs::read_dir(root).unwrap().count();
    let output = Command::new(env!("CARGO_BIN_EXE_taskseal"))
        .args(["doctor", "--root", root])
        .output()
        .expect("doctor must run");
    let after = std::fs::read_dir(root).unwrap().count();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(before, after, "doctor must not mutate its root");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("DOCTOR_PASS"));
    assert!(stdout.contains("P06_REQUIRED"));
    assert!(!stdout.contains("SUPPORTED"));
}

#[test]
fn doctor_refuses_invalid_schema_config_and_published_artifact_with_stable_codes() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let scratch = std::env::temp_dir().join(format!("taskseal-doctor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("schemas/contracts")).unwrap();
    std::fs::create_dir_all(scratch.join("adapters/declarations")).unwrap();
    std::fs::copy(
        base.join("schemas/contracts/adapter-declaration.schema.json"),
        scratch.join("schemas/contracts/adapter-declaration.schema.json"),
    )
    .unwrap();
    for provider in ["codex", "claude"] {
        std::fs::copy(
            base.join(format!("adapters/declarations/{provider}.toml")),
            scratch.join(format!("adapters/declarations/{provider}.toml")),
        )
        .unwrap();
    }
    let binary = env!("CARGO_BIN_EXE_taskseal");
    let run = |root: &std::path::Path| {
        Command::new(binary)
            .args(["doctor", "--root", root.to_str().unwrap()])
            .output()
            .unwrap()
    };

    std::fs::remove_file(scratch.join("schemas/contracts/adapter-declaration.schema.json"))
        .unwrap();
    assert!(
        String::from_utf8(run(&scratch).stderr)
            .unwrap()
            .contains("DOCTOR_SCHEMA_INVALID")
    );
    std::fs::copy(
        base.join("schemas/contracts/adapter-declaration.schema.json"),
        scratch.join("schemas/contracts/adapter-declaration.schema.json"),
    )
    .unwrap();
    std::fs::write(scratch.join("adapters/declarations/codex.toml"), "bad").unwrap();
    assert!(
        String::from_utf8(run(&scratch).stderr)
            .unwrap()
            .contains("DOCTOR_CONFIG_INVALID")
    );
    std::fs::copy(
        base.join("adapters/declarations/codex.toml"),
        scratch.join("adapters/declarations/codex.toml"),
    )
    .unwrap();
    std::fs::create_dir_all(scratch.join(".taskseal/out")).unwrap();
    std::fs::write(scratch.join(".taskseal/out/current.json"), "corrupt").unwrap();
    assert!(
        String::from_utf8(run(&scratch).stderr)
            .unwrap()
            .contains("DOCTOR_ARTIFACT_INVALID")
    );
    std::fs::remove_dir_all(scratch).unwrap();
}
