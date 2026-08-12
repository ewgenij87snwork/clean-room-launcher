use super::level_a::LevelAEntry;
use crate::contracts::adapter::AdapterDeclaration;
use crate::core::inventory::sha256_hex;
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub struct QualificationReceipt {
    pub provider_id: String,
    pub declaration_digest: String,
    pub native_progressive_disclosure: bool,
    pub projection_candidate: bool,
    pub observed_digests: BTreeMap<String, String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    UnsupportedNativeSeam,
    ReceiptBindingMismatch,
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
pub fn declaration_digest(d: &AdapterDeclaration) -> String {
    sha256_hex(
        format!(
            "{}\0{}\0{}\0{}\0{}\0{}",
            d.provider_id,
            d.executable,
            d.version_range,
            d.context_target,
            d.collision_policy,
            d.capability_evidence
        )
        .as_bytes(),
    )
}
pub fn project_native(
    level_a: &[LevelAEntry],
    d: &AdapterDeclaration,
    r: QualificationReceipt,
) -> Result<NativeProjection, ProjectionError> {
    if r.provider_id != d.provider_id || r.declaration_digest != declaration_digest(d) {
        return Err(ProjectionError::ReceiptBindingMismatch);
    }
    if d.context_target != "provider_native_context"
        || !r.native_progressive_disclosure
        || !r.projection_candidate
    {
        return Err(ProjectionError::UnsupportedNativeSeam);
    }
    let mut entries = Vec::new();
    for e in level_a {
        if r.observed_digests.get(&e.id) != Some(&e.body_digest) {
            return Err(ProjectionError::DigestMismatch(e.id.clone()));
        }
        entries.push(NativeEntry {
            id: e.id.clone(),
            name: e.name.clone(),
            body_digest: e.body_digest.clone(),
            native_link: format!("skill://{}/{}", d.provider_id, e.id),
        })
    }
    let startup_bytes =
        serde_json::to_vec(level_a).map_err(|_| ProjectionError::UnsupportedNativeSeam)?;
    Ok(NativeProjection {
        provider_id: d.provider_id.clone(),
        entries,
        startup_bytes,
    })
}
