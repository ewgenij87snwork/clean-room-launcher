#!/bin/sh
# P08 Task 7: one fail-closed release audit over the exact private candidate.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
exec python3 - "$root" "$@" <<'PY'
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(sys.argv[1]).resolve()
PRIVATE_PATH = re.compile(r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)")
CYRILLIC = re.compile(r"[А-Яа-яЁё]")
OFFICIAL_SPDX_SCHEMA = Path("packaging/supply-chain/schemas/cyclonedx-1.7/spdx.schema.json")
OFFICIAL_SPDX_SCHEMA_SHA256 = "c87aa7bb5eb503d40b52ec6bf00de8045df15da7a13cea48d290cf6d36a8d2ea"
UNSUPPORTED_CLAIM = re.compile(
    r"(?:SLSA\s+Level\s+[1-4]|production[- ]ready|fully\s+secure|"
    r"supports?\s+all\s+(?:operating\s+systems|providers)|guaranteed\s+(?:safe|secure))",
    re.IGNORECASE,
)
TELEMETRY_OR_BACKEND = re.compile(
    r"(?:analytics\.(?:example|invalid)|/collect\b|send_analytics\s*\(|"
    r"(?:reqwest|ureq)::|TcpStream\s*::|UdpSocket\s*::)",
    re.IGNORECASE,
)


class AuditError(Exception):
    pass


def refuse(code):
    raise AuditError(code)


def parse_options(values):
    if len(values) % 2:
        refuse("USAGE")
    options = {}
    for key, value in zip(values[0::2], values[1::2]):
        if not key.startswith("--") or key in options:
            refuse("USAGE")
        options[key] = value
    fixture = {"--fixture-root", "--output"}
    release = {"--repo", "--artifact", "--output", "--tool-root", "--cargo-home"}
    if set(options) == fixture:
        return "fixture", options
    if set(options) == release:
        return "release", options
    refuse("USAGE")


def regular(path):
    return path.is_file() and not path.is_symlink()


def sha256(path):
    if not regular(path):
        refuse("SUBJECT_FILE")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path):
    def unique(items):
        value = {}
        for key, item in items:
            if key in value:
                refuse("DUPLICATE_JSON_KEY")
            value[key] = item
        return value

    if not regular(path):
        refuse("JSON_FILE")
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    except (OSError, UnicodeError, json.JSONDecodeError):
        refuse("JSON_FILE")


def build_public_source_projection(repo, source_archive, source_root):
    inventory_path = repo / "qualification/public-release-inventory-v1.json"
    inventory = load_json(inventory_path)
    expected_keys = {
        "schema_version",
        "result",
        "public_paths",
        "conditional_public_paths",
        "excluded_internal_paths",
        "negative_fixture_paths",
        "generic_fixture_root",
        "history_policy",
    }
    if set(inventory) != expected_keys:
        refuse("PUBLIC_INVENTORY_SCHEMA")
    if (
        inventory.get("schema_version") != "taskseal.p06.public-release-inventory.v1"
        or inventory.get("result") != "PASS"
        or inventory.get("history_policy") != "IMMUTABLE_INTERNAL_EVIDENCE_EXCLUDED_FROM_PUBLIC_SOURCE"
        or inventory.get("excluded_internal_paths") != [".taskseal-dev", "reports/gates", "scripts/gates"]
        or inventory.get("negative_fixture_paths") != ["tests/fixtures", "tests/release/audit_fixtures"]
    ):
        refuse("PUBLIC_INVENTORY_SCHEMA")

    required = inventory.get("public_paths")
    conditional = inventory.get("conditional_public_paths")
    if not all(
        isinstance(values, list)
        and values == sorted(set(values))
        and all(isinstance(item, str) and item and not Path(item).is_absolute() and ".." not in Path(item).parts for item in values)
        for values in (required, conditional)
    ):
        refuse("PUBLIC_INVENTORY_PATHS")
    for excluded in inventory["excluded_internal_paths"]:
        if any(item == excluded or item.startswith(excluded + "/") for item in required + conditional):
            refuse("PUBLIC_INVENTORY_BOUNDARY")

    archive_paths = []
    for relative in required:
        exists = run(["git", "cat-file", "-e", f"HEAD:{relative}"], cwd=repo)
        if exists.returncode:
            refuse("PUBLIC_INVENTORY_REQUIRED_PATH")
        archive_paths.append(relative)
    for relative in conditional:
        exists = run(["git", "cat-file", "-e", f"HEAD:{relative}"], cwd=repo)
        if not exists.returncode:
            archive_paths.append(relative)

    archived = run(
        ["git", "archive", "--format=tar", "-o", source_archive, "HEAD", "--", *archive_paths],
        cwd=repo,
    )
    if archived.returncode:
        refuse("SOURCE_ARCHIVE")
    unpacked = run(["tar", "-xf", source_archive, "-C", source_root], cwd=repo)
    if unpacked.returncode:
        refuse("SOURCE_ARCHIVE")
    if load_json(source_root / "qualification/public-release-inventory-v1.json") != inventory:
        refuse("PUBLIC_INVENTORY_STALE")
    if any(path.is_symlink() for path in source_root.rglob("*")):
        refuse("PUBLIC_INVENTORY_SYMLINK")
    for excluded in inventory["excluded_internal_paths"]:
        if (source_root / excluded).exists():
            refuse("PUBLIC_INVENTORY_BOUNDARY")
    for relative in inventory["negative_fixture_paths"]:
        planted = source_root / relative
        if planted.exists():
            shutil.rmtree(planted)
    return inventory, archive_paths


def run(command, *, cwd, env=None, timeout=300):
    merged = dict(os.environ)
    if env:
        merged.update(env)
    try:
        return subprocess.run(
            [str(item) for item in command],
            cwd=cwd,
            env=merged,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        refuse("SCANNER_EXECUTION")


def atomic_json(path, value, *, release_mode):
    raw = json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
    if release_mode and any(marker in raw for marker in ("/Users/", "/home/", "Documents/it")):
        refuse("REPORT_PRIVATE_DATA")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and (path.is_symlink() or not path.is_file()):
        refuse("OUTPUT_PATH")
    handle, temporary = tempfile.mkstemp(prefix=".audit-", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            stream.write(raw)
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def text_files(root):
    for path in sorted(root.rglob("*")):
        if not regular(path) or path.stat().st_size > 2 * 1024 * 1024:
            continue
        try:
            yield path, path.read_text(encoding="utf-8")
        except UnicodeError:
            continue


def blockers_for_slices(blockers):
    result = []
    for slice_id in ("security", "privacy", "legal", "namespace", "claims"):
        owned = [item["code"] for item in blockers if item["slice"] == slice_id]
        result.append(
            {
                "id": slice_id,
                "status": "BLOCKED" if owned else "PASS",
                "blockers": owned,
            }
        )
    return result


def semgrep_matches(target, pattern):
    environment = {
        "SEMGREP_SEND_METRICS": "off",
        "SEMGREP_ENABLE_VERSION_CHECK": "0",
    }
    result = run(
        [
            "semgrep",
            "scan",
            "--quiet",
            "--metrics=off",
            "--json",
            "--lang=rust",
            "--pattern",
            pattern,
            str(target),
        ],
        cwd=ROOT,
        env=environment,
    )
    if result.returncode not in (0, 1):
        refuse("SEMGREP_ERROR")
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError:
        refuse("SEMGREP_ERROR")
    return len(payload.get("results", []))


def trusted_official_schema_finding(item, target, trusted_schema):
    if not regular(trusted_schema) or sha256(trusted_schema) != OFFICIAL_SPDX_SCHEMA_SHA256:
        return False
    if item.get("RuleID") != "generic-api-key":
        return False
    reported = str(item.get("File", "")).replace("\\", "/")
    expected = OFFICIAL_SPDX_SCHEMA.as_posix()
    return reported.endswith(expected) or (target == trusted_schema and reported.endswith(trusted_schema.name))


def gitleaks_findings(target, report_path, *, archive_depth=0, trusted_schema=None):
    command = [
        "gitleaks",
        "dir",
        "--no-banner",
        "--no-color",
        "--redact=100",
        "--report-format=json",
        "--report-path",
        str(report_path),
    ]
    if archive_depth:
        command += ["--max-archive-depth", str(archive_depth)]
    command.append(str(target))
    result = run(command, cwd=ROOT)
    if result.returncode not in (0, 1):
        refuse("GITLEAKS_ERROR")
    if result.returncode == 0:
        return {"raw": 0, "accepted": 0, "filtered": 0}
    payload = load_json(report_path)
    if not isinstance(payload, list):
        refuse("GITLEAKS_ERROR")
    filtered = 0
    accepted = 0
    for item in payload:
        if trusted_schema is not None and trusted_official_schema_finding(item, target, trusted_schema):
            filtered += 1
        else:
            accepted += 1
    return {"raw": len(payload), "accepted": accepted, "filtered": filtered}


def intentional_empty_assignment_finding(item, source_root):
    if item.get("code") != 1007:
        return False
    try:
        path = Path(item["file"])
        if not path.is_absolute():
            path = source_root / path
        path = path.resolve()
        path.relative_to(source_root.resolve())
        line_number = int(item["line"])
        column = int(item["column"])
        body = path.read_text(encoding="utf-8").splitlines()[line_number - 1]
    except (KeyError, ValueError, OSError, UnicodeError, IndexError):
        return False
    if column < 2 or column > len(body) or body[column - 2] != "=" or not body[column - 1].isspace():
        return False
    assignments_only = re.fullmatch(r"\s*(?:[A-Za-z_][A-Za-z0-9_]*=[^\s]*\s*)+", body) is not None
    cdpath_clear = re.search(r"\bCDPATH=\s+cd(?:\s|$)", body) is not None
    return assignments_only or cdpath_clear


def shellcheck_findings(shell_files, source_root):
    result = run(
        ["shellcheck", "--format=json", "--severity=warning", *shell_files],
        cwd=source_root,
    )
    if result.returncode not in (0, 1):
        refuse("SHELLCHECK_ERROR")
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError:
        refuse("SHELLCHECK_ERROR")
    if not isinstance(payload, list):
        refuse("SHELLCHECK_ERROR")
    filtered = sum(1 for item in payload if intentional_empty_assignment_finding(item, source_root))
    return {
        "raw": len(payload),
        "accepted": len(payload) - filtered,
        "filtered_intentional_empty_assignments": filtered,
    }


def claim_blockers(claim_root, add):
    bodies = []
    for path, body in text_files(claim_root):
        bodies.append((path, body))
    joined = "\n".join(body for _, body in bodies)
    if UNSUPPORTED_CLAIM.search(joined):
        add("claims", "UNSUPPORTED_RELEASE_CLAIM", "unsupported maturity, safety or SLSA-level wording")

    security = next((body for path, body in bodies if path.name == "SECURITY.md"), "")
    if not (
        re.search(r"private\s+vulnerability\s+report", security, re.IGNORECASE)
        and re.search(r"do\s+not\s+open\s+a\s+public\s+issue", security, re.IGNORECASE)
    ):
        add("claims", "SECURITY_REPORTING_ROUTE_MISSING", "private reporting route is absent or public disclosure is allowed")
    if not re.search(r"(?:no\s+bounty|does\s+not\s+offer\s+(?:a\s+)?bounty)", security, re.IGNORECASE):
        add("claims", "BOUNTY_STATUS_MISSING", "current no-bounty status is not explicit")
    if "NOT_YET_AVAILABLE" in security:
        add("claims", "SECURITY_REPORTING_ROUTE_UNVERIFIED", "security policy honestly records that no verified private reporting route exists")

    threat_terms = {
        "protected assets": r"protected\s+assets?",
        "trust boundaries": r"trust\s+boundar",
        "attacker inputs": r"attacker[- ]controlled\s+inputs?",
        "home contamination": r"HOME|parent\s+contamination",
        "path escape": r"path|symlink",
        "malicious context": r"malicious\s+context",
        "provider drift": r"(?:adapter|provider)\s+drift",
        "private leakage": r"(?:secret|private)[- ](?:data\s+)?leak",
        "receipt forgery": r"(?:digest|receipt)\s+forgery",
        "mitigations": r"mitigations?",
        "residual risks": r"residual\s+risks?",
    }
    missing = [name for name, pattern in threat_terms.items() if not re.search(pattern, joined, re.IGNORECASE)]
    if missing:
        add("claims", "THREAT_MODEL_INCOMPLETE", f"missing {len(missing)} required threat-model classes")

    codeowners = next((body for path, body in bodies if path.name == "CODEOWNERS"), "")
    if not codeowners:
        add("claims", "PROTECTED_PATH_OWNERSHIP_MISSING", "CODEOWNERS evidence is absent")
    elif "NOT_YET_ENFORCEABLE" in codeowners:
        add("claims", "PROTECTED_PATH_OWNERSHIP_UNVERIFIED", "CODEOWNERS contract has no verified repository-owner mapping")
    elif not any(line.strip() and not line.lstrip().startswith("#") for line in codeowners.splitlines()):
        add("claims", "PROTECTED_PATH_OWNERSHIP_MISSING", "CODEOWNERS has no active protected-path mapping")
    if TELEMETRY_OR_BACKEND.search(joined):
        add("claims", "TELEMETRY_OR_BACKEND_PRESENT", "analytics, telemetry, backend or network client marker detected")
    if CYRILLIC.search(joined):
        add("claims", "PUBLIC_PROSE_NOT_ENGLISH", "non-English public prose detected")
    if not any(path.name.lower() == "readme.md" for path, _ in bodies):
        add("claims", "PUBLIC_RELEASE_COPY_MISSING", "owner-accepted public README/copy is absent")


def fixture_audit(options):
    fixture_root = Path(options["--fixture-root"]).resolve()
    output = Path(options["--output"]).resolve()
    if not fixture_root.is_dir() or fixture_root.is_symlink():
        refuse("FIXTURE_ROOT")
    required = {"privacy", "unsafe", "legal", "advisory", "namespace", "claims"}
    if {path.name for path in fixture_root.iterdir() if path.is_dir()} != required:
        refuse("FIXTURE_CLASSES")

    blockers = []
    seen = set()

    def add(slice_id, code, evidence):
        if code in seen:
            return
        seen.add(code)
        blockers.append({"slice": slice_id, "code": code, "evidence": evidence})

    with tempfile.TemporaryDirectory(prefix="taskseal-p08-audit-fixture-") as temporary:
        temporary = Path(temporary)
        privacy_findings = gitleaks_findings(fixture_root / "privacy", temporary / "gitleaks.json")
        if privacy_findings["accepted"]:
            add("privacy", "PRIVATE_SECRET", "Gitleaks detected the planted synthetic secret")
        if any(PRIVATE_PATH.search(body) for _, body in text_files(fixture_root / "privacy")):
            add("privacy", "PRIVATE_ABSOLUTE_PATH", "absolute HOME-path fixture detected")

        unsafe = fixture_root / "unsafe" / "unsafe.rs"
        if semgrep_matches(unsafe, 'std::process::Command::new("sh")'):
            add("security", "UNSAFE_PROCESS_SHELL", "Semgrep detected untrusted shell execution")
        if semgrep_matches(unsafe, 'std::fs::remove_dir_all("/")'):
            add("security", "UNSAFE_FILESYSTEM_ROOT_DELETE", "Semgrep detected root deletion")

        official_schema = ROOT / OFFICIAL_SPDX_SCHEMA
        official_schema_findings = gitleaks_findings(
            official_schema,
            temporary / "official-schema-gitleaks.json",
            trusted_schema=official_schema,
        )
        fixture_shell_files = sorted(
            path
            for base in (ROOT / "scripts/release", ROOT / "packaging")
            if base.exists()
            for path in base.rglob("*.sh")
        )
        fixture_shellcheck = shellcheck_findings(fixture_shell_files, ROOT)
        if official_schema_findings != {"raw": 2, "accepted": 0, "filtered": 2}:
            refuse("OFFICIAL_SCHEMA_FALSE_POSITIVE_BOUNDARY")
        if fixture_shellcheck != {"raw": 26, "accepted": 0, "filtered_intentional_empty_assignments": 26}:
            refuse("SHELLCHECK_FALSE_POSITIVE_BOUNDARY")

    notice = (fixture_root / "legal" / "NOTICE").read_text(encoding="utf-8")
    if "Third-party dependency notices" not in notice:
        add("legal", "DEPENDENCY_LICENSE_NOTICE_MISSING", "dependency notice fixture is incomplete")
    advisory = load_json(fixture_root / "advisory" / "osv.json")
    if any(item.get("vulns") for item in advisory.get("results", [])):
        add("security", "DEPENDENCY_ADVISORY", "unresolved advisory fixture detected")
    namespace = load_json(fixture_root / "namespace" / "ownership.json")
    if not all(namespace.get(key) is True for key in ("approved_public_name", "repository_owned", "package_owned", "binary_names_owned")):
        add("namespace", "NAMESPACE_OWNERSHIP_UNPROVEN", "public name/repository/package/binary ownership is incomplete")
    claim_blockers(fixture_root / "claims", add)

    blockers.sort(key=lambda item: (item["slice"], item["code"]))
    expected = {
        "PRIVATE_SECRET",
        "PRIVATE_ABSOLUTE_PATH",
        "UNSAFE_PROCESS_SHELL",
        "UNSAFE_FILESYSTEM_ROOT_DELETE",
        "DEPENDENCY_LICENSE_NOTICE_MISSING",
        "DEPENDENCY_ADVISORY",
        "NAMESPACE_OWNERSHIP_UNPROVEN",
        "UNSUPPORTED_RELEASE_CLAIM",
        "SECURITY_REPORTING_ROUTE_MISSING",
        "BOUNTY_STATUS_MISSING",
        "THREAT_MODEL_INCOMPLETE",
        "PROTECTED_PATH_OWNERSHIP_MISSING",
        "TELEMETRY_OR_BACKEND_PRESENT",
        "PUBLIC_PROSE_NOT_ENGLISH",
        "PUBLIC_RELEASE_COPY_MISSING",
    }
    if seen != expected:
        refuse("FIXTURE_DETECTION_INCOMPLETE")
    report = {
        "schema_version": "taskseal.p08.release-audit.v1",
        "mode": "FIXTURE_DETECTION",
        "result": "PASS",
        "acceptance_id": "ACC-P08-T7",
        "evidence_id": "EVD-P08-T7-RED-GREEN",
        "classes": sorted(required),
        "slices": blockers_for_slices(blockers),
        "blockers_detected": blockers,
        "scanner_corrections": {
            "official_schema": official_schema_findings,
            "shellcheck": fixture_shellcheck,
        },
        "promotion_eligible": False,
    }
    atomic_json(output, report, release_mode=False)
    print(f"P08_TASK7_FIXTURE_DETECTIONS_PASS classes={len(required)} blockers={len(blockers)}")


def safe_extract(artifact, destination):
    try:
        with tarfile.open(artifact, "r:gz") as archive:
            members = archive.getmembers()
            if not members:
                refuse("ARTIFACT_EMPTY")
            for member in members:
                parts = Path(member.name).parts
                if member.name.startswith("/") or ".." in parts or member.issym() or member.islnk():
                    refuse("ARTIFACT_PATH")
            archive.extractall(destination)
    except (OSError, tarfile.TarError):
        refuse("ARTIFACT_EXTRACT")


def release_audit(options):
    repo = Path(options["--repo"]).resolve()
    artifact = Path(options["--artifact"]).resolve()
    output = Path(options["--output"]).resolve()
    tool_root = Path(options["--tool-root"]).resolve()
    cargo_home = Path(options["--cargo-home"]).resolve()
    recorded_at = os.environ.get("TASKSEAL_AUDIT_RECORDED_AT", "")
    if repo != ROOT or output != ROOT / "reports/release/audit.json":
        refuse("RELEASE_PATH")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", recorded_at):
        refuse("RECORDED_AT")
    if not regular(artifact) or not tool_root.is_dir() or not cargo_home.is_dir():
        refuse("RELEASE_INPUT")
    for tool in ("cargo-audit", "cargo-deny", "cargo-cyclonedx"):
        if not os.access(tool_root / "bin" / tool, os.X_OK):
            refuse("STANDARD_TOOL_MISSING")

    candidate = load_json(repo / "reports/release/candidate.json")
    expected_sha = candidate.get("release_artifact", {}).get("sha256")
    artifact_sha = sha256(artifact)
    if not re.fullmatch(r"[0-9a-f]{64}", str(expected_sha)) or artifact_sha != expected_sha:
        refuse("ARTIFACT_DIGEST")
    head_result = run(["git", "rev-parse", "HEAD"], cwd=repo)
    if head_result.returncode or not re.fullmatch(r"[0-9a-f]{40}\n", head_result.stdout):
        refuse("HEAD")
    scan_head = head_result.stdout.strip()

    blockers = []
    checks = []
    seen = set()

    def add(slice_id, code, evidence):
        if code in seen:
            return
        seen.add(code)
        blockers.append({"slice": slice_id, "code": code, "evidence": evidence})

    def check(slice_id, name, status, evidence):
        checks.append({"slice": slice_id, "name": name, "status": status, "evidence": evidence})

    open_risks = [
        item
        for item in candidate.get("known_risks", [])
        if item.get("status") not in {"PASS", "CLOSED", "RESOLVED"}
    ]
    if open_risks:
        add("security", "UPSTREAM_RELEASE_RISK_OPEN", f"candidate dossier has {len(open_risks)} unresolved bound risks")
    check("security", "candidate-known-risks", "PASS" if not open_risks else "BLOCKED", f"open={len(open_risks)}")

    standard_env = {
        "PATH": str(tool_root / "bin") + os.pathsep + os.environ.get("PATH", ""),
        "CARGO_HOME": str(cargo_home),
        "CARGO_TERM_COLOR": "never",
        "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_CONFIG_SYSTEM": "/dev/null",
        "GIT_TERMINAL_PROMPT": "0",
        "SEMGREP_SEND_METRICS": "off",
        "SEMGREP_ENABLE_VERSION_CHECK": "0",
    }

    with tempfile.TemporaryDirectory(prefix="taskseal-p08-release-audit-") as temporary:
        temporary = Path(temporary)
        source_archive = temporary / "source.tar"
        source_root = temporary / "source"
        source_root.mkdir()
        public_inventory, public_paths = build_public_source_projection(repo, source_archive, source_root)

        source_leaks = gitleaks_findings(
            source_root,
            temporary / "source-gitleaks.json",
            trusted_schema=source_root / OFFICIAL_SPDX_SCHEMA,
        )
        artifact_leaks = gitleaks_findings(artifact, temporary / "artifact-gitleaks.json", archive_depth=2)
        if source_leaks["accepted"] or artifact_leaks["accepted"]:
            add("privacy", "PRIVATE_SECRET", "Gitleaks found redacted source or artifact findings")
        check(
            "privacy",
            "gitleaks-source-artifact",
            "PASS" if not (source_leaks["accepted"] or artifact_leaks["accepted"]) else "BLOCKED",
            f"source={source_leaks['accepted']};artifact={artifact_leaks['accepted']};filtered_official={source_leaks['filtered']}",
        )

        artifact_root = temporary / "artifact"
        artifact_root.mkdir()
        safe_extract(artifact, artifact_root)
        private_hits = 0
        for path, body in text_files(source_root):
            relative = path.relative_to(source_root)
            if not relative.parts or relative.parts[0] != "tests":
                private_hits += int(bool(PRIVATE_PATH.search(body)))
        private_hits += sum(1 for _, body in text_files(artifact_root) if PRIVATE_PATH.search(body))
        binaries = [path for path in artifact_root.rglob("*") if regular(path) and path.parent.name == "bin"]
        for binary in binaries:
            strings = run(["strings", binary], cwd=repo)
            if strings.returncode == 0 and PRIVATE_PATH.search(strings.stdout):
                private_hits += 1
        if private_hits:
            add("privacy", "PRIVATE_ABSOLUTE_PATH", "absolute HOME path found in public text or artifact strings")
        check(
            "privacy",
            "private-path-source-artifact",
            "PASS" if not private_hits else "BLOCKED",
            f"findings={private_hits};public_paths={len(public_paths)};negative_fixtures={len(public_inventory['negative_fixture_paths'])}",
        )

        verified = run(["python3", repo / "packaging/verify-artifact.py", artifact], cwd=repo)
        if verified.returncode:
            add("security", "ARTIFACT_STRUCTURE_INVALID", "canonical artifact verifier refused")
        check("security", "artifact-structure", "PASS" if not verified.returncode else "BLOCKED", "packaging/verify-artifact.py")

        unsafe_process = semgrep_matches(source_root / "src", 'std::process::Command::new("sh")') if (source_root / "src").exists() else 0
        unsafe_delete = semgrep_matches(source_root / "src", 'std::fs::remove_dir_all("/")') if (source_root / "src").exists() else 0
        if unsafe_process:
            add("security", "UNSAFE_PROCESS_SHELL", "Semgrep found shell command construction in product source")
        if unsafe_delete:
            add("security", "UNSAFE_FILESYSTEM_ROOT_DELETE", "Semgrep found literal root deletion in product source")
        check("security", "semgrep-unsafe-boundaries", "PASS" if not (unsafe_process or unsafe_delete) else "BLOCKED", f"process={unsafe_process};filesystem={unsafe_delete}")

        shell_files = sorted(
            path
            for base in (source_root / "scripts/release", source_root / "packaging")
            if base.exists()
            for path in base.rglob("*.sh")
        )
        shellcheck = shellcheck_findings(shell_files, source_root) if shell_files else {"raw": 0, "accepted": 0, "filtered_intentional_empty_assignments": 0}
        if shellcheck["accepted"]:
            add("security", "SHELL_SAFETY_LINT", "ShellCheck warning-or-higher finding in release or packaging scripts")
        check(
            "security",
            "shellcheck-release-packaging",
            "PASS" if not shellcheck["accepted"] else "BLOCKED",
            f"files={len(shell_files)};raw={shellcheck['raw']};accepted={shellcheck['accepted']};filtered_empty_assignments={shellcheck['filtered_intentional_empty_assignments']}",
        )

        audit_db = cargo_home / "rustsec-advisory-db"
        if not audit_db.is_dir():
            fetched = run(
                ["git", "clone", "--depth", "1", "https://github.com/RustSec/advisory-db.git", audit_db],
                cwd=temporary,
                env=standard_env,
                timeout=600,
            )
            if fetched.returncode:
                refuse("RUSTSEC_DB_FETCH")
        audit_db_head = run(["git", "-C", audit_db, "rev-parse", "HEAD"], cwd=repo, env=standard_env)
        if audit_db_head.returncode or not re.fullmatch(r"[0-9a-f]{40}\n", audit_db_head.stdout):
            refuse("RUSTSEC_DB_HEAD")
        audit = run(
            ["cargo", "audit", "--no-fetch", "--db", audit_db, "--json", "--file", "Cargo.lock"],
            cwd=repo,
            env=standard_env,
            timeout=600,
        )
        try:
            audit_json = json.loads(audit.stdout)
            vulnerabilities = audit_json.get("vulnerabilities", {}).get("list", [])
        except json.JSONDecodeError:
            refuse("CARGO_AUDIT_OUTPUT")
        if vulnerabilities:
            add("security", "DEPENDENCY_ADVISORY", f"cargo-audit unresolved advisories={len(vulnerabilities)}")
        elif audit.returncode:
            refuse("CARGO_AUDIT_ERROR")
        check(
            "security",
            "cargo-audit-rustsec",
            "PASS" if not vulnerabilities else "BLOCKED",
            f"db={audit_db_head.stdout.strip()};advisories={len(vulnerabilities)}",
        )

        license_policy = repo / "packaging/license-policy.toml"
        deny_licenses = run(
            ["cargo", "deny", "--config", license_policy, "--locked", "--offline", "check", "licenses"],
            cwd=repo,
            env=standard_env,
            timeout=600,
        )
        if deny_licenses.returncode:
            add("legal", "DEPENDENCY_LICENSE_POLICY", "cargo-deny license policy refused the locked graph")
        check("legal", "cargo-deny-licenses", "PASS" if not deny_licenses.returncode else "BLOCKED", "locked graph + accepted P07 policy")
        deny_sources = run(
            ["cargo", "deny", "--config", license_policy, "--locked", "--offline", "check", "sources"],
            cwd=repo,
            env=standard_env,
            timeout=600,
        )
        if deny_sources.returncode:
            add("security", "DEPENDENCY_SOURCE_POLICY", "cargo-deny source policy refused the locked graph")
        check("security", "cargo-deny-sources", "PASS" if not deny_sources.returncode else "BLOCKED", "locked dependency graph")

        cyclonedx = run(
            [
                "cargo",
                "cyclonedx",
                "--manifest-path",
                source_root / "Cargo.toml",
                "--format",
                "json",
                "--spec-version",
                "1.5",
                "--license-strict",
                "--override-filename",
                "standard-sbom",
            ],
            cwd=source_root,
            env=standard_env,
            timeout=600,
        )
        standard_sbom = source_root / "standard-sbom.json"
        if cyclonedx.returncode or not regular(standard_sbom):
            refuse("CARGO_CYCLONEDX_ERROR")
        try:
            standard_sbom_json = load_json(standard_sbom)
        except AuditError:
            refuse("CARGO_CYCLONEDX_ERROR")
        standard_components = standard_sbom_json.get("components", [])
        if not standard_components:
            add("security", "SBOM_COMPONENTS_MISSING", "cargo-cyclonedx emitted no dependency components")
        check("security", "cargo-cyclonedx-inventory", "PASS" if standard_components else "BLOCKED", f"components={len(standard_components)}")

        supply_chain = temporary / "supply-chain"
        generated = run(
            [
                repo / "packaging/supply-chain/generate.sh",
                "generate",
                "--artifact",
                artifact,
                "--source-commit",
                candidate.get("source_artifact", {}).get("commit", ""),
                "--target",
                "aarch64-apple-darwin",
                "--builder-id",
                "local://taskseal/p07",
                "--output",
                supply_chain,
            ],
            cwd=repo,
            env=standard_env,
            timeout=600,
        )
        if generated.returncode:
            refuse("SUPPLY_CHAIN_GENERATION")
        verified_metadata = run(
            [repo / "packaging/supply-chain/generate.sh", "verify", "--artifact", artifact, "--output", supply_chain],
            cwd=repo,
            env=standard_env,
            timeout=600,
        )
        if verified_metadata.returncode:
            add("security", "SBOM_PROVENANCE_INVALID", "official-schema and exact-subject verification refused")
        check("security", "cyclonedx-slsa-subject-binding", "PASS" if not verified_metadata.returncode else "BLOCKED", "CycloneDX 1.7 + SLSA v1.2 profile")

        bound_sbom = load_json(supply_chain / "sbom.cdx.json")
        components = bound_sbom.get("components", [])
        notice_path = next((path for path in artifact_root.rglob("NOTICE") if regular(path)), None)
        notice = notice_path.read_text(encoding="utf-8") if notice_path else ""
        missing_notices = [item.get("name") for item in components if item.get("name") and item.get("name") not in notice]
        if missing_notices:
            add("legal", "DEPENDENCY_LICENSE_NOTICE_MISSING", f"artifact notice omits {len(missing_notices)} SBOM components")
        check("legal", "artifact-dependency-notices", "PASS" if not missing_notices else "BLOCKED", f"missing={len(missing_notices)}")

        binary = next((path for path in binaries if path.name == "taskseal"), None)
        if binary is None:
            refuse("ARTIFACT_BINARY")
        signing = run(
            [repo / "packaging/signing/verify.sh", "--artifact", binary, "--platform", "macos", "--claim", "unsigned"],
            cwd=repo,
        )
        if signing.returncode:
            add("security", "SIGNING_STATE_INVALID", "platform verifier disagrees with the unsigned preview label")
        check("security", "macos-signing-label", "PASS" if not signing.returncode else "BLOCKED", "unsigned NOT_QUALIFIED preview")

        claim_root = temporary / "public-claims"
        claim_root.mkdir()
        for relative in ("SECURITY.md", "GOVERNANCE.md", "CHANGELOG.md", "README.md", "docs", "site", ".github/CODEOWNERS", "CODEOWNERS"):
            source = source_root / relative
            destination = claim_root / relative
            if source.is_dir():
                shutil.copytree(source, destination)
            elif regular(source):
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
        claim_blockers(claim_root, add)
        if any(TELEMETRY_OR_BACKEND.search(body) for _, body in text_files(source_root / "src")):
            add("claims", "TELEMETRY_OR_BACKEND_PRESENT", "analytics, telemetry, backend or network client marker detected in product source")

        namespace_record = source_root / "reports/release/namespace-ownership.json"
        if not regular(namespace_record):
            add("namespace", "NAMESPACE_OWNERSHIP_UNPROVEN", "owner-approved public name and dated registry ownership record are absent")
            add("claims", "SECURITY_REPORTING_ROUTE_UNVERIFIED", "repository-bound private reporting route cannot resolve before public repository ownership")
            check("namespace", "official-registry-ownership", "NOT_RUN", "blocked before query by absent owner-approved public name")
        else:
            namespace = load_json(namespace_record)
            if not all(namespace.get(key) is True for key in ("approved_public_name", "repository_owned", "package_owned", "binary_names_owned")):
                add("namespace", "NAMESPACE_OWNERSHIP_UNPROVEN", "namespace record is incomplete")
            check("namespace", "official-registry-ownership", "PASS" if "NAMESPACE_OWNERSHIP_UNPROVEN" not in seen else "BLOCKED", "dated owner record")

    blockers.sort(key=lambda item: (item["slice"], item["code"]))
    checks.sort(key=lambda item: (item["slice"], item["name"]))
    report = {
        "schema_version": "taskseal.p08.release-audit.v1",
        "mode": "EXACT_PRIVATE_CANDIDATE",
        "task": 7,
        "acceptance_id": "ACC-P08-T7",
        "evidence_id": "EVD-P08-T7-FINAL",
        "recorded_at": recorded_at,
        "scan_head": scan_head,
        "subject": {
            "artifact_sha256": artifact_sha,
            "candidate_commit": candidate.get("candidate_commit"),
            "requested_release_state": candidate.get("requested_release_state"),
            "qualification": candidate.get("qualification"),
        },
        "result": "PASS" if not blockers else "BLOCKED",
        "slices": blockers_for_slices(blockers),
        "checks": checks,
        "blockers": blockers,
        "claims": {
            "digest_integrity": "VERIFIED",
            "signed_authenticity": "NOT_PRESENT",
            "slsa_level": "NOT_CLAIMED",
            "promotion_eligible": False,
        },
        "external_queries": [
            {"registry": "RustSec advisory database", "access": "read-only", "recorded_at": recorded_at},
            {"registry": "crates.io package index", "access": "read-only", "recorded_at": recorded_at},
        ],
        "standard_tools": [
            {"name": "cargo-audit", "version": "0.22.2", "sha256": sha256(tool_root / "bin/cargo-audit")},
            {"name": "cargo-deny", "version": "0.20.2", "sha256": sha256(tool_root / "bin/cargo-deny")},
            {"name": "cargo-cyclonedx", "version": "0.5.9", "sha256": sha256(tool_root / "bin/cargo-cyclonedx")},
            {"name": "gitleaks", "version": "8.30.1"},
            {"name": "semgrep", "version": "1.164.0", "metrics": "off"},
            {"name": "shellcheck", "version": "0.11.0"},
        ],
        "forbidden_actions": {
            "publication": False,
            "upload": False,
            "external_contact": False,
            "provider_requests": False,
            "credentials": False,
            "keychain": False,
            "main_mutation": False,
            "integration": False,
        },
    }
    atomic_json(output, report, release_mode=True)
    marker = "PASS" if not blockers else "BLOCKED"
    print(f"P08_TASK7_AUDIT_{marker} slices={len(report['slices'])} blockers={len(blockers)}")
    if blockers:
        raise SystemExit(3)


try:
    mode, options = parse_options(sys.argv[2:])
    if mode == "fixture":
        fixture_audit(options)
    else:
        release_audit(options)
except AuditError as error:
    print("P08_TASK7_AUDIT_REFUSED:" + str(error), file=sys.stderr)
    raise SystemExit(2)
PY
