use crate::core::manifest::{Compilation, Manifest, build_manifest};
use crate::core::render::ArtifactSet;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Write};
use std::path::{Component, Path};

pub type CapabilityDir = Dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedGeneration {
    pub digest: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationTransition {
    StagingCreated,
    ArtifactsFlushed,
    ManifestFlushed,
    StagingVerified,
    GenerationCommitted,
    PointerFlushed,
    PointerCommitted,
}

#[derive(Debug)]
pub struct PublishError {
    code: &'static str,
    source: Option<io::Error>,
}

impl PublishError {
    fn code(code: &'static str) -> Self {
        Self { code, source: None }
    }

    fn io(code: &'static str, source: io::Error) -> Self {
        Self {
            code,
            source: Some(source),
        }
    }
}

impl fmt::Display for PublishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(formatter, "{}:{source}", self.code),
            None => formatter.write_str(self.code),
        }
    }
}

impl std::error::Error for PublishError {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentPointer {
    schema_version: String,
    digest: String,
    manifest: String,
}

pub fn publish(
    root: &CapabilityDir,
    artifacts: &ArtifactSet,
    manifest: &Manifest,
) -> Result<PublishedGeneration, PublishError> {
    publish_impl(root, artifacts, manifest, &mut |_| Ok(()))
}

fn publish_impl(
    root: &CapabilityDir,
    artifacts: &ArtifactSet,
    manifest: &Manifest,
    observer: &mut dyn FnMut(PublicationTransition) -> Result<(), PublishError>,
) -> Result<PublishedGeneration, PublishError> {
    validate_manifest_matches(artifacts, manifest)?;
    for path in artifacts.keys() {
        validate_relative_path(path)?;
    }
    ensure_output_dirs(root)?;

    let generation = format!(".clroom/out/generations/{}", manifest.digest);
    if root.is_dir(&generation) {
        verify_generation(root, &manifest.digest)?;
    } else {
        let staging = format!(".clroom/out/generations/.staging-{}", manifest.digest);
        if root.exists(&staging) {
            return Err(PublishError::code("ORPHAN_STAGING_REFUSED"));
        }
        root.create_dir(&staging)
            .map_err(|error| PublishError::io("STAGING_CREATE_FAILED", error))?;
        observer(PublicationTransition::StagingCreated)?;
        write_generation(root, &staging, artifacts, manifest, observer)?;
        root.rename(&staging, root, &generation)
            .map_err(|error| PublishError::io("GENERATION_COMMIT_FAILED", error))?;
        sync_dir(root, ".clroom/out/generations")?;
        observer(PublicationTransition::GenerationCommitted)?;
    }

    let manifest_path = format!("generations/{}/manifest.json", manifest.digest);
    let pointer = CurrentPointer {
        schema_version: "current.v1".to_owned(),
        digest: manifest.digest.clone(),
        manifest: manifest_path.clone(),
    };
    let pointer_bytes =
        serde_json::to_vec(&pointer).map_err(|_| PublishError::code("POINTER_SERIALIZE_FAILED"))?;
    write_new_synced(root, ".clroom/out/.current.staging", &pointer_bytes)?;
    observer(PublicationTransition::PointerFlushed)?;
    root.rename(
        ".clroom/out/.current.staging",
        root,
        ".clroom/out/current.json",
    )
    .map_err(|error| PublishError::io("POINTER_COMMIT_FAILED", error))?;
    sync_dir(root, ".clroom/out")?;
    observer(PublicationTransition::PointerCommitted)?;

    Ok(PublishedGeneration {
        digest: manifest.digest.clone(),
        manifest_path,
    })
}

#[cfg(test)]
pub(crate) fn publish_with_interruption(
    root: &CapabilityDir,
    artifacts: &ArtifactSet,
    manifest: &Manifest,
    stop_after: PublicationTransition,
) -> Result<PublishedGeneration, PublishError> {
    publish_impl(root, artifacts, manifest, &mut |transition| {
        if transition == stop_after {
            Err(PublishError::code("INJECTED_INTERRUPTION"))
        } else {
            Ok(())
        }
    })
}

pub fn verify_current(root: &CapabilityDir) -> Result<Manifest, PublishError> {
    require_regular_file(root, ".clroom/out/current.json", "POINTER_BOUNDARY_REFUSED")?;
    let bytes = root
        .read(".clroom/out/current.json")
        .map_err(|error| PublishError::io("POINTER_READ_FAILED", error))?;
    let pointer: CurrentPointer =
        serde_json::from_slice(&bytes).map_err(|_| PublishError::code("POINTER_CORRUPT"))?;
    let expected = format!("generations/{}/manifest.json", pointer.digest);
    if pointer.schema_version != "current.v1" || pointer.manifest != expected {
        return Err(PublishError::code("POINTER_CORRUPT"));
    }
    verify_generation(root, &pointer.digest)
}

fn verify_generation(root: &CapabilityDir, digest: &str) -> Result<Manifest, PublishError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PublishError::code("POINTER_CORRUPT"));
    }
    let base = format!(".clroom/out/generations/{digest}");
    let generation_metadata = root
        .symlink_metadata(&base)
        .map_err(|error| PublishError::io("GENERATION_INSPECT_FAILED", error))?;
    if generation_metadata.is_symlink() || !generation_metadata.is_dir() {
        return Err(PublishError::code("GENERATION_BOUNDARY_REFUSED"));
    }
    require_regular_file(
        root,
        &format!("{base}/manifest.json"),
        "MANIFEST_BOUNDARY_REFUSED",
    )?;
    let manifest_bytes = root
        .read(format!("{base}/manifest.json"))
        .map_err(|error| PublishError::io("MANIFEST_READ_FAILED", error))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| PublishError::code("MANIFEST_CORRUPT"))?;
    if manifest.digest != digest {
        return Err(PublishError::code("DIGEST_MISMATCH"));
    }
    let mut artifacts = ArtifactSet::new();
    for output in &manifest.outputs {
        validate_relative_path(output)?;
        require_regular_file(root, &format!("{base}/{output}"), "OUTPUT_BOUNDARY_REFUSED")?;
        let bytes = root
            .read(format!("{base}/{output}"))
            .map_err(|error| PublishError::io("OUTPUT_READ_FAILED", error))?;
        artifacts.insert(output.clone(), bytes);
    }
    validate_manifest_matches(&artifacts, &manifest)?;
    Ok(manifest)
}

fn require_regular_file(
    root: &CapabilityDir,
    path: &str,
    code: &'static str,
) -> Result<(), PublishError> {
    let metadata = root
        .symlink_metadata(path)
        .map_err(|error| PublishError::io(code, error))?;
    if metadata.is_symlink() || !metadata.is_file() {
        return Err(PublishError::code(code));
    }
    Ok(())
}

fn validate_manifest_matches(
    artifacts: &ArtifactSet,
    manifest: &Manifest,
) -> Result<(), PublishError> {
    let rebuilt = build_manifest(&Compilation::new(
        manifest.inputs.clone(),
        artifacts.clone(),
    ))
    .map_err(|_| PublishError::code("MANIFEST_INVALID"))?;
    if &rebuilt != manifest {
        return Err(PublishError::code("DIGEST_MISMATCH"));
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), PublishError> {
    let parsed = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || parsed
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(PublishError::code("UNSAFE_ARTIFACT_PATH"));
    }
    Ok(())
}

fn ensure_output_dirs(root: &CapabilityDir) -> Result<(), PublishError> {
    let mut current = String::new();
    for part in [".clroom", "out", "generations"] {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        match root.symlink_metadata(&current) {
            Ok(metadata) if metadata.is_symlink() || !metadata.is_dir() => {
                return Err(PublishError::code("OUTPUT_BOUNDARY_REFUSED"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => root
                .create_dir(&current)
                .map_err(|error| PublishError::io("OUTPUT_DIR_CREATE_FAILED", error))?,
            Err(error) => return Err(PublishError::io("OUTPUT_DIR_INSPECT_FAILED", error)),
        }
    }
    Ok(())
}

fn write_generation(
    root: &CapabilityDir,
    staging: &str,
    artifacts: &ArtifactSet,
    manifest: &Manifest,
    observer: &mut dyn FnMut(PublicationTransition) -> Result<(), PublishError>,
) -> Result<(), PublishError> {
    for (path, bytes) in artifacts {
        let full = format!("{staging}/{path}");
        if let Some(parent) = Path::new(&full).parent() {
            root.create_dir_all(parent)
                .map_err(|error| PublishError::io("ARTIFACT_DIR_CREATE_FAILED", error))?;
        }
        write_new_synced(root, &full, bytes)?;
    }
    observer(PublicationTransition::ArtifactsFlushed)?;
    let manifest_bytes = serde_json::to_vec(manifest)
        .map_err(|_| PublishError::code("MANIFEST_SERIALIZE_FAILED"))?;
    write_new_synced(root, &format!("{staging}/manifest.json"), &manifest_bytes)?;
    observer(PublicationTransition::ManifestFlushed)?;
    verify_staging(root, staging, manifest)?;
    observer(PublicationTransition::StagingVerified)?;
    sync_dir(root, staging)
}

fn verify_staging(
    root: &CapabilityDir,
    staging: &str,
    manifest: &Manifest,
) -> Result<(), PublishError> {
    let mut artifacts = ArtifactSet::new();
    for output in &manifest.outputs {
        artifacts.insert(
            output.clone(),
            root.read(format!("{staging}/{output}"))
                .map_err(|error| PublishError::io("STAGING_VERIFY_FAILED", error))?,
        );
    }
    validate_manifest_matches(&artifacts, manifest)
}

fn write_new_synced(root: &CapabilityDir, path: &str, bytes: &[u8]) -> Result<(), PublishError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = root
        .open_with(path, &options)
        .map_err(|error| PublishError::io("EXCLUSIVE_WRITE_FAILED", error))?;
    file.write_all(bytes)
        .map_err(|error| PublishError::io("WRITE_FAILED", error))?;
    file.sync_all()
        .map_err(|error| PublishError::io("FSYNC_FAILED", error))
}

fn sync_dir(root: &CapabilityDir, path: &str) -> Result<(), PublishError> {
    root.open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| PublishError::io("DIRECTORY_FSYNC_FAILED", error))
}
