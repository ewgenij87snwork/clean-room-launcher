use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use taskseal::adapters::claude::{
    isolation::IsolationPlan as ClaudeIsolationPlan, projection::Projection,
};
use taskseal::adapters::codex::isolation::IsolationPlan;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally";

const CODEX_CLEAN_DEFAULTS: &[&str] = &[
    "-c",
    "features.apps=false",
    "-c",
    "features.hooks=false",
    "-c",
    "features.plugins=false",
    "-c",
    "developer_instructions=\"\"",
    "-c",
    "notify=[]",
];

const CLAUDE_CLEAN_DEFAULTS: &[&str] = &[
    "--model",
    "haiku",
    "--effort",
    "low",
    "--setting-sources",
    "project,local",
    "--strict-mcp-config",
];

#[cfg(unix)]
const CLAUDE_GATE_SCRIPT: &str = "clroom_tries=0\n\
                                  clroom_session=$2\n\
                                  while [ \"$clroom_tries\" -lt 3000 ]; do\n\
                                    IFS= read -r clroom_owner < \"$1\" || exit 125\n\
                                    if [ \"$clroom_owner\" = \"active:$$:$clroom_session\" ]; then\n\
                                      shift 2\n\
                                      exec \"$@\"\n\
                                    fi\n\
                                    clroom_parent=${clroom_owner#creating:}\n\
                                    clroom_parent=${clroom_parent%%:*}\n\
                                    [ \"$clroom_owner\" = \"creating:$clroom_parent:$clroom_session\" ] || exit 125\n\
                                    case \"$clroom_parent\" in ''|*[!0-9]*) exit 125 ;; esac\n\
                                    [ \"$clroom_parent\" -gt 0 ] 2>/dev/null || exit 125\n\
                                    /bin/kill -0 \"$clroom_parent\" 2>/dev/null || exit 125\n\
                                    clroom_tries=$((clroom_tries + 1))\n\
                                    /bin/sleep 0.01 || exit 125\n\
                                  done\n\
                                  exit 125\n";

pub fn refuse_external_execution() -> Result<ExitCode, String> {
    Err(ZERO_AUTH_REFUSAL.to_owned())
}

pub fn resolve_codex_executable() -> Result<PathBuf, String> {
    resolve_executable("codex", local_codex_unavailable)
}

pub fn resolve_claude_executable() -> Result<PathBuf, String> {
    resolve_executable("claude", local_claude_unavailable)
}

fn resolve_executable(executable: &str, unavailable: fn() -> String) -> Result<PathBuf, String> {
    let Some(paths) = env::var_os("PATH") else {
        return Err(unavailable());
    };
    for directory in env::split_paths(&paths) {
        let candidate = directory.join(executable);
        if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
            return fs::canonicalize(candidate).map_err(|_| unavailable());
        }
    }
    Err(unavailable())
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

pub fn launch_claude(
    plan: &ClaudeIsolationPlan,
    projection: &mut Projection,
    executable: &Path,
    args: &[String],
) -> Result<ExitCode, String> {
    let sandbox = Path::new("/usr/bin/sandbox-exec");
    if !fs::metadata(sandbox).is_ok_and(|metadata| metadata.is_file()) {
        return Err(
            "CLROOM_CLAUDE_ISOLATION_UNAVAILABLE: macOS sandbox-exec is unavailable; continue locally"
                .to_owned(),
        );
    }
    #[cfg(unix)]
    {
        let session_name = projection.session_name().ok_or_else(claude_launch_error)?;
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg(CLAUDE_GATE_SCRIPT)
            .arg("clroom-claude-gate")
            .arg(projection.owner_marker_path())
            .arg(session_name)
            .arg(sandbox)
            .arg("-p")
            .arg(&plan.profile)
            .arg("--")
            .arg(executable)
            .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
            .env("CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN", "1")
            .args(CLAUDE_CLEAN_DEFAULTS)
            .arg("--add-dir")
            .arg(&projection.add_dir)
            .args(args);
        let mut child = command.spawn().map_err(|_| claude_launch_error())?;

        if projection.activate_consumer(child.id()).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(claude_launch_error());
        }

        let status = match child.wait() {
            Ok(status) => status,
            Err(_) => return Err(claude_launch_error()),
        };
        projection.finish_after_consumer_exit();
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }

    #[cfg(not(unix))]
    let status = Command::new(sandbox)
        .arg("-p")
        .arg(&plan.profile)
        .arg("--")
        .arg(executable)
        .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
        .env("CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN", "1")
        .args(CLAUDE_CLEAN_DEFAULTS)
        .arg("--add-dir")
        .arg(&projection.add_dir)
        .args(args)
        .status()
        .map_err(|_| claude_launch_error())?;
    #[cfg(not(unix))]
    {
        projection.finish_after_consumer_exit();
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

fn claude_launch_error() -> String {
    "CLROOM_CLAUDE_LAUNCH_FAILED: installed Claude could not be started; continue locally"
        .to_owned()
}

fn isolated_launch_error(_: io::Error) -> String {
    "CLROOM_ISOLATED_LAUNCH_FAILED: macOS sandboxed Codex could not be started; continue locally"
        .to_owned()
}

fn local_codex_unavailable() -> String {
    "LOCAL_CODEX_UNAVAILABLE: executable 'codex' not found; continue locally".to_owned()
}

fn local_claude_unavailable() -> String {
    "LOCAL_CLAUDE_UNAVAILABLE: executable 'claude' not found; continue locally".to_owned()
}
