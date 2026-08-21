use serde_json::Value;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy)]
pub enum SchemaKind {
    L2,
    TaskPacketV2,
    Catalog,
    Manifest,
}

#[derive(Debug)]
pub struct ContractError(String);

impl Display for ContractError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ContractError {}

pub fn validate(kind: SchemaKind, bytes: &[u8]) -> Result<Value, ContractError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| ContractError(format!("JSON_PARSE:{error}")))?;
    let schema = schema_for(kind);
    let validator = jsonschema::validator_for(&schema)
        .map_err(|error| ContractError(format!("SCHEMA_COMPILE:{error}")))?;
    let mut errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| {
            let path = error.instance_path().as_str();
            let path = if path.is_empty() { "/" } else { path };
            format!("{path}: {error}")
        })
        .collect();
    errors.sort();
    if errors.is_empty() {
        Ok(value)
    } else {
        Err(ContractError(format!(
            "SCHEMA_INVALID:{}",
            errors.join(" | ")
        )))
    }
}

fn schema_for(kind: SchemaKind) -> Value {
    let source = match kind {
        SchemaKind::L2 => include_str!("../../schemas/contracts/l2.schema.json"),
        SchemaKind::TaskPacketV2 => {
            include_str!("../../schemas/contracts/task-packet-v2.schema.json")
        }
        SchemaKind::Catalog => include_str!("../../schemas/contracts/catalog.schema.json"),
        SchemaKind::Manifest => include_str!("../../schemas/contracts/manifest.schema.json"),
    };
    serde_json::from_str(source).expect("checked-in contract schema must be valid JSON")
}
