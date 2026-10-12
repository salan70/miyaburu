# みやぶる / miyaburu

**みやぶる**は、AIコーディングエージェントが生成したMarkdown文書を、ブラウザーで読みやすく表示するためのRust製CLIです。
Markdownファイルを指定するだけで使えます。

```sh
miyaburu docs/plan.md
```

HTMLを生成して既定のブラウザーで開き、すぐに終了します。
サーバーは起動せず、みやぶる自身は外部と通信しません。
文書が外部の画像（`https://…`）を参照している場合だけ、ブラウザーがその画像を取得します。

## 表示の特徴

- 紙の質感を使わない、白基調の**Editorial × Soft Pop**デザイン
- 本文・見出し・UIにはLINE Seed JP、コードにはHackGenを使用
- 見出しの `##` や箇条書きの `-` は表示し、強調の `**` など装飾用の記法は隠す
- 本文やリスト中の句点 `。` の後で、表示上の改行を入れる
- 長いファイルパス・URLは短く表示し、実際の値とコピー機能は維持する
- GFM（表、タスクリスト、脚注、アラートなど）とMermaidに対応し、オフラインで表示できる
- CSSを直接編集せず、設定ファイルから見た目を変更できる
- Codex、Claude Codeなどから、簡単なコマンドで起動できる

フォントは同梱していません。
LINE Seed JPとHackGenがインストールされていない端末では、OSの日本語フォントで表示します。

**まだ対応していないもの**は、コードのシンタックスハイライト、ダークモード、文書の変更を検知した再読み込みです。
Cloudflare Tunnelなどを利用したスマホからの閲覧は、将来の構想でありMVPには含めません。

## 導入

Nix（flakes有効）があれば、次のコマンドで `miyaburu` を使えるようになります。

```sh
nix profile add github:salan70/miyaburu
```

導入せずに試す場合は、`nix run` を使います。

```sh
nix run github:salan70/miyaburu -- docs/plan.md
```

Nixのパッケージは、macOS（Apple Silicon）とLinux（x86_64、aarch64）に用意しています。
Rustのツールチェーンがある場合は、このリポジトリを取得して `cargo install --path .` でも導入できます。

## 使い方

```sh
miyaburu [オプション] <Markdownファイル>
```

| オプション | 動作 |
| --- | --- |
| `-c`、`--config <ファイル>` | 設定ファイルを指定する |
| `-o`、`--output <ディレクトリ>` | HTMLの出力先を指定する |
| `--no-open` | ブラウザーを開かず、生成したHTMLのパスだけを表示する |
| `-h`、`--help` | 使い方を表示する |
| `-V`、`--version` | バージョンを表示する |

[examples/sample.md](examples/sample.md)で、表示の特徴をひととおり確認できます。

```sh
cargo run -- examples/sample.md
```

詳しくは[コマンドラインの仕様](docs/specs/cli.md)を参照してください。

## エージェントから使う

Codex、Claude Codeなどのエージェントに開かせるには、`AGENTS.md` や `CLAUDE.md` に次のように書いておきます。

```markdown
- 計画書や設計書などのMarkdownを書いたら、`miyaburu <ファイル>` で開く。
```

`miyaburu` はブラウザーの終了を待たずに終了するので、エージェントの作業を止めません。
ブラウザーを開けない環境では `--no-open` を付けると、生成したHTMLのパスだけを標準出力に表示します。

## 設定

見た目はTOMLの設定ファイルで変更できます。
実行したディレクトリの `miyaburu.toml`、次に `~/.config/miyaburu/config.toml` を探します。

```toml
[font]
size = 17

[color]
accent = "#e0407a"

[rendering]
break_after_kuten = false
```

書いた項目だけが既定値から変わります。
項目の一覧と既定値は[設定の仕様](docs/specs/configuration.md)にあります。

## 開発環境

実装言語はRustです。開発用のツールチェーンは`flake.nix`、実行コマンドは`justfile`で管理します。
[DocBridge](https://github.com/salan70/docbridge)とその実行に使うNode.js・pnpmは開発・CI専用であり、みやぶる本体の実行時依存ではありません。
npmと`package.json`は使いません。

必要なものは **Nix（flakes有効）・direnv・nix-direnv** です。direnvのシェルフックを事前に設定してください。
既にdotfiles側で設定している場合、再設定は不要です。

初回は次の操作を行います。

```sh
cd miyaburu
direnv allow
```

`.envrc`は`use flake`を実行します。**初回の許可後はcdだけで開発環境が自動的に有効**になります。
`.envrc`の内容を変更した場合は再度`direnv allow`が必要です。
`nix-direnv`を使うことで再入室時のシェル評価がキャッシュされます。

主なコマンドは次のとおりです。

| コマンド | 用途 |
| --- | --- |
| `just --list` | 利用できるコマンドの一覧 |
| `just check` | Rustの書式・テスト・DocBridgeの整合性を検証 |
| `just fmt` | Rustのコードを整形 |
| `just lint` | Clippyによる静的検査 |
| `just build` | Rustをビルド |
| `just docs-audit` | DocBridgeで未連携の候補を確認 |
| `just docbridge <引数>` | DocBridgeの任意のサブコマンドを実行 |
| `just versions` | ツールのバージョンを確認 |

direnvを使わない場面では、`nix develop --command just check`でも実行できます。
環境の依存は`flake.lock`と`Cargo.lock`で固定しています。
DocBridgeの版は`.docbridge-version`で決め、`tools/docbridge.sh`が`pnpm dlx`でその版を取得して実行します。事前の導入は不要です。

DocBridgeの設定は[docbridge.config.json](docbridge.config.json)にあります。
[docs/specs/](docs/specs/)の仕様と`src/`の公開している宣言には、双方向リンクを設定しています。
挙動を変える前に、該当する仕様と[AGENTS.md](AGENTS.md)を確認してください。

## 同梱しているもの

- [Mermaid](https://github.com/mermaid-js/mermaid) 12.1.0（MIT）— `assets/vendor/mermaid.min.js`。ライセンスは`assets/vendor/mermaid.LICENSE`にあります。

## 参考資料

- [UI/UX NUMA](https://github.com/salan70/uiux-numa) — タイポグラフィ、余白、色の役割、デザイン検証
- [DocBridge](https://github.com/salan70/docbridge) — 開発時の文書・コード間の整合性管理
