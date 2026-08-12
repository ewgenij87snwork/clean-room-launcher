use crate::core::inventory::sha256_hex;
use crate::core::render::ArtifactSet;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Compilation {
    inputs: Vec<String>,
    artifacts: ArtifactSet,
}

impl Compilation {
    pub fn new(mut inputs: Vec<String>, artifacts: ArtifactSet) -> Self {
        inputs.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        inputs.dedup();
        Self { inputs, artifacts }
    }

    pub fn artifacts(&self) -> &ArtifactSet {
        &self.artifacts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub digest: String,
}

#[derive(Debug)]
pub struct ManifestError(&'static str);

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ManifestError {}

pub fn build_manifest(compilation: &Compilation) -> Result<Manifest, ManifestError> {
    let outputs = compilation.artifacts.keys().cloned().collect::<Vec<_>>();
    if outputs.is_empty() {
        return Err(ManifestError("EMPTY_ARTIFACT_SET"));
    }
    let digest = generation_digest(&compilation.inputs, &compilation.artifacts);
    Ok(Manifest {
        schema_version: "manifest.v1".to_owned(),
        inputs: compilation.inputs.clone(),
        outputs,
        digest,
    })
}

pub(crate) fn generation_digest(inputs: &[String], artifacts: &ArtifactSet) -> String {
    let mut framed = Vec::new();
    framed.extend_from_slice(b"taskseal-generation-v1\0");
    for input in inputs {
        frame(&mut framed, input.as_bytes());
    }
    framed.push(0xff);
    for (path, bytes) in artifacts {
        frame(&mut framed, path.as_bytes());
        frame(&mut framed, bytes);
    }
    sha256_hex(&framed)
}

fn frame(target: &mut Vec<u8>, bytes: &[u8]) {
    target.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    target.extend_from_slice(bytes);
}

#[cfg(test)]
#[path = "../../tests/core/manifest_publish.rs"]
mod tests;
