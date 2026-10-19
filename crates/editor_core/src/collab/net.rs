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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

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
    closed: Arc<AtomicBool>,
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
        let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
        // The listener may be non-blocking; the link's threads want to block.
        stream.set_nonblocking(false)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
            let mut first = [0u8; 4];
            let seen = stream.peek(&mut first).unwrap_or(0);
            stream.set_read_timeout(None)?;
            if seen == 4 && &first == b"GET " {
                return Self::from_websocket(stream, peer);
            }
        }
        Self::from_tcp(Role::Host, stream, peer)
    }

    /// A browser's WebSocket on an accepted connection: the handshake, then one thread
    /// that reads with a short timeout and writes what is queued in between, since one
    /// socket object does both.
    #[cfg(not(target_arch = "wasm32"))]
    fn from_websocket(stream: TcpStream, peer: String) -> std::io::Result<Link> {
        use tungstenite::Message;
        let teardown = stream.try_clone()?;
        let mut socket = tungstenite::accept(stream).map_err(|e| std::io::Error::other(e.to_string()))?;
        socket.get_ref().set_read_timeout(Some(std::time::Duration::from_millis(15)))?;
        socket.get_ref().set_nodelay(true).ok();
        let closed = Arc::new(AtomicBool::new(false));
        let (in_tx, in_rx) = mpsc::channel::<String>();
        let (out_tx, out_rx) = mpsc::channel::<String>();
        std::thread::Builder::new().name(format!("collab-websocket {peer}")).spawn({
            let closed = Arc::clone(&closed);
            move || {
                'link: loop {
                    match socket.read() {
                        Ok(Message::Text(text)) => {
                            for line in text.as_str().lines().filter(|line| !line.is_empty()) {
                                if in_tx.send(line.to_string()).is_err() {
                                    break 'link;
                                }
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(error))
                            if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                        Err(_) => break,
                    }
                    while let Ok(line) = out_rx.try_recv() {
                        if socket.send(Message::text(line)).is_err() {
                            break 'link;
                        }
                    }
                }
                closed.store(true, Ordering::Relaxed);
            }
        })?;
        Ok(Link {
            role: Role::Host,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                let _ = out_tx.send(line.to_string());
            }),
            peer,
            closed,
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
        let socket = web_sys::WebSocket::new(url).map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let closed = Arc::new(AtomicBool::new(false));
        let (in_tx, in_rx) = mpsc::channel::<String>();
        let waiting: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if let Some(text) = event.data().as_string() {
                for line in text.lines().filter(|line| !line.is_empty()) {
                    let _ = in_tx.send(line.to_string());
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
            let closed = Arc::clone(&closed);
            move || closed.store(true, Ordering::Relaxed)
        });
        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        socket.set_onerror(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();

        let sender = socket.clone();
        Ok(Link {
            role: Role::Guest,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                if sender.ready_state() == web_sys::WebSocket::OPEN {
                    let _ = sender.send_with_str(line);
                } else {
                    waiting.borrow_mut().push(line.to_string());
                }
            }),
            peer: url.to_string(),
            closed,
            _teardown: Teardown::WebSocket(socket),
        })
    }

    /// Connects to a TCP host.
    pub fn join(addr: impl ToSocketAddrs) -> std::io::Result<Link> {
        let stream = TcpStream::connect(addr)?;
        let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
        Self::from_tcp(Role::Guest, stream, peer)
    }

    fn from_tcp(role: Role, stream: TcpStream, peer: String) -> std::io::Result<Link> {
        stream.set_nodelay(true).ok();
        let read_half = stream.try_clone()?;
        let mut write_half = stream.try_clone()?;

        let closed = Arc::new(AtomicBool::new(false));

        let (in_tx, in_rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name(format!("collab-read {peer}"))
            .spawn({
                let closed = Arc::clone(&closed);
                move || {
                    let mut lines = BufReader::new(read_half).lines();
                    while let Some(Ok(line)) = lines.next() {
                        if line.is_empty() {
                            continue;
                        }
                        if in_tx.send(line).is_err() {
                            break;
                        }
                    }
                    closed.store(true, Ordering::Relaxed);
                }
            })?;

        let (out_tx, out_rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name(format!("collab-write {peer}"))
            .spawn(move || {
                while let Ok(mut message) = out_rx.recv() {
                    message.push('\n');
                    if write_half.write_all(message.as_bytes()).is_err() {
                        break;
                    }
                }
            })?;

        Ok(Link {
            role,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                let _ = out_tx.send(line.to_string());
            }),
            peer,
            closed,
            _teardown: Teardown::Tcp(stream),
        })
    }

    // --- shared ------------------------------------------------------------

    /// Whether the other end has gone -- the read side saw the link close.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
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
        self.inbox.try_iter().collect()
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

        std::thread::Builder::new().name("collab-iroh-host".into()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
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
                            let _ =
                                link_tx.send(Err(io_err("the endpoint closed before anyone joined")));
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

        std::thread::Builder::new().name("collab-iroh-guest".into()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
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

        let closed = Arc::new(AtomicBool::new(false));

        let (in_tx, in_rx) = mpsc::channel::<String>();
        let reader_closed = Arc::clone(&closed);
        let reader = tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(recv).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    continue;
                }
                if in_tx.send(line).is_err() {
                    break;
                }
            }
            reader_closed.store(true, Ordering::Relaxed);
        });

        let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let writer = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(mut message) = out_rx.recv().await {
                message.push('\n');
                if send.write_all(message.as_bytes()).await.is_err() {
                    break;
                }
            }
            let _ = send.finish();
        });

        let (teardown_tx, teardown_rx) = tokio::sync::oneshot::channel::<()>();
        let link = Link {
            role,
            inbox: in_rx,
            send_line: Box::new(move |line| {
                let _ = out_tx.send(line.to_string());
            }),
            peer,
            closed,
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
        TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
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
        let edit = Edit::Added { layer: 3, nodes: vec![] };
        host.send(&WireEdit::from_edit(&edit).to_json());

        let line = recv_within(&guest, Duration::from_secs(2)).expect("it arrives");
        let bounds = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
        let back = WireEdit::from_json(&line).unwrap().into_edit(&bounds).unwrap();
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
