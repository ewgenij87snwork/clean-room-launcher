#[path = "../cli/mod.rs"]
mod cli;

fn main() -> std::process::ExitCode {
    cli::run("tseal", std::env::args().skip(1))
}
