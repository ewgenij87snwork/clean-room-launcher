mod dispatch;
mod doctor;
mod help;
mod parser;
mod screen;

use std::process::ExitCode;

pub fn run(invoked_as: &str, args: impl IntoIterator<Item = String>) -> ExitCode {
    let args = args.into_iter().collect::<Vec<_>>();
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
                project: "/Users/alex/work/offerstream",
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
