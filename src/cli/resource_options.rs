use clroom::catalog::selection::{SelectionError, SelectionRequest, SelectionTarget};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    Codex,
    Claude,
}

pub fn prepare(provider: Provider, args: &[String]) -> Result<Vec<String>, String> {
    let mut request = SelectionRequest::default();
    let mut provider_args = Vec::with_capacity(args.len() + 1);
    let mut launcher_options = true;
    let mut raw_chrome_override = false;

    for argument in args {
        if launcher_options && argument == "--" {
            launcher_options = false;
            provider_args.push(argument.clone());
        } else if launcher_options && matches!(argument.as_str(), "--with" | "--without") {
            return Err(invalid_selector());
        } else if launcher_options && let Some(value) = argument.strip_prefix("--with=") {
            request
                .include_value(value)
                .map_err(selection_error_message)?;
        } else if launcher_options && let Some(value) = argument.strip_prefix("--without=") {
            request
                .exclude_value(value)
                .map_err(selection_error_message)?;
        } else {
            if provider == Provider::Claude
                && launcher_options
                && matches!(argument.as_str(), "--chrome" | "--no-chrome")
            {
                raw_chrome_override = true;
            }
            provider_args.push(argument.clone());
        }
    }

    if request
        .includes
        .iter()
        .chain(request.excludes.iter())
        .any(|target| matches!(target, SelectionTarget::All))
    {
        return Err(
            "CLROOM_RESOURCE_ALL_UNAVAILABLE_IN_V0_3: --with=all/--without=all is unavailable in v0.3"
                .to_owned(),
        );
    }

    if request
        .includes
        .iter()
        .chain(request.excludes.iter())
        .any(|target| matches!(target, SelectionTarget::Exact { .. }))
    {
        return Err(
            "CLROOM_RESOURCE_NOT_SELECTABLE: selective provider-native activation is unavailable in v0.3; continue locally"
                .to_owned(),
        );
    }

    if provider == Provider::Claude && !raw_chrome_override {
        // Claude can persist its native Chrome integration in provider-owned state.
        // A clean launch closes that ambient input unless the caller explicitly
        // supplies the provider-native override for this invocation.
        provider_args.insert(0, "--no-chrome".to_owned());
    }

    Ok(provider_args)
}

fn invalid_selector() -> String {
    "CLROOM_RESOURCE_SELECTOR_INVALID: invalid resource selector".to_owned()
}

fn selection_error_message(_error: SelectionError) -> String {
    invalid_selector()
}

#[cfg(test)]
mod tests {
    use super::{prepare, Provider};

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn claude_clean_default_disables_ambient_native_chrome_state() {
        assert_eq!(
            prepare(Provider::Claude, &strings(&["--model", "sonnet"])).unwrap(),
            strings(&["--no-chrome", "--model", "sonnet"])
        );
    }

    #[test]
    fn raw_claude_chrome_override_is_not_shadowed_by_clean_default() {
        for flag in ["--chrome", "--no-chrome"] {
            let args = strings(&[flag, "--model", "sonnet"]);
            assert_eq!(prepare(Provider::Claude, &args).unwrap(), args);
        }
    }

    #[test]
    fn provider_terminator_keeps_later_arguments_literal() {
        let args = strings(&["--", "--chrome"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--no-chrome", "--", "--chrome"])
        );
    }

    #[test]
    fn claude_exact_plugin_is_the_only_v0_4_provider_native_slice() {
        assert!(
            prepare(
                Provider::Claude,
                &strings(&["--with=plugin:review-tools@team"])
            )
            .is_ok()
        );

        for (provider, selector) in [
            (Provider::Codex, "--with=plugin:review-tools@team"),
            (Provider::Claude, "--without=mcp:local-tools"),
        ] {
            let error = prepare(provider, &strings(&[selector])).unwrap_err();
            assert!(error.starts_with("CLROOM_RESOURCE_NOT_SELECTABLE:"));
        }
    }

    #[test]
    fn all_and_malformed_or_unknown_selectors_fail_with_stable_codes() {
        let all = prepare(Provider::Claude, &strings(&["--with=all"])).unwrap_err();
        assert!(all.starts_with("CLROOM_RESOURCE_ALL_UNAVAILABLE_IN_V0_3:"));

        for selector in ["--with", "--without", "--with=", "--with=capability"] {
            let error = prepare(Provider::Claude, &strings(&[selector])).unwrap_err();
            assert!(error.starts_with("CLROOM_RESOURCE_SELECTOR_INVALID:"));
        }
    }
}
