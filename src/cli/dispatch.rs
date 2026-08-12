use std::process::{Command as ProcessCommand, ExitCode};

use super::help::Command;

pub fn run(command: Command, args: &[String]) -> Result<ExitCode, String> {
    let (program, tail) = match command {
        Command::Provider => (args[0].as_str(), &args[1..]),
        Command::Generic => {
            let Some(program) = args.get(1) else {
                return Err(
                    "GENERIC_EXECUTABLE_REQUIRED: use tseal -- <executable> [args...]".to_owned(),
                );
            };
            (program.as_str(), &args[2..])
        }
        _ => unreachable!("dispatch only accepts provider or generic commands"),
    };

    let status = ProcessCommand::new(program)
        .args(tail)
        .status()
        .map_err(|_| format!("PROVIDER_SPAWN_FAILED: {program}"))?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
