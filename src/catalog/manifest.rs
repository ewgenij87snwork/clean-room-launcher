use super::level_b::DecisionRecord;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonView {
    pub id: String,
    pub reason: &'static str,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    CensusMismatch,
    UnknownId,
}
#[derive(Debug, Clone)]
pub struct CatalogManifest {
    pub census: u64,
    pub decisions: Vec<DecisionRecord>,
    pub unavailable_sources: Vec<String>,
}
impl CatalogManifest {
    pub fn new(
        census: u64,
        decisions: Vec<DecisionRecord>,
        unavailable_sources: Vec<String>,
    ) -> Result<Self, ManifestError> {
        if decisions.len() + unavailable_sources.len() != census as usize {
            return Err(ManifestError::CensusMismatch);
        }
        Ok(Self {
            census,
            decisions,
            unavailable_sources,
        })
    }
    pub fn accounted(&self) -> u64 {
        (self.decisions.len() + self.unavailable_sources.len()) as u64
    }
    pub fn explain(&self, id: &str) -> Result<ReasonView, ManifestError> {
        self.view(id)
    }
    pub fn inspect(&self, id: &str) -> Result<ReasonView, ManifestError> {
        self.view(id)
    }
    fn view(&self, id: &str) -> Result<ReasonView, ManifestError> {
        let r = self
            .decisions
            .iter()
            .find(|r| r.id == id)
            .ok_or(ManifestError::UnknownId)?;
        Ok(ReasonView {
            id: r.id.clone(),
            reason: r.reason,
        })
    }
}
