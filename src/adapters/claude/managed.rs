use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presence {
    Present,
    Absent,
    Unknown,
}

pub fn probe() -> Presence {
    probe_paths(&[
        PathBuf::from("/Library/Managed Preferences/com.anthropic.claude.plist"),
        PathBuf::from("/Library/Managed Preferences/com.anthropic.claude.settings.plist"),
        PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json"),
        PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.json"),
        PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.d"),
        PathBuf::from("/Library/Application Support/ClaudeCode/managed-mcp.d"),
    ])
}

pub fn probe_paths(paths: &[impl AsRef<Path>]) -> Presence {
    let mut unknown = false;
    for path in paths {
        match std::fs::symlink_metadata(path.as_ref()) {
            Ok(_) => return Presence::Present,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => unknown = true,
        }
    }
    if unknown {
        Presence::Unknown
    } else {
        Presence::Absent
    }
}

#[cfg(test)]
mod tests {
    use super::{Presence, probe_paths};
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    #[test]
    fn metadata_probe_detects_managed_dropin_without_reading_contents() {
        let root =
            std::env::temp_dir().join(format!("clroom-managed-probe-{}", std::process::id()));
        let dropin = root.join("managed-settings.d");
        std::fs::create_dir_all(&dropin).unwrap();
        assert_eq!(probe_paths(&[dropin]), Presence::Present);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_probe_path_is_unknown_not_absent() {
        let invalid = PathBuf::from(std::ffi::OsString::from_vec(b"invalid\0path".to_vec()));
        assert_eq!(probe_paths(&[invalid]), Presence::Unknown);
    }

    #[test]
    fn all_missing_documented_paths_are_absent() {
        let root =
            std::env::temp_dir().join(format!("clroom-managed-absent-{}", std::process::id()));
        let paths = [root.join("a"), root.join("b"), root.join("c")];
        assert_eq!(super::probe_paths(&paths), Presence::Absent);
        let _ = std::fs::remove_dir_all(root);
    }
}
