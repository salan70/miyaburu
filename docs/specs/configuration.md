# 設定

設定ファイルはTOMLで書きます。
設定がなくても既定値で動きます。書いた項目だけが既定値から変わります。

<!-- @code src/config.rs#find_config_file -->

## 設定ファイルの場所

次の順に探し、最初に見つかった1つだけを使います。

1. `--config` で指定したファイル。
2. コマンドを実行したディレクトリの `miyaburu.toml`。
3. `$XDG_CONFIG_HOME/miyaburu/config.toml`（未設定なら `~/.config/miyaburu/config.toml`）。

`--config` で指定したファイルが読めない場合は、エラーで終了します。

<!-- @code src/config.rs#Config -->

## 設定項目

```toml
[font]
body = "LINE Seed JP"
code = "HackGen"
size = 16
line_height = 1.75

[color]
background = "#ffffff"
surface = "#f6f6f7"
text = "#1f1f1f"
muted = "#4d4d4d"
border = "#e5e5e8"
accent = "#4078e0"
accent_soft = "#e8effc"
link = "#2f63c8"

[layout]
max_width = 720

[rendering]
show_markers = true
break_after_kuten = true
abbreviate = true
abbreviate_min_chars = 48

[mermaid]
enabled = true
theme = "neutral"
```

上の値が既定値です。

| 項目 | 意味 | 指定できる値 |
| --- | --- | --- |
| `font.body` | 本文、見出し、UIのフォント名 | 100文字までの名前 |
| `font.code` | コードのフォント名 | 100文字までの名前 |
| `font.size` | 本文の文字サイズ（px） | 10〜32 |
| `font.line_height` | 本文の行の高さ（文字サイズに対する倍率） | 1.0〜3.0 |
| `color.background` | ページの背景 | 色 |
| `color.surface` | コードブロックや表の見出し行の面 | 色 |
| `color.text` | 本文の文字 | 色 |
| `color.muted` | 補足的な文字 | 色 |
| `color.border` | 区切り線と枠 | 色 |
| `color.accent` | 見出しや箇条書きの記号などの差し色 | 色 |
| `color.accent_soft` | 差し色を薄くした面 | 色 |
| `color.link` | リンク | 色 |
| `layout.max_width` | 本文の最大幅（px） | 320〜2400 |
| `rendering.show_markers` | 見出しの `##`、箇条書きの `-`、インラインコードのバッククォートを表示する | `true`、`false` |
| `rendering.break_after_kuten` | 句点 `。` の後に表示上の改行を入れる | `true`、`false` |
| `rendering.abbreviate` | 長いパスとURLを短く表示する | `true`、`false` |
| `rendering.abbreviate_min_chars` | 短くする対象にする文字数の下限 | 16〜400 |
| `mermaid.enabled` | `mermaid` のコードブロックを図にする | `true`、`false` |
| `mermaid.theme` | Mermaidのテーマ | `default`、`neutral`、`forest`、`base`、`dark` |

- 色は `#rgb`、`#rrggbb`、`#rrggbbaa` のいずれかで書きます。
- 指定したフォントが端末にない場合は、既定のフォント、OSの日本語フォントの順に代替します。
- フォント名には、引用符、`\`、`;`、`{`、`}`、`<`、`>` を使えません。

## 検証

- 知らない項目名は、書き間違いとみなしてエラーにします。
- 範囲や形式が正しくない値は、項目名（例：`font.size`）と理由を示してエラーにします。
- エラーのときは、既定値での表示へ切り替えずに終了コード1で終了します。
