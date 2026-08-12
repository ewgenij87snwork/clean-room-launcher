use super::sources::SkillSource;
use crate::core::inventory::{AdmittedRoot, SourceRecord, inventory, sha256_hex};
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMetadata {
    name: String,
    capability: String,
    trigger_summary: String,
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
        let source_records = inventory(&[AdmittedRoot::new(&source.root, &source.id)])
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
            let metadata_record = files.metadata.ok_or(InventoryError::MissingMetadata)?;
            let body_record = files.body.ok_or(InventoryError::MissingBody)?;
            let metadata: SkillMetadata = serde_json::from_slice(metadata_record.content())
                .map_err(|_| InventoryError::MalformedMetadata)?;
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
                metadata_digest: metadata_record.sha256,
            });
        }
    }

    records.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(records)
}

fn split_logical_path(path: &str) -> (String, &str) {
    match path.rsplit_once('/') {
        Some((directory, file_name)) => (directory.to_owned(), file_name),
        None => (String::new(), path),
    }
}
