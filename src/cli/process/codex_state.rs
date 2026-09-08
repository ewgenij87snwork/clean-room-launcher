use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

const APP_SUPPORT_DIR: &str = "Library/Application Support/Clean Room Launcher/Codex";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexState {
    pub(crate) root: PathBuf,
    pub(crate) shadow_home: PathBuf,
    pub(crate) sqlite_home: PathBuf,
}

pub(super) fn prepare(
    home: &Path,
    ambient_codex_home: &Path,
    selected_global_skill_paths: &[(String, PathBuf)],
) -> Result<CodexState, String> {
    let ambient_auth = ambient_codex_home.join("auth.json");
    if !fs::metadata(&ambient_auth).is_ok_and(|metadata| metadata.is_file()) {
        return Err("CLROOM_CODEX_AUTH_UNAVAILABLE".to_owned());
    }

    let root = home.join(APP_SUPPORT_DIR);
    let shadow_home = root.join("home");
    let sqlite_home = root.join("sqlite");
    ensure_private_directory(&root)?;
    ensure_private_directory(&shadow_home)?;
    ensure_private_directory(&sqlite_home)?;
    reject_shadow_junk(&shadow_home)?;
    project_selected_skills(
        &shadow_home,
        ambient_codex_home,
        home,
        selected_global_skill_paths,
    )?;

    let auth_link = shadow_home.join("auth.json");
    match fs::symlink_metadata(&auth_link) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if fs::read_link(&auth_link).ok().as_deref() != Some(ambient_auth.as_path()) {
                return Err("CLROOM_CODEX_AUTH_REFERENCE_INVALID".to_owned());
            }
        }
        Ok(_) => return Err("CLROOM_CODEX_STATE_DIRTY".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            symlink(&ambient_auth, &auth_link)
                .map_err(|_| "CLROOM_CODEX_AUTH_REFERENCE_FAILED".to_owned())?;
        }
        Err(_) => return Err("CLROOM_CODEX_AUTH_REFERENCE_INVALID".to_owned()),
    }

    Ok(CodexState {
        root,
        shadow_home,
        sqlite_home,
    })
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("CLROOM_CODEX_STATE_DIR_INVALID".to_owned());
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err("CLROOM_CODEX_STATE_DIR_INVALID".to_owned());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path)
                .map_err(|_| "CLROOM_CODEX_STATE_DIR_CREATE_FAILED".to_owned())?;
        }
        Err(_) => return Err("CLROOM_CODEX_STATE_DIR_INVALID".to_owned()),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "CLROOM_CODEX_STATE_PERMISSIONS_FAILED".to_owned())?;
    let metadata = fs::metadata(path).map_err(|_| "CLROOM_CODEX_STATE_DIR_INVALID".to_owned())?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err("CLROOM_CODEX_STATE_PERMISSIONS_REFUSED".to_owned());
    }
    Ok(())
}

fn reject_shadow_junk(shadow_home: &Path) -> Result<(), String> {
    for name in [
        "config.toml",
        "plugins",
        "hooks",
        "AGENTS.md",
        "AGENTS.override.md",
    ] {
        if fs::symlink_metadata(shadow_home.join(name)).is_ok() {
            return Err("CLROOM_CODEX_STATE_DIRTY".to_owned());
        }
    }
    Ok(())
}

fn project_selected_skills(
    shadow_home: &Path,
    ambient_codex_home: &Path,
    home: &Path,
    selected_global_skill_paths: &[(String, PathBuf)],
) -> Result<(), String> {
    let skills_root = shadow_home.join("skills");
    if selected_global_skill_paths.is_empty() {
        if let Ok(entries) = fs::read_dir(&skills_root) {
            for entry in entries {
                let entry = entry.map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
                let target = fs::read_link(entry.path())
                    .map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
                if target.starts_with(ambient_codex_home) || target.starts_with(home) {
                    fs::remove_file(entry.path())
                        .map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
                } else {
                    return Err("CLROOM_CODEX_STATE_DIRTY".to_owned());
                }
            }
        }
        return Ok(());
    }
    ensure_private_directory(&skills_root)?;
    let selected = selected_global_skill_paths
        .iter()
        .map(|(name, path)| (name.as_str(), path.as_path()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for entry in fs::read_dir(&skills_root).map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())? {
        let entry = entry.map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
        let target =
            fs::read_link(entry.path()).map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            return Err("CLROOM_CODEX_STATE_DIRTY".to_owned());
        };
        if selected
            .get(name.as_str())
            .is_some_and(|path| *path == target)
        {
            continue;
        }
        if target.starts_with(ambient_codex_home) || target.starts_with(home) {
            fs::remove_file(entry.path()).map_err(|_| "CLROOM_CODEX_STATE_DIRTY".to_owned())?;
        } else {
            return Err("CLROOM_CODEX_STATE_DIRTY".to_owned());
        }
    }
    for (name, source) in selected_global_skill_paths {
        let link = skills_root.join(name);
        if fs::symlink_metadata(&link).is_err() {
            symlink(source, &link)
                .map_err(|_| "CLROOM_CODEX_SKILL_PROJECTION_FAILED".to_owned())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::prepare;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "clroom-codex-state-test-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_stable_private_clean_home_with_auth_reference() {
        let scratch = Scratch::new();
        let home = scratch.0.join("home");
        fs::create_dir_all(&home).unwrap();
        let ambient_codex_home = home.join(".codex");
        fs::create_dir_all(&ambient_codex_home).unwrap();
        fs::write(ambient_codex_home.join("auth.json"), b"credential bytes").unwrap();
        assert!(fs::metadata(ambient_codex_home.join("auth.json")).is_ok());

        let first = prepare(&home, &ambient_codex_home, &[]).unwrap();
        let second = prepare(&home, &ambient_codex_home, &[]).unwrap();

        assert_eq!(first, second);
        assert!(first.shadow_home.join("auth.json").is_symlink());
        assert!(!first.shadow_home.join("config.toml").exists());
        assert!(!first.shadow_home.join("skills").exists());
        assert!(!first.shadow_home.join("plugins").exists());
        assert!(!first.shadow_home.join("hooks").exists());
        assert_eq!(
            fs::metadata(&first.root).unwrap().permissions().mode() & 0o077,
            0
        );
    }

    #[test]
    fn refuses_ambient_auth_without_creating_provider_state() {
        let scratch = Scratch::new();
        let home = scratch.0.join("home");
        fs::create_dir_all(&home).unwrap();
        let ambient_codex_home = home.join(".codex");
        fs::create_dir_all(&ambient_codex_home).unwrap();

        let error = prepare(&home, &ambient_codex_home, &[]).unwrap_err();

        assert_eq!(error, "CLROOM_CODEX_AUTH_UNAVAILABLE");
        assert!(
            !home
                .join("Library/Application Support/Clean Room Launcher/Codex")
                .exists()
        );
    }

    #[test]
    fn refuses_preexisting_shadow_config_instead_of_deleting_it() {
        let scratch = Scratch::new();
        let home = scratch.0.join("home");
        fs::create_dir_all(&home).unwrap();
        let ambient_codex_home = home.join(".codex");
        fs::create_dir_all(&ambient_codex_home).unwrap();
        fs::write(ambient_codex_home.join("auth.json"), b"credential bytes").unwrap();
        let shadow_home = home.join("Library/Application Support/Clean Room Launcher/Codex/home");
        fs::create_dir_all(&shadow_home).unwrap();
        fs::write(shadow_home.join("config.toml"), b"owner state").unwrap();

        let error = prepare(&home, &ambient_codex_home, &[]).unwrap_err();

        assert_eq!(error, "CLROOM_CODEX_STATE_DIRTY");
        assert_eq!(
            fs::read(shadow_home.join("config.toml")).unwrap(),
            b"owner state"
        );
    }

    #[test]
    fn refreshes_only_clroom_skill_links_across_launches() {
        let scratch = Scratch::new();
        let home = scratch.0.join("home");
        fs::create_dir_all(&home).unwrap();
        let ambient_codex_home = home.join(".codex");
        let source = home.join(".agents/skills/arrow");
        fs::create_dir_all(&ambient_codex_home).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(ambient_codex_home.join("auth.json"), b"credential bytes").unwrap();
        fs::write(source.join("SKILL.md"), b"selected").unwrap();

        let selected = [("arrow".to_owned(), source.clone())];
        let first = prepare(&home, &ambient_codex_home, &selected).unwrap();
        let second = prepare(&home, &ambient_codex_home, &selected).unwrap();
        assert!(second.shadow_home.join("skills/arrow").is_symlink());
        let clean = prepare(&home, &ambient_codex_home, &[]).unwrap();

        assert_eq!(first, second);
        assert!(!clean.shadow_home.join("skills/arrow").exists());
    }
}
