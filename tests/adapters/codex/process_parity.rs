use std::path::Path;

#[test]
fn process_cases_cover_exact_argv_and_terminal_outcomes_without_private_data() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/adapters/codex/process");
    for case in ["unicode", "provider-flag", "failure", "interrupt", "resize"] {
        let text = std::fs::read_to_string(root.join(format!("{case}.fixture"))).unwrap();
        assert!(text.contains("case="));
        assert!(text.contains("argv_digest="));
        assert!(text.contains("exit="));
        assert!(!text.contains("prompt="));
        assert!(!text.contains("transcript="));
    }
}

#[test]
fn process_receipt_is_explicitly_refusal_when_provider_native_observation_is_unavailable() {
    let receipt = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("reports/gates/p06/task-10.json"),
    )
    .unwrap();
    assert!(receipt.contains("\"qualification\":\"NOT_QUALIFIED\""));
    assert!(receipt.contains("\"provider_launch\":false"));
    assert!(receipt.contains("\"raw_output_retained\":false"));
}
