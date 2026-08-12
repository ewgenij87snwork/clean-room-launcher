use super::level_a::LevelAEntry;
use crate::contracts::adapter::AdapterDeclaration;
use crate::core::inventory::sha256_hex;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationReceipt {
    pub schema_version: String,
    pub provider_id: String,
    pub declaration_digest: String,
    pub native_progressive_disclosure: bool,
    pub projection_candidate: bool,
    pub observed_digests: BTreeMap<String, String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidReceipt,
    UnsupportedNativeSeam,
    ReceiptBindingMismatch,
    DigestMismatch(String),
}

pub fn parse_qualification_receipt(
    bytes: &[u8],
    declaration: &AdapterDeclaration,
) -> Result<QualificationReceipt, ProjectionError> {
    let receipt: QualificationReceipt =
        serde_json::from_slice(bytes).map_err(|_| ProjectionError::InvalidReceipt)?;
    if receipt.schema_version != "taskseal.native-projection-qualification.v1"
        || receipt.provider_id != declaration.provider_id
        || receipt.declaration_digest != declaration_digest(declaration)
    {
        return Err(ProjectionError::ReceiptBindingMismatch);
    }
    Ok(receipt)
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
    if r.schema_version != "taskseal.native-projection-qualification.v1"
        || r.provider_id != d.provider_id
        || r.declaration_digest != declaration_digest(d)
    {
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
