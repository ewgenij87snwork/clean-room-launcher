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
    let policy = CodexAuthPolicy::unavailable(
        tuple(),
        "980c03af9c28ddbb7280a52fd72a39790544e8a649ccd2442e078e95e9db6337".into(),
    );
    assert_eq!(evaluate_auth(&policy, &[]), CodexAuthState::Unavailable);
}

#[test]
fn unavailable_policy_refuses_any_requested_auth_name() {
    let policy = CodexAuthPolicy::unavailable(
        tuple(),
        "980c03af9c28ddbb7280a52fd72a39790544e8a649ccd2442e078e95e9db6337".into(),
    );
    assert_eq!(
        evaluate_auth(&policy, &["OPENAI_API_KEY"]),
        CodexAuthState::Refused
    );
}

#[test]
fn unavailable_policy_refuses_wrong_tuple_and_malformed_reference_name() {
    let mut wrong = tuple();
    wrong.version = (0, 148, 0);
    let policy = CodexAuthPolicy::unavailable(
        wrong,
        "980c03af9c28ddbb7280a52fd72a39790544e8a649ccd2442e078e95e9db6337".into(),
    );
    assert_eq!(evaluate_auth(&policy, &[]), CodexAuthState::Refused);
    let policy = CodexAuthPolicy::unavailable(
        tuple(),
        "980c03af9c28ddbb7280a52fd72a39790544e8a649ccd2442e078e95e9db6337".into(),
    );
    assert_eq!(
        evaluate_auth(&policy, &["bad-name"]),
        CodexAuthState::Refused
    );
}
