#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProtectionSignals {
    pub protected: bool,
    pub task_required: bool,
    pub pinned: bool,
    pub project_required: bool,
    pub unknown: bool,
    pub stale: bool,
    pub changed: bool,
    pub relevance_excluded: bool,
}

pub fn apply_baseline_protection(signals: &mut [SkillSignals], records: &[SkillRecord]) {
    for signal in signals {
        if records.iter().any(|record| {
            record.id == signal.id
                && PROTECTED_PROCESS_CATEGORIES.contains(&record.capability.as_str())
        }) {
            signal.signals.protected = true;
        }
    }
}
use super::inventory::SkillRecord;
use super::level_b::SkillSignals;

pub const PROTECTED_PROCESS_CATEGORIES: [&str; 6] = [
    "verification",
    "debugging",
    "security-privacy",
    "planning",
    "evidence",
    "recovery",
];
