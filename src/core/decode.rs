use crate::contracts::adapter::{AdapterDeclaration, parse_declaration};
use crate::contracts::schema::{SchemaKind, validate};
use crate::core::inventory::SourceRecord;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

const DEFAULT_MAX_RECORD_BYTES: u64 = 1_048_576;

#[derive(Debug)]
pub struct ContractSet {
    max_record_bytes: u64,
}

impl ContractSet {
    pub fn standard() -> Self {
        Self {
            max_record_bytes: DEFAULT_MAX_RECORD_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum DecodedDocument {
    Json {
        logical_path: String,
        value: Value,
    },
    Adapter {
        logical_path: String,
        value: AdapterDeclaration,
    },
}

#[derive(Debug)]
pub struct DecodedInputs {
    pub documents: Vec<DecodedDocument>,
}

#[derive(Debug)]
pub struct DecodeError(String);

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DecodeError {}

pub fn decode(
    records: &[SourceRecord],
    contracts: &ContractSet,
) -> Result<DecodedInputs, DecodeError> {
    let mut documents = Vec::with_capacity(records.len());
    let mut ids = BTreeSet::new();

    for record in records {
        if record.byte_len > contracts.max_record_bytes {
            return Err(error(
                "INPUT_TOO_LARGE",
                &record.logical_path,
                "limit=1048576",
            ));
        }
        let text = std::str::from_utf8(record.content())
            .map_err(|_| error("INVALID_UTF8", &record.logical_path, "input is not UTF-8"))?;

        if record.logical_path.ends_with(".adapter.toml") {
            let value = parse_declaration(text)
                .map_err(|code| error(code, &record.logical_path, "adapter declaration"))?;
            admit_id(&mut ids, &value.provider_id, &record.logical_path)?;
            documents.push(DecodedDocument::Adapter {
                logical_path: record.logical_path.clone(),
                value,
            });
            continue;
        }

        let kind = schema_kind(&record.logical_path).ok_or_else(|| {
            error(
                "UNKNOWN_CONTRACT_KIND",
                &record.logical_path,
                "no standard contract suffix",
            )
        })?;
        let value = validate(kind, record.content()).map_err(|source| {
            let message = source.to_string();
            let code = message.split(':').next().unwrap_or("SCHEMA_INVALID");
            error(code, &record.logical_path, &message)
        })?;
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            admit_id(&mut ids, id, &record.logical_path)?;
        }
        documents.push(DecodedDocument::Json {
            logical_path: record.logical_path.clone(),
            value,
        });
    }

    Ok(DecodedInputs { documents })
}

fn schema_kind(path: &str) -> Option<SchemaKind> {
    if path.ends_with(".l2.json") {
        Some(SchemaKind::L2)
    } else if path.ends_with(".task-packet.json") {
        Some(SchemaKind::TaskPacketV2)
    } else if path.ends_with(".catalog.json") {
        Some(SchemaKind::Catalog)
    } else if path.ends_with(".manifest.json") {
        Some(SchemaKind::Manifest)
    } else {
        None
    }
}

fn admit_id(ids: &mut BTreeSet<String>, id: &str, path: &str) -> Result<(), DecodeError> {
    if ids.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(error("DUPLICATE_ID", path, id))
    }
}

fn error(code: &str, path: &str, detail: &str) -> DecodeError {
    DecodeError(format!("{code}:{path}:{detail}"))
}

#[cfg(test)]
#[path = "../../tests/core/decode.rs"]
mod tests;
