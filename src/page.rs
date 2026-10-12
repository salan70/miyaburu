//! 変換結果と設定から、単体で開けるHTMLページを組み立てる。

use crate::config::Config;
use crate::render::Rendered;

/// 同梱しているMermaidのスクリプト。図を含むページの隣に書き出して使う。
pub const MERMAID_JS: &[u8] = include_bytes!("../assets/vendor/mermaid.min.js");

/// ページから相対パスで参照するMermaidのファイル名。
pub const MERMAID_FILE_NAME: &str = "mermaid.min.js";

/// ページの組み立てに使う入力。
#[derive(Debug, Clone, Copy)]
pub struct PageInput<'a> {
    pub rendered: &'a Rendered,
    pub config: &'a Config,
    /// 見出しがない文書の題名に使うファイル名。
    pub document_name: &'a str,
    /// Content-Security-Policyと `<script>` に付けるnonce（英数字のみ）。
    pub nonce: &'a str,
}

const PREVIEW_CSS: &str = include_str!("../assets/preview.css");
const PREVIEW_JS: &str = include_str!("../assets/preview.js");

/// 完成したHTML文書を返す。
///
/// @doc docs/specs/rendering.md#ページの構成
pub fn build_page(input: &PageInput<'_>) -> String {
    let rendered = input.rendered;
    let nonce = escape_html(input.nonce);
    let title = escape_html(rendered.title.as_deref().unwrap_or(input.document_name));
    let theme = input.config.mermaid.theme.as_str();
    let variables = input.config.css_variables();
    let mermaid_script = if rendered.has_mermaid {
        format!("<script nonce=\"{nonce}\" src=\"{MERMAID_FILE_NAME}\"></script>\n")
    } else {
        String::new()
    };

    format!(
        "<!doctype html>\n\
<html lang=\"ja\">\n\
<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src file: data: https: http:; style-src 'unsafe-inline'; font-src file: data:; script-src 'nonce-{nonce}'\">\n\
<meta name=\"referrer\" content=\"no-referrer\">\n\
<title>{title}</title>\n\
<style>{variables}\n{PREVIEW_CSS}</style>\n\
</head>\n\
<body data-mermaid-theme=\"{theme}\">\n\
<main class=\"document\">{body}</main>\n\
{mermaid_script}<script nonce=\"{nonce}\">{PREVIEW_JS}</script>\n\
</body>\n\
</html>\n",
        body = rendered.body_html,
    )
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(rendered: &Rendered, config: &Config) -> String {
        build_page(&PageInput {
            rendered,
            config,
            document_name: "memo.md",
            nonce: "abc123",
        })
    }

    fn rendered(title: Option<&str>, has_mermaid: bool) -> Rendered {
        Rendered {
            body_html: "<p>本文 &amp; <em>強調</em></p>".to_string(),
            title: title.map(str::to_string),
            has_mermaid,
        }
    }

    #[test]
    fn starts_with_doctype_and_uses_nonce_in_csp_and_script() {
        let html = page(&rendered(None, false), &Config::default());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("script-src 'nonce-abc123'"));
        assert!(html.contains("<script nonce=\"abc123\">"));
    }

    #[test]
    fn mermaid_script_only_when_has_mermaid() {
        let config = Config::default();
        assert!(!page(&rendered(None, false), &config).contains("src=\"mermaid.min.js\""));
        let html = page(&rendered(None, true), &config);
        assert!(html.contains("<script nonce=\"abc123\" src=\"mermaid.min.js\"></script>"));
    }

    #[test]
    fn title_is_escaped_and_falls_back_to_document_name() {
        let config = Config::default();
        let html = page(&rendered(Some("<b>題</b>"), false), &config);
        assert!(html.contains("<title>&lt;b&gt;題&lt;/b&gt;</title>"));
        let html = page(&rendered(None, false), &config);
        assert!(html.contains("<title>memo.md</title>"));
    }

    #[test]
    fn body_carries_mermaid_theme() {
        let config = Config::default();
        let html = page(&rendered(None, false), &config);
        let attr = format!("data-mermaid-theme=\"{}\"", config.mermaid.theme.as_str());
        assert!(html.contains(&attr));
    }

    #[test]
    fn embeds_css_variables_and_body_html_verbatim() {
        let config = Config::default();
        let r = rendered(None, false);
        let html = page(&r, &config);
        assert!(html.contains(&config.css_variables()));
        assert!(html.contains(&r.body_html));
    }

    #[test]
    fn assets_are_safe_to_inline_and_offline() {
        assert!(!PREVIEW_JS.contains("</script"));
        assert!(!PREVIEW_CSS.contains("</style"));
        for asset in [PREVIEW_JS, PREVIEW_CSS] {
            assert!(!asset.contains("http://"));
            assert!(!asset.contains("https://"));
        }
    }
}
