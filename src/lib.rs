//! Core utilities for miyaburu.
//!
//! Browser rendering and CLI behavior are not implemented yet.

use std::fs;
use std::io;
use std::path::Path;

/// Read a UTF-8 Markdown source file without changing its contents.
///
/// @doc docs/specs/input.md#markdown-source-loading
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
