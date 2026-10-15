//! 偽トンネルで共有CLIを検証する。外部ネットワークには接続しない。
#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Workspace(PathBuf);
impl Workspace {
    fn new(script: &str) -> Self {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mb-share-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("bin")).unwrap();
        let this = Self(root);
        this.write("note.md", "# 初版\n");
        this.write(
            "bin/cloudflared",
            &script.replacen("#!/bin/sh\n", "#!/bin/sh\necho $$ >> \"$FAKE_PIDS\"\n", 1),
        );
        fs::set_permissions(
            this.0.join("bin/cloudflared"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        this
    }
    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).unwrap();
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_miyaburu"));
        command
            .args(args)
            .current_dir(&self.0)
            .env("HOME", &self.0)
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_CACHE_HOME", self.0.join("cache"))
            .env("PATH", self.0.join("bin"))
            .env("FAKE_LOG", self.0.join("origins"))
            .env("FAKE_PIDS", self.0.join("pids"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn origins(&self) -> Vec<String> {
        fs::read_to_string(self.0.join("origins"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = self.run(&["--stop-all"]);
        // 失敗時にデーモンが先に消えても、偽トンネルを残さない。
        if let Ok(pids) = fs::read_to_string(self.0.join("pids")) {
            for pid in pids.lines().filter(|pid| pid.parse::<u32>().is_ok()) {
                let _ = Command::new("/bin/kill").args(["-KILL", pid]).output();
            }
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}
const FAKE: &str = "#!/bin/sh\nwhile [ $# -gt 0 ]; do\n if [ \"$1\" = --url ]; then shift; echo \"$1\" >> \"$FAKE_LOG\"; fi\n shift\ndone\necho https://fake.trycloudflare.com >&2\nexec /bin/sleep 120\n";
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn request(origin: &str, method: &str, path: &str) -> String {
    String::from_utf8(request_bytes(origin, method, path, Duration::ZERO)).unwrap()
}
fn request_bytes(origin: &str, method: &str, path: &str, delay: Duration) -> Vec<u8> {
    let mut socket = TcpStream::connect(origin.strip_prefix("http://").unwrap()).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    std::thread::sleep(delay);
    write!(
        socket,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = Vec::new();
    socket.read_to_end(&mut response).unwrap();
    response
}
fn response_body(response: &[u8]) -> &[u8] {
    let end = response
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("HTTPヘッダーがありません: {} bytes", response.len()))
        + 4;
    let headers = std::str::from_utf8(&response[..end]).unwrap();
    assert!(headers.starts_with("HTTP/1.1 200"));
    let length: usize = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .expect("Content-Length");
    assert_eq!(length, response.len() - end);
    &response[end..]
}
#[test]
fn default_sharing_updates_and_enforces_http_allowlist() {
    let workspace = Workspace::new(FAKE);
    let first = workspace.run(&["note.md"]);
    success(&first);
    assert_eq!(String::from_utf8_lossy(&first.stdout).lines().count(), 1);
    assert!(String::from_utf8_lossy(&first.stderr).contains("https://fake.trycloudflare.com"));
    let origin = &workspace.origins()[0];
    assert!(request(origin, "GET", "/").contains("初版"));
    for path in [
        "/../note.md",
        "/%2e%2e/note.md",
        "/control",
        "/note.md",
        "/assets/unknown",
    ] {
        assert!(
            request(origin, "GET", path).starts_with("HTTP/1.1 404"),
            "{path}"
        );
    }
    assert!(request(origin, "POST", "/").starts_with("HTTP/1.1 405"));
    let response = request(origin, "GET", "/index.html");
    assert!(
        response
            .to_ascii_lowercase()
            .contains("cache-control: no-store")
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("x-content-type-options: nosniff")
    );
    workspace.write("note.md", "# 更新版\n\n```mermaid\ngraph LR\nA-->B\n```\n");
    success(&workspace.run(&["--no-open", "note.md"]));
    assert_eq!(workspace.origins().len(), 1);
    assert!(request(origin, "GET", "/").contains("更新版"));
    let mermaid = request_bytes(origin, "GET", "/mermaid.min.js", Duration::ZERO);
    assert_eq!(response_body(&mermaid), miyaburu::page::MERMAID_JS);
    let delayed = request_bytes(origin, "GET", "/", Duration::from_millis(200));
    assert!(
        std::str::from_utf8(response_body(&delayed))
            .unwrap()
            .contains("更新版")
    );
    fs::remove_file(workspace.0.join("note.md")).unwrap();
    std::os::unix::fs::symlink(&workspace.0, workspace.0.join("alias")).unwrap();
    let document = workspace.0.join("alias/note.md");
    success(&workspace.run(&["--stop", document.to_str().unwrap()]));
    success(&workspace.run(&["--stop", document.to_str().unwrap()]));
    assert!(TcpStream::connect(origin.strip_prefix("http://").unwrap()).is_err());
}
#[test]
fn concurrent_start_and_individual_and_all_stop() {
    let workspace = Workspace::new(FAKE);
    let first = workspace
        .command(&["--no-open", "note.md"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let second = workspace.run(&["--no-open", "note.md"]);
    success(&first.wait_with_output().unwrap());
    success(&second);
    assert_eq!(workspace.origins().len(), 1);
    workspace.write("other.md", "# 別文書\n");
    success(&workspace.run(&["--no-open", "other.md"]));
    let origins = workspace.origins();
    assert_eq!(origins.len(), 2);
    success(&workspace.run(&["--stop", "note.md"]));
    assert!(request(&origins[1], "GET", "/").contains("別文書"));
    success(&workspace.run(&["--stop-all"]));
    for origin in origins {
        assert!(TcpStream::connect(origin.strip_prefix("http://").unwrap()).is_err());
    }
    success(&workspace.run(&["--stop-all"]));
}
#[test]
fn large_image_update_preserves_binary_response_lengths() {
    let workspace = Workspace::new(FAKE);
    for (index, signature) in [
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".as_slice(),
        b"\xff\xd8\xff\xe0".as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        let mut image = signature.to_vec();
        image.resize(8 * 1024 * 1024 + 1024, 0x80 + index as u8);
        fs::write(workspace.0.join("image.bin"), &image).unwrap();
        workspace.write(
            "note.md",
            &format!("# 画像更新{index}\n\n![画像](image.bin)\n"),
        );
        let output = workspace.run(&["--no-open", "note.md"]);
        success(&output);
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("共有できません"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(workspace.origins().len(), 1);
        let origin = &workspace.origins()[0];
        let response = request_bytes(origin, "GET", "/", Duration::ZERO);
        let html = std::str::from_utf8(response_body(&response)).unwrap();
        assert!(html.contains(&format!("画像更新{index}")));
        let start = html.find("/assets/").expect("shared image");
        let path = html[start..].split('"').next().unwrap();
        let response = request_bytes(origin, "GET", path, Duration::ZERO);
        assert_eq!(response_body(&response), image);
    }
}

#[test]
fn connections_wait_when_response_slots_are_full() {
    let workspace = Workspace::new(FAKE);
    let mut image = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    image.resize(16 * 1024 * 1024, 42);
    fs::write(workspace.0.join("large.png"), image).unwrap();
    workspace.write("note.md", "# 順番待ち\n\n![画像](large.png)\n");
    let output = workspace.run(&["--no-open", "note.md"]);
    success(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("共有URL:"));
    let origin = &workspace.origins()[0];
    let html = request(origin, "GET", "/");
    let start = html.find("/assets/").unwrap();
    let path = html[start..].split('"').next().unwrap();
    let address = origin.strip_prefix("http://").unwrap();
    let mut held = Vec::new();
    for _ in 0..32 {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
        }
        // 本文を受信せず、配信の枠を占めたままにする。
        held.push(stream);
    }
    let mut pending = TcpStream::connect(address).unwrap();
    pending
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    pending
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut byte = [0];
    let result = pending.read(&mut byte);
    drop(held);
    assert!(matches!(result, Err(ref error) if matches!(error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)));
    pending
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut response = Vec::new();
    pending.read_to_end(&mut response).unwrap();
    assert!(response.starts_with(b"HTTP/1.1 200"));
    assert!(
        std::str::from_utf8(response_body(&response))
            .unwrap()
            .contains("順番待ち")
    );
}

/// パニック時も起動待ちのCLIを必ず回収する。
struct RunningCli(std::process::Child);
impl Drop for RunningCli {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait_until(mut condition: impl FnMut() -> bool, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !condition() {
        assert!(Instant::now() < deadline, "待機が上限を超えました");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn interrupted_start_leaves_no_tunnel_or_origin_and_flushes_path_first() {
    let slow = FAKE.replace("echo https://fake.trycloudflare.com >&2\n", "");
    let workspace = Workspace::new(&slow);
    let mut cli = RunningCli(
        workspace
            .command(&["--no-open", "note.md"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    // stdoutの受信も期限付きにし、共有URL待ちより先にパスが届くことを確かめる。
    let stdout = cli.0.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        let result = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = sender.send((result, line));
    });
    let (result, line) = receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("HTMLパスの即時表示");
    assert!(result.unwrap() > 0);
    assert!(line.trim().ends_with("index.html"));
    reader.join().unwrap();
    wait_until(
        || {
            !workspace.origins().is_empty()
                && fs::read_to_string(workspace.0.join("pids"))
                    .is_ok_and(|pids| !pids.trim().is_empty())
        },
        Duration::from_secs(5),
    );
    let origin = workspace.origins()[0].clone();
    let pid: i32 = fs::read_to_string(workspace.0.join("pids"))
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(cli.0.id() as i32, libc::SIGINT) }, 0);
    wait_until(
        || cli.0.try_wait().unwrap().is_some(),
        Duration::from_secs(3),
    );
    wait_until(|| unsafe { libc::kill(pid, 0) } == -1 && TcpStream::connect(origin.strip_prefix("http://").unwrap()).is_err(), Duration::from_secs(5));
}

#[test]
fn failures_fall_back_and_no_share_does_not_launch() {
    for script in ["#!/bin/sh\nexit 1\n", "#!/bin/sh\nexec /bin/sleep 120\n"] {
        let workspace = Workspace::new(script);
        let started = Instant::now();
        let output = workspace.run(&["--no-open", "note.md"]);
        success(&output);
        assert!(started.elapsed() < Duration::from_secs(20));
        assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 1);
        assert!(String::from_utf8_lossy(&output.stderr).contains("共有できません"));
    }
    let workspace = Workspace::new(FAKE);
    success(&workspace.run(&["--no-open", "--no-share", "note.md"]));
    assert!(workspace.origins().is_empty());
    fs::remove_file(workspace.0.join("bin/cloudflared")).unwrap();
    let output = workspace.run(&["--no-open", "note.md"]);
    success(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("共有できません"));
}
