use std::fs;
use std::path::Path;

#[test]
fn checked_in_authority_schema_accepts_only_the_exact_private_receipt() {
    let authority = fs::read(".taskseal-dev/execution-authority.json").expect("private authority exists");
    let schema = fs::read("schemas/contracts/execution-authority.schema.json").expect("schema exists");
    let subject = taskseal::contracts::execution::ExecutionSubject {
        worktree: "/Users/ysorokin/taskseal-wt/p02-contracts",
        branch: "feat/p02-contracts",
        head: "3e3f543629c834270440023d3c030a9891031684",
    };
    assert!(taskseal::contracts::execution::validate_authority(&schema, &authority, &subject).is_ok());

    for poison in [
        ("missing worktree", authority_without(&authority, "worktree_realpath")),
        ("main branch", replace_value(&authority, "feat/p02-contracts", "main")),
        ("wrong worktree", replace_value(&authority, subject.worktree, "/tmp/wrong-worktree")),
        ("stale head", replace_value(&authority, &"3e3f543629c834270440023d3c030a9891031684", &"0".repeat(40))),
        ("unknown field", add_unknown_field(&authority)),
    ] {
        assert!(
            taskseal::contracts::execution::validate_authority(&schema, &poison.1, &subject).is_err(),
            "accepted {} authority",
            poison.0
        );
    }
}

#[test]
fn root_instructions_match_the_sealed_template() {
    let local = fs::read("AGENTS.md").expect("root AGENTS exists");
    let sealed = fs::read(
        "/Users/ysorokin/Documents/it/5-LVL - 2026/Temp in Projects/wisdom/taskseal/templates/taskseal-root-AGENTS.md",
    )
    .expect("sealed root AGENTS exists");
    assert_eq!(local, sealed);
    assert!(fs::read_to_string(".gitignore").unwrap().lines().any(|line| line == ".taskseal-dev/"));
    assert!(!Path::new(".taskseal-dev/execution-authority.json").metadata().unwrap().permissions().readonly());
}

fn authority_without(bytes: &[u8], key: &str) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    value.as_object_mut().unwrap().remove(key);
    serde_json::to_vec(&value).unwrap()
}

fn replace_value(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(bytes.to_vec()).unwrap().replace(from, to).into_bytes()
}

fn add_unknown_field(bytes: &[u8]) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    value.as_object_mut().unwrap().insert("unknown".into(), true.into());
    serde_json::to_vec(&value).unwrap()
}
