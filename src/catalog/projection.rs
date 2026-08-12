use super::level_a::LevelAEntry;
use crate::contracts::adapter::AdapterDeclaration;
use crate::core::inventory::sha256_hex;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationReceipt {
    pub schema_version: String,
    pub provider_id: String,
    pub declaration_digest: String,
    pub native_progressive_disclosure: bool,
    pub projection_candidate: bool,
    #[serde(deserialize_with = "deserialize_digest_map")]
    pub observed_digests: BTreeMap<String, String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidReceipt,
    UnsupportedNativeSeam,
    ReceiptBindingMismatch,
    DigestMismatch(String),
}

fn deserialize_digest_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct DigestMapVisitor;
    impl<'de> Visitor<'de> for DigestMapVisitor {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of unique skill IDs to SHA-256 digests")
        }
        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, String>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate skill ID: {key}"
                    )));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(DigestMapVisitor)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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
    if receipt.observed_digests.is_empty()
        || receipt
            .observed_digests
            .values()
            .any(|digest| !valid_sha256(digest))
    {
        return Err(ProjectionError::InvalidReceipt);
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
    let expected_ids: BTreeSet<_> = level_a.iter().map(|entry| entry.id.as_str()).collect();
    let observed_ids: BTreeSet<_> = r.observed_digests.keys().map(String::as_str).collect();
    if expected_ids != observed_ids {
        return Err(ProjectionError::ReceiptBindingMismatch);
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
