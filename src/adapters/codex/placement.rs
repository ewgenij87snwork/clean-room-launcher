#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexPlacementState {
    Refused,
}

pub fn refuse_projection(_ambient_names: &[&str], _target: &str) -> CodexPlacementState {
    CodexPlacementState::Refused
}
