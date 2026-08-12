use std::path::Path;

pub fn validate_boundary(
    repository: &Path,
    wisdom: &Path,
    branch: &str,
    is_gitlink: bool,
) -> Result<(), &'static str> {
    if branch.is_empty() || branch == "main" {
        return Err("MAIN_OR_EMPTY_BRANCH");
    }
    if repository == wisdom || repository.starts_with(wisdom) || wisdom.starts_with(repository) {
        return Err("NESTED_REPOSITORY_BOUNDARY");
    }
    if is_gitlink {
        return Err("GITLINK_REPOSITORY_BOUNDARY");
    }
    Ok(())
}
