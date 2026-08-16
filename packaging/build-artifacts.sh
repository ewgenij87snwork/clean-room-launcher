#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
out_dir=${1:-"$root/target/artifacts"}
target=${TASKSEAL_TARGET:-}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
commit=${TASKSEAL_SOURCE_COMMIT:-}
if [[ -z "$commit" && -d "$root/.git" ]]; then commit=$(git -C "$root" rev-parse HEAD); fi
toolchain=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$root/rust-toolchain.toml" | head -1)
[[ $commit =~ ^[0-9a-f]{40}$|^[0-9a-f]{64}$ ]] || { echo "invalid source commit" >&2; exit 2; }
if [[ -d "$root/.git" ]]; then
  [[ "$commit" == "$(git -C "$root" rev-parse HEAD)" ]] || { echo "source commit must equal checked-out HEAD" >&2; exit 2; }
  git -C "$root" diff --quiet || { echo "tracked source changes prevent exact binding" >&2; exit 2; }
fi
[[ -f "$root/LICENSE" ]] || { echo "LICENSE is required" >&2; exit 2; }
[[ -n "$version" && -n "$toolchain" ]] || { echo "version/toolchain metadata missing" >&2; exit 2; }
if [[ -n "$target" ]]; then
  cargo_args=(--target "$target"); target_label=$target
else
  cargo_args=(); target_label=$(rustc -vV | sed -n 's/^host: //p')
fi
[[ $target_label =~ ^[A-Za-z0-9._-]+$ ]] || { echo "unsafe target label" >&2; exit 2; }
target_dir=${CARGO_TARGET_DIR:-"$root/target"}
binary="$target_dir/${target:+$target/}release/clroom"
build_cargo_home=${CARGO_HOME:-"${HOME:?HOME is required}/.cargo"}
if [[ $build_cargo_home != /* ]]; then build_cargo_home="$root/$build_cargo_home"; fi
build_cargo_home=$(cd "$build_cargo_home" && pwd -P)
[[ -z ${RUSTFLAGS:-} && -z ${CARGO_ENCODED_RUSTFLAGS:-} ]] || { echo "external rust flags prevent exact path remapping" >&2; exit 2; }
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$root=/workspace/taskseal"$'\x1f'"--remap-path-prefix=$build_cargo_home=/cargo"
export CARGO_NET_OFFLINE=${CARGO_NET_OFFLINE:-true} LC_ALL=C TZ=UTC SOURCE_DATE_EPOCH=0
cargo build --locked --release --bin clroom "${cargo_args[@]}"
[[ -x "$binary" ]] || { echo "built clroom binary not found" >&2; exit 2; }
mkdir -p "$out_dir"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/clean-room-launcher-artifact.XXXXXX"); trap 'rm -rf "$tmp"' EXIT
stage="$tmp/clean-room-launcher-v$version-$target_label"
mkdir -p "$stage/bin" "$stage/share/doc/clean-room-launcher"
install -m 0755 "$binary" "$stage/bin/clroom"
install -m 0644 "$root/LICENSE" "$stage/LICENSE"
python3 "$root/packaging/generate-notice.py" --output "$stage/NOTICE"
script_sha=$(shasum -a 256 "$root/packaging/build-artifacts.sh" | awk '{print $1}')
notice_generator_sha=$(shasum -a 256 "$root/packaging/generate-notice.py" | awk '{print $1}')
license_policy_sha=$(shasum -a 256 "$root/packaging/license-policy.toml" | awk '{print $1}')
notice_policy_sha=$(shasum -a 256 "$root/packaging/dependency-notice-policy.json" | awk '{print $1}')
cargo_lock_sha=$(shasum -a 256 "$root/Cargo.lock" | awk '{print $1}')
rustc_version=$(rustc --version)
cargo_version=$(cargo --version)
python_version=$(python3 --version)
printf 'version=%s\nsource_commit=%s\nrust_toolchain=%s\ntarget=%s\nrustc=%s\ncargo=%s\npython=%s\npackaging_script_sha256=%s\nnotice_generator_sha256=%s\nlicense_policy_sha256=%s\nnotice_policy_sha256=%s\ncargo_lock_sha256=%s\narchive_profile=normalized-local-toolchain\nqualification=NOT_QUALIFIED\nsigning=unsigned-preview-only\ndependencies=cargo-lock\n' "$version" "$commit" "$toolchain" "$target_label" "$rustc_version" "$cargo_version" "$python_version" "$script_sha" "$notice_generator_sha" "$license_policy_sha" "$notice_policy_sha" "$cargo_lock_sha" > "$stage/VERSION"
install -m 0644 "$root/CHANGELOG.md" "$stage/share/doc/clean-room-launcher/CHANGELOG.md"
archive="$out_dir/clean-room-launcher-v$version-$target_label.tar.gz"
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
entries.sort(key=lambda item: (item[0].count("/"), item[0]))
with open(archive, "wb") as raw:
    with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for rel, path in entries:
                info = tar.gettarinfo(path, arcname=rel)
                info.uid = info.gid = 0; info.uname = info.gname = ""; info.mtime = 0
                info.mode = 0o755 if info.isdir() or rel.endswith("/bin/clroom") else 0o644
                if info.isfile():
                    with open(path, "rb") as data: tar.addfile(info, data)
                else: tar.addfile(info)
print(hashlib.sha256(open(archive, "rb").read()).hexdigest())
PY
echo "ARTIFACT=$archive"
echo "QUALIFICATION=NOT_QUALIFIED"
