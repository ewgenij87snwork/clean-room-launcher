use crate::core::inventory::sha256_hex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub id: String,
    pub root: PathBuf,
    pub authority: SkillSourceAuthority,
    pub admitted: bool,
    pub visibility: SkillSourceVisibility,
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
        let admitted = admitted_roots
            .iter()
            .any(|capability| is_within(&normalized, capability));
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
    root.canonicalize().map_err(|_| CatalogError::InvalidPath)
}

fn is_within(candidate: &Path, capability: &Path) -> bool {
    candidate == capability || candidate.starts_with(capability)
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
