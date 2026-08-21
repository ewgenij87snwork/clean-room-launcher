#!/bin/sh
set -eu

exec python3 - "$@" <<'PY'
import json
import os
from pathlib import Path
import subprocess
import sys

class Refused(Exception):
    pass

def refuse(code):
    raise Refused(code)

def parse(values):
    if len(values) % 2:
        refuse("USAGE")
    options = {}
    for key, value in zip(values[0::2], values[1::2]):
        if not key.startswith("--") or key in options:
            refuse("USAGE")
        options[key] = value
    allowed = {"--artifact", "--platform", "--claim", "--identity", "--tool-root"}
    if not {"--artifact", "--platform", "--claim"}.issubset(options) or not set(options).issubset(allowed):
        refuse("USAGE")
    if options["--claim"] == "unsigned" and "--identity" in options:
        refuse("IDENTITY")
    if options["--claim"] != "unsigned" and not options.get("--identity"):
        refuse("IDENTITY")
    return options

def run(command):
    return subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)

def tool(path):
    if not path.is_file() or path.is_symlink() or not os.access(path, os.X_OK):
        refuse("TOOL_UNAVAILABLE")
    return str(path)

def macos(artifact, claim, identity, codesign_path, spctl_path):
    codesign = tool(codesign_path)
    spctl = tool(spctl_path)
    checked = run([codesign, "--verify", "--deep", "--strict", "--verbose=2", str(artifact)])
    checked_text = (checked.stdout + "\n" + checked.stderr).lower()
    if checked.returncode:
        if "not signed at all" not in checked_text:
            refuse("SIGNATURE_INVALID")
        observed = "unsigned"
        mechanism = "absent"
    else:
        details = run([codesign, "-dv", "--verbose=4", str(artifact)])
        detail_text = details.stdout + "\n" + details.stderr
        if details.returncode:
            refuse("SIGNATURE_INVALID")
        if "signature=adhoc" in detail_text.lower():
            if claim != "unsigned":
                refuse("ADHOC_SIGNATURE")
            return "unsigned", "adhoc"
        authorities = [line.split("=", 1)[1].strip() for line in detail_text.splitlines() if line.startswith("Authority=")]
        if identity not in authorities:
            refuse("IDENTITY_MISMATCH")
        assessment = run([spctl, "--assess", "--type", "execute", "--verbose=4", str(artifact)])
        notarized = assessment.returncode == 0 and "source=Notarized Developer ID" in (assessment.stdout + assessment.stderr)
        observed = "signed+notarized" if notarized else "signed"
        mechanism = "authenticated"
    if observed != claim:
        refuse("CLAIM_MISMATCH")
    return observed, mechanism

def windows(artifact, claim, identity, tools):
    if claim == "signed+notarized":
        refuse("CLAIM_UNSUPPORTED")
    powershell = tool(tools / "pwsh")
    expression = "$s=Get-AuthenticodeSignature -LiteralPath $args[0]; [pscustomobject]@{Status=[string]$s.Status;Subject=[string]$s.SignerCertificate.Subject;Timestamp=[string]$s.TimeStamperCertificate.Subject}|ConvertTo-Json -Compress"
    result = run([powershell, "-NoProfile", "-NonInteractive", "-Command", expression, str(artifact)])
    if result.returncode:
        refuse("SIGNATURE_INVALID")
    try:
        evidence = json.loads(result.stdout)
    except json.JSONDecodeError:
        refuse("SIGNATURE_INVALID")
    if evidence.get("Status") == "NotSigned":
        observed = "unsigned"
        mechanism = "absent"
    elif evidence.get("Status") == "Valid" and evidence.get("Subject") == identity:
        observed = "signed"
        mechanism = "authenticated"
    elif evidence.get("Status") == "Valid":
        refuse("IDENTITY_MISMATCH")
    else:
        refuse("SIGNATURE_INVALID")
    if observed != claim:
        refuse("CLAIM_MISMATCH")
    return observed, mechanism

try:
    options = parse(sys.argv[1:])
    artifact = Path(options["--artifact"]).resolve()
    if not artifact.is_file() or artifact.is_symlink():
        refuse("ARTIFACT")
    platform, claim = options["--platform"], options["--claim"]
    if platform not in {"macos", "windows"} or claim not in {"unsigned", "signed", "signed+notarized"}:
        refuse("USAGE")
    fixture = options.get("--tool-root") is not None
    if fixture:
        if os.environ.get("TASKSEAL_SIGNING_FIXTURE") != "1":
            refuse("FIXTURE_REFUSED")
        tools = Path(options["--tool-root"]).resolve()
        evidence_class = "fixture"
    else:
        tools = Path("/usr/bin")
        evidence_class = "platform"
    if platform == "macos":
        codesign_path = tools / "codesign" if fixture else Path("/usr/bin/codesign")
        spctl_path = tools / "spctl" if fixture else Path("/usr/sbin/spctl")
        observed, mechanism = macos(artifact, claim, options.get("--identity"), codesign_path, spctl_path)
    else:
        observed, mechanism = windows(artifact, claim, options.get("--identity"), tools)
    suffix = " signature=adhoc" if mechanism == "adhoc" else ""
    print(f"P07_SIGNING_VERIFY_PASS state={observed} qualification=NOT_QUALIFIED evidence={evidence_class}{suffix}")
except Refused as error:
    print("P07_SIGNING_REFUSED:" + str(error), file=sys.stderr)
    raise SystemExit(1)
PY
