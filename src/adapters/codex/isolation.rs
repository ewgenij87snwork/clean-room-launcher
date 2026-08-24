use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationPlan {
    pub profile: String,
    pub project: PathBuf,
    pub selected_global_skills: usize,
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
    InvalidSkillSelector(String),
    UnknownSkillSelector(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobalSkill {
    name: String,
    namespace: Option<String>,
    source_precedence: usize,
    entry_path: PathBuf,
    canonical_path: PathBuf,
}

#[derive(Debug, Default)]
struct SkillSelection {
    canonical_paths: BTreeSet<PathBuf>,
    allowed_paths: BTreeSet<PathBuf>,
    logical_skills: BTreeSet<(Option<String>, String)>,
}

pub fn plan(
    project: &Path,
    executable: &Path,
    inputs: &IsolationInputs,
) -> Result<IsolationPlan, IsolationError> {
    plan_with_skills(project, executable, inputs, &[])
}

pub fn plan_with_skills(
    project: &Path,
    executable: &Path,
    inputs: &IsolationInputs,
    selectors: &[String],
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
        codex_home.join("config.toml"),
    ];
    let denied_roots = [
        codex_home.join("skills"),
        codex_home.join("plugins"),
        codex_home.join("hooks"),
        home.join(".agents/skills"),
        PathBuf::from("/private/etc/codex/skills"),
    ];
    let credential_roots = [
        home.join(".ssh"),
        home.join(".aws"),
        home.join(".config/gcloud"),
        home.join(".azure"),
    ];
    let inventory = if selectors.is_empty() {
        Vec::new()
    } else {
        discover_global_skills(&denied_roots)
    };
    let selection = resolve_skill_selectors(selectors, &inventory)?;
    let mut denied_subpaths = denied_roots.iter().cloned().collect::<BTreeSet<_>>();
    denied_subpaths.extend(
        inventory
            .iter()
            .map(|skill| skill.canonical_path.clone())
            .filter(|path| !path.starts_with(&project)),
    );

    let mut profile = String::from("(version 1)\n(allow default)\n");
    profile.push_str("(deny file-read*");
    for path in &denied_files {
        profile.push_str("\n  (literal \"");
        profile.push_str(&escape_scheme_path(path)?);
        profile.push_str("\")");
    }
    for path in denied_subpaths {
        profile.push_str("\n  (subpath \"");
        profile.push_str(&escape_scheme_path(&path)?);
        profile.push_str("\")");
    }
    for path in &credential_roots {
        profile.push_str("\n  (subpath \"");
        profile.push_str(&escape_scheme_path(path)?);
        profile.push_str("\")");
    }
    profile.push_str(")\n");
    profile.push_str("(deny file-write*");
    for path in &denied_files {
        profile.push_str("\n  (literal \"");
        profile.push_str(&escape_scheme_path(path)?);
        profile.push_str("\")");
    }
    for path in denied_roots.iter().chain(credential_roots.iter()) {
        profile.push_str("\n  (subpath \"");
        profile.push_str(&escape_scheme_path(path)?);
        profile.push_str("\")");
    }
    profile.push_str(")\n");
    if !selection.allowed_paths.is_empty() {
        profile.push_str("(allow file-read-metadata");
        for root in &denied_roots {
            profile.push_str("\n  (subpath \"");
            profile.push_str(&escape_scheme_path(root)?);
            profile.push_str("\")");
        }
        profile.push_str(")\n");
        profile.push_str("(allow file-read*");
        for root in &denied_roots {
            profile.push_str("\n  (literal \"");
            profile.push_str(&escape_scheme_path(root)?);
            profile.push_str("\")");
        }
        for path in selection.allowed_paths {
            profile.push_str("\n  (literal \"");
            profile.push_str(&escape_scheme_path(&path)?);
            profile.push_str("\")");
            profile.push_str("\n  (subpath \"");
            profile.push_str(&escape_scheme_path(&path)?);
            profile.push_str("\")");
        }
        profile.push_str(")\n");
    }

    Ok(IsolationPlan {
        profile,
        project,
        selected_global_skills: selection.logical_skills.len(),
    })
}

fn discover_global_skills(roots: &[PathBuf]) -> Vec<GlobalSkill> {
    let mut inventory = Vec::new();
    for (source_precedence, root) in roots.iter().enumerate() {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let entry_path = entry.path();
            if !fs::metadata(entry_path.join("SKILL.md")).is_ok_and(|metadata| metadata.is_file()) {
                continue;
            }
            let Ok(canonical_path) = fs::canonicalize(&entry_path) else {
                continue;
            };
            if !fs::metadata(&canonical_path).is_ok_and(|metadata| metadata.is_dir()) {
                continue;
            }
            inventory.push(GlobalSkill {
                namespace: plugin_namespace(&canonical_path),
                name,
                source_precedence,
                entry_path,
                canonical_path,
            });
        }
    }
    inventory.sort_by(|left, right| {
        (
            &left.name,
            &left.namespace,
            left.source_precedence,
            &left.canonical_path,
        )
            .cmp(&(
                &right.name,
                &right.namespace,
                right.source_precedence,
                &right.canonical_path,
            ))
    });
    inventory
}

fn plugin_namespace(skill_path: &Path) -> Option<String> {
    let skills_root = skill_path
        .ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "skills"))?;
    let package_root = skills_root.parent()?;
    for manifest in [
        package_root.join(".codex-plugin/plugin.json"),
        package_root.join(".claude-plugin/plugin.json"),
    ] {
        let Ok(metadata) = fs::metadata(&manifest) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > 64 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(&manifest) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        if let Some(name) = value.get("name").and_then(serde_json::Value::as_str)
            && valid_skill_name(name)
        {
            return Some(name.to_owned());
        }
    }
    None
}

fn resolve_skill_selectors(
    selectors: &[String],
    inventory: &[GlobalSkill],
) -> Result<SkillSelection, IsolationError> {
    let mut selection = SkillSelection::default();
    let mut winners = BTreeMap::new();
    for selector in selectors {
        validate_selector(selector)?;

        let matched = if let Some((namespace, name)) = selector.split_once(':') {
            inventory
                .iter()
                .filter(|skill| skill.namespace.as_deref() == Some(namespace) && skill.name == name)
                .collect::<Vec<_>>()
        } else {
            inventory
                .iter()
                .filter(|skill| {
                    skill.name == *selector || skill.namespace.as_deref() == Some(selector.as_str())
                })
                .collect::<Vec<_>>()
        };

        if matched.is_empty() {
            return Err(IsolationError::UnknownSkillSelector(selector.clone()));
        }
        for skill in matched {
            winners
                .entry((skill.namespace.clone(), skill.name.clone()))
                .or_insert(skill);
        }
    }

    for (logical_skill, skill) in winners {
        selection.logical_skills.insert(logical_skill);
        selection
            .canonical_paths
            .insert(skill.canonical_path.clone());
        selection.allowed_paths.insert(skill.entry_path.clone());
        selection.allowed_paths.insert(skill.canonical_path.clone());
    }
    Ok(selection)
}

fn validate_selector(selector: &str) -> Result<(), IsolationError> {
    let mut parts = selector.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if !valid_skill_name(first)
        || second.is_some_and(|part| !valid_skill_name(part))
        || parts.next().is_some()
    {
        return Err(IsolationError::InvalidSkillSelector(selector.to_owned()));
    }
    Ok(())
}

fn valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
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
