use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

const CONFIG_RELATIVE_PATH: &str = "clroom/skill-sets.yaml";

pub fn expand(terms: &[String], home: &Path) -> Result<Vec<String>, String> {
    if !terms.iter().any(|term| term.starts_with('@')) {
        let mut seen = BTreeSet::new();
        return Ok(terms
            .iter()
            .filter(|term| seen.insert((*term).clone()))
            .cloned()
            .collect());
    }

    let path = config_path(home)?;
    let bytes = fs::read(&path).map_err(|_| {
        format!(
            "CLROOM_SKILL_SET_CONFIG_UNAVAILABLE: skill-set file is unavailable at {}; create it or remove the @set reference",
            path.display()
        )
    })?;
    let sets = serde_yaml::from_slice::<BTreeMap<String, Vec<String>>>(&bytes).map_err(|_| {
        format!(
            "CLROOM_SKILL_SET_CONFIG_INVALID: skill-set file is invalid at {}; continue locally",
            path.display()
        )
    })?;

    validate_sets(&sets, &path)?;
    let mut expanded = Vec::new();
    let mut seen = BTreeSet::new();
    for term in terms {
        if let Some(name) = term.strip_prefix('@') {
            if !valid_name(name) {
                return Err(format!(
                    "CLROOM_SKILL_SET_INVALID: invalid skill set '{term}'; use @name"
                ));
            }
            let selectors = sets.get(name).ok_or_else(|| {
                format!(
                    "CLROOM_SKILL_SET_UNKNOWN: unknown skill set '{term}' in {}; continue locally",
                    path.display()
                )
            })?;
            for selector in selectors {
                if seen.insert(selector.clone()) {
                    expanded.push(selector.clone());
                }
            }
        } else if seen.insert(term.clone()) {
            expanded.push(term.clone());
        }
    }
    Ok(expanded)
}

pub fn config_path(home: &Path) -> Result<PathBuf, String> {
    let base = match env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        Some(value) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(
                    "CLROOM_SKILL_SET_CONFIG_PATH_INVALID: skill-set config root is unavailable"
                        .to_owned(),
                );
            }
            path
        }
        None => home.join(".config"),
    };
    if !base.is_absolute() {
        return Err(
            "CLROOM_SKILL_SET_CONFIG_PATH_INVALID: skill-set config root is unavailable".to_owned(),
        );
    }
    Ok(base.join(CONFIG_RELATIVE_PATH))
}

fn validate_sets(sets: &BTreeMap<String, Vec<String>>, path: &Path) -> Result<(), String> {
    for (name, selectors) in sets {
        if !valid_name(name) || selectors.is_empty() {
            return Err(format!(
                "CLROOM_SKILL_SET_CONFIG_INVALID: skill-set file is invalid at {}; continue locally",
                path.display()
            ));
        }
        for selector in selectors {
            if selector.starts_with('@') {
                return Err(format!(
                    "CLROOM_SKILL_SET_NESTED: nested skill set '{selector}' is not allowed in {}; continue locally",
                    path.display()
                ));
            }
            if !valid_selector(selector) {
                return Err(format!(
                    "CLROOM_SKILL_SET_CONFIG_INVALID: skill-set file is invalid at {}; continue locally",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn valid_selector(selector: &str) -> bool {
    let mut parts = selector.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    valid_name(first) && second.is_none_or(valid_name) && parts.next().is_none()
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

#[cfg(test)]
mod tests {
    use super::expand;
    use std::path::Path;

    #[test]
    fn direct_selectors_are_deduplicated_in_first_seen_order_without_a_config_read() {
        let terms = ["testing", "code-review", "testing"]
            .map(str::to_owned)
            .to_vec();

        assert_eq!(
            expand(&terms, Path::new("/home/does-not-need-to-exist")).unwrap(),
            ["testing", "code-review"].map(str::to_owned)
        );
    }
}
