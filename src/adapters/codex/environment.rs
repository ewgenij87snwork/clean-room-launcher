use super::identity::CodexTuple;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexAuthPolicy {
    pub tuple: CodexTuple,
    pub declaration_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexAuthState {
    Unavailable,
    Refused,
}

impl CodexAuthPolicy {
    pub fn unavailable(tuple: CodexTuple, declaration_digest: String) -> Self {
        Self {
            tuple,
            declaration_digest,
        }
    }
}

pub fn evaluate_auth(policy: &CodexAuthPolicy, requested_names: &[&str]) -> CodexAuthState {
    if policy.tuple.provider_id != "codex"
        || policy.declaration_digest.len() != 64
        || !policy
            .declaration_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || !requested_names.is_empty()
    {
        return CodexAuthState::Refused;
    }
    CodexAuthState::Unavailable
}
