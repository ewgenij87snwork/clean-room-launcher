use super::identity::CodexTuple;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextLayer {
    L0Safety,
    L2Project,
    L3Task,
    ForbiddenAmbient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanaryObservation {
    pub layer: ContextLayer,
    pub nonce_digest: String,
    pub exact_tuple_bound: bool,
    pub state: CanaryState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanaryState {
    Refused,
}

/// Test-only, non-launching canary boundary. It deliberately has no process or
/// environment-reading seam: positive native observations need separate owner
/// authorization for an isolated provider process.
pub fn observe_without_provider(
    tuple: &CodexTuple,
    layer: ContextLayer,
    nonce_digest: &str,
) -> CanaryObservation {
    CanaryObservation {
        layer,
        nonce_digest: nonce_digest.to_owned(),
        exact_tuple_bound: tuple.provider_id == "codex"
            && tuple.artifact_digest
                == "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
            && tuple.version == (0, 147, 0)
            && tuple.os == "macos"
            && tuple.arch == "aarch64",
        state: CanaryState::Refused,
    }
}
