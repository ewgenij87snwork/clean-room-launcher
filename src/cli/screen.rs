use std::{
    fs,
    io::{self, IsTerminal},
    path::Path,
};

pub struct PrepareReady {
    pub provider: &'static str,
}

pub struct RenderContext {
    pub width: usize,
    pub interactive: bool,
    pub plain: bool,
}

#[cfg(test)]
pub fn render_launch_contract(
    boundary_label: &str,
    managed_label: Option<&str>,
    boundary_controls: &[&str],
    user_or_provider_model_choice: bool,
) -> Vec<String> {
    let mut lines = vec![format!("Boundary: {boundary_label}")];
    if let Some(state) = managed_label {
        lines.push(format!("Managed: managed {state}"));
    }
    if !boundary_controls.is_empty() {
        lines.push(format!(
            "Boundary controls: {}",
            boundary_controls.join(", ")
        ));
    }
    if user_or_provider_model_choice {
        lines.push("Model: user/provider model choice".to_owned());
    }
    lines
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlaqueFeatureState {
    apps: bool,
    hooks: bool,
    plugins: bool,
}

impl PlaqueFeatureState {
    pub fn from_provider_args(args: &[String]) -> Self {
        let mut state = Self::default();
        let mut args = args.iter();
        while let Some(argument) = args.next() {
            if argument == "--" {
                break;
            }
            if matches!(argument.as_str(), "--enable" | "--disable") {
                if let Some(feature) = args.next() {
                    state.set(feature, argument == "--enable");
                }
            } else if let Some(feature) = argument.strip_prefix("--enable=") {
                state.set(feature, true);
            } else if let Some(feature) = argument.strip_prefix("--disable=") {
                state.set(feature, false);
            }
        }
        state
    }

    fn set(&mut self, feature: &str, enabled: bool) {
        match feature {
            "apps" => self.apps = enabled,
            "hooks" => self.hooks = enabled,
            "plugins" => self.plugins = enabled,
            _ => {}
        }
    }

    fn apps_label(self) -> &'static str {
        if self.apps { "on" } else { "off" }
    }

    fn hooks_plugins_label(self) -> String {
        match (self.hooks, self.plugins) {
            (false, false) => "off".to_owned(),
            (true, true) => "on".to_owned(),
            (true, false) => "on/off".to_owned(),
            (false, true) => "off/on".to_owned(),
        }
    }
}

pub fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(width, _)| usize::from(width.0))
        .unwrap_or(80)
}

pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
}

pub fn render_isolated_preview(
    project: &Path,
    selected_global_skills: usize,
    feature_state: PlaqueFeatureState,
) -> Vec<String> {
    render_isolated_preview_for(
        project,
        selected_global_skills,
        feature_state,
        RenderContext {
            width: terminal_width(),
            interactive: is_interactive(),
            plain: false,
        },
    )
}

pub fn render_claude_preview(project: &Path, selected_global_skills: usize) -> Vec<String> {
    render_claude_preview_for(
        project,
        selected_global_skills,
        RenderContext {
            width: terminal_width(),
            interactive: is_interactive(),
            plain: false,
        },
    )
}

pub fn render_isolated_preview_for(
    project: &Path,
    selected_global_skills: usize,
    feature_state: PlaqueFeatureState,
    context: RenderContext,
) -> Vec<String> {
    let project_skills = count_project_skills(project, ".agents/skills");
    let rows = vec![
        ("Global AGENTS.md", "off".to_owned()),
        ("Global skills", count_or_off(selected_global_skills)),
        ("Apps", feature_state.apps_label().to_owned()),
        ("Hooks/plugins", feature_state.hooks_plugins_label()),
        ("Dev prompt", "off".to_owned()),
        ("Notifications", "off".to_owned()),
    ];
    render_plaque(rows, project_skills, context)
}

pub fn render_claude_preview_for(
    project: &Path,
    selected_global_skills: usize,
    context: RenderContext,
) -> Vec<String> {
    let project_skills = count_project_skills(project, ".claude/skills");
    let rows = vec![
        ("Global CLAUDE.md", "off".to_owned()),
        ("Global skills", count_or_off(selected_global_skills)),
        ("User settings", "off".to_owned()),
        ("Auto memory", "off".to_owned()),
    ];
    render_plaque(rows, project_skills, context)
}

fn count_project_skills(project: &Path, relative: &str) -> usize {
    let root = project.join(relative);
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .count()
}

fn count_or_off(count: usize) -> String {
    if count == 0 {
        "off".to_owned()
    } else {
        format!("{count} on")
    }
}

fn render_plaque(
    rows: Vec<(&str, String)>,
    project_skills: usize,
    context: RenderContext,
) -> Vec<String> {
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let left = "╓──○──╖";
    let bottom_left = "╙──○──╜";
    let width = 31usize;
    let header_text = format!(" CLEAN ROOM ─ {version} ");
    let header_fill = width.saturating_sub(header_text.chars().count() + 2);
    let mut lines = vec![String::new(), String::new(), String::new()];
    lines.push(format!("{left} ╭─{header_text}{}╮", "─".repeat(header_fill)));
    lines.push(format!("║░░░░░║⠒│{}│", " ".repeat(width)));
    for (label, value) in rows {
        let inner = format!("     {label:<17}{value:>7}     ");
        lines.push(format!("║░░░░░║⠒│{inner}│"));
    }
    lines.push(format!("║░░░░░║⠒│{}│", " ".repeat(width)));
    if project_skills == 0 {
        lines.push(format!("{bottom_left} ╰{}╯", "─".repeat(width)));
    } else {
        lines.push(format!("{bottom_left} ╰───────────╥───────╥───────────╯"));
        lines.push("        ╭───────────╨───────╨───────────╮".to_owned());
        lines.push(format!("        │     Project skills{project_skills:>4} on     │"));
        lines.push("        ╰───────────────────────────────╯".to_owned());
    }
    if context.plain {
        return lines;
    }
    if context.interactive {
        lines
            .into_iter()
            .map(|line| style_interactive_line(&line))
            .collect()
    } else {
        lines
    }
}

fn style_interactive_line(line: &str) -> String {
    if line.contains("CLEAN ROOM") {
        let line = line.replacen("╓──○──╖", "\u{1b}[2m╓──○──╖\u{1b}[0m", 1);
        let line = line.replacen("CLEAN ROOM", "\u{1b}[1;36mCLEAN ROOM\u{1b}[0m", 1);
        let remainder = line.find(" ─ v").unwrap_or(line.len());
        if remainder < line.len() {
            let (head, tail) = line.split_at(remainder);
            return format!("{head}\u{1b}[2m{tail}\u{1b}[0m");
        }
        line
    } else if line.starts_with("╙──○──╜") {
        format!("\u{1b}[2m{line}\u{1b}[0m")
    } else if line.starts_with("║░░░░░║⠒│") {
        let prefix = "║░░░░░║⠒│";
        let rest = &line[prefix.len()..];
        if let Some(label_start) = rest.find(|character: char| !character.is_whitespace()) {
            let after_start = &rest[label_start..];
            if let Some(label_end) = after_start.find("  ") {
                let label = &after_start[..label_end];
                let before = &rest[..label_start];
                let after = &after_start[label_end..];
                return format!(
                    "\u{1b}[2m{prefix}\u{1b}[0m{before}\u{1b}[1m{label}\u{1b}[0m{after}"
                );
            }
        }
        format!("\u{1b}[2m{prefix}\u{1b}[0m{rest}")
    } else {
        line.to_owned()
    }
}

pub fn render_prepare_ready(provider: &'static str) -> Vec<String> {
    vec![format!("Ready: {provider}")]
}
