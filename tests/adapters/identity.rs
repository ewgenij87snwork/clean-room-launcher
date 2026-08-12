use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
use taskseal::{
    adapters::identity::{resolve_identity, revalidate_identity},
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
    let identity = resolve_identity(&declaration(), &executable("0.10.0")).unwrap();
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
        resolve_identity(&declaration(), &path)
            .unwrap_err()
            .to_string(),
        "VERSION_OUT_OF_RANGE"
    );
    let path = executable("0.10.0");
    let identity = resolve_identity(&declaration(), &path).unwrap();
    fs::write(&path, "#!/bin/sh\nprintf 'codex 9.9.9\\n'\n").unwrap();
    assert_eq!(
        revalidate_identity(&identity).unwrap_err().to_string(),
        "EXECUTABLE_REPLACED"
    );
}
