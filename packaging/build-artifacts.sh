#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
out_dir=${1:-"$root/target/artifacts"}
target=${TASKSEAL_TARGET:-}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
commit=${TASKSEAL_SOURCE_COMMIT:-$(git -C "$root" rev-parse HEAD)}
toolchain=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$root/rust-toolchain.toml" | head -1)
[[ $commit =~ ^[0-9a-f]{40}$|^[0-9a-f]{64}$ ]] || { echo "invalid source commit" >&2; exit 2; }
[[ "$commit" == "$(git -C "$root" rev-parse HEAD)" ]] || { echo "source commit must equal checked-out HEAD" >&2; exit 2; }
git -C "$root" diff --quiet || { echo "tracked source changes prevent exact binding" >&2; exit 2; }
[[ -f "$root/LICENSE" ]] || { echo "LICENSE is required" >&2; exit 2; }
[[ -n "$version" && -n "$toolchain" ]] || { echo "version/toolchain metadata missing" >&2; exit 2; }
if [[ -n "$target" ]]; then
  cargo_args=(--target "$target"); target_label=$target; binary="$root/target/$target/release/taskseal"
else
  cargo_args=(); target_label=$(rustc -vV | sed -n 's/^host: //p'); binary="$root/target/release/taskseal"
fi
[[ $target_label =~ ^[A-Za-z0-9._-]+$ ]] || { echo "unsafe target label" >&2; exit 2; }
cargo build --locked --release --bin taskseal "${cargo_args[@]}"
[[ -x "$binary" ]] || { echo "built taskseal binary not found" >&2; exit 2; }
mkdir -p "$out_dir"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-artifact.XXXXXX"); trap 'rm -rf "$tmp"' EXIT
stage="$tmp/taskseal-v$version-$target_label"
mkdir -p "$stage/bin" "$stage/share/doc/taskseal"
install -m 0755 "$binary" "$stage/bin/taskseal"
install -m 0755 "$binary" "$stage/bin/tseal"
install -m 0644 "$root/LICENSE" "$stage/LICENSE"
printf 'TaskSeal v%s local unsigned preview artifact.\nCanonical executable: bin/taskseal\nCompatibility executable: bin/tseal (byte-identical copy)\n' "$version" > "$stage/NOTICE"
printf 'version=%s\nsource_commit=%s\nrust_toolchain=%s\ntarget=%s\nqualification=NOT_QUALIFIED\nsigning=unsigned-preview-only\ndependencies=cargo-lock\n' "$version" "$commit" "$toolchain" "$target_label" > "$stage/VERSION"
install -m 0644 "$root/CHANGELOG.md" "$stage/share/doc/taskseal/CHANGELOG.md"
archive="$out_dir/taskseal-v$version-$target_label.tar.gz"
python3 - "$stage" "$archive" <<'PY'
import gzip, hashlib, os, sys, tarfile
stage, archive = sys.argv[1:]
entries = []
for base, dirs, files in os.walk(stage):
    dirs.sort(); files.sort()
    for name in dirs + files:
        path = os.path.join(base, name)
        rel = os.path.relpath(path, os.path.dirname(stage)).replace(os.sep, "/")
        entries.append((rel, path))
with open(archive, "wb") as raw:
    with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for rel, path in entries:
                info = tar.gettarinfo(path, arcname=rel)
                info.uid = info.gid = 0; info.uname = info.gname = ""; info.mtime = 0
                info.mode = 0o755 if rel.endswith("/bin/taskseal") or rel.endswith("/bin/tseal") else 0o644
                if info.isfile():
                    with open(path, "rb") as data: tar.addfile(info, data)
                else: tar.addfile(info)
print(hashlib.sha256(open(archive, "rb").read()).hexdigest())
PY
echo "ARTIFACT=$archive"
echo "QUALIFICATION=NOT_QUALIFIED"
