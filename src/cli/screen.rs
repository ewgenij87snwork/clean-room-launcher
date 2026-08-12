pub struct PrepareReady {
    pub provider: &'static str,
    pub provider_version: &'static str,
    pub project: &'static str,
    pub preview: &'static str,
    pub skills: &'static str,
}

pub fn render_ready(ready: PrepareReady) -> Vec<String> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80);
    if width < 80 {
        return render_narrow(ready, width);
    }
    vec![
        format!(
            "TaskSeal · ready to prepare a clean start for {} {}",
            ready.provider, ready.provider_version
        ),
        String::new(),
        format!("Project  {}", ready.project),
        format!("Preview  {}", ready.preview),
        format!("Skills   {}  Review", ready.skills),
        "Writes   .taskseal/project.json + .taskseal/out/ · nothing yet".to_owned(),
        "Leaves   ~/.codex · ~/.claude · Git/provider settings unchanged".to_owned(),
        String::new(),
        format!("› Start {} with clean context  Recommended", ready.provider),
        "  Review or change setup".to_owned(),
        "  Exit".to_owned(),
        String::new(),
        "↑↓ select · Enter confirm · Esc exit".to_owned(),
    ]
}

fn render_narrow(ready: PrepareReady, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(wrap(
        &format!(
            "TaskSeal · ready to prepare a clean start for {} {}",
            ready.provider, ready.provider_version
        ),
        width,
    ));
    lines.push(String::new());
    for (label, value) in [
        ("Project", ready.project),
        ("Preview", ready.preview),
        ("Skills", ready.skills),
        (
            "Writes",
            ".taskseal/project.json + .taskseal/out/ · nothing yet",
        ),
        (
            "Leaves",
            "~/.codex · ~/.claude · Git/provider settings unchanged",
        ),
    ] {
        lines.push(label.to_owned());
        lines.extend(wrap(value, width));
    }
    lines.push(String::new());
    lines.extend(wrap(
        &format!("› Start {} with clean context  Recommended", ready.provider),
        width,
    ));
    lines.push("  Review or change setup".to_owned());
    lines.push("  Exit".to_owned());
    lines.push(String::new());
    lines.extend(wrap("↑↓ select · Enter confirm · Esc exit", width));
    lines
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
