//! 見た目と変換の設定。TOMLファイルから読み、CSSを書かずに変更できるようにする。

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::theme::{ColorTheme, is_dark_background};

/// 設定ファイル全体。書かれていない項目は既定値になる。
///
/// @doc docs/specs/configuration.md#設定項目
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub font: FontConfig,
    pub color: ColorConfig,
    pub layout: LayoutConfig,
    pub rendering: RenderingConfig,
    pub mermaid: MermaidConfig,
}

/// `[font]`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FontConfig {
    /// 本文、見出し、UIに使うフォント名。
    pub body: String,
    /// コードに使うフォント名。
    pub code: String,
    /// 本文の文字サイズ（px）。
    pub size: f64,
    /// 本文の行の高さ（文字サイズに対する倍率）。
    pub line_height: f64,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            body: "LINE Seed JP".to_string(),
            code: "HackGen".to_string(),
            size: 16.0,
            line_height: 1.625,
        }
    }
}

/// `[color]`。`theme` の配色を元にし、書かれた色だけを置き換えたもの。
/// 色の値は `#rgb`、`#rrggbb`、`#rrggbbaa` のいずれか。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "ColorTable")]
pub struct ColorConfig {
    /// 元にした組み込みの配色。
    pub theme: ColorTheme,
    pub background: String,
    /// コードブロックや表の見出し行の面。
    pub surface: String,
    pub text: String,
    /// 補足的な文字。
    pub muted: String,
    pub border: String,
    /// 見出しと、箇条書きの記号などに使う差し色。
    pub accent: String,
    /// 差し色を薄くした面。
    pub accent_soft: String,
    pub link: String,
    /// 文中のコードの文字。
    pub code: String,
    /// アラートの種類を示す色。記号と罫に使う。
    pub note: String,
    pub tip: String,
    pub warning: String,
    pub caution: String,
    /// コードブロックの字句の色。
    pub syntax: SyntaxColors,
}

/// `[color.syntax]`。コードブロックで、字句の種類ごとに使う色。
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxColors {
    pub comment: String,
    pub keyword: String,
    pub string: String,
    /// 数値、真偽値などの定数。
    pub constant: String,
    pub function: String,
    pub r#type: String,
}

impl ColorConfig {
    /// 組み込みの配色をそのまま使う。
    pub fn from_theme(theme: ColorTheme) -> Self {
        let palette = theme.palette();
        let syntax = theme.syntax();
        Self {
            theme,
            background: palette.background.to_string(),
            surface: palette.surface.to_string(),
            text: palette.text.to_string(),
            muted: palette.muted.to_string(),
            border: palette.border.to_string(),
            accent: palette.accent.to_string(),
            accent_soft: palette.accent_soft.to_string(),
            link: palette.link.to_string(),
            code: palette.code.to_string(),
            note: palette.note.to_string(),
            tip: palette.tip.to_string(),
            warning: palette.warning.to_string(),
            caution: palette.caution.to_string(),
            syntax: SyntaxColors {
                comment: syntax.comment.to_string(),
                keyword: syntax.keyword.to_string(),
                string: syntax.string.to_string(),
                constant: syntax.constant.to_string(),
                function: syntax.function.to_string(),
                r#type: syntax.r#type.to_string(),
            },
        }
    }

    /// 背景が暗いか。暗いときは、ブラウザーが描く部品と図の色も暗い側に合わせる。
    pub fn is_dark(&self) -> bool {
        is_dark_background(&self.background)
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self::from_theme(ColorTheme::Default)
    }
}

/// `[color]` に書かれた内容。書かれていない色は `theme` の配色から取る。
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ColorTable {
    theme: ColorTheme,
    background: Option<String>,
    surface: Option<String>,
    text: Option<String>,
    muted: Option<String>,
    border: Option<String>,
    accent: Option<String>,
    accent_soft: Option<String>,
    link: Option<String>,
    code: Option<String>,
    note: Option<String>,
    tip: Option<String>,
    warning: Option<String>,
    caution: Option<String>,
    syntax: SyntaxTable,
}

/// `[color.syntax]` に書かれた内容。
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SyntaxTable {
    comment: Option<String>,
    keyword: Option<String>,
    string: Option<String>,
    constant: Option<String>,
    function: Option<String>,
    r#type: Option<String>,
}

impl From<ColorTable> for ColorConfig {
    fn from(table: ColorTable) -> Self {
        let base = Self::from_theme(table.theme);
        let syntax = table.syntax;
        Self {
            theme: base.theme,
            background: table.background.unwrap_or(base.background),
            surface: table.surface.unwrap_or(base.surface),
            text: table.text.unwrap_or(base.text),
            muted: table.muted.unwrap_or(base.muted),
            border: table.border.unwrap_or(base.border),
            accent: table.accent.unwrap_or(base.accent),
            accent_soft: table.accent_soft.unwrap_or(base.accent_soft),
            link: table.link.unwrap_or(base.link),
            code: table.code.unwrap_or(base.code),
            note: table.note.unwrap_or(base.note),
            tip: table.tip.unwrap_or(base.tip),
            warning: table.warning.unwrap_or(base.warning),
            caution: table.caution.unwrap_or(base.caution),
            syntax: SyntaxColors {
                comment: syntax.comment.unwrap_or(base.syntax.comment),
                keyword: syntax.keyword.unwrap_or(base.syntax.keyword),
                string: syntax.string.unwrap_or(base.syntax.string),
                constant: syntax.constant.unwrap_or(base.syntax.constant),
                function: syntax.function.unwrap_or(base.syntax.function),
                r#type: syntax.r#type.unwrap_or(base.syntax.r#type),
            },
        }
    }
}

/// `[layout]`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    /// 本文の最大幅（px）。
    pub max_width: u32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self { max_width: 960 }
    }
}

/// `[rendering]`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderingConfig {
    /// 見出しの `##` や箇条書きの `-` を表示する。
    pub show_markers: bool,
    /// 文章中の句点 `。` の後に表示上の改行を入れる。
    pub break_after_kuten: bool,
    /// 文章中の長いパスとURLを短い表示に置き換える。
    pub abbreviate: bool,
    /// この文字数以上のパスとURLを省略の対象にする。
    pub abbreviate_min_chars: usize,
    /// 言語を書いたコードブロックを、字句の種類ごとに色分けする。
    pub highlight: bool,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            show_markers: true,
            break_after_kuten: true,
            abbreviate: true,
            abbreviate_min_chars: 48,
            highlight: true,
        }
    }
}

/// `[mermaid]`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MermaidConfig {
    pub enabled: bool,
    /// 書かれていなければ、配色に合わせて選ぶ（`Config::mermaid_theme`）。
    pub theme: Option<MermaidTheme>,
}

impl Default for MermaidConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            theme: None,
        }
    }
}

/// Mermaidに渡すテーマ名。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidTheme {
    Default,
    Neutral,
    Forest,
    Base,
    Dark,
}

impl MermaidTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Neutral => "neutral",
            Self::Forest => "forest",
            Self::Base => "base",
            Self::Dark => "dark",
        }
    }
}

/// 設定を読めなかった理由。
#[derive(Debug)]
pub struct ConfigError {
    /// 原因になった設定ファイル。文字列から直接読んだときは `None`。
    pub path: Option<PathBuf>,
    pub kind: ConfigErrorKind,
}

#[derive(Debug)]
pub enum ConfigErrorKind {
    /// ファイルを読めない。
    Read(io::Error),
    /// TOMLとして解釈できない、または未知の項目がある。
    Parse(String),
    /// 値が許される範囲にない。`key` は `font.size` のような項目名。
    Invalid { key: String, message: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "設定ファイル「{}」", path.display())?,
            None => write!(f, "設定")?,
        }
        match &self.kind {
            ConfigErrorKind::Read(error) => write!(f, "を読み込めません: {error}"),
            ConfigErrorKind::Parse(message) => write!(f, "を解釈できません: {message}"),
            ConfigErrorKind::Invalid { key, message } => {
                write!(f, "の {key} が正しくありません: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

const BODY_FALLBACKS: &[&str] = &[
    "LINE Seed JP",
    "LINE Seed JP_TTF",
    "LINE Seed JP_OTF",
    "Hiragino Sans",
    "Yu Gothic",
    "Meiryo",
];
const CODE_FALLBACKS: &[&str] = &[
    "HackGen",
    "HackGen Console",
    "HackGen35",
    "SFMono-Regular",
    "Menlo",
    "Consolas",
];

impl Config {
    /// TOMLの文字列から設定を作り、値を検証する。
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(|error| ConfigError {
            path: None,
            kind: ConfigErrorKind::Parse(error.to_string()),
        })?;
        config.validate()?;
        Ok(config)
    }

    /// 設定ファイルを読む。失敗の理由にはファイルのパスを含める。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|error| ConfigError {
            path: Some(path.to_path_buf()),
            kind: ConfigErrorKind::Read(error),
        })?;
        Self::from_toml_str(&text).map_err(|error| ConfigError {
            path: Some(path.to_path_buf()),
            ..error
        })
    }

    /// 値が許される範囲にあるかを調べ、最初に見つかった違反を返す。
    fn validate(&self) -> Result<(), ConfigError> {
        validate_font_name("font.body", &self.font.body)?;
        validate_font_name("font.code", &self.font.code)?;
        if !(10.0..=32.0).contains(&self.font.size) {
            return Err(invalid_value(
                "font.size",
                "10〜32 の範囲で指定してください",
            ));
        }
        if !(1.0..=3.0).contains(&self.font.line_height) {
            return Err(invalid_value(
                "font.line_height",
                "1.0〜3.0 の範囲で指定してください",
            ));
        }
        let colors = [
            ("color.background", &self.color.background),
            ("color.surface", &self.color.surface),
            ("color.text", &self.color.text),
            ("color.muted", &self.color.muted),
            ("color.border", &self.color.border),
            ("color.accent", &self.color.accent),
            ("color.accent_soft", &self.color.accent_soft),
            ("color.link", &self.color.link),
            ("color.code", &self.color.code),
            ("color.note", &self.color.note),
            ("color.tip", &self.color.tip),
            ("color.warning", &self.color.warning),
            ("color.caution", &self.color.caution),
            ("color.syntax.comment", &self.color.syntax.comment),
            ("color.syntax.keyword", &self.color.syntax.keyword),
            ("color.syntax.string", &self.color.syntax.string),
            ("color.syntax.constant", &self.color.syntax.constant),
            ("color.syntax.function", &self.color.syntax.function),
            ("color.syntax.type", &self.color.syntax.r#type),
        ];
        for (key, value) in colors {
            if !is_hex_color(value) {
                return Err(invalid_value(
                    key,
                    "#rgb、#rrggbb、#rrggbbaa のいずれかで指定してください",
                ));
            }
        }
        if !(320..=2400).contains(&self.layout.max_width) {
            return Err(invalid_value(
                "layout.max_width",
                "320〜2400 の範囲で指定してください",
            ));
        }
        if !(16..=400).contains(&self.rendering.abbreviate_min_chars) {
            return Err(invalid_value(
                "rendering.abbreviate_min_chars",
                "16〜400 の範囲で指定してください",
            ));
        }
        Ok(())
    }

    /// Mermaidに渡すテーマ。`mermaid.theme` が書かれていなければ、既定の配色では `neutral`、
    /// ほかの配色では図をページの色に合わせる `base` にする。
    pub fn mermaid_theme(&self) -> MermaidTheme {
        self.mermaid.theme.unwrap_or(match self.color.theme {
            ColorTheme::Default => MermaidTheme::Neutral,
            _ => MermaidTheme::Base,
        })
    }

    /// ページのCSSが参照する変数を `:root{...}` として返す。
    pub fn css_variables(&self) -> String {
        let font = &self.font;
        let color = &self.color;
        let color_scheme = if color.is_dark() { "dark" } else { "light" };
        let declarations = [
            // スクロールバーなど、ブラウザーが描く部品の明暗を背景に合わせる。
            ("color-scheme", color_scheme.to_string()),
            (
                "--mb-font-body",
                font_stack(&font.body, BODY_FALLBACKS, "system-ui, sans-serif"),
            ),
            (
                "--mb-font-code",
                font_stack(&font.code, CODE_FALLBACKS, "ui-monospace, monospace"),
            ),
            ("--mb-font-size", format!("{}px", font.size)),
            ("--mb-line-height", font.line_height.to_string()),
            ("--mb-max-width", format!("{}px", self.layout.max_width)),
            ("--mb-color-bg", css_safe(&color.background)),
            ("--mb-color-surface", css_safe(&color.surface)),
            ("--mb-color-text", css_safe(&color.text)),
            ("--mb-color-muted", css_safe(&color.muted)),
            ("--mb-color-border", css_safe(&color.border)),
            ("--mb-color-accent", css_safe(&color.accent)),
            ("--mb-color-accent-soft", css_safe(&color.accent_soft)),
            ("--mb-color-link", css_safe(&color.link)),
            ("--mb-color-code", css_safe(&color.code)),
            ("--mb-color-note", css_safe(&color.note)),
            ("--mb-color-tip", css_safe(&color.tip)),
            ("--mb-color-warning", css_safe(&color.warning)),
            ("--mb-color-caution", css_safe(&color.caution)),
            ("--mb-syntax-comment", css_safe(&color.syntax.comment)),
            ("--mb-syntax-keyword", css_safe(&color.syntax.keyword)),
            ("--mb-syntax-string", css_safe(&color.syntax.string)),
            ("--mb-syntax-constant", css_safe(&color.syntax.constant)),
            ("--mb-syntax-function", css_safe(&color.syntax.function)),
            ("--mb-syntax-type", css_safe(&color.syntax.r#type)),
        ];
        let body: String = declarations
            .iter()
            .map(|(name, value)| format!("{name}:{value};"))
            .collect();
        format!(":root{{{body}}}")
    }
}

/// 設定ファイルを探す。`cwd/miyaburu.toml`、次に `user_config_dir/miyaburu/config.toml`。
///
/// @doc docs/specs/configuration.md#設定ファイルの場所
pub fn find_config_file(cwd: &Path, user_config_dir: Option<&Path>) -> Option<PathBuf> {
    let project = cwd.join("miyaburu.toml");
    if project.is_file() {
        return Some(project);
    }
    let user = user_config_dir?.join("miyaburu").join("config.toml");
    user.is_file().then_some(user)
}

/// 値の違反を表す `ConfigError` を作る。
fn invalid_value(key: &str, message: &str) -> ConfigError {
    ConfigError {
        path: None,
        kind: ConfigErrorKind::Invalid {
            key: key.to_string(),
            message: message.to_string(),
        },
    }
}

/// フォント名が空でなく、100文字以下で、CSSを壊す文字を含まないかを調べる。
fn validate_font_name(key: &str, name: &str) -> Result<(), ConfigError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid_value(key, "フォント名が空です"));
    }
    if name.chars().count() > 100 {
        return Err(invalid_value(key, "フォント名は100文字までです"));
    }
    if name
        .chars()
        .any(|c| c.is_control() || "\"'\\;{}<>".contains(c))
    {
        return Err(invalid_value(
            key,
            "フォント名に使えない文字が含まれています",
        ));
    }
    Ok(())
}

/// `#` の後に16進数が3桁、6桁、8桁のいずれか続くかを調べる。
fn is_hex_color(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    matches!(digits.len(), 3 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
}

/// 指定のフォントを先頭にし、同梱しない前提の代替フォントを続ける。
fn font_stack(first: &str, fallbacks: &[&str], generic: &str) -> String {
    let first = css_safe(first.trim());
    let mut names = vec![first.as_str()];
    names.extend(fallbacks.iter().copied().filter(|name| *name != first));
    let quoted: Vec<String> = names
        .iter()
        .filter(|name| !name.is_empty())
        .map(|name| format!("\"{name}\""))
        .collect();
    format!("{}, {generic}", quoted.join(", "))
}

/// CSSの宣言を壊す文字を取り除く。検証前の値が渡っても `<style>` の外へ出さないため。
fn css_safe(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '"' | '\'' | '\\' | ';' | '{' | '}' | '<' | '>')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト終了時に消える一時ディレクトリ。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "miyaburu-config-test-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn invalid_key(text: &str) -> String {
        match Config::from_toml_str(text) {
            Err(ConfigError {
                kind: ConfigErrorKind::Invalid { key, .. },
                ..
            }) => key,
            other => panic!("Invalid のはずが {other:?}"),
        }
    }

    #[test]
    fn empty_text_is_default() {
        assert_eq!(Config::from_toml_str("").unwrap(), Config::default());
    }

    #[test]
    fn partial_text_keeps_other_defaults() {
        let config = Config::from_toml_str("[font]\nsize = 18\n").unwrap();
        let mut expected = Config::default();
        expected.font.size = 18.0;
        assert_eq!(config, expected);
        assert_eq!(config.font.size, 18.0);
    }

    #[test]
    fn unknown_key_is_parse_error() {
        let error = Config::from_toml_str("[font]\nwieght = 1\n").unwrap_err();
        assert!(matches!(error.kind, ConfigErrorKind::Parse(_)));
        assert!(error.path.is_none());
    }

    #[test]
    fn unknown_theme_is_parse_error() {
        let error = Config::from_toml_str("[mermaid]\ntheme = \"pink\"\n").unwrap_err();
        assert!(matches!(error.kind, ConfigErrorKind::Parse(_)));
    }

    #[test]
    fn known_theme_is_parsed() {
        let config = Config::from_toml_str("[mermaid]\ntheme = \"forest\"\n").unwrap();
        assert_eq!(config.mermaid.theme, Some(MermaidTheme::Forest));
    }

    #[test]
    fn color_theme_replaces_every_color() {
        let config = Config::from_toml_str("[color]\ntheme = \"dracula\"\n").unwrap();
        assert_eq!(config.color, ColorConfig::from_theme(ColorTheme::Dracula));
        assert_eq!(config.color.background, "#282a36");
        assert_ne!(config.color.caution, Config::default().color.caution);
    }

    #[test]
    fn written_colors_override_the_theme() {
        let config =
            Config::from_toml_str("[color]\naccent = \"#e0407a\"\ntheme = \"tokyo-night\"\n")
                .unwrap();
        let mut expected = ColorConfig::from_theme(ColorTheme::TokyoNight);
        expected.accent = "#e0407a".to_string();
        assert_eq!(config.color, expected);
    }

    #[test]
    fn syntax_colors_follow_the_theme_unless_written() {
        let themed = Config::from_toml_str("[color]\ntheme = \"dracula\"\n").unwrap();
        assert_eq!(themed.color.syntax.keyword, "#ff79c6");
        assert_ne!(themed.color.syntax, Config::default().color.syntax);

        let written = Config::from_toml_str(
            "[color]\ntheme = \"dracula\"\n[color.syntax]\ntype = \"#abc\"\ncomment = \"#123456\"\n",
        )
        .unwrap();
        let mut expected = ColorConfig::from_theme(ColorTheme::Dracula);
        expected.syntax.r#type = "#abc".to_string();
        expected.syntax.comment = "#123456".to_string();
        assert_eq!(written.color, expected);

        let css = written.css_variables();
        assert!(css.contains("--mb-syntax-type:#abc;"), "{css}");
        assert!(css.contains("--mb-syntax-keyword:#ff79c6;"), "{css}");
    }

    #[test]
    fn syntax_colors_are_validated_like_other_colors() {
        assert_eq!(
            invalid_key("[color.syntax]\nstring = \"green\"\n"),
            "color.syntax.string"
        );
        assert_eq!(
            invalid_key("[color.syntax]\ntype = \"#12\"\n"),
            "color.syntax.type"
        );
        let error = Config::from_toml_str("[color.syntax]\nnumber = \"#123456\"\n").unwrap_err();
        assert!(matches!(error.kind, ConfigErrorKind::Parse(_)), "{error:?}");
    }

    #[test]
    fn highlight_is_on_unless_turned_off() {
        assert!(Config::default().rendering.highlight);
        let config = Config::from_toml_str("[rendering]\nhighlight = false\n").unwrap();
        assert!(!config.rendering.highlight);
    }

    #[test]
    fn unknown_color_theme_is_parse_error_listing_the_names() {
        let error = Config::from_toml_str("[color]\ntheme = \"drakula\"\n").unwrap_err();
        let ConfigErrorKind::Parse(message) = error.kind else {
            panic!("Parse のはずが {error:?}");
        };
        assert!(message.contains("`tokyo-night`"), "{message}");
    }

    #[test]
    fn mermaid_theme_follows_the_color_theme_unless_written() {
        assert_eq!(Config::default().mermaid_theme(), MermaidTheme::Neutral);
        let themed = Config::from_toml_str("[color]\ntheme = \"nord\"\n").unwrap();
        assert_eq!(themed.mermaid_theme(), MermaidTheme::Base);
        let written =
            Config::from_toml_str("[color]\ntheme = \"nord\"\n[mermaid]\ntheme = \"dark\"\n")
                .unwrap();
        assert_eq!(written.mermaid_theme(), MermaidTheme::Dark);
    }

    #[test]
    fn color_scheme_follows_the_background() {
        let css = Config::default().css_variables();
        assert!(css.contains("color-scheme:light;"), "{css}");
        let css = Config::from_toml_str("[color]\ntheme = \"dracula\"\n")
            .unwrap()
            .css_variables();
        assert!(css.contains("color-scheme:dark;"), "{css}");
        assert!(css.contains("--mb-color-caution:#ff5555;"), "{css}");
        // 背景だけを書き換えた場合も、その背景で判断する。
        let css = Config::from_toml_str("[color]\nbackground = \"#101010\"\n")
            .unwrap()
            .css_variables();
        assert!(css.contains("color-scheme:dark;"), "{css}");
    }

    #[test]
    fn boundary_values_are_accepted() {
        let text = format!(
            "[font]\nbody = \"{}\"\nsize = 32.0\nline_height = 3.0\n\
             [color]\nbackground = \"#abc\"\n\
             [layout]\nmax_width = 2400\n\
             [rendering]\nabbreviate_min_chars = 16\n",
            "a".repeat(100)
        );
        assert!(Config::from_toml_str(&text).is_ok());
    }

    #[test]
    fn invalid_values_report_their_key() {
        let cases: Vec<(&str, String)> = vec![
            ("font.body", "[font]\nbody = \"\"\n".to_string()),
            ("font.body", "[font]\nbody = \"   \"\n".to_string()),
            (
                "font.body",
                format!("[font]\nbody = \"{}\"\n", "a".repeat(101)),
            ),
            ("font.body", "[font]\nbody = \"A\\\"B\"\n".to_string()),
            ("font.body", "[font]\nbody = \"A;B\"\n".to_string()),
            ("font.body", "[font]\nbody = \"A\\tB\"\n".to_string()),
            ("font.code", "[font]\ncode = \"A<B\"\n".to_string()),
            ("font.size", "[font]\nsize = 9.9\n".to_string()),
            ("font.size", "[font]\nsize = 32.1\n".to_string()),
            ("font.size", "[font]\nsize = nan\n".to_string()),
            (
                "font.line_height",
                "[font]\nline_height = 0.9\n".to_string(),
            ),
            (
                "font.line_height",
                "[font]\nline_height = 3.1\n".to_string(),
            ),
            (
                "layout.max_width",
                "[layout]\nmax_width = 319\n".to_string(),
            ),
            (
                "layout.max_width",
                "[layout]\nmax_width = 2401\n".to_string(),
            ),
            (
                "rendering.abbreviate_min_chars",
                "[rendering]\nabbreviate_min_chars = 15\n".to_string(),
            ),
            (
                "rendering.abbreviate_min_chars",
                "[rendering]\nabbreviate_min_chars = 401\n".to_string(),
            ),
        ];
        for (expected, text) in cases {
            assert_eq!(invalid_key(&text), expected, "{text}");
        }
        for key in [
            "background",
            "surface",
            "text",
            "muted",
            "border",
            "accent",
            "accent_soft",
            "link",
            "code",
            "note",
            "tip",
            "warning",
            "caution",
        ] {
            let text = format!("[color]\n{key} = \"#12345\"\n");
            assert_eq!(invalid_key(&text), format!("color.{key}"), "{text}");
        }
    }

    #[test]
    fn invalid_color_values_are_rejected() {
        for value in ["red", "#12345", "#gggggg", "#1234567", "ffffff"] {
            let text = format!("[color]\nbackground = \"{value}\"\n");
            assert_eq!(invalid_key(&text), "color.background", "{value}");
        }
    }

    #[test]
    fn load_reports_missing_file_as_read_error() {
        let dir = TempDir::new();
        let path = dir.0.join("missing.toml");
        let error = Config::load(&path).unwrap_err();
        assert!(matches!(error.kind, ConfigErrorKind::Read(_)));
        assert_eq!(error.path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn load_reports_invalid_file_with_its_path() {
        let dir = TempDir::new();
        let path = dir.0.join("miyaburu.toml");
        std::fs::write(&path, "[layout]\nmax_width = 100\n").unwrap();
        let error = Config::load(&path).unwrap_err();
        assert!(matches!(error.kind, ConfigErrorKind::Invalid { .. }));
        assert_eq!(error.path.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn load_reads_valid_file() {
        let dir = TempDir::new();
        let path = dir.0.join("miyaburu.toml");
        std::fs::write(&path, "[layout]\nmax_width = 800\n").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.layout.max_width, 800);
    }

    #[test]
    fn find_config_file_prefers_cwd_then_user_dir() {
        let dir = TempDir::new();
        let cwd = dir.0.join("project");
        let user_dir = dir.0.join("user");
        let user_file = user_dir.join("miyaburu").join("config.toml");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(user_file.parent().unwrap()).unwrap();

        assert_eq!(find_config_file(&cwd, Some(&user_dir)), None);

        std::fs::write(&user_file, "").unwrap();
        assert_eq!(
            find_config_file(&cwd, Some(&user_dir)),
            Some(user_file.clone())
        );
        assert_eq!(find_config_file(&cwd, None), None);

        let project_file = cwd.join("miyaburu.toml");
        std::fs::write(&project_file, "").unwrap();
        assert_eq!(find_config_file(&cwd, Some(&user_dir)), Some(project_file));
    }

    #[test]
    fn default_css_variables_include_colors_and_fonts() {
        let css = Config::default().css_variables();
        assert!(css.contains("--mb-color-bg:#ffffff;"), "{css}");
        assert!(
            css.contains("--mb-font-body:\"LINE Seed JP\", \"LINE Seed JP_TTF\""),
            "{css}"
        );
    }

    #[test]
    fn custom_body_font_comes_first() {
        let mut config = Config::default();
        config.font.body = "Noto Sans JP".to_string();
        let css = config.css_variables();
        assert!(
            css.contains("--mb-font-body:\"Noto Sans JP\", \"LINE Seed JP\""),
            "{css}"
        );
    }

    #[test]
    fn error_display_includes_path() {
        let error = ConfigError {
            path: Some(PathBuf::from("/tmp/miyaburu.toml")),
            kind: ConfigErrorKind::Parse("bad".to_string()),
        };
        assert!(error.to_string().contains("/tmp/miyaburu.toml"));
    }
}
