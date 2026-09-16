use clroom::catalog::selection::{SelectionError, SelectionRequest, SelectionTarget};

const CODEX_BROWSER_BACKEND: &str = "browser@openai-bundled";
const CLAUDE_BROWSER_BACKEND: &str = "chrome";

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
    let mut include_browser = false;
    let mut exclude_browser = false;
    let mut browser_backend: Option<String> = None;

    for argument in args {
        if launcher_options && argument == "--" {
            launcher_options = false;
            provider_args.push(argument.clone());
        } else if launcher_options && matches!(argument.as_str(), "--with" | "--without") {
            return Err(invalid_selector());
        } else if launcher_options && argument == "--browser" {
            return Err(invalid_browser_backend());
        } else if launcher_options && let Some(value) = argument.strip_prefix("--browser=") {
            if value.is_empty() || browser_backend.replace(value.to_owned()).is_some() {
                return Err(invalid_browser_backend());
            }
        } else if launcher_options && let Some(value) = argument.strip_prefix("--with=") {
            if value == "browser" {
                include_browser = true;
            } else {
                request
                    .include_value(value)
                    .map_err(selection_error_message)?;
            }
        } else if launcher_options && let Some(value) = argument.strip_prefix("--without=") {
            if value == "browser" {
                exclude_browser = true;
            } else {
                request
                    .exclude_value(value)
                    .map_err(selection_error_message)?;
            }
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
            "CLROOM_RESOURCE_NOT_SELECTABLE: exact plugin/MCP activation is unavailable in v0.3; continue locally"
                .to_owned(),
        );
    }

    if browser_backend.is_some() && !include_browser {
        return Err(
            "CLROOM_BROWSER_BACKEND_REQUIRES_CAPABILITY: --browser=<provider-native-ref> requires --with=browser"
                .to_owned(),
        );
    }

    if include_browser && raw_browser_override {
        return Err(
            "CLROOM_BROWSER_SELECTION_CONFLICT: do not mix --with=browser with a provider-native browser flag before --; choose the CLROOM selector or pass provider-native arguments directly"
                .to_owned(),
        );
    }

    if include_browser || exclude_browser {
        match provider {
            Provider::Codex => {
                if exclude_browser {
                    // Codex clean defaults already keep plugins closed. A portable
                    // exclusion is therefore a deterministic no-op on this adapter.
                } else {
                    let backend = browser_backend.as_deref().unwrap_or(CODEX_BROWSER_BACKEND);
                    if backend != CODEX_BROWSER_BACKEND {
                        return Err(unsupported_browser_backend(provider, backend));
                    }
                    return Err(format!(
                        "CLROOM_CAPABILITY_NOT_SELECTABLE: Codex browser backend {backend} is recognized but not qualified for activation in v0.3; continue locally"
                    ));
                }
            }
            Provider::Claude => {
                if exclude_browser {
                    insert_provider_flag(&mut provider_args, "--no-chrome");
                } else {
                    let backend = browser_backend.as_deref().unwrap_or(CLAUDE_BROWSER_BACKEND);
                    if backend != CLAUDE_BROWSER_BACKEND {
                        return Err(unsupported_browser_backend(provider, backend));
                    }
                    insert_provider_flag(&mut provider_args, "--chrome");
                }
            }
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

fn invalid_browser_backend() -> String {
    "CLROOM_BROWSER_BACKEND_INVALID: use exactly one --browser=<provider-native-ref> before --"
        .to_owned()
}

fn unsupported_browser_backend(provider: Provider, backend: &str) -> String {
    let provider_name = match provider {
        Provider::Codex => "codex",
        Provider::Claude => "claude",
    };
    format!(
        "CLROOM_BROWSER_BACKEND_UNSUPPORTED: {backend} is not a qualified browser backend for {provider_name} in v0.3"
    )
}

fn selection_error_message(_error: SelectionError) -> String {
    invalid_selector()
}

#[cfg(test)]
mod tests {
    use super::{prepare, Provider};
    use clroom::catalog::selection::{SelectionError, SelectionRequest};

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
    fn claude_browser_alias_maps_to_native_flag() {
        let args = strings(&["--model", "sonnet", "--with=browser"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--model", "sonnet", "--chrome"])
        );
    }

    #[test]
    fn explicit_claude_browser_backend_maps_to_native_flag() {
        let args = strings(&["--with=browser", "--browser=chrome"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--chrome"])
        );
    }

    #[test]
    fn browser_alias_is_not_a_catalog_selector() {
        let mut request = SelectionRequest::default();
        assert_eq!(
            request.include_value("browser"),
            Err(SelectionError::InvalidSelector)
        );
        assert_eq!(
            prepare(Provider::Claude, &strings(&["--with=browser"])).unwrap(),
            strings(&["--chrome"])
        );
    }

    #[test]
    fn browser_alias_exclusion_wins_within_the_cli_layer() {
        let args = strings(&["--with=browser", "--without=browser"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--no-chrome"])
        );
        assert_eq!(
            prepare(Provider::Codex, &args).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn portable_and_native_claude_browser_controls_do_not_mix() {
        for native in ["--chrome", "--no-chrome"] {
            let error = prepare(
                Provider::Claude,
                &strings(&["--with=browser", native]),
            )
            .unwrap_err();
            assert!(error.starts_with("CLROOM_BROWSER_SELECTION_CONFLICT:"));
        }
    }

    #[test]
    fn generated_browser_flag_stays_before_provider_terminator() {
        let args = strings(&["--with=browser", "--browser=chrome", "--", "literal"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--chrome", "--", "literal"])
        );
    }

    #[test]
    fn clroom_browser_options_after_terminator_are_literal_provider_input() {
        let args = strings(&["--", "--with=browser", "--browser=chrome"]);
        assert_eq!(
            prepare(Provider::Claude, &args).unwrap(),
            strings(&["--no-chrome", "--", "--with=browser", "--browser=chrome"])
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
    fn codex_browser_backend_is_recognized_but_fails_closed_until_e2e_qualification() {
        for args in [
            strings(&["--with=browser"]),
            strings(&["--with=browser", "--browser=browser@openai-bundled"]),
        ] {
            let error = prepare(Provider::Codex, &args).unwrap_err();
            assert!(error.starts_with("CLROOM_CAPABILITY_NOT_SELECTABLE:"));
            assert!(error.contains("browser@openai-bundled"));
        }
    }

    #[test]
    fn codex_browser_exclusion_is_a_clean_noop() {
        assert_eq!(
            prepare(Provider::Codex, &strings(&["--without=browser"])).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn browser_backend_requires_portable_capability_and_must_match_provider() {
        for provider in [Provider::Codex, Provider::Claude] {
            let error = prepare(provider, &strings(&["--browser=anything"])).unwrap_err();
            assert!(error.starts_with("CLROOM_BROWSER_BACKEND_REQUIRES_CAPABILITY:"));
        }

        let claude = prepare(
            Provider::Claude,
            &strings(&["--with=browser", "--browser=browser@openai-bundled"]),
        )
        .unwrap_err();
        assert!(claude.starts_with("CLROOM_BROWSER_BACKEND_UNSUPPORTED:"));

        let codex = prepare(
            Provider::Codex,
            &strings(&["--with=browser", "--browser=chrome"]),
        )
        .unwrap_err();
        assert!(codex.starts_with("CLROOM_BROWSER_BACKEND_UNSUPPORTED:"));
    }

    #[test]
    fn duplicate_or_empty_browser_backend_is_rejected() {
        for args in [
            strings(&["--with=browser", "--browser="]),
            strings(&["--with=browser", "--browser"]),
            strings(&["--with=browser", "--browser=chrome", "--browser=chrome"]),
        ] {
            let error = prepare(Provider::Claude, &args).unwrap_err();
            assert!(error.starts_with("CLROOM_BROWSER_BACKEND_INVALID:"));
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
