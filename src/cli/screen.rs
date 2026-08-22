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
        "" | "1" => UnqualifiedAction::LaunchCodex,
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
            "Provider {} · already installed local CLI · nothing launched",
            ready.provider
        ),
        "Writes   no launcher state".to_owned(),
        "Leaves   existing provider state unchanged".to_owned(),
        String::new(),
    ];
    if !context.interactive {
        lines.extend([
            "Preview only · provider not launched".to_owned(),
            "  clroom codex [ARGS...] to launch explicitly".to_owned(),
        ]);
    } else if context.plain {
        lines.extend([
            "1. Launch Codex  Recommended".to_owned(),
            String::new(),
            "Enter launch Codex · q stop".to_owned(),
        ]);
    } else {
        lines.extend([
            "› Launch Codex  Recommended".to_owned(),
            String::new(),
            "Enter launch Codex · q stop".to_owned(),
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

pub fn render_isolated_preview(project: &Path, selected_global_skills: usize) -> Vec<String> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80);
    let interactive = std::io::stderr().is_terminal();
    let plain = !interactive
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.eq_ignore_ascii_case("dumb"));
    render_isolated_preview_for(
        project,
        selected_global_skills,
        RenderContext {
            width,
            interactive,
            plain,
        },
    )
}

pub fn render_isolated_preview_for(
    _project: &Path,
    selected_global_skills: usize,
    context: RenderContext,
) -> Vec<String> {
    let styled = context.interactive && !context.plain;
    let panel_width = 33;
    let content_width = panel_width - 4;
    let title = "CLEAN ROOM";
    let title_border = "─".repeat(panel_width - title.chars().count() - 5);
    let border = "─".repeat(panel_width - 2);
    // Visual model: a wall-mounted boundary status plaque. The seven-cell mounting
    // rail stays fixed at the left; one-cell standoff markers connect it to the
    // right plaque. Plaque height follows `rows` automatically. If row text grows,
    // change `panel_width` and the exact receipt test together—never hand-pad height.
    let mounting_rail_top = "╓──○──╖";
    let mounting_rail_bottom = "╙──○──╜";
    let mounting_rail_fill = "║░░░░░║";
    let standoff_marker = "⠒";
    let mut lines = vec![String::new(), String::new(), String::new()];
    lines.push(if styled {
        format!(
            "\u{1b}[2m{mounting_rail_top}\u{1b}[0m \u{1b}[2m╭─ \u{1b}[0m\u{1b}[1;36m{title}\u{1b}[0m\u{1b}[2m {title_border}╮\u{1b}[0m"
        )
    } else {
        format!("{mounting_rail_top} ╭─ {title} {title_border}╮")
    });
    let skill_state = if selected_global_skills == 0 {
        "off".to_owned()
    } else {
        format!("{selected_global_skills} on")
    };
    let rows = vec![
        (String::new(), String::new()),
        (
            "    Global AGENTS.md  off".to_owned(),
            "    \u{1b}[1mGlobal AGENTS.md\u{1b}[0m  off".to_owned(),
        ),
        (
            format!("    Global skills{skill_state:>8}"),
            format!("    \u{1b}[1mGlobal skills\u{1b}[0m{skill_state:>8}"),
        ),
        (
            "    Apps              off".to_owned(),
            "    \u{1b}[1mApps\u{1b}[0m              off".to_owned(),
        ),
        (
            "    Hooks/plugins     off".to_owned(),
            "    \u{1b}[1mHooks/plugins\u{1b}[0m     off".to_owned(),
        ),
        (
            "    Dev prompt        off".to_owned(),
            "    \u{1b}[1mDev prompt\u{1b}[0m        off".to_owned(),
        ),
        (
            "    Notifications     off".to_owned(),
            "    \u{1b}[1mNotifications\u{1b}[0m     off".to_owned(),
        ),
        (String::new(), String::new()),
    ];
    for (plain, decorated) in rows {
        let padding = " ".repeat(content_width.saturating_sub(plain.chars().count()));
        lines.push(if styled {
            format!(
                "\u{1b}[2m{mounting_rail_fill}{standoff_marker}│\u{1b}[0m {}{padding} \u{1b}[2m│\u{1b}[0m",
                decorated
            )
        } else {
            format!("{mounting_rail_fill}{standoff_marker}│ {plain}{padding} │")
        });
    }
    lines.push(if styled {
        format!("\u{1b}[2m{mounting_rail_bottom} ╰{border}╯\u{1b}[0m")
    } else {
        format!("{mounting_rail_bottom} ╰{border}╯")
    });
    lines.push(String::new());
    lines
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
