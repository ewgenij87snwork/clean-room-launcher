use super::state::{AccessClass, SavedStart, StateStore};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::Arc,
    thread,
};

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("clroom-state-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn start(label: &str, access_class: AccessClass) -> SavedStart {
    SavedStart {
        provider: "codex".to_owned(),
        argv: vec!["--model".to_owned(), label.to_owned(), String::new()],
        project_digest: "a".repeat(64),
        access_class,
        qualification_digest: "b".repeat(64),
    }
}

#[test]
fn store_keeps_three_mru_records_as_literal_argv_vectors() {
    // Break caught: shell text, a fourth record, or an old record first destroys repeat-start safety.
    let store = StateStore::at(scratch("mru"));
    store.save(start("one", AccessClass::Standard)).unwrap();
    store
        .save(start("two words", AccessClass::FullAccess))
        .unwrap();
    store.save(start("three", AccessClass::Standard)).unwrap();
    store.save(start("four", AccessClass::Standard)).unwrap();

    let saved = store.load().unwrap();
    assert_eq!(saved.starts.len(), 3);
    assert_eq!(saved.starts[0].argv, ["--model", "four", ""]);
    assert_eq!(saved.starts[1].argv, ["--model", "three", ""]);
    assert_eq!(saved.starts[2].access_class, AccessClass::FullAccess);
    assert!(
        !fs::read_to_string(store.state_path())
            .unwrap()
            .contains("/Users/")
    );
}

#[test]
fn sensitive_argv_refuses_without_creating_state() {
    // Break caught: partial/redacted secrets still reach a private state file.
    let store = StateStore::at(scratch("secret"));
    let mut candidate = start("safe", AccessClass::Standard);
    candidate.argv.push("--token=sk-live-secret".to_owned());

    assert_eq!(
        store.save(candidate).unwrap_err().code(),
        "SAVED_START_SENSITIVE_ARGUMENT_REFUSED"
    );
    assert!(!store.state_path().exists());
}

#[test]
fn final_zero_auth_ingestion_refuses_access_token_argv_before_state_creation() {
    for (index, flag) in ["--with-access-token", "--access-token"]
        .into_iter()
        .enumerate()
    {
        let store = StateStore::at(scratch(&format!("access-token-{index}")));
        let mut candidate = start("safe", AccessClass::Standard);
        candidate
            .argv
            .extend([flag.to_owned(), "must-not-be-stored".to_owned()]);

        assert_eq!(
            store.save(candidate).unwrap_err().code(),
            "SAVED_START_SENSITIVE_ARGUMENT_REFUSED"
        );
        assert!(!store.state_path().exists());
    }
}

#[test]
fn final_zero_auth_inline_access_token_save_refuses_before_any_write() {
    // Existing-GREEN proof: inline access-token forms must refuse before lock/temp/state creation.
    for (index, argument) in [
        "--with-access-token=must-not-be-serialized",
        "--access-token=must-not-be-serialized",
    ]
    .into_iter()
    .enumerate()
    {
        let store = StateStore::at(scratch(&format!("inline-access-token-{index}")));
        let mut candidate = start("safe", AccessClass::Standard);
        candidate.argv.push(argument.to_owned());

        assert_eq!(
            store.save(candidate).unwrap_err().code(),
            "SAVED_START_SENSITIVE_ARGUMENT_REFUSED"
        );
        assert!(
            fs::read_dir(store.root()).unwrap().next().is_none(),
            "sensitive inline argv must refuse before lock, temp, or state creation"
        );
    }
}

#[test]
fn final_zero_auth_ingestion_raw_token_state_refuses_before_deserialization() {
    let store = StateStore::at(scratch("raw-sensitive-before-json"));
    fs::write(
        store.state_path(),
        br#"{"schema_version":"taskseal.saved-start.v1","starts":[{"argv":["--access-token","must-not-be-deserialized"]"#,
    )
    .unwrap();
    fs::set_permissions(store.state_path(), fs::Permissions::from_mode(0o600)).unwrap();

    assert_eq!(
        store.load().unwrap_err().code(),
        "SAVED_START_SENSITIVE_ARGUMENT_REFUSED"
    );
}

#[test]
fn record_requires_a_closed_provider_and_digest_fields() {
    // Break caught: arbitrary provider labels or path-like metadata enter a record advertised as closed/digested.
    let store = StateStore::at(scratch("closed-record"));
    let mut provider = start("safe", AccessClass::Standard);
    provider.provider = "other-provider".to_owned();
    assert_eq!(
        store.save(provider).unwrap_err().code(),
        "STATE_RECORD_REFUSED"
    );

    let mut digest = start("safe", AccessClass::Standard);
    digest.project_digest = "not-a-digest".to_owned();
    assert_eq!(
        store.save(digest).unwrap_err().code(),
        "STATE_RECORD_REFUSED"
    );
    assert!(!store.state_path().exists());
}

#[test]
fn malformed_or_permissive_state_refuses_without_replacement() {
    // Break caught: malformed/private-state damage is silently overwritten or exposed by broad modes.
    let store = StateStore::at(scratch("unsafe"));
    fs::write(store.state_path(), b"not json").unwrap();
    fs::set_permissions(store.state_path(), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(store.load().unwrap_err().code(), "STATE_CORRUPT_REFUSED");
    assert_eq!(fs::read(store.state_path()).unwrap(), b"not json");

    fs::write(
        store.state_path(),
        br#"{"schema_version":"taskseal.saved-start.v1","starts":[]}"#,
    )
    .unwrap();
    fs::set_permissions(store.state_path(), fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        store.load().unwrap_err().code(),
        "STATE_PERMISSIONS_REFUSED"
    );
}

#[test]
fn symlinked_state_and_orphan_temp_refuse() {
    // Break caught: attacker-controlled redirection or interrupted replacement is followed/cleaned blindly.
    let root = scratch("symlink");
    let store = StateStore::at(root.clone());
    let target = root.join("target.json");
    fs::write(&target, b"target").unwrap();
    symlink(&target, store.state_path()).unwrap();
    assert_eq!(store.load().unwrap_err().code(), "STATE_SYMLINK_REFUSED");

    fs::remove_file(store.state_path()).unwrap();
    fs::write(store.interrupted_temp_path(), b"partial").unwrap();
    assert_eq!(
        store
            .save(start("safe", AccessClass::Standard))
            .unwrap_err()
            .code(),
        "STATE_INTERRUPTED_WRITE_REFUSED"
    );
}

#[test]
fn concurrent_saves_remain_valid_and_bounded() {
    // Break caught: read-modify-write races lose a committed record or leave partial JSON.
    let store = Arc::new(StateStore::at(scratch("concurrent")));
    let mut workers = Vec::new();
    for label in ["one", "two", "three", "four"] {
        let store = Arc::clone(&store);
        workers.push(thread::spawn(move || {
            store.save(start(label, AccessClass::Standard))
        }));
    }
    for worker in workers {
        worker.join().unwrap().unwrap();
    }

    let saved = store.load().unwrap();
    assert!(saved.starts.len() <= 3);
    assert_eq!(
        fs::metadata(store.state_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    assert_eq!(
        fs::metadata(store.root()).unwrap().permissions().mode() & 0o077,
        0
    );
}
