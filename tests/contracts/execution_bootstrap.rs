use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn checked_in_authority_schema_accepts_only_the_exact_private_receipt() {
    let authority = fs::read(".taskseal-dev/execution-authority.json").expect("private authority exists");
    let schema = fs::read("schemas/contracts/execution-authority.schema.json").expect("schema exists");
    let command_output = |args: &[&str]| {
        let output = Command::new("git").args(args).output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    let worktree = std::env::current_dir().unwrap();
    let branch = command_output(&["branch", "--show-current"]);
    let head = command_output(&["rev-parse", "HEAD"]);
    let subject = taskseal::contracts::execution::ExecutionSubject {
        worktree: worktree.to_str().unwrap(),
        branch: &branch,
        head: &head,
    };
    assert!(taskseal::contracts::execution::validate_authority(&schema, &authority, &subject).is_ok());

    for poison in [
        ("missing worktree", authority_without(&authority, "worktree_realpath")),
        ("main branch", replace_value(&authority, &branch, "main")),
        ("wrong worktree", replace_value(&authority, subject.worktree, "/tmp/wrong-worktree")),
        ("stale head", replace_value(&authority, &head, &"0".repeat(40))),
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
    let authority: serde_json::Value = serde_json::from_slice(
        &fs::read(".taskseal-dev/execution-authority.json").expect("private authority exists"),
    )
    .unwrap();
    let checkpoint = Path::new(authority["plan_checkpoint_path"].as_str().unwrap());
    let sealed_path = checkpoint.parent().unwrap().parent().unwrap().join("templates/taskseal-root-AGENTS.md");
    let sealed = fs::read(sealed_path).expect("sealed root AGENTS exists");
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
