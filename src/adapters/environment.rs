use super::identity::ProviderIdentity;
pub use super::session::ProviderNativePreauthenticatedSession;
use super::session::{PreauthenticatedSessionError, require_preauthenticated_session};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyMode {
    Deny,
}
pub struct EnvironmentPolicy {
    pub identity_digest: String,
    pub provider_id: String,
    pub version: (u64, u64, u64),
    pub os: String,
    pub arch: String,
    pub proxy_mode: ProxyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchEnvironment {
    ProviderNativePreauthenticatedSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentError {
    PolicyIdentityMismatch,
    ProviderNativePreauthenticatedSessionUnavailable,
    ProviderNativePreauthenticatedSessionAmbiguous,
}
impl fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PolicyIdentityMismatch => "POLICY_IDENTITY_MISMATCH",
            Self::ProviderNativePreauthenticatedSessionUnavailable => {
                "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE"
            }
            Self::ProviderNativePreauthenticatedSessionAmbiguous => {
                "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS"
            }
        })
    }
}
impl std::error::Error for EnvironmentError {}

pub fn build_environment(
    session: ProviderNativePreauthenticatedSession,
    identity: &ProviderIdentity,
    policy: &EnvironmentPolicy,
) -> Result<LaunchEnvironment, EnvironmentError> {
    if policy.identity_digest != identity.artifact_digest
        || policy.provider_id != identity.provider_id
        || policy.version != identity.version
        || policy.os != identity.os
        || policy.arch != identity.arch
    {
        return Err(EnvironmentError::PolicyIdentityMismatch);
    }

    match require_preauthenticated_session(session) {
        Ok(()) => Ok(LaunchEnvironment::ProviderNativePreauthenticatedSession),
        Err(PreauthenticatedSessionError::Unavailable) => {
            Err(EnvironmentError::ProviderNativePreauthenticatedSessionUnavailable)
        }
        Err(PreauthenticatedSessionError::Ambiguous) => {
            Err(EnvironmentError::ProviderNativePreauthenticatedSessionAmbiguous)
        }
    }
}
