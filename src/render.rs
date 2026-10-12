//! Markdownを表示用のHTML断片へ変換する。

use std::collections::HashSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd,
};

/// 変換時の切り替え。
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// 見出しの `##` や箇条書きの `-` を表示する。
    pub show_markers: bool,
    /// 文章中の句点 `。` の後に表示上の改行を入れる。
    pub break_after_kuten: bool,
    /// 文章中の長いパスとURLを短い表示に置き換える。
    pub abbreviate: bool,
    /// この文字数以上のパスとURLを省略の対象にする。
    pub abbreviate_min_chars: usize,
    /// ` ```mermaid ` のコードブロックを図として扱う。
    pub mermaid: bool,
    /// 相対リンクと相対画像の基準にするディレクトリ（絶対パス）。
    pub base_dir: Option<PathBuf>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            show_markers: true,
            break_after_kuten: true,
            abbreviate: true,
            abbreviate_min_chars: 48,
            mermaid: true,
            base_dir: None,
        }
    }
}

/// 変換結果。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rendered {
    /// `<body>` の中に置くHTML断片。
    pub body_html: String,
    /// 最初のレベル1見出しの文字列。
    pub title: Option<String>,
    /// Mermaidの図を1つ以上含む。
    pub has_mermaid: bool,
}

/// Markdownの原文を表示用のHTML断片へ変換する。原文は書き換えない。
///
/// @doc docs/specs/rendering.md#変換の規則
pub fn render_markdown(source: &str, options: &RenderOptions) -> Rendered {
    let parser_options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(source, parser_options)
        .into_offset_iter()
        .collect();
    let mut renderer = Renderer {
        src: source,
        opts: options,
        out: String::new(),
        pending: None,
        in_heading: false,
        in_cell: false,
        in_head: false,
        links: Vec::new(),
        aligns: Vec::new(),
        col: 0,
        lists: Vec::new(),
        used_ids: HashSet::new(),
        title: None,
        has_mermaid: false,
    };
    let mut i = 0;
    while i < events.len() {
        i = renderer.step(&events, i);
    }
    Rendered {
        body_html: renderer.out,
        title: renderer.title,
        has_mermaid: renderer.has_mermaid,
    }
}

const KUTEN_BREAK: &str = "<br class=\"kuten-break\">";
const KUTEN_CLOSERS: &str = "」』）)】］]｝}〉》”’\"'";

type Events<'a> = [(Event<'a>, Range<usize>)];

/// 地の文の断片。
enum Seg<'t> {
    Plain(&'t str),
    Url(&'t str),
    Path(&'t str),
}

struct ListState {
    ordered: bool,
    next: u64,
}

struct Renderer<'a, 'o> {
    src: &'a str,
    opts: &'o RenderOptions,
    out: String,
    /// 直前の文が `。` で終わり、改行をまだ出していない。
    pending: Option<usize>,
    in_heading: bool,
    in_cell: bool,
    in_head: bool,
    /// リンクごとに `<a>` を出したか。
    links: Vec<bool>,
    aligns: Vec<Alignment>,
    col: usize,
    lists: Vec<ListState>,
    used_ids: HashSet<String>,
    title: Option<String>,
    has_mermaid: bool,
}

impl Renderer<'_, '_> {
    /// `i` 番目のイベントを処理し、次に処理するイベントの位置を返す。
    fn step(&mut self, events: &Events, i: usize) -> usize {
        match &events[i].0 {
            Event::Start(tag) => self.start(tag, &events[i].1, events, i),
            Event::End(end) => {
                self.end(end);
                i + 1
            }
            Event::Text(t) => {
                // パーサーは `_` などの位置で文字を分けるので、続く文字をつなげる。
                let mut joined = t.to_string();
                let mut next = i + 1;
                while let Some((Event::Text(more), _)) = events.get(next) {
                    joined.push_str(more);
                    next += 1;
                }
                self.text(&joined);
                next
            }
            Event::Code(c) => {
                self.resolve_pending();
                self.out.push_str("<code>");
                self.out.push_str(&escape(c));
                self.out.push_str("</code>");
                i + 1
            }
            Event::Html(h) | Event::InlineHtml(h) => {
                self.resolve_pending();
                self.out.push_str(&escape(h));
                i + 1
            }
            Event::FootnoteReference(label) => {
                self.resolve_pending();
                let label = escape(label);
                self.out.push_str(&format!(
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{label}\" id=\"fnref-{label}\">{label}</a></sup>"
                ));
                i + 1
            }
            Event::SoftBreak => {
                if self.pending.is_some() {
                    self.resolve_pending();
                } else {
                    self.out.push('\n');
                }
                i + 1
            }
            Event::HardBreak => {
                self.pending = None;
                self.out.push_str("<br>");
                i + 1
            }
            Event::Rule => {
                self.pending = None;
                self.out.push_str("<hr>\n");
                i + 1
            }
            Event::TaskListMarker(checked) => {
                self.out.push_str(if *checked {
                    "<input type=\"checkbox\" disabled checked>"
                } else {
                    "<input type=\"checkbox\" disabled>"
                });
                i + 1
            }
            _ => i + 1,
        }
    }

    fn start(&mut self, tag: &Tag, range: &Range<usize>, events: &Events, i: usize) -> usize {
        match tag {
            Tag::Paragraph => {
                self.pending = None;
                self.out.push_str("<p>");
            }
            Tag::Heading { level, .. } => {
                self.pending = None;
                let (text, end) = collect_plain(events, i + 1);
                let n = *level as usize;
                if n == 1 && self.title.is_none() {
                    self.title = Some(text.clone());
                }
                let id = self.unique_id(&slugify(&text));
                self.out.push_str(&format!("<h{n} id=\"{}\">", escape(&id)));
                if self.opts.show_markers {
                    self.marker(&format!("{} ", "#".repeat(n)));
                }
                self.in_heading = true;
                return self.inner_until(events, i + 1, end);
            }
            Tag::BlockQuote(kind) => {
                self.pending = None;
                let class = match kind {
                    Some(BlockQuoteKind::Note) => " class=\"alert alert-note\"",
                    Some(BlockQuoteKind::Tip) => " class=\"alert alert-tip\"",
                    Some(BlockQuoteKind::Important) => " class=\"alert alert-important\"",
                    Some(BlockQuoteKind::Warning) => " class=\"alert alert-warning\"",
                    Some(BlockQuoteKind::Caution) => " class=\"alert alert-caution\"",
                    None => "",
                };
                self.out.push_str(&format!("<blockquote{class}>"));
            }
            Tag::CodeBlock(kind) => {
                self.pending = None;
                let (code, end) = collect_raw(events, i + 1);
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
                    CodeBlockKind::Indented => "",
                };
                if lang == "mermaid" && self.opts.mermaid {
                    self.has_mermaid = true;
                    self.out
                        .push_str(&format!("<pre class=\"mermaid\">{}</pre>\n", escape(&code)));
                } else if lang.is_empty() {
                    self.out.push_str(&format!(
                        "<pre class=\"code-block\"><code>{}</code></pre>\n",
                        escape(&code)
                    ));
                } else {
                    self.out.push_str(&format!(
                        "<pre class=\"code-block\"><code class=\"language-{}\">{}</code></pre>\n",
                        escape(lang),
                        escape(&code)
                    ));
                }
                return end + 1;
            }
            Tag::HtmlBlock => {
                self.pending = None;
                let (html, end) = collect_raw(events, i + 1);
                self.out.push_str(&format!(
                    "<pre class=\"code-block raw-html\"><code>{}</code></pre>\n",
                    escape(html.trim_end_matches('\n'))
                ));
                return end + 1;
            }
            Tag::List(start) => {
                self.pending = None;
                let class = if self.opts.show_markers {
                    " class=\"md-list\""
                } else {
                    ""
                };
                match start {
                    Some(n) => {
                        self.out.push_str(&format!("<ol{class} start=\"{n}\">"));
                        self.lists.push(ListState {
                            ordered: true,
                            next: *n,
                        });
                    }
                    None => {
                        self.out.push_str(&format!("<ul{class}>"));
                        self.lists.push(ListState {
                            ordered: false,
                            next: 0,
                        });
                    }
                }
            }
            Tag::Item => {
                self.pending = None;
                let task = matches!(events.get(i + 1), Some((Event::TaskListMarker(_), _)));
                self.out
                    .push_str(if task { "<li class=\"task\">" } else { "<li>" });
                let text = self.item_marker(range.start);
                if self.opts.show_markers {
                    self.marker(&text);
                }
                self.out.push_str("<div class=\"li-body\">");
            }
            Tag::FootnoteDefinition(label) => {
                self.pending = None;
                let label = escape(label);
                self.out.push_str(&format!(
                    "<div class=\"footnote\" id=\"fn-{label}\"><span class=\"footnote-label\">{label}</span><div class=\"footnote-body\">"
                ));
            }
            Tag::Table(aligns) => {
                self.pending = None;
                self.aligns = aligns.clone();
                self.out.push_str("<div class=\"table-wrap\"><table>");
            }
            Tag::TableHead => {
                self.in_head = true;
                self.col = 0;
                self.out.push_str("<thead><tr>");
            }
            Tag::TableRow => {
                self.col = 0;
                self.out.push_str("<tr>");
            }
            Tag::TableCell => {
                self.pending = None;
                self.in_cell = true;
                let class = match self.aligns.get(self.col) {
                    Some(Alignment::Left) => " class=\"align-left\"",
                    Some(Alignment::Center) => " class=\"align-center\"",
                    Some(Alignment::Right) => " class=\"align-right\"",
                    _ => "",
                };
                self.col += 1;
                let name = if self.in_head { "th" } else { "td" };
                self.out.push_str(&format!("<{name}{class}>"));
            }
            Tag::Emphasis => self.inline_open("<em>"),
            Tag::Strong => self.inline_open("<strong>"),
            Tag::Strikethrough => self.inline_open("<del>"),
            Tag::Superscript => self.inline_open("<sup>"),
            Tag::Subscript => self.inline_open("<sub>"),
            Tag::Link {
                link_type,
                dest_url,
                ..
            } => {
                if let (Some((Event::Text(t), _)), Some((Event::End(TagEnd::Link), _))) =
                    (events.get(i + 1), events.get(i + 2))
                    && is_http(dest_url)
                    && (t.as_ref() == dest_url.as_ref() || *link_type == LinkType::Autolink)
                {
                    self.resolve_pending();
                    self.url_atom(dest_url);
                    return i + 3;
                }
                match resolve_url(dest_url, self.opts.base_dir.as_deref()) {
                    Some(href) => {
                        self.out
                            .push_str(&format!("<a href=\"{}\">", escape(&href)));
                        self.links.push(true);
                    }
                    None => self.links.push(false),
                }
            }
            Tag::Image { dest_url, .. } => {
                self.resolve_pending();
                let (alt, end) = collect_plain(events, i + 1);
                match resolve_url(dest_url, self.opts.base_dir.as_deref()) {
                    Some(src) => self.out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">",
                        escape(&src),
                        escape(&alt)
                    )),
                    None => self.out.push_str(&escape(&alt)),
                }
                return end + 1;
            }
            _ => {}
        }
        i + 1
    }

    fn end(&mut self, end: &TagEnd) {
        self.end_tag(end);
        // 閉じタグの後に改行を入れる。
        if matches!(
            end,
            TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Superscript
                | TagEnd::Subscript
                | TagEnd::Link
        ) && self.pending.is_some()
        {
            self.pending = Some(self.out.len());
        }
    }

    fn end_tag(&mut self, end: &TagEnd) {
        match end {
            TagEnd::Paragraph => {
                self.pending = None;
                self.out.push_str("</p>\n");
            }
            TagEnd::Heading(level) => {
                self.in_heading = false;
                self.out.push_str(&format!("</h{}>\n", *level as usize));
            }
            TagEnd::BlockQuote(_) => {
                self.pending = None;
                self.out.push_str("</blockquote>\n");
            }
            TagEnd::List(_) => {
                self.pending = None;
                let ordered = self.lists.pop().is_some_and(|l| l.ordered);
                self.out
                    .push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
            }
            TagEnd::Item => {
                self.pending = None;
                self.out.push_str("</div></li>\n");
            }
            TagEnd::FootnoteDefinition => {
                self.pending = None;
                self.out.push_str("</div></div>\n");
            }
            TagEnd::Table => self.out.push_str("</tbody></table></div>\n"),
            TagEnd::TableHead => {
                self.in_head = false;
                self.out.push_str("</tr></thead><tbody>");
            }
            TagEnd::TableRow => self.out.push_str("</tr>"),
            TagEnd::TableCell => {
                self.in_cell = false;
                self.out
                    .push_str(if self.in_head { "</th>" } else { "</td>" });
            }
            TagEnd::Emphasis => self.out.push_str("</em>"),
            TagEnd::Strong => self.out.push_str("</strong>"),
            TagEnd::Strikethrough => self.out.push_str("</del>"),
            TagEnd::Superscript => self.out.push_str("</sup>"),
            TagEnd::Subscript => self.out.push_str("</sub>"),
            TagEnd::Link => {
                let emitted = self.links.pop().unwrap_or(false);
                if emitted {
                    self.out.push_str("</a>");
                }
            }
            _ => {}
        }
    }

    /// `from` から `end`（含まない）までを処理し、`end` の次の位置を返す。
    fn inner_until(&mut self, events: &Events, from: usize, end: usize) -> usize {
        let mut j = from;
        while j < end {
            j = self.step(events, j);
        }
        // 閉じるイベントも通常どおり処理する。
        self.step(events, end)
    }

    fn marker(&mut self, text: &str) {
        self.out
            .push_str("<span class=\"md-marker\" aria-hidden=\"true\">");
        self.out.push_str(&escape(text));
        self.out.push_str("</span>");
    }

    /// 項目の記号を原文から決める。番号は開始番号から 1 ずつ増やす。
    fn item_marker(&mut self, offset: usize) -> String {
        let head = self.src.get(offset..).unwrap_or("").trim_start();
        let Some(list) = self.lists.last_mut() else {
            return "- ".to_string();
        };
        if list.ordered {
            let delim = head
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .chars()
                .next()
                .filter(|c| matches!(c, '.' | ')'))
                .unwrap_or('.');
            let n = list.next;
            list.next += 1;
            format!("{n}{delim} ")
        } else {
            let bullet = head
                .chars()
                .next()
                .filter(|c| matches!(c, '-' | '*' | '+'))
                .unwrap_or('-');
            format!("{bullet} ")
        }
    }

    fn unique_id(&mut self, base: &str) -> String {
        let base = if base.is_empty() { "section" } else { base };
        let mut id = base.to_string();
        let mut n = 0;
        while self.used_ids.contains(&id) {
            n += 1;
            id = format!("{base}-{n}");
        }
        self.used_ids.insert(id.clone());
        id
    }

    fn kuten_enabled(&self) -> bool {
        self.opts.break_after_kuten && !self.in_heading && !self.in_cell
    }

    /// 保留中の句点の改行を出す。
    fn resolve_pending(&mut self) {
        if let Some(pos) = self.pending.take() {
            self.out.insert_str(pos, KUTEN_BREAK);
        }
    }

    /// 開始タグは改行の後ろに置くので、改行の位置は動かさない。
    fn inline_open(&mut self, html: &str) {
        self.out.push_str(html);
    }

    fn text(&mut self, t: &str) {
        if !self.links.is_empty() || self.in_heading {
            self.plain(t);
            return;
        }
        for seg in split_segments(t, self.opts) {
            match seg {
                Seg::Plain(s) => self.plain(s),
                Seg::Url(u) => {
                    self.resolve_pending();
                    self.url_atom(u);
                }
                Seg::Path(p) => {
                    self.resolve_pending();
                    self.out.push_str(&path_html(p));
                }
            }
        }
    }

    /// 句点の後の改行を考えながら、文字をそのまま出す。
    fn plain(&mut self, t: &str) {
        if !self.kuten_enabled() {
            self.out.push_str(&escape(t));
            return;
        }
        let mut s = t;
        if let Some(pos) = self.pending {
            let trimmed = s.trim_start();
            let Some(first) = trimmed.chars().next() else {
                return;
            };
            self.pending = None;
            if first != '。' && !KUTEN_CLOSERS.contains(first) {
                self.out.insert_str(pos, KUTEN_BREAK);
            }
            s = trimmed;
        }
        let mut seg_start = 0;
        let mut i = 0;
        while i < s.len() {
            let Some(c) = s[i..].chars().next() else {
                break;
            };
            if c != '。' {
                i += c.len_utf8();
                continue;
            }
            let end = i + c.len_utf8();
            self.out.push_str(&escape(&s[seg_start..end]));
            let rest = s[end..].trim_start();
            match rest.chars().next() {
                None => {
                    self.pending = Some(self.out.len());
                    seg_start = s.len();
                    break;
                }
                Some(n) if n == '。' || KUTEN_CLOSERS.contains(n) => {
                    seg_start = end;
                    i = end;
                }
                Some(_) => {
                    self.out.push_str(KUTEN_BREAK);
                    seg_start = s.len() - rest.len();
                    i = seg_start;
                }
            }
        }
        self.out.push_str(&escape(&s[seg_start..]));
    }

    /// URLをリンクとして出す。長いときは省略した形にする。
    fn url_atom(&mut self, url: &str) {
        let full = escape(url);
        if self.opts.abbreviate && url.chars().count() >= self.opts.abbreviate_min_chars {
            self.out.push_str(&format!(
                "<span class=\"abbr\"><a class=\"abbr-target\" href=\"{full}\" title=\"{full}\">{}</a>{}</span>",
                escape(&short_url(url)),
                copy_button(&full, "URLをコピー")
            ));
        } else {
            self.out.push_str(&format!("<a href=\"{full}\">{full}</a>"));
        }
    }
}

fn copy_button(full_escaped: &str, label: &str) -> String {
    format!(
        "<button type=\"button\" class=\"copy-button\" data-copy=\"{full_escaped}\" aria-label=\"{label}\"></button>"
    )
}

fn path_html(path: &str) -> String {
    let full = escape(path);
    format!(
        "<span class=\"abbr\"><span class=\"abbr-target\" title=\"{full}\">{}</span>{}</span>",
        escape(&short_path(path)),
        copy_button(&full, "パスをコピー")
    )
}

fn is_http(url: &str) -> bool {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// `ホスト/…/最後の要素`。パスがなければホストだけ。
fn short_url(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (host, after) = rest.split_at(host_end);
    let path = after.split(['?', '#']).next().unwrap_or("");
    match path.split('/').rfind(|s| !s.is_empty()) {
        Some(last) => format!("{host}/…/{last}"),
        None => host.to_string(),
    }
}

/// `…/最後の要素`。末尾が `/` なら最後のディレクトリ名に `/` を付ける。
fn short_path(path: &str) -> String {
    let last = path.split('/').rfind(|s| !s.is_empty()).unwrap_or(path);
    if path.ends_with('/') {
        format!("…/{last}/")
    } else {
        format!("…/{last}")
    }
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '/' | '.' | '_' | '~' | '+' | '@' | '%' | '-')
        || (!c.is_ascii() && c.is_alphanumeric())
}

fn is_url_char(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_graphic() && !matches!(c, '<' | '>' | '"' | '\'' | '`')
    } else {
        c.is_alphanumeric()
    }
}

/// URLの末尾から、ASCII以外の文字と文末の記号を外す。`(` と対応する `)` は残す。
fn trim_url_end(url: &str) -> &str {
    let mut url = url;
    loop {
        let t = url.trim_end_matches(|c: char| !c.is_ascii());
        let t = match t.chars().next_back() {
            Some('.' | ',' | ';' | ':' | '!' | '?') => &t[..t.len() - 1],
            Some(')') if t.matches(')').count() > t.matches('(').count() => &t[..t.len() - 1],
            _ => t,
        };
        if t.len() == url.len() {
            return t;
        }
        url = t;
    }
}

/// 地の文を、URL、パス、それ以外に分ける。
fn split_segments<'t>(text: &'t str, opts: &RenderOptions) -> Vec<Seg<'t>> {
    let mut segs = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let hay = &text[pos..];
        let found = ["https://", "http://"]
            .iter()
            .filter_map(|p| hay.find(p).map(|i| (i, p.len())))
            .min();
        let Some((idx, scheme_len)) = found else {
            break;
        };
        let start = pos + idx;
        let tail = &text[start..];
        let mut len = tail.find(|c: char| !is_url_char(c)).unwrap_or(tail.len());
        if let Some(next) = ["https://", "http://"]
            .iter()
            .filter_map(|p| tail[1..len].find(p))
            .min()
        {
            len = next + 1;
        }
        let url = trim_url_end(&tail[..len]);
        if url.len() <= scheme_len {
            push_plain(&mut segs, &text[pos..start + scheme_len], opts);
            pos = start + scheme_len;
            continue;
        }
        push_plain(&mut segs, &text[pos..start], opts);
        segs.push(Seg::Url(url));
        pos = start + url.len();
    }
    push_plain(&mut segs, &text[pos..], opts);
    segs
}

/// URLを含まない文字列から、省略するパスを切り出す。
fn push_plain<'t>(segs: &mut Vec<Seg<'t>>, piece: &'t str, opts: &RenderOptions) {
    if piece.is_empty() {
        return;
    }
    if !opts.abbreviate {
        segs.push(Seg::Plain(piece));
        return;
    }
    let mut plain_start = 0;
    let mut run_start: Option<usize> = None;
    let sentinel = (piece.len(), '\0');
    for (i, c) in piece.char_indices().chain(std::iter::once(sentinel)) {
        if is_path_char(c) {
            run_start.get_or_insert(i);
            continue;
        }
        let Some(rs) = run_start.take() else {
            continue;
        };
        // 並びは ASCII の文字で始まり、ASCII の文字で終わる。
        let run = &piece[rs..i];
        let lead = run.len() - run.trim_start_matches(|c: char| !c.is_ascii()).len();
        let rs = rs + lead;
        let path = path_end(&piece[rs..i]);
        let after_scheme = path.starts_with('/') && piece[..rs].ends_with(':');
        if !after_scheme
            && path.matches('/').count() >= 2
            && path.chars().count() >= opts.abbreviate_min_chars
            && path.split('/').any(|s| !s.is_empty())
        {
            if plain_start < rs {
                segs.push(Seg::Plain(&piece[plain_start..rs]));
            }
            segs.push(Seg::Path(path));
            plain_start = rs + path.len();
        }
    }
    if plain_start < piece.len() {
        segs.push(Seg::Plain(&piece[plain_start..]));
    }
}

/// パスの末尾から、ASCII以外の文字と文末の `.` を外す。
fn path_end(path: &str) -> &str {
    path.trim_end_matches(|c: char| !c.is_ascii() || c == '.')
}

/// 閉じるイベントまでの文字だけを集める。戻り値は閉じるイベントの位置。
fn collect_plain(events: &Events, from: usize) -> (String, usize) {
    let mut text = String::new();
    let mut depth = 0;
    let mut j = from;
    while j < events.len() {
        match &events[j].0 {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
        j += 1;
    }
    (text, j)
}

/// ブロックの中の文字をそのまま集める。戻り値は閉じるイベントの位置。
fn collect_raw(events: &Events, from: usize) -> (String, usize) {
    let mut text = String::new();
    let mut j = from;
    while j < events.len() {
        match &events[j].0 {
            Event::End(_) => break,
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) | Event::Code(t) => {
                text.push_str(t)
            }
            _ => {}
        }
        j += 1;
    }
    (text, j)
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
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

/// リンク先と画像の参照先を検査し、必要なら `file://` に直す。許可しないときは `None`。
fn resolve_url(dest: &str, base_dir: Option<&Path>) -> Option<String> {
    let cleaned: String = dest.chars().filter(|c| !c.is_control()).collect();
    let url = cleaned.trim();
    if url.starts_with('#') {
        return Some(url.to_string());
    }
    // スキームを省いた外部参照（`//host/path`）は、許可する種類に当たらないので通さない。
    if url.starts_with("//") {
        return None;
    }
    if let Some((scheme, _)) = url.split_once(':')
        && scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto"
        )
        .then(|| url.to_string());
    }
    let Some(base) = base_dir else {
        return Some(url.to_string());
    };
    let split = url.find(['?', '#']).unwrap_or(url.len());
    let (path, suffix) = url.split_at(split);
    let base = base.to_string_lossy();
    let joined = if path.starts_with('/') {
        path.to_string()
    } else if path.is_empty() {
        base.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), path)
    };
    let slash = if joined.starts_with('/') { "" } else { "/" };
    Some(format!("file://{slash}{}{suffix}", percent_encode(&joined)))
}

/// パスをパーセントエンコードする。すでに `%XX` の形のものはそのまま残す。
fn percent_encode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len());
    for (i, &b) in bytes.iter().enumerate() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/')
            || (b == b'%'
                && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit));
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(src: &str) -> String {
        render_markdown(src, &RenderOptions::default()).body_html
    }

    fn html_with(src: &str, f: impl FnOnce(&mut RenderOptions)) -> String {
        let mut options = RenderOptions::default();
        f(&mut options);
        render_markdown(src, &options).body_html
    }

    #[test]
    fn heading_shows_marker_and_id() {
        assert!(html("## 見出し").contains(
            "<h2 id=\"見出し\"><span class=\"md-marker\" aria-hidden=\"true\">## </span>見出し</h2>"
        ));
    }

    #[test]
    fn heading_ids_are_unique_and_slugged() {
        let out = html("# Hello World!\n\n# Hello World!\n\n# Hello World!");
        assert!(out.contains("id=\"hello-world\""));
        assert!(out.contains("id=\"hello-world-1\""));
        assert!(out.contains("id=\"hello-world-2\""));
    }

    #[test]
    fn title_is_first_h1_plain_text() {
        let r = render_markdown(
            "## 前\n\n# **太字**と`コード`\n\n# 二つ目",
            &RenderOptions::default(),
        );
        assert_eq!(r.title.as_deref(), Some("太字とコード"));
        assert_eq!(
            render_markdown("本文", &RenderOptions::default()).title,
            None
        );
    }

    #[test]
    fn markers_hidden_when_disabled() {
        let out = html_with("## 見出し\n\n- a\n\n1. b", |o| o.show_markers = false);
        assert!(!out.contains("md-marker"));
        assert!(!out.contains("md-list"));
        assert!(out.contains("<div class=\"li-body\">"));
    }

    #[test]
    fn kuten_break_between_sentences() {
        assert!(
            html("一文目。二文目。").contains("<p>一文目。<br class=\"kuten-break\">二文目。</p>")
        );
    }

    #[test]
    fn list_item_with_strong_and_kuten() {
        assert!(html("- **重要**。次へ。").contains(
            "<li><span class=\"md-marker\" aria-hidden=\"true\">- </span><div class=\"li-body\"><strong>重要</strong>。<br class=\"kuten-break\">次へ。</div></li>"
        ));
    }

    #[test]
    fn no_break_before_closing_bracket() {
        assert!(html("「はい。」と答えた。").contains("<p>「はい。」と答えた。</p>"));
    }

    #[test]
    fn consecutive_kuten_break_once() {
        assert!(html("えっ。。次").contains("えっ。。<br class=\"kuten-break\">次"));
    }

    #[test]
    fn kuten_drops_following_space() {
        assert!(html("前。 後").contains("前。<br class=\"kuten-break\">後"));
    }

    #[test]
    fn kuten_before_closing_tag_goes_after_tag() {
        assert!(
            html("**重要。** 次").contains("<strong>重要。</strong><br class=\"kuten-break\">次")
        );
    }

    #[test]
    fn kuten_before_soft_break_merges() {
        let out = html("一。\n二");
        assert!(out.contains("<p>一。<br class=\"kuten-break\">二</p>"));
        assert!(!out.contains('\n') || !out.contains("<br class=\"kuten-break\">\n"));
    }

    #[test]
    fn kuten_before_hard_break_merges() {
        assert!(html("一。  \n二").contains("<p>一。<br>二</p>"));
    }

    #[test]
    fn non_ascii_path_and_url_chars() {
        let dir = "/Users/someone/Projects/Tools/miyaburu/documents";
        let out = html(&format!("{dir}/日本語.md"));
        assert!(out.contains(&format!("title=\"{dir}/日本語.md\">…/日本語.md</span>")));
        assert!(out.contains(&format!("data-copy=\"{dir}/日本語.md\"")));
        let out = html(&format!("{dir}/config.mdにあります"));
        assert!(out.contains(&format!("title=\"{dir}/config.md\">…/config.md</span>")));
        assert!(out.contains("</span>にあります</p>"), "{out}");
        let out = html("https://example.com/日本語/page.html");
        assert!(out.contains("<a href=\"https://example.com/日本語/page.html\">"));
        let out = html("https://example.com/aを見る");
        assert!(out.contains("<a href=\"https://example.com/a\">https://example.com/a</a>を見る"));
        let out = html("https://example.com/a、b");
        assert!(out.contains("<a href=\"https://example.com/a\">https://example.com/a</a>、b"));
    }

    #[test]
    fn url_ends_at_next_scheme() {
        let out = html("https://a.com/xhttps://b.com/y");
        assert!(out.contains("<a href=\"https://a.com/x\">"));
        assert!(out.contains("<a href=\"https://b.com/y\">"));
    }

    #[test]
    fn url_keeps_balanced_parens() {
        let out = html("https://en.wikipedia.org/wiki/Rust_(programming_language)");
        assert!(
            out.contains(
                "href=\"https://en.wikipedia.org/wiki/Rust_(programming_language)\" title="
            )
        );
        let out = html("(https://example.com/a)");
        assert!(out.contains("(<a href=\"https://example.com/a\">https://example.com/a</a>)"));
    }

    #[test]
    fn kuten_break_decided_by_text_after_inline_open() {
        let out = html("「一。**」** と答えた。");
        assert!(!out.contains("kuten-break"), "{out}");
        let out = html("一。**二**");
        assert!(
            out.contains("一。<br class=\"kuten-break\"><strong>二</strong>"),
            "{out}"
        );
        let out = html("一。[二](https://e.com/)");
        assert!(
            out.contains("一。<br class=\"kuten-break\"><a href"),
            "{out}"
        );
    }

    #[test]
    fn plain_soft_break_is_newline() {
        assert!(html("一\n二").contains("<p>一\n二</p>"));
    }

    #[test]
    fn no_break_at_end_of_nested_block() {
        let out = html("- 親。\n  - 子。");
        assert!(!out.contains("kuten-break"));
    }

    #[test]
    fn no_break_in_heading_or_cell_or_code() {
        let out = html("# 一。二\n\n| a |\n|---|\n| 一。二 |\n\n`a。b`\n\n```\n一。二\n```");
        assert!(!out.contains("kuten-break"));
        assert!(out.contains("<code>a。b</code>"));
    }

    #[test]
    fn kuten_break_can_be_disabled() {
        assert!(!html_with("一。二", |o| o.break_after_kuten = false).contains("kuten-break"));
    }

    #[test]
    fn bullet_and_ordered_markers_follow_source() {
        let out = html("* a\n* b\n\n+ c\n\n3) x\n4) y");
        assert!(out.contains("aria-hidden=\"true\">* </span>"));
        assert!(out.contains("aria-hidden=\"true\">+ </span>"));
        assert!(out.contains("<ol class=\"md-list\" start=\"3\">"));
        assert!(out.contains("aria-hidden=\"true\">3) </span>"));
        assert!(out.contains("aria-hidden=\"true\">4) </span>"));
    }

    #[test]
    fn task_items() {
        let out = html("- [ ] a\n- [x] b");
        assert!(out.contains("<li class=\"task\"><span class=\"md-marker\" aria-hidden=\"true\">- </span><div class=\"li-body\"><input type=\"checkbox\" disabled>a"));
        assert!(out.contains("<input type=\"checkbox\" disabled checked>b"));
    }

    #[test]
    fn alert_blockquote() {
        let out = html("> [!WARNING]\n> 注意");
        assert!(out.contains("<blockquote class=\"alert alert-warning\">"));
        assert!(html("> 引用").contains("<blockquote>"));
    }

    #[test]
    fn code_block_with_language() {
        let out = html("```rust\nlet a = 1 < 2;\n```");
        assert!(out.contains(
            "<pre class=\"code-block\"><code class=\"language-rust\">let a = 1 &lt; 2;\n</code></pre>"
        ));
        assert!(html("```\nx\n```").contains("<pre class=\"code-block\"><code>x\n</code></pre>"));
    }

    #[test]
    fn mermaid_block() {
        let r = render_markdown("```mermaid\ngraph TD;\n```", &RenderOptions::default());
        assert!(r.has_mermaid);
        assert!(
            r.body_html
                .contains("<pre class=\"mermaid\">graph TD;\n</pre>")
        );
        let off = html_with("```mermaid\ngraph TD;\n```", |o| o.mermaid = false);
        assert!(off.contains("language-mermaid"));
        assert!(!off.contains("class=\"mermaid\""));
    }

    #[test]
    fn table_with_alignment() {
        let out = html("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |");
        assert!(out.contains("<div class=\"table-wrap\"><table><thead><tr><th class=\"align-left\">a</th><th class=\"align-center\">b</th><th class=\"align-right\">c</th></tr></thead><tbody><tr><td class=\"align-left\">1</td>"));
        assert!(out.contains("</tbody></table></div>"));
    }

    #[test]
    fn inline_formatting_and_rule() {
        let out = html("**a** *b* ~~c~~\n\n---");
        assert!(out.contains("<strong>a</strong> <em>b</em> <del>c</del>"));
        assert!(out.contains("<hr>"));
    }

    #[test]
    fn footnotes() {
        let out = html("本文[^1]\n\n[^1]: 注釈");
        assert!(
            out.contains(
                "<sup class=\"footnote-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup>"
            )
        );
        assert!(out.contains("<div class=\"footnote\" id=\"fn-1\"><span class=\"footnote-label\">1</span><div class=\"footnote-body\"><p>注釈</p>"));
    }

    #[test]
    fn raw_html_block_is_escaped() {
        let out = html("<script>alert(1)</script>");
        assert!(!out.contains("<script"));
        assert!(out.contains("<pre class=\"code-block raw-html\"><code>&lt;script&gt;alert(1)&lt;/script&gt;</code></pre>"));
    }

    #[test]
    fn raw_html_inline_is_escaped() {
        let out = html("a <b>x</b> c");
        assert!(!out.contains("<b>"));
        assert!(out.contains("&lt;b&gt;x&lt;/b&gt;"));
    }

    #[test]
    fn dangerous_link_scheme_is_text_only() {
        let out = html("[x](javascript:alert(1))");
        assert!(!out.contains("<a"));
        assert!(out.contains("<p>x</p>"));
        assert!(!html("[x](data:text/html,a)").contains("<a"));
        assert!(!html("[x](//evil.example/x)").contains("<a"));
        assert!(!html("![alt](//evil.example/i.png)").contains("<img"));
        assert!(!html("![alt](vbscript:x)").contains("<img"));
        assert!(html("![alt](vbscript:x)").contains("alt"));
    }

    #[test]
    fn allowed_links_and_images() {
        assert!(
            html("[a](https://example.com/x)").contains("<a href=\"https://example.com/x\">a</a>")
        );
        assert!(
            html("[a](mailto:a@example.com)").contains("<a href=\"mailto:a@example.com\">a</a>")
        );
        assert!(html("[a](#sec)").contains("<a href=\"#sec\">a</a>"));
        assert!(html("![alt](a.png)").contains("<img src=\"a.png\" alt=\"alt\" loading=\"lazy\">"));
    }

    #[test]
    fn relative_links_use_base_dir() {
        let with_base = |src: &str| {
            let o = RenderOptions {
                base_dir: Some(PathBuf::from("/home/me/my docs")),
                ..RenderOptions::default()
            };
            render_markdown(src, &o).body_html
        };
        assert!(
            with_base("[a](x/y.md#top)").contains("href=\"file:///home/me/my%20docs/x/y.md#top\"")
        );
        assert!(with_base("[a](/abs/f.md)").contains("href=\"file:///abs/f.md\""));
        assert!(
            with_base("![a](p%20q.png)").contains("src=\"file:///home/me/my%20docs/p%20q.png\"")
        );
        assert!(with_base("[a](#top)").contains("href=\"#top\""));
        assert!(with_base("[a](https://e.com/)").contains("href=\"https://e.com/\""));
    }

    #[test]
    fn bare_url_is_linked_and_abbreviated() {
        let url = "https://www.example.com/docs/guide/section/page.html?q=1#top";
        let out = html(&format!("見て {url} です"));
        assert!(out.contains(&format!(
            "<span class=\"abbr\"><a class=\"abbr-target\" href=\"{url}\" title=\"{url}\">example.com/…/page.html</a><button type=\"button\" class=\"copy-button\" data-copy=\"{url}\" aria-label=\"URLをコピー\"></button></span>"
        )));
    }

    #[test]
    fn short_url_is_linked_but_not_abbreviated() {
        let out = html("https://example.com/a, です");
        assert!(out.contains("<a href=\"https://example.com/a\">https://example.com/a</a>, です"));
        assert!(!out.contains("abbr"));
    }

    #[test]
    fn url_trailing_punctuation_excluded() {
        let out = html("見る（https://example.com/a）。次");
        assert!(out.contains("<a href=\"https://example.com/a\">https://example.com/a</a>）。<br class=\"kuten-break\">次"));
    }

    #[test]
    fn long_url_host_only_when_no_path() {
        let out = html_with("https://example.com/", |o| o.abbreviate_min_chars = 10);
        assert!(out.contains(">example.com</a>"));
    }

    #[test]
    fn autolink_and_same_text_link_are_abbreviated() {
        let url = "https://example.com/aaaa/bbbb/cccc/dddd/eeee/last";
        assert!(html(&format!("<{url}>")).contains("abbr-target"));
        assert!(html(&format!("[{url}]({url})")).contains("example.com/…/last"));
        let described = html(&format!("[説明]({url})"));
        assert!(!described.contains("abbr"));
        assert!(described.contains(&format!("<a href=\"{url}\">説明</a>")));
    }

    #[test]
    fn abbreviate_off_still_links_without_button() {
        let url = "https://example.com/aaaa/bbbb/cccc/dddd/eeee/last";
        let out = html_with(
            &format!("{url} と /usr/local/lib/aaaa/bbbb/cccc/dddd/eeee/ffff"),
            |o| o.abbreviate = false,
        );
        assert!(out.contains(&format!("<a href=\"{url}\">{url}</a>")));
        assert!(!out.contains("copy-button"));
        assert!(!out.contains("abbr"));
    }

    #[test]
    fn long_path_is_abbreviated() {
        let path = "/Users/someone/Projects/Tools/miyaburu/src/render.rs";
        let out = html(&format!("ファイル{path}を開く"));
        assert!(out.contains(&format!(
            "ファイル<span class=\"abbr\"><span class=\"abbr-target\" title=\"{path}\">…/render.rs</span><button type=\"button\" class=\"copy-button\" data-copy=\"{path}\" aria-label=\"パスをコピー\"></button></span>を開く"
        )));
    }

    #[test]
    fn path_with_trailing_slash_keeps_slash() {
        let out = html("/Users/someone/Projects/Tools/miyaburu/src/long_directory/");
        assert!(out.contains(">…/long_directory/</span>"));
    }

    #[test]
    fn short_path_and_code_are_left_alone() {
        assert!(!html("/usr/bin/env").contains("abbr"));
        let long = "/Users/someone/Projects/Tools/miyaburu/src/render.rs";
        assert!(!html(&format!("`{long}`")).contains("abbr"));
        assert!(!html(&format!("```\n{long}\n```")).contains("abbr"));
    }

    #[test]
    fn escapes_text_and_attributes() {
        let out = html("a & b < c \"d\" 'e'");
        assert!(out.contains("a &amp; b &lt; c &quot;d&quot; &#39;e&#39;"));
        assert!(html("![a\"b](x.png)").contains("alt=\"a&quot;b\""));
    }
}
