use std::{fs, path::PathBuf, process::Command, sync::atomic::{AtomicU64, Ordering}};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temporary_result() -> PathBuf {
    std::env::temp_dir().join(format!(
        "clroom-real-reduction-{}-{}.json",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn sole_gate_rejects_ambiguous_canary_fields_that_look_like_real_codex_observation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gate = root.join(
        "scripts/gates/p06/successors/clroom-macos-isolated-launch-v1/verify.sh",
    );
    if !gate.exists() {
        let inventory = fs::read_to_string(
            root.join("qualification/public-release-inventory-v1.json"),
        )
        .expect("public release inventory exists when internal gates are excluded");
        assert!(inventory.contains("\"scripts/gates\""));
        return;
    }

    // Break caught: offline sandbox evidence is mislabeled as observed inside real Codex.
    let result = temporary_result();
    fs::write(
        &result,
        r#"{"schema_version":"clroom.macos-isolated-launch.result.v1","plan_id":"P06-CLROOM-MACOS-ISOLATED-LAUNCH-V1","tuple_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","manifest_digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","process_count":1,"allowed_project_canary":true,"denied_global_agents_canary":true,"denied_user_skill_canary":true,"browser_or_login":false,"exit_class":"HELP_EXIT_0","raw_retained":false}"#,
    )
    .unwrap();
    let output = Command::new(gate).arg(&result).output().unwrap();
    assert!(!output.status.success());
    let _ = fs::remove_file(result);
}
