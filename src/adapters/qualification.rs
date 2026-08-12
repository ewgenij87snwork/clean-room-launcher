#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleClaim {
    pub provider_id: String,
    pub declaration_digest: String,
    pub artifact_digest: String,
    pub version: (u64, u64, u64),
    pub os: String,
    pub arch: String,
    pub interpreter_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub kind: String,
    pub claim: TupleClaim,
    pub observed_at: u64,
    pub expires_at: u64,
    pub digest: String,
    pub refused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderQualificationReceipt { pub claim: TupleClaim, pub evidence: Vec<EvidenceRef> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationReason { InvalidClaim, StaleEvidence, RefusedEvidence, TupleMismatch, ProviderLaunchMissing }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationState { Qualified(ProviderQualificationReceipt), NotQualified { reason: QualificationReason }, Refused { reason: QualificationReason } }

pub fn qualify(evidence: &[EvidenceRef], claim: &TupleClaim, as_of: u64) -> QualificationState {
    if !valid_claim(claim) { return QualificationState::Refused { reason: QualificationReason::InvalidClaim }; }
    if evidence.iter().any(|item| item.claim != *claim) { return QualificationState::Refused { reason: QualificationReason::TupleMismatch }; }
    if evidence.iter().any(|item| item.refused) { return QualificationState::Refused { reason: QualificationReason::RefusedEvidence }; }
    if evidence.iter().any(|item| item.observed_at > item.expires_at || item.expires_at < as_of) { return QualificationState::NotQualified { reason: QualificationReason::StaleEvidence }; }
    if !evidence.iter().any(|item| item.kind == "provider-launch-observed") { return QualificationState::NotQualified { reason: QualificationReason::ProviderLaunchMissing }; }
    QualificationState::Qualified(ProviderQualificationReceipt { claim: claim.clone(), evidence: evidence.to_vec() })
}

fn valid_claim(claim: &TupleClaim) -> bool {
    !claim.provider_id.is_empty() && [claim.declaration_digest.as_str(), claim.artifact_digest.as_str()].into_iter().all(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())) && !claim.os.is_empty() && !claim.arch.is_empty()
}
