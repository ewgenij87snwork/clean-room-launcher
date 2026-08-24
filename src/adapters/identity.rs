use super::session::{
    PreauthenticatedSessionError, ProviderNativePreauthenticatedSession,
    require_preauthenticated_session,
};
use crate::{contracts::adapter::AdapterDeclaration, core::inventory::sha256_hex};
use std::{
    fmt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ProviderIdentity {
    pub provider_id: String,
    pub real_executable: PathBuf,
    pub artifact_digest: String,
    pub version: (u64, u64, u64),
    pub os: String,
    pub arch: String,
    pub interpreter: Option<(PathBuf, String)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    Unavailable,
    Replaced,
    CommandMismatch,
    Version,
    VersionOutOfRange,
    UnsupportedInterpreter,
    PolicyIdentityMismatch,
    InvalidPolicy,
    RequiredAuthMissing,
    ProviderNativePreauthenticatedSessionUnavailable,
    ProviderNativePreauthenticatedSessionAmbiguous,
}
impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "EXECUTABLE_UNAVAILABLE",
            Self::Replaced => "EXECUTABLE_REPLACED",
            Self::CommandMismatch => "DECLARATION_COMMAND_MISMATCH",
            Self::Version => "UNKNOWN_VERSION",
            Self::VersionOutOfRange => "VERSION_OUT_OF_RANGE",
            Self::UnsupportedInterpreter => "UNSUPPORTED_INTERPRETER",
            Self::PolicyIdentityMismatch => "POLICY_IDENTITY_MISMATCH",
            Self::InvalidPolicy => "INVALID_ENVIRONMENT_POLICY",
            Self::RequiredAuthMissing => "REQUIRED_AUTH_MISSING",
            Self::ProviderNativePreauthenticatedSessionUnavailable => {
                "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE"
            }
            Self::ProviderNativePreauthenticatedSessionAmbiguous => {
                "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS"
            }
        })
    }
}
impl std::error::Error for AdapterError {}

pub fn resolve_identity(
    session: ProviderNativePreauthenticatedSession,
    declaration: &AdapterDeclaration,
    command: &Path,
) -> Result<ProviderIdentity, AdapterError> {
    require_preauthenticated_session(session).map_err(|error| match error {
        PreauthenticatedSessionError::Unavailable => {
            AdapterError::ProviderNativePreauthenticatedSessionUnavailable
        }
        PreauthenticatedSessionError::Ambiguous => {
            AdapterError::ProviderNativePreauthenticatedSessionAmbiguous
        }
    })?;
    if !command.is_absolute() {
        return Err(AdapterError::Unavailable);
    }
    if command.file_name().and_then(|name| name.to_str()) != Some(&declaration.executable) {
        return Err(AdapterError::CommandMismatch);
    }
    let real_executable = command
        .canonicalize()
        .map_err(|_| AdapterError::Unavailable)?;
    let before = std::fs::read(&real_executable).map_err(|_| AdapterError::Unavailable)?;
    let interpreter = interpreter(&before)?;
    let output = discover_version_in_closed_sandbox(&real_executable)?;
    if !output.status.success() {
        return Err(AdapterError::Version);
    }
    if std::fs::read(&real_executable).map_err(|_| AdapterError::Unavailable)? != before {
        return Err(AdapterError::Replaced);
    }
    let version = std::str::from_utf8(&output.stdout)
        .ok()
        .and_then(|text| text.split_whitespace().find_map(parse_version))
        .ok_or(AdapterError::Version)?;
    let minimum = declaration
        .version_range
        .strip_prefix(">=")
        .and_then(parse_version)
        .ok_or(AdapterError::VersionOutOfRange)?;
    if version < minimum {
        return Err(AdapterError::VersionOutOfRange);
    }
    Ok(ProviderIdentity {
        provider_id: declaration.provider_id.clone(),
        real_executable,
        artifact_digest: sha256_hex(&before),
        version,
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        interpreter,
    })
}

fn discover_version_in_closed_sandbox(
    executable: &Path,
) -> Result<std::process::Output, AdapterError> {
    let sandbox = Path::new("/usr/bin/sandbox-exec");
    if !sandbox.is_file() {
        return Err(AdapterError::Unavailable);
    }
    // Keep macOS runtime/loader services available, but close the version-only
    // probe to network, writes, provider state, common auth roots, and user
    // document surfaces. The executable is re-read and hashed after the probe.
    let mut profile =
        String::from("(version 1)\n(allow default)\n(deny network*)\n(deny file-write*)\n");
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for denied in [
            home.join(".ssh"),
            home.join(".aws"),
            home.join(".config/gcloud"),
            home.join(".azure"),
            home.join(".claude"),
            home.join(".agents"),
            home.join(".codex/auth.json"),
            home.join(".codex/skills"),
            home.join(".codex/plugins"),
            home.join("Desktop"),
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Library"),
        ] {
            profile.push_str("(deny file-read* (literal \"");
            profile.push_str(&escape_scheme_path(&denied)?);
            profile.push_str("\") (subpath \"");
            profile.push_str(&escape_scheme_path(&denied)?);
            profile.push_str("\"))\n");
        }
    }
    Command::new(sandbox)
        .args(["-p", &profile, "--"])
        .arg(executable)
        .arg("--version")
        .env_clear()
        .current_dir("/")
        .output()
        .map_err(|_| AdapterError::Unavailable)
}

fn escape_scheme_path(path: &Path) -> Result<String, AdapterError> {
    let value = path.to_str().ok_or(AdapterError::Unavailable)?;
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}
pub fn revalidate_identity(identity: &ProviderIdentity) -> Result<(), AdapterError> {
    if sha256_hex(&std::fs::read(&identity.real_executable).map_err(|_| AdapterError::Unavailable)?)
        != identity.artifact_digest
    {
        return Err(AdapterError::Replaced);
    }
    if let Some((path, digest)) = &identity.interpreter
        && sha256_hex(&std::fs::read(path).map_err(|_| AdapterError::Unavailable)?) != *digest
    {
        return Err(AdapterError::Replaced);
    }
    Ok(())
}
fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut p = value.split('.');
    match (
        p.next()?.parse().ok(),
        p.next()?.parse().ok(),
        p.next()?.parse().ok(),
        p.next(),
    ) {
        (Some(a), Some(b), Some(c), None) => Some((a, b, c)),
        _ => None,
    }
}
fn interpreter(bytes: &[u8]) -> Result<Option<(PathBuf, String)>, AdapterError> {
    let Some(line) = bytes
        .split(|b| *b == b'\n')
        .next()
        .filter(|line| line.starts_with(b"#!"))
    else {
        return Ok(None);
    };
    let value = std::str::from_utf8(&line[2..])
        .map_err(|_| AdapterError::UnsupportedInterpreter)?
        .trim();
    if value.contains(char::is_whitespace) {
        return Err(AdapterError::UnsupportedInterpreter);
    }
    let path = PathBuf::from(value)
        .canonicalize()
        .map_err(|_| AdapterError::UnsupportedInterpreter)?;
    let digest =
        sha256_hex(&std::fs::read(&path).map_err(|_| AdapterError::UnsupportedInterpreter)?);
    Ok(Some((path, digest)))
}
