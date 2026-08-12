#[path = "../cli/mod.rs"]
mod cli;

fn main() -> std::process::ExitCode {
    cli::run("taskseal", std::env::args().skip(1))
}
