use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static PROBE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn repository_root() -> PathBuf {
    std::env::current_dir().expect("test runs from repository root")
}

fn run_probe(extra_args: &[&str]) -> std::process::Output {
    let root = repository_root();
    let sequence = PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_root = std::env::temp_dir().join(format!(
        "taskseal-provider-test-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&temp_root).expect("create isolated probe temp root");
    let provider = extra_args
        .windows(2)
        .find(|args| args[0] == "--provider")
        .map(|args| args[1])
        .expect("probe test declares a provider");
    let fake_bin = temp_root.join("bin");
    std::fs::create_dir(&fake_bin).expect("create fake provider bin");
    let fake_provider = fake_bin.join(provider);
    let script = match provider {
        "codex" => {
            "#!/bin/sh\ncase \"$1\" in\n  --version) printf 'codex 0.147.0\\n' ;;\n  debug)\n    if [ \"$3\" = 'TASKSEAL_START_PROBE' ]; then\n      printf '{\"skills\":[\"TASKSEAL_CANARY_TRIGGER\"]}\\n'\n    else\n      printf '{}\\n'\n    fi\n    ;;\n  *) exit 64 ;;\nesac\n"
        }
        "claude" => "#!/bin/sh\nprintf '2.1.223 (Claude Code)\\n'\n",
        _ => panic!("unsupported fake provider {provider}"),
    };
    std::fs::write(&fake_provider, script).expect("write fake provider");
    std::fs::set_permissions(&fake_provider, std::fs::Permissions::from_mode(0o700))
        .expect("make fake provider executable");
    let mut probe_paths = vec![fake_bin.clone()];
    probe_paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let probe_path = std::env::join_paths(probe_paths).expect("construct probe PATH");
    let output = Command::new(root.join("scripts/probe/provider-capabilities.sh"))
        .arg("--root")
        .arg(&root)
        .args(extra_args)
        .env("TMPDIR", &temp_root)
        .env("PATH", probe_path)
        .output()
        .expect("provider capability probe must be executable");
    std::fs::remove_dir_all(&fake_bin).expect("remove fake provider bin");
    let residue = std::fs::read_dir(&temp_root)
        .expect("read isolated probe temp root")
        .count();
    std::fs::remove_dir_all(&temp_root).expect("remove isolated test temp root");
    assert_eq!(residue, 0, "provider probe left temporary state behind");
    output
}

#[test]
fn provider_probe_requires_opaque_preauthentication_before_process_birth() {
    for (index, (state, expected)) in [
        (None, "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_REQUIRED\n"),
        (
            Some("unavailable"),
            "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_UNAVAILABLE\n",
        ),
        (
            Some("ambiguous"),
            "PROVIDER_NATIVE_PREAUTHENTICATED_SESSION_AMBIGUOUS\n",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let root = repository_root();
        let fake_root = std::env::temp_dir().join(format!(
            "taskseal-provider-preauth-{}-{index}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&fake_root);
        std::fs::create_dir_all(&fake_root).unwrap();
        let capture = fake_root.join("provider-born");
        let fake = fake_root.join("codex");
        std::fs::write(
            &fake,
            "#!/bin/sh\n: > \"$TASKSEAL_PROVIDER_CAPTURE\"\nprintf 'codex 0.147.0\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let inherited_path = std::env::var("PATH").unwrap_or_default();
        let probe_path = format!("{}:{inherited_path}", fake_root.display());

        let mut command = Command::new(root.join("scripts/probe/provider-capabilities.sh"));
        command
            .args([
                "--root",
                root.to_str().unwrap(),
                "--provider",
                "codex",
                "--fixture",
                "no-native-isolation",
            ])
            .env("PATH", probe_path)
            .env("TASKSEAL_PROVIDER_CAPTURE", &capture);
        if let Some(state) = state {
            command.args(["--preauthenticated-session", state]);
        }
        let output = command.output().expect("guarded probe must run");
        let provider_born = capture.exists();
        std::fs::remove_dir_all(&fake_root).unwrap();

        assert_eq!(output.status.code(), Some(78));
        assert!(output.stdout.is_empty());
        assert_eq!(String::from_utf8(output.stderr).unwrap(), expected);
        assert!(
            !provider_born,
            "provider process was born before session guard"
        );
    }
}

#[test]
fn codex_fixture_produces_closed_capability_truth_without_a_clean_overclaim() {
    let output = run_probe(&[
        "--provider",
        "codex",
        "--fixture",
        "qualified-home",
        "--preauthenticated-session",
        "available",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
    for required in [
        "\"executable_digest\"",
        "\"version\"",
        "\"discovery_roots\"",
        "\"native_metadata_lifecycle\"",
        "\"runtime_filter\"",
        "\"auth_dependencies\"",
        "\"ambient_sources\"",
        "\"projection_candidate\"",
        "\"state\"",
    ] {
        assert!(
            report.contains(required),
            "missing field {required}: {report}"
        );
    }
    assert!(report.contains("\"state\":\"narrowed\""), "{report}");
    assert!(
        report.contains("\"metadata_at_start\":\"qualified\""),
        "{report}"
    );
    assert!(
        report.contains("\"body_on_invocation\":\"unsupported\""),
        "{report}"
    );
    assert!(
        report.contains("\"projection_candidate\":false"),
        "boolean field encoded incorrectly: {report}"
    );
    assert!(
        report.contains("\"persistent_state_unchanged\":true"),
        "{report}"
    );
    assert!(
        !report.contains("TASKSEAL_CANARY_BODY_7E5B1E21"),
        "body leaked: {report}"
    );
}

#[test]
fn absent_native_isolation_refuses_a_requested_clean_claim() {
    let output = run_probe(&[
        "--provider",
        "codex",
        "--fixture",
        "no-native-isolation",
        "--require-clean-claim",
        "--preauthenticated-session",
        "available",
    ]);
    assert!(
        !output.status.success(),
        "unsupported clean claim was accepted"
    );
    let error = String::from_utf8(output.stderr).expect("probe error is UTF-8");
    assert!(error.contains("UNSUPPORTED_CLEAN_CLAIM"), "{error}");
}

#[test]
fn wrong_version_and_poisoned_ambient_source_cannot_qualify() {
    for fixture in ["wrong-version", "poisoned-home"] {
        let output = run_probe(&[
            "--provider",
            "codex",
            "--fixture",
            fixture,
            "--preauthenticated-session",
            "available",
        ]);
        assert!(
            output.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
        assert!(
            report.contains("\"state\":\"unsupported\"")
                || report.contains("\"state\":\"narrowed\""),
            "{fixture}: {report}"
        );
        assert!(
            !report.contains("TASKSEAL_POISON_BODY_933BF642"),
            "{fixture}: body leaked"
        );
    }
}

#[test]
fn claude_evidence_is_no_spend_and_never_runtime_qualified() {
    let output = run_probe(&[
        "--provider",
        "claude",
        "--fixture",
        "no-spend",
        "--preauthenticated-session",
        "available",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8(output.stdout).expect("probe output is UTF-8 JSON");
    assert!(report.contains("\"provider\":\"claude\""), "{report}");
    assert!(report.contains("\"state\":\"unsupported\""), "{report}");
    assert!(report.contains("\"model_invoked\":false"), "{report}");
}
