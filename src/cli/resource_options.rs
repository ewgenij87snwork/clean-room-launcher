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
    let mut raw_browser_override = false;

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
            if launcher_options && matches!(argument.as_str(), "--chrome" | "--no-chrome") {
                raw_browser_override = true;
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
        return Err(selection_error_message(
            SelectionError::AllUnavailableInV03,
        ));
    }

    if request
        .includes
        .iter()
        .chain(request.excludes.iter())
        .any(|target| matches!(target, SelectionTarget::Exact { .. }))
    {
        return Err(
            "CLROOM_RESOURCE_NOT_SELECTABLE: exact plugin/MCP activation is unavailable in v0.3; continue locally"
                .to_owned(),
        );
    }

    let include_browser = request.includes.contains(&SelectionTarget::Browser);
    let exclude_browser = request.excludes.contains(&SelectionTarget::Browser);
    if include_browser || exclude_browser {
        match provider {
            Provider::Codex => {
                return Err(
                    "CLROOM_RESOURCE_NOT_SELECTABLE: codex:browser:browser is not qualified in v0.3; continue locally"
                        .to_owned(),
                );
            }
            Provider::Claude => insert_provider_flag(
                &mut provider_args,
                if exclude_browser {
                    "--no-chrome"
                } else {
                    "--chrome"
                },
            ),
        }
    } else if provider == Provider::Claude && !raw_browser_override {
        // Chrome can be enabled by persistent provider state. A clean launch must
        // explicitly close that ambient capability unless this invocation opts in.
        provider_args.insert(0, "--no-chrome".to_owned());
    }

    Ok(provider_args)
}

fn insert_provider_flag(args: &mut Vec<String>, flag: &str) {
    let index = args
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(args.len());
    args.insert(index, flag.to_owned());
}

fn invalid_selector() -> String {
    "CLROOM_RESOURCE_SELECTOR_INVALID: invalid resource selector; use --with=browser or --without=browser in v0.3"
        .to_owned()
}

fn selection_error_message(error: SelectionError) -> String {
    match error {
        SelectionError::AllUnavailableInV03 => {
            "CLROOM_RESOURCE_ALL_UNAVAILABLE_IN_V0_3: --with=all/--without=all is unavailable in v0.3"
                .to_owned()
        }
        _ => invalid_selector(),
    }
}

#[cfg(test)]
mod tests {
    use super::{prepare, Provider};

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn claude_clean_default_disables_ambient_browser_state() {
        assert_eq!(
            prepare(Provider::Claude, &strings(&["--model", "sonnet"])).unwrap(),
            strings(&["--no-chrome", "--model", "sonnet"])
        );
    }

    #[test]
    fn raw_claude_browser_override_is_not_shadowed_by_clean_default() {
        for flag in ["--chrome", "--no-chrome"] {
            let args = strings(&[flag, "--model", "sonnet"]);
            assert_eq!(prepare(Provider::Claude, &args).unwrap(), args);
        }
    }

    #[test]
    fn claude_browser_selector_maps_to_native_flag() {
        let args = strings(&["--model", "sonnet", "--with=browser"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--model", "sonnet", "--chrome"])
        );
    }

    #[test]
    fn exclusion_wins_and_is_inserted_after_raw_provider_flags() {
        let args = strings(&[
            "--with=browser",
            "--without=browser",
            "--chrome",
        ]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--chrome", "--no-chrome"])
        );
    }

    #[test]
    fn generated_browser_flag_stays_before_provider_terminator() {
        let args = strings(&["--with=browser", "--", "literal"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--chrome", "--", "literal"])
        );
    }

    #[test]
    fn selector_after_terminator_is_literal_provider_input() {
        let args = strings(&["--", "--with=browser"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--no-chrome", "--", "--with=browser"])
        );
    }

    #[test]
    fn raw_browser_flag_after_terminator_does_not_override_clean_default() {
        let args = strings(&["--", "--chrome"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--no-chrome", "--", "--chrome"])
        );
    }

    #[test]
    fn codex_browser_selection_fails_closed() {
        for selector in ["--with=browser", "--without=browser"] {
            let error = prepare(Provider::Codex, &strings(&[selector])).unwrap_err();
            assert!(error.starts_with("CLROOM_RESOURCE_NOT_SELECTABLE:"));
        }
    }

    #[test]
    fn exact_plugin_and_mcp_activation_remain_v04() {
        for selector in ["--with=plugin:demo@market", "--without=mcp:demo"] {
            let error = prepare(Provider::Claude, &strings(&[selector])).unwrap_err();
            assert!(error.starts_with("CLROOM_RESOURCE_NOT_SELECTABLE:"));
        }
    }

    #[test]
    fn all_and_malformed_selectors_fail_with_stable_codes() {
        let all = prepare(Provider::Claude, &strings(&["--with=all"])).unwrap_err();
        assert!(all.starts_with("CLROOM_RESOURCE_ALL_UNAVAILABLE_IN_V0_3:"));

        for selector in ["--with", "--without", "--with=", "--with=hook:x"] {
            let error = prepare(Provider::Claude, &strings(&[selector])).unwrap_err();
            assert!(error.starts_with("CLROOM_RESOURCE_SELECTOR_INVALID:"));
        }
    }
}
