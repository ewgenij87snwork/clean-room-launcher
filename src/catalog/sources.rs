use crate::core::inventory::sha256_hex;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillSourceAuthority {
    Project,
    ExplicitImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSourceVisibility {
    Admitted,
    OutsideBoundary,
}

#[derive(Debug, Clone)]
pub struct SkillSource {
    pub id: String,
    pub root: PathBuf,
    pub authority: SkillSourceAuthority,
    pub admitted: bool,
    pub visibility: SkillSourceVisibility,
    pub(crate) root_capability: Option<Arc<Dir>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSourceConfig {
    sources: Vec<(PathBuf, SkillSourceAuthority)>,
}

impl SkillSourceConfig {
    pub fn new(sources: Vec<(PathBuf, SkillSourceAuthority)>) -> Self {
        Self { sources }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    MissingRoot,
    SymlinkRoot,
    DuplicateRoot,
    InvalidPath,
}

pub fn enumerate_sources(
    config: &SkillSourceConfig,
    capability_roots: &[PathBuf],
) -> Result<Vec<SkillSource>, CatalogError> {
    let admitted_roots = capability_roots
        .iter()
        .map(|root| normalize_root(root))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    let mut sources = Vec::with_capacity(config.sources.len());

    for (root, authority) in &config.sources {
        let normalized = normalize_root(root)?;
        if !seen.insert(normalized.clone()) {
            return Err(CatalogError::DuplicateRoot);
        }
        let mut admitted = false;
        let mut root_capability = None;
        for capability in &admitted_roots {
            match descriptor_admits(&normalized, capability) {
                Ok(Some(handle)) => {
                    admitted = true;
                    root_capability = Some(Arc::new(handle));
                    break;
                }
                Ok(None) => {}
                Err(error) => return Err(error),
            }
        }
        let visibility = if admitted {
            SkillSourceVisibility::Admitted
        } else {
            SkillSourceVisibility::OutsideBoundary
        };
        sources.push(SkillSource {
            id: stable_id(&normalized, *authority)?,
            root: normalized,
            authority: *authority,
            admitted,
            visibility,
            root_capability,
        });
    }

    sources.sort_by(|left, right| left.root.as_os_str().cmp(right.root.as_os_str()));
    Ok(sources)
}

fn normalize_root(root: &Path) -> Result<PathBuf, CatalogError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| CatalogError::MissingRoot)?;
    if metadata.file_type().is_symlink() {
        return Err(CatalogError::SymlinkRoot);
    }
    if !metadata.is_dir() {
        return Err(CatalogError::InvalidPath);
    }
    if !root.is_absolute() {
        return Err(CatalogError::InvalidPath);
    }
    Ok(root.to_path_buf())
}

fn descriptor_admits(candidate: &Path, capability: &Path) -> Result<Option<Dir>, CatalogError> {
    let relative = match candidate.strip_prefix(capability) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let mut dir = Dir::open_ambient_dir(capability, ambient_authority())
        .map_err(|_| CatalogError::InvalidPath)?;
    for component in relative.components() {
        let name = component.as_os_str();
        let metadata = dir
            .symlink_metadata(name)
            .map_err(|_| CatalogError::MissingRoot)?;
        if metadata.file_type().is_symlink() {
            return Err(CatalogError::SymlinkRoot);
        }
        dir = dir.open_dir(name).map_err(|_| CatalogError::InvalidPath)?
    }
    Ok(Some(dir))
}

fn stable_id(root: &Path, authority: SkillSourceAuthority) -> Result<String, CatalogError> {
    let root = root.to_str().ok_or(CatalogError::InvalidPath)?;
    let authority = match authority {
        SkillSourceAuthority::Project => "project",
        SkillSourceAuthority::ExplicitImport => "explicit-import",
    };
    let digest = sha256_hex(format!("{authority}\0{root}").as_bytes());
    Ok(format!("skill-source-{}", &digest[..16]))
}
