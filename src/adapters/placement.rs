use crate::{
    contracts::adapter::AdapterDeclaration,
    catalog::projection::declaration_digest,
    core::{inventory::sha256_hex, manifest::Manifest, publish::verify_current},
};
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use std::{fmt, io::Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementOutcome { Created, OwnedIdentical }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementReceipt {
    pub declaration_digest: String,
    pub manifest_digest: String,
    pub context_digest: String,
    pub target: String,
    pub outcome: PlacementOutcome,
}

#[derive(Debug)]
pub struct PlacementError(&'static str);
impl fmt::Display for PlacementError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.0) } }
impl std::error::Error for PlacementError {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredReceipt { schema_version: String, declaration_digest: String, manifest_digest: String, context_digest: String, outcome: String }

pub fn place_context(project: &Dir, manifest: &Manifest, declaration: &AdapterDeclaration) -> Result<PlacementReceipt, PlacementError> {
    let current = verify_current(project).map_err(|_| PlacementError("PLACEMENT_SOURCE_UNVERIFIED"))?;
    if current != *manifest { return Err(PlacementError("PLACEMENT_MANIFEST_MISMATCH")); }
    let context_path = format!(".taskseal/out/generations/{}/context.md", manifest.digest);
    let context = project.read(&context_path).map_err(|_| PlacementError("PLACEMENT_CONTEXT_UNAVAILABLE"))?;
    // The source is verified both before and after the byte read; any changed
    // generation refuses instead of staging an unbound snapshot.
    if verify_current(project).map_err(|_| PlacementError("PLACEMENT_SOURCE_REPLACED"))? != *manifest
        || project.read(&context_path).map_err(|_| PlacementError("PLACEMENT_CONTEXT_UNAVAILABLE"))? != context
    {
        return Err(PlacementError("PLACEMENT_SOURCE_REPLACED"));
    }
    let context_digest = sha256_hex(&context);
    let declaration_digest = declaration_digest(declaration);
    let target = format!(".taskseal/runtime/generations/{declaration_digest}/{}", manifest.digest);
    ensure_dir(project, ".taskseal")?;
    ensure_dir(project, ".taskseal/runtime")?;
    ensure_dir(project, ".taskseal/runtime/generations")?;
    ensure_dir(project, &format!(".taskseal/runtime/generations/{declaration_digest}"))?;
    match project.symlink_metadata(&target) {
        Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => return verify_owned(project, &target, &declaration_digest, manifest, &context_digest),
        Ok(_) => return Err(PlacementError("PLACEMENT_TARGET_UNOWNED")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(PlacementError("PLACEMENT_TARGET_INSPECT_FAILED")),
    }
    project.create_dir(&target).map_err(|_| PlacementError("PLACEMENT_CREATE_REFUSED"))?;
    write_new(project, &format!("{target}/context.md"), &context)?;
    let stored = StoredReceipt { schema_version: "taskseal-placement.v1".into(), declaration_digest: declaration_digest.clone(), manifest_digest: manifest.digest.clone(), context_digest: context_digest.clone(), outcome: "STAGED_NOT_APPLIED".into() };
    let bytes = serde_json::to_vec(&stored).map_err(|_| PlacementError("PLACEMENT_RECEIPT_SERIALIZE_FAILED"))?;
    write_new(project, &format!("{target}/placement.json"), &bytes)?;
    Ok(PlacementReceipt { declaration_digest, manifest_digest: manifest.digest.clone(), context_digest, target, outcome: PlacementOutcome::Created })
}

fn ensure_dir(project: &Dir, path: &str) -> Result<(), PlacementError> {
    match project.symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => Ok(()),
        Ok(_) => Err(PlacementError("PLACEMENT_BOUNDARY_REFUSED")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => project.create_dir(path).map_err(|_| PlacementError("PLACEMENT_DIRECTORY_CREATE_FAILED")),
        Err(_) => Err(PlacementError("PLACEMENT_BOUNDARY_REFUSED")),
    }
}

fn write_new(project: &Dir, path: &str, bytes: &[u8]) -> Result<(), PlacementError> {
    let mut options = OpenOptions::new(); options.write(true).create_new(true);
    let mut file = project.open_with(path, &options).map_err(|_| PlacementError("PLACEMENT_WRITE_REFUSED"))?;
    file.write_all(bytes).map_err(|_| PlacementError("PLACEMENT_WRITE_FAILED"))?;
    file.sync_all().map_err(|_| PlacementError("PLACEMENT_SYNC_FAILED"))
}

fn verify_owned(project: &Dir, target: &str, declaration_digest: &str, manifest: &Manifest, context_digest: &str) -> Result<PlacementReceipt, PlacementError> {
    let entries = project.read_dir(target).map_err(|_| PlacementError("PLACEMENT_OWNERSHIP_CORRUPT"))?
        .map(|entry| entry.map_err(|_| PlacementError("PLACEMENT_OWNERSHIP_CORRUPT")))
        .collect::<Result<Vec<_>, _>>()?;
    if entries.len() != 2 || entries.iter().any(|entry| !matches!(entry.file_name().to_str(), Some("context.md" | "placement.json"))) {
        return Err(PlacementError("PLACEMENT_OWNERSHIP_MISMATCH"));
    }
    let receipt_path = format!("{target}/placement.json");
    let context_path = format!("{target}/context.md");
    for path in [&receipt_path, &context_path] {
        let metadata = project.symlink_metadata(path).map_err(|_| PlacementError("PLACEMENT_OWNERSHIP_MISSING"))?;
        if metadata.is_symlink() || !metadata.is_file() { return Err(PlacementError("PLACEMENT_OWNERSHIP_MISMATCH")); }
    }
    let stored: StoredReceipt = serde_json::from_slice(&project.read(&receipt_path).map_err(|_| PlacementError("PLACEMENT_OWNERSHIP_MISSING"))?).map_err(|_| PlacementError("PLACEMENT_OWNERSHIP_CORRUPT"))?;
    let context = project.read(&context_path).map_err(|_| PlacementError("PLACEMENT_CONTEXT_MISSING"))?;
    if stored.schema_version != "taskseal-placement.v1" || stored.outcome != "STAGED_NOT_APPLIED" || stored.declaration_digest != declaration_digest || stored.manifest_digest != manifest.digest || stored.context_digest != context_digest || sha256_hex(&context) != *context_digest { return Err(PlacementError("PLACEMENT_OWNERSHIP_MISMATCH")); }
    Ok(PlacementReceipt { declaration_digest: declaration_digest.into(), manifest_digest: manifest.digest.clone(), context_digest: context_digest.into(), target: target.into(), outcome: PlacementOutcome::OwnedIdentical })
}
