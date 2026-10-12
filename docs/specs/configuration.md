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
line_height = 1.625

[color]
theme = "default"
background = "#ffffff"
surface = "#f6f6f7"
text = "#1f1f1f"
muted = "#4d4d4d"
border = "#e5e5e8"
accent = "#4078e0"
accent_soft = "#e8effc"
link = "#2f63c8"
code = "#c4314b"
note = "#2f63c8"
tip = "#1a7f4b"
warning = "#9a6200"
caution = "#c8372d"

[color.syntax]
comment = "#6e6e76"
keyword = "#c4314b"
string = "#1a7f4b"
constant = "#9a6200"
function = "#2f63c8"
type = "#0f7b8a"

[layout]
max_width = 960

[rendering]
show_markers = true
break_after_kuten = true
abbreviate = true
abbreviate_min_chars = 48
highlight = true

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
| `color.theme` | 元にする[組み込みの配色](#組み込みの配色) | 配色の名前 |
| `color.background` | ページの背景 | 色 |
| `color.surface` | コードブロックや表の見出し行の面 | 色 |
| `color.text` | 本文の文字 | 色 |
| `color.muted` | 補足的な文字 | 色 |
| `color.border` | 区切り線と枠 | 色 |
| `color.accent` | 見出しと、箇条書きの記号などの差し色 | 色 |
| `color.accent_soft` | 差し色を薄くした面 | 色 |
| `color.link` | リンク | 色 |
| `color.code` | 文中のコードの文字 | 色 |
| `color.note` | アラート `[!NOTE]` の記号と罫 | 色 |
| `color.tip` | アラート `[!TIP]` の記号と罫 | 色 |
| `color.warning` | アラート `[!WARNING]` の記号と罫 | 色 |
| `color.caution` | アラート `[!CAUTION]` の記号と罫 | 色 |
| `color.syntax.comment` | コードブロックのコメント | 色 |
| `color.syntax.keyword` | コードブロックのキーワードと、タグやキーの名前 | 色 |
| `color.syntax.string` | コードブロックの文字列 | 色 |
| `color.syntax.constant` | コードブロックの数値、真偽値などの定数 | 色 |
| `color.syntax.function` | コードブロックの関数と、属性の名前 | 色 |
| `color.syntax.type` | コードブロックの型 | 色 |
| `layout.max_width` | 本文の最大幅（px） | 320〜2400 |
| `rendering.show_markers` | 見出しの `##`、箇条書きの `-`、インラインコードのバッククォートを表示する | `true`、`false` |
| `rendering.break_after_kuten` | 句点 `。` の後に表示上の改行を入れる | `true`、`false` |
| `rendering.abbreviate` | 長いパスとURLを短く表示する | `true`、`false` |
| `rendering.abbreviate_min_chars` | 短くする対象にする文字数の下限 | 16〜400 |
| `rendering.highlight` | コードブロックを[字句の種類ごとに色分け](rendering.md#シンタックスハイライト)する | `true`、`false` |
| `mermaid.enabled` | `mermaid` のコードブロックを図にする | `true`、`false` |
| `mermaid.theme` | Mermaidのテーマ | `default`、`neutral`、`forest`、`base`、`dark` |

- 色は `#rgb`、`#rrggbb`、`#rrggbbaa` のいずれかで書きます。
- アラート `[!IMPORTANT]` は、色を足さずに `color.text` で示します。
- コードブロックの差分（`diff`）は、足した行を `color.tip`、消した行を `color.caution` で示します。
- `mermaid.theme` を書かない場合は、`color.theme` が `default` なら `neutral`、それ以外なら `base` になります。
  `base` は、図の色をページの色に合わせます。
- 指定したフォントが端末にない場合は、既定のフォント、OSの日本語フォントの順に代替します。
- フォント名には、引用符、`\`、`;`、`{`、`}`、`<`、`>` を使えません。

<!-- @code src/theme.rs#ColorTheme -->

## 組み込みの配色

`color.theme` に名前を書くと、`[color]` と `[color.syntax]` の色がすべてその配色に変わります。
同じ `[color]` と `[color.syntax]` に書いた色は、配色の色より優先します。

```toml
[color]
theme = "tokyo-night"
accent = "#e0407a"

[color.syntax]
comment = "#7f88b0"
```

| 名前 | 元にした配色 | 背景 |
| --- | --- | --- |
| `default` | みやぶるの既定の配色 | 明るい |
| `dracula` | Dracula | 暗い |
| `alucard` | Alucard（Dracula のライト版） | 明るい |
| `monokai` | Monokai | 暗い |
| `kanagawa` | Kanagawa（Wave） | 暗い |
| `kanagawa-lotus` | Kanagawa（Lotus） | 明るい |
| `tokyo-night` | Tokyo Night | 暗い |
| `tokyo-night-storm` | Tokyo Night（Storm） | 暗い |
| `catppuccin-mocha` | Catppuccin（Mocha） | 暗い |
| `catppuccin-latte` | Catppuccin（Latte） | 明るい |
| `nord` | Nord | 暗い |
| `gruvbox-dark` | Gruvbox（Dark） | 暗い |
| `gruvbox-light` | Gruvbox（Light） | 明るい |
| `one-dark` | One Dark | 暗い |
| `solarized-dark` | Solarized（Dark） | 暗い |
| `solarized-light` | Solarized（Light） | 明るい |
| `rose-pine` | Rosé Pine | 暗い |
| `rose-pine-dawn` | Rosé Pine（Dawn） | 明るい |
| `everforest` | Everforest（Dark） | 暗い |
| `github-dark` | GitHub（Dark） | 暗い |
| `github-light` | GitHub（Light） | 明るい |
| `xcode-dark` | Xcode の Default（Dark） | 暗い |
| `xcode-light` | Xcode の Default（Light） | 明るい |
| `xcode-dusk` | Xcode の Dusk | 暗い |
| `xcode-midnight` | Xcode の Midnight | 暗い |
| `xcode-civic` | Xcode の Civic | 暗い |

- 色は、元にした配色が公開している値から、役割に合うものを選んでいます。
  補足の文字に使える明るさの色がない配色では、文字の色をコメントの色へ寄せた色（Dracula、Monokai、Nord）か、背景へ寄せた色（Xcode の Dusk、Midnight、Civic）を使います。
  `color.accent_soft` は、差し色を背景に混ぜた色です。
- `[color.syntax]` の色も、元にした配色がその種類の字句に使っている色です。
  - コードブロックの面（`color.surface`）の上でコントラスト比が 3.0 に届かない色は、届くまで文字の色へ寄せています。
  - 元の配色で2つの種類が同じ色になるところは、片方を近い役割の色に替えています（Gruvbox と Everforest の関数、Solarized と Rosé Pine の定数）。
- 背景が暗いときは、スクロールバーなどブラウザーが描く部品も暗い側で表示します。
  背景が暗いかどうかは、`color.background` の明るさで決めます。
- OSの外観（ライト、ダーク）に合わせた自動の切り替えは行いません。

## 検証

- 知らない項目名は、書き間違いとみなしてエラーにします。
- 範囲や形式が正しくない値は、項目名（例：`font.size`）と理由を示してエラーにします。
- エラーのときは、既定値での表示へ切り替えずに終了コード1で終了します。
