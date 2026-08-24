use std::path::Path;

#[test]
fn state_cases_cover_normal_failure_and_interruption_teardown() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/adapters/codex/state");
    for case in ["normal", "provider-failure", "interrupted-prebirth"] {
        let text = std::fs::read_to_string(root.join(format!("{case}.fixture"))).unwrap();
        assert!(text.contains("protected_before="));
        assert!(text.contains("protected_after="));
        assert!(text.contains("taskseal_ephemeral_removed=true"));
        assert!(!text.contains("credential="));
        assert!(!text.contains("private_content="));
    }
}

#[test]
fn state_receipt_refuses_or_is_publicly_excluded() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let receipt_path = root.join("reports/gates/p06/task-11.json");
    if !receipt_path.is_file() {
        let inventory =
            std::fs::read_to_string(root.join("qualification/public-release-inventory-v1.json"))
                .unwrap();
        assert!(inventory.contains("IMMUTABLE_INTERNAL_EVIDENCE_EXCLUDED_FROM_PUBLIC_SOURCE"));
        assert!(inventory.contains("\"reports/gates\""));
        return;
    }
    let receipt = std::fs::read_to_string(receipt_path).unwrap();
    assert!(receipt.contains("\"qualification\":\"NOT_QUALIFIED\""));
    assert!(receipt.contains("\"protected_state_unchanged\":true"));
    assert!(receipt.contains("\"raw_output_retained\":false"));
}
