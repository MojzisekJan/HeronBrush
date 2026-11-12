//! The transport for a live editing session.
//!
//! Two editors, one map, one link. A link is either
//!
//! - **TCP** ([`Link::host`] / [`Link::join`]) -- one binds a port, the other connects.
//!   Works on a LAN or with a port-forward, nothing else needed.
//! - **iroh** (`host_iroh` / `join_iroh`) -- one shares its endpoint id, the other
//!   dials it. iroh hole-punches and falls back to n0's free relays, so this reaches the
//!   other person across the open internet with no server to run.
//!
//! - **WebSocket** -- the web build, which cannot open TCP, joins a desktop host with
//!   `ws://address:port` ([`Link::join_websocket`]); the host's port takes a WebSocket
//!   handshake as well as the plain line protocol ([`Link::from_accepted`]), one text
//!   message per JSON line.
//!
//! Either way the link is symmetric once up: [`Link::send`] pushes a JSON line,
//! [`Link::drain`] takes the other side's off the inbox. Messages are newline-delimited,
//! one JSON [`WireMessage`](super::wire::WireMessage) per line. Reading blocks, so it
//! runs off the UI thread -- on its own OS threads for TCP, on a one-thread tokio runtime
//! for iroh. [`Link::inbox`] is a plain [`std::sync::mpsc::Receiver`] the UI drains
//! between frames.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// The largest world snapshot, which a guest gets as it joins.
pub const MAX_WORLD_FRAME: usize = 64 << 20;
/// The largest message that carries an edit: `Edit`, `TrackedEdit`, and `Relayed`, which
/// wraps one. An edit can hold the old and the new state of the whole map, so it may be
/// twice a world snapshot; a map small enough to send must never make an edit that is not.
pub const MAX_EDIT_FRAME: usize = 2 * MAX_WORLD_FRAME;
/// The largest of the messages that carry no map data: `Hello`, `Selection`, `Camera`
/// and `Left`.
pub const MAX_FRAME: usize = 8 << 20;
/// The most messages a link holds waiting, each way.
pub const MAX_QUEUED_MESSAGES: usize = 256;
/// The most bytes a link holds waiting, each way: one largest edit and the traffic
/// around it.
pub const MAX_QUEUED_BYTES: usize = 2 * MAX_EDIT_FRAME;

/// The size a message may reach, by what it is, from how its JSON opens. Until the
/// longest tag could be in, a frame might still be an edit.
pub fn frame_limit(start: &[u8]) -> usize {
    const EDITS: [&[u8]; 3] = [b"{\"Edit\"", b"{\"TrackedEdit\"", b"{\"Relayed\""];
    const WORLD: &[u8] = b"{\"World\"";
    if start.len() < 16 || EDITS.iter().any(|tag| start.starts_with(tag)) {
        MAX_EDIT_FRAME
    } else if start.starts_with(WORLD) {
        MAX_WORLD_FRAME
    } else {
        MAX_FRAME
    }
}

/// Whether `edit`, made in a world whose layers are `layers`, goes out within
/// [`MAX_EDIT_FRAME`] however it is wrapped: as a guest's tracked edit, relayed on by
/// the host. Measured by writing it, which a session does to send it anyway.
pub fn edit_fits(edit: &crate::history::Edit, layers: &[Option<String>]) -> bool {
    let wire = super::wire::WireEdit::from_edit_in(edit, layers);
    let json = super::wire::WireMessage::TrackedEdit {
        id: "0".repeat(32),
        edit: wire,
    }
    .to_json();
    // `Relayed` adds its tag and the sender's number around it.
    json.len() + 64 <= edit_limit()
}

#[cfg(not(test))]
fn edit_limit() -> usize {
    MAX_EDIT_FRAME
}

#[cfg(test)]
thread_local! {
    /// A smaller edit limit, so a test can reach it without a map of 128 MiB.
    pub(crate) static TEST_EDIT_LIMIT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(MAX_EDIT_FRAME) };
}

#[cfg(test)]
fn edit_limit() -> usize {
    TEST_EDIT_LIMIT.with(std::cell::Cell::get)
}

/// Why a frame was refused.
fn too_large(limit: usize) -> String {
    format!("sent a message larger than {} MiB", limit >> 20)
}

/// Reads one newline-ended frame into `frame`, refusing it once it outgrows
/// [`frame_limit`]: at most the limit and one read buffer is ever held. `Ok(None)` at
/// the end of the stream.
fn read_frame(reader: &mut impl BufRead, frame: &mut Vec<u8>) -> Result<Option<String>, String> {
    frame.clear();
    loop {
        let available = match reader.fill_buf() {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        };
        if available.is_empty() {
            return Ok(None);
        }
        let (take, ends) = match available.iter().position(|b| *b == b'\n') {
            Some(at) => (at, true),
            None => (available.len(), false),
        };
        if frame.len() + take > MAX_EDIT_FRAME {
            return Err(too_large(MAX_EDIT_FRAME));
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take + usize::from(ends));
        let limit = frame_limit(frame);
        if frame.len() > limit {
            return Err(too_large(limit));
        }
        if ends {
            return String::from_utf8(std::mem::take(frame))
                .map(Some)
                .map_err(|_| "sent a message that is not text".to_string());
        }
    }
}

/// How much a link holds waiting one way, against [`MAX_QUEUED_MESSAGES`] and
/// [`MAX_QUEUED_BYTES`].
#[derive(Default)]
struct Queue {
    messages: AtomicUsize,
    bytes: AtomicUsize,
}

impl Queue {
    /// Counts a message in; `false` when that puts the queue over a limit.
    fn push(&self, size: usize) -> bool {
        let messages = self.messages.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = self.bytes.fetch_add(size, Ordering::Relaxed) + size;
        messages <= MAX_QUEUED_MESSAGES && bytes <= MAX_QUEUED_BYTES
    }

    /// Counts a message out.
    fn pop(&self, size: usize) {
        self.messages.fetch_sub(1, Ordering::Relaxed);
        self.bytes.fetch_sub(size, Ordering::Relaxed);
    }
}

/// What every transport shares: whether the link is down, why, and its two queues.
#[derive(Clone, Default)]
struct Guard {
    closed: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<String>>>,
    incoming: Arc<Queue>,
    outgoing: Arc<Queue>,
}

impl Guard {
    /// Marks the link down for `reason`, keeping the first reason given.
    fn drop_peer(&self, reason: String) {
        if let Ok(mut held) = self.reason.lock() {
            held.get_or_insert(reason);
        }
        self.closed.store(true, Ordering::Relaxed);
    }

    /// Hands a received line to the inbox, unless that overfills it. `false` when the
    /// reader should stop.
    fn deliver(&self, inbox: &Sender<String>, line: String) -> bool {
        if !self.incoming.push(line.len()) {
            self.drop_peer(format!(
                "sent more than {MAX_QUEUED_MESSAGES} messages or {} MiB faster than they could be applied",
                MAX_QUEUED_BYTES >> 20
            ));
            return false;
        }
        inbox.send(line).is_ok()
    }

    /// Counts a line out for sending, unless the peer has fallen too far behind to take
    /// it, or it is larger than the other side accepts. `false` when it is not sent.
    fn queue_out(&self, line: &str) -> bool {
        let limit = frame_limit(line.as_bytes());
        if line.len() > limit {
            crate::complain!(
                "Not sent: a {} MiB message is larger than the {} MiB a live session takes",
                line.len() >> 20,
                limit >> 20
            );
            return false;
        }
        if !self.outgoing.push(line.len() + 1) {
            self.outgoing.pop(line.len() + 1);
            self.drop_peer(format!(
                "stopped taking messages: more than {MAX_QUEUED_MESSAGES} or {} MiB waiting",
                MAX_QUEUED_BYTES >> 20
            ));
            return false;
        }
        true
    }
}

/// The ALPN the two editors agree to speak over iroh.
const ALPN: &[u8] = b"heron-brush/collab/1";

/// Which end of the session this editor is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Bound the port or shared the endpoint id.
    Host,
    /// Connected to a host.
    Guest,
}

/// Kept only so [`Drop`] can tear the transport down: shutting the socket or dropping the
/// signal that keeps the tokio runtime alive, so the reader stops and the other end sees
/// the link close.
enum Teardown {
    Tcp(TcpStream),
    /// The browser's socket, closed when the link goes.
    #[cfg(target_arch = "wasm32")]
    WebSocket(web_sys::WebSocket),
    /// Only built by the iroh transport, so it only exists where that is compiled.
    #[cfg(feature = "iroh")]
    Iroh(tokio::sync::oneshot::Sender<()>),
}

/// A live link to the other editor.
pub struct Link {
    role: Role,
    /// The other editor's messages, as JSON lines, oldest first.
    pub inbox: Receiver<String>,
    #[cfg(not(target_arch = "wasm32"))]
    send_line: Box<dyn Fn(&str) + Send>,
    /// The browser has one thread, and its socket cannot leave it.
    #[cfg(target_arch = "wasm32")]
    send_line: Box<dyn Fn(&str)>,
    peer: String,
    guard: Guard,
    _teardown: Teardown,
}

impl Drop for Link {
    fn drop(&mut self) {
        match &self._teardown {
            Teardown::Tcp(stream) => {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
            #[cfg(target_arch = "wasm32")]
            Teardown::WebSocket(socket) => {
                let _ = socket.close();
            }
            // Teardown::Iroh drops its oneshot sender here, which the runtime thread is
            // waiting on, so there is nothing to do but let it go.
            #[cfg(feature = "iroh")]
            Teardown::Iroh(_) => {}
        }
    }
}

impl Link {
    // --- TCP -----------------------------------------------------------------

    /// Binds `addr` and blocks until the other editor connects. Run off the UI thread.
    pub fn host(addr: impl ToSocketAddrs) -> std::io::Result<Link> {
        let listener = TcpListener::bind(addr)?;
        let (stream, peer) = listener.accept()?;
        Self::from_tcp(Role::Host, stream, peer.to_string())
    }

    /// Wraps a connection a caller already accepted, for a host that keeps its listener
    /// open across rejoins rather than binding afresh for each. A connection that opens
    /// with an HTTP `GET` is a browser asking for a WebSocket, and is answered as one.
    pub fn from_accepted(stream: TcpStream) -> std::io::Result<Link> {
        let peer = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();
        Self::accept_with_timeout(stream, peer, std::time::Duration::from_secs(5))
    }

    fn accept_with_timeout(
        stream: TcpStream,
        peer: String,
        timeout: std::time::Duration,
    ) -> std::io::Result<Link> {
        stream.set_nonblocking(true)?;
        let deadline = std::time::Instant::now() + timeout;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut first = [0u8; 4];
            let mut seen = 0;
            loop {
                match stream.peek(&mut first) {
                    Ok(0) => return Err(std::io::ErrorKind::UnexpectedEof.into()),
                    Ok(n) => {
                        seen = n;
                        if first[..n] != b"GET "[..n] {
                            break;
                        }
                        if n == 4 {
                            return Self::from_websocket(stream, peer, deadline);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
                if std::time::Instant::now() >= deadline {
                    if seen > 0 {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                    // Quiet native clients may wait for the host's world snapshot.
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        stream.set_nonblocking(false)?;
        Self::from_tcp(Role::Host, stream, peer)
    }

    /// A browser's WebSocket on an accepted connection: the handshake, then one thread
    /// that reads with a short timeout and writes what is queued in between, since one
    /// socket object does both.
    #[cfg(not(target_arch = "wasm32"))]
    fn from_websocket(
        stream: TcpStream,
        peer: String,
        deadline: std::time::Instant,
    ) -> std::io::Result<Link> {
        use tungstenite::Message;
        let teardown = stream.try_clone()?;
        // The library refuses a larger message before holding it; the per-kind limit
        // is checked on what it hands over.
        let config = tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(MAX_EDIT_FRAME))
            .max_frame_size(Some(MAX_EDIT_FRAME));
        let mut handshake = tungstenite::accept_with_config(stream, Some(config));
        let mut socket = loop {
            match handshake {
                Ok(socket) => break socket,
                Err(tungstenite::HandshakeError::Failure(error)) => {
                    return Err(std::io::Error::other(error.to_string()));
                }
                Err(tungstenite::HandshakeError::Interrupted(pending)) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    handshake = pending.handshake();
                }
            }
        };
        socket.get_ref().set_nonblocking(false)?;
        socket
            .get_ref()
            .set_read_timeout(Some(std::time::Duration::from_millis(15)))?;
        socket.get_ref().set_nodelay(true).ok();
        let guard = Guard::default();
        let (in_tx, in_rx) = mpsc::channel::<String>();
        let (out_tx, out_rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name(format!("collab-websocket {peer}"))
            .spawn({
                let guard = guard.clone();
                move || {
                    'link: loop {
                        if guard.closed.load(Ordering::Relaxed) {
                            break;
                        }
                        match socket.read() {
                            Ok(Message::Text(text)) => {
                                for line in text.as_str().lines().filter(|line| !line.is_empty()) {
                                    let limit = frame_limit(line.as_bytes());
                                    if line.len() > limit {
                                        guard.drop_peer(too_large(limit));
                                        break 'link;
                                    }
                                    if !guard.deliver(&in_tx, line.to_string()) {
                                        break 'link;
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Ok(_) => {}
                            Err(tungstenite::Error::Io(error))
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) => {}
                            Err(tungstenite::Error::Capacity(_)) => {
                                guard.drop_peer(too_large(MAX_EDIT_FRAME));
                                break;
                            }
                            Err(_) => break,
                        }
                        while let Ok(line) = out_rx.try_recv() {
                            guard.outgoing.pop(line.len() + 1);
                            if socket.send(Message::text(line)).is_err() {
                                break 'link;
                            }
                        }
                    }
                    guard.closed.store(true, Ordering::Relaxed);
                }
            })?;
        let sender = guard.clone();
        Ok(Link {
            role: Role::Host,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                if sender.queue_out(line) {
                    let _ = out_tx.send(line.to_string());
                }
            }),
            peer,
            guard,
            _teardown: Teardown::Tcp(teardown),
        })
    }

    /// Joins a desktop host from the browser over a WebSocket (`ws://address:port`).
    /// Returns at once; lines sent before the socket opens wait for it.
    #[cfg(target_arch = "wasm32")]
    pub fn join_websocket(url: &str) -> std::io::Result<Link> {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast as _;
        let socket =
            web_sys::WebSocket::new(url).map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let guard = Guard::default();
        let (in_tx, in_rx) = mpsc::channel::<String>();
        let waiting: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new({
            let (guard, socket) = (guard.clone(), socket.clone());
            move |event: web_sys::MessageEvent| {
                if let Some(text) = event.data().as_string() {
                    for line in text.lines().filter(|line| !line.is_empty()) {
                        let limit = frame_limit(line.as_bytes());
                        if line.len() > limit {
                            guard.drop_peer(too_large(limit));
                        }
                        if guard.closed.load(Ordering::Relaxed)
                            || !guard.deliver(&in_tx, line.to_string())
                        {
                            let _ = socket.close();
                            return;
                        }
                    }
                }
            }
        });
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
        let on_open = Closure::<dyn FnMut()>::new({
            let (socket, waiting) = (socket.clone(), Rc::clone(&waiting));
            move || {
                for line in waiting.borrow_mut().drain(..) {
                    let _ = socket.send_with_str(&line);
                }
            }
        });
        socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();
        let on_close = Closure::<dyn FnMut()>::new({
            let closed = Arc::clone(&guard.closed);
            move || closed.store(true, Ordering::Relaxed)
        });
        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        socket.set_onerror(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();

        let sender = socket.clone();
        let limits = guard.clone();
        Ok(Link {
            role: Role::Guest,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                // The browser keeps its own outgoing buffer; only the size is checked here.
                if line.len() > frame_limit(line.as_bytes()) {
                    limits.queue_out(line);
                } else if sender.ready_state() == web_sys::WebSocket::OPEN {
                    let _ = sender.send_with_str(line);
                } else {
                    waiting.borrow_mut().push(line.to_string());
                }
            }),
            peer: url.to_string(),
            guard,
            _teardown: Teardown::WebSocket(socket),
        })
    }

    /// Connects to a TCP host.
    pub fn join(addr: impl ToSocketAddrs) -> std::io::Result<Link> {
        let stream = TcpStream::connect(addr)?;
        let peer = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();
        Self::from_tcp(Role::Guest, stream, peer)
    }

    fn from_tcp(role: Role, stream: TcpStream, peer: String) -> std::io::Result<Link> {
        stream.set_nodelay(true).ok();
        let read_half = stream.try_clone()?;
        let mut write_half = stream.try_clone()?;

        let guard = Guard::default();

        let (in_tx, in_rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name(format!("collab-read {peer}"))
            .spawn({
                let guard = guard.clone();
                let shut = stream.try_clone()?;
                move || {
                    // Framed by hand rather than with `lines`, so a message too large is
                    // refused before it is held.
                    let mut reader = BufReader::new(read_half);
                    let mut frame = Vec::new();
                    loop {
                        match read_frame(&mut reader, &mut frame) {
                            Ok(Some(line)) if line.is_empty() => {}
                            Ok(Some(line)) => {
                                if !guard.deliver(&in_tx, line) {
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(reason) => {
                                guard.drop_peer(reason);
                                break;
                            }
                        }
                    }
                    guard.closed.store(true, Ordering::Relaxed);
                    let _ = shut.shutdown(std::net::Shutdown::Both);
                }
            })?;

        let (out_tx, out_rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name(format!("collab-write {peer}"))
            .spawn({
                let guard = guard.clone();
                move || {
                    while let Ok(mut message) = out_rx.recv() {
                        message.push('\n');
                        let size = message.len();
                        let written = write_half.write_all(message.as_bytes());
                        guard.outgoing.pop(size);
                        if written.is_err() {
                            break;
                        }
                    }
                }
            })?;

        let sender = guard.clone();
        let shut = stream.try_clone()?;
        Ok(Link {
            role,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                if sender.queue_out(line) {
                    let _ = out_tx.send(line.to_string());
                } else if sender.closed.load(Ordering::Relaxed) {
                    let _ = shut.shutdown(std::net::Shutdown::Both);
                }
            }),
            peer,
            guard,
            _teardown: Teardown::Tcp(stream),
        })
    }

    // --- shared ------------------------------------------------------------

    /// Whether the other end has gone -- the read side saw the link close.
    pub fn is_closed(&self) -> bool {
        self.guard.closed.load(Ordering::Relaxed)
    }

    /// Why this end dropped the other, when it was for breaking a limit rather than the
    /// link simply closing.
    pub fn close_reason(&self) -> Option<String> {
        self.guard
            .reason
            .lock()
            .ok()
            .and_then(|reason| reason.clone())
    }

    /// This editor's role.
    pub fn role(&self) -> Role {
        self.role
    }

    /// The other editor, as text.
    pub fn peer(&self) -> &str {
        &self.peer
    }

    /// Sends one JSON line to the other editor. Silent if the link has dropped; the next
    /// empty `drain` with `is_closed` is how the caller notices.
    pub fn send(&self, json_line: &str) {
        debug_assert!(!json_line.contains('\n'), "one line per message");
        (self.send_line)(json_line);
    }

    /// Every line waiting in the inbox right now, without blocking.
    pub fn drain(&self) -> Vec<String> {
        let lines: Vec<String> = self.inbox.try_iter().collect();
        for line in &lines {
            self.guard.incoming.pop(line.len());
        }
        lines
    }
}

// --- iroh ---------------------------------------------------------------------
//
// Behind the `iroh` feature. A default build reaches the other editor over the local
// network and nothing else; iroh brings 240 packages and 9 MB of binary with it, for
// the one thing TCP cannot do -- meet across the open internet with no server to run.
#[cfg(feature = "iroh")]
mod over_the_internet {
    use super::*;

    fn io_err(error: impl std::fmt::Display) -> std::io::Error {
        std::io::Error::other(error.to_string())
    }

    /// An [`iroh::EndpointAddr`] as an opaque copy-pasteable token: base32 of its JSON.
    fn encode_ticket(addr: &iroh::EndpointAddr) -> String {
        let json = serde_json::to_vec(addr).expect("an EndpointAddr serializes");
        data_encoding::BASE32_NOPAD.encode(&json)
    }

    fn decode_ticket(ticket: &str) -> std::io::Result<iroh::EndpointAddr> {
        let json = data_encoding::BASE32_NOPAD
            .decode(ticket.trim().to_ascii_uppercase().as_bytes())
            .map_err(|_| io_err("that does not look like a session token"))?;
        serde_json::from_slice(&json).map_err(|_| io_err("that session token is malformed"))
    }

    /// Binds an iroh endpoint and, once it is reachable, returns a token for the other editor
    /// to join with, plus a receiver that yields the link when a peer dials the token.
    ///
    /// The endpoint outlives any one connection: when a link drops -- the guest closed, or
    /// the network went out from under it -- the same token keeps working, and the receiver
    /// yields a fresh link when someone dials it again. The endpoint closes when the
    /// receiver is dropped.
    pub fn host_iroh() -> std::io::Result<(String, Receiver<std::io::Result<Link>>)> {
        let (ticket_tx, ticket_rx) = mpsc::sync_channel::<std::io::Result<String>>(1);
        let (link_tx, link_rx) = mpsc::channel::<std::io::Result<Link>>();

        std::thread::Builder::new()
            .name("collab-iroh-host".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ticket_tx.send(Err(io_err(error)));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let endpoint = match iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                        .alpns(vec![ALPN.to_vec()])
                        .bind()
                        .await
                    {
                        Ok(endpoint) => endpoint,
                        Err(error) => {
                            let _ = ticket_tx.send(Err(io_err(error)));
                            return;
                        }
                    };
                    endpoint.online().await;
                    let _ = ticket_tx.send(Ok(encode_ticket(&endpoint.addr())));

                    // Accept for as long as the endpoint stands, each connection running as a
                    // task of its own: a second guest joins while the first is still linked --
                    // the hub holds them all -- and the same token stays good throughout.
                    loop {
                        let connection = match endpoint.accept().await {
                            Some(incoming) => match incoming.await {
                                Ok(connection) => connection,
                                Err(error) => {
                                    if link_tx.send(Err(io_err(error))).is_err() {
                                        break;
                                    }
                                    continue;
                                }
                            },
                            None => {
                                let _ = link_tx
                                    .send(Err(io_err("the endpoint closed before anyone joined")));
                                break;
                            }
                        };
                        let task_tx = link_tx.clone();
                        tokio::spawn(async move {
                            let _ = run_iroh_connection(Role::Host, connection, &task_tx).await;
                        });
                    }
                    endpoint.close().await;
                });
            })?;

        match ticket_rx.recv() {
            Ok(Ok(ticket)) => Ok((ticket, link_rx)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(io_err("the iroh host thread stopped before binding")),
        }
    }

    /// Dials the iroh endpoint the `ticket` names, and hands back the link once it is up.
    /// Blocks; run off the UI thread.
    pub fn join_iroh(ticket: &str) -> std::io::Result<Link> {
        let addr = decode_ticket(ticket)?;
        let (link_tx, link_rx) = mpsc::channel::<std::io::Result<Link>>();

        std::thread::Builder::new()
            .name("collab-iroh-guest".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = link_tx.send(Err(io_err(error)));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let endpoint = match iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                        .alpns(vec![ALPN.to_vec()])
                        .bind()
                        .await
                    {
                        Ok(endpoint) => endpoint,
                        Err(error) => {
                            let _ = link_tx.send(Err(io_err(error)));
                            return;
                        }
                    };
                    let connection = match endpoint.connect(addr, ALPN).await {
                        Ok(connection) => connection,
                        Err(error) => {
                            let _ = link_tx.send(Err(io_err(error)));
                            return;
                        }
                    };
                    let _ = run_iroh_connection(Role::Guest, connection, &link_tx).await;
                    endpoint.close().await;
                });
            })?;

        match link_rx.recv() {
            Ok(result) => result,
            Err(_) => Err(io_err("the iroh guest thread stopped before connecting")),
        }
    }

    /// Opens the bi stream, builds the [`Link`], hands it back through `link_tx`, and runs
    /// until the `Link` is dropped or the connection ends. `Err` means no one is listening
    /// on `link_tx` any more, so a host's accept loop should stop.
    async fn run_iroh_connection(
        role: Role,
        connection: iroh::endpoint::Connection,
        link_tx: &Sender<std::io::Result<Link>>,
    ) -> Result<(), ()> {
        let peer = connection.remote_id().to_string();
        let streams = match role {
            Role::Host => connection.accept_bi().await,
            Role::Guest => connection.open_bi().await,
        };
        let (mut send, recv) = match streams {
            Ok(streams) => streams,
            Err(error) => {
                return link_tx.send(Err(io_err(error))).map_err(|_| ());
            }
        };

        // The side that opened the stream primes it with a blank line, since QUIC does not
        // let the other side `accept_bi` until the first byte arrives. The reader skips it.
        if role == Role::Guest {
            use tokio::io::AsyncWriteExt;
            if send.write_all(b"\n").await.is_err() {
                return link_tx
                    .send(Err(io_err("the link closed before it opened")))
                    .map_err(|_| ());
            }
        }

        let guard = Guard::default();

        let (in_tx, in_rx) = mpsc::channel::<String>();
        let reader_guard = guard.clone();
        let reader = tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            // Framed by hand, as the TCP reader is, so a message too large is refused
            // before it is held.
            let mut reader = tokio::io::BufReader::new(recv);
            let mut frame = Vec::new();
            'link: loop {
                let available = match reader.fill_buf().await {
                    Ok(bytes) if bytes.is_empty() => break,
                    Ok(bytes) => bytes,
                    Err(_) => break,
                };
                let (take, ends) = match available.iter().position(|b| *b == b'\n') {
                    Some(at) => (at, true),
                    None => (available.len(), false),
                };
                if frame.len() + take > MAX_EDIT_FRAME {
                    reader_guard.drop_peer(too_large(MAX_EDIT_FRAME));
                    break;
                }
                frame.extend_from_slice(&available[..take]);
                reader.consume(take + usize::from(ends));
                let limit = frame_limit(&frame);
                if frame.len() > limit {
                    reader_guard.drop_peer(too_large(limit));
                    break;
                }
                if ends {
                    let Ok(line) = String::from_utf8(std::mem::take(&mut frame)) else {
                        reader_guard.drop_peer("sent a message that is not text".into());
                        break 'link;
                    };
                    if !line.is_empty() && !reader_guard.deliver(&in_tx, line) {
                        break;
                    }
                }
            }
            reader_guard.closed.store(true, Ordering::Relaxed);
        });

        let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let writer_guard = guard.clone();
        let writer = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(mut message) = out_rx.recv().await {
                message.push('\n');
                let written = send.write_all(message.as_bytes()).await;
                writer_guard.outgoing.pop(message.len());
                if written.is_err() || writer_guard.closed.load(Ordering::Relaxed) {
                    break;
                }
            }
            let _ = send.finish();
        });

        let (teardown_tx, teardown_rx) = tokio::sync::oneshot::channel::<()>();
        let sender = guard.clone();
        let link = Link {
            role,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                if sender.queue_out(line) {
                    let _ = out_tx.send(line.to_string());
                }
            }),
            peer,
            guard,
            _teardown: Teardown::Iroh(teardown_tx),
        };
        if link_tx.send(Ok(link)).is_err() {
            return Err(()); // the caller gave up before the link was ready
        }

        // Stay alive until the Link is dropped (teardown_rx resolves with an error) or the
        // connection ends on its own.
        tokio::select! {
            _ = teardown_rx => {}
            _ = reader => {}
            _ = writer => {}
            _ = connection.closed() => {}
        }
        connection.close(0u32.into(), b"bye");
        Ok(())
    }
}

#[cfg(feature = "iroh")]
pub use over_the_internet::{host_iroh, join_iroh};

#[cfg(test)]
mod tests {
    use super::*;
    use web_time::{Duration, Instant};

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn recv_within(link: &Link, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(line) = link.inbox.try_recv() {
                return Some(line);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn linked_pair() -> (Link, Link) {
        let port = free_port();
        let addr = format!("127.0.0.1:{port}");
        let host_thread = std::thread::spawn(move || Link::host(&addr).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Link::join(format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();
        (host, guest)
    }

    /// A frame of `size` bytes, a message tagged `tag`.
    fn frame(tag: &str, size: usize) -> Vec<u8> {
        let mut out = format!("{{\"{tag}\":\"").into_bytes();
        out.resize(size - 2, b'x');
        out.extend_from_slice(b"\"}");
        out
    }

    #[test]
    fn frames_up_to_their_limit_are_read_and_larger_ones_refused() {
        for (tag, limit) in [
            ("Hello", MAX_FRAME),
            ("Selection", MAX_FRAME),
            ("World", MAX_WORLD_FRAME),
            ("Edit", MAX_EDIT_FRAME),
            ("TrackedEdit", MAX_EDIT_FRAME),
            ("Relayed", MAX_EDIT_FRAME),
        ] {
            let world = tag;
            for (size, fits) in [(limit, true), (limit + 1, false)] {
                let mut bytes = frame(tag, size);
                bytes.push(b'\n');
                let mut reader = std::io::BufReader::new(std::io::Cursor::new(bytes));
                let mut held = Vec::new();
                let read = read_frame(&mut reader, &mut held);
                assert_eq!(read.is_ok(), fits, "world {world}, {size} bytes");
                if let Ok(Some(line)) = read {
                    assert_eq!(line.len(), size);
                }
                // A refused frame is never held past its limit and one read buffer.
                assert!(held.len() <= limit + 8192);
            }
        }
    }

    #[test]
    fn a_peer_that_floods_the_inbox_is_dropped_with_a_reason() {
        // A raw socket, so nothing on the sending side holds the flood back.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut flood = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let host = Link::from_tcp(Role::Host, stream, "flood".into()).unwrap();
        for index in 0..MAX_QUEUED_MESSAGES + 50 {
            if flood
                .write_all(format!("{{\"Hello\":\"{index}\"}}\n").as_bytes())
                .is_err()
            {
                break;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while !host.is_closed() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(host.is_closed());
        assert!(host.close_reason().unwrap().contains("256 messages"));
    }

    #[test]
    fn a_session_that_keeps_up_stays_connected() {
        let (host, guest) = linked_pair();
        // Twice as many messages as the queue holds, taken as they come.
        let total = MAX_QUEUED_MESSAGES * 2;
        let mut received = 0;
        for index in 0..total {
            guest.send(&format!("{{\"Hello\":\"{index}\"}}"));
            if index % 32 == 31 {
                let deadline = Instant::now() + Duration::from_secs(5);
                while received <= index && Instant::now() < deadline {
                    received += host.drain().len();
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
        assert_eq!(received, total);
        assert!(!host.is_closed() && !guest.is_closed());
        assert!(host.close_reason().is_none() && guest.close_reason().is_none());
    }

    #[test]
    fn incomplete_websocket_handshake_expires() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client.write_all(b"GET ").unwrap();
        let (stream, _) = listener.accept().unwrap();
        let started = Instant::now();
        let error = Link::accept_with_timeout(stream, "test".into(), Duration::from_millis(100))
            .err()
            .expect("incomplete handshake must expire");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn fragmented_websocket_prefix_is_recognized() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        client.write_all(b"G").unwrap();
        let (stream, _) = listener.accept().unwrap();
        let server = std::thread::spawn(move || Link::from_accepted(stream));
        std::thread::sleep(Duration::from_millis(30));
        client.write_all(b"ET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").unwrap();
        let link = server.join().unwrap().unwrap();
        let mut response = String::new();
        BufReader::new(&client).read_line(&mut response).unwrap();
        assert!(response.contains("101"));
        drop(link);
    }

    #[test]
    fn a_message_travels_both_ways() {
        let (host, guest) = linked_pair();
        assert_eq!(host.role(), Role::Host);
        assert_eq!(guest.role(), Role::Guest);

        host.send(r#"{"hello":"from host"}"#);
        assert_eq!(
            recv_within(&guest, Duration::from_secs(2)).as_deref(),
            Some(r#"{"hello":"from host"}"#)
        );

        guest.send(r#"{"hello":"from guest"}"#);
        assert_eq!(
            recv_within(&host, Duration::from_secs(2)).as_deref(),
            Some(r#"{"hello":"from guest"}"#)
        );
    }

    #[test]
    fn messages_keep_their_order() {
        let (host, guest) = linked_pair();
        for i in 0..20 {
            host.send(&format!("{{\"n\":{i}}}"));
        }
        let mut seen = Vec::new();
        while seen.len() < 20 {
            match recv_within(&guest, Duration::from_secs(2)) {
                Some(line) => seen.push(line),
                None => break,
            }
        }
        assert_eq!(seen.len(), 20);
        for (i, line) in seen.iter().enumerate() {
            assert_eq!(line, &format!("{{\"n\":{i}}}"));
        }
    }

    #[test]
    fn a_dropped_peer_just_stops_delivering() {
        let (host, guest) = linked_pair();
        drop(guest);
        host.send(r#"{"still":"here"}"#);
        std::thread::sleep(Duration::from_millis(50));
        assert!(host.drain().is_empty());
    }

    #[test]
    fn a_real_edit_survives_the_round_trip() {
        use crate::collab::wire::WireEdit;
        use crate::history::Edit;

        let (host, guest) = linked_pair();
        let edit = Edit::Added {
            layer: 3,
            nodes: vec![],
        };
        host.send(&WireEdit::from_edit(&edit).to_json());

        let line = recv_within(&guest, Duration::from_secs(2)).expect("it arrives");
        let bounds = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
        let back = WireEdit::from_json(&line)
            .unwrap()
            .into_edit(&bounds)
            .unwrap();
        assert!(matches!(back, Edit::Added { layer: 3, .. }));
    }

    #[test]
    #[ignore = "reaches n0's relay servers over the network"]
    #[cfg(feature = "iroh")]
    fn two_editors_meet_over_iroh() {
        let (ticket, host_link_rx) = host_iroh().expect("the host endpoint binds");
        assert!(!ticket.is_empty());

        let guest_thread = std::thread::spawn(move || join_iroh(&ticket).unwrap());

        let host = host_link_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("no one joined over iroh")
            .expect("the guest connected");
        let guest = guest_thread.join().unwrap();

        host.send(r#"{"over":"iroh"}"#);
        assert_eq!(
            recv_within(&guest, Duration::from_secs(5)).as_deref(),
            Some(r#"{"over":"iroh"}"#)
        );
    }
}
