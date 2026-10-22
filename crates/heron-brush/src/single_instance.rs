//! One editor per user: launching
//! `heron-brush map.map` while an editor runs hands the map to that editor, which opens it
//! (asking about unsaved changes as File > Open does) and comes to the front, and the new
//! process leaves with status 0. `--new-instance` opens a window of its own regardless.
//!
//! The running editor listens on a per-user local endpoint: a Unix domain socket (mode
//! 0600) in `$XDG_RUNTIME_DIR` or the per-user temporary directory, and on Windows a
//! localhost TCP port written with a random token to a file in the config directory, the
//! token proving the caller can read that user's files. The endpoint's name is derived
//! from the config directory, so a sandboxed config (`XDG_CONFIG_HOME`) gets an editor
//! of its own. An endpoint left behind by a crash answers nothing and is taken over.
//!
//! The exchange is two lines from the caller and one back:
//!
//! ```text
//! heron-brush-instance 1 <token or ->
//! open <escaped absolute path>        or   activate
//!                                          ok
//! ```

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use web_time::Duration;

/// The first word of every exchange, with the protocol version.
pub const MAGIC: &str = "heron-brush-instance 1";
/// The flag that skips handing over.
pub const NEW_INSTANCE: &str = "--new-instance";
/// How long either end waits on the other before giving up.
const TIMEOUT: Duration = Duration::from_secs(3);
/// The longest line either end reads, so a stray client cannot make the editor buffer
/// without end.
const MAX_LINE: u64 = 64 * 1024;

/// What a second launch asks of the running editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open this map.
    Open(PathBuf),
    /// Come to the front; no map was named.
    Activate,
}

/// Takes `--new-instance` out of the arguments, wherever it is. Returns whether it was
/// there.
pub fn strip_new_instance(args: &mut Vec<String>) -> bool {
    let before = args.len();
    args.retain(|arg| arg != NEW_INSTANCE);
    args.len() != before
}

/// Writes a path as one line of printable ASCII: bytes outside it, and the backslash,
/// as `\xHH`.
pub fn escape(path: &Path) -> String {
    let mut out = String::new();
    for byte in path_bytes(path) {
        if (0x20..0x7f).contains(&byte) && byte != b'\\' {
            out.push(byte as char);
        } else {
            out.push_str(&format!("\\x{byte:02x}"));
        }
    }
    out
}

/// Reads a path [`escape`] wrote.
pub fn unescape(text: &str) -> Result<PathBuf, String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'\\' {
            let hex = tail
                .get(..3)
                .filter(|h| h[0] == b'x')
                .ok_or("a bad escape")?;
            let value = std::str::from_utf8(&hex[1..])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or("a bad escape")?;
            bytes.push(value);
            rest = &tail[3..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    path_from_bytes(bytes)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> Result<PathBuf, String> {
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: Vec<u8>) -> Result<PathBuf, String> {
    String::from_utf8(bytes)
        .map(PathBuf::from)
        .map_err(|_| "a path that is not UTF-8".to_string())
}

/// The request line.
pub fn encode(request: &Request) -> String {
    match request {
        Request::Open(path) => format!("open {}", escape(path)),
        Request::Activate => "activate".to_string(),
    }
}

/// Reads a request line.
pub fn decode(line: &str) -> Result<Request, String> {
    let line = line.trim_end_matches(['\r', '\n']);
    match line.split_once(' ') {
        Some(("open", path)) if !path.is_empty() => unescape(path).map(Request::Open),
        None if line == "activate" => Ok(Request::Activate),
        _ => Err(format!("an unknown request: {line:?}")),
    }
}

fn read_line(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = String::new();
    reader.take(MAX_LINE).read_line(&mut line)?;
    if !line.ends_with('\n') {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the line was cut short",
        ));
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// The caller's side of the exchange: whether the editor at the other end took the
/// request.
pub fn send(stream: impl Read + Write, token: &str, request: &Request) -> io::Result<bool> {
    let mut reader = BufReader::new(stream);
    let message = format!(
        "{MAGIC} {}\n{}\n",
        if token.is_empty() { "-" } else { token },
        encode(request)
    );
    reader.get_mut().write_all(message.as_bytes())?;
    reader.get_mut().flush()?;
    Ok(read_line(&mut reader)? == "ok")
}

/// The running editor's side of the exchange: the request, when the caller spoke the
/// protocol and knew the token; it is told `ok` or `error` either way.
pub fn receive(stream: impl Read + Write, token: &str) -> Option<Request> {
    let mut reader = BufReader::new(stream);
    let request = (|| {
        let hello = read_line(&mut reader).ok()?;
        let given = hello.strip_prefix(MAGIC)?.strip_prefix(' ')?;
        if given != if token.is_empty() { "-" } else { token } {
            return None;
        }
        decode(&read_line(&mut reader).ok()?).ok()
    })();
    let _ = reader.get_mut().write_all(if request.is_some() {
        b"ok\n"
    } else {
        b"error\n"
    });
    let _ = reader.get_mut().flush();
    request
}

/// A short, stable name for the config directory, so each one gets an editor of its own.
fn tag(config: &Path) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in path_bytes(config) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:012x}", hash & 0xffff_ffff_ffff)
}

/// Where the running editor listens.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// The socket (Unix) or the file naming the port and token (elsewhere).
    pub path: PathBuf,
}

impl Endpoint {
    /// The endpoint for the editor using `config` as its config directory.
    pub fn for_config(config: &Path) -> Self {
        let tag = tag(config);
        #[cfg(unix)]
        {
            // A socket path has to fit in about a hundred bytes, which a config path
            // may not; the per-user runtime or temporary directory is short and private.
            let user = std::env::var("USER").unwrap_or_default();
            let dir = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .filter(|d| d.is_dir())
                .unwrap_or_else(std::env::temp_dir);
            Self {
                path: dir.join(format!("heron-brush-{user}-{tag}.sock")),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                path: config.join(format!("heron-brush-{tag}.instance")),
            }
        }
    }

    /// The endpoint for this user's editor.
    pub fn current() -> Self {
        Self::for_config(&editor_core::preferences::config_dir())
    }
}

/// How a launch came out.
pub enum Claim {
    /// No editor ran; this one listens now.
    Primary(Server),
    /// The running editor took the request; this process can leave.
    Forwarded,
    /// Neither: the endpoint could not be used, so this editor runs on its own.
    Alone(String),
}

/// Hands `request` to the editor listening at `endpoint`, or becomes that editor.
pub fn claim(endpoint: &Endpoint, request: &Request) -> Claim {
    // Twice: two launches at once may both find nobody and race for the endpoint; the
    // loser hands its request to the winner.
    let mut error = String::new();
    for _ in 0..2 {
        match forward(endpoint, request) {
            Ok(true) => return Claim::Forwarded,
            Ok(false) => return Claim::Alone("the running editor refused the request".into()),
            Err(_) => {}
        }
        match Server::bind(endpoint) {
            Ok(server) => return Claim::Primary(server),
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => error = e.to_string(),
            Err(e) => return Claim::Alone(e.to_string()),
        }
    }
    Claim::Alone(error)
}

/// Tries the running editor. `Err` means nobody answered: no editor runs, or the one
/// that did crashed and left its endpoint behind.
pub fn forward(endpoint: &Endpoint, request: &Request) -> io::Result<bool> {
    #[cfg(unix)]
    {
        let stream = std::os::unix::net::UnixStream::connect(&endpoint.path)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        send(stream, "", request)
    }
    #[cfg(not(unix))]
    {
        let text = std::fs::read_to_string(&endpoint.path)?;
        let (port, token) = text
            .trim()
            .split_once(' ')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a bad instance file"))?;
        let port: u16 = port
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a bad port"))?;
        let stream = std::net::TcpStream::connect_timeout(
            &(std::net::Ipv4Addr::LOCALHOST, port).into(),
            TIMEOUT,
        )?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        // Some other program on a port the dead editor once had does not answer `ok`,
        // which counts as nobody answering.
        match send(stream, token, request) {
            Ok(true) => Ok(true),
            Ok(false) | Err(_) => Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "no editor answered",
            )),
        }
    }
}

/// The running editor's endpoint, listening.
pub struct Server {
    #[cfg(unix)]
    listener: std::os::unix::net::UnixListener,
    #[cfg(not(unix))]
    listener: std::net::TcpListener,
    #[cfg(not(unix))]
    token: String,
    /// Removes the endpoint when the editor closes.
    cleanup: Cleanup,
}

/// Removes the endpoint file once the editor is done with it, if it is still this
/// editor's.
struct Cleanup {
    path: PathBuf,
    #[cfg(unix)]
    identity: Option<(u64, u64)>,
}

#[cfg(unix)]
fn identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path)
        .ok()
        .map(|m| (m.dev(), m.ino()))
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        #[cfg(unix)]
        let ours = self.identity.is_some() && identity(&self.path) == self.identity;
        #[cfg(not(unix))]
        let ours = true;
        if ours {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl Server {
    /// Listens at `endpoint`. A socket nobody answers on is a crashed editor's, and is
    /// replaced; one that answers is `AddrInUse`.
    pub fn bind(endpoint: &Endpoint) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            use std::os::unix::net::{UnixListener, UnixStream};
            let listener = match UnixListener::bind(&endpoint.path) {
                Ok(listener) => listener,
                Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                    if UnixStream::connect(&endpoint.path).is_ok() {
                        return Err(e);
                    }
                    std::fs::remove_file(&endpoint.path)?;
                    UnixListener::bind(&endpoint.path)?
                }
                Err(e) => return Err(e),
            };
            std::fs::set_permissions(&endpoint.path, std::fs::Permissions::from_mode(0o600))?;
            let cleanup = Cleanup {
                path: endpoint.path.clone(),
                identity: identity(&endpoint.path),
            };
            Ok(Self { listener, cleanup })
        }
        #[cfg(not(unix))]
        {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
            let token = random_token();
            if let Some(dir) = endpoint.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(
                &endpoint.path,
                format!("{} {token}\n", listener.local_addr()?.port()),
            )?;
            Ok(Self {
                listener,
                token,
                cleanup: Cleanup {
                    path: endpoint.path.clone(),
                },
            })
        }
    }

    /// Answers launches on a thread of its own, handing each request to `deliver`.
    /// Returns what removes the endpoint when dropped, which the editor keeps until it
    /// closes.
    pub fn serve(self, deliver: impl Fn(Request) + Send + 'static) -> Listening {
        let Self {
            listener, cleanup, ..
        } = self;
        #[cfg(not(unix))]
        let token = self.token;
        #[cfg(unix)]
        let token = String::new();
        std::thread::Builder::new()
            .name("single-instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let _ = stream.set_read_timeout(Some(TIMEOUT));
                    let _ = stream.set_write_timeout(Some(TIMEOUT));
                    if let Some(request) = receive(stream, &token) {
                        deliver(request);
                    }
                }
            })
            .ok();
        Listening { _cleanup: cleanup }
    }
}

/// A running editor's endpoint; dropping it removes the endpoint file.
pub struct Listening {
    _cleanup: Cleanup,
}

#[cfg(not(unix))]
fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for round in 0..2u32 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u32(round);
        hasher.write_u32(std::process::id());
        hasher.write_u128(
            web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_survive_the_wire() {
        let odd = PathBuf::from("/maps/a b\\c\nd é.map");
        for request in [
            Request::Open(PathBuf::from("/maps/crate.map")),
            Request::Open(odd),
            Request::Activate,
        ] {
            let line = encode(&request);
            assert!(!line.contains('\n') && line.is_ascii(), "{line:?}");
            assert_eq!(decode(&line), Ok(request));
        }
        assert!(decode("open").is_err());
        assert!(decode("open ").is_err());
        assert!(decode("close /x").is_err());
        assert!(decode("open bad\\x4").is_err());
        assert!(decode("open bad\\q41").is_err());
    }

    #[test]
    fn the_flag_is_taken_out_wherever_it_is() {
        let mut args = vec!["a.map".to_string(), NEW_INSTANCE.to_string()];
        assert!(strip_new_instance(&mut args));
        assert_eq!(args, vec!["a.map".to_string()]);
        let mut args = vec!["--version".to_string()];
        assert!(!strip_new_instance(&mut args));
        assert_eq!(args, vec!["--version".to_string()]);
    }

    /// One end of the exchange against the other, over a socket pair.
    #[cfg(unix)]
    #[test]
    fn a_caller_with_the_right_token_is_heard() {
        use std::os::unix::net::UnixStream;
        let exchange = |sent: &'static str, expected: &'static str| {
            let (caller, editor) = UnixStream::pair().unwrap();
            let editor = std::thread::spawn(move || receive(editor, expected));
            let answered = send(caller, sent, &Request::Activate).unwrap();
            (answered, editor.join().unwrap())
        };
        assert_eq!(exchange("", ""), (true, Some(Request::Activate)));
        assert_eq!(
            exchange("secret", "secret"),
            (true, Some(Request::Activate))
        );
        assert_eq!(exchange("guess", "secret"), (false, None));
    }

    #[cfg(unix)]
    #[test]
    fn a_stranger_gets_an_error_not_a_hang() {
        use std::os::unix::net::UnixStream;
        let (mut caller, editor) = UnixStream::pair().unwrap();
        let editor = std::thread::spawn(move || receive(editor, ""));
        caller.write_all(b"GET / HTTP/1.0\n").unwrap();
        let mut reply = String::new();
        BufReader::new(&caller).read_line(&mut reply).unwrap();
        assert_eq!(reply, "error\n");
        assert_eq!(editor.join().unwrap(), None);
    }

    fn scratch(name: &str) -> Endpoint {
        let config = std::env::temp_dir().join(format!(
            "heron-brush-instance-test-{name}-{}",
            std::process::id()
        ));
        let endpoint = Endpoint::for_config(&config);
        let _ = std::fs::remove_file(&endpoint.path);
        endpoint
    }

    #[test]
    fn each_config_directory_has_its_own_endpoint() {
        assert_ne!(
            Endpoint::for_config(Path::new("/a")).path,
            Endpoint::for_config(Path::new("/b")).path
        );
        assert_eq!(
            Endpoint::for_config(Path::new("/a")).path,
            Endpoint::for_config(Path::new("/a")).path
        );
        // Short enough for a Unix socket even for a deep config directory.
        let deep = PathBuf::from("/x".repeat(200));
        assert!(Endpoint::for_config(&deep).path.as_os_str().len() < 104 || cfg!(not(unix)));
    }

    #[test]
    fn the_second_launch_hands_its_map_to_the_first() {
        let endpoint = scratch("handover");
        let Claim::Primary(server) = claim(&endpoint, &Request::Activate) else {
            panic!("nobody ran, so the first launch listens")
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let listening = server.serve(move |request| tx.send(request).unwrap());
        let map = Request::Open(PathBuf::from("/maps/crate.map"));
        assert!(matches!(claim(&endpoint, &map), Claim::Forwarded));
        assert_eq!(rx.recv_timeout(TIMEOUT).unwrap(), map);
        drop(listening);
        assert!(!endpoint.path.exists(), "the endpoint goes with the editor");
    }

    #[cfg(unix)]
    #[test]
    fn a_crashed_editors_socket_does_not_block_the_next_launch() {
        let endpoint = scratch("stale");
        // A socket bound and abandoned without cleanup, as a crash leaves it.
        let listener = std::os::unix::net::UnixListener::bind(&endpoint.path).unwrap();
        drop(listener);
        assert!(endpoint.path.exists());
        assert!(
            forward(&endpoint, &Request::Activate).is_err(),
            "nobody answers on a dead socket"
        );
        let Claim::Primary(server) = claim(&endpoint, &Request::Activate) else {
            panic!("the stale socket is taken over")
        };
        drop(server);
    }

    #[cfg(unix)]
    #[test]
    fn a_stale_plain_file_is_replaced_too() {
        let endpoint = scratch("file");
        std::fs::write(&endpoint.path, "junk").unwrap();
        let Claim::Primary(server) = claim(&endpoint, &Request::Activate) else {
            panic!("a leftover file is replaced")
        };
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&endpoint.path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "only this user can talk to the editor"
        );
        drop(server);
    }
}
