#[allow(dead_code)]
pub(crate) mod consent;
mod dispatch;
mod doctor;
mod help;
mod output;
mod parser;
mod process;
mod screen;
#[allow(dead_code)] // T8 is the first user-flow consumer; T7 seals the store and its TDD contract.
pub(crate) mod state;

use std::process::ExitCode;

pub fn run(invoked_as: &str, args: impl IntoIterator<Item = String>) -> ExitCode {
    let args = args.into_iter().collect::<Vec<_>>();
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
    let command = match parser::parse(args.clone()) {
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
        }
        parser::Command::Doctor => match doctor::run(&args[1..]) {
            Ok(report) => println!("{report}"),
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
            return match dispatch::run(command, &args) {
                Ok(exit) => exit,
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::from(2)
                }
            };
        }
        _ => println!("{invoked_as}: command accepted"),
    }

    ExitCode::SUCCESS
}
