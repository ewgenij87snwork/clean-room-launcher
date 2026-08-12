use serde_json::{Map, Value};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct CanonicalError(&'static str);

impl Display for CanonicalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result { formatter.write_str(self.0) }
}
impl std::error::Error for CanonicalError {}

#[derive(Clone, Copy)]
pub enum MergeOperation { Replace, AppendUnique, DenyUnion, MapOverride, OrderedChain, NoInherit }

pub fn canonicalize(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let normalized = normalize(value)?;
    serde_json::to_vec(&normalized).map_err(|_| CanonicalError("CANONICAL_SERIALIZE"))
}

pub fn merge(left: &Value, right: &Value, operation: MergeOperation) -> Result<Value, CanonicalError> {
    let left_map = left.as_object().ok_or(CanonicalError("MERGE_OBJECT_REQUIRED"))?;
    let right_map = right.as_object().ok_or(CanonicalError("MERGE_OBJECT_REQUIRED"))?;
    match operation {
        MergeOperation::Replace => Ok(right.clone()),
        MergeOperation::NoInherit => Err(CanonicalError("MERGE_NO_INHERIT")),
        MergeOperation::DenyUnion => {
            if left_map.keys().any(|key| right_map.contains_key(key)) { Err(CanonicalError("MERGE_COLLISION")) } else { Ok(right.clone()) }
        }
        MergeOperation::MapOverride => {
            let mut result = left_map.clone();
            result.extend(right_map.clone());
            Ok(Value::Object(result))
        }
        MergeOperation::AppendUnique => {
            let mut result = left_map.clone();
            for (key, value) in right_map {
                if let Some(existing) = result.get_mut(key) {
                    let existing_array = existing.as_array_mut().ok_or(CanonicalError("MERGE_ARRAY_REQUIRED"))?;
                    let incoming = value.as_array().ok_or(CanonicalError("MERGE_ARRAY_REQUIRED"))?;
                    for item in incoming { if !existing_array.contains(item) { existing_array.push(item.clone()); } }
                } else { result.insert(key.clone(), value.clone()); }
            }
            Ok(Value::Object(result))
        }
        MergeOperation::OrderedChain => {
            let mut result = left_map.clone();
            for (key, value) in right_map { if result.contains_key(key) { return Err(CanonicalError("MERGE_COLLISION")); } result.insert(key.clone(), value.clone()); }
            Ok(Value::Object(result))
        }
    }
}

fn normalize(value: &Value) -> Result<Value, CanonicalError> {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys { sorted.insert(key.clone(), normalize(&map[key])?); }
            Ok(Value::Object(sorted))
        }
        Value::Array(items) => Ok(Value::Array(items.iter().map(normalize).collect::<Result<_, _>>()?)),
        Value::Number(number) if number.to_string() == "-0" => Err(CanonicalError("CANONICAL_NEGATIVE_ZERO")),
        other => Ok(other.clone()),
    }
}
