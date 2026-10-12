//! コードブロックのシンタックスハイライト。
//!
//! 構文の解析は syntect に任せ、ここでは字句を少数の種類に分けて `<span>` で囲む。
//! 色は付けない。種類ごとの色は、ページのCSSが配色から決める。

use std::sync::OnceLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// これより長いコードは色分けしない。表示までの時間を、文書の長さに見合う範囲に収める。
const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

/// 色を分ける字句の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Comment,
    Keyword,
    String,
    /// 数値、真偽値、エスケープなどの定数。
    Constant,
    Function,
    Type,
    /// 差分で足された行。
    Inserted,
    /// 差分で消された行。
    Deleted,
}

impl Token {
    fn class(self) -> &'static str {
        match self {
            Self::Comment => "tok-comment",
            Self::Keyword => "tok-keyword",
            Self::String => "tok-string",
            Self::Constant => "tok-constant",
            Self::Function => "tok-function",
            Self::Type => "tok-type",
            Self::Inserted => "tok-inserted",
            Self::Deleted => "tok-deleted",
        }
    }
}

/// スコープの接頭辞と、その字句の種類。
///
/// 字句には内側から順にスコープが重なっている。内側から見て、最初に当てはまった行で種類を決める。
/// 1つのスコープに複数の行が当てはまるときは、接頭辞の長い行を取る。
/// `None` の行は、外側のスコープの色を引き継がず、地の色に戻す。
const RULES: &[(&str, Option<Token>)] = &[
    ("comment", Some(Token::Comment)),
    // 差分の `@@ -1,3 +1,4 @@` や `--- a/src/main.rs` の行。
    ("meta.diff", Some(Token::Comment)),
    // その後ろに続く、変更箇所を含む関数などの名前。
    ("entity.name.section.diff", None),
    ("string", Some(Token::String)),
    ("markup.raw", Some(Token::String)),
    ("constant", Some(Token::Constant)),
    ("support.constant", Some(Token::Constant)),
    ("entity.name.constant", Some(Token::Constant)),
    // `self` や `this`。
    ("variable.language", Some(Token::Constant)),
    // `4px` の `px`。数値と同じ色にする。
    ("keyword.other.unit", Some(Token::Constant)),
    ("keyword", Some(Token::Keyword)),
    ("storage", Some(Token::Keyword)),
    // HTMLのタグ名と、YAMLやTOMLのキー。
    ("entity.name.tag", Some(Token::Keyword)),
    ("entity.name.section", Some(Token::Keyword)),
    ("markup.heading", Some(Token::Keyword)),
    ("entity.name.function", Some(Token::Function)),
    ("support.function", Some(Token::Function)),
    ("support.macro", Some(Token::Function)),
    ("variable.function", Some(Token::Function)),
    ("variable.annotation", Some(Token::Function)),
    // HTMLの属性名と、CSSのセレクター。
    ("entity.other.attribute-name", Some(Token::Function)),
    ("entity.name", Some(Token::Type)),
    ("entity.other.inherited-class", Some(Token::Type)),
    ("support.type", Some(Token::Type)),
    ("support.class", Some(Token::Type)),
    ("markup.inserted", Some(Token::Inserted)),
    ("markup.deleted", Some(Token::Deleted)),
    // 変数と引数の名前には色を付けない。文字列の中に書いた変数も、文字列の色にしない。
    ("variable", None),
    ("entity.name.variable", None),
    // 文字列に埋め込まれた式と変数は、文字列の色にしない。
    ("meta.interpolation", None),
    ("meta.embedded", None),
    ("meta.template.expression", None),
    ("meta.group.expansion", None),
    ("source", None),
];

/// 演算子のスコープ。`&&` のような記号だけの演算子には色を付けず、色の数を抑える。
/// `and` や `as` のように単語で書く演算子は、キーワードの色にする。
const OPERATOR: &str = "keyword.operator";

/// 文字列として解析されるキーに付くスコープ。JSONのキーを、YAMLやTOMLのキーと同じ色にする。
const MAPPING_KEY: &str = "meta.mapping.key";

/// Markdownでよく書かれる言語名のうち、構文定義の名前や拡張子と一致しないもの。
/// 右は、構文定義の名前か拡張子。
const ALIASES: &[(&str, &str)] = &[
    ("shell", "sh"),
    ("golang", "go"),
    ("python3", "py"),
    ("mjs", "js"),
    ("cjs", "js"),
    // JSXの構文定義はないので、JSXを含むTypeScriptの定義で読む。
    ("jsx", "tsx"),
    ("csharp", "C#"),
    ("fsharp", "F#"),
    ("objc", "Objective-C"),
    ("docker", "Dockerfile"),
    ("jsonc", "json"),
    ("json5", "json"),
    ("postgres", "sql"),
    ("postgresql", "sql"),
    ("mysql", "sql"),
    ("sqlite", "sql"),
    ("viml", "vim"),
    ("elisp", "el"),
    ("batch", "bat"),
    ("apache", "Apache Conf"),
];

struct Highlighter {
    syntaxes: SyntaxSet,
    rules: Vec<(Scope, Option<Token>)>,
    operator: Scope,
    mapping_key: Scope,
}

/// 構文定義は、色分けするコードブロックが最初に現れたときに一度だけ読み込む。
fn highlighter() -> Option<&'static Highlighter> {
    static HIGHLIGHTER: OnceLock<Option<Highlighter>> = OnceLock::new();
    HIGHLIGHTER
        .get_or_init(|| {
            let rules = RULES
                .iter()
                .map(|&(prefix, token)| Some((Scope::new(prefix).ok()?, token)))
                .collect::<Option<Vec<_>>>()?;
            Some(Highlighter {
                syntaxes: two_face::syntax::extra_newlines(),
                rules,
                operator: Scope::new(OPERATOR).ok()?,
                mapping_key: Scope::new(MAPPING_KEY).ok()?,
            })
        })
        .as_ref()
}

impl Highlighter {
    /// コードブロックの言語名から構文定義を探す。`rust,ignore` のような指定は、最初の名前で探す。
    fn find_syntax(&self, lang: &str) -> Option<&SyntaxReference> {
        let name = lang.split(',').next().unwrap_or(lang).trim();
        if name.is_empty() {
            return None;
        }
        let lower = name.to_ascii_lowercase();
        let token = ALIASES
            .iter()
            .find(|(alias, _)| *alias == lower)
            .map_or(name, |(_, token)| token);
        let syntax = self.syntaxes.find_syntax_by_token(token)?;
        // 色を付ける字句がない。
        (syntax.name != "Plain Text").then_some(syntax)
    }

    /// 重なったスコープから、字句の種類を決める。
    fn classify(&self, stack: &ScopeStack, text: &str) -> Option<Token> {
        let scopes = stack.as_slice();
        let (scope, token) = scopes.iter().rev().find_map(|&scope| {
            self.rules
                .iter()
                .filter(|(prefix, _)| prefix.is_prefix_of(scope))
                .max_by_key(|(prefix, _)| prefix.len())
                .map(|&(_, token)| (scope, token))
        })?;
        match token? {
            Token::Keyword
                if self.operator.is_prefix_of(scope)
                    && !text.chars().any(char::is_alphanumeric) =>
            {
                None
            }
            Token::String
                if scopes
                    .iter()
                    .any(|&scope| self.mapping_key.is_prefix_of(scope)) =>
            {
                Some(Token::Keyword)
            }
            token => Some(token),
        }
    }
}

/// コードを、字句の種類ごとに `<span class="tok-…">` で囲んだHTML断片にする。
///
/// 言語名に対応する構文定義がないとき、コードが長すぎるとき、解析に失敗したときは `None` を返す。
/// 返すHTMLから要素を除くと、エスケープした元のコードと一致する。
///
/// @doc docs/specs/rendering.md#シンタックスハイライト
pub(crate) fn highlight(lang: &str, code: &str) -> Option<String> {
    if code.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }
    let highlighter = highlighter()?;
    let syntax = highlighter.find_syntax(lang)?;
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut painter = Painter::new(code.len());
    for line in LinesWithEndings::from(code) {
        let ops = state.parse_line(line, &highlighter.syntaxes).ok()?;
        let mut start = 0;
        for (offset, op) in &ops {
            if *offset > start {
                let text = line.get(start..*offset)?;
                painter.push(text, highlighter.classify(&stack, text));
                start = *offset;
            }
            stack.apply(op).ok()?;
        }
        let text = line.get(start..)?;
        painter.push(text, highlighter.classify(&stack, text));
    }
    Some(painter.finish())
}

/// 同じ種類が続く字句を、1つの `<span>` にまとめて書き出す。
struct Painter {
    out: String,
    open: Option<Token>,
}

impl Painter {
    fn new(code_len: usize) -> Self {
        Self {
            out: String::with_capacity(code_len * 2),
            open: None,
        }
    }

    fn push(&mut self, text: &str, token: Option<Token>) {
        // 空白は色が見えないので、開いている `<span>` に含めて、要素の数を増やさない。
        if text.trim().is_empty() {
            push_escaped(&mut self.out, text);
            return;
        }
        if token != self.open {
            if self.open.is_some() {
                self.out.push_str("</span>");
            }
            if let Some(token) = token {
                self.out.push_str("<span class=\"");
                self.out.push_str(token.class());
                self.out.push_str("\">");
            }
            self.open = token;
        }
        push_escaped(&mut self.out, text);
    }

    fn finish(mut self) -> String {
        if self.open.is_some() {
            self.out.push_str("</span>");
        }
        self.out
    }
}

fn push_escaped(out: &mut String, text: &str) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escaped(text: &str) -> String {
        let mut out = String::new();
        push_escaped(&mut out, text);
        out
    }

    /// `<span …>` と `</span>` を取り除く。
    fn without_spans(html: &str) -> String {
        let mut out = String::new();
        let mut rest = html;
        while let Some(start) = rest.find('<') {
            out.push_str(&rest[..start]);
            let end = rest[start..].find('>').unwrap();
            rest = &rest[start + end + 1..];
        }
        out.push_str(rest);
        out
    }

    #[test]
    fn tokens_are_wrapped_by_kind() {
        assert_eq!(
            highlight("rust", "let a = 1; // 注\n").unwrap(),
            "<span class=\"tok-keyword\">let</span> a = <span class=\"tok-constant\">1</span>; \
             <span class=\"tok-comment\">// 注\n</span>"
        );
        let out = highlight("rust", "fn main() { println!(\"a\\n\"); }\n").unwrap();
        assert!(out.contains("<span class=\"tok-keyword\">fn </span>"));
        assert!(out.contains("<span class=\"tok-function\">main</span>"));
        assert!(out.contains("<span class=\"tok-function\">println!</span>"));
        assert!(out.contains("<span class=\"tok-string\">&quot;a</span>"));
        // 文字列の中のエスケープは、定数の色にする。
        assert!(out.contains("<span class=\"tok-constant\">\\n</span>"));
        let out = highlight("ts", "class Repo extends Base {}\n").unwrap();
        assert!(out.contains("<span class=\"tok-type\">Repo </span>"));
    }

    #[test]
    fn removing_the_spans_gives_back_the_escaped_code() {
        let cases = [
            (
                "rust",
                "fn f<'a>(x: &'a str) -> bool { x < \"b\" && x != \"<&>\" }\n",
            ),
            (
                "html",
                "<a href=\"x?a=1&amp;b=2\" title='t'>文字 &lt;</a>\n<script>if (a < b) {}</script>",
            ),
            ("sh", "echo \"hello $name\" | tee -a out.log\n"),
            ("json", "{\n  \"k\": [1, \"v\", null]\n}"),
            ("diff", "@@ -1 +1 @@\n-a\n+b\n"),
            ("py", "def f(a):\n    return f\"{a!r}\"  # 注\n"),
            ("css", ".a > b { margin: 0 4px; }\r\n"),
            ("yaml", "a:\n  - b: \"c\"\n"),
            ("toml", "[a]\nb = 'c'\n"),
            ("md", "# 見出し\n\n`code` と **強調**\n"),
            ("rust", ""),
        ];
        for (lang, code) in cases {
            let out = highlight(lang, code).unwrap();
            assert_eq!(without_spans(&out), escaped(code), "{lang}");
        }
    }

    #[test]
    fn unknown_language_is_left_alone() {
        assert_eq!(highlight("no-such-language", "x"), None);
        assert_eq!(highlight("", "x"), None);
        assert_eq!(highlight("mermaid", "graph TD;"), None);
        // 色を付ける字句がない言語。
        assert_eq!(highlight("txt", "x"), None);
        assert_eq!(highlight("text", "x"), None);
    }

    #[test]
    fn language_names_follow_the_habits_of_markdown() {
        let highlighter = highlighter().unwrap();
        let name = |lang: &str| {
            highlighter
                .find_syntax(lang)
                .map(|syntax| syntax.name.as_str())
        };
        for lang in [
            "rust",
            "rs",
            "sh",
            "bash",
            "zsh",
            "shell",
            "js",
            "javascript",
            "jsx",
            "ts",
            "typescript",
            "tsx",
            "py",
            "python",
            "rb",
            "ruby",
            "go",
            "golang",
            "c",
            "cpp",
            "c++",
            "cs",
            "csharp",
            "java",
            "kotlin",
            "kt",
            "swift",
            "dart",
            "php",
            "html",
            "xml",
            "css",
            "scss",
            "json",
            "jsonc",
            "yaml",
            "yml",
            "toml",
            "ini",
            "sql",
            "diff",
            "patch",
            "md",
            "markdown",
            "dockerfile",
            "docker",
            "makefile",
            "nix",
            "lua",
            "graphql",
            "proto",
        ] {
            assert!(name(lang).is_some(), "{lang}");
        }
        assert_eq!(name("Rust"), Some("Rust"));
        assert_eq!(name("JSON"), Some("JSON"));
        // rustdoc の書き方。
        assert_eq!(name("rust,ignore"), Some("Rust"));
        assert_eq!(name("shell"), Some("Bourne Again Shell (bash)"));
        assert_eq!(name("jsx"), Some("TypeScriptReact"));
        assert_eq!(name("csharp"), Some("C#"));
        assert_eq!(name("fsharp"), Some("F#"));
        assert_eq!(name("objc"), Some("Objective-C"));
    }

    #[test]
    fn only_operators_written_as_words_are_colored() {
        assert_eq!(highlight("rust", "a && b\n").unwrap(), "a &amp;&amp; b\n");
        let out = highlight("py", "a and b\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-keyword\">and </span>"),
            "{out}"
        );
        // 演算子でない記号は、キーワードの一部として色を付ける。
        let out = highlight("css", "@media print {}\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-keyword\">@media </span>"),
            "{out}"
        );
    }

    #[test]
    fn keys_are_told_apart_from_string_values() {
        assert_eq!(
            highlight("json", "{\"k\": \"v\"}").unwrap(),
            "{<span class=\"tok-keyword\">&quot;k&quot;</span>: \
             <span class=\"tok-string\">&quot;v&quot;</span>}"
        );
        let out = highlight("yaml", "k: \"v\"\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-keyword\">k</span>"),
            "{out}"
        );
        let out = highlight("toml", "k = \"v\"\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-keyword\">k </span>"),
            "{out}"
        );
    }

    #[test]
    fn diff_marks_inserted_and_deleted_lines() {
        assert_eq!(
            highlight("diff", "@@ -1,2 +1,2 @@ fn main() {\n a\n-b\n+c\n").unwrap(),
            "<span class=\"tok-comment\">@@ -1,2 +1,2 @@ </span>fn main() {\n a\n\
             <span class=\"tok-deleted\">-b\n</span><span class=\"tok-inserted\">+c\n</span>"
        );
    }

    #[test]
    fn expressions_embedded_in_a_string_are_not_colored_as_the_string() {
        let out = highlight("sh", "echo \"a $b\"\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-string\">&quot;a </span>$b"),
            "{out}"
        );
        let out = highlight("ts", "const s = `a ${b}`;\n").unwrap();
        assert!(
            out.contains("</span>${b}<span class=\"tok-string\">`</span>"),
            "{out}"
        );
        // 埋め込みを示すスコープがなく、変数のスコープだけが付く言語。
        let out = highlight("php", "<?php echo \"a $b\";\n").unwrap();
        assert!(
            out.contains("<span class=\"tok-string\">&quot;a </span>$b"),
            "{out}"
        );
    }

    #[test]
    fn names_of_variables_are_not_colored() {
        // 宣言した変数の名前に `entity.name` が付く言語。
        let out = highlight("kotlin", "val name = 1\n").unwrap();
        assert!(out.contains("</span>name = "), "{out}");
        let out = highlight("rust", "fn f(count: u32) { let total = count; }\n").unwrap();
        assert!(out.contains("(count: "), "{out}");
        assert!(out.contains(" total = count; }"), "{out}");
    }

    #[test]
    fn too_long_code_is_left_alone() {
        assert!(highlight("rust", "let a = 1;\n").is_some());
        let code = "let a = 1;\n".repeat(MAX_HIGHLIGHT_BYTES / 11 + 1);
        assert_eq!(highlight("rust", &code), None);
    }
}
