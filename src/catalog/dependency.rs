use super::level_b::{BodyDecision, DecisionRecord};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    edges: BTreeMap<String, Vec<String>>,
}
impl DependencyGraph {
    pub fn new<const N: usize>(items: [(&str, Vec<&str>); N]) -> Self {
        let edges = items
            .into_iter()
            .map(|(id, deps)| (id.to_owned(), deps.into_iter().map(str::to_owned).collect()))
            .collect();
        Self { edges }
    }
    pub fn from_ids(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            edges: ids.into_iter().map(|id| (id, Vec::new())).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedDecision {
    pub id: String,
    pub decision: BodyDecision,
    pub reason_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    MissingNode(String),
    Cycle(Vec<String>),
    RequiredDependencyRefused(String),
}

pub fn close_dependencies(
    decisions: &[DecisionRecord],
    graph: &DependencyGraph,
) -> Result<Vec<ClosedDecision>, DependencyError> {
    let known: BTreeSet<_> = decisions.iter().map(|r| r.id.as_str()).collect();
    let mut output: BTreeMap<String, ClosedDecision> = decisions
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                ClosedDecision {
                    id: r.id.clone(),
                    decision: r.decision,
                    reason_chain: vec![r.id.clone()],
                },
            )
        })
        .collect();
    for root in decisions
        .iter()
        .filter(|r| r.decision == BodyDecision::LoadNow)
    {
        let mut path = vec![root.id.clone()];
        visit(&root.id, &root.id, graph, &known, &mut path, &mut output)?;
    }
    Ok(output.into_values().collect())
}

fn visit(
    current: &str,
    root: &str,
    graph: &DependencyGraph,
    known: &BTreeSet<&str>,
    path: &mut Vec<String>,
    output: &mut BTreeMap<String, ClosedDecision>,
) -> Result<(), DependencyError> {
    let deps = graph
        .edges
        .get(current)
        .ok_or_else(|| DependencyError::MissingNode(current.to_owned()))?;
    for dep in deps {
        if !known.contains(dep.as_str()) {
            return Err(DependencyError::MissingNode(dep.clone()));
        }
        if let Some(index) = path.iter().position(|id| id == dep) {
            let mut cycle = path[index..].to_vec();
            cycle.push(dep.clone());
            return Err(DependencyError::Cycle(cycle));
        }
        path.push(dep.clone());
        let record = output.get_mut(dep).expect("known decision");
        if record.decision == BodyDecision::Refuse {
            return Err(DependencyError::RequiredDependencyRefused(dep.clone()));
        }
        record.decision = BodyDecision::LoadNow;
        record.reason_chain = path.clone();
        visit(dep, root, graph, known, path, output)?;
        path.pop();
    }
    let _ = root;
    Ok(())
}
