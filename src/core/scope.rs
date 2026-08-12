use crate::core::decode::{DecodedDocument, DecodedInputs};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug)]
pub struct Target {
    roots: BTreeSet<String>,
}

impl Target {
    pub fn new<I, S>(roots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedScope {
    pub ids: Vec<String>,
    pub layers: Vec<ScopeLayer>,
    pub targets: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeLayer {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub sections: Value,
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
    let mut parents = BTreeMap::<String, Vec<String>>::new();
    let mut sections = BTreeMap::<String, Value>::new();
    for document in &inputs.documents {
        if let DecodedDocument::Json { value, .. } = document {
            let Some(id) = value.get("scope_id").and_then(|value| value.as_str()) else {
                continue;
            };
            let parent_ids = value
                .get("parent_scope_ids")
                .and_then(|value| value.as_array())
                .ok_or_else(|| ScopeError(format!("MISSING_PARENT_LINKS:{id}")))?
                .iter()
                .map(|parent| {
                    parent
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| ScopeError(format!("INVALID_PARENT_LINK:{id}")))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if parents.insert(id.to_owned(), parent_ids).is_some() {
                return Err(ScopeError(format!("DUPLICATE_SCOPE:{id}")));
            }
            sections.insert(
                id.to_owned(),
                value.get("sections").cloned().unwrap_or(Value::Null),
            );
        }
    }
    let available: BTreeSet<String> = parents.keys().cloned().collect();
    let mut output = Vec::new();
    let mut complete = BTreeSet::new();
    for root in &target.roots {
        visit(
            root,
            &available,
            &parents,
            &mut BTreeSet::new(),
            &mut complete,
            &mut output,
        )?;
    }
    let layers = output
        .iter()
        .map(|id| ScopeLayer {
            id: id.clone(),
            parent_ids: parents.get(id).cloned().unwrap_or_default(),
            sections: sections.get(id).cloned().unwrap_or(Value::Null),
        })
        .collect();
    Ok(ResolvedScope {
        ids: output,
        layers,
        targets: target.roots.clone(),
    })
}

fn visit(
    id: &str,
    available: &BTreeSet<String>,
    parents: &BTreeMap<String, Vec<String>>,
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
    if let Some(parent_ids) = parents.get(id) {
        for parent in parent_ids {
            visit(parent, available, parents, visiting, complete, output)?;
        }
    }
    visiting.remove(id);
    complete.insert(id.to_owned());
    output.push(id.to_owned());
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/core/scope.rs"]
mod tests;
