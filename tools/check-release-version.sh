#!/bin/sh
set -eu

error() {
  echo "エラー: $*" >&2
  exit 1
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  error '使い方: check-release-version.sh TAG [Cargo.tomlのパス]'
fi
tag=$1
manifest=${2:-Cargo.toml}
if ! printf '%s\n' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  error 'タグは v数字.数字.数字 の形式で指定してください'
fi

version=$(awk '
  /^\[package\]$/ { package=1; next }
  /^\[/ { package=0 }
  package && /^[[:space:]]*version[[:space:]]*=/ {
    line=$0
    sub(/^[^=]*=[[:space:]]*/, "", line)
    if (line !~ /^"[^\"]+"([[:space:]]*(#.*)?)?$/) exit 2
    sub(/^"/, "", line)
    sub(/".*/, "", line)
    print line
    found=1
    exit
  }
  END { if (!found) exit 1 }
' "$manifest") || error 'Cargo.tomlの[package]節に有効なversionがありません'
[ -n "$version" ] || error 'Cargo.tomlの[package]節に有効なversionがありません'
[ "v$version" = "$tag" ] || error 'タグとCargo.tomlのversionが一致しません'
