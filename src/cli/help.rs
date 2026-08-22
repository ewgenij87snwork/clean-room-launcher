#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Guided,
    Provider,
    Generic,
    Status,
    Starts,
    Scan,
    Init,
    Prepare,
    Check,
    Explain,
    Inspect,
    Doctor,
    Help,
}

pub struct CommandSpec {
    pub command: Command,
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
    pub summary: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub example: &'static str,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        command: Command::Help,
        canonical: "help",
        aliases: &["--help", "-h"],
        summary: "Show concise help",
        description: "Show concise help",
        usage: "help [COMMAND]",
        example: "help inspect",
    },
    CommandSpec {
        command: Command::Provider,
        canonical: "codex",
        aliases: &[],
        summary: "Launch the installed Codex CLI natively",
        description: "Launch the installed Codex CLI with unchanged arguments",
        usage: "codex [ARGS...]",
        example: "codex --help",
    },
    CommandSpec {
        command: Command::Generic,
        canonical: "--",
        aliases: &[],
        summary: "Refuse external execution under zero-auth",
        description: "Refuse external execution under zero-auth",
        usage: "-- <executable> [ARGS...]",
        example: "-- executable --help",
    },
    CommandSpec {
        command: Command::Status,
        canonical: "status",
        aliases: &[],
        summary: "Show current status",
        description: "Show current status",
        usage: "status",
        example: "status",
    },
    CommandSpec {
        command: Command::Starts,
        canonical: "starts",
        aliases: &["start"],
        summary: "Show saved starts",
        description: "Show saved starts",
        usage: "starts",
        example: "starts",
    },
    CommandSpec {
        command: Command::Scan,
        canonical: "scan",
        aliases: &[],
        summary: "Scan local inputs",
        description: "Scan local inputs",
        usage: "scan",
        example: "scan",
    },
    CommandSpec {
        command: Command::Init,
        canonical: "init",
        aliases: &[],
        summary: "Initialize local setup",
        description: "Initialize local setup",
        usage: "init",
        example: "init",
    },
    CommandSpec {
        command: Command::Prepare,
        canonical: "prepare",
        aliases: &[],
        summary: "Prepare local context",
        description: "Prepare local context",
        usage: "prepare",
        example: "prepare",
    },
    CommandSpec {
        command: Command::Check,
        canonical: "check",
        aliases: &[],
        summary: "Check local context",
        description: "Check local context",
        usage: "check",
        example: "check",
    },
    CommandSpec {
        command: Command::Explain,
        canonical: "explain",
        aliases: &[],
        summary: "Explain a skill decision",
        description: "Explain one skill decision",
        usage: "explain <skill>",
        example: "explain skill:rust",
    },
    CommandSpec {
        command: Command::Inspect,
        canonical: "inspect",
        aliases: &[],
        summary: "Inspect a skill decision",
        description: "Inspect one skill decision",
        usage: "inspect <skill>",
        example: "inspect skill:rust",
    },
    CommandSpec {
        command: Command::Doctor,
        canonical: "doctor",
        aliases: &[],
        summary: "Check installation safely",
        description: "Check installation safely",
        usage: "doctor --root <path>",
        example: "doctor --root .",
    },
];

pub fn resolve(token: &str) -> Option<&'static CommandSpec> {
    COMMANDS
        .iter()
        .find(|spec| spec.canonical == token || spec.aliases.contains(&token))
}

pub fn respond(args: &[String], invoked_as: &str) -> Result<Option<String>, String> {
    let Some(first) = args.first() else {
        return Ok(None);
    };
    if matches!(first.as_str(), "help" | "--help" | "-h") {
        return match args.get(1) {
            None => Ok(Some(top(invoked_as))),
            Some(subject) => card(invoked_as, subject)
                .map(Some)
                .ok_or_else(|| unknown_topic(subject, invoked_as)),
        };
    }
    if args.len() == 2
        && matches!(args[1].as_str(), "--help" | "-h")
        && let Some(spec) = resolve(first)
        && !matches!(spec.command, Command::Provider | Command::Generic)
    {
        return Ok(Some(render_card(invoked_as, spec)));
    }
    Ok(None)
}

pub fn top(invoked_as: &str) -> String {
    let help = resolve("help").expect("help registry entry is required");
    let inspect = resolve("inspect").expect("inspect registry entry is required");
    let explain = resolve("explain").expect("explain registry entry is required");
    let doctor = resolve("doctor").expect("doctor registry entry is required");
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("$HOME"));
    let skill_sets_path = super::skill_sets::config_path(&home)
        .unwrap_or_else(|_| std::path::PathBuf::from("$HOME/.config/clroom/skill-sets.yaml"));
    format!(
        "Clean Room Launcher\nA clean-room launcher for coding-agent CLIs.\n\nUsage: {invoked_as} [COMMAND]\n\nStart with:\n  {invoked_as}                 Review setup before a provider launch\n  {invoked_as} codex [ARGS...] Launch the installed Codex CLI natively\n  {invoked_as} {doctor} --root . {doctor_summary}\n\nChoose skills for one launch:\n  {invoked_as} codex --skill-set=code-review,@review\n  Direct skills, namespaces, and named @sets share this one option.\n  Skill sets file: {skill_sets_path}\n\nLearn:\n  {invoked_as} {help} {inspect}    Explain an available skill decision\n\nCommands:\n  {help} [COMMAND]        {help_summary}\n  {inspect} <skill>       {inspect_summary}\n  {explain} <skill>       {explain_summary}",
        doctor = doctor.canonical,
        doctor_summary = doctor.summary,
        help = help.canonical,
        inspect = inspect.canonical,
        help_summary = help.summary,
        inspect_summary = inspect.summary,
        explain = explain.canonical,
        explain_summary = explain.summary,
        skill_sets_path = skill_sets_path.display(),
    )
}

pub fn card(invoked_as: &str, token: &str) -> Option<String> {
    resolve(token).map(|spec| render_card(invoked_as, spec))
}

fn render_card(invoked_as: &str, spec: &CommandSpec) -> String {
    format!(
        "Clean Room Launcher — {}\n\n{} without changing state.\n\nUsage: {invoked_as} {}\n\nExample:\n  {invoked_as} {}\n\nFor more: {invoked_as} help",
        spec.canonical, spec.description, spec.usage, spec.example
    )
}

fn unknown_topic(topic: &str, invoked_as: &str) -> String {
    format!("UNKNOWN_HELP_TOPIC: {topic}; try \"{invoked_as} help\"")
}
