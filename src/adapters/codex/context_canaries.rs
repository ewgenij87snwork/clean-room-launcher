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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCanaryState {
    Passed,
    Refused,
}

const REQUIRED_NONCE_DIGESTS: [&str; 3] = [
    "f7b586cf98ee8c8f2ba2ea22d9a24c4a05f59a5b30ec93b25fbe54c1f1ce3914",
    "4395db3ed4c16654d9c9a5d0af713a0d6019c0fdc6c18a10acbd5f69bd688a6b",
    "7e4b5c4f10b7a5601d19f0b9e9cc96cec0d1b7a791d1bfd02c12e63d601aeb21",
];

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

pub fn evaluate_native_observation(
    tuple: &CodexTuple,
    observed_nonce_digests: &[&str],
) -> NativeCanaryState {
    if !is_exact_tuple(tuple)
        || observed_nonce_digests.len() != REQUIRED_NONCE_DIGESTS.len()
        || !REQUIRED_NONCE_DIGESTS
            .iter()
            .all(|required| observed_nonce_digests.iter().any(|observed| observed == required))
    {
        return NativeCanaryState::Refused;
    }
    NativeCanaryState::Passed
}

fn is_exact_tuple(tuple: &CodexTuple) -> bool {
    tuple.provider_id == "codex"
        && tuple.artifact_digest
            == "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
        && tuple.version == (0, 147, 0)
        && tuple.os == "macos"
        && tuple.arch == "aarch64"
}
