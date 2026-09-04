use std::{collections::BTreeSet, path::Path};

use taskseal::adapters::claude::managed::Presence;

const CODEX_CLEAN_DEFAULTS: &[&str] = &[
    "-c",
    "features.apps=false",
    "-c",
    "features.hooks=false",
    "-c",
    "features.plugins=false",
    "-c",
    "developer_instructions=\"\"",
    "-c",
    "notify=[]",
    "-c",
    "shell_environment_policy.inherit=\"none\"",
    "-c",
    "shell_environment_policy.include_only=[\"PATH\",\"HOME\",\"TMPDIR\",\"TERM\",\"COLORTERM\",\"LANG\",\"LC_ALL\",\"LC_CTYPE\",\"TZ\"]",
    "-c",
    "shell_environment_policy.ignore_default_excludes=false",
];

const CODEX_INCLUDE_ONLY_DEFAULTS: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "TERM",
    "COLORTERM",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TZ",
];

const CLAUDE_CLEAN_DEFAULTS: &[&str] = &[
    "--setting-sources",
    "project,local",
    "--strict-mcp-config",
    "--settings",
    "{\"sandbox\":{\"enabled\":true,\"failIfUnavailable\":true,\"allowUnsandboxedCommands\":false,\"excludedCommands\":[]}}",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    Codex,
    Claude,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryState {
    Clean,
    Expanded,
    Unknown,
    NotLaunchable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchContract {
    pub provider: Provider,
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
        let requested = pass_env.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let include_only = CODEX_INCLUDE_ONLY_DEFAULTS
            .iter()
            .copied()
            .chain(requested)
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(",");
        let include_only = format!("shell_environment_policy.include_only=[{include_only}]");
        let mut argv = CODEX_CLEAN_DEFAULTS
            .iter()
            .map(|argument| {
                if argument.starts_with("shell_environment_policy.include_only=") {
                    include_only.clone()
                } else {
                    (*argument).to_owned()
                }
            })
            .collect::<Vec<_>>();
        argv.extend_from_slice(user_args);
        let (boundary, boundary_controls, model_choice) = analyze(Provider::Codex, user_args);
        Self {
            provider: Provider::Codex,
            argv,
            boundary,
            boundary_controls,
            managed: None,
            user_or_provider_model_choice: model_choice,
        }
    }

    pub fn claude(user_args: &[String], add_dir: &Path, managed: Presence) -> Self {
        let mut argv = CLAUDE_CLEAN_DEFAULTS
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>();
        argv.push("--add-dir".to_owned());
        argv.push(add_dir.as_os_str().to_string_lossy().into_owned());
        argv.extend_from_slice(user_args);
        let (mut boundary, boundary_controls, model_choice) = analyze(Provider::Claude, user_args);
        if boundary == BoundaryState::Clean && managed != Presence::Absent {
            boundary = BoundaryState::Unknown;
        }
        Self {
            provider: Provider::Claude,
            argv,
            boundary,
            boundary_controls,
            managed: Some(managed),
            user_or_provider_model_choice: model_choice,
        }
    }

    pub fn boundary_label(&self) -> &'static str {
        match self.boundary {
            BoundaryState::Clean => "clean",
            BoundaryState::Expanded => "boundary expanded",
            BoundaryState::Unknown if self.managed != Some(Presence::Absent) => "managed/unknown",
            BoundaryState::Unknown => "unknown",
            BoundaryState::NotLaunchable => "not launchable",
        }
    }

    pub fn managed_label(&self) -> Option<&'static str> {
        self.managed.map(|managed| match managed {
            Presence::Present => "present",
            Presence::Absent => "absent",
            Presence::Unknown => "unknown",
        })
    }
}

fn analyze(provider: Provider, args: &[String]) -> (BoundaryState, Vec<&'static str>, bool) {
    let mut controls = Vec::new();
    let mut unknown = false;
    let mut model_choice = false;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].as_str();
        if argument == "--" {
            break;
        }
        let lower = argument.to_ascii_lowercase();
        let mut recognized = true;
        let control = match provider {
            Provider::Codex => match argument {
                "--yolo" => {
                    unknown = true;
                    None
                }
                "-c" | "--config" => Some("config"),
                "--profile" | "-p" => Some("profile"),
                "--add-dir" => Some("add-dir"),
                "--sandbox" | "-s" => Some("sandbox"),
                "--ask-for-approval" | "-a" => Some("approval"),
                "--config-file" | "--instructions" | "--project-doc" => Some("instructions/config"),
                "--mcp-server" => Some("mcp"),
                "--plugin" => Some("plugin"),
                "--hook" => Some("hook"),
                "--enable" => Some("feature"),
                "--model" | "-m" => {
                    model_choice = true;
                    None
                }
                _ if lower.starts_with("--enable=") => Some("feature"),
                _ => {
                    recognized = false;
                    None
                }
            },
            Provider::Claude => match argument {
                "--setting-sources" | "--settings" => Some("setting-sources"),
                "--mcp-config" => Some("mcp"),
                "--add-dir" => Some("add-dir"),
                "--permission-mode"
                | "--dangerously-skip-permissions"
                | "--allow-dangerously-skip-permissions" => Some("permissions/sandbox"),
                "--plugin-dir" | "--agents" | "--hooks" => Some("plugin/hook/agent"),
                "--model" => {
                    model_choice = true;
                    None
                }
                "--agent" => {
                    unknown = true;
                    None
                }
                _ => {
                    recognized = false;
                    None
                }
            },
        };
        if let Some(control) = control
            && !controls.contains(&control)
        {
            controls.push(control);
        }
        if !recognized
            && argument.starts_with('-')
            && (provider == Provider::Claude
                || [
                    "config",
                    "setting",
                    "mcp",
                    "plugin",
                    "hook",
                    "instruction",
                    "sandbox",
                    "permission",
                    "add-dir",
                    "profile",
                    "approval",
                ]
                .iter()
                .any(|word| lower.contains(word)))
        {
            unknown = true;
        }
        if matches!(
            argument,
            "-c" | "--config"
                | "--profile"
                | "-p"
                | "--add-dir"
                | "--sandbox"
                | "-s"
                | "--ask-for-approval"
                | "-a"
                | "--config-file"
                | "--instructions"
                | "--project-doc"
                | "--mcp-server"
                | "--plugin"
                | "--hook"
                | "--enable"
                | "--model"
                | "-m"
                | "--setting-sources"
                | "--settings"
                | "--mcp-config"
                | "--permission-mode"
                | "--plugin-dir"
                | "--agents"
                | "--hooks"
                | "--agent"
        ) {
            index += 1;
        }
        index += 1;
    }
    let boundary = if !controls.is_empty() {
        BoundaryState::Expanded
    } else if unknown {
        BoundaryState::Unknown
    } else {
        BoundaryState::Clean
    };
    (boundary, controls, model_choice)
}

#[cfg(test)]
mod tests {
    use super::{BoundaryState, LaunchContract, Presence};
    use std::path::Path;

    #[test]
    fn claude_context_and_tool_flags_are_unknown() {
        for flag in ["--system-prompt", "--append-system-prompt", "--tools"] {
            let contract = LaunchContract::claude(
                &[flag.to_owned(), "synthetic".to_owned()],
                Path::new("/tmp/view"),
                Presence::Absent,
            );
            assert_eq!(contract.boundary, BoundaryState::Unknown, "flag={flag}");
        }
    }

    #[test]
    fn unsafe_provider_shortcuts_are_unknown() {
        let codex = LaunchContract::codex(&["--yolo".to_owned()]);
        assert_eq!(codex.boundary, BoundaryState::Unknown);
        let claude = LaunchContract::claude(
            &["--agent".to_owned()],
            Path::new("/tmp/view"),
            Presence::Absent,
        );
        assert_eq!(claude.boundary, BoundaryState::Unknown);
    }
}
