use super::level_a::LevelAEntry;
#[derive(Debug)]
pub struct CapabilityTruth {
    pub native_progressive_disclosure: bool,
    pub observed_digest: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    UnsupportedNativeSeam,
    DigestMismatch,
}
#[derive(Debug)]
pub struct NativeEntry {
    pub name: String,
    pub body_digest: String,
}
#[derive(Debug)]
pub struct NativeProjection {
    pub entries: Vec<NativeEntry>,
    pub startup_bytes: Vec<u8>,
}
pub fn project_native(
    level_a: &[LevelAEntry],
    truth: CapabilityTruth,
) -> Result<NativeProjection, ProjectionError> {
    if !truth.native_progressive_disclosure {
        return Err(ProjectionError::UnsupportedNativeSeam);
    }
    if let Some(observed) = truth.observed_digest
        && level_a.iter().any(|e| e.body_digest != observed)
    {
        return Err(ProjectionError::DigestMismatch);
    }
    let entries = level_a
        .iter()
        .map(|e| NativeEntry {
            name: e.name.clone(),
            body_digest: e.body_digest.clone(),
        })
        .collect::<Vec<_>>();
    let startup_bytes = serde_json::to_vec(level_a).expect("serializable");
    Ok(NativeProjection {
        entries,
        startup_bytes,
    })
}
