use std::process::ExitCode;

pub const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally";

pub fn refuse_external_execution() -> Result<ExitCode, String> {
    Err(ZERO_AUTH_REFUSAL.to_owned())
}
