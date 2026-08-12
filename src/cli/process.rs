use std::process::{Command, ExitCode, Stdio};

pub enum ProviderExit {
    Code(ExitCode),
    TerminatedBySignal,
}

pub fn run_foreground(program: &str, args: &[String]) -> Result<ProviderExit, String> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|_| format!("PROVIDER_SPAWN_FAILED: {program}"))?
        .wait()
        .map_err(|_| format!("PROVIDER_WAIT_FAILED: {program}"))?;

    match status.code() {
        Some(code @ 0..=255) => Ok(ProviderExit::Code(ExitCode::from(code as u8))),
        Some(_) => Err(format!("PROVIDER_EXIT_OUT_OF_RANGE: {program}")),
        None => Ok(ProviderExit::TerminatedBySignal),
    }
}
