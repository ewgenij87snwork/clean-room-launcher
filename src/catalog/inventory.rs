use super::sources::SkillSource;
use crate::core::inventory::{SourceRecord, sha256_hex};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub capability: String,
    pub trigger_summary: String,
    pub source_id: String,
    pub body_path: String,
    pub body_digest: String,
    pub body_bytes: u64,
    pub metadata_digest: String,
    pub native_frontmatter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    SourceRefused,
    MalformedMetadata,
    MissingMetadata,
    MissingBody,
    DuplicateName(String),
    ChangedDuringRead,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMetadata {
    name: String,
    capability: String,
    trigger_summary: String,
}


#[cfg(test)]
pub(crate) fn inventory_skills_with_mutation(
    source: &SkillSource,
    observer: &mut dyn FnMut(&std::path::Path),
) -> Result<Vec<SkillRecord>, InventoryError> {
    let records = crate::core::inventory::inventory_capability_with_observer(
        source
            .root_capability
            .as_deref()
            .ok_or(InventoryError::SourceRefused)?,
        &source.id,
        observer,
    )
    .map_err(|error| match error {
        crate::core::inventory::CoreError::Refused {
            code: "PATH_RACE", ..
        } => InventoryError::ChangedDuringRead,
        _ => InventoryError::SourceRefused,
    })?;
    inventory_records(source, records)
}

#[cfg(test)]
fn inventory_records(
    source: &SkillSource,
    source_records: Vec<SourceRecord>,
) -> Result<Vec<SkillRecord>, InventoryError> {
    let mut groups: BTreeMap<String, SkillFiles> = BTreeMap::new();
    for record in source_records {
        let (directory, file_name) = split_logical_path(&record.logical_path);
        let files = groups.entry(directory).or_default();
        match file_name {
            "skill.json" => files.metadata = Some(record),
            "SKILL.md" => files.body = Some(record),
            _ => {}
        }
    }
    let mut records = Vec::new();
    let mut names = BTreeSet::new();
    for (directory, files) in groups {
        if files.metadata.is_none() && files.body.is_none() {
            continue;
        }
        let body_record = files.body.ok_or(InventoryError::MissingBody)?;
        let (metadata, metadata_digest, native_frontmatter) =
            metadata_for(&files.metadata, &body_record, &directory)?;
        if metadata.name.is_empty()
            || metadata.capability.is_empty()
            || metadata.trigger_summary.is_empty()
        {
            return Err(InventoryError::MalformedMetadata);
        }
        if !names.insert(metadata.name.clone()) {
            return Err(InventoryError::DuplicateName(metadata.name));
        }
        let id_digest =
            sha256_hex(format!("{}\0{}\0{}", source.id, directory, metadata.name).as_bytes());
        records.push(SkillRecord {
            id: format!("skill-{}", &id_digest[..16]),
            name: metadata.name,
            capability: metadata.capability,
            trigger_summary: metadata.trigger_summary,
            source_id: source.id.clone(),
            body_path: body_record.logical_path,
            body_digest: body_record.sha256,
            body_bytes: body_record.byte_len,
            metadata_digest,
            native_frontmatter,
        })
    }
    Ok(records)
}

#[derive(Default)]
struct SkillFiles {
    metadata: Option<SourceRecord>,
    body: Option<SourceRecord>,
}

pub fn inventory_skills(sources: &[SkillSource]) -> Result<Vec<SkillRecord>, InventoryError> {
    let mut records = Vec::new();
    let mut names = BTreeSet::new();

    for source in sources.iter().filter(|source| source.admitted) {
        let source_records = crate::core::inventory::inventory_capability(
            source
                .root_capability
                .as_deref()
                .ok_or(InventoryError::SourceRefused)?,
            &source.id,
        )
        .map_err(map_core_error)?;
        let mut groups: BTreeMap<String, SkillFiles> = BTreeMap::new();
        for record in source_records {
            let (directory, file_name) = split_logical_path(&record.logical_path);
            let files = groups.entry(directory).or_default();
            match file_name {
                "skill.json" => files.metadata = Some(record),
                "SKILL.md" => files.body = Some(record),
                _ => {}
            }
        }

        for (directory, files) in groups {
            if files.metadata.is_none() && files.body.is_none() {
                continue;
            }
            let body_record = files.body.ok_or(InventoryError::MissingBody)?;
            let (metadata, metadata_digest, native_frontmatter) =
                metadata_for(&files.metadata, &body_record, &directory)?;
            if metadata.name.is_empty()
                || metadata.capability.is_empty()
                || metadata.trigger_summary.is_empty()
            {
                return Err(InventoryError::MalformedMetadata);
            }
            if !names.insert(metadata.name.clone()) {
                return Err(InventoryError::DuplicateName(metadata.name));
            }
            let id_digest =
                sha256_hex(format!("{}\0{}\0{}", source.id, directory, metadata.name).as_bytes());
            records.push(SkillRecord {
                id: format!("skill-{}", &id_digest[..16]),
                name: metadata.name,
                capability: metadata.capability,
                trigger_summary: metadata.trigger_summary,
                source_id: source.id.clone(),
                body_path: body_record.logical_path,
                body_digest: body_record.sha256,
                body_bytes: body_record.byte_len,
                metadata_digest,
                native_frontmatter,
            });
        }
    }

    records.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(records)
}

fn map_core_error(error: crate::core::inventory::CoreError) -> InventoryError {
    match error {
        crate::core::inventory::CoreError::Refused {
            code: "PATH_RACE", ..
        } => InventoryError::ChangedDuringRead,
        _ => InventoryError::SourceRefused,
    }
}

fn metadata_for(
    sidecar: &Option<SourceRecord>,
    body: &SourceRecord,
    directory: &str,
) -> Result<(SkillMetadata, String, bool), InventoryError> {
    if let Some(record) = sidecar {
        let metadata = serde_json::from_slice(record.content())
            .map_err(|_| InventoryError::MalformedMetadata)?;
        return Ok((metadata, record.sha256.clone(), false));
    }
    let text =
        std::str::from_utf8(body.content()).map_err(|_| InventoryError::MalformedMetadata)?;
    let normalized = text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .replace("\r\n", "\n");
    let frontmatter = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|parts| parts.0))
        .ok_or(InventoryError::MissingMetadata)?;
    let parsed: std::collections::BTreeMap<String, serde_yaml::Value> =
        serde_yaml::from_str(frontmatter).map_err(|_| InventoryError::MalformedMetadata)?;
    let name = parsed
        .get("name")
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_owned)
        .ok_or(InventoryError::MalformedMetadata)?;
    let description = parsed
        .get("description")
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_owned)
        .ok_or(InventoryError::MalformedMetadata)?;
    let directory_name = directory.rsplit('/').next().unwrap_or(directory);
    if !valid_native_name(&name) || name != directory_name || description.chars().count() > 1024 {
        return Err(InventoryError::MalformedMetadata);
    }
    Ok((
        SkillMetadata {
            name,
            capability: description.clone(),
            trigger_summary: description.split_whitespace().collect::<Vec<_>>().join(" "),
        },
        sha256_hex(frontmatter.as_bytes()),
        true,
    ))
}

fn valid_native_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn split_logical_path(path: &str) -> (String, &str) {
    match path.rsplit_once('/') {
        Some((directory, file_name)) => (directory.to_owned(), file_name),
        None => (String::new(), path),
    }
}
