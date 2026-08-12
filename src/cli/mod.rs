mod doctor;
mod parser;
mod screen;

use std::process::ExitCode;

pub fn run(invoked_as: &str, args: impl IntoIterator<Item = String>) -> ExitCode {
    let args = args.into_iter().collect::<Vec<_>>();
    let command = match parser::parse(args.clone()) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    match command {
        parser::Command::Help => {
            println!(
                "TaskSeal — guided AI CLI launcher\n\nUsage: {invoked_as} [COMMAND]\n\nTry: {invoked_as} help"
            );
        }
        parser::Command::Guided => {
            let screen = screen::render_ready(screen::PrepareReady {
                provider: "Codex",
                provider_version: "0.xx",
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
        _ => println!("{invoked_as}: command accepted"),
    }

    ExitCode::SUCCESS
}
