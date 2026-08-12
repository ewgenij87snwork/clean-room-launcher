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
    pub metadata_digest: String,
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
        let (metadata, metadata_digest) = metadata_for(&files.metadata, &body_record)?;
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
            metadata_digest,
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
        .map_err(|_| InventoryError::SourceRefused)?;
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
            let (metadata, metadata_digest) = metadata_for(&files.metadata, &body_record)?;
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
                metadata_digest,
            });
        }
    }

    records.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(records)
}

fn metadata_for(
    sidecar: &Option<SourceRecord>,
    body: &SourceRecord,
) -> Result<(SkillMetadata, String), InventoryError> {
    if let Some(record) = sidecar {
        let metadata = serde_json::from_slice(record.content())
            .map_err(|_| InventoryError::MalformedMetadata)?;
        return Ok((metadata, record.sha256.clone()));
    }
    let text =
        std::str::from_utf8(body.content()).map_err(|_| InventoryError::MalformedMetadata)?;
    let frontmatter = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|parts| parts.0))
        .ok_or(InventoryError::MissingMetadata)?;
    let field = |name: &str| {
        frontmatter.lines().find_map(|line| {
            line.strip_prefix(name)
                .and_then(|value| value.strip_prefix(':'))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
    };
    let name = field("name").ok_or(InventoryError::MalformedMetadata)?;
    let description = field("description").ok_or(InventoryError::MalformedMetadata)?;
    Ok((
        SkillMetadata {
            name,
            capability: description.clone(),
            trigger_summary: description,
        },
        sha256_hex(frontmatter.as_bytes()),
    ))
}

fn split_logical_path(path: &str) -> (String, &str) {
    match path.rsplit_once('/') {
        Some((directory, file_name)) => (directory.to_owned(), file_name),
        None => (String::new(), path),
    }
}
