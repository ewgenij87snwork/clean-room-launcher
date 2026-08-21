use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use taskseal::adapters::codex::isolation::IsolationPlan;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally";

const CODEX_CLEAN_DEFAULTS: &[&str] = &[
    "-c",
    "features.hooks=false",
    "-c",
    "features.plugins=false",
    "-c",
    "developer_instructions=\"\"",
    "-c",
    "notify=[]",
];

pub fn refuse_external_execution() -> Result<ExitCode, String> {
    Err(ZERO_AUTH_REFUSAL.to_owned())
}

pub fn resolve_codex_executable() -> Result<PathBuf, String> {
    let Some(paths) = env::var_os("PATH") else {
        return Err(local_codex_unavailable());
    };
    for directory in env::split_paths(&paths) {
        let candidate = directory.join("codex");
        if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
            return fs::canonicalize(candidate).map_err(|_| local_codex_unavailable());
        }
    }
    Err(local_codex_unavailable())
}

pub fn launch_isolated_codex(
    plan: &IsolationPlan,
    executable: &Path,
    args: &[String],
) -> Result<ExitCode, String> {
    let sandbox = Path::new("/usr/bin/sandbox-exec");
    if !fs::metadata(sandbox).is_ok_and(|metadata| metadata.is_file()) {
        return Err(
            "CLROOM_ISOLATION_UNAVAILABLE: macOS sandbox-exec is unavailable; continue locally"
                .to_owned(),
        );
    }
    let mut command = Command::new(sandbox);
    command
        .arg("-p")
        .arg(&plan.profile)
        .arg("--")
        .arg(executable)
        .args(CODEX_CLEAN_DEFAULTS)
        .args(args);
    #[cfg(unix)]
    {
        Err(isolated_launch_error(command.exec()))
    }
    #[cfg(not(unix))]
    {
        let status = command.status().map_err(isolated_launch_error)?;
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

fn isolated_launch_error(_: io::Error) -> String {
    "CLROOM_ISOLATED_LAUNCH_FAILED: macOS sandboxed Codex could not be started; continue locally"
        .to_owned()
}

fn local_codex_unavailable() -> String {
    "LOCAL_CODEX_UNAVAILABLE: executable 'codex' not found; continue locally".to_owned()
}
