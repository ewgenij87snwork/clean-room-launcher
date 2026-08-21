use super::state::AccessClass;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchProposal {
    project_digest: String,
    planned_writes: Vec<String>,
    protected_non_writes: Vec<String>,
    provider: String,
    argv_digest: String,
    qualification_digest: String,
    access_class: AccessClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchConsent(LaunchProposal);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentError(&'static str);

impl ConsentError {
    pub fn code(&self) -> &'static str {
        self.0
    }
}
impl fmt::Display for ConsentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ConsentError {}

impl LaunchProposal {
    pub fn new(
        project_digest: String,
        planned_writes: Vec<String>,
        protected_non_writes: Vec<String>,
        provider: String,
        argv_digest: String,
        qualification_digest: String,
        access_class: AccessClass,
    ) -> Result<Self, ConsentError> {
        if !valid_digest(&project_digest)
            || !valid_digest(&argv_digest)
            || !valid_digest(&qualification_digest)
            || !matches!(provider.as_str(), "codex" | "claude")
            || !valid_paths(&planned_writes)
            || !valid_paths(&protected_non_writes)
        {
            return Err(ConsentError("CONSENT_PROPOSAL_REFUSED"));
        }
        Ok(Self {
            project_digest,
            planned_writes,
            protected_non_writes,
            provider,
            argv_digest,
            qualification_digest,
            access_class,
        })
    }

    pub fn disclosure(&self) -> String {
        let access = match self.access_class {
            AccessClass::Standard => "STANDARD",
            AccessClass::FullAccess => "FULL ACCESS",
        };
        format!(
            "{access} · {} planned writes · {} protected non-writes",
            self.planned_writes.len(),
            self.protected_non_writes.len()
        )
    }
}

impl LaunchConsent {
    pub fn resolve(decision: Decision, proposal: &LaunchProposal) -> Result<Self, ConsentError> {
        match decision {
            Decision::Approve => Ok(Self(proposal.clone())),
            Decision::Cancel => Err(ConsentError("CONSENT_CANCELLED")),
        }
    }
    pub fn verify(&self, proposal: &LaunchProposal) -> Result<(), ConsentError> {
        if &self.0 == proposal {
            Ok(())
        } else {
            Err(ConsentError("CONSENT_STALE_REFUSED"))
        }
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_paths(paths: &[String]) -> bool {
    paths
        .windows(2)
        .all(|pair| pair[0].as_bytes() < pair[1].as_bytes())
        && paths.iter().all(|path| {
            !path.is_empty()
                && !path.starts_with('/')
                && !path.contains('\\')
                && !path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
        })
}
