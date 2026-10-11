# Input

<!-- @code src/lib.rs#read_markdown -->

## Markdown source loading

The CLI will accept an existing Markdown file path. Loading is kept separate
from rendering: the original UTF-8 text must remain unchanged, including
Markdown markers, Japanese punctuation, newlines, and links.

The initial Rust implementation is `read_markdown` in `src/lib.rs`.
A missing file, unreadable file, or invalid UTF-8 produces a normal I/O error.
The CLI must eventually report this failure clearly instead of silently
substituting an empty document.

Rendering transformations operate on a parsed representation, not by writing
modified content back to the source file.
