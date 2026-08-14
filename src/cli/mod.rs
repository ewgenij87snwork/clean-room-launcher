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

use std::io::Write;
use std::process::ExitCode;

pub fn run(invoked_as: &str, args: impl IntoIterator<Item = String>) -> ExitCode {
    let mut source = args.into_iter();
    let Some(first) = source.next() else {
        return run_local(invoked_as, Vec::new());
    };
    if let Some(exit) = external_prefix(&first, &mut source) {
        return exit;
    }
    if first == "--output"
        && let Some(format) = source.next()
    {
        if format == "json"
            && let Some(command) = source.next()
        {
            if let Some(exit) = external_prefix(&command, &mut source) {
                return exit;
            }
            return run_local(
                invoked_as,
                [first, format, command].into_iter().chain(source).collect(),
            );
        }
        return run_local(
            invoked_as,
            [first, format].into_iter().chain(source).collect(),
        );
    }

    run_local(
        invoked_as,
        std::iter::once(first).chain(source).collect::<Vec<_>>(),
    )
}

fn external_prefix(command: &str, source: &mut impl Iterator<Item = String>) -> Option<ExitCode> {
    let spec = help::resolve(command)?;
    if !matches!(
        spec.command,
        parser::Command::Provider | parser::Command::Generic
    ) {
        return None;
    }
    let generic_executable_present =
        !matches!(spec.command, parser::Command::Generic) || source.next().is_some();
    Some(external_refusal(spec.command, generic_executable_present))
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
            let screen = screen::render_unqualified(screen::PrepareReady {
                provider: "Codex",
                project: "project://offerstream",
                preview: "18 KB at start · 43 KB less (read-only measurement)",
                skills: "5 summaries now · 38 load on use · 4 unavailable",
            });
            println!("{}", screen.join("\n"));
            if screen::terminal_is_interactive() {
                if std::io::stdout().flush().is_err() {
                    eprintln!("INTERACTIVE_OUTPUT_FAILED");
                    return ExitCode::from(2);
                }
                match screen::read_unqualified_action() {
                    Ok(screen::UnqualifiedAction::ContinueLocally) => {
                        return run_taskseal_owned_local(invoked_as, parser::Command::Status);
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
        command => return run_taskseal_owned_local(invoked_as, command),
    }

    ExitCode::SUCCESS
}

fn run_taskseal_owned_local(invoked_as: &str, command: parser::Command) -> ExitCode {
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
