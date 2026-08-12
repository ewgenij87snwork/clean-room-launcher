use std::collections::BTreeMap;
use taskseal::adapters::{
    environment::{EnvironmentKey, EnvironmentPolicy, ProxyMode, build_environment},
    identity::ProviderIdentity,
};
fn identity() -> ProviderIdentity {
    ProviderIdentity {
        provider_id: "codex".into(),
        real_executable: "/qualified/codex".into(),
        artifact_digest: "a".repeat(64),
        version: (0, 147, 0),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        interpreter: None,
    }
}
fn policy(identity: &ProviderIdentity) -> EnvironmentPolicy {
    EnvironmentPolicy {
        identity_digest: identity.artifact_digest.clone(),
        provider_id: identity.provider_id.clone(),
        version: identity.version,
        os: identity.os.clone(),
        arch: identity.arch.clone(),
        allowed: vec![EnvironmentKey::QualifiedAuth],
        required_auth: vec![EnvironmentKey::QualifiedAuth],
        proxy_mode: ProxyMode::Deny,
    }
}
#[test]
fn typed_policy_admits_only_required_auth_and_binds_exact_identity() {
    let identity = identity();
    let parent = BTreeMap::from([
        ("QUALIFIED_AUTH".into(), "secret".into()),
        ("HOME".into(), "poison".into()),
        ("PATH".into(), "poison".into()),
        ("ALL_PROXY".into(), "poison".into()),
    ]);
    let environment = build_environment(&parent, &identity, &policy(&identity)).unwrap();
    assert_eq!(environment.redacted_keys(), vec!["QUALIFIED_AUTH"]);
    let mut wrong = policy(&identity);
    wrong.os = "other".into();
    assert_eq!(
        build_environment(&parent, &identity, &wrong)
            .unwrap_err()
            .to_string(),
        "POLICY_IDENTITY_MISMATCH"
    );
}
#[test]
fn typed_policy_refuses_missing_required_auth_and_invalid_requirement() {
    let identity = identity();
    assert_eq!(
        build_environment(&BTreeMap::new(), &identity, &policy(&identity))
            .unwrap_err()
            .to_string(),
        "REQUIRED_AUTH_MISSING"
    );
    let mut invalid = policy(&identity);
    invalid.allowed.clear();
    assert_eq!(
        build_environment(&BTreeMap::new(), &identity, &invalid)
            .unwrap_err()
            .to_string(),
        "INVALID_ENVIRONMENT_POLICY"
    );
}
