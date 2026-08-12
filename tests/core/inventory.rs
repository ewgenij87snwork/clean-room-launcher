use std::fs;
use std::path::PathBuf;

#[test]
fn inventory_returns_sorted_records_with_content_hashes() {
    let root = temp_root("sorted-tree");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("z.txt"), b"zulu").unwrap();
    fs::write(root.join("nested/a.txt"), b"alpha").unwrap();

    let records = super::inventory(&[super::AdmittedRoot::new(&root, "repo")]).unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].logical_path, "repo/nested/a.txt");
    assert_eq!(records[0].byte_len, 5);
    assert_eq!(
        records[0].sha256,
        "8ed3f6ad685b959ead7022518e1af76cd816f8e8ec7ccdda1ed4018e8f2223f8"
    );
    assert_eq!(records[1].logical_path, "repo/z.txt");
    assert_eq!(records[1].byte_len, 4);
}

#[test]
fn inventory_excludes_siblings_and_refuses_symlink_entries() {
    let parent = temp_root("boundary");
    let admitted = parent.join("admitted");
    let sibling = parent.join("sibling");
    fs::create_dir_all(&admitted).unwrap();
    fs::create_dir_all(&sibling).unwrap();
    fs::write(admitted.join("allowed.txt"), b"allowed").unwrap();
    fs::write(sibling.join("poison.txt"), b"must not be read").unwrap();

    let records = super::inventory(&[super::AdmittedRoot::new(&admitted, "repo")]).unwrap();
    assert_eq!(
        records
            .iter()
            .map(|record| record.logical_path.as_str())
            .collect::<Vec<_>>(),
        ["repo/allowed.txt"]
    );

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&sibling, admitted.join("escape")).unwrap();
        let error = super::inventory(&[super::AdmittedRoot::new(&admitted, "repo")]).unwrap_err();
        assert!(error.to_string().starts_with("SYMLINK_ESCAPE:"));
    }
}

#[test]
fn inventory_refuses_symlink_roots_and_invalid_logical_prefixes() {
    let parent = temp_root("root-boundary");
    let admitted = parent.join("admitted");
    fs::create_dir_all(&admitted).unwrap();
    fs::write(admitted.join("allowed.txt"), b"allowed").unwrap();

    for prefix in ["", "/repo", "repo/../sibling", "repo//nested"] {
        let error = super::inventory(&[super::AdmittedRoot::new(&admitted, prefix)]).unwrap_err();
        assert!(error.to_string().starts_with("INVALID_LOGICAL_PREFIX:"));
    }

    #[cfg(unix)]
    {
        let alias = parent.join("alias");
        std::os::unix::fs::symlink(&admitted, &alias).unwrap();
        let error = super::inventory(&[super::AdmittedRoot::new(&alias, "repo")]).unwrap_err();
        assert!(error.to_string().starts_with("SYMLINK_ESCAPE:"));
    }
}

#[cfg(unix)]
#[test]
fn inventory_refuses_non_utf8_names_instead_of_lossy_normalization() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let invalid = PathBuf::from(OsString::from_vec(vec![b'x', 0xff]));
    let error = super::join_logical("repo", &invalid).unwrap_err();
    assert!(error.to_string().starts_with("INVALID_UTF8_PATH:"));
}

#[test]
fn inventory_refuses_a_file_changed_after_opening() {
    let root = temp_root("file-race");
    fs::create_dir_all(&root).unwrap();
    let raced = root.join("raced.txt");
    fs::write(&raced, b"before").unwrap();

    let error =
        super::inventory_with_observer(&[super::AdmittedRoot::new(&root, "repo")], &mut |_| {
            fs::write(&raced, b"after-and-longer").unwrap()
        })
        .unwrap_err();
    assert!(error.to_string().starts_with("PATH_RACE:"));
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("taskseal-p03-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}
