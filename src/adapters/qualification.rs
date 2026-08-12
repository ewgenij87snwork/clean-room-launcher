#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TupleClaim {
    pub provider_id: String,
    pub declaration_digest: String,
    pub artifact_digest: String,
    pub version: (u64, u64, u64),
    pub os: String,
    pub arch: String,
    pub interpreter_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub kind: String,
    pub claim: TupleClaim,
    pub observed_at: u64,
    pub expires_at: u64,
    pub digest: String,
    pub refused: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvidence {
    schema_version: String,
    kind: String,
    provider_id: String,
    declaration_digest: String,
    artifact_digest: String,
    version: [u64; 3],
    os: String,
    arch: String,
    interpreter_digest: Option<String>,
    observed_at: u64,
    expires_at: u64,
    output_digest: String,
    refused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderQualificationReceipt { pub claim: TupleClaim, pub evidence: Vec<EvidenceRef> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationReason { InvalidClaim, StaleEvidence, RefusedEvidence, TupleMismatch, ProviderLaunchMissing }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationState { Qualified(ProviderQualificationReceipt), NotQualified { reason: QualificationReason }, Refused { reason: QualificationReason } }

pub fn parse_evidence(bytes: &[u8], output: &[u8], claim: &TupleClaim) -> Result<EvidenceRef, QualificationReason> {
    let stored: StoredEvidence = serde_json::from_slice(bytes).map_err(|_| QualificationReason::InvalidClaim)?;
    if stored.schema_version != "taskseal.provider-evidence.v1" || stored.kind.is_empty() || stored.output_digest != crate::core::inventory::sha256_hex(output) || stored.provider_id != claim.provider_id || stored.declaration_digest != claim.declaration_digest || stored.artifact_digest != claim.artifact_digest || stored.version != [claim.version.0, claim.version.1, claim.version.2] || stored.os != claim.os || stored.arch != claim.arch || stored.interpreter_digest != claim.interpreter_digest || stored.observed_at > stored.expires_at {
        return Err(QualificationReason::InvalidClaim);
    }
    Ok(EvidenceRef { kind: stored.kind, claim: claim.clone(), observed_at: stored.observed_at, expires_at: stored.expires_at, digest: stored.output_digest, refused: stored.refused })
}

pub fn qualify(evidence: &[EvidenceRef], claim: &TupleClaim, as_of: u64) -> QualificationState {
    if !valid_claim(claim) { return QualificationState::Refused { reason: QualificationReason::InvalidClaim }; }
    if evidence.iter().any(|item| item.claim != *claim) { return QualificationState::Refused { reason: QualificationReason::TupleMismatch }; }
    if evidence.iter().any(|item| item.refused) { return QualificationState::Refused { reason: QualificationReason::RefusedEvidence }; }
    if evidence.iter().any(|item| item.observed_at > item.expires_at || item.expires_at < as_of) { return QualificationState::NotQualified { reason: QualificationReason::StaleEvidence }; }
    // Generic foundation may validate that launch evidence is absent, but never creates a
    // provider qualification. Only the tuple-specific T12 verifier may construct one.
    let _ = evidence;
    QualificationState::NotQualified { reason: QualificationReason::ProviderLaunchMissing }
}

pub fn receipt_digest(receipt: &ProviderQualificationReceipt) -> Result<String, QualificationReason> {
    let bytes = serde_json::to_vec(receipt).map_err(|_| QualificationReason::InvalidClaim)?;
    Ok(crate::core::inventory::sha256_hex(&bytes))
}

fn valid_claim(claim: &TupleClaim) -> bool {
    !claim.provider_id.is_empty() && [claim.declaration_digest.as_str(), claim.artifact_digest.as_str()].into_iter().all(valid_digest) && !claim.os.is_empty() && !claim.arch.is_empty()
}

fn valid_digest(digest: &str) -> bool { digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) }
use serde::Deserialize;
