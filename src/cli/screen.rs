use std::{
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnqualifiedAction {
    ContinueLocally,
    LaunchCodex,
    Stop,
}

pub fn terminal_is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn read_unqualified_action() -> io::Result<UnqualifiedAction> {
    let mut input = String::new();
    let bytes_read = std::io::stdin().read_line(&mut input)?;
    if bytes_read == 0 {
        Ok(UnqualifiedAction::Stop)
    } else {
        Ok(parse_unqualified_action(&input))
    }
}

pub fn parse_unqualified_action(input: &str) -> UnqualifiedAction {
    match input.trim() {
        "" | "1" => UnqualifiedAction::ContinueLocally,
        "2" => UnqualifiedAction::LaunchCodex,
        _ => UnqualifiedAction::Stop,
    }
}

pub fn render_unqualified(ready: PrepareReady) -> Vec<String> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80);
    let interactive = terminal_is_interactive();
    let plain = !interactive
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.eq_ignore_ascii_case("dumb"));
    render_unqualified_for(
        ready,
        RenderContext {
            width,
            interactive,
            plain,
        },
    )
}

pub fn render_unqualified_for(ready: PrepareReady, context: RenderContext) -> Vec<String> {
    let mut lines = vec![
        "Clean Room Launcher".to_owned(),
        "A clean-room launcher for coding-agent CLIs.".to_owned(),
        String::new(),
        "Command  clroom codex [ARGS...]".to_owned(),
        format!(
            "Provider {} · local preauthenticated session only · nothing launched",
            ready.provider
        ),
        "Writes   .clroom/project.json + .clroom/out/ · nothing yet".to_owned(),
        "Leaves   ~/.codex · Git/provider state unchanged".to_owned(),
        String::new(),
    ];
    if !context.interactive {
        lines.extend([
            "Continue locally".to_owned(),
            "  clroom status · clroom scan · clroom prepare · clroom check".to_owned(),
        ]);
    } else if context.plain {
        lines.extend([
            "1. Continue locally  Recommended".to_owned(),
            "2. Launch Codex".to_owned(),
            String::new(),
            "Enter continue locally · 2 launch Codex".to_owned(),
        ]);
    } else {
        lines.extend([
            "› Continue locally  Recommended".to_owned(),
            "  Launch Codex".to_owned(),
            String::new(),
            "Enter continue locally · 2 launch Codex".to_owned(),
        ]);
    }
    if context.width >= 80 {
        return lines;
    }
    lines
        .into_iter()
        .flat_map(|line| {
            if line.is_empty() {
                return vec![line];
            }
            if let Some((label, value)) = labelled_value(&line) {
                let mut stacked = vec![label.to_owned()];
                stacked.extend(wrap(value, context.width));
                stacked
            } else if line.chars().count() <= context.width {
                vec![line]
            } else {
                wrap(&line, context.width)
            }
        })
        .collect()
}

pub fn render_isolated_preview(project: &Path, provider: &str) -> Vec<String> {
    vec![
        "Clean Room Launcher".to_owned(),
        format!("Project  {}", project.display()),
        "Protects global instructions and ambient skills".to_owned(),
        format!("Provider {provider} · existing provider state stays untouched"),
        "Action   Launch Codex".to_owned(),
    ]
}

fn labelled_value(line: &str) -> Option<(&'static str, &str)> {
    [
        ("Command", "Command  "),
        ("Provider", "Provider "),
        ("Writes", "Writes   "),
        ("Leaves", "Leaves   "),
    ]
    .into_iter()
    .find_map(|(label, prefix)| line.strip_prefix(prefix).map(|value| (label, value)))
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next_len = line.chars().count() + usize::from(!line.is_empty()) + word.chars().count();
        if next_len > width && !line.is_empty() {
            lines.push(line);
            line = word.to_owned();
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
