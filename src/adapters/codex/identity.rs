use crate::{
    adapters::identity::{AdapterError, ProviderIdentity, resolve_identity, revalidate_identity},
    adapters::session::ProviderNativePreauthenticatedSession,
    contracts::adapter::AdapterDeclaration,
};
use std::path::Path;

const INSTALLED_ARTIFACT_DIGEST: &str =
    "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37";
const INSTALLED_VERSION: (u64, u64, u64) = (0, 147, 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexTuple {
    pub provider_id: String,
    pub artifact_digest: String,
    pub version: (u64, u64, u64),
    pub os: String,
    pub arch: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CodexTupleError {
    Identity(AdapterError),
    DeclarationMismatch,
    UnqualifiedTuple,
}

pub fn resolve_installed_tuple(
    session: ProviderNativePreauthenticatedSession,
    declaration: &AdapterDeclaration,
    command: &Path,
) -> Result<CodexTuple, CodexTupleError> {
    validate_approved_declaration(declaration)?;
    let identity =
        resolve_identity(session, declaration, command).map_err(CodexTupleError::Identity)?;
    revalidate_identity(&identity).map_err(CodexTupleError::Identity)?;
    bind_resolved_tuple(declaration, &identity)
}

pub fn bind_resolved_tuple(
    declaration: &AdapterDeclaration,
    identity: &ProviderIdentity,
) -> Result<CodexTuple, CodexTupleError> {
    validate_approved_declaration(declaration)?;
    if identity.provider_id != "codex"
        || identity.artifact_digest != INSTALLED_ARTIFACT_DIGEST
        || identity.version != INSTALLED_VERSION
        || identity.os != "macos"
        || identity.arch != "aarch64"
    {
        return Err(CodexTupleError::UnqualifiedTuple);
    }
    Ok(CodexTuple {
        provider_id: identity.provider_id.clone(),
        artifact_digest: identity.artifact_digest.clone(),
        version: identity.version,
        os: identity.os.clone(),
        arch: identity.arch.clone(),
    })
}

fn validate_approved_declaration(declaration: &AdapterDeclaration) -> Result<(), CodexTupleError> {
    if declaration.provider_id != "codex"
        || declaration.executable != "codex"
        || declaration.version_range != ">=0.147.0"
        || declaration.context_target != "provider_native_context"
        || declaration.collision_policy != "deny"
        || declaration.capability_evidence != "narrowed_metadata_only"
        || declaration.qualified
    {
        return Err(CodexTupleError::DeclarationMismatch);
    }
    Ok(())
}
