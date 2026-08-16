#[path = "../cli/mod.rs"]
mod cli;

fn main() -> std::process::ExitCode {
    cli::run("clroom", std::env::args().skip(1))
}
