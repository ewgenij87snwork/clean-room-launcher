use taskseal::adapters::codex::{
    context_canaries::{CanaryState, ContextLayer, observe_without_provider},
    identity::CodexTuple,
};

fn exact_tuple() -> CodexTuple {
    CodexTuple {
        provider_id: "codex".into(),
        artifact_digest: "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37".into(),
        version: (0, 147, 0),
        os: "macos".into(),
        arch: "aarch64".into(),
    }
}

const L0_NONCE_SHA256: &str = "f7b586cf98ee8c8f2ba2ea22d9a24c4a05f59a5b30ec93b25fbe54c1f1ce3914";
const L2_NONCE_SHA256: &str = "4395db3ed4c16654d9c9a5d0af713a0d6019c0fdc6c18a10acbd5f69bd688a6b";
const L3_NONCE_SHA256: &str = "7e4b5c4f10b7a5601d19f0b9e9cc96cec0d1b7a791d1bfd02c12e63d601aeb21";
const FORBIDDEN_NONCE_SHA256: &str = "7cb282822bd4a9eebc8cf2e8fbef163cbf21e36c75a6b6c4b0b1d442f0b2e4c0";

#[test]
fn forbidden_ambient_canary_refuses_before_any_native_provider_process() {
    let observation = observe_without_provider(
        &exact_tuple(),
        ContextLayer::ForbiddenAmbient,
        FORBIDDEN_NONCE_SHA256,
    );
    assert_eq!(observation.state, CanaryState::Refused);
    assert!(observation.exact_tuple_bound);
    assert_eq!(observation.nonce_digest, FORBIDDEN_NONCE_SHA256);
}

#[test]
fn required_layer_canaries_remain_not_qualified_without_a_provider_process() {
    for (layer, nonce_digest) in [
        (ContextLayer::L0Safety, L0_NONCE_SHA256),
        (ContextLayer::L2Project, L2_NONCE_SHA256),
        (ContextLayer::L3Task, L3_NONCE_SHA256),
    ] {
        let observation = observe_without_provider(&exact_tuple(), layer, nonce_digest);
        assert_eq!(observation.state, CanaryState::Refused);
        assert!(observation.exact_tuple_bound);
        assert_eq!(observation.nonce_digest, nonce_digest);
    }
}

#[test]
fn a_nonexact_tuple_cannot_be_mistaken_for_a_bound_native_canary() {
    let mut tuple = exact_tuple();
    tuple.version = (0, 148, 0);
    let observation = observe_without_provider(&tuple, ContextLayer::L0Safety, L0_NONCE_SHA256);
    assert_eq!(observation.state, CanaryState::Refused);
    assert!(!observation.exact_tuple_bound);
}
