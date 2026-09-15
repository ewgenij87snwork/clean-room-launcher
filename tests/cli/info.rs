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
    assert!(stdout.contains("Browser: unknown / not selectable / unqualified"));
    assert!(stdout.contains("Resources: no resource targets requested\n"));
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
    assert!(value["capabilities"].is_array());
    assert!(value["resources"].is_array());
    assert!(value["combined"]["qualified_closure"].is_array());
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
fn provider_info_accepts_separate_plugin_targets_without_enabling_them() {
    let output = run_without_providers(&[
        "--output",
        "json",
        "info",
        "codex",
        "plugin:alpha@market",
        "plugin:beta@market",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let resources = value["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0]["resource"]["installation"], "not_installed");
    assert_eq!(resources[0]["resource"]["selection"], "not_selectable");
    assert_eq!(resources[0]["qualified_closure"], serde_json::json!([]));
    assert_eq!(resources[1]["resource"]["installation"], "not_installed");
    assert_eq!(
        value["combined"]["conflicts"],
        serde_json::json!(["PLUGIN_NOT_INSTALLED"])
    );
}

#[test]
fn provider_info_rejects_comma_targets_and_non_plugin_target_kinds() {
    for args in [
        vec!["info", "codex", "plugin:a@market,plugin:b@market"],
        vec!["info", "codex", "mcp:server"],
        vec!["info", "codex", "plugin:../escape@market"],
    ] {
        let output = run_without_providers(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "INFO_RESOURCE_TARGET_INVALID: use separate plugin:<provider-resource-id> targets\n"
        );
    }
}
