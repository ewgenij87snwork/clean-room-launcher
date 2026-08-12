use crate::core::budget::{BudgetInput, BudgetLimits};
use crate::core::inventory::{AdmittedRoot, inventory};
use crate::core::manifest::{Compilation, build_manifest};
use crate::core::operations::{CoreService, Decision, Disposition, RuntimeFacts};
use crate::core::publish::publish;
use crate::core::render::ArtifactSet;
use cap_std::{ambient_authority, fs::Dir};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1000);

fn root() -> (std::path::PathBuf, Dir) {
    let path = std::env::temp_dir().join(format!(
        "taskseal-p03-operations-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    let dir = Dir::open_ambient_dir(&path, ambient_authority()).unwrap();
    (path, dir)
}

#[test]
fn audit_inspect_and_doctor_are_typed_and_do_not_write() {
    let (path, root) = root();
    root.write("source.txt", b"abc").unwrap();
    let sources = inventory(&[AdmittedRoot::new(&path, "project")]).unwrap();
    let before = fs::read_dir(&path).unwrap().count();
    let audit = CoreService::audit(&sources);
    let decisions = vec![Decision::new(
        "skill:rust",
        Disposition::Deferred,
        "DETAILS_LOAD_ON_INVOKE",
    )];
    let inspected = CoreService::inspect(&decisions, "skill:rust").unwrap();
    let doctor = CoreService::doctor(&RuntimeFacts::new(true, true, false));

    assert_eq!((audit.source_count, audit.total_bytes), (1, 3));
    assert_eq!(inspected.decision, decisions[0]);
    assert!(doctor.ready);
    assert_eq!(before, fs::read_dir(&path).unwrap().count());
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn compile_is_deterministic_and_verify_refuses_tampered_output() {
    let report = CoreService::compile(
        BudgetInput::from_layers("safe", "project", "task", 3, true),
        BudgetLimits::new(100, 3, 100),
        vec!["scope:project".to_owned()],
    )
    .unwrap();
    let repeated = CoreService::compile(
        BudgetInput::from_layers("safe", "project", "task", 3, true),
        BudgetLimits::new(100, 3, 100),
        vec!["scope:project".to_owned()],
    )
    .unwrap();
    assert_eq!(report.manifest, repeated.manifest);
    assert_eq!(report.artifacts, repeated.artifacts);

    let (path, root) = root();
    publish(&root, &report.artifacts, &report.manifest).unwrap();
    assert_eq!(
        CoreService::verify(&root).unwrap().manifest,
        report.manifest
    );
    root.write(
        format!(
            ".taskseal/out/generations/{}/context.md",
            report.manifest.digest
        ),
        b"tamper",
    )
    .unwrap();
    assert_eq!(
        CoreService::verify(&root).unwrap_err().code,
        "VERIFY_DIGEST_MISMATCH"
    );
    drop(root);
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn stable_errors_and_decisions_are_shared_across_operations() {
    let error = CoreService::compile(
        BudgetInput::from_layers("too large", "", "", 1, false),
        BudgetLimits::new(1, 1, 1),
        vec![],
    )
    .unwrap_err();
    assert_eq!(error.code, "COMPILE_BUDGET_EXCEEDED");
    assert_eq!(error.decision.reason_code, "BUDGET_EXCEEDED");
    assert_eq!(
        CoreService::inspect(&[], "missing").unwrap_err().code,
        "INSPECT_NOT_FOUND"
    );

    let artifacts = ArtifactSet::from([("context.md".to_owned(), b"x".to_vec())]);
    let compilation = Compilation::new(vec![], artifacts);
    assert!(build_manifest(&compilation).is_ok());
}
