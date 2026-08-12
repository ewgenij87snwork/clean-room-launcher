use std::io::IsTerminal;

pub enum Mode {
    Human,
    Json,
}

pub fn select(args: Vec<String>) -> Result<(Mode, Vec<String>), String> {
    if args.first().is_some_and(|arg| arg == "--output") {
        let Some(format) = args.get(1) else {
            return Err("OUTPUT_FORMAT_REQUIRED: use --output json".to_owned());
        };
        if format != "json" {
            return Err(format!(
                "UNKNOWN_OUTPUT_FORMAT: {format}; use --output json"
            ));
        }
        return Ok((Mode::Json, args[2..].to_vec()));
    }
    Ok((Mode::Human, args))
}

pub fn guided_json() -> &'static str {
    r#"{"command":"guided","next_action":"tseal prepare","schema_version":"taskseal.output.v1","status":"P06_REQUIRED"}"#
}

pub fn stdin_is_terminal() -> bool {
    std::io::stdin().is_terminal()
}
