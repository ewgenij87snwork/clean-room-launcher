use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSeam {
    pub schema_version: String,
    pub plan: String,
    pub task: u32,
    pub result: String,
    pub subject_digests: Vec<(String, String)>,
}
