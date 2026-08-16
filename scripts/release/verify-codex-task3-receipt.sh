#!/bin/sh
# Verify the immutable implementation subject and receipt-only seal at any descendant tip.
set -eu

refuse() {
  echo "P08_T3_RECEIPT_REFUSED:$1" >&2
  exit 1
}

repo= receipt_file= receipt_path= tip_arg=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo) repo=${2:?}; shift 2 ;;
    --receipt-file) receipt_file=${2:?}; shift 2 ;;
    --receipt-path) receipt_path=${2:?}; shift 2 ;;
    --tip) tip_arg=${2:?}; shift 2 ;;
    *) refuse USAGE ;;
  esac
done
[ -n "$repo" ] && [ -n "$receipt_file" ] && [ -n "$receipt_path" ] && [ -n "$tip_arg" ] || refuse USAGE
repo=$(CDPATH= cd -- "$repo" && pwd -P) || refuse REPOSITORY_MISSING
[ -f "$receipt_file" ] || refuse RECEIPT_MISSING
case "$receipt_path" in
  reports/gates/p08/task-3.json) ;;
  *) refuse RECEIPT_PATH_INVALID ;;
esac
tip=$(git -C "$repo" rev-parse "$tip_arg^{commit}" 2>/dev/null) || refuse TIP_INVALID
ruby -rjson -e '
  x=JSON.parse(File.read(ARGV[0]))
  abort unless x["schema_version"] == "taskseal.p08.task-receipt.v1" && x["plan_id"] == "P08" && x["task"] == 3 && x["acceptance_id"] == "ACC-P08-T3" && x.dig("subject_sha256", "algorithm") == "sha256 of sorted path, tab, sha256, newline records"
' "$receipt_file" >/dev/null 2>&1 || refuse RECEIPT_IDENTITY_INVALID

receipt_value() {
  ruby -rjson -e 'x=JSON.parse(File.read(ARGV[0])); value=ARGV[1].split(".").reduce(x){|memo,key| memo.fetch(key)}; abort unless value.is_a?(String); puts value' "$receipt_file" "$1" 2>/dev/null
}

implementation_head=$(receipt_value binding.implementation_head) || refuse RECEIPT_SCHEMA_INVALID
receipt_parent=$(receipt_value binding.receipt_commit_parent) || refuse RECEIPT_SCHEMA_INVALID
seal_role=$(receipt_value binding.receipt_seal_role) || refuse RECEIPT_SCHEMA_INVALID
[ "$implementation_head" = "$receipt_parent" ] || refuse RECEIPT_PARENT_BINDING_MISMATCH
case "$seal_role" in
  receipt-only-child|replacement-receipt-only-child) ;;
  *) refuse RECEIPT_SEAL_ROLE_INVALID ;;
esac
git -C "$repo" cat-file -e "$implementation_head^{commit}" 2>/dev/null || refuse IMPLEMENTATION_HEAD_MISSING
git -C "$repo" merge-base --is-ancestor "$implementation_head" "$tip" 2>/dev/null || refuse IMPLEMENTATION_NOT_ANCESTOR

subjects_tmp=$(mktemp "${TMPDIR:-/tmp}/taskseal-p08-t3-subjects.XXXXXX")
commits_tmp=$(mktemp "${TMPDIR:-/tmp}/taskseal-p08-t3-commits.XXXXXX")
trap 'rm -f "$subjects_tmp" "$commits_tmp"' EXIT HUP INT TERM
ruby -rjson -e '
  x=JSON.parse(File.read(ARGV[0])); subjects=x.fetch("subjects")
  abort unless subjects.is_a?(Hash) && !subjects.empty?
  subjects.keys.sort.each do |path|
    digest=subjects[path]
    abort "SUBJECT_SHA256_FORMAT_INVALID" unless path.match?(/\A(?:reports\/release|schemas\/release|scripts\/release|tests\/release)\/[A-Za-z0-9._\/-]+\z/) && digest.is_a?(String) && digest.match?(/\A[0-9a-f]{64}\z/)
    puts "#{path}\t#{digest}"
  end
' "$receipt_file" >"$subjects_tmp" 2>"$subjects_tmp.error" || {
  if grep -q SUBJECT_SHA256_FORMAT_INVALID "$subjects_tmp.error"; then rm -f "$subjects_tmp.error"; refuse SUBJECT_SHA256_FORMAT_INVALID; fi
  rm -f "$subjects_tmp.error"; refuse RECEIPT_SCHEMA_INVALID
}
rm -f "$subjects_tmp.error"

recomputed_tmp=$subjects_tmp.recomputed
: >"$recomputed_tmp"
while IFS="	" read -r subject_path expected_sha; do
  git -C "$repo" cat-file -e "$implementation_head:$subject_path" 2>/dev/null || refuse SUBJECT_MISSING
  actual_sha=$(git -C "$repo" show "$implementation_head:$subject_path" | shasum -a 256 | awk '{print $1}')
  [ "$actual_sha" = "$expected_sha" ] || refuse SUBJECT_SHA256_MISMATCH
  printf '%s\t%s\n' "$subject_path" "$actual_sha" >>"$recomputed_tmp"
done <"$subjects_tmp"
aggregate=$(shasum -a 256 "$recomputed_tmp" | awk '{print $1}')
expected_aggregate=$(receipt_value subject_sha256.value) || refuse RECEIPT_SCHEMA_INVALID
[ "$aggregate" = "$expected_aggregate" ] || refuse SUBJECT_AGGREGATE_MISMATCH

receipt_sha=$(shasum -a 256 "$receipt_file" | awk '{print $1}')
git -C "$repo" log --format=%H "$implementation_head..$tip" -- "$receipt_path" | while IFS= read -r commit; do
  git -C "$repo" cat-file -e "$commit:$receipt_path" 2>/dev/null || continue
  candidate_sha=$(git -C "$repo" show "$commit:$receipt_path" | shasum -a 256 | awk '{print $1}')
  [ "$candidate_sha" = "$receipt_sha" ] && printf '%s\n' "$commit"
done >"$commits_tmp"
[ "$(wc -l <"$commits_tmp" | tr -d ' ')" = 1 ] || refuse RECEIPT_COMMIT_NOT_UNIQUE
seal_commit=$(cat "$commits_tmp")
parent_line=$(git -C "$repo" rev-list --parents -n 1 "$seal_commit")
parent_fields=$(printf '%s\n' "$parent_line" | awk '{print NF}')
seal_parent=$(printf '%s\n' "$parent_line" | awk 'NF == 2 {print $2}')
[ "$parent_fields" -eq 2 ] && [ "$seal_parent" = "$implementation_head" ] || refuse RECEIPT_NOT_DIRECT_CHILD
changed=$(git -C "$repo" diff-tree --no-commit-id --name-only -r "$seal_commit")
[ "$changed" = "$receipt_path" ] || refuse RECEIPT_CHILD_NOT_RECEIPT_ONLY
git -C "$repo" merge-base --is-ancestor "$seal_commit" "$tip" 2>/dev/null || refuse RECEIPT_NOT_DURABLE_AT_TIP

rm -f "$recomputed_tmp"
trap - EXIT HUP INT TERM
rm -f "$subjects_tmp" "$commits_tmp"
echo "P08_T3_RECEIPT_DURABILITY_PASS implementation=$implementation_head seal=$seal_commit tip=$tip"
