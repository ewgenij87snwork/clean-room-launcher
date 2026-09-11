use clroom::adapters::qualification::{
    EvidenceRef, QualificationReason, QualificationState, TupleClaim, parse_evidence, qualify,
    seal_receipt, verify_receipt, verify_receipt_bytes,
};

fn claim() -> TupleClaim {
    TupleClaim {
        provider_id: "fixture".into(),
        declaration_digest: "d".repeat(64),
        artifact_digest: "a".repeat(64),
        version: (1, 2, 3),
        os: "macos".into(),
        arch: "aarch64".into(),
        interpreter_digest: None,
    }
}

#[test]
fn generic_evidence_is_not_qualified_and_stale_or_wrong_tuple_refuses() {
    let expected = claim();
    let generic = EvidenceRef {
        kind: "generic-foundation".into(),
        claim: expected.clone(),
        observed_at: 100,
        expires_at: 200,
        digest: "e".repeat(64),
        refused: false,
    };
    assert!(matches!(
        qualify(std::slice::from_ref(&generic), &expected, 150),
        QualificationState::NotQualified {
            reason: QualificationReason::ProviderLaunchMissing
        }
    ));
    assert!(matches!(
        qualify(&[generic], &expected, 201),
        QualificationState::NotQualified {
            reason: QualificationReason::StaleEvidence
        }
    ));
    let wrong = TupleClaim {
        provider_id: "other".into(),
        ..expected.clone()
    };
    let wrong_evidence = EvidenceRef {
        kind: "generic-foundation".into(),
        claim: wrong,
        observed_at: 100,
        expires_at: 200,
        digest: "f".repeat(64),
        refused: false,
    };
    assert!(matches!(
        qualify(&[wrong_evidence], &expected, 150),
        QualificationState::Refused {
            reason: QualificationReason::TupleMismatch
        }
    ));
}

#[test]
fn closed_evidence_parser_refuses_unknown_fields_and_wrong_tuple() {
    let expected = claim();
    let output = b"privacy-clean observation\n";
    let valid = format!(
        r#"{{"schema_version":"clroom.provider-evidence.v1","kind":"provider-launch-observed","provider_id":"fixture","declaration_digest":"{}","artifact_digest":"{}","version":[1,2,3],"os":"macos","arch":"aarch64","interpreter_digest":null,"observed_at":1,"expires_at":2,"output_digest":"{}","refused":false}}"#,
        expected.declaration_digest,
        expected.artifact_digest,
        clroom::core::inventory::sha256_hex(output)
    );
    assert!(parse_evidence(valid.as_bytes(), output, &expected).is_ok());
    assert!(parse_evidence(br#"{"unknown":true}"#, output, &expected).is_err());
    let wrong_provider = valid.replace("\"provider_id\":\"fixture\"", "\"provider_id\":\"other\"");
    assert!(parse_evidence(wrong_provider.as_bytes(), output, &expected).is_err());
    assert!(parse_evidence(valid.as_bytes(), b"altered", &expected).is_err());
}

#[test]
fn portable_receipt_is_canonical_bound_and_rejects_tampering() {
    let expected = claim();
    let evidence = vec![
        EvidenceRef {
            kind: "foundation-environment".into(),
            claim: expected.clone(),
            observed_at: 100,
            expires_at: 200,
            digest: "e".repeat(64),
            refused: false,
        },
        EvidenceRef {
            kind: "foundation-placement".into(),
            claim: expected.clone(),
            observed_at: 100,
            expires_at: 200,
            digest: "f".repeat(64),
            refused: false,
        },
    ];
    let receipt = seal_receipt(expected.clone(), evidence, 100, 200).unwrap();
    verify_receipt(&receipt).unwrap();
    let bytes = serde_json::to_vec(&receipt).unwrap();
    assert_eq!(verify_receipt_bytes(&bytes).unwrap(), receipt);
    assert_eq!(
        receipt.schema_version,
        "clroom.provider-qualification-receipt.v1"
    );
    assert_eq!(receipt.verifier_version, "clroom.macos.t4.v1");
    assert_eq!(receipt.receipt_digest.len(), 64);

    let mut tampered = receipt.clone();
    tampered.evidence[0].digest = "d".repeat(64);
    assert_eq!(
        verify_receipt(&tampered),
        Err(QualificationReason::ReceiptMismatch)
    );

    let mut rejected = receipt;
    rejected.evidence[0].refused = true;
    assert_eq!(
        verify_receipt(&rejected),
        Err(QualificationReason::RefusedEvidence)
    );

    let unknown = String::from_utf8(bytes)
        .unwrap()
        .replace('}', ",\"unknown\":true}");
    assert_eq!(
        verify_receipt_bytes(unknown.as_bytes()),
        Err(QualificationReason::InvalidClaim)
    );
}
#[cfg(target_os = "macos")]
#[test]
fn sleeping_candidate_cannot_qualify_as_real_codex() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let root = std::env::temp_dir().join(format!("clroom-qualification-{}", std::process::id()));
    fs::create_dir(&root).expect("temporary qualification directory");
    let candidate = root.join("sleeping-candidate");
    fs::write(&candidate, "#!/bin/sh\n/bin/sleep 8\n").expect("candidate script");
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o700)).expect("candidate mode");
    let evidence = root.join("evidence.json");
    let executable = ["/opt/homebrew/bin/codex", "/usr/local/bin/codex", "/usr/bin/codex"]
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .unwrap_or("");
    if executable.is_empty() {
        return;
    }
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/release/qualify-real-provider.sh");
    let result = Command::new("/bin/bash")
        .args([
            script.to_str().expect("script path"),
            "--provider", "codex",
            "--executable", executable,
            "--candidate", candidate.to_str().expect("candidate path"),
            "--source-head", "0000000000000000000000000000000000000000",
            "--version", "0.2.0",
            "--output", evidence.to_str().expect("evidence path"),
        ])
        .output()
        .expect("qualification harness");

    assert!(!result.status.success());
    assert_eq!(result.status.code(), Some(1));
    assert!(result.status.code() == Some(1) || result.status.code() == Some(2));
    fs::remove_dir_all(root).expect("temporary qualification cleanup");
}
