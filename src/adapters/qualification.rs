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
pub struct ProviderQualificationReceipt {
    pub schema_version: String,
    pub verifier_version: String,
    pub claim: TupleClaim,
    pub evidence: Vec<EvidenceRef>,
    pub observed_at: u64,
    pub expires_at: u64,
    pub receipt_digest: String,
}

#[derive(serde::Serialize)]
struct ReceiptCore<'a> {
    schema_version: &'a str,
    verifier_version: &'a str,
    claim: &'a TupleClaim,
    evidence: &'a [EvidenceRef],
    observed_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationReason {
    InvalidClaim,
    StaleEvidence,
    RefusedEvidence,
    TupleMismatch,
    ProviderLaunchMissing,
    ReceiptMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationState {
    Qualified(ProviderQualificationReceipt),
    NotQualified { reason: QualificationReason },
    Refused { reason: QualificationReason },
}

pub fn parse_evidence(
    bytes: &[u8],
    output: &[u8],
    claim: &TupleClaim,
) -> Result<EvidenceRef, QualificationReason> {
    let stored: StoredEvidence =
        serde_json::from_slice(bytes).map_err(|_| QualificationReason::InvalidClaim)?;
    if stored.schema_version != "taskseal.provider-evidence.v1"
        || stored.kind.is_empty()
        || stored.output_digest != crate::core::inventory::sha256_hex(output)
        || stored.provider_id != claim.provider_id
        || stored.declaration_digest != claim.declaration_digest
        || stored.artifact_digest != claim.artifact_digest
        || stored.version != [claim.version.0, claim.version.1, claim.version.2]
        || stored.os != claim.os
        || stored.arch != claim.arch
        || stored.interpreter_digest != claim.interpreter_digest
        || stored.observed_at > stored.expires_at
    {
        return Err(QualificationReason::InvalidClaim);
    }
    Ok(EvidenceRef {
        kind: stored.kind,
        claim: claim.clone(),
        observed_at: stored.observed_at,
        expires_at: stored.expires_at,
        digest: stored.output_digest,
        refused: stored.refused,
    })
}

pub fn qualify(evidence: &[EvidenceRef], claim: &TupleClaim, as_of: u64) -> QualificationState {
    if !valid_claim(claim) {
        return QualificationState::Refused {
            reason: QualificationReason::InvalidClaim,
        };
    }
    if evidence.iter().any(|item| item.claim != *claim) {
        return QualificationState::Refused {
            reason: QualificationReason::TupleMismatch,
        };
    }
    if evidence.iter().any(|item| item.refused) {
        return QualificationState::Refused {
            reason: QualificationReason::RefusedEvidence,
        };
    }
    if evidence
        .iter()
        .any(|item| item.observed_at > item.expires_at || item.expires_at < as_of)
    {
        return QualificationState::NotQualified {
            reason: QualificationReason::StaleEvidence,
        };
    }
    // Generic foundation may validate that launch evidence is absent, but never creates a
    // provider qualification. Only the tuple-specific T12 verifier may construct one.
    let _ = evidence;
    QualificationState::NotQualified {
        reason: QualificationReason::ProviderLaunchMissing,
    }
}

pub fn seal_receipt(
    claim: TupleClaim,
    mut evidence: Vec<EvidenceRef>,
    observed_at: u64,
    expires_at: u64,
) -> Result<ProviderQualificationReceipt, QualificationReason> {
    evidence.sort_by(|left, right| left.kind.cmp(&right.kind));
    let mut receipt = ProviderQualificationReceipt {
        schema_version: "taskseal.provider-qualification-receipt.v1".into(),
        verifier_version: "taskseal.p06.t4.v1".into(),
        claim,
        evidence,
        observed_at,
        expires_at,
        receipt_digest: String::new(),
    };
    validate_receipt(&receipt)?;
    receipt.receipt_digest = receipt_digest(&receipt)?;
    Ok(receipt)
}

pub fn verify_receipt(receipt: &ProviderQualificationReceipt) -> Result<(), QualificationReason> {
    validate_receipt(receipt)?;
    if receipt.receipt_digest != receipt_digest(receipt)? {
        return Err(QualificationReason::ReceiptMismatch);
    }
    Ok(())
}

pub fn receipt_digest(
    receipt: &ProviderQualificationReceipt,
) -> Result<String, QualificationReason> {
    let core = ReceiptCore {
        schema_version: &receipt.schema_version,
        verifier_version: &receipt.verifier_version,
        claim: &receipt.claim,
        evidence: &receipt.evidence,
        observed_at: receipt.observed_at,
        expires_at: receipt.expires_at,
    };
    let bytes = serde_json::to_vec(&core).map_err(|_| QualificationReason::InvalidClaim)?;
    Ok(crate::core::inventory::sha256_hex(&bytes))
}

fn validate_receipt(receipt: &ProviderQualificationReceipt) -> Result<(), QualificationReason> {
    if receipt.schema_version != "taskseal.provider-qualification-receipt.v1"
        || receipt.verifier_version != "taskseal.p06.t4.v1"
        || !valid_claim(&receipt.claim)
        || receipt.observed_at > receipt.expires_at
        || !valid_digest(&receipt.receipt_digest) && !receipt.receipt_digest.is_empty()
    {
        return Err(QualificationReason::InvalidClaim);
    }
    let mut previous_kind = None;
    for item in &receipt.evidence {
        if !valid_kind(&item.kind) || item.claim != receipt.claim {
            return Err(QualificationReason::TupleMismatch);
        }
        if item.refused {
            return Err(QualificationReason::RefusedEvidence);
        }
        if !valid_digest(&item.digest)
            || item.observed_at > item.expires_at
            || item.observed_at < receipt.observed_at
            || item.expires_at > receipt.expires_at
        {
            return Err(QualificationReason::StaleEvidence);
        }
        if previous_kind.is_some_and(|previous| previous >= item.kind.as_str()) {
            return Err(QualificationReason::InvalidClaim);
        }
        previous_kind = Some(item.kind.as_str());
    }
    if receipt.evidence.is_empty() {
        return Err(QualificationReason::InvalidClaim);
    }
    Ok(())
}

fn valid_claim(claim: &TupleClaim) -> bool {
    [
        claim.provider_id.as_str(),
        claim.os.as_str(),
        claim.arch.as_str(),
    ]
    .into_iter()
    .all(valid_label)
        && [
            claim.declaration_digest.as_str(),
            claim.artifact_digest.as_str(),
        ]
        .into_iter()
        .all(valid_digest)
        && claim.interpreter_digest.as_deref().is_none_or(valid_digest)
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_kind(kind: &str) -> bool {
    valid_label(kind)
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
}
use serde::Deserialize;
