//! Quick Tunnelの寿命と本人専用の管理接続。

use fs2::FileExt;
use miyaburu::render::SharedAsset;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(15);
const IO_TIMEOUT: Duration = Duration::from_secs(1);
// スマホ回線で大きな画像を返すため、HTTP応答は管理操作と別の期限にする。
const HTTP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const FRAME_LIMIT: usize = 128 * 1024 * 1024;
static TERMINATE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

extern "C" fn terminate(_: libc::c_int) {
    TERMINATE.store(true, Ordering::Relaxed);
}

// 部分読み込みを繰り返す相手にも、接続全体の期限を適用する。
struct ManagementStream {
    stream: UnixStream,
    deadline: Instant,
}
impl ManagementStream {
    fn new(stream: UnixStream, deadline: Instant) -> std::io::Result<Self> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        let timeout = Duration::from_millis(remaining.as_millis().max(1) as u64);
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream, deadline })
    }

    fn ready(&self, events: libc::c_short) -> std::io::Result<()> {
        loop {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(std::io::ErrorKind::TimedOut.into());
            }
            let mut poll = libc::pollfd {
                fd: self.stream.as_raw_fd(),
                events,
                revents: 0,
            };
            let result = unsafe {
                libc::poll(
                    &mut poll,
                    1,
                    remaining.as_millis().min(i32::MAX as u128).max(1) as i32,
                )
            };
            if result > 0 {
                return Ok(());
            }
            if result == 0 {
                return Err(std::io::ErrorKind::TimedOut.into());
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}
impl Read for ManagementStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        loop {
            self.ready(libc::POLLIN)?;
            match self.stream.read(bytes) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => return result,
            }
        }
    }
}
impl Write for ManagementStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        loop {
            self.ready(libc::POLLOUT)?;
            match self.stream.write(bytes) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 共有ページと、許可した画像の一括スナップショット。
/// @doc docs/specs/sharing.md#スナップショット
#[derive(Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub html: Vec<u8>,
    pub assets: Vec<SharedAsset>,
}

#[derive(Serialize, Deserialize)]
enum Request {
    Update(Snapshot),
    Status,
    Stop,
}

fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn private_dir(path: &Path) -> Result<(), String> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(message(e)),
    }
    let metadata = fs::symlink_metadata(path).map_err(message)?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err("共有用ディレクトリの所有者または権限が不正です".into());
    }
    Ok(())
}

fn root(cache: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(cache).map_err(message)?;
    let root = cache.join("sharing");
    private_dir(&root)?;
    Ok(root)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    // FNV-1aを固定定数で実装し、RustのHash実装やツールチェーンに依存させない。
    bytes.iter().fold(14695981039346656037, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
    })
}

fn session(document: &Path, cache: &Path) -> Result<PathBuf, String> {
    let path = root(cache)?.join(format!(
        "{:016x}",
        stable_hash(document.as_os_str().as_bytes())
    ));
    private_dir(&path)?;
    Ok(path)
}

fn socket_path(dir: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(dir).map_err(message)?;
    let hash = stable_hash(canonical.as_os_str().as_bytes());
    let sockets = PathBuf::from(format!("/tmp/miyaburu-share-{}", unsafe {
        libc::geteuid()
    }));
    private_dir(&sockets)?;
    Ok(sockets.join(format!("{hash:016x}.sock")))
}

fn lock_file(dir: &Path, name: &str) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join(name))
        .map_err(message)
}

fn lock_until(file: &File, deadline: Instant) -> Result<(), String> {
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(e) => return Err(message(e)),
        }
        if Instant::now() >= deadline {
            return Err("共有管理の待機が15秒を超えました".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn write_frame(stream: &mut impl Write, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > FRAME_LIMIT {
        return Err("共有スナップショットが大きすぎます".into());
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(message)?;
    stream.write_all(bytes).map_err(message)
}

fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut size = [0; 4];
    stream.read_exact(&mut size).map_err(message)?;
    let size = u32::from_be_bytes(size) as usize;
    if size > FRAME_LIMIT {
        return Err("管理メッセージが大きすぎます".into());
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes).map_err(message)?;
    Ok(bytes)
}

fn connect_management(path: &Path, deadline: Instant) -> Result<UnixStream, String> {
    // UnixStream::connect自体にはtimeoutがないため、非同期接続で待機を制限する。
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() >= address.sun_path.len() {
        return Err("管理ソケットのパスが長すぎます".into());
    }
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as libc::c_char;
    }
    #[cfg(target_os = "macos")]
    {
        address.sun_len = std::mem::size_of_val(&address) as u8;
    }
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(message(std::io::Error::last_os_error()));
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(message(std::io::Error::last_os_error()));
    }
    stream.set_nonblocking(true).map_err(message)?;
    let connected = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as libc::socklen_t,
        )
    };
    if connected != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(message(error));
        }
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("管理接続がタイムアウトしました".into());
            }
            let mut poll = libc::pollfd {
                fd: stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let result = unsafe {
                libc::poll(
                    &mut poll,
                    1,
                    remaining.as_millis().min(i32::MAX as u128).max(1) as i32,
                )
            };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(message(error));
            }
            if result == 0 {
                return Err("管理接続がタイムアウトしました".into());
            }
            let mut error: libc::c_int = 0;
            let mut size = std::mem::size_of_val(&error) as libc::socklen_t;
            if unsafe {
                libc::getsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut error as *mut libc::c_int).cast(),
                    &mut size,
                )
            } < 0
            {
                return Err(message(std::io::Error::last_os_error()));
            }
            if error != 0 {
                return Err(message(std::io::Error::from_raw_os_error(error)));
            }
            break;
        }
    }
    stream.set_nonblocking(false).map_err(message)?;
    Ok(stream)
}

fn request(dir: &Path, request: &Request, deadline: Instant) -> Result<String, String> {
    request_frame(
        dir,
        &serde_json::to_vec(request).map_err(message)?,
        deadline,
    )
}

fn request_frame(dir: &Path, bytes: &[u8], deadline: Instant) -> Result<String, String> {
    if bytes.len() > FRAME_LIMIT {
        return Err("共有スナップショットが大きすぎます".into());
    }
    if Instant::now() >= deadline {
        return Err("共有管理がタイムアウトしました".into());
    }
    let stream =
        connect_management(&socket_path(dir)?, deadline).map_err(|e| format!("管理接続: {e}"))?;
    let mut stream = ManagementStream::new(stream, deadline).map_err(message)?;
    write_frame(&mut stream, bytes).map_err(|e| format!("管理送信: {e}"))?;
    let bytes = read_frame(&mut stream).map_err(|e| format!("管理受信: {e}"))?;
    let response = serde_json::from_slice::<Result<String, String>>(&bytes).map_err(message)?;
    if Instant::now() >= deadline {
        return Err("共有管理がタイムアウトしました".into());
    }
    response
}

fn snapshot_frame(snapshot: &Snapshot) -> Result<Vec<u8>, String> {
    let mut bytes = b"{\"Update\":".to_vec();
    serde_json::to_writer(&mut bytes, snapshot).map_err(message)?;
    bytes.push(b'}');
    if bytes.len() > FRAME_LIMIT {
        return Err("共有スナップショットが大きすぎます".into());
    }
    Ok(bytes)
}

fn alive(dir: &Path) -> Result<bool, String> {
    let file = lock_file(dir, "alive.lock")?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
        Err(e) => Err(message(e)),
    }
}

/// 文書の共有を起動し、生存中ならURLを再利用して一括更新する。
/// @doc docs/specs/sharing.md#共有の起動と更新
pub(crate) fn start_or_update(
    document: &Path,
    snapshot: &Snapshot,
    cache_root: &Path,
) -> Result<String, String> {
    let deadline = Instant::now() + TIMEOUT;
    let dir = session(document, cache_root)?;
    let control = lock_file(&dir, "control.lock")?;
    lock_until(&control, deadline)?;
    if alive(&dir)? {
        return snapshot_frame(snapshot)
            .and_then(|bytes| request_frame(&dir, &bytes, deadline))
            .map_err(|error| {
                format!("{error}。既存の共有は更新前の内容で継続している可能性があります")
            });
    }
    // 更新フレーム内の同じJSONを初期スナップショットにも使う。
    let bytes = snapshot_frame(snapshot)?;
    if Instant::now() >= deadline {
        return Err("共有管理がタイムアウトしました".into());
    }
    let mut initial = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("snapshot.json"))
        .map_err(message)?;
    initial
        .write_all(&bytes[b"{\"Update\":".len()..bytes.len() - 1])
        .map_err(message)?;
    let log = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("daemon.log"))
        .map_err(message)?;
    let mut command = Command::new(std::env::current_exe().map_err(message)?);
    command
        .arg("--share-daemon")
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(log.try_clone().map_err(message)?))
        .stderr(Stdio::from(log));
    // 子の端末からの切り離しだけを、fork直後に行う。
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut startup = Starting(Some(command.spawn().map_err(message)?));
    loop {
        if Instant::now() + IO_TIMEOUT >= deadline {
            return Err("共有URLの取得が15秒を超えました".into());
        }
        if let Ok(url) = request(&dir, &Request::Status, deadline - IO_TIMEOUT) {
            let mut acknowledgement = startup
                .0
                .as_mut()
                .unwrap()
                .stdin
                .take()
                .ok_or("共有の起動確認パイプを取得できません")?;
            acknowledgement.write_all(b"R").map_err(message)?;
            drop(acknowledgement);
            startup.0.take();
            return Ok(url);
        }
        if startup
            .0
            .as_mut()
            .unwrap()
            .try_wait()
            .map_err(message)?
            .is_some()
        {
            return Err("共有プロセスが終了しました。daemon.logを確認してください".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn stop_dir(dir: &Path, deadline: Instant) -> Result<(), String> {
    let control = lock_file(dir, "control.lock")?;
    lock_until(&control, deadline)?;
    if alive(dir)? {
        request(dir, &Request::Stop, deadline)?;
        let life = lock_file(dir, "alive.lock")?;
        lock_until(&life, deadline)?;
    }
    // 生存ロックを確認した後だけ、古いソケットを削除する。
    let path = socket_path(dir)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(message(e)),
    }
}

/// 指定文書の共有を停止する。停止済みでも成功する。
/// @doc docs/specs/sharing.md#共有の停止
pub(crate) fn stop(document: &Path, cache_root: &Path) -> Result<(), String> {
    stop_dir(&session(document, cache_root)?, Instant::now() + TIMEOUT)
}

/// このキャッシュに属する共有をすべて停止する。
/// @doc docs/specs/sharing.md#全共有の停止
pub(crate) fn stop_all(cache_root: &Path) -> Result<(), String> {
    let deadline = Instant::now() + TIMEOUT;
    let mut errors = Vec::new();
    for entry in fs::read_dir(root(cache_root)?).map_err(message)? {
        let result = (|| {
            let path = entry.map_err(message)?.path();
            if fs::symlink_metadata(&path).map_err(message)?.is_dir() {
                private_dir(&path)
                    .and_then(|()| stop_dir(&path, deadline))
                    .map_err(|error| format!("{}: {error}", path.display()))?;
            }
            Ok::<(), String>(())
        })();
        if let Err(error) = result {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

// 起動途中のすべてのエラー経路で、所有している子だけを掃除する。
struct Starting(Option<Child>);
impl Drop for Starting {
    fn drop(&mut self) {
        let Some(child) = &mut self.0 else { return };
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        unsafe {
            libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
        }
        let deadline = Instant::now() + IO_TIMEOUT;
        while Instant::now() < deadline {
            if !matches!(child.try_wait(), Ok(None)) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        // setsidで作った、この子の専用グループを掃除する。
        unsafe {
            libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

struct Tunnel(Child);
impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct SocketGuard(PathBuf);
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn public_url(line: &str) -> Option<String> {
    for (offset, _) in line.match_indices("https://") {
        let candidate: String = line[offset..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '-' | '.'))
            .collect();
        if let Some(host) = candidate
            .strip_prefix("https://")
            .and_then(|s| s.strip_suffix(".trycloudflare.com"))
            && !host.is_empty()
            && host.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Some(candidate);
        }
    }
    None
}

fn http(mut stream: TcpStream, snapshot: Arc<RwLock<Arc<Snapshot>>>) {
    // macOSではacceptにlistenerのO_NONBLOCKが継承される。
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let mut header = Vec::new();
    let mut byte = [0];
    let deadline = Instant::now() + IO_TIMEOUT;
    while header.len() < 8192 && Instant::now() < deadline {
        if stream.read_exact(&mut byte).is_err() {
            return;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    if !header.ends_with(b"\r\n\r\n") {
        return;
    }
    let text = String::from_utf8_lossy(&header);
    let mut parts = text.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("").split('?').next().unwrap_or("");
    let snapshot = match snapshot.read() {
        Ok(s) => Arc::clone(&s),
        Err(_) => return,
    };
    let (status, content_type, body): (&str, &str, &[u8]) = if method != "GET" {
        (
            "405 Method Not Allowed",
            "text/plain",
            b"Method Not Allowed",
        )
    } else if path == "/" || path == "/index.html" {
        ("200 OK", "text/html; charset=utf-8", &snapshot.html)
    } else if let Some(asset) = snapshot.assets.iter().find(|a| a.path == path) {
        ("200 OK", &asset.content_type, &asset.bytes)
    } else {
        ("404 Not Found", "text/plain", b"Not Found")
    };
    if content_type.contains(['\r', '\n']) {
        return;
    }
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n{}\r\n",
        body.len(),
        if method != "GET" {
            "Allow: GET\r\n"
        } else {
            ""
        }
    );
    // 受信を少しずつ進める相手にも、応答全体の書き込み期限を適用する。
    if stream.set_nonblocking(true).is_err() {
        return;
    }
    let deadline = Instant::now() + HTTP_RESPONSE_TIMEOUT;
    for mut bytes in [header.as_bytes(), body] {
        while !bytes.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            let mut poll = libc::pollfd {
                fd: stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
            if ready <= 0 {
                return;
            }
            match stream.write(bytes) {
                Ok(0) => return,
                Ok(size) => bytes = &bytes[size..],
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    continue;
                }
                Err(_) => return,
            }
        }
    }
}

/// 内部引数から起動する共有デーモン。HTTPと本人専用IPCを提供する。
/// @doc docs/specs/sharing.md#共有デーモン
pub(crate) fn daemon(session_dir: &Path) -> Result<(), String> {
    private_dir(session_dir)?;
    // ハンドラーでは終了フラグだけを更新し、kill/waitは通常経路で行う。
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = terminate as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut()) != 0 {
            return Err(message(std::io::Error::last_os_error()));
        }
    }
    thread::spawn(|| {
        let mut marker = [0];
        if std::io::stdin().read_exact(&mut marker).is_err() || marker != *b"R" {
            TERMINATE.store(true, Ordering::Relaxed);
        }
    });
    let life = lock_file(session_dir, "alive.lock")?;
    life.try_lock_exclusive().map_err(message)?;
    let mut bytes = Vec::new();
    File::open(session_dir.join("snapshot.json"))
        .map_err(message)?
        .take((FRAME_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(message)?;
    if bytes.len() > FRAME_LIMIT {
        return Err("共有スナップショットが大きすぎます".into());
    }
    let initial: Snapshot = serde_json::from_slice(&bytes).map_err(message)?;
    let snapshot = Arc::new(RwLock::new(Arc::new(initial)));
    let listener = TcpListener::bind("127.0.0.1:0").map_err(message)?;
    listener.set_nonblocking(true).map_err(message)?;
    let address = listener.local_addr().map_err(message)?;
    let socket = socket_path(session_dir)?;
    let _ = fs::remove_file(&socket);
    let management = UnixListener::bind(&socket).map_err(message)?;
    let _socket = SocketGuard(socket.clone());
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(message)?;
    management.set_nonblocking(true).map_err(message)?;
    let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let http_stop = stopping.clone();
    let http_snapshot = snapshot.clone();
    let server = thread::spawn(move || {
        let active = Arc::new(AtomicUsize::new(0));
        while !http_stop.load(Ordering::Relaxed) {
            // 枠が空くまではacceptせず、カーネルの待ち行列に残す。
            if active.load(Ordering::Relaxed) >= 32 {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            if let Ok((stream, _)) = listener.accept() {
                active.fetch_add(1, Ordering::Relaxed);
                let active = active.clone();
                let snapshot = http_snapshot.clone();
                thread::spawn(move || {
                    http(stream, snapshot);
                    active.fetch_sub(1, Ordering::Relaxed);
                });
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });
    let result = run_tunnel(session_dir, address, &management, &snapshot);
    stopping.store(true, Ordering::Relaxed);
    let _ = server.join();
    result
}

fn run_tunnel(
    dir: &Path,
    address: std::net::SocketAddr,
    management: &UnixListener,
    snapshot: &Arc<RwLock<Arc<Snapshot>>>,
) -> Result<(), String> {
    // バインドだけでなく、実際のHTTPルートが応答することも確認する。
    let mut probe = TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(message)?;
    probe.set_read_timeout(Some(IO_TIMEOUT)).map_err(message)?;
    probe.set_write_timeout(Some(IO_TIMEOUT)).map_err(message)?;
    probe
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .map_err(message)?;
    let mut prefix = [0; 16];
    probe.read_exact(&mut prefix).map_err(message)?;
    if &prefix != b"HTTP/1.1 200 OK\r" {
        return Err("共有HTTPルートが応答しません".into());
    }
    if TERMINATE.load(Ordering::Relaxed) {
        return Ok(());
    }
    let child = Command::new("cloudflared")
        .args([
            "tunnel",
            "--no-autoupdate",
            "--url",
            &format!("http://{address}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(message)?;
    let mut tunnel = Tunnel(child);
    let mut stderr = tunnel
        .0
        .stderr
        .take()
        .ok_or("cloudflaredのログを取得できません")?;
    let mut log = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("cloudflared.log"))
        .map_err(message)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        let mut pending = String::new();
        let mut found = false;
        loop {
            let size = match stderr.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let _ = log.write_all(&buffer[..size]);
            if !found {
                pending.extend(
                    buffer[..size]
                        .iter()
                        .map(|b| if b.is_ascii() { *b as char } else { ' ' }),
                );
                while let Some(end) = pending.find('\n') {
                    if let Some(url) = public_url(&pending[..end]) {
                        let _ = sender.try_send(url);
                        found = true;
                    }
                    pending.drain(..=end);
                }
                if pending.len() > 8192 {
                    pending.drain(..pending.len() - 4096);
                }
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(13);
    let mut url = None;
    loop {
        if TERMINATE.load(Ordering::Relaxed) {
            return Ok(());
        }
        if tunnel.0.try_wait().map_err(message)?.is_some() {
            return Err("cloudflaredが終了しました".into());
        }
        if let Ok(value) = receiver.try_recv() {
            url = Some(value);
        }
        if url.is_none() && Instant::now() >= deadline {
            return Err("共有URLの取得がタイムアウトしました".into());
        }
        match management.accept() {
            Ok((stream, _)) => {
                let mut stream = match ManagementStream::new(stream, Instant::now() + TIMEOUT) {
                    Ok(stream) => stream,
                    Err(_) => continue,
                };
                let request = read_frame(&mut stream)
                    .and_then(|b| serde_json::from_slice::<Request>(&b).map_err(message));
                let request = request.and_then(|request| {
                    if Instant::now() >= stream.deadline {
                        Err("共有管理がタイムアウトしました".into())
                    } else {
                        Ok(request)
                    }
                });
                let mut stop = false;
                let response = match request {
                    Ok(Request::Stop) => {
                        stop = true;
                        Ok(String::new())
                    }
                    Ok(Request::Status) => url.clone().ok_or_else(|| "共有を準備しています".into()),
                    Ok(Request::Update(next)) => {
                        if let Some(url) = &url {
                            match snapshot.write() {
                                Ok(mut current) => {
                                    *current = Arc::new(next);
                                    Ok(url.clone())
                                }
                                Err(e) => Err(message(e)),
                            }
                        } else {
                            Err("共有を準備しています".into())
                        }
                    }
                    Err(e) => Err(e),
                };
                let _ = write_frame(
                    &mut stream,
                    &serde_json::to_vec(&response).map_err(message)?,
                );
                if stop {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(message(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_process_slow_response_is_complete() {
        let body = vec![42; 16 * 1024 * 1024];
        let expected_length = body.len();
        let snapshot = Arc::new(RwLock::new(Arc::new(Snapshot {
            html: vec![],
            assets: vec![SharedAsset {
                path: "/assets/slow".into(),
                content_type: "image/png".into(),
                bytes: body,
            }],
        })));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || http(listener.accept().unwrap().0, snapshot));
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client
            .write_all(b"GET /assets/slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            client.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
        }
        // スマホ回線で一時的に受信が止まっても、画像全体を返す。
        thread::sleep(Duration::from_millis(1200));
        let mut received = Vec::new();
        client.read_to_end(&mut received).unwrap();
        worker.join().unwrap();
        assert!(
            String::from_utf8(header)
                .unwrap()
                .contains(&format!("Content-Length: {expected_length}\r\n"))
        );
        assert_eq!(received.len(), expected_length);
        assert!(received.iter().all(|byte| *byte == 42));
    }

    #[test]
    fn share_process_url_allowlist() {
        assert_eq!(
            public_url("INF https://quiet-paper.trycloudflare.com |"),
            Some("https://quiet-paper.trycloudflare.com".into())
        );
        assert!(public_url("https://evil.example/ https://a.trycloudflare.com.evil").is_none());
    }
    #[test]
    fn share_process_management_deadline() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        let worker = thread::spawn(move || {
            for _ in 0..8 {
                if writer.write_all(&[0]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(40));
            }
        });
        let started = Instant::now();
        let mut reader =
            ManagementStream::new(reader, started + Duration::from_millis(100)).unwrap();
        assert!(read_frame(&mut reader).is_err());
        assert!(started.elapsed() < Duration::from_millis(250));
        drop(reader);
        worker.join().unwrap();
    }

    #[test]
    #[ignore = "共有プロセステストから別プロセスとして呼び出す"]
    fn share_process_daemon_child() {
        let dir = std::env::var_os("MIYABURU_PROCESS_TEST_DIR").unwrap();
        daemon(Path::new(&dir)).unwrap();
    }

    #[test]
    fn share_process_stable_hash() {
        assert_eq!(stable_hash(b""), 0xcbf29ce484222325);
        assert_eq!(stable_hash(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(stable_hash(b"foobar"), 0x85944171f73967e8);
        assert_eq!(stable_hash(b"hello"), 0xa430d84680aabd0b);
    }

    #[test]
    fn share_process_daemon_update_stop() {
        daemon_fixture(true);
    }

    #[test]
    fn share_process_parent_eof_before_ack() {
        daemon_fixture(false);
    }

    fn daemon_fixture(acknowledge: bool) {
        struct Fixture {
            dir: PathBuf,
            child: Option<Child>,
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                if let Some(child) = &mut self.child
                    && matches!(child.try_wait(), Ok(None))
                {
                    unsafe {
                        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
                    }
                    let _ = child.wait();
                }
                if let Ok(socket) = socket_path(&self.dir) {
                    let _ = fs::remove_file(socket);
                }
                let _ = fs::remove_dir_all(&self.dir);
            }
        }
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("miyaburu-process-{}-{unique}", std::process::id()));
        private_dir(&dir).unwrap();
        let mut fixture = Fixture {
            dir: dir.clone(),
            child: None,
        };
        let initial = Snapshot {
            html: b"before".to_vec(),
            assets: vec![],
        };
        fs::write(
            dir.join("snapshot.json"),
            serde_json::to_vec(&initial).unwrap(),
        )
        .unwrap();
        // PATHを隔離する前に、Nix環境でも利用できるsleepを特定します。
        let sleep = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|path| path.join("sleep"))
            .find(|path| path.is_file())
            .expect("テスト用のsleepが見つかりません");
        let fake = dir.join("cloudflared");
        fs::write(&fake, b"#!/bin/sh\nprintf '%s' \"$4\" > \"$MIYABURU_PROCESS_TEST_DIR/origin\"\nprintf '%s' \"$$\" > \"$MIYABURU_PROCESS_TEST_DIR/tunnel-pid\"\nprintf '%s\\n' 'https://process-test.trycloudflare.com' >&2\nexec \"$MIYABURU_TEST_SLEEP\" 120\n").unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
        let log = File::create(dir.join("test.log")).unwrap();
        fixture.child = Some(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "share::tests::share_process_daemon_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("MIYABURU_PROCESS_TEST_DIR", &dir)
                .env("MIYABURU_TEST_SLEEP", sleep)
                .env("PATH", &dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(url) = request(&dir, &Request::Status, deadline) {
                assert_eq!(url, "https://process-test.trycloudflare.com");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "共有デーモンが起動しません: {}",
                fs::read_to_string(dir.join("test.log")).unwrap()
            );
            thread::sleep(Duration::from_millis(20));
        }
        assert!(alive(&dir).unwrap());
        let mut acknowledgement = fixture.child.as_mut().unwrap().stdin.take().unwrap();
        if acknowledge {
            acknowledgement.write_all(b"R").unwrap();
        }
        drop(acknowledgement);
        if !acknowledge {
            loop {
                if let Some(status) = fixture.child.as_mut().unwrap().try_wait().unwrap() {
                    assert!(status.success());
                    fixture.child = None;
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "親のEOF後もデーモンが生存しています"
                );
                thread::sleep(Duration::from_millis(20));
            }
            let pid: libc::pid_t = fs::read_to_string(dir.join("tunnel-pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
            assert!(!alive(&dir).unwrap());
            return;
        }
        let next = Snapshot {
            html: b"after".to_vec(),
            assets: vec![],
        };
        assert_eq!(
            request(&dir, &Request::Update(next), deadline).unwrap(),
            "https://process-test.trycloudflare.com"
        );
        let origin = fs::read_to_string(dir.join("origin")).unwrap();
        let mut client = TcpStream::connect(origin.trim_start_matches("http://")).unwrap();
        client.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.ends_with("after"));
        stop_dir(&dir, deadline).unwrap();
        assert!(fixture.child.as_mut().unwrap().wait().unwrap().success());
        fixture.child = None;
        assert!(!alive(&dir).unwrap());
        stop_dir(&dir, deadline).unwrap();
    }

    #[test]
    fn share_process_http_delayed_header() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
        let deadline = Instant::now() + IO_TIMEOUT;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("HTTP接続のacceptに失敗: {error}"),
            }
        };
        // 継承しないOSでも、macOSと同じaccept直後の状態を再現する。
        stream.set_nonblocking(true).unwrap();
        let snapshot = Arc::new(RwLock::new(Arc::new(Snapshot {
            html: b"delayed".to_vec(),
            assets: vec![],
        })));
        let worker = thread::spawn(move || http(stream, snapshot));
        thread::sleep(Duration::from_millis(100));
        client.write_all(b"GET / HTTP/1.1\r\n").unwrap();
        thread::sleep(Duration::from_millis(50));
        client.write_all(b"Host: localhost\r\n\r\n").unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.ends_with("delayed"));
        worker.join().unwrap();
    }

    #[test]
    fn share_process_http_routes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let snapshot = Arc::new(RwLock::new(Arc::new(Snapshot {
            html: b"page".to_vec(),
            assets: vec![],
        })));
        let worker = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            http(stream, snapshot);
        });
        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(b"GET /../snapshot.json HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 404"));
        assert!(response.contains("Cache-Control: no-store"));
        worker.join().unwrap();
    }
}
