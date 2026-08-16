use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const STATE_FILE: &str = "saved-starts.json";
const LOCK_FILE: &str = "saved-starts.lock";
const TEMP_PREFIX: &str = "saved-starts.json.tmp-";
const SCHEMA_VERSION: &str = "taskseal.saved-start.v1";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessClass {
    Standard,
    FullAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStart {
    pub provider: String,
    pub argv: Vec<String>,
    pub project_digest: String,
    pub access_class: AccessClass,
    pub qualification_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStarts {
    pub schema_version: String,
    pub starts: Vec<SavedStart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateError(&'static str);

impl StateError {
    pub fn code(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for StateError {}

#[derive(Debug, Clone)]
pub struct StateStore {
    root: PathBuf,
}

impl StateStore {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    #[cfg(target_os = "macos")]
    #[allow(dead_code)] // T8 is the first user-flow consumer; T7 only establishes the sealed store.
    pub fn for_current_user() -> Result<Self, StateError> {
        let home = std::env::var_os("HOME").ok_or(StateError("STATE_HOME_UNAVAILABLE"))?;
        Ok(Self::at(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Clean Room Launcher"),
        ))
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(dead_code)] // T8 is the first user-flow consumer; T7 only establishes the sealed store.
    pub fn for_current_user() -> Result<Self, StateError> {
        Err(StateError("STATE_PLATFORM_UNSUPPORTED"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    pub fn interrupted_temp_path(&self) -> PathBuf {
        self.root.join(format!("{TEMP_PREFIX}orphan"))
    }

    pub fn load(&self) -> Result<SavedStarts, StateError> {
        self.with_lock(|| self.read_unlocked())
    }

    pub fn save(&self, candidate: SavedStart) -> Result<(), StateError> {
        if contains_sensitive_value(&candidate.argv) {
            return Err(StateError("SAVED_START_SENSITIVE_ARGUMENT_REFUSED"));
        }
        if !valid_record(&candidate) {
            return Err(StateError("STATE_RECORD_REFUSED"));
        }
        self.with_lock(|| {
            let mut saved = self.read_unlocked()?;
            saved.starts.retain(|existing| existing != &candidate);
            saved.starts.insert(0, candidate);
            saved.starts.truncate(3);
            self.replace_unlocked(&saved)
        })
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, StateError>,
    ) -> Result<T, StateError> {
        self.ensure_root()?;
        let lock_path = self.root.join(LOCK_FILE);
        self.refuse_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|_| StateError("STATE_LOCK_OPEN_FAILED"))?;
        self.require_private_regular(&lock_path)?;
        lock.lock().map_err(|_| StateError("STATE_LOCK_FAILED"))?;
        self.refuse_interrupted_temp()?;
        operation()
    }

    fn ensure_root(&self) -> Result<(), StateError> {
        match fs::symlink_metadata(&self.root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(StateError("STATE_SYMLINK_REFUSED"));
            }
            Ok(metadata) if !metadata.is_dir() => return Err(StateError("STATE_ROOT_REFUSED")),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.root)
                    .map_err(|_| StateError("STATE_ROOT_CREATE_FAILED"))?;
            }
            Err(_) => return Err(StateError("STATE_ROOT_REFUSED")),
        }
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
            .map_err(|_| StateError("STATE_ROOT_PERMISSIONS_FAILED"))?;
        let mode = fs::metadata(&self.root)
            .map_err(|_| StateError("STATE_ROOT_REFUSED"))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(StateError("STATE_ROOT_PERMISSIONS_REFUSED"));
        }
        Ok(())
    }

    fn read_unlocked(&self) -> Result<SavedStarts, StateError> {
        let state_path = self.state_path();
        match fs::symlink_metadata(&state_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SavedStarts {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    starts: Vec::new(),
                });
            }
            Err(_) => return Err(StateError("STATE_READ_FAILED")),
            Ok(_) => {}
        }
        self.require_private_regular(&state_path)?;
        let bytes = fs::read(&state_path).map_err(|_| StateError("STATE_READ_FAILED"))?;
        if contains_sensitive_bytes(&bytes) {
            return Err(StateError("SAVED_START_SENSITIVE_ARGUMENT_REFUSED"));
        }
        let saved: SavedStarts =
            serde_json::from_slice(&bytes).map_err(|_| StateError("STATE_CORRUPT_REFUSED"))?;
        if saved.schema_version != SCHEMA_VERSION
            || saved.starts.len() > 3
            || saved.starts.iter().any(|start| !valid_record(start))
        {
            return Err(StateError("STATE_CORRUPT_REFUSED"));
        }
        if saved
            .starts
            .iter()
            .any(|start| contains_sensitive_value(&start.argv))
        {
            return Err(StateError("SAVED_START_SENSITIVE_ARGUMENT_REFUSED"));
        }
        Ok(saved)
    }

    fn replace_unlocked(&self, saved: &SavedStarts) -> Result<(), StateError> {
        let bytes = serde_json::to_vec(saved).map_err(|_| StateError("STATE_SERIALIZE_FAILED"))?;
        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(|_| StateError("STATE_TEMP_CREATE_FAILED"))?;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
            .map_err(|_| StateError("STATE_TEMP_PERMISSIONS_FAILED"))?;
        file.write_all(&bytes)
            .map_err(|_| StateError("STATE_WRITE_FAILED"))?;
        file.sync_all()
            .map_err(|_| StateError("STATE_FSYNC_FAILED"))?;
        fs::rename(&temp_path, self.state_path())
            .map_err(|_| StateError("STATE_REPLACE_FAILED"))?;
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| StateError("STATE_DIRECTORY_FSYNC_FAILED"))
    }

    fn refuse_interrupted_temp(&self) -> Result<(), StateError> {
        let entries = fs::read_dir(&self.root).map_err(|_| StateError("STATE_ROOT_REFUSED"))?;
        for entry in entries {
            let entry = entry.map_err(|_| StateError("STATE_ROOT_REFUSED"))?;
            if entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
                return Err(StateError("STATE_INTERRUPTED_WRITE_REFUSED"));
            }
        }
        Ok(())
    }

    fn refuse_symlink(&self, path: &Path) -> Result<(), StateError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Err(StateError("STATE_SYMLINK_REFUSED"))
            }
            Ok(_) | Err(_) => Ok(()),
        }
    }

    fn require_private_regular(&self, path: &Path) -> Result<(), StateError> {
        let metadata = fs::symlink_metadata(path).map_err(|_| StateError("STATE_READ_FAILED"))?;
        if metadata.file_type().is_symlink() {
            return Err(StateError("STATE_SYMLINK_REFUSED"));
        }
        if !metadata.is_file() {
            return Err(StateError("STATE_FILE_REFUSED"));
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(StateError("STATE_PERMISSIONS_REFUSED"));
        }
        Ok(())
    }
}

fn contains_sensitive_value(argv: &[String]) -> bool {
    argv.iter()
        .any(|value| super::zero_auth::is_sensitive_argument(value))
}

fn contains_sensitive_bytes(bytes: &[u8]) -> bool {
    [
        br"\u".as_slice(),
        b"sk-".as_slice(),
        b"bearer ".as_slice(),
        b"--token".as_slice(),
        b"--with-access-token".as_slice(),
        b"--access-token".as_slice(),
        b"--api-key".as_slice(),
        b"--api_key".as_slice(),
        b"--password".as_slice(),
        b"--secret".as_slice(),
        b"token=".as_slice(),
        b"token:".as_slice(),
        b"api-key=".as_slice(),
        b"api_key=".as_slice(),
        b"password=".as_slice(),
        b"secret=".as_slice(),
    ]
    .iter()
    .any(|needle| {
        bytes.windows(needle.len()).any(|window| {
            window
                .iter()
                .zip(needle.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
    })
}

fn valid_record(candidate: &SavedStart) -> bool {
    matches!(candidate.provider.as_str(), "codex" | "claude")
        && valid_digest(&candidate.project_digest)
        && valid_digest(&candidate.qualification_digest)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
