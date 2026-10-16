#!/bin/sh
set -eu
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
cd "$root"

assert() {
  if ! "$@"; then
    echo "検証失敗: $*" >&2
    exit 1
  fi
}

assert_archive_has() {
  if ! tar -tzf "$1" | grep -qx "$2"; then
    echo "検証失敗: tar内に $2 がありません ($1)" >&2
    exit 1
  fi
}

assert_fails() {
  if "$@" >"$tmp/assert.out" 2>&1; then
    echo "検証失敗: 成功してはいけないコマンド: $*" >&2
    exit 1
  fi
  if [ ! -s "$tmp/assert.out" ]; then
    echo "検証失敗: 失敗理由が出力されませんでした: $*" >&2
    exit 1
  fi
}

cat > "$tmp/package-valid.toml" <<'EOF'
version = "9.9.9"
[package]
version = "1.2.3"
EOF
assert sh tools/check-release-version.sh v1.2.3 "$tmp/package-valid.toml"
assert_fails sh tools/check-release-version.sh v1.2.4 "$tmp/package-valid.toml"
assert_fails sh tools/check-release-version.sh v1.2.3-rc.1 "$tmp/package-valid.toml"
cat > "$tmp/package-missing.toml" <<'EOF'
version = "1.2.3"
[package]
name = "fixture"
EOF
assert_fails sh tools/check-release-version.sh v1.2.3 "$tmp/package-missing.toml"

printf '#!/bin/sh\necho miyaburu 0.1.0\n' > "$tmp/fixture"
chmod +x "$tmp/fixture"
for target in aarch64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  out="$tmp/$target"
  archive="miyaburu-${target}.tar.gz"
  sh tools/package-release.sh "$target" "$tmp/fixture" "$out"
  assert test -f "$out/$archive"
  assert test -f "$out/$archive.sha256"
  assert_archive_has "$out/$archive" miyaburu
  assert_archive_has "$out/$archive" mermaid.LICENSE
  assert test "$(tar -tzf "$out/$archive" | grep -vc '/$')" -eq 2
  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$out/$archive" | awk '{print $1}')
  else
    actual=$(shasum -a 256 "$out/$archive" | awk '{print $1}')
  fi
  expected=$(awk 'NR == 1 {print $1}' "$out/$archive.sha256")
  assert test "${#expected}" -eq 64
  assert test "$actual" = "$expected"
  assert test "$(cat "$out/$archive.sha256")" = "$expected  $archive"
  mkdir "$tmp/extract-$target"
  tar -xzf "$out/$archive" -C "$tmp/extract-$target" miyaburu
  assert "$tmp/extract-$target/miyaburu"
done
echo '版照合・配布物・3 targetのchecksum・tar内容・抽出後実行: OK'
