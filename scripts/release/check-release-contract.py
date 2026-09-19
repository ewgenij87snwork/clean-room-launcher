#!/usr/bin/env python3
import argparse, fnmatch, json, os, subprocess, sys, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "schemas/release/release-contract-v1.json"

def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()

def load_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))

def matches(path, pattern):
    return fnmatch.fnmatchcase(path, pattern) or Path(path).match(pattern)

def latest_published_release(repository):
    req = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/releases/latest",
        headers={"Accept":"application/vnd.github+json","User-Agent":"clroom-release-contract-v1"},
    )
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=20) as response:
        data = json.load(response)
    if data.get("draft") or data.get("prerelease"):
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:PUBLISHED_BASELINE_NOT_STABLE")
    return data["tag_name"], data.get("published_at")

def classify(paths, contract):
    result = {}
    unknown = []
    for path in paths:
        domains = [d["id"] for d in contract["domains"] if any(matches(path, p) for p in d["patterns"])]
        if not domains:
            unknown.append(path)
        result[path] = domains
    return result, unknown

def ensure_ref(ref):
    try:
        run("git","rev-parse","--verify",ref)
    except subprocess.CalledProcessError:
        raise SystemExit(f"RELEASE_CONTRACT_BLOCKED:MISSING_GIT_REF:{ref}")

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--review", default=None)
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY","y-sor/clean-room-launcher"))
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--require-head-reviewed", action="store_true")
    args=parser.parse_args()

    contract=load_json(CONTRACT)
    if contract.get("schema_version")!="clroom.release-contract.v1":
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:CONTRACT_SCHEMA")

    if args.self_test:
        sample=["src/cli/mod.rs","Cargo.lock",".github/workflows/ci.yml","scripts/release/readiness.sh","README.md","tests/cli/info.rs"]
        classified, unknown=classify(sample,contract)
        if unknown or any(not classified[p] for p in sample):
            raise SystemExit("RELEASE_CONTRACT_SELF_TEST_FAIL")
        if classify(["totally-new-root.bin"],contract)[1] != ["totally-new-root.bin"]:
            raise SystemExit("RELEASE_CONTRACT_SELF_TEST_FAIL_UNKNOWN")
        assurance = contract["release_assurance_only_patterns"]
        if any(matches("src/cli/mod.rs", pattern) for pattern in assurance):
            raise SystemExit("RELEASE_CONTRACT_SELF_TEST_FAIL_ASSURANCE_TOO_BROAD")
        if not any(matches("scripts/release/check-release-contract.py", pattern) for pattern in assurance):
            raise SystemExit("RELEASE_CONTRACT_SELF_TEST_FAIL_ASSURANCE_MISSING")
        print("RELEASE_CONTRACT_SELF_TEST_PASS")
        return

    version = __import__("tomllib").loads((ROOT/"Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    review_path = Path(args.review or ROOT/f"reports/release/v{version}-review.json")
    review=load_json(review_path)
    if review.get("schema_version")!="clroom.release-review.v1":
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:REVIEW_SCHEMA")
    if review.get("release") != f"v{version}":
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:REVIEW_VERSION")

    latest_tag, published_at = latest_published_release(args.repository)
    if review.get("baseline_release") != latest_tag:
        raise SystemExit(f"RELEASE_CONTRACT_BLOCKED:BASELINE_DRIFT:{latest_tag}")
    ensure_ref(latest_tag)
    base_commit=run("git","rev-list","-n","1",latest_tag)

    reviewed=review.get("reviewed_through_commit","")
    ensure_ref(reviewed)
    if subprocess.call(["git","merge-base","--is-ancestor",base_commit,reviewed],cwd=ROOT)!=0:
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:REVIEWED_COMMIT_NOT_AFTER_BASELINE")
    if subprocess.call(["git","merge-base","--is-ancestor",reviewed,"HEAD"],cwd=ROOT)!=0:
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:REVIEWED_COMMIT_NOT_ANCESTOR")

    changed=run("git","diff","--name-only",f"{base_commit}..{reviewed}").splitlines()
    classified, unknown=classify(changed,contract)
    if unknown:
        print("\n".join(f"UNCLASSIFIED_RELEASE_DELTA:{p}" for p in unknown),file=sys.stderr)
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:UNCLASSIFIED_RELEASE_DELTA")

    changed_domains=sorted({d for ds in classified.values() for d in ds})
    dispositions=review.get("domain_dispositions",{})
    domain_allowed=set(contract["domain_satisfying_dispositions"])
    near_miss_allowed=set(contract["near_miss_dispositions"])
    for domain in changed_domains:
        item=dispositions.get(domain)
        if not item or item.get("decision") not in domain_allowed or not item.get("evidence"):
            raise SystemExit(f"RELEASE_CONTRACT_BLOCKED:DOMAIN_DISPOSITION:{domain}")

    for item in review.get("near_misses",[]):
        if item.get("decision") not in near_miss_allowed:
            raise SystemExit("RELEASE_CONTRACT_BLOCKED:NEAR_MISS_DISPOSITION")
        if item.get("decision")=="CONTRACT_EXPAND" and not item.get("durable_control"):
            raise SystemExit("RELEASE_CONTRACT_BLOCKED:NEAR_MISS_CONTROL")

    head = run("git","rev-parse","HEAD")
    if args.require_head_reviewed and reviewed != head:
        raise SystemExit(
            f"RELEASE_CONTRACT_BLOCKED:HEAD_NOT_REVIEWED:reviewed={reviewed}:head={head}"
        )

    tail=run("git","diff","--name-only",f"{reviewed}..HEAD").splitlines()
    allowed_tail=contract["release_assurance_only_patterns"]
    bad_tail=[p for p in tail if not any(matches(p,pat) for pat in allowed_tail)]
    if bad_tail:
        print("\n".join(f"POST_REVIEW_PRODUCT_CHANGE:{p}" for p in bad_tail),file=sys.stderr)
        raise SystemExit("RELEASE_CONTRACT_BLOCKED:POST_REVIEW_PRODUCT_CHANGE")

    if args.report:
        print(f"RELEASE={review['release']}")
        print(f"PUBLISHED_BASELINE={latest_tag}")
        print(f"PUBLISHED_AT={published_at}")
        print(f"BASE_COMMIT={base_commit}")
        print(f"REVIEWED_THROUGH={reviewed}")
        print(f"HEAD={head}")
        print("CHANGED_DOMAINS="+",".join(changed_domains))
        print(f"CHANGED_FILES={len(changed)}")
        print("=== COMMITS SINCE PUBLISHED RELEASE ===")
        print(run("git","log","--oneline",f"{base_commit}..HEAD"))
        print("=== FILES SINCE PUBLISHED RELEASE ===")
        for p in changed:
            print(f"{p}\t{','.join(classified[p])}")
        if tail:
            print("=== RELEASE-ASSURANCE TAIL ===")
            print("\n".join(tail))
        print("=== ARTIFACT CAPABILITY GATES ===")
        for gate in review.get("artifact_capability_gates",[]):
            print(f"{gate['phase']}\t{gate['id']}\t{gate['requirement']}")
    print(f"RELEASE_CONTRACT_PASS release={review['release']} baseline={latest_tag} reviewed_through={reviewed}")

if __name__=="__main__":
    main()
