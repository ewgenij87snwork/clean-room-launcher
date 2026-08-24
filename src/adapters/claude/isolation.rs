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
pub enum IsolationError {
    UnsupportedPlatform,
    InvalidProject,
    InvalidExecutable,
    InvalidHome,
    InvalidAllowedPath,
}

pub fn plan(
    project: &Path,
    executable: &Path,
    home: &Path,
    projection_root: &Path,
    projection_view: &Path,
    allowed_source_paths: &[PathBuf],
) -> Result<IsolationPlan, IsolationError> {
    if env::consts::OS != "macos" || env::consts::ARCH != "aarch64" {
        return Err(IsolationError::UnsupportedPlatform);
    }

    let project = canonical_directory(project).ok_or(IsolationError::InvalidProject)?;
    validate_executable(executable)?;
    let home = safe_root(home).ok_or(IsolationError::InvalidHome)?;
    let projection_root = safe_allowed_directory(projection_root, &project)
        .ok_or(IsolationError::InvalidAllowedPath)?;
    let projection_view = safe_allowed_directory(projection_view, &project)
        .ok_or(IsolationError::InvalidAllowedPath)?;
    if !projection_view.starts_with(&projection_root) {
        return Err(IsolationError::InvalidAllowedPath);
    }
    let allowed_source_paths = allowed_source_paths
        .iter()
        .map(|path| safe_allowed_path(path, &project))
        .collect::<Result<Vec<_>, _>>()?;

    let denied_read_roots = vec![
        home.join(".claude"),
        home.join(".agents/skills"),
        home.join(".codex/skills"),
        home.join(".codex/plugins/cache"),
        projection_root.clone(),
        home.join(".ssh"),
        home.join(".aws"),
        home.join(".config/gcloud"),
        home.join(".azure"),
    ];
    let mut denied_write_roots = vec![
        home.join(".claude/skills"),
        home.join(".agents/skills"),
        home.join(".codex/skills"),
        home.join(".codex/plugins/cache"),
        home.join(".claude/plugins/cache"),
        projection_root,
        home.join(".ssh"),
        home.join(".aws"),
        home.join(".config/gcloud"),
        home.join(".azure"),
    ];
    let denied_write_files = [
        home.join(".claude/CLAUDE.md"),
        home.join(".claude/settings.json"),
        home.join(".claude/settings.local.json"),
    ];
    denied_write_roots.extend(allowed_source_paths.iter().cloned());

    let mut profile = String::from("(version 1)\n(allow default)\n");
    profile.push_str("(deny file-read*");
    for path in &denied_read_roots {
        push_subpath(&mut profile, path)?;
    }
    profile.push_str(")\n");
    profile.push_str("(deny file-write*");
    for path in &denied_write_files {
        push_literal(&mut profile, path)?;
    }
    for path in &denied_write_roots {
        push_literal(&mut profile, path)?;
        push_subpath(&mut profile, path)?;
    }
    profile.push_str(")\n");

    // Metadata/listing of denied roots is intentionally not reopened.  The
    // projected view and explicit source files are the only readable seams.
    profile.push_str("(allow file-read-metadata");
    push_literal(&mut profile, &projection_view)?;
    push_subpath(&mut profile, &projection_view)?;
    profile.push_str(")\n");
    profile.push_str("(allow file-read*");
    push_literal(&mut profile, &projection_view)?;
    push_subpath(&mut profile, &projection_view)?;
    if !allowed_source_paths.is_empty() {
        for path in &allowed_source_paths {
            push_literal(&mut profile, path)?;
            push_subpath(&mut profile, path)?;
        }
    }
    profile.push_str(")\n");

    Ok(IsolationPlan { profile, project })
}

fn safe_allowed_directory(path: &Path, project: &Path) -> Option<PathBuf> {
    let canonical = canonical_directory(path)?;
    (!canonical.starts_with(project)).then(|| normalize_private_path(&canonical))
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

fn validate_executable(executable: &Path) -> Result<(), IsolationError> {
    if !executable.is_absolute()
        || !fs::metadata(executable).is_ok_and(|metadata| metadata.is_file())
    {
        return Err(IsolationError::InvalidExecutable);
    }
    Ok(())
}

fn safe_root(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || path.as_os_str().is_empty() || path.to_str().is_none() {
        return None;
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(path)
        .ok()
        .map(|canonical| normalize_private_path(&canonical))
}

fn safe_allowed_path(path: &Path, project: &Path) -> Result<PathBuf, IsolationError> {
    if !path.is_absolute() || path.starts_with(project) {
        return Err(IsolationError::InvalidAllowedPath);
    }
    Ok(normalize_private_path(path))
}

fn normalize_private_path(path: &Path) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(suffix) = value.strip_prefix("/var/") {
        PathBuf::from(format!("/private/var/{suffix}"))
    } else if let Some(suffix) = value.strip_prefix("/tmp/") {
        PathBuf::from(format!("/private/tmp/{suffix}"))
    } else {
        path.to_path_buf()
    }
}

fn push_literal(profile: &mut String, path: &Path) -> Result<(), IsolationError> {
    profile.push_str("\n  (literal \"");
    profile.push_str(&escape_path(path)?);
    profile.push_str("\")");
    Ok(())
}

fn push_subpath(profile: &mut String, path: &Path) -> Result<(), IsolationError> {
    profile.push_str("\n  (subpath \"");
    profile.push_str(&escape_path(path)?);
    profile.push_str("\")");
    Ok(())
}

fn escape_path(path: &Path) -> Result<String, IsolationError> {
    let value = path.to_str().ok_or(IsolationError::InvalidAllowedPath)?;
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}
