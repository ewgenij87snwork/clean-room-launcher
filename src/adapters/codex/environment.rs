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
        || policy.tuple.artifact_digest
            != "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37"
        || policy.tuple.version != (0, 147, 0)
        || policy.tuple.os != "macos"
        || policy.tuple.arch != "aarch64"
        || policy.declaration_digest
            != "980c03af9c28ddbb7280a52fd72a39790544e8a649ccd2442e078e95e9db6337"
        || policy.declaration_digest.len() != 64
        || !policy
            .declaration_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || requested_names.iter().any(|name| {
            name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
        })
        || !requested_names.is_empty()
    {
        return CodexAuthState::Refused;
    }
    CodexAuthState::Unavailable
}
