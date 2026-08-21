use taskseal::adapters::{
    environment::{
        EnvironmentPolicy, LaunchEnvironment, ProviderNativePreauthenticatedSession, ProxyMode,
        build_environment,
    },
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
        proxy_mode: ProxyMode::Deny,
    }
}
#[test]
fn typed_policy_admits_only_an_opaque_preauthenticated_session_state() {
    // Break caught: a raw credential-like value is copied into the provider launch environment.
    let identity = identity();
    let environment = build_environment(
        ProviderNativePreauthenticatedSession::Available,
        &identity,
        &policy(&identity),
    )
    .unwrap();
    assert_eq!(
        environment,
        LaunchEnvironment::ProviderNativePreauthenticatedSession
    );

    let mut wrong = policy(&identity);
    wrong.os = "other".into();
    assert_eq!(
        build_environment(
            ProviderNativePreauthenticatedSession::Available,
            &identity,
            &wrong,
        )
        .unwrap_err()
        .to_string(),
        "POLICY_IDENTITY_MISMATCH"
    );
}
#[test]
fn unavailable_and_ambiguous_session_states_refuse_without_fallback() {
    // Break caught: missing or conflicting session evidence falls through to auth/billing input.
    let identity = identity();
    assert_eq!(
        build_environment(
            ProviderNativePreauthenticatedSession::Unavailable,
            &identity,
            &policy(&identity),
        )
        .unwrap_err()
        .to_string(),
        "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE"
    );
    assert_eq!(
        build_environment(
            ProviderNativePreauthenticatedSession::Ambiguous,
            &identity,
            &policy(&identity),
        )
        .unwrap_err()
        .to_string(),
        "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS"
    );
}
