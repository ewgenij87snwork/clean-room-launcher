#[allow(dead_code)]
pub(crate) mod consent;
mod dispatch;
mod doctor;
mod help;
mod output;
mod parser;
mod process;
mod screen;
mod starts;
#[allow(dead_code)] // T8 is the first user-flow consumer; T7 seals the store and its TDD contract.
pub(crate) mod state;
mod zero_auth;

use std::io::Write;
use std::process::ExitCode;

pub fn run(invoked_as: &str, args: impl IntoIterator<Item = String>) -> ExitCode {
    let mut source = args.into_iter();
    let Some(first) = (match next_argument(&mut source) {
        Ok(argument) => argument,
        Err(exit) => return exit,
    }) else {
        return run_local(invoked_as, Vec::new());
    };
    if first == "codex" {
        return run_codex(&mut source);
    }
    if let Some(exit) = external_prefix(&first) {
        return exit;
    }
    if first == "--output"
        && let Some(format) = (match next_argument(&mut source) {
            Ok(argument) => argument,
            Err(exit) => return exit,
        })
    {
        if format != "json" {
            return run_local(invoked_as, vec![first, format]);
        }
        if let Some(command) = match next_argument(&mut source) {
            Ok(argument) => argument,
            Err(exit) => return exit,
        } {
            if let Some(exit) = external_prefix(&command) {
                return exit;
            }
            return run_local(invoked_as, vec![first, format, command]);
        }
        return run_local(invoked_as, vec![first, format]);
    }

    match local_prefix(first, &mut source) {
        Ok(args) => run_local(invoked_as, args),
        Err(exit) => exit,
    }
}

fn run_codex(source: &mut impl Iterator<Item = String>) -> ExitCode {
    let first = match next_argument(source) {
        Ok(argument) => argument,
        Err(exit) => return exit,
    };
    if first
        .as_deref()
        .is_some_and(|argument| matches!(argument, "login" | "logout"))
    {
        return external_refusal(parser::Command::Provider, false);
    }
    let mut args = first.into_iter().collect::<Vec<_>>();
    while let Some(argument) = match next_argument(source) {
        Ok(argument) => argument,
        Err(exit) => return exit,
    } {
        args.push(argument);
    }
    match process::launch_codex(&args) {
        Ok(exit) => exit,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn next_argument(source: &mut impl Iterator<Item = String>) -> Result<Option<String>, ExitCode> {
    match source.next() {
        Some(argument) if zero_auth::is_sensitive_argument(&argument) => {
            eprintln!("{}", zero_auth::ARGUMENT_REFUSAL);
            Err(ExitCode::from(2))
        }
        argument => Ok(argument),
    }
}

fn local_prefix(
    first: String,
    source: &mut impl Iterator<Item = String>,
) -> Result<Vec<String>, ExitCode> {
    let additional = match first.as_str() {
        "help" | "--help" | "-h" | "explain" | "inspect" => 1,
        "doctor" | "start" => 2,
        _ => 0,
    };
    let mut args = vec![first];
    for _ in 0..additional {
        let Some(argument) = next_argument(source)? else {
            break;
        };
        args.push(argument);
    }
    Ok(args)
}

fn external_prefix(command: &str) -> Option<ExitCode> {
    let spec = help::resolve(command)?;
    if !matches!(
        spec.command,
        parser::Command::Provider | parser::Command::Generic
    ) {
        return None;
    }
    // Zero-auth closes the generic route at its boundary. Inspecting even the
    // nominal executable would consume a credential-shaped value in that slot.
    Some(external_refusal(spec.command, true))
}

fn external_refusal(command: parser::Command, generic_executable_present: bool) -> ExitCode {
    match dispatch::run(command, generic_executable_present) {
        Ok(exit) => exit,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run_local(invoked_as: &str, args: Vec<String>) -> ExitCode {
    let (output_mode, args) = match output::select(args) {
        Ok(selection) => selection,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    if matches!(output_mode, output::Mode::Json) {
        if args.is_empty() {
            println!("{}", output::guided_json());
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "OUTPUT_UNSUPPORTED_FOR_COMMAND: {}; use human output",
            args[0]
        );
        return ExitCode::from(2);
    }
    match help::respond(&args, invoked_as) {
        Ok(Some(output)) => {
            println!("{output}");
            return ExitCode::SUCCESS;
        }
        Ok(None) => {}
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    }
    let command = match parser::parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    match command {
        parser::Command::Help => unreachable!("help is handled before parsing"),
        parser::Command::Guided => {
            let screen = screen::render_unqualified(screen::PrepareReady { provider: "Codex" });
            println!("{}", screen.join("\n"));
            if screen::terminal_is_interactive() {
                if std::io::stdout().flush().is_err() {
                    eprintln!("INTERACTIVE_OUTPUT_FAILED");
                    return ExitCode::from(2);
                }
                match screen::read_unqualified_action() {
                    Ok(screen::UnqualifiedAction::ContinueLocally) => {
                        return run_launcher_owned_local(invoked_as, parser::Command::Status);
                    }
                    Ok(screen::UnqualifiedAction::LaunchCodex) => {
                        return match process::launch_codex(&[]) {
                            Ok(exit) => exit,
                            Err(message) => {
                                eprintln!("{message}");
                                ExitCode::from(2)
                            }
                        };
                    }
                    Ok(screen::UnqualifiedAction::Stop) => {}
                    Err(_) => {
                        eprintln!("INTERACTIVE_INPUT_FAILED");
                        return ExitCode::from(2);
                    }
                }
            }
        }
        parser::Command::Doctor => match doctor::run(&args[1..]) {
            Ok(report) => println!("{report}"),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        },
        parser::Command::Starts => match starts::run(&args) {
            Ok(output) => println!("{output}"),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        },
        parser::Command::Init if !output::stdin_is_terminal() => {
            eprintln!("NON_INTERACTIVE_INPUT_REQUIRED: rerun init interactively");
            return ExitCode::from(2);
        }
        parser::Command::Provider | parser::Command::Generic => {
            unreachable!("external commands refuse before tail collection")
        }
        command => return run_launcher_owned_local(invoked_as, command),
    }

    ExitCode::SUCCESS
}

fn run_launcher_owned_local(invoked_as: &str, command: parser::Command) -> ExitCode {
    assert!(matches!(
        command,
        parser::Command::Status
            | parser::Command::Scan
            | parser::Command::Init
            | parser::Command::Prepare
            | parser::Command::Check
            | parser::Command::Explain
            | parser::Command::Inspect
    ));
    println!("{invoked_as}: command accepted");
    ExitCode::SUCCESS
}
