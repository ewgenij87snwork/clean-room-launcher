use crate::core::merge::MergedContext;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug)]
pub struct PolicyError(String);
impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for PolicyError {}

pub fn enforce_denies(context: &MergedContext, emitted: &[&str]) -> Result<(), PolicyError> {
    let emitted: BTreeSet<&str> = emitted.iter().copied().collect();
    for denied in &context.denies {
        if !emitted.contains(denied.as_str()) {
            return Err(PolicyError(format!("DENY_WEAKENED:{denied}")));
        }
    }
    Ok(())
}
