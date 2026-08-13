use std::process::ExitCode;

use super::help::Command;
use super::process;

pub fn run(command: Command, args: &[String]) -> Result<ExitCode, String> {
    match command {
        Command::Generic if args.get(1).is_none() => {
            Err("GENERIC_EXECUTABLE_REQUIRED: use tseal -- <executable> [args...]".to_owned())
        }
        Command::Provider | Command::Generic => process::refuse_external_execution(),
        _ => unreachable!("dispatch only accepts provider or generic commands"),
    }
}
