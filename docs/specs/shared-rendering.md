# 共有用の描画

通常のローカル描画とは別に、共有サーバーへ渡すHTML断片と画像を生成します。
既存の `RenderOptions` と `render_markdown` の形と挙動は変更しません。

<!-- @code src/render.rs#SharedAsset -->

## 共有画像

`SharedAsset` は `path: String`、`content_type: String`、`bytes: Vec<u8>` を持ちます。
`Debug`、`Clone`、`PartialEq`、serdeのシリアライズ・デシリアライズに対応します。
`path` は先頭が `/` の許可リスト用URLです。元のファイルパスを公開しません。
読み込んだ画像だけを配信対象にします。

<!-- @code src/render.rs#SharedRendered -->

## 共有描画結果

`SharedRendered` は `rendered: Rendered` と `assets: Vec<SharedAsset>` を持ちます。
HTML断片内のローカル画像は、対応する `/assets/<識別子>` に置き換えます。
識別子は描画結果内で一意です。画像のクエリーとフラグメントはURLに保持し、
ファイルの読み込みには使いません。

<!-- @code src/render.rs#render_shared_markdown -->

## 共有描画

`render_shared_markdown(source: &str, options: &RenderOptions)` は
`Result<SharedRendered, String>` を返します。
ローカルリンクはリンクを無効にし、表示文字列を保持します。
外部リンク・画像とページ内リンクは既存描画と同じ規則で保持します。
相対画像は `base_dir` を基準に読み込み、絶対パスとローカルの `file:` 画像も扱います。
相対画像に基準ディレクトリがない場合はエラーです。
日本語と空白を含むパスに対応し、パスの `%XX` は一度だけデコードします。
不正なパーセント表記、非UTF-8、NUL、不足画像は文字列のエラーを返します。

拡張子ではなくバイト列のシグネチャを検査し、PNG、JPEG、GIF、WebP、BMP、TIFF、ICOを
受け入れます。シグネチャ検査は完全な画像デコードではありません。
非画像や未対応形式はエラーにします。SVGはactive contentを含み得るため配信しません。
呼び出し側はエラーを共有失敗として扱い、ローカル表示にフォールバックできます。
