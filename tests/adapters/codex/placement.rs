use std::fs;
use taskseal::adapters::codex::placement::{
    CodexPlacementState, preflight_projection, preflight_provider_handoff,
};

#[test]
fn projection_refuses_before_provider_birth_for_any_ambient_or_target_request() {
    assert_eq!(
        preflight_projection(&["HOME"], "provider_native_context"),
        CodexPlacementState::Refused
    );
    assert_eq!(
        preflight_projection(&[], "user-owned-target"),
        CodexPlacementState::Refused
    );
}

#[test]
fn projection_refusal_preserves_tracked_target_bytes_without_reading_ambient_values() {
    let target = std::env::temp_dir().join(format!("taskseal-p06-t7-{}", std::process::id()));
    let _ = fs::remove_file(&target);
    fs::write(&target, b"owner-bytes\n").unwrap();
    assert_eq!(
        preflight_projection(&["HOME"], "tracked-target"),
        CodexPlacementState::Refused
    );
    assert_eq!(fs::read(&target).unwrap(), b"owner-bytes\n");
    fs::remove_file(target).unwrap();
}

#[test]
fn actual_provider_handoff_refuses_before_process_or_projection() {
    assert_eq!(
        preflight_provider_handoff().unwrap_err(),
        "P06_REQUIRED: provider tuple is not qualified"
    );
}
