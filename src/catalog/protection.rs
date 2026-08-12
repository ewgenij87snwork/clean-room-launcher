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
