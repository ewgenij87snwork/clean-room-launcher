use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn signing_claims_require_platform_verifier_results() {
    let verifier = root().join("packaging/signing/verify.sh");
    assert!(verifier.is_file(), "signing verifier is missing");
    let temp = std::env::temp_dir().join(format!("p07-signing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let tools = temp.join("tools");
    fs::create_dir_all(&tools).unwrap();
    let artifact = temp.join("taskseal");
    fs::write(&artifact, b"immutable-artifact").unwrap();

    executable(&tools.join("codesign"), r#"#!/bin/sh
case "$TASKSEAL_SIGNING_SCENARIO:$1" in
  unsigned:--verify) echo 'code object is not signed at all' >&2; exit 1 ;;
  adhoc:--verify|wrong:--verify|valid:--verify|notarized:--verify) exit 0 ;;
  tampered:--verify) echo 'invalid signature' >&2; exit 1 ;;
  adhoc:-dv) echo 'Signature=adhoc' >&2; exit 0 ;;
  wrong:-dv) echo 'Authority=Wrong Identity' >&2; exit 0 ;;
  valid:-dv|notarized:-dv) echo 'Authority=TaskSeal Test Identity' >&2; exit 0 ;;
esac
exit 2
"#);
    executable(&tools.join("spctl"), r#"#!/bin/sh
[ "$TASKSEAL_SIGNING_SCENARIO" = notarized ] || exit 1
echo 'source=Notarized Developer ID' >&2
"#);

    let verify = |scenario: &str, claim: &str, identity: Option<&str>| {
        let mut command = Command::new(&verifier);
        command.current_dir(root()).env("TASKSEAL_SIGNING_FIXTURE", "1").env("TASKSEAL_SIGNING_SCENARIO", scenario).args(["--artifact", artifact.to_str().unwrap(), "--platform", "macos", "--claim", claim, "--tool-root", tools.to_str().unwrap()]);
        if let Some(value) = identity { command.args(["--identity", value]); }
        command.output().unwrap()
    };

    assert!(verify("unsigned", "unsigned", None).status.success());
    assert!(!verify("adhoc", "signed", Some("TaskSeal Test Identity")).status.success());
    let adhoc_preview = verify("adhoc", "unsigned", None);
    assert!(adhoc_preview.status.success(), "{}", String::from_utf8_lossy(&adhoc_preview.stderr));
    assert_eq!(String::from_utf8(adhoc_preview.stdout).unwrap(), "P07_SIGNING_VERIFY_PASS state=unsigned qualification=NOT_QUALIFIED evidence=fixture signature=adhoc\n");
    assert!(!verify("wrong", "signed", Some("TaskSeal Test Identity")).status.success());
    assert!(!verify("tampered", "signed", Some("TaskSeal Test Identity")).status.success());
    assert!(verify("valid", "signed", Some("TaskSeal Test Identity")).status.success());
    assert!(!verify("valid", "signed+notarized", Some("TaskSeal Test Identity")).status.success());
    let accepted = verify("notarized", "signed+notarized", Some("TaskSeal Test Identity"));
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert_eq!(String::from_utf8(accepted.stdout).unwrap(), "P07_SIGNING_VERIFY_PASS state=signed+notarized qualification=NOT_QUALIFIED evidence=fixture\n");

    fs::remove_dir_all(&temp).unwrap();
}
