# 配布と導入

## 対応環境

リリースバイナリは Apple Silicon Mac（`aarch64-apple-darwin`）、x86_64 Linux（`x86_64-unknown-linux-musl`）、aarch64 Linux（`aarch64-unknown-linux-musl`）向けです。Linux版はmuslを使用します。Intel MacとWindowsは対象外です。

## 配布物

各リリースには `miyaburu-${target}.tar.gz` と、同名に `.sha256` を付けたチェックサムファイルを添付します。アーカイブのルートには通常ファイル `miyaburu` を格納します。チェックサムは `64桁のSHA-256  アーカイブ名` の1行形式です。公開リリースには `install.sh` も添付します。

## curlによる導入

最新の正式リリースを導入します。

```sh
curl -fsSL https://github.com/salan70/miyaburu/releases/latest/download/install.sh | sh
```

インストーラーは最新正式リリースを一度解決し、その固定タグからアーカイブとチェックサムを取得します。導入先の既定値は `${HOME}/.local/bin` です。導入後の更新も同じコマンドで行えます。

版を固定する場合は、インストーラー自体も固定タグから取得し、環境変数を `sh` に渡します。たとえばCargo.tomlの版が `0.1.0` の場合:

```sh
curl -fsSL https://github.com/salan70/miyaburu/releases/download/v0.1.0/install.sh | MIYABURU_VERSION=v0.1.0 sh
```

導入先を指定する場合:

```sh
curl -fsSL https://github.com/salan70/miyaburu/releases/latest/download/install.sh | MIYABURU_INSTALL_DIR="$HOME/.local/bin" sh
```

導入先がPATHに含まれていない場合、導入成功後に案内します。シェル設定は自動変更しません。標準の導入先を現在のシェルで有効にするには次を実行します。継続して使う場合は、同じ行を利用中のシェルの設定ファイルにも追加してください。

```sh
export PATH="$HOME/.local/bin:$PATH"
```

標準導入を削除するには次を実行します。

```sh
rm "$HOME/.local/bin/miyaburu"
```

生成済みHTMLや設定は残ります。共有機能に必要な `cloudflared` は別途導入してください。インストーラーは自動導入しません。

## 検証とセキュリティ

インストーラーはバイナリを配置する前にSHA-256を照合します。不一致、取得失敗、非対応環境、または不正な版指定では失敗し、既存バイナリを変更しません。転送はHTTPSのみです。チェックサム照合は転送中などの破損を検知するもので、配布元自体の改ざんを保証しません。署名とmacOS notarizationは実施していません。

## リリース

`Cargo.toml` の版と `v` を付けたGitタグを一致させます。変更をcommit・pushした後、たとえば次のようにタグを作成してpushします。

```sh
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actionsの配布処理が完了するまで待ちます。

3環境すべてのビルドと実行確認が成功すると、配布物をDraftへまとめてから正式公開します。途中で失敗してDraftが残った場合は、同じタグのDraftと添付物を確認し、未公開のDraftだけを削除してジョブを再実行してください。正式公開済みのリリースは削除・上書きしません。
