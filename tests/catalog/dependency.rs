use super::dependency::{DependencyError, DependencyGraph, close_dependencies};
use super::level_b::{BodyDecision, DecisionRecord};

fn decision(id: &str, kind: BodyDecision) -> DecisionRecord {
    DecisionRecord {
        id: id.to_owned(),
        decision: kind,
        reason: "ROOT",
    }
}

#[test]
fn closes_direct_and_transitive_dependencies_with_reason_chains() {
    let decisions = vec![
        decision("app", BodyDecision::LoadNow),
        decision("parser", BodyDecision::LoadOnInvoke),
        decision("base", BodyDecision::LoadOnInvoke),
    ];
    let graph = DependencyGraph::new([
        ("app", vec!["parser"]),
        ("parser", vec!["base"]),
        ("base", vec![]),
    ]);
    let closed = close_dependencies(&decisions, &graph).unwrap();
    assert!(closed.iter().all(|r| r.decision == BodyDecision::LoadNow));
    assert_eq!(
        closed.iter().find(|r| r.id == "base").unwrap().reason_chain,
        vec!["app", "parser", "base"]
    );
}

#[test]
fn missing_nodes_and_cycles_refuse_deterministically() {
    let decisions = vec![decision("app", BodyDecision::LoadNow)];
    assert_eq!(
        close_dependencies(
            &decisions,
            &DependencyGraph::new([("app", vec!["missing"])])
        ),
        Err(DependencyError::MissingNode("missing".to_owned()))
    );
    let cyclic_decisions = vec![
        decision("app", BodyDecision::LoadNow),
        decision("loop", BodyDecision::LoadOnInvoke),
    ];
    let cyclic = DependencyGraph::new([("app", vec!["loop"]), ("loop", vec!["app"])]);
    assert_eq!(
        close_dependencies(&cyclic_decisions, &cyclic),
        Err(DependencyError::Cycle(vec![
            "app".to_owned(),
            "loop".to_owned(),
            "app".to_owned()
        ]))
    );
}
