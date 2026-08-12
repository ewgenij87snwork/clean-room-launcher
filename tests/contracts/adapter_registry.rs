use taskseal::contracts::adapter::parse_declaration;

#[test]
fn valid_unqualified_declarations_parse_without_support_claims() {
    for path in [
        "adapters/declarations/codex.toml",
        "adapters/declarations/claude.toml",
    ] {
        let text = std::fs::read_to_string(path).unwrap();
        let declaration = parse_declaration(&text).unwrap();
        assert!(!declaration.qualified);
        assert!(!declaration.capability_evidence.contains("PASS"));
    }
}

#[test]
fn malformed_duplicate_path_collision_and_mutable_pass_are_refused() {
    for text in [
        include_str!("../../fixtures/contracts/adapters/duplicate-provider.toml"),
        include_str!("../../fixtures/contracts/adapters/duplicate-path.toml"),
        include_str!("../../fixtures/contracts/adapters/mutable-pass.toml"),
        include_str!("../../fixtures/contracts/adapters/malformed.toml"),
    ] {
        assert!(parse_declaration(text).is_err());
    }
}
