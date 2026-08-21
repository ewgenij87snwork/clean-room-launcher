#[derive(Debug, PartialEq, Eq)]
pub struct AdapterDeclaration {
    pub provider_id: String,
    pub executable: String,
    pub version_range: String,
    pub context_target: String,
    pub collision_policy: String,
    pub capability_evidence: String,
    pub qualified: bool,
}

pub fn parse_declaration(text: &str) -> Result<AdapterDeclaration, &'static str> {
    let mut values = std::collections::BTreeMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, raw) = line.split_once('=').ok_or("TOML_SYNTAX")?;
        let key = key.trim();
        if values.contains_key(key) {
            return Err("DUPLICATE_KEY");
        }
        let value = raw.trim().trim_matches('"');
        if value.is_empty() {
            return Err("EMPTY_VALUE");
        }
        values.insert(key, value.to_string());
    }
    let allowed = [
        "provider_id",
        "executable",
        "version_range",
        "context_target",
        "collision_policy",
        "capability_evidence",
        "qualified",
    ];
    if values.keys().any(|key| !allowed.contains(key)) {
        return Err("UNKNOWN_KEY");
    }
    let required = |key: &str| values.get(key).cloned().ok_or("MISSING_KEY");
    let qualified = required("qualified")?;
    if qualified != "false" {
        return Err("MUTABLE_OR_OPTIMISTIC_QUALIFICATION");
    }
    let evidence = required("capability_evidence")?;
    if evidence.contains("PASS") {
        return Err("MUTABLE_PASS_STATE");
    }
    let collision = required("collision_policy")?;
    if !["deny", "replace", "append"].contains(&collision.as_str()) {
        return Err("INVALID_COLLISION_POLICY");
    }
    Ok(AdapterDeclaration {
        provider_id: required("provider_id")?,
        executable: required("executable")?,
        version_range: required("version_range")?,
        context_target: required("context_target")?,
        collision_policy: collision,
        capability_evidence: evidence,
        qualified: false,
    })
}
