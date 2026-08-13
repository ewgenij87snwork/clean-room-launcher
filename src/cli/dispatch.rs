use std::process::ExitCode;

use super::help::Command;
use super::process;

pub fn run(command: Command, generic_executable_present: bool) -> Result<ExitCode, String> {
    match command {
        Command::Generic if !generic_executable_present => {
            Err("GENERIC_EXECUTABLE_REQUIRED: use tseal -- <executable> [args...]".to_owned())
        }
        Command::Provider | Command::Generic => process::refuse_external_execution(),
        _ => unreachable!("dispatch only accepts provider or generic commands"),
    }
}
