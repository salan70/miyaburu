# 開発ツールはflake.nixで管理し、実行コマンドはjustfileを正本とする。
# direnvで環境が有効なら「just <レシピ>」で実行できる。

# 引数なしでレシピ一覧を表示。
default:
    @just --list

# DocBridgeの依存関係をpackage-lock.jsonどおりに導入する。Rust製CLIの実行時依存ではない。
setup:
    npm ci --ignore-scripts --no-audit --no-fund

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

# DocBridgeでコードと仕様書のリンクを検証する。
docs-check:
    npm run docs:check

# DocBridgeで未連携の候補を調べる。
docs-audit:
    npm run docs:audit

# 全体の基本チェック。事前にjust setupを一度実行すること。
check: fmt-check test docs-check

# ツールチェーンのバージョンを確認する。
versions:
    rustc --version
    cargo --version
    node --version
    npm --version
    just --version
