#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
exec python3 - "$root" "$@" <<'PY'
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import uuid
import jsonschema

ROOT = Path(sys.argv[1])
POLICY_PATH = ROOT / "packaging/supply-chain/policy.toml"
EXPECTED_FILES = {"checksums.sha256", "sbom.cdx.json", "provenance.intoto.json"}
HEX40 = re.compile(r"[0-9a-f]{40}\Z")
SECRET = re.compile(r"(?:ghp_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16})")
SCHEMA_ROOT = ROOT / "packaging/supply-chain/schemas"

class Refused(Exception):
    pass

def refuse(message):
    raise Refused(message)

def parse_cli(values):
    if not values or values[0] not in {"generate", "verify"}:
        refuse("USAGE")
    mode, rest = values[0], values[1:]
    if len(rest) % 2:
        refuse("USAGE")
    options = {}
    for key, value in zip(rest[0::2], rest[1::2]):
        if not key.startswith("--") or key in options:
            refuse("USAGE")
        options[key] = value
    required = {"--artifact", "--output"} | ({"--source-commit", "--target", "--builder-id"} if mode == "generate" else set())
    if set(options) != required:
        refuse("USAGE")
    return mode, options

def regular(path):
    return path.is_file() and not path.is_symlink()

def digest(path):
    if not regular(path):
        refuse("ARTIFACT")
    return hashlib.sha256(path.read_bytes()).hexdigest()

def load_json(path):
    def unique(items):
        value = {}
        for key, item in items:
            if key in value:
                refuse("DUPLICATE_JSON_KEY")
            value[key] = item
        return value
    if not regular(path):
        refuse("METADATA_FILE")
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    except (OSError, UnicodeError, json.JSONDecodeError):
        refuse("JSON")

def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    path.chmod(0o644)

def validate_official_schemas():
    manifest = load_json(SCHEMA_ROOT / "manifest.json")
    if manifest.get("schema_version") != "taskseal.p07.official-schemas.v1" or not isinstance(manifest.get("sources"), list):
        refuse("SCHEMA_MANIFEST")
    for item in manifest["sources"]:
        if set(item) != {"path", "sha256", "url"}:
            refuse("SCHEMA_MANIFEST")
        path = SCHEMA_ROOT / item["path"]
        if not regular(path) or digest(path) != item["sha256"]:
            refuse("SCHEMA_DIGEST")

def validate_sbom_schema(sbom):
    schema_path = SCHEMA_ROOT / "cyclonedx-1.7/bom-1.7.schema.json"
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        resolver = jsonschema.RefResolver(base_uri=schema_path.parent.as_uri() + "/", referrer=schema)
        jsonschema.Draft7Validator(schema, resolver=resolver).validate(sbom)
    except (OSError, UnicodeError, json.JSONDecodeError, jsonschema.ValidationError, jsonschema.SchemaError, jsonschema.RefResolutionError):
        refuse("SBOM_SCHEMA")

def validate_slsa_profile(provenance, rules, artifact_name, artifact_sha):
    if set(provenance) != {"_type", "subject", "predicateType", "predicate"}:
        refuse("PROVENANCE_PROFILE")
    if provenance["_type"] != rules["statement_type"] or provenance["predicateType"] != rules["slsa_predicate"]:
        refuse("PROVENANCE_PROFILE")
    if provenance["subject"] != [{"name": artifact_name, "digest": {"sha256": artifact_sha}}]:
        refuse("PROVENANCE_SUBJECT")
    predicate = provenance["predicate"]
    if not isinstance(predicate, dict) or set(predicate) != {"buildDefinition", "runDetails"}:
        refuse("PROVENANCE_PROFILE")
    build, run = predicate["buildDefinition"], predicate["runDetails"]
    if not isinstance(build, dict) or set(build) != {"buildType", "externalParameters", "internalParameters", "resolvedDependencies"}:
        refuse("PROVENANCE_PROFILE")
    external, internal = build["externalParameters"], build["internalParameters"]
    if build["buildType"] != "https://taskseal.invalid/build-types/cargo-release/v1" or not isinstance(external, dict) or set(external) != {"source_commit", "target", "qualification"} or external["qualification"] != rules["qualification"]:
        refuse("PROVENANCE_CLAIM")
    if not isinstance(internal, dict) or internal != {"locked": True, "network": False}:
        refuse("PROVENANCE_CLAIM")
    dependencies = build["resolvedDependencies"]
    source_commit = external["source_commit"]
    if dependencies != [{"uri": "git+local://taskseal@" + source_commit, "digest": {"gitCommit": source_commit}}]:
        refuse("PROVENANCE_CLAIM")
    if not isinstance(run, dict) or set(run) != {"builder", "metadata"} or set(run["builder"]) != {"id"} or run["builder"]["id"] not in rules["allowed_builders"] or set(run["metadata"]) != {"invocationId"} or run["metadata"]["invocationId"] != "urn:taskseal:p07:" + artifact_sha:
        refuse("PROVENANCE_CLAIM")

def policy():
    try:
        value = tomllib.loads(POLICY_PATH.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError):
        refuse("POLICY")
    if set(value) != {"schema_version", "cyclonedx_spec", "slsa_predicate", "statement_type", "qualification", "allowed_builders"}:
        refuse("POLICY")
    if value["schema_version"] != "taskseal.p07.supply-chain-policy.v1" or value["qualification"] != "NOT_QUALIFIED":
        refuse("POLICY")
    return value

def cargo_components():
    environment = dict(os.environ)
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=ROOT, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    if result.returncode:
        refuse("CARGO_METADATA")
    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError:
        refuse("CARGO_METADATA")
    packages = []
    root_package = None
    for package in metadata.get("packages", []):
        item = {"type": "library", "name": package.get("name"), "version": package.get("version"), "bom-ref": f"pkg:cargo/{package.get('name')}@{package.get('version')}", "purl": f"pkg:cargo/{package.get('name')}@{package.get('version')}", "licenses": [{"expression": package.get("license")}]}
        if not all(isinstance(item[key], str) and item[key] for key in ("name", "version")) or not item["licenses"][0]["expression"]:
            refuse("COMPONENT_LICENSE")
        if package.get("name") == "taskseal":
            root_package = item
        else:
            packages.append(item)
    if root_package is None or not packages:
        refuse("COMPONENTS")
    return root_package, sorted(packages, key=lambda item: (item["name"], item["version"]))

def generate(options, rules):
    artifact = Path(options["--artifact"]).resolve()
    output = Path(options["--output"]).resolve()
    source_commit, target, builder = options["--source-commit"], options["--target"], options["--builder-id"]
    if not HEX40.fullmatch(source_commit) or not re.fullmatch(r"[a-z0-9_-]+(?:-[a-z0-9_-]+)+", target) or builder not in rules["allowed_builders"]:
        refuse("PARAMETERS")
    artifact_sha = digest(artifact)
    if output.exists() and (output.is_symlink() or not output.is_dir()):
        refuse("OUTPUT")
    output.mkdir(parents=True, exist_ok=True)
    for child in output.iterdir():
        if child.name not in EXPECTED_FILES or child.is_symlink() or not child.is_file():
            refuse("OUTPUT")
        child.unlink()
    root_package, components = cargo_components()
    root_component = {**root_package, "type": "application", "hashes": [{"alg": "SHA-256", "content": artifact_sha}]}
    serial = str(uuid.UUID(artifact_sha[:32]))
    sbom = {"bomFormat": "CycloneDX", "specVersion": rules["cyclonedx_spec"], "serialNumber": "urn:uuid:" + serial, "version": 1, "metadata": {"component": root_component, "properties": [{"name": "taskseal:qualification", "value": rules["qualification"]}]}, "components": components}
    provenance = {"_type": rules["statement_type"], "subject": [{"name": artifact.name, "digest": {"sha256": artifact_sha}}], "predicateType": rules["slsa_predicate"], "predicate": {"buildDefinition": {"buildType": "https://taskseal.invalid/build-types/cargo-release/v1", "externalParameters": {"source_commit": source_commit, "target": target, "qualification": rules["qualification"]}, "internalParameters": {"locked": True, "network": False}, "resolvedDependencies": [{"uri": "git+local://taskseal@" + source_commit, "digest": {"gitCommit": source_commit}}]}, "runDetails": {"builder": {"id": builder}, "metadata": {"invocationId": "urn:taskseal:p07:" + artifact_sha}}}}
    (output / "checksums.sha256").write_text(f"{artifact_sha}  {artifact.name}\n", encoding="utf-8")
    write_json(output / "sbom.cdx.json", sbom)
    write_json(output / "provenance.intoto.json", provenance)
    verify({"--artifact": str(artifact), "--output": str(output)}, rules, emit=False)

def verify(options, rules, emit=True):
    artifact = Path(options["--artifact"]).resolve()
    output = Path(options["--output"]).resolve()
    if not output.is_dir() or output.is_symlink() or {path.name for path in output.iterdir()} != EXPECTED_FILES or any(not regular(path) for path in output.iterdir()):
        refuse("OUTPUT")
    artifact_sha = digest(artifact)
    if (output / "checksums.sha256").read_text(encoding="utf-8") != f"{artifact_sha}  {artifact.name}\n":
        refuse("CHECKSUM")
    sbom = load_json(output / "sbom.cdx.json")
    provenance = load_json(output / "provenance.intoto.json")
    validate_official_schemas()
    validate_sbom_schema(sbom)
    if sbom.get("bomFormat") != "CycloneDX" or sbom.get("specVersion") != rules["cyclonedx_spec"] or sbom.get("version") != 1:
        refuse("SBOM_PROFILE")
    root_component = sbom.get("metadata", {}).get("component", {})
    if root_component.get("name") != "taskseal" or {item.get("content") for item in root_component.get("hashes", []) if item.get("alg") == "SHA-256"} != {artifact_sha}:
        refuse("SBOM_SUBJECT")
    components = sbom.get("components")
    if not isinstance(components, list) or not components or any(not item.get("name") or not item.get("version") or not item.get("licenses") or not item["licenses"][0].get("expression") for item in components):
        refuse("SBOM_COMPONENTS")
    validate_slsa_profile(provenance, rules, artifact.name, artifact_sha)
    combined = b"".join((output / name).read_bytes() for name in sorted(EXPECTED_FILES))
    try:
        text = combined.decode("utf-8")
    except UnicodeDecodeError:
        refuse("PRIVATE_DATA")
    if SECRET.search(text) or "/Users/" in text or "/home/" in text:
        refuse("PRIVATE_DATA")
    if emit:
        print("P07_SUPPLY_CHAIN_VERIFY_PASS qualification=NOT_QUALIFIED")

try:
    mode, options = parse_cli(sys.argv[2:])
    rules = policy()
    if mode == "generate":
        generate(options, rules)
        print("P07_SUPPLY_CHAIN_GENERATE_PASS qualification=NOT_QUALIFIED")
    else:
        verify(options, rules)
except Refused as error:
    print("P07_SUPPLY_CHAIN_REFUSED:" + str(error), file=sys.stderr)
    raise SystemExit(1)
PY
