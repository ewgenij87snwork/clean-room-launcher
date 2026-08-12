use std::process::ExitCode;

use super::help::Command;
use super::process::{self, ProviderExit};

pub fn run(command: Command, args: &[String]) -> Result<ExitCode, String> {
    let (program, tail) = match command {
        Command::Provider => return Err("P06_REQUIRED: provider tuple is not qualified".to_owned()),
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

    match process::run_foreground(program, tail)? {
        ProviderExit::Code(exit) => Ok(exit),
        ProviderExit::TerminatedBySignal => {
            eprintln!("PROVIDER_TERMINATED_BY_SIGNAL: {program}");
            Ok(ExitCode::from(1))
        }
    }
}
