use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationPlan {
    pub profile: String,
    pub project: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationInputs {
    pub home: PathBuf,
    pub codex_home: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationError {
    UnsupportedPlatform,
    InvalidProject,
    InvalidExecutable,
    InvalidHome,
    InvalidCodexHome,
}

pub fn plan(
    project: &Path,
    executable: &Path,
    inputs: &IsolationInputs,
) -> Result<IsolationPlan, IsolationError> {
    if env::consts::OS != "macos" || env::consts::ARCH != "aarch64" {
        return Err(IsolationError::UnsupportedPlatform);
    }

    let project = canonical_directory(project).ok_or(IsolationError::InvalidProject)?;
    validate_executable(executable)?;

    let home = safe_ambient_root(&inputs.home).ok_or(IsolationError::InvalidHome)?;
    let codex_home =
        safe_ambient_root(&inputs.codex_home).ok_or(IsolationError::InvalidCodexHome)?;

    let denied_files = [
        codex_home.join("AGENTS.md"),
        codex_home.join("AGENTS.override.md"),
    ];
    let denied_subpaths = [
        codex_home.join("skills"),
        home.join(".agents/skills"),
        PathBuf::from("/private/etc/codex/skills"),
    ];

    let mut profile = String::from("(version 1)\n(allow default)\n");
    profile.push_str("(deny file-read*");
    for path in denied_files {
        profile.push_str("\n  (literal \"");
        profile.push_str(&escape_scheme_path(&path)?);
        profile.push_str("\")");
    }
    for path in denied_subpaths {
        profile.push_str("\n  (subpath \"");
        profile.push_str(&escape_scheme_path(&path)?);
        profile.push_str("\")");
    }
    profile.push_str(")\n");

    Ok(IsolationPlan { profile, project })
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(path).ok()
}

fn safe_ambient_root(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || path.as_os_str().is_empty() || path.to_str().is_none() {
        return None;
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    let path = path.to_str()?;
    let normalized = if let Some(suffix) = path.strip_prefix("/var/") {
        format!("/private/var/{suffix}")
    } else if let Some(suffix) = path.strip_prefix("/tmp/") {
        format!("/private/tmp/{suffix}")
    } else if path == "/var" {
        "/private/var".to_owned()
    } else if path == "/tmp" {
        "/private/tmp".to_owned()
    } else {
        path.to_owned()
    };
    Some(PathBuf::from(normalized))
}

fn validate_executable(executable: &Path) -> Result<(), IsolationError> {
    if !executable.is_absolute() {
        return Err(IsolationError::InvalidExecutable);
    }
    let metadata = fs::metadata(executable).map_err(|_| IsolationError::InvalidExecutable)?;
    if !metadata.is_file() {
        return Err(IsolationError::InvalidExecutable);
    }
    Ok(())
}

fn escape_scheme_path(path: &Path) -> Result<String, IsolationError> {
    let value = path.to_str().ok_or(IsolationError::InvalidHome)?;
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}
