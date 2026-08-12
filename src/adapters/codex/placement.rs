#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexPlacementState {
    Refused,
}

pub fn preflight_projection(ambient_names: &[&str], target: &str) -> CodexPlacementState {
    let _ = (ambient_names, target);
    CodexPlacementState::Refused
}
