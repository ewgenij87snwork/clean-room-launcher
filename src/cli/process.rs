use std::process::{Command, ExitCode};

pub const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally";

pub fn refuse_external_execution() -> Result<ExitCode, String> {
    Err(ZERO_AUTH_REFUSAL.to_owned())
}

/// Transfer control to the user's already-installed local Codex executable.
/// No environment values are inspected or copied: `Command` inherits the
/// parent environment and all three standard streams by default.
pub fn launch_codex(args: &[String]) -> Result<ExitCode, String> {
    let status = Command::new("codex").args(args).status().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "LOCAL_CODEX_UNAVAILABLE: executable 'codex' not found; continue locally".to_owned()
        } else {
            "LOCAL_CODEX_LAUNCH_FAILED: local executable could not be started; continue locally"
                .to_owned()
        }
    })?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
