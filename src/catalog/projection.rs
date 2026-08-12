use super::level_a::LevelAEntry;
use crate::contracts::adapter::AdapterDeclaration;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct CapabilityTruth {
    pub native_progressive_disclosure: bool,
    pub observed_digests: BTreeMap<String, String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    UnsupportedNativeSeam,
    DigestMismatch(String),
}
#[derive(Debug)]
pub struct NativeEntry {
    pub id: String,
    pub name: String,
    pub body_digest: String,
    pub native_link: String,
}
#[derive(Debug)]
pub struct NativeProjection {
    pub provider_id: String,
    pub entries: Vec<NativeEntry>,
    pub startup_bytes: Vec<u8>,
}

pub fn project_native(
    level_a: &[LevelAEntry],
    declaration: &AdapterDeclaration,
    truth: CapabilityTruth,
) -> Result<NativeProjection, ProjectionError> {
    if !declaration.qualified
        || declaration.context_target != "provider_native_context"
        || !truth.native_progressive_disclosure
    {
        return Err(ProjectionError::UnsupportedNativeSeam);
    }
    let mut entries = Vec::with_capacity(level_a.len());
    for entry in level_a {
        if truth.observed_digests.get(&entry.id) != Some(&entry.body_digest) {
            return Err(ProjectionError::DigestMismatch(entry.id.clone()));
        }
        entries.push(NativeEntry {
            id: entry.id.clone(),
            name: entry.name.clone(),
            body_digest: entry.body_digest.clone(),
            native_link: format!("skill://{}/{}", declaration.provider_id, entry.id),
        });
    }
    let startup_bytes =
        serde_json::to_vec(level_a).map_err(|_| ProjectionError::UnsupportedNativeSeam)?;
    Ok(NativeProjection {
        provider_id: declaration.provider_id.clone(),
        entries,
        startup_bytes,
    })
}
