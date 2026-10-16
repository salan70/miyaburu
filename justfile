# 開発ツールはflake.nixで管理し、実行コマンドはjustfileを正本とする。
# direnvで環境が有効なら「just <レシピ>」で実行できる。

# 引数なしでレシピ一覧を表示。
default:
    @just --list

# Rustの書式を整える。
fmt:
    cargo fmt

# Rustの書式を検査する。
fmt-check:
    cargo fmt --check

# Rustのテストを実行する。
test:
    cargo test

# Rustコードを静的に検査する。
lint:
    cargo clippy --all-targets -- -D warnings

# Rustをビルドする。
build:
    cargo build

# 任意のDocBridgeサブコマンドを実行する（例: just docbridge graph）。Rust製CLIの実行時依存ではない。
docbridge *ARGS:
    ./tools/docbridge.sh {{ARGS}}

# DocBridgeでコードと仕様書のリンクを検証する。
docs-check:
    ./tools/docbridge.sh check

# DocBridgeで未連携の候補を調べる。
docs-audit:
    ./tools/docbridge.sh check --audit

# インストーラーの導入動作を検査する。
install-check:
    sh tools/test-install.sh

# リリース配布物を検査する。
release-check:
    sh tools/test-release.sh
    actionlint .github/workflows/ci.yml .github/workflows/release.yml
    shellcheck install.sh tools/check-release-version.sh tools/package-release.sh tools/test-install.sh tools/test-release.sh

# 全体の基本チェック。
check: fmt-check test docs-check install-check release-check

# ツールチェーンのバージョンを確認する。
versions:
    rustc --version
    cargo --version
    node --version
    pnpm --version
    just --version
    @echo "docbridge $(tr -d '[:space:]' < .docbridge-version)"
