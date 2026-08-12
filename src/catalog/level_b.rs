use super::protection::ProtectionSignals;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyDecision {
    LoadNow,
    LoadOnInvoke,
    Unavailable,
    Refuse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    pub id: String,
    pub decision: BodyDecision,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSignals {
    pub id: String,
    pub signals: ProtectionSignals,
}

impl SkillSignals {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            signals: ProtectionSignals::default(),
        }
    }
    pub fn protected(mut self) -> Self {
        self.signals.protected = true;
        self
    }
    pub fn task_required(mut self) -> Self {
        self.signals.task_required = true;
        self
    }
    pub fn pinned(mut self) -> Self {
        self.signals.pinned = true;
        self
    }
    pub fn project_required(mut self) -> Self {
        self.signals.project_required = true;
        self
    }
    pub fn unknown(mut self) -> Self {
        self.signals.unknown = true;
        self
    }
    pub fn stale(mut self) -> Self {
        self.signals.stale = true;
        self
    }
    pub fn changed(mut self) -> Self {
        self.signals.changed = true;
        self
    }
    pub fn relevance_excluded(mut self) -> Self {
        self.signals.relevance_excluded = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionError {
    RelevanceFilterNotAuthorized,
}

pub fn decide_bodies(items: &[SkillSignals]) -> Result<Vec<DecisionRecord>, DecisionError> {
    let mut output = Vec::with_capacity(items.len());
    for item in items {
        let s = &item.signals;
        if s.relevance_excluded {
            return Err(DecisionError::RelevanceFilterNotAuthorized);
        }
        let (decision, reason) = if s.protected {
            (BodyDecision::LoadNow, "PROTECTED")
        } else if s.project_required {
            (BodyDecision::LoadNow, "PROJECT_REQUIRED")
        } else if s.task_required {
            (BodyDecision::LoadNow, "TASK_REQUIRED")
        } else if s.pinned {
            (BodyDecision::LoadNow, "PINNED")
        } else if s.changed {
            (BodyDecision::Refuse, "CHANGED")
        } else if s.stale {
            (BodyDecision::Refuse, "STALE")
        } else if s.unknown {
            (BodyDecision::Refuse, "UNKNOWN")
        } else {
            (BodyDecision::LoadOnInvoke, "DEFERRED_NATIVE")
        };
        output.push(DecisionRecord {
            id: item.id.clone(),
            decision,
            reason,
        });
    }
    output.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    Ok(output)
}
