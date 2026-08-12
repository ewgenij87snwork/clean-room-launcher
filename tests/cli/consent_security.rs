#[path = "../../src/cli/consent.rs"]
mod consent;
mod state {
    pub use super::super::state::*;
}

use super::state::AccessClass;
use consent::{Decision, LaunchConsent, LaunchProposal};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn proposal(access: AccessClass) -> LaunchProposal {
    LaunchProposal::new(
        digest('a'),
        vec![".taskseal/runtime/context.md".to_owned()],
        vec!["AGENTS.md".to_owned()],
        "codex".to_owned(),
        digest('b'),
        digest('c'),
        access,
    )
    .unwrap()
}

#[test]
fn full_access_disclosure_is_explicit_and_never_prints_argv_digest() {
    let disclosure = proposal(AccessClass::FullAccess).disclosure();
    assert!(disclosure.contains("FULL ACCESS"));
    assert!(!disclosure.contains(&digest('b')));
}

#[test]
fn consent_binds_every_proposal_field() {
    let original = proposal(AccessClass::Standard);
    let token = LaunchConsent::resolve(Decision::Approve, &original).unwrap();
    assert!(token.verify(&original).is_ok());
    let changed = LaunchProposal::new(
        digest('d'),
        vec![".taskseal/runtime/context.md".to_owned()],
        vec!["AGENTS.md".to_owned()],
        "codex".to_owned(),
        digest('b'),
        digest('c'),
        AccessClass::Standard,
    )
    .unwrap();
    assert_eq!(
        token.verify(&changed).unwrap_err().code(),
        "CONSENT_STALE_REFUSED"
    );
}

#[test]
fn cancellation_returns_without_a_token() {
    assert_eq!(
        LaunchConsent::resolve(Decision::Cancel, &proposal(AccessClass::Standard))
            .unwrap_err()
            .code(),
        "CONSENT_CANCELLED"
    );
}

#[test]
fn invalid_provider_digest_or_path_refuses() {
    assert!(
        LaunchProposal::new(
            "bad".to_owned(),
            vec!["../escape".to_owned()],
            vec![],
            "other".to_owned(),
            digest('b'),
            digest('c'),
            AccessClass::Standard
        )
        .is_err()
    );
}
