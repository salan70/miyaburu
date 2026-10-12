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

## 開発者向け

みやぶる本体はRustで実装します。
[DocBridge](https://github.com/salan70/docbridge)の実行には**開発時のみ**Node.js 22以上が必要ですが、完成したRust CLIの利用者には要求しません。

```sh
cargo fmt --check
cargo test

npm install
npm run docs:check
npm run docs:audit
```

これらのチェックは、初期構築PRのGitHub Actionsで成功を確認しています。
ただし、完成したプレビューCLIの検証ではありません。
依存関係をローカルで解決した後にnpmのロックファイルをコミットし、CIを`npm ci`に切り替える予定です。

DocBridgeの設定は[docbridge.config.json](docbridge.config.json)にあります。
[入力仕様](docs/specs/input.md)と[`src/lib.rs`](src/lib.rs)には、実際のコード・仕様の双方向リンクを設定しています。

プレビューの実装に着手する前に、[表示仕様](docs/specs/rendering.md)と[AGENTS.md](AGENTS.md)を確認してください。
機能を実装したら、その機能に対応するRustの宣言と仕様の節をDocBridgeで紐付けます。
存在しないコードへのリンクは作りません。

## 参考資料

- [UI/UX NUMA](https://github.com/salan70/uiux-numa) — タイポグラフィ、余白、色の役割、デザイン検証
- [DocBridge](https://github.com/salan70/docbridge) — 開発時の文書・コード間の整合性管理
