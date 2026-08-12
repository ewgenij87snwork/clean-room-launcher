use super::identity::{AdapterError, ProviderIdentity};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentKey {
    QualifiedAuth,
}
impl EnvironmentKey {
    fn name(self) -> &'static str {
        "QUALIFIED_AUTH"
    }
}
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
    pub allowed: Vec<EnvironmentKey>,
    pub required_auth: Vec<EnvironmentKey>,
    pub proxy_mode: ProxyMode,
}
#[derive(Debug)]
pub struct LaunchEnvironment(BTreeMap<String, String>);
impl LaunchEnvironment {
    pub fn redacted_keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}
pub fn build_environment(
    parent: &BTreeMap<String, String>,
    identity: &ProviderIdentity,
    policy: &EnvironmentPolicy,
) -> Result<LaunchEnvironment, AdapterError> {
    if policy.identity_digest != identity.artifact_digest
        || policy.provider_id != identity.provider_id
        || policy.version != identity.version
        || policy.os != identity.os
        || policy.arch != identity.arch
    {
        return Err(AdapterError::PolicyIdentityMismatch);
    }
    if policy
        .required_auth
        .iter()
        .any(|key| !policy.allowed.contains(key))
    {
        return Err(AdapterError::InvalidPolicy);
    }
    let values: BTreeMap<String, String> = policy
        .allowed
        .iter()
        .filter_map(|key| {
            parent
                .get(key.name())
                .map(|value| (key.name().to_owned(), value.clone()))
        })
        .collect();
    if !policy.required_auth.is_empty()
        && !policy
            .required_auth
            .iter()
            .any(|key| values.contains_key(key.name()))
    {
        return Err(AdapterError::RequiredAuthMissing);
    }
    Ok(LaunchEnvironment(values))
}
