use crate::core::manifest::{Compilation, build_manifest};
use crate::core::publish::{
    PublicationTransition, publish, publish_with_interruption, verify_current,
};
use crate::core::render::ArtifactSet;
use cap_std::{ambient_authority, fs::Dir};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn root() -> (std::path::PathBuf, Dir) {
    let path = std::env::temp_dir().join(format!(
        "taskseal-p03-publish-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    let dir = Dir::open_ambient_dir(&path, ambient_authority()).unwrap();
    (path, dir)
}

fn compilation(content: &[u8]) -> Compilation {
    let artifacts = ArtifactSet::from([("context.md".to_owned(), content.to_vec())]);
    Compilation::new(vec!["l0:base".to_owned()], artifacts)
}

#[test]
fn publishes_one_complete_immutable_generation_and_verifies_bytes() {
    let (path, root) = root();
    let compilation = compilation(b"complete\n");
    let manifest = build_manifest(&compilation).unwrap();
    let published = publish(&root, compilation.artifacts(), &manifest).unwrap();

    assert_eq!(published.digest, manifest.digest);
    assert_eq!(verify_current(&root).unwrap(), manifest);
    assert_eq!(
        root.read(format!(
            ".clroom/out/generations/{}/context.md",
            manifest.digest
        ))
        .unwrap(),
        b"complete\n"
    );
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn pointer_switches_between_complete_generations_and_tamper_refuses() {
    let (path, root) = root();
    let old = compilation(b"old\n");
    let old_manifest = build_manifest(&old).unwrap();
    publish(&root, old.artifacts(), &old_manifest).unwrap();
    let new = compilation(b"new\n");
    let new_manifest = build_manifest(&new).unwrap();
    publish(&root, new.artifacts(), &new_manifest).unwrap();
    assert_eq!(verify_current(&root).unwrap(), new_manifest);

    root.write(
        format!(".clroom/out/generations/{}/context.md", new_manifest.digest),
        b"tampered\n",
    )
    .unwrap();
    assert!(
        verify_current(&root)
            .unwrap_err()
            .to_string()
            .contains("DIGEST_MISMATCH")
    );
    assert!(root.is_file(format!(
        ".clroom/out/generations/{}/manifest.json",
        old_manifest.digest
    )));
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn refuses_unsafe_artifact_path_and_corrupt_pointer_without_writing_through() {
    let (path, root) = root();
    let artifacts = ArtifactSet::from([("../escape".to_owned(), b"x".to_vec())]);
    let compilation = Compilation::new(vec![], artifacts);
    let manifest = build_manifest(&compilation).unwrap();
    assert!(
        publish(&root, compilation.artifacts(), &manifest)
            .unwrap_err()
            .to_string()
            .contains("UNSAFE_ARTIFACT_PATH")
    );
    assert!(!path.join("escape").exists());

    root.create_dir_all(".clroom/out").unwrap();
    root.write(".clroom/out/current.json", b"not-json").unwrap();
    assert!(
        verify_current(&root)
            .unwrap_err()
            .to_string()
            .contains("POINTER_CORRUPT")
    );
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn every_interruption_exposes_only_old_or_new_complete_generation() {
    let transitions = [
        PublicationTransition::StagingCreated,
        PublicationTransition::ArtifactsFlushed,
        PublicationTransition::ManifestFlushed,
        PublicationTransition::StagingVerified,
        PublicationTransition::GenerationCommitted,
        PublicationTransition::PointerFlushed,
        PublicationTransition::PointerCommitted,
    ];
    for transition in transitions {
        let (path, root) = root();
        let old = compilation(b"old complete\n");
        let old_manifest = build_manifest(&old).unwrap();
        publish(&root, old.artifacts(), &old_manifest).unwrap();
        let new = compilation(b"new complete\n");
        let new_manifest = build_manifest(&new).unwrap();
        assert!(
            publish_with_interruption(&root, new.artifacts(), &new_manifest, transition).is_err()
        );
        let visible = verify_current(&root).unwrap();
        assert!(visible == old_manifest || visible == new_manifest);
        drop(root);
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn refuses_orphan_staging_and_adjacent_hash_pointer_attack() {
    let (path, root) = root();
    let value = compilation(b"value\n");
    let manifest = build_manifest(&value).unwrap();
    root.create_dir_all(format!(
        ".clroom/out/generations/.staging-{}",
        manifest.digest
    ))
    .unwrap();
    assert!(
        publish(&root, value.artifacts(), &manifest)
            .unwrap_err()
            .to_string()
            .contains("ORPHAN_STAGING_REFUSED")
    );

    let adjacent = "a".repeat(64);
    root.create_dir_all(format!(".clroom/out/generations/{adjacent}"))
        .unwrap();
    root.write(
        format!(".clroom/out/generations/{adjacent}/manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    root.write(
        ".clroom/out/current.json",
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "current.v1",
            "digest": adjacent,
            "manifest": format!("generations/{}/manifest.json", "a".repeat(64))
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        verify_current(&root)
            .unwrap_err()
            .to_string()
            .contains("DIGEST_MISMATCH")
    );
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[cfg(unix)]
#[test]
fn refuses_replaced_source_symlink_inside_an_existing_generation() {
    use std::os::unix::fs::symlink;

    let (path, root) = root();
    let value = compilation(b"trusted\n");
    let manifest = build_manifest(&value).unwrap();
    publish(&root, value.artifacts(), &manifest).unwrap();
    let output = path.join(format!(
        ".clroom/out/generations/{}/context.md",
        manifest.digest
    ));
    fs::remove_file(&output).unwrap();
    let replacement = path.join("replacement.txt");
    fs::write(&replacement, b"trusted\n").unwrap();
    symlink(&replacement, &output).unwrap();
    assert!(
        verify_current(&root)
            .unwrap_err()
            .to_string()
            .contains("OUTPUT_BOUNDARY_REFUSED")
    );
    drop(root);
    fs::remove_dir_all(path).unwrap();
}
