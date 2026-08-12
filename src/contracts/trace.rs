use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRef {
    pub id: String,
    pub plan: String,
    pub task: String,
    pub acceptance: String,
    pub evidence: String,
    pub disposition: String,
}

pub fn load_control_map(path: &str) -> Result<Vec<ControlRef>, &'static str> {
    let text = std::fs::read_to_string(path).map_err(|_| "READ_MAP")?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 { continue; }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 10 { return Err("FIELD_COUNT"); }
        let owner = fields[3].trim();
        let first_task = owner.split(';').next().unwrap_or("");
        let (plan, task) = first_task.split_once('-').unwrap_or(("", ""));
        rows.push(ControlRef { id: fields[0].into(), plan: plan.into(), task: task.into(), acceptance: fields[5].into(), evidence: fields[6].into(), disposition: fields[1].into() });
    }
    Ok(rows)
}

pub fn validate_control_map(rows: &[ControlRef]) -> Result<(), &'static str> {
    let mut ids = HashSet::new();
    for row in rows {
        if row.id.is_empty() || row.acceptance.is_empty() || row.evidence.is_empty() { return Err("MISSING_TRACE_FIELD"); }
        if !ids.insert(&row.id) { return Err("DUPLICATE_CONTROL"); }
        if row.disposition == "ADOPT" && row.plan.is_empty() && !row.evidence.starts_with("authority:") { return Err("UNREVIEWED_OWNER"); }
        if row.evidence.contains("stale") || row.acceptance.contains("CONFLICT") { return Err("STALE_OR_CONFLICTING"); }
    }
    if rows.len() != 445 { return Err("CONTROL_COUNT"); }
    Ok(())
}
