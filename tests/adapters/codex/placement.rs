use taskseal::adapters::codex::placement::{CodexPlacementState, refuse_projection};

#[test]
fn projection_refuses_before_provider_birth_for_any_ambient_or_target_request() {
    assert_eq!(refuse_projection(&["HOME"], "provider_native_context"), CodexPlacementState::Refused);
    assert_eq!(refuse_projection(&[], "user-owned-target"), CodexPlacementState::Refused);
}
