//! みやぶるのコマンドライン入口。

use std::env;
use std::ffi::OsString;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use miyaburu::config::{Config, find_config_file};
use miyaburu::page::{MERMAID_FILE_NAME, MERMAID_JS, PageInput, build_page};
use miyaburu::read_markdown;
use miyaburu::render::{RenderOptions, render_markdown};

const USAGE: &str = "\
使い方: miyaburu [オプション] <Markdownファイル>

Markdownファイルを読みやすいHTMLに変換し、既定のブラウザーで開きます。

オプション:
  -c, --config <ファイル>      設定ファイル（TOML）を指定する
  -o, --output <ディレクトリ>  HTMLの出力先を指定する（既定はキャッシュ用ディレクトリ）
      --no-open                ブラウザーを開かず、生成したHTMLのパスだけを表示する
  -h, --help                   この説明を表示する
  -V, --version                バージョンを表示する
";

#[derive(Debug, PartialEq)]
enum Invocation {
    Help,
    Version,
    Preview(PreviewArgs),
}

#[derive(Debug, PartialEq)]
struct PreviewArgs {
    input: PathBuf,
    config: Option<PathBuf>,
    output: Option<PathBuf>,
    open: bool,
}

/// 引数を解釈して実行し、結果を終了コードで返す。
///
/// @doc docs/specs/cli.md#終了コード
fn main() -> ExitCode {
    let invocation = match parse_args(env::args_os().skip(1)) {
        Ok(invocation) => invocation,
        Err(message) => {
            eprintln!("miyaburu: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let args = match invocation {
        Invocation::Help => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Invocation::Version => {
            println!("miyaburu {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Invocation::Preview(args) => args,
    };

    let page = match generate(&args) {
        Ok(page) => page,
        Err(message) => {
            eprintln!("miyaburu: {message}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", page.display());

    if args.open
        && let Err(error) = open_in_browser(&page)
    {
        eprintln!(
            "miyaburu: 「{}」をブラウザーで開けません: {error}",
            page.display()
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// コマンドラインの引数を解釈する。
///
/// @doc docs/specs/cli.md#使い方
fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, String> {
    let mut input = None;
    let mut config = None;
    let mut output = None;
    let mut open = true;
    let mut options_ended = false;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if options_ended || !text.starts_with('-') || text == "-" {
            if input.replace(PathBuf::from(&arg)).is_some() {
                return Err("Markdownファイルは1つだけ指定してください".to_string());
            }
            continue;
        }

        let (name, inline_value) = match text.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (text.as_ref(), None),
        };
        match name {
            "--" => options_ended = true,
            "-h" | "--help" => return Ok(Invocation::Help),
            "-V" | "--version" => return Ok(Invocation::Version),
            "--no-open" => open = false,
            "-c" | "--config" => config = Some(option_value(name, inline_value, &mut args)?),
            "-o" | "--output" => output = Some(option_value(name, inline_value, &mut args)?),
            _ => return Err(format!("不明なオプションです: {name}")),
        }
    }

    let input = input.ok_or("Markdownファイルを指定してください")?;
    Ok(Invocation::Preview(PreviewArgs {
        input,
        config,
        output,
        open,
    }))
}

fn option_value(
    name: &str,
    inline_value: Option<String>,
    rest: &mut impl Iterator<Item = OsString>,
) -> Result<PathBuf, String> {
    match inline_value {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        Some(_) => Err(format!("{name} には値が必要です")),
        None => rest
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| format!("{name} には値が必要です")),
    }
}

/// HTMLを生成し、書き出した `index.html` のパスを返す。
fn generate(args: &PreviewArgs) -> Result<PathBuf, String> {
    let source = read_markdown(&args.input)
        .map_err(|error| format!("「{}」を読み込めません: {error}", args.input.display()))?;
    let config = load_config(args.config.as_deref())?;

    let document = fs::canonicalize(&args.input).unwrap_or_else(|_| args.input.clone());
    let options = RenderOptions {
        show_markers: config.rendering.show_markers,
        break_after_kuten: config.rendering.break_after_kuten,
        abbreviate: config.rendering.abbreviate,
        abbreviate_min_chars: config.rendering.abbreviate_min_chars,
        mermaid: config.mermaid.enabled,
        base_dir: document.parent().map(Path::to_path_buf),
    };
    let rendered = render_markdown(&source, &options);

    let document_name = document
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let html = build_page(&PageInput {
        rendered: &rendered,
        config: &config,
        document_name: &document_name,
        nonce: &nonce(),
    });

    let directory = match &args.output {
        Some(directory) => directory.clone(),
        None => default_output_dir(&document),
    };
    create_private_dir(&directory).map_err(|error| write_error(&directory, &error))?;
    if rendered.has_mermaid {
        write_private(&directory.join(MERMAID_FILE_NAME), MERMAID_JS, &document)?;
    }
    let page = directory.join("index.html");
    write_private(&page, html.as_bytes(), &document)?;
    Ok(page)
}

fn write_error(path: &Path, error: &io::Error) -> String {
    format!("「{}」へ書き込めません: {error}", path.display())
}

/// 本人だけが読めるファイルとして書き出す。
///
/// 一時ファイルに書いてから置き換えるので、出力先がシンボリックリンクでもリンク先は書き換えない。
/// 入力のMarkdown自身が出力先のときは、原文を守るために中止する。
///
/// @doc docs/specs/cli.md#出力先
fn write_private(path: &Path, contents: &[u8], document: &Path) -> Result<(), String> {
    if fs::canonicalize(path).is_ok_and(|existing| existing == document) {
        return Err(format!(
            "出力先「{}」が入力のMarkdownファイルと同じです",
            path.display()
        ));
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temporary);
    create_private_file(&temporary, contents)
        .and_then(|()| fs::rename(&temporary, path))
        .map_err(|error| {
            let _ = fs::remove_file(&temporary);
            write_error(path, &error)
        })
}

fn create_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(contents)
}

/// 指定があればその設定を、なければ既定の場所の設定を読む。どこにもなければ既定値を使う。
fn load_config(explicit: Option<&Path>) -> Result<Config, String> {
    let found = match explicit {
        Some(path) => Some(path.to_path_buf()),
        None => {
            let cwd = env::current_dir()
                .map_err(|error| format!("現在のディレクトリを取得できません: {error}"))?;
            find_config_file(&cwd, user_config_dir().as_deref())
        }
    };
    match found {
        Some(path) => Config::load(&path).map_err(|error| error.to_string()),
        None => Ok(Config::default()),
    }
}

fn home_dir() -> Option<PathBuf> {
    absolute_env("HOME")
}

fn absolute_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// `miyaburu/config.toml` を探す、利用者ごとの設定ディレクトリ。
fn user_config_dir() -> Option<PathBuf> {
    absolute_env("XDG_CONFIG_HOME").or_else(|| home_dir().map(|home| home.join(".config")))
}

/// 利用者ごとのキャッシュ用ディレクトリ。共有の一時ディレクトリは避ける。
fn user_cache_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return home_dir().map(|home| home.join("Library/Caches"));
    }
    absolute_env("XDG_CACHE_HOME").or_else(|| home_dir().map(|home| home.join(".cache")))
}

/// 文書ごとに決まる出力先。同じ文書を開き直すと同じ場所を上書きする。
///
/// @doc docs/specs/cli.md#出力先
fn default_output_dir(document: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    document.hash(&mut hasher);
    user_cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("miyaburu")
        .join(format!("{:016x}", hasher.finish()))
}

/// 本人だけが読めるディレクトリを作る。文書の内容をほかの利用者へ見せないため。
/// すでにあるディレクトリの権限は変えない。
fn create_private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

/// ページごとに変わる、スクリプト許可用の値。
fn nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let mut first = DefaultHasher::new();
    (nanos, std::process::id()).hash(&mut first);
    let mut second = DefaultHasher::new();
    (first.finish(), &nanos as *const u128 as usize).hash(&mut second);
    format!("{:016x}{:016x}", first.finish(), second.finish())
}

/// 起動役のコマンドの終了を待つ上限。
const OPENER_WAIT: Duration = Duration::from_secs(2);

/// 既定のブラウザーで開く。
///
/// @doc docs/specs/cli.md#ブラウザーの起動
fn open_in_browser(page: &Path) -> io::Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command.arg(page);
    launch(command, OPENER_WAIT)
}

/// 起動役のコマンドを実行し、失敗を拾う。
///
/// 起動役はふつうすぐに終わるので、その終了を `wait` まで待つ。
/// 時間内に終わらないときは、ブラウザーを前面で動かしているとみなし、呼び出し元を待たせずに戻る。
///
/// @doc docs/specs/cli.md#ブラウザーの起動
fn launch(mut command: Command, wait: Duration) -> io::Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + wait;
    loop {
        match child.try_wait()? {
            Some(status) if status.success() => return Ok(()),
            Some(status) => {
                let program = command.get_program().to_string_lossy().into_owned();
                return Err(io::Error::other(match status.code() {
                    Some(code) => format!("{program} が終了コード {code} で失敗しました"),
                    None => format!("{program} が異常終了しました"),
                }));
            }
            None if Instant::now() >= deadline => return Ok(()),
            None => thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Invocation, PreviewArgs, default_output_dir, nonce, parse_args};
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    fn parse(args: &[&str]) -> Result<Invocation, String> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn parses_a_single_file() {
        assert_eq!(
            parse(&["docs/plan.md"]),
            Ok(Invocation::Preview(PreviewArgs {
                input: PathBuf::from("docs/plan.md"),
                config: None,
                output: None,
                open: true,
            }))
        );
    }

    #[test]
    fn parses_options_in_both_forms() {
        assert_eq!(
            parse(&["--config=a.toml", "-o", "out", "--no-open", "plan.md"]),
            Ok(Invocation::Preview(PreviewArgs {
                input: PathBuf::from("plan.md"),
                config: Some(PathBuf::from("a.toml")),
                output: Some(PathBuf::from("out")),
                open: false,
            }))
        );
    }

    #[test]
    fn treats_arguments_after_double_dash_as_files() {
        assert_eq!(
            parse(&["--", "-plan.md"]),
            Ok(Invocation::Preview(PreviewArgs {
                input: PathBuf::from("-plan.md"),
                config: None,
                output: None,
                open: true,
            }))
        );
    }

    #[test]
    fn help_and_version_win_over_other_arguments() {
        assert_eq!(parse(&["plan.md", "--help"]), Ok(Invocation::Help));
        assert_eq!(parse(&["-V"]), Ok(Invocation::Version));
    }

    #[test]
    fn rejects_invalid_invocations() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["a.md", "b.md"]).is_err());
        assert!(parse(&["--unknown", "a.md"]).is_err());
        assert!(parse(&["a.md", "--config"]).is_err());
        assert!(parse(&["a.md", "--output="]).is_err());
    }

    #[test]
    fn output_dir_is_stable_per_document() {
        let first = default_output_dir(Path::new("/tmp/a.md"));
        assert_eq!(first, default_output_dir(Path::new("/tmp/a.md")));
        assert_ne!(first, default_output_dir(Path::new("/tmp/b.md")));
    }

    #[cfg(unix)]
    #[test]
    fn launch_reports_a_failed_opener() {
        use super::launch;
        use std::process::Command;
        use std::time::Duration;

        let wait = Duration::from_secs(5);
        assert!(launch(Command::new("true"), wait).is_ok());
        let error = launch(Command::new("false"), wait).expect_err("false must fail");
        assert!(error.to_string().contains("false"));
        assert!(launch(Command::new("miyaburu-no-such-opener"), wait).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn launch_does_not_wait_for_a_foreground_browser() {
        use super::launch;
        use std::process::Command;
        use std::time::{Duration, Instant};

        let started = Instant::now();
        let mut command = Command::new("sleep");
        command.arg("3");
        assert!(launch(command, Duration::from_millis(100)).is_ok());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn nonce_is_alphanumeric() {
        let value = nonce();
        assert_eq!(value.len(), 32);
        assert!(value.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
