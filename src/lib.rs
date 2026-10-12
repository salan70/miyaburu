//! みやぶるの中核となる処理。
//!
//! Markdownの読み込み、HTMLへの変換、設定、ページの組み立てを提供する。

pub mod config;
pub mod page;
pub mod render;
pub mod theme;

use std::fs;
use std::io;
use std::path::Path;

/// UTF-8のMarkdownファイルを、内容を変えずに読み込む。
///
/// @doc docs/specs/input.md#markdown原文の読み込み
pub fn read_markdown(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::read_markdown;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reads_markdown_unchanged() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        let file = std::env::temp_dir().join(format!("miyaburu-{}-{nonce}.md", std::process::id()));
        let source = "## 見出し\n- **重要**。次へ。\n";
        fs::write(&file, source).expect("create temporary Markdown");

        let loaded = read_markdown(&file).expect("read temporary Markdown");
        let _ = fs::remove_file(&file);

        assert_eq!(loaded, source);
    }
}
