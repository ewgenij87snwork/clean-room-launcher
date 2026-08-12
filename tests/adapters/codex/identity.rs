use taskseal::adapters::codex::identity::{CodexTupleError, bind_installed_tuple};
use taskseal::adapters::identity::ProviderIdentity;

fn installed(version: (u64, u64, u64)) -> ProviderIdentity {
    ProviderIdentity {
        provider_id: "codex".into(),
        real_executable: "/private/var/empty/codex".into(),
        artifact_digest: "19c4f144c5226a9f17c58e6f0fa854843b0f77a6eb420f40e2745a12f10f5d37".into(),
        version,
        os: "macos".into(),
        arch: "aarch64".into(),
        interpreter: None,
    }
}

#[test]
fn binds_only_the_installed_codex_macos_arm64_tuple() {
    let tuple = bind_installed_tuple(&installed((0, 147, 0))).unwrap();
    assert_eq!(tuple.provider_id, "codex");
    assert_eq!(tuple.version, (0, 147, 0));
    assert_eq!(tuple.os, "macos");
    assert_eq!(tuple.arch, "aarch64");
}

#[test]
fn refuses_the_next_codex_version_without_a_new_tuple_receipt() {
    assert_eq!(
        bind_installed_tuple(&installed((0, 148, 0))).unwrap_err(),
        CodexTupleError::UnqualifiedTuple
    );
}
