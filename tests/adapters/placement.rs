use cap_std::{ambient_authority, fs::Dir};
use std::{fs, sync::atomic::{AtomicU64, Ordering}};
use taskseal::{
    adapters::placement::{place_context, PlacementOutcome},
    contracts::adapter::AdapterDeclaration,
    core::{manifest::{Compilation, build_manifest}, publish::publish, render::ArtifactSet},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn project() -> (std::path::PathBuf, Dir, taskseal::core::manifest::Manifest) {
    let path = std::env::temp_dir().join(format!("taskseal-p06-placement-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    fs::create_dir(&path).unwrap();
    let root = Dir::open_ambient_dir(&path, ambient_authority()).unwrap();
    let compilation = Compilation::new(vec!["l0".into()], ArtifactSet::from([("context.md".into(), b"safe\n".to_vec())]));
    let manifest = build_manifest(&compilation).unwrap();
    publish(&root, compilation.artifacts(), &manifest).unwrap();
    (path, root, manifest)
}

fn declaration() -> AdapterDeclaration {
    AdapterDeclaration { provider_id: "fixture".into(), executable: "fixture".into(), version_range: ">=0.0.0".into(), context_target: "provider_native_context".into(), collision_policy: "deny".into(), capability_evidence: "unqualified".into(), qualified: false }
}

#[test]
fn stages_verified_context_then_only_accepts_owned_identical_generation() {
    let (path, root, manifest) = project();
    let first = place_context(&root, &manifest, &declaration()).unwrap();
    assert_eq!(first.outcome, PlacementOutcome::Created);
    assert!(first.target.starts_with(".taskseal/runtime/generations/"));
    assert_eq!(root.read(format!("{}/context.md", first.target)).unwrap(), b"safe\n");
    let again = place_context(&root, &manifest, &declaration()).unwrap();
    assert_eq!(again.outcome, PlacementOutcome::OwnedIdentical);
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn refuses_unowned_preexisting_target_without_overwrite() {
    let (path, root, manifest) = project();
    root.create_dir_all(".taskseal").unwrap();
    root.write(".taskseal/runtime", b"user-owned\n").unwrap();
    assert!(place_context(&root, &manifest, &declaration()).unwrap_err().to_string().contains("PLACEMENT"));
    assert_eq!(root.read(".taskseal/runtime").unwrap(), b"user-owned\n");
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn refuses_owned_generation_when_context_is_replaced_by_symlink_or_extra_file() {
    use std::os::unix::fs::symlink;
    let (path, root, manifest) = project();
    let placed = place_context(&root, &manifest, &declaration()).unwrap();
    let absolute = path.join(&placed.target);
    fs::remove_file(absolute.join("context.md")).unwrap();
    symlink("/dev/null", absolute.join("context.md")).unwrap();
    assert!(place_context(&root, &manifest, &declaration()).unwrap_err().to_string().contains("PLACEMENT"));
    fs::remove_file(absolute.join("context.md")).unwrap();
    fs::write(absolute.join("context.md"), b"safe\n").unwrap();
    fs::write(absolute.join("extra"), b"x").unwrap();
    assert!(place_context(&root, &manifest, &declaration()).unwrap_err().to_string().contains("PLACEMENT"));
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn refuses_symlinked_runtime_ancestor_and_missing_ownership_receipt() {
    use std::os::unix::fs::symlink;
    let (path, root, manifest) = project();
    root.create_dir_all(".taskseal").unwrap();
    let outside = path.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, path.join(".taskseal/runtime")).unwrap();
    assert!(place_context(&root, &manifest, &declaration()).unwrap_err().to_string().contains("PLACEMENT"));
    fs::remove_file(path.join(".taskseal/runtime")).unwrap();
    let placed = place_context(&root, &manifest, &declaration()).unwrap();
    fs::remove_file(path.join(&placed.target).join("placement.json")).unwrap();
    assert!(place_context(&root, &manifest, &declaration()).unwrap_err().to_string().contains("PLACEMENT"));
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn refuses_stale_generated_context_before_staging() {
    let (path, root, manifest) = project();
    root.write(
        format!(".taskseal/out/generations/{}/context.md", manifest.digest),
        b"tampered generated context",
    )
    .unwrap();

    assert!(place_context(&root, &manifest, &declaration())
        .unwrap_err()
        .to_string()
        .contains("PLACEMENT"));

    drop(root);
    fs::remove_dir_all(path).unwrap();
}
