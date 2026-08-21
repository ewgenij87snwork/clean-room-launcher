use serde_json::Value;

pub struct ExecutionSubject<'a> {
    pub worktree: &'a str,
    pub branch: &'a str,
    pub head: &'a str,
}

pub fn validate_authority(
    schema_bytes: &[u8],
    authority_bytes: &[u8],
    expected: &ExecutionSubject<'_>,
) -> Result<(), String> {
    let schema: Value = serde_json::from_slice(schema_bytes).map_err(|error| error.to_string())?;
    let authority: Value =
        serde_json::from_slice(authority_bytes).map_err(|error| error.to_string())?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| error.to_string())?;
    let errors: Vec<String> = validator
        .iter_errors(&authority)
        .map(|error| error.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    for (field, expected_value) in [
        ("worktree_realpath", expected.worktree),
        ("branch", expected.branch),
        ("head", expected.head),
    ] {
        if authority.get(field).and_then(Value::as_str) != Some(expected_value) {
            return Err(format!("AUTHORITY_SUBJECT_MISMATCH:{field}"));
        }
    }
    Ok(())
}
