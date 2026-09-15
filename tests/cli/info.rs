use std::process::Command;

fn run_without_providers(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_clroom"))
        .args(args)
        .env("PATH", "/nonexistent")
        .output()
        .expect("clroom must run")
}

#[test]
fn provider_info_reports_absent_provider_without_launching_it() {
    let output = run_without_providers(&["info", "codex"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Provider: Codex (not installed)\n"));
    assert!(stdout.contains("Clean launch: unqualified\n"));
    assert!(stdout.contains("Clean launch reason: PROVIDER_NOT_INSTALLED\n"));
    assert!(stdout.contains("Browser: unknown / not selectable / unqualified"));
    assert!(stdout.contains("Plugins: unknown / not selectable / unqualified"));
    assert!(stdout.contains("MCP: unknown / not selectable / unqualified"));
    assert!(stdout.contains("Resources: provider-level report only; detailed inventory not inspected\n"));
    assert!(stdout.contains("Schema: clroom.provider-info.v1\n"));
}

#[test]
fn provider_info_json_is_one_versioned_document_on_stdout() {
    let output = run_without_providers(&["--output", "json", "info", "claude"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], "clroom.provider-info.v1");
    assert_eq!(value["provider"]["id"], "claude");
    assert_eq!(value["provider"]["installed"], false);
    assert_eq!(value["clean_launch"]["qualification"], "unqualified");
    assert_eq!(
        value["clean_launch"]["reason_code"],
        "PROVIDER_NOT_INSTALLED"
    );
    assert_eq!(value["capabilities"].as_array().unwrap().len(), 3);
    assert!(value["resources"].as_array().unwrap().is_empty());
}

#[test]
fn provider_info_refuses_sensitive_arguments_before_dispatch() {
    for args in [
        vec!["info", "codex", "--api-key=secret"],
        vec!["--output", "json", "info", "codex", "--token=secret"],
    ] {
        let output = run_without_providers(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "ZERO_AUTH_ARGUMENT_REFUSAL: sensitive argument refused before dispatch; continue locally\n"
        );
    }
}

#[test]
fn provider_info_does_not_silently_accept_unimplemented_resource_targets() {
    let output = run_without_providers(&["info", "codex", "plugin:superpowers"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "INFO_RESOURCE_TARGETS_UNAVAILABLE: this build supports provider-level info only\n"
    );
}
