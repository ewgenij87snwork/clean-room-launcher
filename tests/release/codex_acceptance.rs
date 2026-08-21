use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const NEEDED_SHA: &str = "5f07933adc98d602398359e57783d348658eaa60fa7907da263b53291f8184b1";
const UNUSED_SHA: &str = "32eea1ee666999e53a16feb944d355ef49bcff8ec379a3e02ee690172752f55e";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp(label: &str) -> PathBuf {
    let path = env::temp_dir().join(format!(
        "taskseal-codex-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn write(path: &Path, bytes: &str) {
    fs::write(path, bytes).unwrap();
}

fn sha256(path: &Path) -> String {
    let output = Command::new("shasum")
        .args(["-a", "256", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

fn make_archive(dir: &Path, binary: &[u8]) -> PathBuf {
    let stage = dir.join("taskseal-v0.1.0-test");
    fs::create_dir_all(stage.join("bin")).unwrap();
    fs::create_dir_all(stage.join("share/doc/taskseal")).unwrap();
    fs::write(stage.join("bin/taskseal"), binary).unwrap();
    fs::write(stage.join("bin/tseal"), binary).unwrap();
    write(&stage.join("LICENSE"), "fixture license\n");
    write(&stage.join("NOTICE"), "fixture notice\n");
    write(
        &stage.join("VERSION"),
        "source_commit=0000000000000000000000000000000000000000\nqualification=NOT_QUALIFIED\n",
    );
    write(
        &stage.join("share/doc/taskseal/CHANGELOG.md"),
        "fixture changelog\n",
    );
    let archive = dir.join("taskseal-v0.1.0-test.tar.gz");
    let python = r#"
import gzip, os, sys, tarfile
stage, archive = sys.argv[1:]
entries = []
for base, dirs, files in os.walk(stage):
    dirs.sort(); files.sort()
    for name in dirs + files:
        path = os.path.join(base, name)
        rel = os.path.relpath(path, os.path.dirname(stage)).replace(os.sep, "/")
        entries.append((rel, path))
entries.sort(key=lambda item: (item[0].count("/"), item[0]))
with open(archive, "wb") as raw:
    with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for rel, path in entries:
                info = tar.gettarinfo(path, arcname=rel)
                info.uid = info.gid = 0; info.uname = info.gname = ""; info.mtime = 0
                info.mode = 0o755 if info.isdir() or rel.endswith("/bin/taskseal") or rel.endswith("/bin/tseal") else 0o644
                if info.isfile():
                    with open(path, "rb") as data: tar.addfile(info, data)
                else: tar.addfile(info)
"#;
    let status = Command::new("python3")
        .args([
            "-c",
            python,
            stage.to_str().unwrap(),
            archive.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    archive
}

struct Fixture {
    _dir: PathBuf,
    artifact: PathBuf,
    installed: PathBuf,
    p06: PathBuf,
    p04: PathBuf,
    capture: PathBuf,
    output: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let dir = temp(label);
        let binary = b"#!/bin/sh\nexit 0\n";
        let artifact = make_archive(&dir, binary);
        let installed = dir.join("installed/bin/tseal");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::write(&installed, binary).unwrap();
        fs::set_permissions(&installed, fs::Permissions::from_mode(0o755)).unwrap();

        let artifact_sha = sha256(&artifact);
        let installed_sha = sha256(&installed);
        let p06 = dir.join("p06.json");
        write(
            &p06,
            &format!(
                "{{\"schema_version\":\"taskseal.codex-state-preservation-receipt.v1\",\"task\":11,\"result\":\"accepted\",\"qualification\":\"NOT_QUALIFIED\",\"tuple\":{{\"provider_id\":\"codex\",\"artifact_digest\":\"{artifact_sha}\",\"version\":[0,147,0],\"os\":\"macos\",\"arch\":\"aarch64\"}},\"provider_launch\":false,\"protected_state_unchanged\":true}}"
            ),
        );
        let p04 = dir.join("p04.json");
        write(
            &p04,
            &format!(
                "{{\"schema_version\":\"taskseal.p04.acceptance-evidence.v1\",\"census\":{{\"admitted\":2,\"loaded_now\":0}},\"context_bytes\":{{\"startup_level_a\":527,\"full_bodies_at_startup\":0}},\"inventory_body_digests\":{{\"a\":\"{NEEDED_SHA}\",\"b\":\"{UNUSED_SHA}\"}}}}"
            ),
        );
        let capture = dir.join("capture.json");
        write(
            &capture,
            &format!(
                "{{\"schema_version\":\"taskseal.codex-clean-launch-capture.v1\",\"capture_mode\":\"TEST_ONLY_FIXTURE\",\"promotion_eligible\":false,\"live_observation\":\"NOT_RUN\",\"terminal\":true,\"argv\":[\"tseal\",\"codex\",\"--\"],\"artifact_provenance\":{{\"install_source\":\"RELEASE_ARCHIVE\",\"archive_sha256\":\"{artifact_sha}\",\"installed_tseal_sha256\":\"{installed_sha}\",\"installed_path_class\":\"EXTERNAL_INSTALL_PREFIX\",\"developer_checkout\":false}},\"canaries\":{{\"needed\":{{\"id\":\"a\",\"body_sha256\":\"{NEEDED_SHA}\"}},\"unused\":{{\"id\":\"b\",\"body_sha256\":\"{UNUSED_SHA}\"}},\"protected\":{{\"id\":\"b\",\"body_sha256\":\"{UNUSED_SHA}\"}}}},\"startup\":{{\"context_sha256\":\"{}\",\"context_bytes\":527,\"compact_screen\":true,\"clean_startup\":true,\"full_bodies_visible\":0}},\"catalog\":{{\"admitted\":2,\"loaded_now\":0,\"needed_name_visible\":true,\"unused_body_present\":false,\"invoked_body_available\":true}},\"protected_state\":{{\"before_sha256\":\"{}\",\"after_sha256\":\"{}\",\"unchanged\":true}},\"cleanup\":{{\"exit\":\"NOT_RUN\",\"relaunch\":\"NOT_RUN\",\"uninstall\":\"NOT_RUN\"}}}}",
                digest('b'),
                digest('c'),
                digest('c')
            ),
        );
        let output = dir.join("codex-alpha.json");
        Self {
            _dir: dir,
            artifact,
            installed,
            p06,
            p04,
            capture,
            output,
        }
    }

    fn run(&self) -> Output {
        self.run_with(&sha256(&self.artifact), &sha256(&self.capture), true)
    }

    fn run_with(&self, artifact_sha: &str, capture_sha: &str, enable_fixture: bool) -> Output {
        let mut command = Command::new("sh");
        command.arg(root().join("scripts/release/accept-codex.sh"));
        if enable_fixture {
            command.arg("--test-only-fixture");
            command.env("TASKSEAL_TEST_ONLY_FIXTURE", "1");
        }
        command.args([
            "--artifact",
            self.artifact.to_str().unwrap(),
            "--artifact-sha256",
            artifact_sha,
            "--installed-tseal",
            self.installed.to_str().unwrap(),
            "--p06",
            self.p06.to_str().unwrap(),
            "--p04",
            self.p04.to_str().unwrap(),
            "--capture",
            self.capture.to_str().unwrap(),
            "--capture-sha256",
            capture_sha,
            "--output",
            self.output.to_str().unwrap(),
        ]);
        command.output().unwrap()
    }

    fn replace_capture(&self, from: &str, to: &str) {
        let source = fs::read_to_string(&self.capture).unwrap().replace(from, to);
        write(&self.capture, &source);
    }
}

fn assert_refusal(output: Output, reason: &str, receipt: &Path) {
    assert!(!output.status.success(), "{reason} was accepted");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(reason),
        "expected {reason}, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!receipt.exists(), "{reason} created retained output");
}

#[test]
fn fixture_wiring_binds_archive_installed_binary_capture_and_never_promotes() {
    let fixture = Fixture::new("prepared");
    let result = fixture.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "P08_CODEX_ACCEPTANCE_PREPARED_NOT_QUALIFIED\n"
    );
    let receipt = fs::read_to_string(&fixture.output).unwrap();
    for required in [
        "\"schema_version\":\"taskseal.codex-clean-launch-acceptance.v1\"",
        "\"result\":\"PREPARED_NOT_QUALIFIED\"",
        "\"live_observation\":\"NOT_RUN\"",
        "\"reason\":\"OWNER_GATE_REQUIRED_PROVIDER_PROCESS\"",
        "\"capture_mode\":\"TEST_ONLY_FIXTURE\"",
        "\"fixture_only\":true",
        "\"promotion_eligible\":false",
        "\"command\":[\"tseal\",\"codex\",\"--\"]",
        "\"installed_artifact_verified\":true",
        "\"canaries\":{",
        "\"cleanup\":{\"exit\":\"NOT_RUN\",\"relaunch\":\"NOT_RUN\",\"uninstall\":\"NOT_RUN\"}",
        "\"capture_sha256\":\"",
        "\"capture_schema_sha256\":\"",
        "\"output_sha256\":\"",
    ] {
        assert!(receipt.contains(required), "missing {required}: {receipt}");
    }
    for forbidden in [
        fixture.installed.to_str().unwrap(),
        "#!/bin/sh",
        "raw_prompt",
        "credential",
        "private_path",
    ] {
        assert!(
            !receipt.contains(forbidden),
            "retained unsafe value: {forbidden}"
        );
    }
}

#[test]
fn developer_checkout_and_unmarked_fixture_are_refused_explicitly() {
    let mut checkout = Fixture::new("checkout");
    let original_installed_sha = sha256(&checkout.installed);
    checkout.installed = root().join("scripts/release/accept-codex.sh");
    checkout.replace_capture(
        &format!("\"installed_tseal_sha256\":\"{original_installed_sha}\""),
        &format!(
            "\"installed_tseal_sha256\":\"{}\"",
            sha256(&checkout.installed)
        ),
    );
    assert_refusal(
        checkout.run(),
        "DEVELOPER_CHECKOUT_REFUSED",
        &checkout.output,
    );

    let fixture = Fixture::new("fixture-marker");
    let output = fixture.run_with(&sha256(&fixture.artifact), &sha256(&fixture.capture), false);
    assert_refusal(output, "PRODUCTION_ARTIFACT_PIN_REQUIRED", &fixture.output);
}

#[test]
fn harness_refuses_every_reviewed_evidence_and_observation_failure() {
    let checksum = Fixture::new("checksum");
    assert_refusal(
        checksum.run_with(&digest('0'), &sha256(&checksum.capture), true),
        "ARTIFACT_CHECKSUM_MISMATCH",
        &checksum.output,
    );

    let capture_digest = Fixture::new("capture-digest");
    assert_refusal(
        capture_digest.run_with(&sha256(&capture_digest.artifact), &digest('0'), true),
        "CAPTURE_CHECKSUM_MISMATCH",
        &capture_digest.output,
    );

    let provenance = Fixture::new("provenance");
    write(&provenance.installed, "changed installed bytes\n");
    assert_refusal(
        provenance.run(),
        "INSTALLED_ARTIFACT_MISMATCH",
        &provenance.output,
    );

    let p06 = Fixture::new("p06");
    write(&p06.p06, "{\"schema_version\":\"fabricated\"}\n");
    assert_refusal(p06.run(), "P06_ARTIFACT_OR_RECEIPT_MISMATCH", &p06.output);

    let p04 = Fixture::new("p04");
    write(&p04.p04, "{\"schema_version\":\"fabricated\"}\n");
    assert_refusal(p04.run(), "P04_CANARY_EVIDENCE_INVALID", &p04.output);

    let mutations = vec![
        (
            "terminal",
            "\"terminal\":true".to_owned(),
            "\"terminal\":false".to_owned(),
            "NON_TERMINAL_EXECUTION",
        ),
        (
            "argv",
            "[\"tseal\",\"codex\",\"--\"]".to_owned(),
            "[\"tseal\",\"codex\",\"--\",\"--dangerously-bypass-approvals-and-sandbox\"]"
                .to_owned(),
            "UNSAFE_ARGS",
        ),
        (
            "canary",
            format!("\"protected\":{{\"id\":\"b\",\"body_sha256\":\"{UNUSED_SHA}\"}}"),
            "\"protected\":null".to_owned(),
            "CANARY_EVIDENCE_INVALID",
        ),
        (
            "body",
            "\"unused_body_present\":false".to_owned(),
            "\"unused_body_present\":true".to_owned(),
            "UNEXPECTED_BODY_VISIBILITY",
        ),
        (
            "startup",
            format!("\"context_sha256\":\"{}\"", digest('b')),
            "\"context_sha256\":\"invalid\"".to_owned(),
            "STARTUP_CONTEXT_INVALID",
        ),
        (
            "protected",
            format!("\"after_sha256\":\"{}\"", digest('c')),
            format!("\"after_sha256\":\"{}\"", digest('d')),
            "PROTECTED_MUTATION",
        ),
        (
            "cleanup",
            "\"uninstall\":\"NOT_RUN\"".to_owned(),
            "\"uninstall\":\"OBSERVED_PASS\"".to_owned(),
            "CLEANUP_ABSENT",
        ),
        (
            "private",
            "{\"schema_version\"".to_owned(),
            "{\"raw_prompt\":\"secret\",\"schema_version\"".to_owned(),
            "CAPTURE_SCHEMA_INVALID",
        ),
    ];
    for (label, from, to, reason) in mutations {
        let fixture = Fixture::new(label);
        fixture.replace_capture(&from, &to);
        assert_refusal(fixture.run(), reason, &fixture.output);
    }
}

#[test]
fn output_verifier_refuses_post_write_edits() {
    let fixture = Fixture::new("edited-output");
    let result = fixture.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let verified = Command::new("sh")
        .arg(root().join("scripts/release/accept-codex.sh"))
        .args(["--verify-output", fixture.output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(verified.status.success());
    fs::OpenOptions::new()
        .append(true)
        .open(&fixture.output)
        .unwrap()
        .write_all(b" ")
        .unwrap();
    let edited = Command::new("sh")
        .arg(root().join("scripts/release/accept-codex.sh"))
        .args(["--verify-output", fixture.output.to_str().unwrap()])
        .output()
        .unwrap();
    assert_refusal(edited, "OUTPUT_EDITED", &PathBuf::from("/never-created"));
}

fn git(repo: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap()
}

fn verify_receipt(repo: &Path, receipt_file: &Path, tip: &str) -> Output {
    Command::new("sh")
        .arg(root().join("scripts/release/verify-codex-task3-receipt.sh"))
        .args([
            "--repo",
            repo.to_str().unwrap(),
            "--receipt-file",
            receipt_file.to_str().unwrap(),
            "--receipt-path",
            "reports/gates/p08/task-3.json",
            "--tip",
            tip,
        ])
        .output()
        .unwrap()
}

fn git_show_sha256(repo: &Path, commit: &str, path: &str) -> String {
    let show = git(repo, &["show", &format!("{commit}:{path}")]);
    assert!(show.status.success(), "missing {commit}:{path}");
    let bytes = temp("subject-bytes").join("subject");
    fs::write(&bytes, show.stdout).unwrap();
    sha256(&bytes)
}

#[test]
fn receipt_validator_recomputes_subjects_topology_and_descendant_durability() {
    let scratch = temp("receipt-durability");
    let clone = scratch.join("repository");
    let cloned = Command::new("git")
        .args(["clone", "--quiet", "--no-hardlinks"])
        .arg(root())
        .arg(&clone)
        .status()
        .unwrap();
    assert!(cloned.success());
    assert!(
        git(
            &clone,
            &["config", "user.email", "taskseal-test@example.invalid"]
        )
        .status
        .success()
    );
    assert!(
        git(&clone, &["config", "user.name", "TaskSeal Test"])
            .status
            .success()
    );
    let implementation_head = String::from_utf8(git(&clone, &["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    let subject_paths = [
        "reports/release/codex-alpha.json",
        "scripts/release/accept-codex.sh",
        "tests/release/codex_acceptance.rs",
    ];
    let subjects: Vec<(&str, String)> = subject_paths
        .iter()
        .map(|path| (*path, git_show_sha256(&clone, &implementation_head, path)))
        .collect();
    let aggregate_file = scratch.join("aggregate");
    let aggregate_records: String = subjects
        .iter()
        .map(|(path, digest)| format!("{path}\t{digest}\n"))
        .collect();
    write(&aggregate_file, &aggregate_records);
    let aggregate = sha256(&aggregate_file);
    let subjects_json = subjects
        .iter()
        .map(|(path, digest)| format!("\"{path}\":\"{digest}\""))
        .collect::<Vec<_>>()
        .join(",");
    let receipt_rel = "reports/gates/p08/task-3.json";
    let clone_receipt = clone.join(receipt_rel);
    let valid_receipt = format!(
        "{{\"schema_version\":\"taskseal.p08.task-receipt.v1\",\"plan_id\":\"P08\",\"task\":3,\"acceptance_id\":\"ACC-P08-T3\",\"binding\":{{\"implementation_head\":\"{implementation_head}\",\"receipt_commit_parent\":\"{implementation_head}\",\"receipt_seal_role\":\"replacement-receipt-only-child\"}},\"subjects\":{{{subjects_json}}},\"subject_sha256\":{{\"algorithm\":\"sha256 of sorted path, tab, sha256, newline records\",\"value\":\"{aggregate}\"}}}}\n"
    );
    write(&clone_receipt, &valid_receipt);
    assert!(git(&clone, &["add", receipt_rel]).status.success());
    assert!(
        git(
            &clone,
            &["commit", "--quiet", "-m", "seal synthetic Task 3 receipt"]
        )
        .status
        .success()
    );
    write(&clone.join("descendant-marker"), "durable\n");
    assert!(git(&clone, &["add", "descendant-marker"]).status.success());
    assert!(
        git(
            &clone,
            &["commit", "--quiet", "-m", "test descendant durability"]
        )
        .status
        .success()
    );
    let descendant = verify_receipt(&clone, &clone.join("reports/gates/p08/task-3.json"), "HEAD");
    assert!(
        descendant.status.success(),
        "{}",
        String::from_utf8_lossy(&descendant.stderr)
    );
    assert!(String::from_utf8_lossy(&descendant.stdout).contains("P08_T3_RECEIPT_DURABILITY_PASS"));

    let current_receipt = root().join("reports/gates/p08/task-3.json");
    let current = verify_receipt(&root(), &current_receipt, "HEAD");
    assert!(
        current.status.success(),
        "{}",
        String::from_utf8_lossy(&current.stderr)
    );
    assert!(String::from_utf8_lossy(&current.stdout).contains("P08_T3_RECEIPT_DURABILITY_PASS"));

    let malformed_format = scratch.join("malformed-subject-format.json");
    write(
        &malformed_format,
        &fs::read_to_string(&current_receipt).unwrap().replacen(
            "109f84a1118a867d20fa8bb0796d1ca6dbc6306e395d38986ff713d93115c739",
            "7951495b2527a1e39b3f09ccd2b00199b4ad8e05",
            1,
        ),
    );
    assert_refusal(
        verify_receipt(&root(), &malformed_format, "HEAD"),
        "SUBJECT_SHA256_FORMAT_INVALID",
        &PathBuf::from("/never-created"),
    );

    let tampered_subject = scratch.join("tampered-subject.json");
    let original = valid_receipt.clone();
    write(
        &tampered_subject,
        &original.replacen(&subjects[0].1, &digest('0'), 1),
    );
    assert_refusal(
        verify_receipt(&clone, &tampered_subject, "HEAD"),
        "SUBJECT_SHA256_MISMATCH",
        &PathBuf::from("/never-created"),
    );

    let tampered_aggregate = scratch.join("tampered-aggregate.json");
    write(
        &tampered_aggregate,
        &original.replacen(&aggregate, &digest('1'), 1),
    );
    assert_refusal(
        verify_receipt(&clone, &tampered_aggregate, "HEAD"),
        "SUBJECT_AGGREGATE_MISMATCH",
        &PathBuf::from("/never-created"),
    );

    let tampered_identity = scratch.join("tampered-identity.json");
    write(
        &tampered_identity,
        &original.replacen("\"task\":3", "\"task\":4", 1),
    );
    assert_refusal(
        verify_receipt(&clone, &tampered_identity, "HEAD"),
        "RECEIPT_IDENTITY_INVALID",
        &PathBuf::from("/never-created"),
    );

    write(&clone_receipt, "{\"tampered\":true}\n");
    assert!(git(&clone, &["add", receipt_rel]).status.success());
    assert!(
        git(&clone, &["commit", "--quiet", "-m", "tamper receipt"])
            .status
            .success()
    );
    write(&clone_receipt, &original);
    assert!(git(&clone, &["add", receipt_rel]).status.success());
    assert!(
        git(
            &clone,
            &["commit", "--quiet", "-m", "duplicate receipt bytes"]
        )
        .status
        .success()
    );
    assert_refusal(
        verify_receipt(&clone, &clone_receipt, "HEAD"),
        "RECEIPT_COMMIT_NOT_UNIQUE",
        &PathBuf::from("/never-created"),
    );
}

#[test]
fn committed_alpha_receipt_keeps_live_action_unexecuted_and_exact() {
    let receipt = fs::read_to_string(root().join("reports/release/codex-alpha.json"))
        .expect("committed Codex alpha receipt is missing");
    for required in [
        "\"schema_version\": \"taskseal.codex-clean-launch-acceptance.v1\"",
        "\"result\": \"PREPARED_NOT_QUALIFIED\"",
        "\"live_observation\": \"NOT_RUN\"",
        "\"reason\": \"OWNER_GATE_REQUIRED_PROVIDER_PROCESS\"",
        "\"live_argv\": [",
        "\"tseal\"",
        "\"codex\"",
        "\"--\"",
        "\"capture_schema\": {",
        "\"fixture_can_promote\": false",
        "\"installed_artifact_required\": true",
    ] {
        assert!(receipt.contains(required), "missing {required}");
    }
    for forbidden in [
        "raw_prompt",
        "credential",
        "/Users/",
        "provider_response",
        "--safe",
    ] {
        assert!(
            !receipt.contains(forbidden),
            "unsafe retained evidence: {forbidden}"
        );
    }
}

#[test]
fn committed_live_observation_binds_the_campaign_without_promoting_the_stale_p06_tuple() {
    let observation =
        fs::read_to_string(root().join("reports/release/codex-live-observation.json"))
            .expect("sanitized Codex live observation is missing");
    for required in [
        "\"schema_version\": \"taskseal.p08.codex-live-observation.v1\"",
        "\"result\": \"OBSERVED_NOT_QUALIFIED\"",
        "\"reason\": \"P06_EXACT_TUPLE_NOT_QUALIFIED_AND_CURRENT_TUPLE_MISMATCH\"",
        "\"artifact_sha256\": \"ea8e60d2b4097ce766758bd70543628d0c15e9c7ab0ebc0d29d76c59da896b0c\"",
        "\"installed_tseal_sha256\": \"3ebcba17d54a983c2c3ab504b7ea7975b7654f275d8465826690e39e8047d52b\"",
        "\"version\": [0, 146, 1]",
        "\"executable_sha256\": \"35d248101b211d6248ad4e6b8c1d441fe81236da87afb9f3e9ea51a049e9f179\"",
        "\"process_budget\": 5",
        "\"processes_executed\": 5",
        "\"model_requests\": 4",
        "\"real_terminal\": true",
        "\"required_context_observed\": true",
        "\"needed_body_observed\": true",
        "\"unused_and_protected_bodies_absent\": true",
        "\"protected_canaries_unchanged\": true",
        "\"raw_provider_output_retained\": false",
        "\"temporary_roots_removed\": true",
        "\"qualification\": \"NOT_QUALIFIED\"",
        "\"tuple_match\": false",
        "\"GLOBAL_PROVIDER_CONTEXT_ISOLATION_UNPROVEN\"",
        "\"LIVE_STARTUP_CONTEXT_BYTE_COUNT_NOT_RETAINED\"",
        "\"p06_exact_qualified_tuple\": false",
        "\"overall\": false",
    ] {
        assert!(observation.contains(required), "missing {required}");
    }
    for forbidden in [
        "TASKSEAL_SKILL_BODY_",
        "provider_response",
        "raw_prompt",
        "auth.json",
        "access_token",
        "/Users/",
        "\"qualification\": \"QUALIFIED\"",
        "\"result\": \"PASS\"",
    ] {
        assert!(
            !observation.contains(forbidden),
            "unsafe or unsupported retained claim: {forbidden}"
        );
    }
}

use std::io::Write;
