# エージェント向けの指示

このリポジトリは、Rust製のMarkdownプレビューCLI **みやぶる（miyaburu）** を実装します。
現在の契約は、公開している `README.md` と `docs/specs/` に書いてあります。
実装していない機能を、実装済みとして扱わないでください。

## 挙動を変える前に

- 先に `docs/specs/` の該当する節を読みます。
- デザインの原則と再利用できるトークンは [uiux-numa](https://github.com/salan70/uiux-numa) を参照します。カタログ全体を持ち込むことはしません。
- 利用者が選んだRustの構成と、ローカルかつオフラインで動く方針を保ちます。
- 見た目の方向は、白を基調としたデジタルな Editorial × Soft Pop です。文章には LINE Seed JP、コードには HackGen を使います。
- 文書とコメントは日本語で書きます。Rustの識別子とDocBridgeのリンク記法は英語のままにします。

## DocBridge（自分たちで使って確かめる）

DocBridgeは**開発時だけ**のツールです。Rustの実行ファイルの実行時依存にしてはいけません。

- 公開しているRustの宣言に `/// @doc docs/specs/<ファイル>.md#<節>` を付けます。
  対応するMarkdownの見出しのすぐ上に `<!-- @code src/<ファイル>.rs#<シンボル> -->` を置き、相互に参照させます。
- 節の指定は、見出しの文字列を小文字にしたものです。日本語はそのまま使えます（例：`#markdown原文の読み込み`）。
  記号と空白は `-` に変わるので、見出しを変えたらリンクも直します。
- メソッドは `Config::load` のように `型名::メソッド名` で指定します。
- リンクするのは、利用者から見える契約です。内部の補助関数すべてに付ける必要はありません。
- `src/main.rs` の関数は非公開ですが、コマンドラインの契約を担うので結んでいます。
  そのために `docbridge.config.json` の `visibility` へ `private` を加えています。
- リンクのあるコードや文書を変えたら、`just docs-check` を実行します。
- `just docs-audit` はときどき実行し、リンクの候補を探します。意味のない一対一のリンクを増やすためには使いません。
  非公開の補助関数とテストも候補に挙がるので、契約を担うものだけを選びます。
- 注釈の付いた宣言を変えるときは、リンク先の仕様も見直します。
- DocBridgeが必要とするNode.js（22以上）とpnpmは、**開発者とCIだけ**のものです。
  利用者は、コンパイルしたRustの実行ファイルだけで使えるようにします。
- DocBridgeは `tools/docbridge.sh` が、`.docbridge-version` の版を `pnpm dlx` で呼び出します。
  npmと `package.json` は使いません。版を上げるときは `.docbridge-version` を書き換えます。

## Markdown の共有

- ユーザーが読む計画・設計・報告などの Markdown を作成・更新したら、メインは完了報告や確認依頼の前に `miyaburu --no-open <ファイル>` で外部共有します。委譲実装役は共有しません。
- 実出力から取得した共有 URL をクリックできる Markdown リンクで返します。失敗時は失敗と生成済み HTML のパスを伝え、URL を捏造しません。
- ブラウザは明示依頼がない限り起動せず、同じファイルで再実行して既存共有を更新します。

## 構成

| 場所 | 役割 |
| --- | --- |
| `src/lib.rs` | Markdownの読み込み。 |
| `src/render.rs` | MarkdownからHTML断片への変換。 |
| `src/highlight.rs` | コードブロックのシンタックスハイライト。 |
| `src/config.rs` | TOMLの設定と検証。 |
| `src/theme.rs` | 組み込みの配色。 |
| `src/page.rs` | HTML文書の組み立て。 |
| `src/main.rs` | コマンドラインの入口。 |
| `assets/preview.css`、`assets/preview.js` | ページの見た目と動作。実行ファイルに埋め込みます。 |
| `assets/vendor/` | 同梱しているMermaid。手で編集しません。 |
| `tests/cli.rs` | 実行ファイルを通した結合テスト。 |
| `tools/docbridge.sh`、`.docbridge-version` | DocBridgeの呼び出しと、使う版の定義。 |

## 検証

開発用のツールは `flake.nix` が用意し、`direnv`（`use flake`）で有効になります。
リポジトリに入ったら、最初に一度だけ `direnv allow` を実行します。以降はディレクトリに入るだけで使えます。

- `just check`：Rustの書式、テスト、DocBridgeの整合性を確かめます。
- `just lint`：Clippyで静的に検査します。
- `just --list`：ほかのレシピを確認できます。

CIは、同じレシピを `nix develop --command` 経由で実行します。
