use taskseal::adapters::codex::{
    environment::{CodexAuthPolicy, CodexAuthState, evaluate_auth},
    identity::CodexTuple,
};

fn tuple() -> CodexTuple {
    CodexTuple {
        provider_id: "codex".into(),
        artifact_digest: "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37".into(),
        version: (0, 147, 0),
        os: "macos".into(),
        arch: "aarch64".into(),
    }
}

#[test]
fn unavailable_policy_refuses_without_reading_auth_values() {
    let policy = CodexAuthPolicy::unavailable(tuple(), "d".repeat(64));
    assert_eq!(evaluate_auth(&policy, &[]), CodexAuthState::Unavailable);
}

#[test]
fn unavailable_policy_refuses_any_requested_auth_name() {
    let policy = CodexAuthPolicy::unavailable(tuple(), "d".repeat(64));
    assert_eq!(
        evaluate_auth(&policy, &["OPENAI_API_KEY"]),
        CodexAuthState::Refused
    );
}
