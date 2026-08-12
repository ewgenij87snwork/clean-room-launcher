use super::sources::{
    CatalogError, SkillSourceAuthority, SkillSourceConfig, SkillSourceVisibility, enumerate_sources,
};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/catalog/sources")
        .join(name)
}

#[test]
fn admits_only_explicit_capability_roots_and_reports_outside_sources() {
    let project = fixture("project");
    let imported = fixture("imported-user-library");
    let outside = fixture("outside-boundary");
    let config = SkillSourceConfig::new(vec![
        (project.clone(), SkillSourceAuthority::Project),
        (imported.clone(), SkillSourceAuthority::ExplicitImport),
        (outside.clone(), SkillSourceAuthority::ExplicitImport),
    ]);

    let sources = enumerate_sources(&config, &[project.clone(), imported.clone()]).unwrap();

    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].root, imported);
    assert_eq!(sources[0].visibility, SkillSourceVisibility::Admitted);
    assert!(sources[0].admitted);
    assert_eq!(sources[1].root, outside);
    assert_eq!(
        sources[1].visibility,
        SkillSourceVisibility::OutsideBoundary
    );
    assert!(!sources[1].admitted);
    assert_eq!(sources[2].root, project);
    assert_eq!(sources[2].visibility, SkillSourceVisibility::Admitted);
    assert!(sources[2].admitted);
    assert!(
        sources
            .iter()
            .all(|source| source.id.starts_with("skill-source-"))
    );
}

#[test]
fn refuses_duplicate_roots_after_normalization() {
    let project = fixture("project");
    let duplicate = project.join(".");
    let config = SkillSourceConfig::new(vec![
        (project.clone(), SkillSourceAuthority::Project),
        (duplicate, SkillSourceAuthority::ExplicitImport),
    ]);

    let error = enumerate_sources(&config, &[project]).unwrap_err();

    assert_eq!(error, CatalogError::DuplicateRoot);
}

#[cfg(unix)]
#[test]
fn refuses_symlink_roots_instead_of_following_escape() {
    use std::os::unix::fs::symlink;

    let scratch = std::env::temp_dir().join(format!("taskseal-p04-source-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let link = scratch.join("escape");
    symlink(fixture("outside-boundary"), &link).unwrap();
    let config = SkillSourceConfig::new(vec![(link, SkillSourceAuthority::Project)]);

    let error = enumerate_sources(&config, std::slice::from_ref(&scratch)).unwrap_err();
    fs::remove_dir_all(scratch).unwrap();

    assert_eq!(error, CatalogError::SymlinkRoot);
}

#[test]
fn poisoned_home_is_not_an_implicit_source() {
    let project = fixture("project");
    let poisoned_home = fixture("poisoned-home");
    let config = SkillSourceConfig::new(vec![(project.clone(), SkillSourceAuthority::Project)]);

    let sources = enumerate_sources(&config, &[project]).unwrap();

    assert_eq!(sources.len(), 1);
    assert!(sources.iter().all(|source| source.root != poisoned_home));
}

#[cfg(unix)]
#[test]
fn refuses_symlink_in_any_parent_component() {
    use std::os::unix::fs::symlink;
    let scratch = std::env::temp_dir().join(format!("taskseal-p04-parent-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(scratch.join("real/root")).unwrap();
    symlink(scratch.join("real"), scratch.join("alias")).unwrap();
    let root = scratch.join("alias/root");
    let config = SkillSourceConfig::new(vec![(root.clone(), SkillSourceAuthority::Project)]);
    assert_eq!(
        enumerate_sources(&config, std::slice::from_ref(&scratch)).unwrap_err(),
        CatalogError::SymlinkRoot
    );
    fs::remove_dir_all(scratch).unwrap();
}
