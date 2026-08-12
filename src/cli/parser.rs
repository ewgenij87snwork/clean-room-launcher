#[derive(Debug, Eq, PartialEq)]
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

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let Some(first) = args.into_iter().next() else {
        return Ok(Command::Guided);
    };

    match first.as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "codex" | "claude" => Ok(Command::Provider),
        "--" => Ok(Command::Generic),
        "status" => Ok(Command::Status),
        "starts" | "start" => Ok(Command::Starts),
        "scan" => Ok(Command::Scan),
        "init" => Ok(Command::Init),
        "prepare" => Ok(Command::Prepare),
        "check" => Ok(Command::Check),
        "explain" => Ok(Command::Explain),
        "inspect" => Ok(Command::Inspect),
        "doctor" => Ok(Command::Doctor),
        unknown => Err(format!("UNKNOWN_COMMAND: {unknown}; try help")),
    }
}
