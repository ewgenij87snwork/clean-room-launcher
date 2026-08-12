use taskseal::contracts::error::ErrorRecord;
use taskseal::contracts::receipt::ReceiptSeam;

#[test]
fn error_record_round_trips_with_stable_json() {
    let error = ErrorRecord {
        schema_version: "error.v1".into(),
        code: "NON_INTERACTIVE_INPUT_REQUIRED".into(),
        cause: "stdin is not available".into(),
        safe_action: "rerun with explicit input".into(),
        details: vec!["provider=codex".into()],
    };
    let bytes = serde_json::to_vec(&error).unwrap();
    let decoded: ErrorRecord = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
}

#[test]
fn receipt_seam_round_trips_and_contains_no_private_payload_fields() {
    let receipt = ReceiptSeam {
        schema_version: "receipt-seam.v1".into(),
        plan: "P02".into(),
        task: 6,
        result: "accepted".into(),
        subject_digests: vec![("src/lib.rs".into(), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into())],
    };
    let bytes = serde_json::to_vec(&receipt).unwrap();
    let decoded: ReceiptSeam = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    assert!(!String::from_utf8(bytes).unwrap().contains("prompt"));
}

#[test]
fn unknown_fields_and_private_payloads_are_refused() {
    let unknown = br#"{"schema_version":"error.v1","code":"X","cause":"c","safe_action":"a","details":[],"unknown":true}"#;
    assert!(serde_json::from_slice::<ErrorRecord>(unknown).is_err());
    let private = br#"{"schema_version":"receipt-seam.v1","plan":"P02","task":6,"result":"accepted","subject_digests":[],"transcript":"secret"}"#;
    assert!(serde_json::from_slice::<ReceiptSeam>(private).is_err());
}
