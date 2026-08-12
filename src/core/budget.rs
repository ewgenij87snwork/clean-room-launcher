use std::fmt;

#[derive(Debug, Clone)]
pub struct BudgetInput {
    bytes: Vec<u8>,
    layers: Option<[Vec<u8>; 3]>,
    records: u64,
    protected: bool,
}
impl BudgetInput {
    pub fn new(content: impl AsRef<[u8]>, records: u64, protected: bool) -> Self {
        Self {
            bytes: content.as_ref().to_vec(),
            layers: None,
            records,
            protected,
        }
    }

    pub fn from_layers(
        l0: impl AsRef<[u8]>,
        l2: impl AsRef<[u8]>,
        l3: impl AsRef<[u8]>,
        records: u64,
        protected: bool,
    ) -> Self {
        let layers = [
            l0.as_ref().to_vec(),
            l2.as_ref().to_vec(),
            l3.as_ref().to_vec(),
        ];
        let bytes = layers.iter().flatten().copied().collect();
        Self {
            bytes,
            layers: Some(layers),
            records,
            protected,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BudgetLimits {
    bytes: u64,
    records: u64,
    tokens: u64,
}
impl BudgetLimits {
    pub fn new(bytes: u64, records: u64, tokens: u64) -> Self {
        Self {
            bytes,
            records,
            tokens,
        }
    }
    pub fn strictest(limits: impl IntoIterator<Item = Self>) -> Self {
        limits
            .into_iter()
            .reduce(|left, right| Self {
                bytes: left.bytes.min(right.bytes),
                records: left.records.min(right.records),
                tokens: left.tokens.min(right.tokens),
            })
            .unwrap_or(Self::new(u64::MAX, u64::MAX, u64::MAX))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BudgetMeasurement {
    pub bytes: u64,
    pub records: u64,
    pub token_upper_bound: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BudgetedContext {
    pub bytes: Vec<u8>,
    pub measured: BudgetMeasurement,
    pub(crate) layers: Option<[Vec<u8>; 3]>,
}

#[derive(Debug)]
pub struct BudgetError(String);
impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for BudgetError {}

pub fn enforce_budgets(
    input: BudgetInput,
    limits: BudgetLimits,
) -> Result<BudgetedContext, BudgetError> {
    let measured = BudgetMeasurement {
        bytes: input.bytes.len() as u64,
        records: input.records,
        token_upper_bound: input.bytes.len() as u64,
    };
    for (dimension, value, limit) in [
        ("bytes", measured.bytes, limits.bytes),
        ("records", measured.records, limits.records),
        ("tokens", measured.token_upper_bound, limits.tokens),
    ] {
        if value > limit {
            let code = if input.protected {
                "PROTECTED_BUDGET_EXCEEDED"
            } else {
                "BUDGET_EXCEEDED"
            };
            return Err(BudgetError(format!(
                "{code}:dimension={dimension}:measured={value}:limit={limit}:algorithm=utf8-byte-upper-bound/v1"
            )));
        }
    }
    Ok(BudgetedContext {
        bytes: input.bytes,
        measured,
        layers: input.layers,
    })
}

#[cfg(test)]
#[path = "../../tests/core/budget.rs"]
mod tests;
