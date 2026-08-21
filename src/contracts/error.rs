use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ErrorRecord {
    pub schema_version: String,
    pub code: String,
    pub cause: String,
    pub safe_action: String,
    pub details: Vec<String>,
}
