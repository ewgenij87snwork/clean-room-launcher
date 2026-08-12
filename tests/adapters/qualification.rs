use taskseal::adapters::qualification::{parse_evidence, qualify, EvidenceRef, QualificationReason, QualificationState, TupleClaim};

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

#[test]
fn closed_evidence_parser_refuses_unknown_fields_and_wrong_tuple() {
    let expected = claim();
    let output = b"privacy-clean observation\n";
    let valid = format!(r#"{{"schema_version":"taskseal.provider-evidence.v1","kind":"provider-launch-observed","provider_id":"fixture","declaration_digest":"{}","artifact_digest":"{}","version":[1,2,3],"os":"macos","arch":"aarch64","interpreter_digest":null,"observed_at":1,"expires_at":2,"output_digest":"{}","refused":false}}"#, expected.declaration_digest, expected.artifact_digest, taskseal::core::inventory::sha256_hex(output));
    assert!(parse_evidence(valid.as_bytes(), output, &expected).is_ok());
    assert!(parse_evidence(br#"{"unknown":true}"#, output, &expected).is_err());
    let wrong_provider = valid.replace("\"provider_id\":\"fixture\"", "\"provider_id\":\"other\"");
    assert!(parse_evidence(wrong_provider.as_bytes(), output, &expected).is_err());
    assert!(parse_evidence(valid.as_bytes(), b"altered", &expected).is_err());
}
