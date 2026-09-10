#[path = "../cli/mod.rs"]
mod cli;

fn main() -> std::process::ExitCode {
    cli::run("clroom-claude", std::env::args().skip(1))
}
