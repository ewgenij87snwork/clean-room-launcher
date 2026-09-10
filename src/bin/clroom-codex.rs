#[path = "../cli/mod.rs"]
mod cli;

fn main() -> std::process::ExitCode {
    cli::run("clroom-codex", std::env::args().skip(1))
}
