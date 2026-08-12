use crate::core::budget::BudgetedContext;
use std::collections::BTreeMap;
use std::fmt;

pub type ArtifactSet = BTreeMap<String, Vec<u8>>;

#[derive(Debug)]
pub struct RenderError(String);
impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}

pub fn render(input: &BudgetedContext) -> Result<ArtifactSet, RenderError> {
    let layers = input
        .layers
        .as_ref()
        .ok_or_else(|| RenderError("MISSING_RENDER_LAYER:L0/L2/L3".into()))?;
    let mut context = Vec::new();
    for (heading, bytes) in [
        (b"# L0\n".as_slice(), &layers[0]),
        (b"# L2\n".as_slice(), &layers[1]),
        (b"# L3\n".as_slice(), &layers[2]),
    ] {
        context.extend_from_slice(heading);
        context.extend_from_slice(trim_terminal_newlines(bytes));
        context.push(b'\n');
    }
    let mut artifacts = BTreeMap::new();
    artifacts.insert("context.md".to_owned(), context);
    Ok(artifacts)
}

fn trim_terminal_newlines(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && matches!(bytes[end - 1], b'\n' | b'\r') {
        end -= 1;
    }
    &bytes[..end]
}

#[cfg(test)]
#[path = "../../tests/core/render.rs"]
mod tests;
