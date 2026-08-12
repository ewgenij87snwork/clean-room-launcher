use super::inventory::SkillRecord;
use serde::Serialize;

const MAX_SUMMARY_BYTES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LevelAEntry {
    pub id: String,
    pub name: String,
    pub capability: String,
    pub trigger_summary: String,
    pub source_id: String,
    pub body_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelAError {
    MissingSummary,
    SummaryTooLong,
    UnsafeSummary,
}

pub fn build_level_a(records: &[SkillRecord]) -> Result<Vec<LevelAEntry>, LevelAError> {
    let mut entries = Vec::with_capacity(records.len());
    for record in records {
        validate_summary(&record.trigger_summary)?;
        entries.push(LevelAEntry {
            id: record.id.clone(),
            name: record.name.clone(),
            capability: record.capability.clone(),
            trigger_summary: record.trigger_summary.clone(),
            source_id: record.source_id.clone(),
            body_digest: record.body_digest.clone(),
        });
    }
    entries.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(entries)
}

fn validate_summary(summary: &str) -> Result<(), LevelAError> {
    if summary.is_empty() {
        return Err(LevelAError::MissingSummary);
    }
    if summary.len() > MAX_SUMMARY_BYTES {
        return Err(LevelAError::SummaryTooLong);
    }
    if summary.chars().any(char::is_control) {
        return Err(LevelAError::UnsafeSummary);
    }
    Ok(())
}
