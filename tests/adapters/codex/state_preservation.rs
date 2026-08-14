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
fn state_receipt_refuses_without_native_provider_observation() {
    let receipt = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("reports/gates/p06/task-11.json"),
    )
    .unwrap();
    assert!(receipt.contains("\"qualification\":\"NOT_QUALIFIED\""));
    assert!(receipt.contains("\"protected_state_unchanged\":true"));
    assert!(receipt.contains("\"raw_output_retained\":false"));
}
