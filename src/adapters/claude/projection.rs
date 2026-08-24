use std::{
    collections::BTreeMap,
    fs::{self, DirBuilder, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::{
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, symlink},
    process::{Command, Stdio},
};

static PROJECTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const APP_TEMP_DIR: &str = "clroom";
const STORAGE_DIR: &str = "claude-projections-v2";
const ACTIVE_DIR: &str = "active";
const QUARANTINE_DIR: &str = "quarantine";
const SESSION_PREFIX: &str = "session-";
const OWNER_MARKER: &str = ".clroom-projection-owner-v2";
const OWNER_MARKER_TEMP: &str = ".clroom-projection-owner-v2.tmp";
const RELEASE_MARKER: &str = ".clroom-projection-release-v2";
const RELEASE_MARKER_TEMP: &str = ".clroom-projection-release-v2.tmp";
const RANDOM_SOURCE: &str = "/dev/urandom";
const MARKER_LIMIT: u64 = 128;
const RANDOM_SUFFIX_LEN: usize = 32;
const MAX_PROCESS_ID: u32 = i32::MAX as u32;

#[derive(Debug)]
pub struct Projection {
    root: PathBuf,
    storage_root: PathBuf,
    phase: ProjectionPhase,
    pub add_dir: PathBuf,
    pub selected_global_skills: usize,
    allowed_source_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProjectionPhase {
    Creating,
    Active,
    Finished,
}

impl Drop for Projection {
    fn drop(&mut self) {
        if self.phase == ProjectionPhase::Creating {
            self.phase = ProjectionPhase::Finished;
            let _ = quarantine_and_delete(&self.root, &self.storage_root.join(QUARANTINE_DIR));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidSelector(String),
    UnknownSelector(String),
    NameCollision(String),
    Unavailable,
}

#[derive(Debug)]
struct GlobalSkill {
    name: String,
    namespace: Option<String>,
    source_precedence: usize,
    canonical_path: PathBuf,
}

pub fn project(home: &Path, selectors: &[String]) -> Result<Projection, ProjectionError> {
    if !home.is_absolute() {
        return Err(ProjectionError::Unavailable);
    }
    let roots = [home.join(".claude/skills"), home.join(".agents/skills")];
    let approved_target_roots = approved_target_roots(home, &roots);
    let inventory = discover_global_skills(&roots, &approved_target_roots);
    let selected = resolve_skill_selectors(selectors, &inventory)?;
    reject_native_name_collisions(&selected)?;
    let (storage_root, root) = create_root()?;
    let add_dir = root.join("view");
    let allowed_source_paths = selected
        .iter()
        .map(|skill| skill.canonical_path.clone())
        .collect();
    let projection = Projection {
        root,
        storage_root,
        phase: ProjectionPhase::Creating,
        add_dir,
        selected_global_skills: selected.len(),
        allowed_source_paths,
    };
    let skills_dir = projection.add_dir.join(".claude/skills");
    fs::create_dir_all(&skills_dir).map_err(|_| ProjectionError::Unavailable)?;

    for skill in &selected {
        link_skill(&skill.canonical_path, &skills_dir.join(&skill.name))?;
    }

    Ok(projection)
}

impl Projection {
    pub fn allowed_source_paths(&self) -> &[PathBuf] {
        &self.allowed_source_paths
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    pub fn owner_marker_path(&self) -> PathBuf {
        self.root.join(OWNER_MARKER)
    }

    pub fn release_marker_path(&self) -> PathBuf {
        self.root.join(RELEASE_MARKER)
    }

    pub fn session_name(&self) -> Option<&str> {
        self.root.file_name()?.to_str()
    }

    pub fn activate_consumer(&mut self, consumer_pid: u32) -> Result<(), ProjectionError> {
        if consumer_pid == 0 {
            return Err(ProjectionError::Unavailable);
        }
        let start = process_start_identity(consumer_pid).ok_or(ProjectionError::Unavailable)?;
        for approved_source in &self.allowed_source_paths {
            let current_source =
                fs::canonicalize(approved_source).map_err(|_| ProjectionError::Unavailable)?;
            if current_source != *approved_source {
                return Err(ProjectionError::Unavailable);
            }
            let name = approved_source
                .file_name()
                .ok_or(ProjectionError::Unavailable)?;
            let link = self.add_dir.join(".claude/skills").join(name);
            let link_target = fs::read_link(&link).map_err(|_| ProjectionError::Unavailable)?;
            let link_target = if link_target.is_absolute() {
                link_target
            } else {
                link.parent()
                    .ok_or(ProjectionError::Unavailable)?
                    .join(link_target)
            };
            if fs::canonicalize(link_target).map_err(|_| ProjectionError::Unavailable)?
                != *approved_source
            {
                return Err(ProjectionError::Unavailable);
            }
        }
        write_owner_marker(&self.root, OwnerState::Active(consumer_pid, start))?;
        self.phase = ProjectionPhase::Active;
        Ok(())
    }

    pub fn release_consumer(&mut self, consumer_pid: u32) -> Result<(), ProjectionError> {
        if self.phase != ProjectionPhase::Active || consumer_pid == 0 {
            return Err(ProjectionError::Unavailable);
        }
        let start = process_start_identity(consumer_pid).ok_or(ProjectionError::Unavailable)?;
        if marked_projection_owner(&self.root)
            != Some(OwnerState::Active(consumer_pid, start.clone()))
        {
            return Err(ProjectionError::Unavailable);
        }
        let session = self.session_name().ok_or(ProjectionError::Unavailable)?;
        write_release_marker(
            &self.root,
            format!("released:{consumer_pid}:{session}:{start}\n"),
        )
    }

    pub fn finish_after_consumer_exit(&mut self) -> Result<(), ProjectionError> {
        if self.phase != ProjectionPhase::Finished {
            self.phase = ProjectionPhase::Finished;
            quarantine_and_delete(&self.root, &self.storage_root.join(QUARANTINE_DIR))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnerState {
    Creating(u32, String),
    Active(u32, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessState {
    Live,
    Dead,
    Unknown,
}

fn create_root() -> Result<(PathBuf, PathBuf), ProjectionError> {
    let temp_root = std::env::temp_dir();
    validate_private_directory(&temp_root)?;
    let app_root = temp_root.join(APP_TEMP_DIR);
    ensure_private_directory(&app_root)?;
    let storage_root = app_root.join(STORAGE_DIR);
    ensure_private_directory(&storage_root)?;
    let active_root = storage_root.join(ACTIVE_DIR);
    let quarantine_root = storage_root.join(QUARANTINE_DIR);
    ensure_private_directory(&active_root)?;
    ensure_private_directory(&quarantine_root)?;

    reap_quarantine(&quarantine_root)?;
    reap_stale_active(&active_root, &quarantine_root)?;

    for _ in 0..16 {
        let sequence = PROJECTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix = random_suffix()?;
        let candidate = active_root.join(format!(
            "{SESSION_PREFIX}{}-{sequence}-{suffix}",
            std::process::id()
        ));
        match create_private_directory(&candidate) {
            Ok(()) => {
                let Some(start) = process_start_identity(std::process::id()) else {
                    let _ = fs::remove_dir_all(&candidate);
                    return Err(ProjectionError::Unavailable);
                };
                if write_owner_marker(&candidate, OwnerState::Creating(std::process::id(), start))
                    .is_err()
                {
                    let _ = fs::remove_dir_all(&candidate);
                    return Err(ProjectionError::Unavailable);
                }
                return Ok((storage_root, candidate));
            }
            Err(ProjectionError::Unavailable) if candidate.exists() => continue,
            Err(error) => return Err(error),
        }
    }
    Err(ProjectionError::Unavailable)
}

#[cfg(unix)]
fn ensure_private_directory(path: &Path) -> Result<(), ProjectionError> {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(ProjectionError::Unavailable),
    }
    validate_private_directory(path)
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> Result<(), ProjectionError> {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|_| ProjectionError::Unavailable)?;
    validate_private_directory(path)
}

#[cfg(unix)]
fn validate_private_directory(path: &Path) -> Result<(), ProjectionError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ProjectionError::Unavailable)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || metadata.mode() & 0o077 != 0 {
        return Err(ProjectionError::Unavailable);
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_directory(_: &Path) -> Result<(), ProjectionError> {
    Err(ProjectionError::Unavailable)
}

#[cfg(not(unix))]
fn create_private_directory(_: &Path) -> Result<(), ProjectionError> {
    Err(ProjectionError::Unavailable)
}

fn random_suffix() -> Result<String, ProjectionError> {
    let mut bytes = [0_u8; 16];
    fs::File::open(RANDOM_SOURCE)
        .and_then(|mut source| source.read_exact(&mut bytes))
        .map_err(|_| ProjectionError::Unavailable)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut suffix = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        suffix.push(HEX[(byte >> 4) as usize] as char);
        suffix.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(suffix)
}

#[cfg(unix)]
fn write_owner_marker(path: &Path, state: OwnerState) -> Result<(), ProjectionError> {
    let marker = path.join(OWNER_MARKER);
    let temporary = path.join(OWNER_MARKER_TEMP);
    let session_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ProjectionError::Unavailable)?;
    let _ = fs::remove_file(&temporary);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|_| ProjectionError::Unavailable)?;
    let value = match state {
        OwnerState::Creating(pid, start) => format!("creating:{pid}:{session_name}:{start}\n"),
        OwnerState::Active(pid, start) => format!("active:{pid}:{session_name}:{start}\n"),
    };
    if file.write_all(value.as_bytes()).is_err() || file.flush().is_err() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(ProjectionError::Unavailable);
    }
    drop(file);
    if fs::rename(&temporary, marker).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ProjectionError::Unavailable);
    }
    Ok(())
}

fn write_release_marker(path: &Path, value: String) -> Result<(), ProjectionError> {
    let marker = path.join(RELEASE_MARKER);
    let temporary = path.join(RELEASE_MARKER_TEMP);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|_| ProjectionError::Unavailable)?;
    if file.write_all(value.as_bytes()).is_err() || file.flush().is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ProjectionError::Unavailable);
    }
    drop(file);
    fs::rename(&temporary, &marker).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        ProjectionError::Unavailable
    })
}

#[cfg(not(unix))]
fn write_owner_marker(_: &Path, _: OwnerState) -> Result<(), ProjectionError> {
    Err(ProjectionError::Unavailable)
}

fn reap_quarantine(quarantine_root: &Path) -> Result<(), ProjectionError> {
    let Ok(entries) = fs::read_dir(quarantine_root) else {
        return Err(ProjectionError::Unavailable);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(owner) = marked_projection_owner(&path) else {
            continue;
        };
        if owner_process_state(&owner) == ProcessState::Dead {
            match fs::remove_dir_all(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(ProjectionError::Unavailable),
            }
        }
    }
    Ok(())
}

fn reap_stale_active(active_root: &Path, quarantine_root: &Path) -> Result<(), ProjectionError> {
    let Ok(entries) = fs::read_dir(active_root) else {
        return Err(ProjectionError::Unavailable);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(owner) = marked_projection_owner(&path) else {
            continue;
        };
        if owner_process_state(&owner) == ProcessState::Dead {
            quarantine_and_delete(&path, quarantine_root)?;
        }
    }
    Ok(())
}

fn marked_projection_owner(path: &Path) -> Option<OwnerState> {
    if !valid_session_directory(path) {
        return None;
    }
    let marker = path.join(OWNER_MARKER);
    let marker_metadata = fs::symlink_metadata(&marker).ok()?;
    if marker_metadata.file_type().is_symlink()
        || !marker_metadata.is_file()
        || marker_metadata.len() > MARKER_LIMIT
        || !private_marker(&marker_metadata)
    {
        return None;
    }
    let recorded = fs::read_to_string(marker).ok()?;
    let value = recorded.strip_suffix('\n')?;
    if value.contains('\n') {
        return None;
    }
    let mut fields = value.splitn(4, ':');
    let state = fields.next()?;
    let pid = parse_process_id(fields.next()?)?;
    let recorded_session = fields.next()?;
    let recorded_start = fields.next()?;
    if path.file_name()?.to_str()? != recorded_session {
        return None;
    }
    let launcher_pid = session_launcher_pid(path)?;
    match state {
        "creating" if pid == launcher_pid => {
            Some(OwnerState::Creating(pid, recorded_start.to_owned()))
        }
        "active" => Some(OwnerState::Active(pid, recorded_start.to_owned())),
        _ => None,
    }
}

fn owner_process_state(owner: &OwnerState) -> ProcessState {
    let (pid, recorded_start) = match owner {
        OwnerState::Creating(pid, start) | OwnerState::Active(pid, start) => (*pid, start),
    };
    match process_state(pid) {
        ProcessState::Live => {
            if process_start_identity(pid).as_deref() == Some(recorded_start) {
                ProcessState::Live
            } else {
                ProcessState::Dead
            }
        }
        state => state,
    }
}

fn valid_session_directory(path: &Path) -> bool {
    session_launcher_pid(path).is_some()
}

fn session_launcher_pid(path: &Path) -> Option<u32> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return None;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let name = path.file_name().and_then(|name| name.to_str())?;
    let value = name.strip_prefix(SESSION_PREFIX)?;
    let mut parts = value.split('-');
    let launcher_pid = parse_process_id(parts.next()?)?;
    let sequence = parts.next()?;
    let parsed_sequence = sequence.parse::<u64>().ok()?;
    if sequence != parsed_sequence.to_string() {
        return None;
    }
    let suffix = parts.next()?;
    if suffix.len() != RANDOM_SUFFIX_LEN
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || parts.next().is_some()
    {
        return None;
    }
    Some(launcher_pid)
}

fn parse_process_id(value: &str) -> Option<u32> {
    let pid = value.parse::<u32>().ok()?;
    (pid != 0 && pid <= MAX_PROCESS_ID && value == pid.to_string()).then_some(pid)
}

#[cfg(unix)]
fn private_marker(metadata: &fs::Metadata) -> bool {
    metadata.mode() & 0o777 == 0o600
}

#[cfg(not(unix))]
fn private_marker(_: &fs::Metadata) -> bool {
    false
}

fn quarantine_and_delete(path: &Path, quarantine_root: &Path) -> Result<(), ProjectionError> {
    if !valid_session_directory(path) {
        return Err(ProjectionError::Unavailable);
    }
    let Some(name) = path.file_name() else {
        return Err(ProjectionError::Unavailable);
    };
    let quarantined = quarantine_root.join(name);
    match fs::rename(path, &quarantined) {
        Ok(()) => match fs::remove_dir_all(quarantined) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ProjectionError::Unavailable),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(ProjectionError::Unavailable),
    }
    Ok(())
}

#[cfg(unix)]
fn process_state(pid: u32) -> ProcessState {
    let pid = pid.to_string();
    match Command::new("/bin/kill")
        .arg("-0")
        .arg(&pid)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => match Command::new("/bin/ps")
            .args(["-p", &pid, "-o", "state="])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                let state = String::from_utf8_lossy(&output.stdout);
                if state.trim_start().starts_with('Z') || state.trim().is_empty() {
                    ProcessState::Dead
                } else {
                    ProcessState::Live
                }
            }
            Ok(_) => ProcessState::Unknown,
            Err(_) => ProcessState::Unknown,
        },
        Ok(_) => ProcessState::Dead,
        Err(_) => ProcessState::Unknown,
    }
}

#[cfg(unix)]
fn process_start_identity(pid: u32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "lstart="])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
}

#[cfg(not(unix))]
fn process_state(_: u32) -> ProcessState {
    ProcessState::Unknown
}

#[cfg(not(unix))]
fn process_start_identity(_: u32) -> Option<String> {
    None
}

fn approved_target_roots(home: &Path, discovery_roots: &[PathBuf]) -> Vec<PathBuf> {
    let Ok(canonical_home) = fs::canonicalize(home) else {
        return Vec::new();
    };
    discovery_roots
        .iter()
        .cloned()
        .chain([
            home.join(".codex/skills"),
            home.join(".codex/plugins/cache"),
            home.join(".claude/plugins/cache"),
        ])
        .filter_map(|root| canonical_nonsymlink_directory(&root))
        .filter(|root| root.starts_with(&canonical_home))
        .collect()
}

fn discover_global_skills(
    roots: &[PathBuf],
    approved_target_roots: &[PathBuf],
) -> Vec<GlobalSkill> {
    let mut inventory = Vec::new();
    for (source_precedence, root) in roots.iter().enumerate() {
        let Some(root) = canonical_nonsymlink_directory(root) else {
            continue;
        };
        if !approved_target_roots
            .iter()
            .any(|approved| approved == &root)
        {
            continue;
        }
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
            if !approved_target_roots
                .iter()
                .any(|root| canonical_path.starts_with(root))
            {
                continue;
            }
            inventory.push(GlobalSkill {
                namespace: plugin_namespace(&canonical_path),
                name,
                source_precedence,
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

fn canonical_nonsymlink_directory(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(path).ok()
}

#[cfg(unix)]
fn link_skill(source: &Path, destination: &Path) -> Result<(), ProjectionError> {
    symlink(source, destination).map_err(|_| ProjectionError::Unavailable)
}

#[cfg(not(unix))]
fn link_skill(_: &Path, _: &Path) -> Result<(), ProjectionError> {
    Err(ProjectionError::Unavailable)
}

fn plugin_namespace(skill_path: &Path) -> Option<String> {
    let skills_root = skill_path
        .ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "skills"))?;
    let package_root = skills_root.parent()?;
    let manifest = package_root.join(".claude-plugin/plugin.json");
    let metadata = fs::metadata(&manifest).ok()?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return None;
    }
    let bytes = fs::read(manifest).ok()?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|name| valid_skill_name(name))
        .map(str::to_owned)
}

fn resolve_skill_selectors<'a>(
    selectors: &[String],
    inventory: &'a [GlobalSkill],
) -> Result<Vec<&'a GlobalSkill>, ProjectionError> {
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
            return Err(ProjectionError::UnknownSelector(selector.clone()));
        }
        for skill in matched {
            winners
                .entry((skill.namespace.clone(), skill.name.clone()))
                .or_insert(skill);
        }
    }
    Ok(winners.into_values().collect())
}

fn validate_selector(selector: &str) -> Result<(), ProjectionError> {
    let mut parts = selector.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if !valid_skill_name(first)
        || second.is_some_and(|part| !valid_skill_name(part))
        || parts.next().is_some()
    {
        return Err(ProjectionError::InvalidSelector(selector.to_owned()));
    }
    Ok(())
}

fn valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn reject_native_name_collisions(selected: &[&GlobalSkill]) -> Result<(), ProjectionError> {
    let mut names = BTreeMap::new();
    for skill in selected {
        if let Some(existing) = names.insert(skill.name.as_str(), skill.canonical_path.as_path())
            && existing != skill.canonical_path
        {
            return Err(ProjectionError::NameCollision(skill.name.clone()));
        }
    }
    Ok(())
}
