use std::io::IsTerminal;

pub struct PrepareReady {
    pub provider: &'static str,
    pub project: &'static str,
    pub preview: &'static str,
    pub skills: &'static str,
}

pub struct RenderContext {
    pub width: usize,
    pub interactive: bool,
    pub plain: bool,
}

pub fn render_unqualified(ready: PrepareReady) -> Vec<String> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80);
    let interactive = std::io::stdout().is_terminal();
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
        "TaskSeal · provider qualification required".to_owned(),
        String::new(),
        format!("Project  {}", ready.project),
        format!("Preview  {}", ready.preview),
        format!("Skills   {}  Review", ready.skills),
        format!(
            "Provider {} · P06_REQUIRED · nothing launched",
            ready.provider
        ),
        "Writes   .taskseal/project.json + .taskseal/out/ · nothing yet".to_owned(),
        "Leaves   ~/.codex · ~/.claude · Git/provider state unchanged".to_owned(),
        String::new(),
    ];
    if !context.interactive {
        lines.extend([
            "Continue locally".to_owned(),
            "  tseal status · tseal scan · tseal prepare · tseal check".to_owned(),
        ]);
    } else if context.plain {
        lines.extend([
            "1. Continue locally  Recommended".to_owned(),
            "2. Review or change setup".to_owned(),
            "3. Exit".to_owned(),
            String::new(),
            "Enter number · Esc exit".to_owned(),
        ]);
    } else {
        lines.extend([
            "› Continue locally  Recommended".to_owned(),
            "  Review or change setup".to_owned(),
            "  Exit".to_owned(),
            String::new(),
            "Enter continue locally · Esc exit".to_owned(),
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

fn labelled_value(line: &str) -> Option<(&'static str, &str)> {
    [
        ("Project", "Project  "),
        ("Preview", "Preview  "),
        ("Skills", "Skills   "),
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
