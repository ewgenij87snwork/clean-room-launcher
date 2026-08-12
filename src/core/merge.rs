use crate::core::scope::{ResolvedScope, ScopeLayer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenancedValue {
    pub value: String,
    pub source_scope: String,
}

#[derive(Debug, Default)]
pub struct MergedContext {
    pub replace: BTreeMap<String, ProvenancedValue>,
    pub append_unique: BTreeSet<String>,
    pub denies: BTreeSet<String>,
    pub map_override: BTreeMap<String, ProvenancedValue>,
    pub ordered_chain: Vec<String>,
    pub no_inherit: BTreeMap<String, ProvenancedValue>,
}

#[derive(Debug)]
pub struct MergeError(String);
impl fmt::Display for MergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for MergeError {}

pub fn merge(scopes: &[ResolvedScope]) -> Result<MergedContext, MergeError> {
    let mut result = MergedContext::default();
    let mut ancestry = BTreeMap::<String, BTreeSet<String>>::new();
    for scope in scopes {
        for layer in &scope.layers {
            let mut ancestors = BTreeSet::new();
            for parent in &layer.parent_ids {
                ancestors.insert(parent.clone());
                if let Some(inherited) = ancestry.get(parent) {
                    ancestors.extend(inherited.iter().cloned());
                }
            }
            ancestry.insert(layer.id.clone(), ancestors);
            apply_layer(
                &mut result,
                layer,
                &ancestry,
                scope.targets.contains(&layer.id),
            )?;
        }
    }
    Ok(result)
}

fn apply_layer(
    result: &mut MergedContext,
    layer: &ScopeLayer,
    ancestry: &BTreeMap<String, BTreeSet<String>>,
    is_target: bool,
) -> Result<(), MergeError> {
    let sections = layer
        .sections
        .as_object()
        .ok_or_else(|| MergeError(format!("INVALID_SECTIONS:{}", layer.id)))?;
    for (operation, value) in sections {
        match operation.as_str() {
            "replace" => apply_map(&mut result.replace, value, layer, ancestry, true)?,
            "append_unique" => extend_set(&mut result.append_unique, value, layer)?,
            "deny_union" => extend_set(&mut result.denies, value, layer)?,
            "map_override" => apply_map(&mut result.map_override, value, layer, ancestry, false)?,
            "ordered_chain" => result.ordered_chain.extend(string_list(value, layer)?),
            "no_inherit" if is_target => {
                apply_map(&mut result.no_inherit, value, layer, ancestry, false)?
            }
            "no_inherit" => {}
            other => {
                return Err(MergeError(format!(
                    "UNKNOWN_MERGE_OPERATION:{}:{other}",
                    layer.id
                )));
            }
        }
    }
    Ok(())
}

fn apply_map(
    target: &mut BTreeMap<String, ProvenancedValue>,
    value: &serde_json::Value,
    layer: &ScopeLayer,
    ancestry: &BTreeMap<String, BTreeSet<String>>,
    refuse_sibling_collision: bool,
) -> Result<(), MergeError> {
    let map = value
        .as_object()
        .ok_or_else(|| MergeError(format!("MERGE_OBJECT_REQUIRED:{}", layer.id)))?;
    for (key, value) in map {
        let value = value
            .as_str()
            .ok_or_else(|| MergeError(format!("MERGE_STRING_REQUIRED:{}:{key}", layer.id)))?;
        if let Some(existing) = target.get(key) {
            let is_ancestor = ancestry
                .get(&layer.id)
                .is_some_and(|parents| parents.contains(&existing.source_scope));
            if refuse_sibling_collision && !is_ancestor && existing.value != value {
                return Err(MergeError(format!(
                    "MERGE_COLLISION:{key}:{}:{}",
                    existing.source_scope, layer.id
                )));
            }
        }
        target.insert(
            key.clone(),
            ProvenancedValue {
                value: value.to_owned(),
                source_scope: layer.id.clone(),
            },
        );
    }
    Ok(())
}

fn extend_set(
    target: &mut BTreeSet<String>,
    value: &serde_json::Value,
    layer: &ScopeLayer,
) -> Result<(), MergeError> {
    target.extend(string_list(value, layer)?);
    Ok(())
}

fn string_list(value: &serde_json::Value, layer: &ScopeLayer) -> Result<Vec<String>, MergeError> {
    value
        .as_array()
        .ok_or_else(|| MergeError(format!("MERGE_ARRAY_REQUIRED:{}", layer.id)))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| MergeError(format!("MERGE_STRING_REQUIRED:{}", layer.id)))
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/core/merge_policy.rs"]
mod tests;
