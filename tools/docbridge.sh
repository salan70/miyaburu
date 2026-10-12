#!/usr/bin/env bash
# DocBridge を単一のバージョン定義（.docbridge-version）で呼び出す。
# justfile と CI はすべてこの版を参照する。npm と package.json は使わない。
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
version="$(tr -d '[:space:]' < .docbridge-version)"
exec pnpm dlx "docbridge@${version}" "$@"
