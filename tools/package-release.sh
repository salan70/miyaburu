#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
  echo '使い方: package-release.sh <target> <binary> <出力先>' >&2
  exit 2
fi
target=$1
binary=$2
out=$3
archive="miyaburu-${target}.tar.gz"
mkdir -p "$out"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
install -m 755 "$binary" "$tmp/miyaburu"
if [ ! -f assets/vendor/mermaid.LICENSE ]; then
  echo 'エラー: assets/vendor/mermaid.LICENSE がありません' >&2
  exit 1
fi
cp assets/vendor/mermaid.LICENSE "$tmp/mermaid.LICENSE"
tar -czf "$out/$archive" -C "$tmp" miyaburu mermaid.LICENSE
if command -v sha256sum >/dev/null 2>&1; then
  hash=$(sha256sum "$out/$archive" | awk '{print $1}')
else
  hash=$(shasum -a 256 "$out/$archive" | awk '{print $1}')
fi
printf '%s  %s\n' "$hash" "$archive" > "$out/$archive.sha256"
