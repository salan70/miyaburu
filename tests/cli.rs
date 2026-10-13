//! 実行ファイルを通した結合テスト。ブラウザーは開かない。

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// テストごとの作業ディレクトリ。終わったら消す。
struct Workspace(PathBuf);

impl Workspace {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "miyaburu-cli-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create workspace");
        Self(path)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, content).expect("write file");
        path
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 利用者の設定に左右されないよう、作業ディレクトリで実行し、設定の探索先も中に閉じ込める。
fn miyaburu(workspace: &Workspace, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_miyaburu"))
        .arg("--no-share")
        .args(args)
        .current_dir(&workspace.0)
        .env("HOME", &workspace.0)
        .env("XDG_CONFIG_HOME", workspace.0.join("config"))
        .env("XDG_CACHE_HOME", workspace.0.join("cache"))
        .output()
        .expect("run miyaburu")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn generates_a_page_without_opening_a_browser() {
    let workspace = Workspace::new("page");
    workspace.write(
        "plan.md",
        "# 計画\n\n一文目。二文目。\n\n- **重要**。次へ。\n",
    );

    let output = miyaburu(&workspace, &["--no-open", "--output", "out", "plan.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let page = workspace.0.join("out/index.html");
    assert_eq!(stdout(&output).trim(), "out/index.html");
    let html = fs::read_to_string(&page).expect("read generated page");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>計画</title>"));
    assert!(html.contains("一文目。<br class=\"kuten-break\">二文目。"));
    assert!(html.contains("<span class=\"md-marker\" aria-hidden=\"true\"># </span>計画"));
    assert!(!workspace.0.join("out/mermaid.min.js").exists());
}

#[test]
fn writes_mermaid_next_to_the_page_only_when_needed() {
    let workspace = Workspace::new("mermaid");
    workspace.write("flow.md", "```mermaid\nflowchart LR\n    A --> B\n```\n");

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "flow.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let html = fs::read_to_string(workspace.0.join("out/index.html")).expect("read page");
    assert!(html.contains("<pre class=\"mermaid\">"));
    assert!(html.contains("src=\"mermaid.min.js\""));
    assert!(workspace.0.join("out/mermaid.min.js").is_file());
}

#[test]
fn uses_the_cache_directory_by_default() {
    let workspace = Workspace::new("cache");
    workspace.write("note.md", "本文\n");

    let output = miyaburu(&workspace, &["--no-open", "note.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let page = PathBuf::from(stdout(&output).trim());
    assert!(page.is_file());
    assert!(page.ends_with("index.html"));
    assert!(
        page.starts_with(fs::canonicalize(&workspace.0).expect("canonical workspace"))
            || page.starts_with(&workspace.0)
    );
}

#[test]
fn applies_the_config_file_in_the_current_directory() {
    let workspace = Workspace::new("config");
    workspace.write("note.md", "一文目。二文目。\n");
    workspace.write(
        "miyaburu.toml",
        "[color]\naccent = \"#e0407a\"\n\n[rendering]\nbreak_after_kuten = false\n",
    );

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let html = fs::read_to_string(workspace.0.join("out/index.html")).expect("read page");
    assert!(html.contains("--mb-color-accent:#e0407a;"));
    assert!(!html.contains("kuten-break\">"));
}

#[test]
fn highlights_code_blocks_with_the_colors_of_the_theme() {
    let workspace = Workspace::new("highlight");
    workspace.write("note.md", "```rust\nlet a = \"b\";\n```\n");
    workspace.write(
        "themed.toml",
        "[color]\ntheme = \"dracula\"\n\n[color.syntax]\nstring = \"#abcdef\"\n",
    );
    workspace.write("plain.toml", "[rendering]\nhighlight = false\n");

    let themed = miyaburu(
        &workspace,
        &[
            "--no-open",
            "--config",
            "themed.toml",
            "-o",
            "themed",
            "note.md",
        ],
    );
    assert!(themed.status.success(), "stderr: {}", stderr(&themed));
    let html = fs::read_to_string(workspace.0.join("themed/index.html")).expect("read page");
    assert!(html.contains("<span class=\"tok-keyword\">let</span>"));
    assert!(html.contains("<span class=\"tok-string\">&quot;b&quot;</span>"));
    assert!(html.contains("--mb-syntax-keyword:#ff79c6;"));
    assert!(html.contains("--mb-syntax-string:#abcdef;"));
    assert!(html.contains(".tok-keyword {"));

    let plain = miyaburu(
        &workspace,
        &[
            "--no-open",
            "--config",
            "plain.toml",
            "-o",
            "plain",
            "note.md",
        ],
    );
    assert!(plain.status.success(), "stderr: {}", stderr(&plain));
    let html = fs::read_to_string(workspace.0.join("plain/index.html")).expect("read page");
    assert!(html.contains("<code class=\"language-rust\">let a = &quot;b&quot;;\n</code>"));
}

#[test]
fn applies_a_built_in_color_theme_and_keeps_written_colors() {
    let workspace = Workspace::new("theme");
    workspace.write("note.md", "```mermaid\ngraph LR\n  A --> B\n```\n");
    workspace.write(
        "miyaburu.toml",
        "[color]\ntheme = \"dracula\"\naccent = \"#e0407a\"\n",
    );

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let html = fs::read_to_string(workspace.0.join("out/index.html")).expect("read page");
    assert!(html.contains("--mb-color-bg:#282a36;"));
    assert!(html.contains("--mb-color-accent:#e0407a;"));
    assert!(html.contains("color-scheme:dark;"));
    // 図は、ページの色に合わせるテーマで描く。
    assert!(html.contains("data-mermaid-theme=\"base\""));
}

#[test]
fn reports_an_unknown_color_theme() {
    let workspace = Workspace::new("badtheme");
    workspace.write("note.md", "本文\n");
    workspace.write("miyaburu.toml", "[color]\ntheme = \"drakula\"\n");

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert_eq!(output.status.code(), Some(1));
    let message = stderr(&output);
    assert!(message.contains("miyaburu.toml"), "stderr: {message}");
    assert!(message.contains("drakula"), "stderr: {message}");
    assert!(!workspace.0.join("out/index.html").exists());
}

#[test]
fn reports_an_invalid_config_with_its_key() {
    let workspace = Workspace::new("badconfig");
    workspace.write("note.md", "本文\n");
    workspace.write("bad.toml", "[font]\nsize = 99\n");

    let output = miyaburu(
        &workspace,
        &["--no-open", "--config", "bad.toml", "note.md"],
    );

    assert_eq!(output.status.code(), Some(1));
    let message = stderr(&output);
    assert!(message.contains("bad.toml"), "stderr: {message}");
    assert!(message.contains("font.size"), "stderr: {message}");
    assert!(stdout(&output).is_empty());
}

#[test]
fn reports_a_missing_file_clearly() {
    let workspace = Workspace::new("missing");

    let output = miyaburu(&workspace, &["--no-open", "nowhere.md"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("nowhere.md"));
}

#[cfg(unix)]
#[test]
fn writes_files_only_the_owner_can_read() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = Workspace::new("private");
    workspace.write("note.md", "# 秘密のメモ\n");

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let mode = |name: &str| {
        fs::metadata(workspace.0.join(name))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode("out"), 0o700);
    assert_eq!(mode("out/index.html"), 0o600);
}

#[cfg(unix)]
#[test]
fn refuses_to_overwrite_the_source_through_a_symlink() {
    let workspace = Workspace::new("selflink");
    let source = "# 原文\n\n本文\n";
    let note = workspace.write("note.md", source);
    fs::create_dir(workspace.0.join("out")).expect("create out");
    std::os::unix::fs::symlink(&note, workspace.0.join("out/index.html")).expect("symlink");

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("index.html"));
    assert_eq!(fs::read_to_string(&note).expect("read source"), source);
}

#[cfg(unix)]
#[test]
fn replaces_an_unrelated_symlink_instead_of_writing_through_it() {
    let workspace = Workspace::new("otherlink");
    workspace.write("note.md", "本文\n");
    let other = workspace.write("other.txt", "別のファイル\n");
    fs::create_dir(workspace.0.join("out")).expect("create out");
    let page = workspace.0.join("out/index.html");
    std::os::unix::fs::symlink(&other, &page).expect("symlink");

    let output = miyaburu(&workspace, &["--no-open", "-o", "out", "note.md"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        fs::read_to_string(&other).expect("read other"),
        "別のファイル\n"
    );
    assert!(
        !fs::symlink_metadata(&page)
            .expect("page metadata")
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&page)
            .expect("read page")
            .starts_with("<!doctype html>")
    );
}

#[test]
fn rejects_a_wrong_invocation_with_usage() {
    let workspace = Workspace::new("usage");

    let output = miyaburu(&workspace, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("使い方"));
}

#[test]
fn prints_help_and_version() {
    let workspace = Workspace::new("help");

    let help = miyaburu(&workspace, &["--help"]);
    assert!(help.status.success());
    assert!(stdout(&help).contains("--no-open"));

    let version = miyaburu(&workspace, &["--version"]);
    assert_eq!(
        stdout(&version).trim(),
        format!("miyaburu {}", env!("CARGO_PKG_VERSION"))
    );
}
