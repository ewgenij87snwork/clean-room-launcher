pub struct PrepareReady {
    pub provider: &'static str,
    pub project: &'static str,
    pub preview: &'static str,
    pub skills: &'static str,
}

pub fn render_unqualified(ready: PrepareReady) -> Vec<String> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80);
    let lines = vec![
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
        "Leaves   ~/.codex · ~/.claude · Git/provider settings unchanged".to_owned(),
        String::new(),
        "› Review or change setup  Recommended".to_owned(),
        "  Exit".to_owned(),
        String::new(),
        "Enter review · Esc exit".to_owned(),
    ];
    if width >= 80 {
        return lines;
    }
    lines
        .into_iter()
        .flat_map(|line| {
            let fields = line.split_once("  ").or_else(|| {
                line.strip_prefix("Provider ")
                    .map(|value| ("Provider", value))
            });
            if let Some((label, value)) = fields {
                let mut stacked = vec![label.to_owned()];
                stacked.extend(wrap(value, width));
                stacked
            } else {
                wrap(&line, width)
            }
        })
        .collect()
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
