use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use clroom::adapters::claude::{
    isolation::IsolationPlan as ClaudeIsolationPlan, projection::Projection,
};
use clroom::adapters::codex::isolation::{IsolationInputs, IsolationPlan, plan_with_skills};
use clroom::adapters::{
    identity::{ProviderIdentity, resolve_identity, revalidate_identity},
    session::ProviderNativePreauthenticatedSession,
};
use clroom::contracts::adapter::parse_declaration;

use super::launch_contract::{CodexInvocation, LaunchContract, classify_codex_invocation};

#[derive(Clone, Copy)]
enum ProviderEnvironment {
    Codex,
    Claude,
}

pub const INTERNAL_PROVIDER_CHAIN_GUARD: &str = "CLROOM_INTERNAL_PROVIDER_CHAIN";

fn parent_environment(
    provider: ProviderEnvironment,
    requested_names: &[String],
) -> Vec<(OsString, OsString)> {
    env::vars_os()
        .filter(|(name, _)| {
            let Some(name) = name.to_str() else {
                return false;
            };
            let common = matches!(
                name,
                "PATH"
                    | "HOME"
                    | "TMPDIR"
                    | "TERM"
                    | "COLORTERM"
                    | "LANG"
                    | "LC_ALL"
                    | "LC_CTYPE"
                    | "TZ"
                    | "HTTP_PROXY"
                    | "HTTPS_PROXY"
                    | "ALL_PROXY"
                    | "NO_PROXY"
                    | "http_proxy"
                    | "https_proxy"
                    | "all_proxy"
                    | "no_proxy"
            ) || name.starts_with("LC_");
            common
                || match provider {
                    ProviderEnvironment::Codex => matches!(
                        name,
                        "OPENAI_API_KEY" | "AZURE_OPENAI_API_KEY" | "CODEX_HOME"
                    ),
                    ProviderEnvironment::Claude => matches!(
                        name,
                        "ANTHROPIC_API_KEY"
                            | "ANTHROPIC_AUTH_TOKEN"
                            | "CLAUDE_CODE_OAUTH_TOKEN"
                            | "AWS_ACCESS_KEY_ID"
                            | "AWS_SECRET_ACCESS_KEY"
                            | "AWS_SESSION_TOKEN"
                            | "AWS_REGION"
                            | "AWS_PROFILE"
                            | "GOOGLE_APPLICATION_CREDENTIALS"
                            | "CLOUD_ML_REGION"
                            | "ANTHROPIC_VERTEX_PROJECT_ID"
                    ),
                }
                || requested_names.iter().any(|requested| requested == name)
        })
        .collect()
}

fn apply_parent_environment(
    command: &mut Command,
    provider: ProviderEnvironment,
    requested_names: &[String],
) {
    command
        .env_clear()
        .envs(parent_environment(provider, requested_names));
}

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const ZERO_AUTH_REFUSAL: &str = "ZERO_AUTH_REFUSAL: provider-native preauthenticated session unavailable or ambiguous; continue locally";

#[cfg(unix)]
const CLAUDE_GATE_SCRIPT: &str = "clroom_tries=0\n\
                                  clroom_session=$2\n\
                                  while [ \"$clroom_tries\" -lt 3000 ]; do\n\
                                    IFS= read -r clroom_owner < \"$1\" || exit 125\n\
                                    case \"$clroom_owner\" in\n\
                                      \"active:$$:$clroom_session:\"*)\n\
                                        [ -f \"$3\" ] || { /bin/sleep 0.01 || exit 125; clroom_tries=$((clroom_tries + 1)); continue; }\n\
                                        IFS= read -r clroom_release < \"$3\" || clroom_release=\n\
                                        clroom_owner_prefix=\"active:$$:$clroom_session:\"\n\
                                        clroom_start=${clroom_owner#\"$clroom_owner_prefix\"}\n\
                                        [ -n \"$clroom_start\" ] || exit 125\n\
                                        [ \"$clroom_release\" = \"released:$$:$clroom_session:$clroom_start\" ] || exit 125\n\
                                        case \"$clroom_release\" in\n\
                                          \"released:$$:$clroom_session:\"*) shift 3; exec \"$@\" ;;\n\
                                        esac\n\
                                        clroom_tries=$((clroom_tries + 1))\n\
                                        /bin/sleep 0.01 || exit 125\n\
                                        continue\n\
                                        ;;\n\
                                    esac\n\
                                    clroom_parent=${clroom_owner#creating:}\n\
                                    clroom_parent=${clroom_parent%%:*}\n\
                                    case \"$clroom_owner\" in\n\
                                      \"creating:$clroom_parent:$clroom_session:\"*) : ;;\n\
                                      *) exit 125 ;;\n\
                                    esac\n\
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

pub fn preflight_codex(
    executable: &Path,
    provider_args: &[String],
) -> Result<ProviderIdentity, String> {
    if !Path::new("/usr/bin/sandbox-exec").is_file() {
        return Err(
            "CLROOM_ISOLATION_UNAVAILABLE: macOS sandbox-exec is unavailable; continue locally"
                .to_owned(),
        );
    }
    let identity = resolve_launch_identity(executable, "codex", ">=0.147.0")?;
    if matches!(
        classify_codex_invocation(provider_args),
        CodexInvocation::Exec(_)
    ) {
        verify_codex_exec_clean_user_config(&identity, executable)?;
    }
    Ok(identity)
}

fn verify_codex_exec_clean_user_config(
    identity: &ProviderIdentity,
    executable: &Path,
) -> Result<(), String> {
    let root = std::env::temp_dir().join(format!(
        "clroom-codex-preflight-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| codex_exec_unsupported())?
            .as_nanos()
    ));
    let project = root.join("project");
    let home = root.join("home");
    let codex_home = home.join(".codex");
    let fixture_paths = [
        codex_home.join("skills/ambient/SKILL.md"),
        codex_home.join("plugins/cache/ambient/plugin.json"),
        codex_home.join("hooks/ambient-hook"),
        home.join(".agents/skills/ambient/SKILL.md"),
        home.join(".ssh/canary"),
        home.join(".aws/canary"),
        home.join(".config/gcloud/canary"),
        home.join(".azure/canary"),
    ];
    if fs::create_dir_all(&project).is_err()
        || fs::create_dir_all(&codex_home).is_err()
        || fixture_paths.iter().any(|path| {
            path.parent()
                .is_none_or(|parent| fs::create_dir_all(parent).is_err())
                || fs::write(path, b"synthetic CLROOM preflight canary\n").is_err()
        })
        || fs::write(codex_home.join("config.toml"), b"synthetic CLROOM config\n").is_err()
        || fs::write(
            codex_home.join("AGENTS.md"),
            b"synthetic CLROOM instruction\n",
        )
        .is_err()
        || fs::write(
            codex_home.join("AGENTS.override.md"),
            b"synthetic CLROOM override\n",
        )
        .is_err()
    {
        let _ = fs::remove_dir_all(&root);
        return Err(codex_exec_unsupported());
    }

    let plan = match plan_with_skills(
        &project,
        executable,
        &IsolationInputs {
            home: home.clone(),
            codex_home: codex_home.clone(),
        },
        &[],
    ) {
        Ok(plan) => plan,
        Err(_) => {
            let _ = fs::remove_dir_all(&root);
            return Err(codex_exec_unsupported());
        }
    };
    let sandbox = Path::new("/usr/bin/sandbox-exec");
    let status = Command::new(sandbox)
        .args(["-p", &plan.profile, "--"])
        .arg(&identity.real_executable)
        .args(["exec", "--ignore-user-config", "--help"])
        .env_clear()
        .env("HOME", &home)
        .env("CODEX_HOME", &codex_home)
        .env("PATH", "/usr/bin:/bin")
        .current_dir(&project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let result = match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) | Err(_) => Err(codex_exec_unsupported()),
    };
    let _ = fs::remove_dir_all(&root);
    result
}

pub fn preflight_claude(executable: &Path) -> Result<ProviderIdentity, String> {
    if !Path::new("/usr/bin/sandbox-exec").is_file() {
        return Err("CLROOM_CLAUDE_ISOLATION_UNAVAILABLE: macOS sandbox-exec is unavailable; continue locally".to_owned());
    }
    resolve_launch_identity(executable, "claude", ">=2.1.223")
}

fn resolve_executable(executable: &str, unavailable: fn() -> String) -> Result<PathBuf, String> {
    let Some(paths) = env::var_os("PATH") else {
        return Err(unavailable());
    };
    for directory in env::split_paths(&paths) {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(executable);
        if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
            if executable_is_current_clroom(&candidate) {
                return Err("CLROOM_PROVIDER_RECURSION_REFUSED: provider resolution returned to CLROOM; remove the CLROOM executable from the provider PATH".to_owned());
            }
            return Ok(candidate);
        }
    }
    Err(unavailable())
}

fn executable_is_current_clroom(candidate: &Path) -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    let Ok(candidate_real) = fs::canonicalize(candidate) else {
        return false;
    };
    let Ok(current_real) = fs::canonicalize(current) else {
        return false;
    };
    if candidate_real == current_real {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return fs::metadata(candidate_real)
            .ok()
            .zip(fs::metadata(current_real).ok())
            .is_some_and(|(left, right)| left.dev() == right.dev() && left.ino() == right.ino());
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn launch_isolated_codex(
    plan: &IsolationPlan,
    _executable: &Path,
    contract: &LaunchContract,
    identity: &ProviderIdentity,
    requested_names: &[String],
) -> Result<ExitCode, String> {
    let sandbox = Path::new("/usr/bin/sandbox-exec");
    if !fs::metadata(sandbox).is_ok_and(|metadata| metadata.is_file()) {
        return Err(
            "CLROOM_ISOLATION_UNAVAILABLE: macOS sandbox-exec is unavailable; continue locally"
                .to_owned(),
        );
    }
    let mut command = Command::new(sandbox);
    apply_parent_environment(&mut command, ProviderEnvironment::Codex, requested_names);
    command
        .arg("-p")
        .arg(&plan.profile)
        .arg("--")
        .arg(&identity.real_executable)
        .env(INTERNAL_PROVIDER_CHAIN_GUARD, "1")
        .args(&contract.argv);
    #[cfg(unix)]
    {
        revalidate_launch_identity(identity)?;
        Err(isolated_launch_error(command.exec()))
    }
    #[cfg(not(unix))]
    {
        revalidate_launch_identity(identity)?;
        let status = command.status().map_err(isolated_launch_error)?;
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

pub fn launch_claude(
    plan: &ClaudeIsolationPlan,
    projection: &mut Projection,
    _executable: &Path,
    contract: &LaunchContract,
    identity: &ProviderIdentity,
    requested_names: &[String],
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
        apply_parent_environment(&mut command, ProviderEnvironment::Claude, requested_names);
        command
            .arg("-c")
            .arg(CLAUDE_GATE_SCRIPT)
            .arg("clroom-claude-gate")
            .arg(projection.owner_marker_path())
            .arg(session_name)
            .arg(projection.release_marker_path())
            .arg(sandbox)
            .arg("-p")
            .arg(&plan.profile)
            .arg("--")
            .arg(&identity.real_executable)
            .env(INTERNAL_PROVIDER_CHAIN_GUARD, "1")
            .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
            .env("CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN", "1")
            .env("CLAUDE_CODE_SUBPROCESS_ENV_SCRUB", "1")
            .args(&contract.argv);
        revalidate_launch_identity(identity)?;
        let mut child = command.spawn().map_err(|_| claude_launch_error())?;

        if projection.activate_consumer(child.id()).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(claude_launch_error());
        }

        if revalidate_launch_identity(identity).is_err()
            || projection.release_consumer(child.id()).is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = projection.finish_after_consumer_exit();
            return Err(claude_launch_error());
        }

        let status = match child.wait() {
            Ok(status) => status,
            Err(_) => return Err(claude_launch_error()),
        };
        projection
            .finish_after_consumer_exit()
            .map_err(|_| claude_launch_error())?;
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }

    #[cfg(not(unix))]
    {
        let mut command = Command::new(sandbox);
        apply_parent_environment(&mut command, ProviderEnvironment::Claude, requested_names);
        let status = command
            .arg("-p")
            .arg(&plan.profile)
            .arg("--")
            .arg(&identity.real_executable)
            .env(INTERNAL_PROVIDER_CHAIN_GUARD, "1")
            .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
            .env("CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN", "1")
            .env("CLAUDE_CODE_SUBPROCESS_ENV_SCRUB", "1")
            .args(&contract.argv)
            .status()
            .map_err(|_| claude_launch_error())?;
        projection
            .finish_after_consumer_exit()
            .map_err(|_| claude_launch_error())?;
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

fn resolve_launch_identity(
    executable: &Path,
    provider: &str,
    version_range: &str,
) -> Result<ProviderIdentity, String> {
    let declaration = parse_declaration(&format!(
        "provider_id = \"{provider}\"\nexecutable = \"{provider}\"\nversion_range = \"{version_range}\"\ncontext_target = \"provider_native_context\"\ncollision_policy = \"deny\"\ncapability_evidence = \"unsupported_no_spend_only\"\nqualified = false\n"
    )).map_err(|_| "CLROOM_IDENTITY_INVALID: provider declaration is invalid; continue locally".to_owned())?;
    resolve_identity(
        ProviderNativePreauthenticatedSession::Available,
        &declaration,
        executable,
    )
    .map_err(|error| format!("CLROOM_IDENTITY_INVALID: {error}; continue locally"))
}

fn revalidate_launch_identity(identity: &ProviderIdentity) -> Result<(), String> {
    revalidate_identity(identity)
        .map_err(|error| format!("CLROOM_IDENTITY_INVALID: {error}; continue locally"))
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

fn codex_exec_unsupported() -> String {
    "CLROOM_CODEX_EXEC_UNSUPPORTED: installed Codex does not expose a qualified 'exec --ignore-user-config' path; continue locally".to_owned()
}

fn local_claude_unavailable() -> String {
    "LOCAL_CLAUDE_UNAVAILABLE: executable 'claude' not found; continue locally".to_owned()
}

#[cfg(all(test, unix))]
mod tests {
    use super::CLAUDE_GATE_SCRIPT;
    use std::{
        fs,
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn claude_gate_rejects_release_with_different_process_start_identity() {
        // Break caught: independent PID/session prefix checks accept a release
        // marker that is not bound to the active gate process identity.
        let root =
            std::env::temp_dir().join(format!("clroom-claude-gate-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let owner = root.join("owner");
        let release = root.join("release");
        let capture = root.join("executed");
        let session = format!("session-test-{}", std::process::id());
        fs::write(
            &owner,
            format!("creating:{}:{session}:launcher-start\n", std::process::id()),
        )
        .unwrap();

        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(CLAUDE_GATE_SCRIPT)
            .arg("clroom-claude-gate-test")
            .arg(&owner)
            .arg(&session)
            .arg(&release)
            .arg("/usr/bin/touch")
            .arg(&capture)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        fs::write(&owner, format!("active:{pid}:{session}:start-a\n")).unwrap();
        fs::write(&release, format!("released:{pid}:{session}:start-b\n")).unwrap();

        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline && !capture.exists() {
            thread::sleep(Duration::from_millis(10));
        }
        if child.try_wait().unwrap().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
        let executed = capture.exists();
        let _ = fs::remove_dir_all(&root);
        assert!(
            !executed,
            "mismatched process-start identity released the provider gate"
        );
    }
}
