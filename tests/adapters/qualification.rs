use taskseal::adapters::qualification::{qualify, EvidenceRef, QualificationReason, QualificationState, TupleClaim};

fn claim() -> TupleClaim { TupleClaim { provider_id: "fixture".into(), declaration_digest: "d".repeat(64), artifact_digest: "a".repeat(64), version: (1, 2, 3), os: "macos".into(), arch: "aarch64".into(), interpreter_digest: None } }

#[test]
fn generic_evidence_is_not_qualified_and_stale_or_wrong_tuple_refuses() {
    let expected = claim();
    let generic = EvidenceRef { kind: "generic-foundation".into(), claim: expected.clone(), observed_at: 100, expires_at: 200, digest: "e".repeat(64), refused: false };
    assert!(matches!(qualify(std::slice::from_ref(&generic), &expected, 150), QualificationState::NotQualified { reason: QualificationReason::ProviderLaunchMissing }));
    assert!(matches!(qualify(&[generic], &expected, 201), QualificationState::NotQualified { reason: QualificationReason::StaleEvidence }));
    let wrong = TupleClaim { provider_id: "other".into(), ..expected.clone() };
    let wrong_evidence = EvidenceRef { kind: "generic-foundation".into(), claim: wrong, observed_at: 100, expires_at: 200, digest: "f".repeat(64), refused: false };
    assert!(matches!(qualify(&[wrong_evidence], &expected, 150), QualificationState::Refused { reason: QualificationReason::TupleMismatch }));
}
