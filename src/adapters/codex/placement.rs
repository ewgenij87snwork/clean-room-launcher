#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexPlacementState {
    Refused,
}

pub fn preflight_projection(ambient_names: &[&str], target: &str) -> CodexPlacementState {
    let _ = (ambient_names, target);
    CodexPlacementState::Refused
}

pub fn preflight_provider_handoff() -> Result<(), &'static str> {
    let ambient_names = ["HOME", "PATH", "ALL_PROXY", "HTTP_PROXY", "HTTPS_PROXY"];
    match preflight_projection(&ambient_names, "provider_native_context") {
        CodexPlacementState::Refused => Err("P06_REQUIRED: provider tuple is not qualified"),
    }
}
