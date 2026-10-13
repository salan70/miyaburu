# みやぶる / miyaburu

**みやぶる**は、AIコーディングエージェントが生成したMarkdown文書を、ブラウザーで読みやすく表示するためのRust製CLIです。
Markdownファイルを指定するだけで使えます。

```sh
miyaburu docs/plan.md
```

ローカルHTMLを生成し、パスを標準出力へ送信して既定のブラウザーで開いた後、Cloudflare Quick Tunnelによるスマホ向け共有を試みて終了します。
共有URLの取得を待つ前にパスをflushするため、呼び出し元はすぐ受け取れます。ブラウザー起動に失敗した場合は終了コード1となり、新しい共有を起動しません。
共有URLと余白付きQRコードは標準エラー、ローカルHTMLのパスは標準出力に1行で表示します。
共有には `cloudflared` を別途インストールし、PATHに置いてください。未導入や15秒以内にURLを取得できない場合は、共有を片付けてローカル表示に戻ります。共有の失敗だけでは終了コード1になりません。
`--no-share` ならサーバーやトンネルを起動しない従来のローカル動作になります。
文書が外部の画像（`https://…`）を参照している場合は、ブラウザーがその画像を取得します。

## 表示の特徴

- 紙の質感を使わない、白基調の**Editorial × Soft Pop**デザイン
- 本文・見出し・UIにはLINE Seed JP、コードにはHackGenを使用
- 文字の大きさと行の高さをそろえ、エディタで開いた原文に近い見た目で表示する
- スクロール中も、読んでいる位置の見出しと親の見出しを画面の上端に留める
- 見出しの `##`、箇条書きの `-`、インラインコードのバッククォートは表示し、強調の `**` など装飾用の記法は隠す
- 原文の改行はそのまま改行として表示し、本文やリスト中の句点 `。` の後でも、表示上の改行を入れる
- 長いファイルパス・URLは短く表示し、実際の値とコピー機能は維持する
- GFM（表、タスクリスト、脚注、アラートなど）とMermaidに対応し、オフラインで表示できる
- コードブロックを、選んだ配色に合わせてシンタックスハイライトする
- CSSを直接編集せず、設定ファイルから見た目を変更できる
- Dracula、Monokai、Kanagawa、Tokyo Night、Xcodeなど、よく使われる配色を名前で選べる
- Codex、Claude Codeなどから、簡単なコマンドで起動できる

フォントは同梱していません。
LINE Seed JPとHackGenがインストールされていない端末では、OSの日本語フォントで表示します。

**まだ対応していないもの**は、OSの外観に合わせた配色の自動切り替え、文書の変更を検知した再読み込みです。
共有URLを知る人は文書を閲覧できます。認証や常設デプロイは対象外です。

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
| `--no-open` | ブラウザーだけを抑止する（共有は止めない） |
| `--no-share` | 共有せずローカルHTMLだけを生成する |
| `--stop <Markdownファイル>` | 指定文書の共有を停止する |
| `--stop-all` | すべての共有を停止する |
| `-h`、`--help` | 使い方を表示する |
| `-V`、`--version` | バージョンを表示する |

[examples/sample.md](examples/sample.md)で、表示の特徴をひととおり確認できます。

```sh
cargo run -- examples/sample.md
```

共有はコマンド終了後もバックグラウンドで続きます。同じ文書を再実行するとURLを再利用し、HTMLと画像をまとめて更新します。スマホでは手動で再読み込みしてください。
`miyaburu --stop docs/plan.md` または `miyaburu --stop-all` で停止します。停止済みも成功です。
文書が消えた場合も、共有開始時と同じ絶対パスを `--stop` に指定して停止を試みられます。
親ディレクトリが存在すればシンボリックリンクを解決するため、`/tmp` と `/private/tmp` や別名の親パスからも同じ文書の共有を停止できます。親も消えている場合は、指定した絶対パスで照合します。

詳しくは[コマンドラインの仕様](docs/specs/cli.md)を参照してください。

## エージェントから使う

Codex、Claude Codeなどのエージェントに開かせるには、`AGENTS.md` や `CLAUDE.md` に次のように書いておきます。

```markdown
- 計画書や設計書などのMarkdownを書いたら、`miyaburu <ファイル>` で開く。
- 実行結果から共有URLを取得できたら、ユーザーへの返答にクリックできるMarkdownリンクを載せる。QRコード画像はチャットに貼らない。
- 共有URLを取得できない場合は、生成されたローカルHTMLのパスを伝える。URLを作り上げない。
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
theme = "tokyo-night"
accent = "#e0407a"

[rendering]
break_after_kuten = false
```

書いた項目だけが既定値から変わります。
`color.theme` には、次の配色を指定できます。同じ `[color]` に書いた色は、配色の色より優先します。

| 背景 | 名前 |
| --- | --- |
| 暗い | `dracula`、`monokai`、`kanagawa`、`tokyo-night`、`tokyo-night-storm`、`catppuccin-mocha`、`nord`、`gruvbox-dark`、`one-dark`、`solarized-dark`、`rose-pine`、`everforest`、`github-dark`、`xcode-dark`、`xcode-dusk`、`xcode-midnight`、`xcode-civic` |
| 明るい | `default`、`github-light`、`xcode-light`、`alucard`（Draculaのライト版）、`solarized-light`、`catppuccin-latte`、`gruvbox-light`、`rose-pine-dawn`、`kanagawa-lotus` |

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
[docs/specs/](docs/specs/)の仕様と、`src/`で契約を担う宣言には、双方向リンクを設定しています。
挙動を変える前に、該当する仕様と[AGENTS.md](AGENTS.md)を確認してください。

## 同梱しているもの

- [Mermaid](https://github.com/mermaid-js/mermaid) 12.1.0（MIT）— `assets/vendor/mermaid.min.js`。ライセンスは`assets/vendor/mermaid.LICENSE`にあります。
- シンタックスハイライトの構文定義 — [two-face](https://crates.io/crates/two-face) クレートが、[bat](https://github.com/sharkdp/bat)の同梱する定義をまとめたもの。ビルドのときに実行ファイルへ埋め込まれます。定義ごとのライセンスは、two-faceの[一覧](https://codeberg.org/CosmicHarper/two-face/src/branch/main/generated/acknowledgements_full.md)にあります。

## 参考資料

- [UI/UX NUMA](https://github.com/salan70/uiux-numa) — タイポグラフィ、余白、色の役割、デザイン検証
- [DocBridge](https://github.com/salan70/docbridge) — 開発時の文書・コード間の整合性管理
