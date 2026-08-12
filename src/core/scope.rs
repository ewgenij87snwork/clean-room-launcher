use crate::core::decode::{DecodedDocument, DecodedInputs};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug)]
pub struct Target {
    roots: BTreeSet<String>,
    parents: Vec<(String, String)>,
}

impl Target {
    pub fn new<I, S, P, C, R>(roots: I, parents: P) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        P: IntoIterator<Item = (C, R)>,
        C: Into<String>,
        R: Into<String>,
    {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
            parents: parents
                .into_iter()
                .map(|(child, parent)| (child.into(), parent.into()))
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedScope {
    pub ids: Vec<String>,
}

#[derive(Debug)]
pub struct ScopeError(String);

impl fmt::Display for ScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScopeError {}

pub fn resolve_scopes(
    inputs: &DecodedInputs,
    target: &Target,
) -> Result<ResolvedScope, ScopeError> {
    let available: BTreeSet<String> = inputs
        .documents
        .iter()
        .filter_map(|document| match document {
            DecodedDocument::Json { value, .. } => value
                .get("id")
                .and_then(|id| id.as_str())
                .map(str::to_owned),
            DecodedDocument::Adapter { .. } => None,
        })
        .collect();
    let mut parent_by_child = BTreeMap::new();
    for (child, parent) in &target.parents {
        if parent_by_child
            .insert(child.clone(), parent.clone())
            .is_some()
        {
            return Err(ScopeError(format!("AMBIGUOUS_PARENT:{child}")));
        }
    }

    let mut resolved = Vec::new();
    let mut complete = BTreeSet::new();
    for root in &target.roots {
        visit(
            root,
            &available,
            &parent_by_child,
            &mut BTreeSet::new(),
            &mut complete,
            &mut resolved,
        )?;
    }
    Ok(ResolvedScope { ids: resolved })
}

fn visit(
    id: &str,
    available: &BTreeSet<String>,
    parents: &BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
    complete: &mut BTreeSet<String>,
    output: &mut Vec<String>,
) -> Result<(), ScopeError> {
    if complete.contains(id) {
        return Ok(());
    }
    if !available.contains(id) {
        return Err(ScopeError(format!("MISSING_SCOPE:{id}")));
    }
    if !visiting.insert(id.to_owned()) {
        return Err(ScopeError(format!("SCOPE_CYCLE:{id}")));
    }
    if let Some(parent) = parents.get(id) {
        visit(parent, available, parents, visiting, complete, output)?;
    }
    visiting.remove(id);
    complete.insert(id.to_owned());
    output.push(id.to_owned());
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/core/scope.rs"]
mod tests;
