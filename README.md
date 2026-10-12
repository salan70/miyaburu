# みやぶる / miyaburu

**みやぶる**は、AIコーディングエージェントが生成したMarkdown文書を、ブラウザーで読みやすく表示するためのRust製CLIです。
目指す操作は、Markdownファイルを指定するだけです。

```sh
miyaburu docs/plan.md
```

> **現在は初期構築段階です。** 上記コマンドは目標とする使い方であり、**まだ実装されていません**。
> 現時点で実装されているのは、RustによるMarkdown原文の読み込みと基本テストです。

## 目指す表示体験

- 紙の質感を使わない、白基調の**Editorial × Soft Pop**デザイン
- 本文・見出し・UIにはLINE Seed JP、コードにはHackGenを使用
- 見出しの `##` や箇条書きの `-` は表示し、強調の `**` など装飾用の記法は隠す
- 本文やリスト中の句点 `。` の後で、表示上の改行を入れる
- 長いファイルパス・URLは省略し、実際の値やコピー機能は維持する
- GFM、コードハイライト、Mermaidへの対応と、オフラインでの表示
- CSSを直接編集せず、設定ファイルから見た目を変更できる
- Codex、Claude Codeなどから、簡単なコマンドで起動できる

初期の対象は**PC上のローカルプレビュー**です。
Cloudflare Tunnelなどを利用したスマホからの閲覧は、将来の構想でありMVPには含めません。

## 開発環境

実装言語はRustです。開発用のツールチェーンは`flake.nix`、実行コマンドは`justfile`で管理します。
[DocBridge](https://github.com/salan70/docbridge)のNode.jsは開発・CI専用であり、みやぶる本体の実行時依存ではありません。

必要なものは **Nix（flakes有効）・direnv・nix-direnv** です。direnvのシェルフックを事前に設定してください。
既にdotfiles側で設定している場合、再設定は不要です。

初回は次の操作を行います。

```sh
cd miyaburu
direnv allow
just setup
```

`.envrc`は`use flake`を実行します。**初回の許可後はcdだけで開発環境が自動的に有効**になります。
`.envrc`の内容を変更した場合は再度`direnv allow`が必要です。
`nix-direnv`を使うことで再入室時のシェル評価がキャッシュされます。

主なコマンドは次のとおりです。

| コマンド | 用途 |
| --- | --- |
| `just --list` | 利用できるコマンドの一覧 |
| `just setup` | 開発用DocBridgeの依存関係を導入 |
| `just check` | Rustの書式・テスト・DocBridgeの整合性を検証 |
| `just fmt` | Rustのコードを整形 |
| `just lint` | Clippyによる静的検査 |
| `just build` | Rustライブラリをビルド |
| `just docs-audit` | DocBridgeで未連携の候補を確認 |
| `just versions` | ツールのバージョンを確認 |

direnvを使わない場面では、`nix develop --command just check`でも実行できます。
環境の依存を固定するため、**`flake.lock`はNixが生成したものをGitにコミットする予定**です。現段階では未生成です。

DocBridgeの設定は[docbridge.config.json](docbridge.config.json)にあります。
[入力仕様](docs/specs/input.md)と[`src/lib.rs`](src/lib.rs)には双方向リンクを設定しています。
機能の実装前には[表示仕様](docs/specs/rendering.md)と[AGENTS.md](AGENTS.md)を確認してください。
この初期段階ではブラウザープレビューCLIはまだ動作しません。

## 参考資料

- [UI/UX NUMA](https://github.com/salan70/uiux-numa) — タイポグラフィ、余白、色の役割、デザイン検証
- [DocBridge](https://github.com/salan70/docbridge) — 開発時の文書・コード間の整合性管理
