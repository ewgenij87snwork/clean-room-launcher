use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
use taskseal::{
    adapters::{
        environment::ProviderNativePreauthenticatedSession,
        identity::{resolve_identity, revalidate_identity},
    },
    contracts::adapter::AdapterDeclaration,
};
fn declaration() -> AdapterDeclaration {
    AdapterDeclaration {
        provider_id: "codex".into(),
        executable: "codex".into(),
        version_range: ">=0.9.0".into(),
        context_target: "provider_native_context".into(),
        collision_policy: "deny".into(),
        capability_evidence: "narrowed_metadata_only".into(),
        qualified: false,
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn executable(version: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "taskseal-v5-identity-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let path = root.join("codex");
    fs::write(&path, format!("#!/bin/sh\nprintf 'codex {version}\\n'\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[test]
fn identity_binds_exact_numeric_tuple() {
    let identity = resolve_identity(
        ProviderNativePreauthenticatedSession::Available,
        &declaration(),
        &executable("0.10.0"),
    )
    .unwrap();
    assert_eq!(identity.provider_id, "codex");
    assert_eq!(identity.version, (0, 10, 0));
    assert_eq!(identity.os, std::env::consts::OS);
    assert_eq!(identity.arch, std::env::consts::ARCH);
    assert!(identity.interpreter.is_some());
}
#[test]
fn identity_refuses_out_of_range_and_post_check_replacement() {
    let path = executable("0.8.0");
    assert_eq!(
        resolve_identity(
            ProviderNativePreauthenticatedSession::Available,
            &declaration(),
            &path,
        )
        .unwrap_err()
        .to_string(),
        "VERSION_OUT_OF_RANGE"
    );
    let path = executable("0.10.0");
    let identity = resolve_identity(
        ProviderNativePreauthenticatedSession::Available,
        &declaration(),
        &path,
    )
    .unwrap();
    fs::write(&path, "#!/bin/sh\nprintf 'codex 9.9.9\\n'\n").unwrap();
    assert_eq!(
        revalidate_identity(&identity).unwrap_err().to_string(),
        "EXECUTABLE_REPLACED"
    );
}

#[test]
fn provider_identity_refuses_opaque_session_states_before_process_birth() {
    for (index, (session, expected)) in [
        (
            ProviderNativePreauthenticatedSession::Unavailable,
            "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE",
        ),
        (
            ProviderNativePreauthenticatedSession::Ambiguous,
            "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let root = std::env::temp_dir().join(format!(
            "taskseal-v5-identity-preauth-{}-{index}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let capture = root.join("provider-born");
        let command = root.join("codex");
        fs::write(
            &command,
            format!(
                "#!/bin/sh\n: > '{}'\nprintf 'codex 0.10.0\\n'\n",
                capture.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            resolve_identity(session, &declaration(), &command)
                .unwrap_err()
                .to_string(),
            expected
        );
        assert!(!capture.exists(), "provider identity process was born");
    }
}
