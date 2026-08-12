use taskseal::{
    adapters::{
        codex::identity::{CodexTupleError, bind_resolved_tuple, resolve_installed_tuple},
        identity::ProviderIdentity,
    },
    contracts::adapter::AdapterDeclaration,
};

fn declaration() -> AdapterDeclaration {
    AdapterDeclaration {
        provider_id: "codex".into(),
        executable: "codex".into(),
        version_range: ">=0.147.0".into(),
        context_target: "provider_native_context".into(),
        collision_policy: "deny".into(),
        capability_evidence: "narrowed_metadata_only".into(),
        qualified: false,
    }
}

fn installed(version: (u64, u64, u64)) -> ProviderIdentity {
    ProviderIdentity {
        provider_id: "codex".into(),
        real_executable: "codex".into(),
        artifact_digest: "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37".into(),
        version,
        os: "macos".into(),
        arch: "aarch64".into(),
        interpreter: None,
    }
}

#[test]
fn binds_only_the_installed_codex_macos_arm64_tuple() {
    let tuple = bind_resolved_tuple(&declaration(), &installed((0, 147, 0))).unwrap();
    assert_eq!(tuple.provider_id, "codex");
    assert_eq!(tuple.version, (0, 147, 0));
    assert_eq!(tuple.os, "macos");
    assert_eq!(tuple.arch, "aarch64");
}

#[test]
fn refuses_the_next_codex_version_without_a_new_tuple_receipt() {
    assert_eq!(
        bind_resolved_tuple(&declaration(), &installed((0, 148, 0))).unwrap_err(),
        CodexTupleError::UnqualifiedTuple
    );
}

#[test]
fn resolver_entrypoint_refuses_before_a_missing_command_can_be_bound() {
    assert!(matches!(
        resolve_installed_tuple(&declaration(), std::path::Path::new("missing-codex")),
        Err(CodexTupleError::Identity(_))
    ));
}

#[test]
fn binder_refuses_an_altered_approved_declaration() {
    let mut altered = declaration();
    altered.version_range = ">=0.148.0".into();
    assert_eq!(
        bind_resolved_tuple(&altered, &installed((0, 147, 0))).unwrap_err(),
        CodexTupleError::DeclarationMismatch
    );
}

#[test]
fn resolver_rejects_an_altered_declaration_before_touching_the_command() {
    let mut altered = declaration();
    altered.version_range = ">=0.148.0".into();
    assert_eq!(
        resolve_installed_tuple(&altered, std::path::Path::new("missing-codex")).unwrap_err(),
        CodexTupleError::DeclarationMismatch
    );
}
