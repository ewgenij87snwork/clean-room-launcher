use std::{fs, path::Path, process::Command};

fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

fn run(name: &str, scenario: Option<&str>, injected: Option<&str>) -> (bool, String, String, String) {
    let temp = std::env::temp_dir().join(format!("p07-homebrew-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    let result = temp.join("result.json");
    let fake = root().join("tests/packaging/fixtures/homebrew/fake_brew.py");
    let mut command = Command::new("python3");
    command.current_dir(root()).args(["packaging/homebrew/lifecycle.py", "--fake", "--fake-brew", fake.to_str().unwrap(), "--workspace", temp.to_str().unwrap(), "--output", result.to_str().unwrap()]);
    if let Some(value) = scenario { command.args(["--scenario", value]); }
    if let Some(step) = injected { command.args(["--inject-failure", step]); }
    let output = command.output().expect("lifecycle executable");
    (output.status.success(), fs::read_to_string(result).unwrap(), fs::read_to_string(temp.join("ledger.jsonl")).unwrap_or_default(), String::from_utf8(output.stdout).unwrap())
}

#[test]
fn fake_lifecycle_executes_each_approved_transition_and_cleans_up() {
    let (ok, json, ledger, marker) = run("success", None, None);
    assert!(ok, "{}", json);
    for needle in ["tap", "item_trust", "style", "audit", "install_n", "upgrade_n_plus_1", "rollback_n", "unlink", "link", "uninstall", "untrust", "untap"] { assert!(json.contains(needle), "missing step {}", needle); }
    for command in ["[\"tap\"", "[\"trust\",\"--formula\"", "[\"upgrade\"", "[\"unlink\"", "[\"link\"", "[\"untrust\",\"--formula\""] { assert!(ledger.contains(command), "missing ledger command {}: {}", command, ledger); }
    assert!(json.contains("\"cleanup_complete\":true"));
    assert!(json.contains("\"poison_provider_absent\":true"), "poison capture was not mechanically observed: {}", json);
    assert!(!json.contains("raw_output") && !json.contains("/Users/"));
    assert_eq!(marker, "P07_HOMEBREW_LIFECYCLE_TEST_PASS\n");
}

#[test]
fn fake_lifecycle_runs_finally_cleanup_after_injected_failure() {
    let (ok, json, ledger, _) = run("failure", None, Some("upgrade"));
    assert!(!ok);
    assert!(json.contains("\"failure_class\":\"UPGRADE_REFUSED\""));
    assert!(json.contains("\"cleanup_complete\":true"));
    assert!(ledger.contains("[\"untrust\",\"--formula\"") && ledger.contains("[\"untap\""));
}

#[test]
fn fake_lifecycle_closed_refusal_matrix_is_canonical_and_cleanup_owned() {
    let matrix = [
        ("reported_prefix_mismatch", "LIVE_HOMEBREW_BOUNDARY_REFUSED"),
        ("reported_repository_mismatch", "LIVE_HOMEBREW_BOUNDARY_REFUSED"),
        ("reported_cellar_mismatch", "LIVE_HOMEBREW_BOUNDARY_REFUSED"),
        ("missing_require_tap_trust", "TAP_TRUST_REFUSED"),
        ("wrong_allowed_taps", "TAP_TRUST_REFUSED"),
        ("whole_tap_trust", "TAP_TRUST_REFUSED"),
        ("missing_item_trust", "TAP_TRUST_REFUSED"),
        ("tap_clone_failed", "TAP_TRUST_REFUSED"),
        ("non_loopback_bind", "FORMULA_RENDER_REFUSED"),
        ("non_loopback_url", "FORMULA_RENDER_REFUSED"),
        ("extra_served_name", "FORMULA_RENDER_REFUSED"),
        ("checksum_substitution", "ARTIFACT_DIGEST_MISMATCH"),
        ("cache_substitution", "ARTIFACT_DIGEST_MISMATCH"),
        ("metadata_substitution", "ARTIFACT_METADATA_MISMATCH"),
        ("stale_link", "ROLLBACK_REFUSED"),
        ("unexpected_installed_path", "INSTALL_REFUSED"),
        ("smoke_refusal", "INSTALL_REFUSED"),
        ("config_mutation", "CONFIG_MUTATION_REFUSED"),
        ("sentinel_taskseal_mutation", "CONFIG_MUTATION_REFUSED"),
        ("sentinel_provider_mutation", "CONFIG_MUTATION_REFUSED"),
        ("sentinel_git_mutation", "CONFIG_MUTATION_REFUSED"),
        ("sentinel_homebrew_mutation", "CONFIG_MUTATION_REFUSED"),
        ("sentinel_unrelated_mutation", "CONFIG_MUTATION_REFUSED"),
        ("partial_uninstall", "UNINSTALL_REFUSED"),
        ("cleanup_failure", "CLEANUP_REFUSED"),
    ];
    for (scenario, refusal) in matrix {
        let (ok, json, ledger, _) = run(scenario, Some(scenario), None);
        assert!(!ok, "{} unexpectedly passed: {}", scenario, json);
        assert!(json.contains(&format!("\"failure_class\":\"{}\"", refusal)), "{}: {}", scenario, json);
        assert!(json.contains("\"raw_output\"")==false && json.contains("/Users/")==false, "{} leaked evidence: {}", scenario, json);
        if scenario == "tap_clone_failed" { assert!(json.contains("\"diagnostic\":\"tap_clone_refused\""), "{}: {}", scenario, json); }
        if scenario == "smoke_refusal" { assert!(json.contains("\"diagnostic\":\"smoke_formula_test_sandbox_refused\""), "{}: {}", scenario, json); }
        if !scenario.starts_with("reported_") {
            assert!(ledger.contains("[\"untap\",\"taskseal-local/preview\"]"), "{} did not run cleanup: {}", scenario, ledger);
        }
        if scenario == "cleanup_failure" || scenario == "partial_uninstall" || scenario.starts_with("reported_") { assert!(json.contains("\"cleanup_complete\":false")); }
        else { assert!(json.contains("\"cleanup_complete\":true"), "{}: {}", scenario, json); }
    }
}

#[test]
fn fixture_archive_pair_is_deterministic_distinct_and_executable_parity() {
    let temp = std::env::temp_dir().join(format!("p07-fixture-pair-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let generator = root().join("tests/packaging/fixtures/homebrew/make_fixture_archives.py");
    let output = Command::new("python3").args([generator.to_str().unwrap(), "--output-dir", temp.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let manifest = fs::read_to_string(temp.join("fixture-archives.json")).unwrap();
    assert!(manifest.contains("0.0.1") && manifest.contains("0.0.2"));
    assert!(manifest.contains("clean-room-launcher-v0.0.1") && manifest.contains("clean-room-launcher-v0.0.2"));
    assert_ne!(fs::read(temp.join("clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz")).unwrap(), fs::read(temp.join("clean-room-launcher-v0.0.2-aarch64-apple-darwin.tar.gz")).unwrap());
    let inspect = r#"import sys, tarfile
for path, version in zip(sys.argv[1:], ('0.0.1', '0.0.2')):
  with tarfile.open(path, 'r:gz') as archive:
    names = {member.name: member for member in archive.getmembers() if member.isfile()}
    root = f'clean-room-launcher-v{version}-aarch64-apple-darwin'
    assert set(names) == {f'{root}/LICENSE', f'{root}/NOTICE', f'{root}/VERSION', f'{root}/bin/clroom', f'{root}/share/doc/clean-room-launcher/CHANGELOG.md'}
    assert names[f'{root}/bin/clroom'].mode & 0o777 == 0o755
    version_text = archive.extractfile(names[f'{root}/VERSION']).read().decode()
    assert f'version={version}' in version_text and 'evidence_class=lifecycle-fixture' in version_text and 'source_commit=' in version_text and 'target=aarch64-apple-darwin' in version_text
"#;
    let first = temp.join("clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz"); let second = temp.join("clean-room-launcher-v0.0.2-aarch64-apple-darwin.tar.gz");
    assert!(Command::new("python3").args(["-c", inspect, first.to_str().unwrap(), second.to_str().unwrap()]).status().unwrap().success());
}

#[test]
fn real_mode_prepares_only_a_disposable_local_git_source_and_real_current_evidence() {
    let temp = std::env::temp_dir().join(format!("p07-real-preparation-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let source = temp.join("source");
    fs::create_dir_all(source.join("bin")).unwrap();
    let fake = root().join("tests/packaging/fixtures/homebrew/fake_brew.py");
    fs::copy(fake, source.join("bin/brew")).unwrap();
    let chmod = Command::new("chmod").args(["755", source.join("bin/brew").to_str().unwrap()]).status().unwrap();
    assert!(chmod.success());
    for args in [["init"].as_slice(), ["add", "."].as_slice(), ["-c", "user.name=p07", "-c", "user.email=p07@example.invalid", "commit", "-m", "fixture"].as_slice()] {
        assert!(Command::new("git").current_dir(&source).args(args).status().unwrap().success());
    }
    let portable = source.join("Library/Homebrew/vendor/portable-ruby/fixture/bin");
    fs::create_dir_all(&portable).unwrap();
    fs::write(portable.join("ruby"), "fixture portable ruby\n").unwrap();
    assert!(Command::new("chmod").args(["755", portable.join("ruby").to_str().unwrap()]).status().unwrap().success());
    std::os::unix::fs::symlink("fixture", source.join("Library/Homebrew/vendor/portable-ruby/current")).unwrap();
    let api_cache = temp.join("api-cache");
    let package = api_cache.join("internal/packages.arm64_fixture.jws.json");
    fs::create_dir_all(package.parent().unwrap()).unwrap();
    fs::write(&package, "fixture package metadata\n").unwrap();
    fs::write(api_cache.join("internal/packages.arm64_fixture.jws.json.payload"), "fixture payload\n").unwrap();
    fs::write(api_cache.join("internal/packages.arm64_fixture.jws.json.payload.index"), "fixture index\n").unwrap();
    let archives = temp.join("archives");
    let generator = root().join("tests/packaging/fixtures/homebrew/make_fixture_archives.py");
    assert!(Command::new("python3").args([generator.to_str().unwrap(), "--output-dir", archives.to_str().unwrap()]).status().unwrap().success());
    let archive = archives.join("clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz");
    let next_archive = archives.join("clean-room-launcher-v0.0.2-aarch64-apple-darwin.tar.gz");
    let digest = String::from_utf8(Command::new("shasum").args(["-a", "256", archive.to_str().unwrap()]).output().unwrap().stdout).unwrap().split_whitespace().next().unwrap().to_owned();
    let next_digest = String::from_utf8(Command::new("shasum").args(["-a", "256", next_archive.to_str().unwrap()]).output().unwrap().stdout).unwrap().split_whitespace().next().unwrap().to_owned();
    let input = temp.join("input-contract.json");
    fs::write(&input, format!(r#"{{"schema_version":"taskseal.p07.homebrew-input.v1","evidence_class":"real-current","archive":{{"filename":"clean-room-launcher-v0.0.1-aarch64-apple-darwin.tar.gz","sha256":"{digest}","size":1}},"artifact":{{"version":"0.0.1","source_commit":"1111111111111111111111111111111111111111","target":"aarch64-apple-darwin","qualification":"NOT_QUALIFIED","signing":"unsigned-preview-only","root":"clean-room-launcher-v0.0.1-aarch64-apple-darwin","members":["LICENSE","NOTICE","VERSION","bin/clroom","share/doc/clean-room-launcher/CHANGELOG.md"],"clroom_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},"host":{{"system":"Darwin","machine":"arm64","macho_arch":"arm64","minimum_macos":"13.0","homebrew_symbol":"ventura"}}}}"#)).unwrap();
    let next_input = temp.join("next-input-contract.json");
    fs::write(&next_input, format!(r#"{{"schema_version":"taskseal.p07.homebrew-input.v1","evidence_class":"real-current","archive":{{"filename":"clean-room-launcher-v0.0.2-aarch64-apple-darwin.tar.gz","sha256":"{next_digest}","size":1}},"artifact":{{"version":"0.0.2","source_commit":"1111111111111111111111111111111111111111","target":"aarch64-apple-darwin","qualification":"NOT_QUALIFIED","signing":"unsigned-preview-only","root":"clean-room-launcher-v0.0.2-aarch64-apple-darwin","members":["LICENSE","NOTICE","VERSION","bin/clroom","share/doc/clean-room-launcher/CHANGELOG.md"],"clroom_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},"host":{{"system":"Darwin","machine":"arm64","macho_arch":"arm64","minimum_macos":"13.0","homebrew_symbol":"ventura"}}}}"#)).unwrap();
    let workspace = temp.join("workspace"); fs::create_dir_all(&workspace).unwrap(); let result = temp.join("result.json");
    let output = Command::new("python3").current_dir(root()).env("AWS_SECRET_ACCESS_KEY", "must-not-inherit").args(["packaging/homebrew/lifecycle.py", "--brew-source", source.to_str().unwrap(), "--api-cache-source", api_cache.to_str().unwrap(), "--input-contract", input.to_str().unwrap(), "--real-archive", archive.to_str().unwrap(), "--expected-sha256", &digest, "--expected-source-commit", "1111111111111111111111111111111111111111", "--next-input-contract", next_input.to_str().unwrap(), "--next-real-archive", next_archive.to_str().unwrap(), "--next-expected-sha256", &next_digest, "--scenario", "require_native_install_boundary", "--workspace", workspace.to_str().unwrap(), "--output", result.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success(), "stderr={} result={}", String::from_utf8_lossy(&output.stderr), fs::read_to_string(&result).unwrap_or_default());
    let json = fs::read_to_string(result).unwrap();
    assert!(json.contains("\"evidence_class\":\"real-current-two-version\""), "{}", json);
    assert!(json.contains("\"clone_local\"") && json.contains("\"origin_removed\"") && json.contains("\"tap_git_ready\""), "{}", json);
    assert!(json.contains("\"network_boundary\":\"homebrew-native-sandbox-loopback-proxy\""), "{}", json);
    assert!(json.contains("upgrade_n_plus_1") && json.contains("rollback_n"), "{}", json);
    assert!(json.contains("\"name\":\"formula_syntax\"") && !json.contains("\"name\":\"style\"") && !json.contains("\"name\":\"audit\""), "{}", json);
    assert!(!json.contains("\"name\":\"smoke\""), "real-current duplicated installed-binary proof through network-dependent brew test: {}", json);
    let refused_workspace = temp.join("install-refusal-workspace"); fs::create_dir_all(&refused_workspace).unwrap(); let refused_result = temp.join("install-refusal-result.json");
    let refused = Command::new("python3").current_dir(root()).args(["packaging/homebrew/lifecycle.py", "--brew-source", source.to_str().unwrap(), "--api-cache-source", api_cache.to_str().unwrap(), "--input-contract", input.to_str().unwrap(), "--real-archive", archive.to_str().unwrap(), "--expected-sha256", &digest, "--expected-source-commit", "1111111111111111111111111111111111111111", "--next-input-contract", next_input.to_str().unwrap(), "--next-real-archive", next_archive.to_str().unwrap(), "--next-expected-sha256", &next_digest, "--scenario", "install_archive_fetch_failed", "--workspace", refused_workspace.to_str().unwrap(), "--output", refused_result.to_str().unwrap()]).output().unwrap();
    assert!(!refused.status.success()); let refused_json = fs::read_to_string(refused_result).unwrap();
    assert!(refused_json.contains("\"failure_class\":\"INSTALL_REFUSED\"") && refused_json.contains("\"diagnostic\":\"install_unclassified_error_formula_dependency_unsupported\"") && refused_json.contains("\"cleanup_complete\":true"), "{}", refused_json);
}
