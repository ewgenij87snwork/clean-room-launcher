use crate::core::budget::{BudgetInput, BudgetLimits, enforce_budgets};
use crate::core::inventory::SourceRecord;
use crate::core::manifest::{Compilation, Manifest, build_manifest};
use crate::core::publish::{CapabilityDir, verify_current};
use crate::core::render::{ArtifactSet, render};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Included,
    Deferred,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub subject: String,
    pub disposition: Disposition,
    pub reason_code: String,
}

impl Decision {
    pub fn new(subject: &str, disposition: Disposition, reason_code: &str) -> Self {
        Self {
            subject: subject.to_owned(),
            disposition,
            reason_code: reason_code.to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub source_count: u64,
    pub total_bytes: u64,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompileReport {
    pub artifacts: ArtifactSet,
    pub manifest: Manifest,
    pub decision: Decision,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VerifyReport {
    pub manifest: Manifest,
    pub decision: Decision,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InspectReport {
    pub decision: Decision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFacts {
    provider_found: bool,
    isolation_supported: bool,
    ambient_home_detected: bool,
}

impl RuntimeFacts {
    pub fn new(
        provider_found: bool,
        isolation_supported: bool,
        ambient_home_detected: bool,
    ) -> Self {
        Self {
            provider_found,
            isolation_supported,
            ambient_home_detected,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DoctorReport {
    pub ready: bool,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OperationError {
    pub code: &'static str,
    pub decision: Decision,
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.code, self.decision.reason_code)
    }
}

impl std::error::Error for OperationError {}

pub struct CoreService;

impl CoreService {
    pub fn audit(sources: &[SourceRecord]) -> AuditReport {
        AuditReport {
            source_count: sources.len() as u64,
            total_bytes: sources.iter().map(|source| source.byte_len).sum(),
            decisions: sources
                .iter()
                .map(|source| {
                    Decision::new(
                        &source.logical_path,
                        Disposition::Included,
                        "ADMITTED_SOURCE",
                    )
                })
                .collect(),
        }
    }

    pub fn compile(
        input: BudgetInput,
        limits: BudgetLimits,
        inputs: Vec<String>,
    ) -> Result<CompileReport, OperationError> {
        let budgeted = enforce_budgets(input, limits).map_err(|error| {
            let message = error.to_string();
            let reason = stable_reason(&message);
            operation_error("COMPILE_BUDGET_EXCEEDED", "compilation", reason)
        })?;
        let artifacts = render(&budgeted).map_err(|error| {
            operation_error(
                "COMPILE_RENDER_REFUSED",
                "compilation",
                stable_reason(&error.to_string()),
            )
        })?;
        let manifest =
            build_manifest(&Compilation::new(inputs, artifacts.clone())).map_err(|error| {
                operation_error(
                    "COMPILE_MANIFEST_REFUSED",
                    "compilation",
                    stable_reason(&error.to_string()),
                )
            })?;
        Ok(CompileReport {
            artifacts,
            manifest,
            decision: Decision::new("compilation", Disposition::Included, "COMPILE_READY"),
        })
    }

    pub fn verify(root: &CapabilityDir) -> Result<VerifyReport, OperationError> {
        let manifest = verify_current(root).map_err(|error| {
            let message = error.to_string();
            let reason = stable_reason(&message);
            let code = if reason == "DIGEST_MISMATCH" {
                "VERIFY_DIGEST_MISMATCH"
            } else {
                "VERIFY_REFUSED"
            };
            operation_error(code, "current-generation", reason)
        })?;
        Ok(VerifyReport {
            manifest,
            decision: Decision::new("current-generation", Disposition::Included, "VERIFIED"),
        })
    }

    pub fn inspect(decisions: &[Decision], subject: &str) -> Result<InspectReport, OperationError> {
        decisions
            .iter()
            .find(|decision| decision.subject == subject)
            .cloned()
            .map(|decision| InspectReport { decision })
            .ok_or_else(|| operation_error("INSPECT_NOT_FOUND", subject, "DECISION_NOT_FOUND"))
    }

    pub fn doctor(facts: &RuntimeFacts) -> DoctorReport {
        let decisions = vec![
            fact_decision("provider", facts.provider_found, "PROVIDER_FOUND"),
            fact_decision(
                "isolation",
                facts.isolation_supported,
                "ISOLATION_SUPPORTED",
            ),
            fact_decision(
                "ambient-home",
                !facts.ambient_home_detected,
                "AMBIENT_HOME_ABSENT",
            ),
        ];
        DoctorReport {
            ready: decisions
                .iter()
                .all(|decision| decision.disposition == Disposition::Included),
            decisions,
        }
    }
}

fn fact_decision(subject: &str, accepted: bool, success_reason: &str) -> Decision {
    if accepted {
        Decision::new(subject, Disposition::Included, success_reason)
    } else {
        Decision::new(subject, Disposition::Refused, "REQUIRED_FACT_FALSE")
    }
}

fn operation_error(code: &'static str, subject: &str, reason_code: &str) -> OperationError {
    OperationError {
        code,
        decision: Decision::new(subject, Disposition::Refused, reason_code),
    }
}

fn stable_reason(message: &str) -> &str {
    message.split([':', '=']).next().unwrap_or("UNKNOWN")
}

#[cfg(test)]
#[path = "../../tests/core/operations.rs"]
mod tests;
