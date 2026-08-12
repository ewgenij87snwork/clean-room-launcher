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
