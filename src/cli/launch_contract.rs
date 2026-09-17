use std::path::Path;

use clroom::adapters::claude::managed::Presence;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryState {
    Clean,
    Expanded,
    Unknown,
    NotLaunchable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provider {
    Codex,
    Claude,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexInvocation {
    Interactive,
    Exec(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchContract {
    pub provider: &'static str,
    pub argv: Vec<String>,
    pub boundary: BoundaryState,
    pub boundary_controls: Vec<&'static str>,
    pub managed: Option<Presence>,
    pub user_or_provider_model_choice: bool,
}

impl LaunchContract {
    pub fn codex(user_args: &[String]) -> Self {
        Self::codex_with_pass_env(user_args, &[])
    }

    pub fn codex_with_pass_env(user_args: &[String], pass_env: &[String]) -> Self {
        let (boundary, boundary_controls, model_choice) =
            analyze(Provider::Codex, user_args, !pass_env.is_empty());
        let mut argv = vec![
            "--disable".to_owned(),
            "apps".to_owned(),
            "--disable".to_owned(),
            "hooks".to_owned(),
            "--disable".to_owned(),
            "plugins".to_owned(),
        ];
        argv.extend_from_slice(user_args);
        Self {
            provider: "codex",
            argv,
            boundary,
            boundary_controls,
            managed: None,
            user_or_provider_model_choice: model_choice,
        }
    }

    pub fn claude(user_args: &[String], add_dir: &Path, managed: Presence) -> Self {
        Self::claude_with_pass_env(user_args, add_dir, managed, &[])
    }

    pub fn claude_with_pass_env(
        user_args: &[String],
        add_dir: &Path,
        managed: Presence,
        pass_env: &[String],
    ) -> Self {
        let mut argv = vec![
            "--setting-sources".to_owned(),
            "".to_owned(),
            "--add-dir".to_owned(),
            add_dir.display().to_string(),
        ];
        argv.extend_from_slice(user_args);
        let (mut boundary, boundary_controls, model_choice) =
            analyze(Provider::Claude, user_args, !pass_env.is_empty());
        if boundary == BoundaryState::Clean && managed != Presence::Absent {
            boundary = BoundaryState::Unknown;
        }
        Self {
            provider: "claude",
            argv,
            boundary,
            boundary_controls,
            managed: Some(managed),
            user_or_provider_model_choice: model_choice,
        }
    }
}

fn analyze(
    provider: Provider,
    args: &[String],
    environment_expanded: bool,
) -> (BoundaryState, Vec<&'static str>, bool) {
    let mut controls = Vec::new();
    let mut model_choice = false;
    if environment_expanded {
        controls.push("environment");
    }
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" {
            break;
        }
        match provider {
            Provider::Codex => {
                if matches!(argument.as_str(), "--model" | "-m") {
                    model_choice = true;
                    index += 2;
                    continue;
                }
                if let Some(value) = argument.strip_prefix("--model=") {
                    model_choice |= !value.is_empty();
                    index += 1;
                    continue;
                }
                if matches!(
                    argument.as_str(),
                    "--enable"
                        | "--disable"
                        | "-c"
                        | "--config"
                        | "--profile"
                        | "--add-dir"
                        | "--sandbox"
                        | "--ask-for-approval"
                ) {
                    if !matches!(argument.as_str(), "--disable") {
                        controls.push(control_for(argument));
                    }
                    index += 2;
                    continue;
                }
                if argument.starts_with("--enable=") {
                    controls.push("feature");
                    index += 1;
                    continue;
                }
                if argument.starts_with("--disable=") {
                    index += 1;
                    continue;
                }
                if argument.starts_with('-') {
                    return (BoundaryState::Unknown, controls, model_choice);
                }
            }
            Provider::Claude => {
                if argument == "--model" {
                    model_choice = true;
                    index += 2;
                    continue;
                }
                if let Some(value) = argument.strip_prefix("--model=") {
                    model_choice |= !value.is_empty();
                    index += 1;
                    continue;
                }
                if matches!(
                    argument.as_str(),
                    "--setting-sources"
                        | "--mcp-config"
                        | "--add-dir"
                        | "--permission-mode"
                ) {
                    controls.push(control_for(argument));
                    index += 2;
                    continue;
                }
                if argument == "--dangerously-skip-permissions" {
                    controls.push("permissions");
                    index += 1;
                    continue;
                }
                if argument == "--chrome" {
                    controls.push("browser");
                    index += 1;
                    continue;
                }
                if argument == "--no-chrome" {
                    index += 1;
                    continue;
                }
                if argument.starts_with('-') {
                    return (BoundaryState::Unknown, controls, model_choice);
                }
            }
        }
        index += 1;
    }
    controls.sort_unstable();
    controls.dedup();
    let boundary = if controls.is_empty() {
        BoundaryState::Clean
    } else {
        BoundaryState::Expanded
    };
    (boundary, controls, model_choice)
}

fn control_for(argument: &str) -> &'static str {
    match argument {
        "--enable" => "feature",
        "-c" | "--config" => "config",
        "--profile" => "profile",
        "--add-dir" => "filesystem",
        "--sandbox" => "sandbox",
        "--ask-for-approval" => "approval",
        "--setting-sources" => "settings",
        "--mcp-config" => "mcp",
        "--permission-mode" => "permissions",
        _ => "provider",
    }
}

pub fn classify_codex_invocation(args: &[String]) -> CodexInvocation {
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" {
            break;
        }
        if matches!(argument.as_str(), "exec" | "e") {
            return CodexInvocation::Exec(index);
        }
        if matches!(
            argument.as_str(),
            "--enable"
                | "--disable"
                | "-c"
                | "--config"
                | "--profile"
                | "--add-dir"
                | "--sandbox"
                | "--ask-for-approval"
                | "--model"
                | "-m"
        ) {
            index += 2;
            continue;
        }
        if argument.starts_with("--enable=")
            || argument.starts_with("--disable=")
            || argument.starts_with("--model=")
        {
            index += 1;
            continue;
        }
        if argument.starts_with('-') {
            return CodexInvocation::Interactive;
        }
        break;
    }
    CodexInvocation::Interactive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_codex_launch_keeps_plugins_apps_and_hooks_disabled() {
        let contract = LaunchContract::codex(&[]);
        assert_eq!(contract.boundary, BoundaryState::Clean);
        assert_eq!(
            contract.argv,
            ["--disable", "apps", "--disable", "hooks", "--disable", "plugins"]
        );
    }

    #[test]
    fn explicit_plugin_enable_is_boundary_expanded_without_changing_clean_defaults() {
        let args = vec!["--enable".to_owned(), "plugins".to_owned()];
        let contract = LaunchContract::codex(&args);
        assert_eq!(contract.boundary, BoundaryState::Expanded);
        assert!(contract.boundary_controls.contains(&"feature"));
        assert_eq!(&contract.argv[6..], args);
    }

    #[test]
    fn unsafe_provider_shortcuts_are_unknown() {
        let contract = LaunchContract::codex(&["--future-sandbox-boundary".to_owned()]);
        assert_eq!(contract.boundary, BoundaryState::Unknown);
    }

    #[test]
    fn claude_browser_flag_expands_boundary_and_disable_is_clean() {
        let enabled = LaunchContract::claude(
            &["--chrome".to_owned()],
            Path::new("/tmp/selected-projection"),
            Presence::Absent,
        );
        assert_eq!(enabled.boundary, BoundaryState::Expanded);
        assert!(enabled.boundary_controls.contains(&"browser"));

        let disabled = LaunchContract::claude(
            &["--no-chrome".to_owned()],
            Path::new("/tmp/selected-projection"),
            Presence::Absent,
        );
        assert_eq!(disabled.boundary, BoundaryState::Clean);
    }

    #[test]
    fn claude_context_and_tool_flags_are_unknown() {
        for argument in ["--tools", "--allowedTools", "--disallowedTools", "--agents"] {
            let contract = LaunchContract::claude(
                &[argument.to_owned()],
                Path::new("/tmp/selected-projection"),
                Presence::Absent,
            );
            assert_eq!(contract.boundary, BoundaryState::Unknown, "{argument}");
        }
    }

    #[test]
    fn claude_pass_env_expands_contract_without_forwarding_launcher_options() {
        let contract = LaunchContract::claude_with_pass_env(
            &[],
            Path::new("/tmp/selected-projection"),
            Presence::Absent,
            &["EXTRA".to_owned()],
        );
        assert_eq!(contract.boundary, BoundaryState::Expanded);
        assert!(contract.boundary_controls.contains(&"environment"));
        assert!(!contract.argv.iter().any(|argument| argument == "--pass-env"));
    }

    #[test]
    fn codex_exec_gets_native_clean_user_config_suppression_after_subcommand() {
        let args = vec!["exec".to_owned(), "echo ready".to_owned()];
        let contract = LaunchContract::codex(&args);
        assert_eq!(classify_codex_invocation(&args), CodexInvocation::Exec(0));
        assert_eq!(&contract.argv[6..], args);
    }

    #[test]
    fn codex_exec_does_not_duplicate_user_supplied_clean_user_config_suppression() {
        let args = vec![
            "exec".to_owned(),
            "--ignore-user-config".to_owned(),
            "echo ready".to_owned(),
        ];
        assert_eq!(classify_codex_invocation(&args), CodexInvocation::Exec(0));
    }

    #[test]
    fn codex_exec_inserts_suppression_before_literal_after_terminator() {
        let args = vec![
            "exec".to_owned(),
            "--".to_owned(),
            "--ignore-user-config".to_owned(),
        ];
        assert_eq!(classify_codex_invocation(&args), CodexInvocation::Exec(0));
    }

    #[test]
    fn codex_invocation_classification_is_fail_closed_for_interactive_paths() {
        assert_eq!(classify_codex_invocation(&[]), CodexInvocation::Interactive);
        assert_eq!(
            classify_codex_invocation(&["--future".to_owned()]),
            CodexInvocation::Interactive
        );
    }
}
