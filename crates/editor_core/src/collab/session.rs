//! A live editing session: the [`super::net::Link`]s plus the handshake and the JSON envelope.
//!
//! `main.rs` holds an `Option<Session>`. It calls [`Session::send_edit`] from
//! `Document`'s `on_edit` hook, and [`Session::poll`] from the UI thread between frames,
//! applying whatever comes back. Nothing here blocks except [`Session::host`], which the
//! caller runs on a thread.
//!
//! With more than two people the host is the hub: every guest talks to the host, and
//! the host hands each message on to the rest, wrapped in `Relayed` with who it came
//! from. Peer 0 is the host itself; guests are numbered as they join. Edits are echoed to their sender too, so all guests
//! reconcile optimistic changes in the same host-defined order.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use geometry_math::bbox::Bbox3d;
use map_model::node::{NodeId, World};

use super::net::{Link, Role};
use super::wire::{WireEdit, WireMessage, WireWorld};
use crate::history::Edit;

/// The host's own id in relayed messages.
pub const HOST_PEER: u32 = 0;
static NEXT_EDIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Something another editor did, ready to apply.
pub enum SessionEvent {
    /// Adopt this world wholesale -- sent to a guest as it joins.
    AdoptWorld(World),
    /// Apply this edit.
    ApplyEdit(Edit),
    /// A participant's selection changed; show it.
    PeerSelection {
        /// Who.
        peer: u32,
        /// What they have selected.
        ids: HashSet<NodeId>,
    },
    /// A participant's display name arrived or changed.
    PeerName {
        /// Who.
        peer: u32,
        /// What to call them.
        name: String,
    },
    /// Where a participant's 3D camera stands and looks; draw their cursor there.
    PeerCamera {
        /// Who.
        peer: u32,
        /// Where they stand, in map units.
        position: geometry_math::vec::Vec3d,
        /// The way they look, unit length.
        direction: geometry_math::vec::Vec3d,
    },
    /// A line came through that could not be understood; kept so the caller can log it.
    Undecodable(String),
    /// One participant is gone. The session goes on for everyone else.
    PeerLeft {
        /// Who.
        peer: u32,
        /// What they were called, if they ever said.
        name: Option<String>,
    },
    /// This editor's own link died: a guest lost its host. The session is over until a
    /// rejoin brings a new one.
    ConnectionLost,
}

/// Where a session came from: what there is to dial again when the link drops.
#[derive(Debug, Clone)]
pub enum Origin {
    /// This editor bound a TCP port; hosting again means listening on it again.
    TcpHost(u16),
    /// This editor joined a TCP host at this address.
    TcpGuest(String),
    /// This editor hosts over iroh. The endpoint outlives any one link, so there is
    /// nothing to redial: the receiver `host_iroh` returned keeps yielding sessions as
    /// guests dial the same token.
    IrohHost,
    /// This editor joined the iroh session this token names.
    IrohGuest(String),
}

/// A pending connection or a live host's accept queue. Dropping it stops the
/// background accept loop, releasing the TCP port even if nobody connects again.
pub struct SessionInbox {
    receiver: std::sync::mpsc::Receiver<std::io::Result<Session>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl From<std::sync::mpsc::Receiver<std::io::Result<Session>>> for SessionInbox {
    fn from(receiver: std::sync::mpsc::Receiver<std::io::Result<Session>>) -> Self {
        Self {
            receiver,
            stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}
impl std::ops::Deref for SessionInbox {
    type Target = std::sync::mpsc::Receiver<std::io::Result<Session>>;
    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}
impl Drop for SessionInbox {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// One participant's link, as the host keeps it. A guest has exactly one of these --
/// the host's -- and its id goes unused.
struct PeerLink {
    id: u32,
    link: Link,
}

/// This machine's address on the local network, for a guest to dial.
///
/// Found by asking the routing table where a packet to a public address would leave
/// from, which needs no packet sent -- a UDP socket is connectionless, so `connect`
/// only picks the route. Falls back to naming the port alone, which still tells a
/// person on the same machine what to type.
fn local_address() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("203.0.113.1:9")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "this-machine".to_string())
}

/// The one answer a build without the `iroh` feature has for the internet route.
#[cfg(not(feature = "iroh"))]
fn without_iroh() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this build reaches other editors on the local network only; \
         a build with the `iroh` feature also reaches them over the internet",
    )
}

/// A live session: this editor's links to the others.
///
/// A guest holds one link, to the host. A host holds one per guest, added by
/// [`Session::add_peer`] as they dial in, and relays between them.
pub struct Session {
    role: Role,
    origin: Origin,
    links: RefCell<Vec<PeerLink>>,
    /// The id the next joiner gets. Host only; ids start at 1, the host being 0.
    next_peer: Cell<u32>,
    /// What each participant asked to be called.
    names: RefCell<HashMap<u32, String>>,
    /// Set once `ConnectionLost` has been reported, so it is reported only once.
    lost: Cell<bool>,
    cameras: RefCell<HashMap<u32, WireMessage>>,
    pending_edits: RefCell<Vec<(String, Edit)>>,
}

impl Session {
    /// Hosts on `0.0.0.0:port` over TCP and blocks until a guest joins. Run on a thread.
    pub fn host(port: u16) -> std::io::Result<Session> {
        Ok(Session::new(
            Link::host(("0.0.0.0", port))?,
            Origin::TcpHost(port),
        ))
    }

    /// Joins a TCP host at `addr` (`"host:port"`). The web build, which cannot open TCP,
    /// joins the same host over a WebSocket on that port.
    pub fn join(addr: &str) -> std::io::Result<Session> {
        #[cfg(target_arch = "wasm32")]
        let link = Link::join_websocket(&if addr.contains("://") {
            addr.to_string()
        } else {
            format!("ws://{addr}")
        })?;
        #[cfg(not(target_arch = "wasm32"))]
        let link = Link::join(addr)?;
        Ok(Session::new(link, Origin::TcpGuest(addr.to_string())))
    }

    /// Binds an iroh endpoint. Returns the token to share and a receiver that yields a
    /// session each time another editor dials the token in -- the first join, and every
    /// rejoin after a drop, come through the same receiver.
    #[cfg(feature = "iroh")]
    pub fn host_iroh() -> std::io::Result<(String, SessionInbox)> {
        let (ticket, link_rx) = super::net::host_iroh()?;
        let (session_tx, session_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(result) = link_rx.recv() {
                let result = result.map(|link| Session::new(link, Origin::IrohHost));
                if session_tx.send(result).is_err() {
                    return;
                }
            }
        });
        Ok((ticket, session_rx.into()))
    }

    /// Joins an iroh session the `ticket` names. Blocks; run on a thread.
    #[cfg(feature = "iroh")]
    pub fn join_iroh(ticket: &str) -> std::io::Result<Session> {
        Ok(Session::new(
            super::net::join_iroh(ticket)?,
            Origin::IrohGuest(ticket.to_string()),
        ))
    }

    /// Hosts on the local network and hands back the `address:port` a guest pastes,
    /// alongside a receiver that yields the session once one connects.
    ///
    /// The same shape as [`Session::host_iroh`], so the window's Host action is one call
    /// either way -- what differs is how far the token reaches: this one to the machines
    /// that can see this one, iroh's to anywhere.
    pub fn host_on_this_network(port: u16) -> std::io::Result<(String, SessionInbox)> {
        // A page cannot listen for connections: the browser joins, a desktop editor hosts.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = port;
            Err(std::io::Error::other(
                "hosting a live session needs the desktop app; join one from the browser",
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Bound here rather than on the thread so a port already in use is reported now,
            // while there is still a caller to report it to.
            let listener = std::net::TcpListener::bind(("0.0.0.0", port))?;
            let port = listener.local_addr()?.port();
            listener.set_nonblocking(true)?;
            let token = format!("{}:{port}", local_address());
            let (session_tx, session_rx) = std::sync::mpsc::channel();
            let inbox = SessionInbox::from(session_rx);
            let stop = inbox.stop.clone();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let result = Link::from_accepted(stream)
                                .map(|link| Session::new(link, Origin::TcpHost(port)));
                            if session_tx.send(result).is_err() {
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(web_time::Duration::from_millis(10));
                        }
                        Err(error) => {
                            let _ = session_tx.send(Err(error));
                            break;
                        }
                    }
                }
            });
            Ok((token, inbox))
        }
    }

    /// What a build without the `iroh` feature answers with. The local-network route is
    /// compiled in and works ([`Session::host_on_this_network`]); this one is not here,
    /// and saying so beats a caller wondering which build it is talking to.
    #[cfg(not(feature = "iroh"))]
    pub fn host_iroh() -> std::io::Result<(String, SessionInbox)> {
        Err(without_iroh())
    }

    /// See [`Session::host_iroh`]'s note for a build without the feature.
    #[cfg(not(feature = "iroh"))]
    pub fn join_iroh(_ticket: &str) -> std::io::Result<Session> {
        Err(without_iroh())
    }

    /// Dials `origin` again, blocking: what a rejoin thread runs after the link
    /// dropped. A TCP host listens for the guest to come back; a guest dials the same
    /// address or token. An iroh host has nothing to dial -- its receiver keeps
    /// yielding sessions on its own -- so this refuses.
    pub fn redial(origin: &Origin) -> std::io::Result<Session> {
        match origin {
            Origin::TcpHost(port) => Session::host(*port),
            Origin::TcpGuest(addr) => Session::join(addr),
            Origin::IrohGuest(ticket) => Session::join_iroh(ticket),
            Origin::IrohHost => Err(std::io::Error::other(
                "an iroh host keeps its endpoint; the next guest joins by the same token",
            )),
        }
    }

    /// Where this session came from.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    fn new(link: Link, origin: Origin) -> Session {
        let role = link.role();
        Session {
            role,
            origin,
            links: RefCell::new(vec![PeerLink { id: 1, link }]),
            next_peer: Cell::new(2),
            names: RefCell::new(HashMap::new()),
            lost: Cell::new(false),
            cameras: RefCell::new(HashMap::new()),
            pending_edits: RefCell::new(Vec::new()),
        }
    }

    /// Takes another joiner into this session as a new participant, returning the id
    /// they got. The host calls this with the session a fresh link arrived wrapped in.
    pub fn add_peer(&self, joined: Session) -> u32 {
        let id = self.next_peer.get();
        self.next_peer.set(id + 1);
        let mut incoming = joined.links.into_inner();
        let link = incoming.remove(0).link;
        // The people already in introduced themselves long ago, so the newcomer is
        // handed their names the way live ones would arrive: relayed by the host.
        for (peer, name) in self.names.borrow().iter() {
            link.send(
                &WireMessage::Relayed {
                    from: *peer,
                    message: Box::new(WireMessage::Hello(name.clone())),
                }
                .to_json(),
            );
        }
        // A newcomer must see stationary cameras without waiting for movement.
        for (peer, camera) in self.cameras.borrow().iter() {
            link.send(
                &WireMessage::Relayed {
                    from: *peer,
                    message: Box::new(camera.clone()),
                }
                .to_json(),
            );
        }
        self.links.borrow_mut().push(PeerLink { id, link });
        id
    }

    /// How many others this editor is linked to right now.
    pub fn participants(&self) -> usize {
        self.links.borrow().len()
    }

    /// Sends one JSON line to every link.
    fn broadcast(&self, json: &str) {
        for peer in self.links.borrow().iter() {
            peer.link.send(json);
        }
    }

    /// Tells the others the name to show for this editor.
    pub fn send_name(&self, name: &str) {
        self.broadcast(&WireMessage::Hello(name.to_string()).to_json());
    }

    /// What a participant asked to be called, if they ever said.
    pub fn name_of(&self, peer: u32) -> Option<String> {
        self.names.borrow().get(&peer).cloned()
    }

    /// Whether this editor hosts the session.
    pub fn is_host(&self) -> bool {
        self.role == Role::Host
    }

    /// The other end, as text: the host's address for a guest, the latest joiner's for
    /// a host.
    pub fn peer(&self) -> String {
        self.links
            .borrow()
            .last()
            .map(|peer| peer.link.peer().to_string())
            .unwrap_or_else(|| "no one".to_string())
    }

    /// Sends the whole world to everyone. The host does this as the first guest joins.
    pub fn send_world(&self, world: &World) {
        self.broadcast(&WireMessage::World(WireWorld::from_world(world)).to_json());
    }

    /// Sends the whole world to one participant: what a joiner arriving mid-session
    /// gets, without disturbing the people already in.
    pub fn send_world_to(&self, peer: u32, world: &World) {
        let json = WireMessage::World(WireWorld::from_world(world)).to_json();
        for peer_link in self.links.borrow().iter() {
            if peer_link.id == peer {
                peer_link.link.send(&json);
            }
        }
    }

    /// Sends one edit to everyone this editor is linked to.
    pub fn send_edit(&self, edit: &Edit) {
        let message = if self.is_host() {
            WireMessage::Edit(WireEdit::from_edit(edit))
        } else {
            let id = format!(
                "{:08x}-{}",
                map_model::ids::session_prefix(),
                NEXT_EDIT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            self.pending_edits
                .borrow_mut()
                .push((id.clone(), edit.clone()));
            WireMessage::TrackedEdit {
                id,
                edit: WireEdit::from_edit(edit),
            }
        };
        self.broadcast(&message.to_json());
    }

    /// Edits the host has not acknowledged, to replay after adopting its world
    /// on reconnect. A repeated add is safe because remote application is idempotent.
    pub fn take_unacknowledged(&self) -> Vec<Edit> {
        self.pending_edits
            .borrow_mut()
            .drain(..)
            .map(|(_, edit)| edit)
            .collect()
    }

    /// Tells the others what this editor has selected.
    pub fn send_selection(&self, ids: &HashSet<NodeId>) {
        let nodes = ids.iter().map(|id| id.0).collect();
        self.broadcast(&WireMessage::Selection(nodes).to_json());
    }

    /// Tells the others where this editor's 3D camera stands and looks.
    pub fn send_camera(
        &self,
        position: geometry_math::vec::Vec3d,
        direction: geometry_math::vec::Vec3d,
    ) {
        let message = WireMessage::Camera {
            position: [position.x(), position.y(), position.z()],
            direction: [direction.x(), direction.y(), direction.z()],
        };
        if self.is_host() {
            self.cameras.borrow_mut().insert(HOST_PEER, message.clone());
        }
        self.broadcast(&message.to_json());
    }

    /// Everything the others have sent since the last poll, in order. Does not block.
    ///
    /// On the host this is also the relay: each message a guest sent is handed on to
    /// every other guest, wrapped in `Relayed` with the sender's id, and a guest that
    /// dropped is announced with `Left`.
    pub fn poll(&self, bounds: &Bbox3d) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        let mut gone: Vec<u32> = Vec::new();
        {
            let links = self.links.borrow();
            for peer_link in links.iter() {
                for line in peer_link.link.drain() {
                    match WireMessage::from_json(&line) {
                        Ok(message) => {
                            if self.role == Role::Host {
                                // Only the host may announce worlds, peer departures
                                // or relay envelopes. Validate edits before forwarding.
                                let valid = match &message {
                                    WireMessage::Edit(edit)
                                    | WireMessage::TrackedEdit { edit, .. } => {
                                        edit.clone().into_edit(bounds).is_ok()
                                    }
                                    WireMessage::Hello(_)
                                    | WireMessage::Selection(_)
                                    | WireMessage::Camera { .. } => true,
                                    _ => false,
                                };
                                if !valid {
                                    events.push(SessionEvent::Undecodable(
                                        "invalid guest message".into(),
                                    ));
                                    continue;
                                }
                                let echo_edit = matches!(
                                    &message,
                                    WireMessage::Edit(_) | WireMessage::TrackedEdit { .. }
                                );
                                let relayed = WireMessage::Relayed {
                                    from: peer_link.id,
                                    message: Box::new(message.clone()),
                                }
                                .to_json();
                                for other in links
                                    .iter()
                                    .filter(|other| echo_edit || other.id != peer_link.id)
                                {
                                    other.link.send(&relayed);
                                }
                            }
                            // A guest's unwrapped messages come straight from the host.
                            let from = if self.role == Role::Host {
                                peer_link.id
                            } else {
                                HOST_PEER
                            };
                            self.absorb(from, message, bounds, &mut events);
                        }
                        Err(e) => events.push(SessionEvent::Undecodable(e.to_string())),
                    }
                }
                if peer_link.link.is_closed() {
                    gone.push(peer_link.id);
                }
            }
        }
        if !gone.is_empty() {
            let mut links = self.links.borrow_mut();
            links.retain(|peer_link| !gone.contains(&peer_link.id));
            for peer in gone {
                if self.role == Role::Host {
                    let notice = WireMessage::Left { peer }.to_json();
                    for other in links.iter() {
                        other.link.send(&notice);
                    }
                    self.cameras.borrow_mut().remove(&peer);
                    let name = self.names.borrow_mut().remove(&peer);
                    events.push(SessionEvent::PeerLeft { peer, name });
                } else if !self.lost.replace(true) {
                    events.push(SessionEvent::ConnectionLost);
                }
            }
        }
        events
    }

    /// Turns one decoded message from `from` into events, unwrapping the host's relay
    /// envelope on the way.
    fn absorb(
        &self,
        from: u32,
        message: WireMessage,
        bounds: &Bbox3d,
        events: &mut Vec<SessionEvent>,
    ) {
        match message {
            WireMessage::World(world) => match world.into_world(bounds) {
                Ok(world) => events.push(SessionEvent::AdoptWorld(world)),
                Err(e) => events.push(SessionEvent::Undecodable(e.to_string())),
            },
            WireMessage::Edit(edit) => match edit.into_edit(bounds) {
                Ok(edit) => events.push(SessionEvent::ApplyEdit(edit)),
                Err(e) => events.push(SessionEvent::Undecodable(e.to_string())),
            },
            WireMessage::TrackedEdit { id, edit } => match edit.into_edit(bounds) {
                Ok(edit) => {
                    self.pending_edits
                        .borrow_mut()
                        .retain(|(pending, _)| pending != &id);
                    events.push(SessionEvent::ApplyEdit(edit));
                }
                Err(e) => events.push(SessionEvent::Undecodable(e.to_string())),
            },
            WireMessage::Selection(nodes) => {
                events.push(SessionEvent::PeerSelection {
                    peer: from,
                    ids: nodes.into_iter().map(NodeId).collect(),
                });
            }
            WireMessage::Hello(name) => {
                self.names.borrow_mut().insert(from, name.clone());
                events.push(SessionEvent::PeerName { peer: from, name });
            }
            WireMessage::Camera {
                position,
                direction,
            } => {
                let valid = position
                    .iter()
                    .chain(direction.iter())
                    .all(|v| v.is_finite())
                    && (1e-12..=4.0).contains(&direction.iter().map(|v| v * v).sum::<f64>());
                if !valid {
                    return;
                }
                if self.is_host() {
                    self.cameras.borrow_mut().insert(
                        from,
                        WireMessage::Camera {
                            position,
                            direction,
                        },
                    );
                }
                events.push(SessionEvent::PeerCamera {
                    peer: from,
                    position: geometry_math::vec::Vec3d::new(position[0], position[1], position[2]),
                    direction: geometry_math::vec::Vec3d::new(
                        direction[0],
                        direction[1],
                        direction[2],
                    ),
                });
            }
            WireMessage::Relayed {
                from: origin,
                message,
            } => {
                self.absorb(origin, *message, bounds, events);
            }
            WireMessage::Left { peer } => {
                let name = self.names.borrow_mut().remove(&peer);
                events.push(SessionEvent::PeerLeft { peer, name });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::io::map_reader::read_map;
    use map_model::map_format::MapFormat;
    use map_model::node::{build_tree, flatten_tree};
    use std::net::TcpListener;
    use web_time::{Duration, Instant};

    fn bounds() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    const A_BOX: &str = "\
{
\"classname\" \"worldspawn\"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
";

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn poll_within(session: &Session, timeout: Duration) -> Vec<SessionEvent> {
        let deadline = Instant::now() + timeout;
        loop {
            let events = session.poll(&bounds());
            if !events.is_empty() || Instant::now() >= deadline {
                return events;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_guest_adopts_the_host_world_then_receives_edits() {
        let port = free_port();
        let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();

        assert!(host.is_host());
        assert!(!guest.is_host());

        let host_world = build_tree(&read_map(A_BOX, None).unwrap(), &bounds()).world;
        host.send_world(&host_world);

        let events = poll_within(&guest, Duration::from_secs(2));
        let SessionEvent::AdoptWorld(adopted) = &events[0] else {
            panic!("expected the world first, got something else");
        };
        assert_eq!(
            flatten_map(&host_world),
            flatten_map(adopted),
            "the guest's adopted world matches the host's"
        );

        host.send_edit(&Edit::Added {
            layer: 1,
            nodes: vec![],
        });
        let events = poll_within(&guest, Duration::from_secs(2));
        assert!(matches!(
            events[0],
            SessionEvent::ApplyEdit(Edit::Added { layer: 1, .. })
        ));
    }

    fn flatten_map(world: &World) -> String {
        map_model::io::map_writer::write_map(&flatten_tree(world, MapFormat::Standard, None))
    }

    #[test]
    fn three_editors_hear_each_other_through_the_host() {
        // The host takes its first guest the usual way, then folds a second one in.
        let port = free_port();
        let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let first = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();

        let listener = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let second = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let second_id = host.add_peer(listener.join().unwrap());
        assert_eq!(host.participants(), 2);

        // The first guest speaks; the host's poll relays it to the second, stamped
        // with who it came from.
        first.send_name("Ada");
        let ids: HashSet<NodeId> = HashSet::from([NodeId(7)]);
        first.send_selection(&ids);
        let events = poll_within(&host, Duration::from_secs(2));
        assert!(events
            .iter()
            .any(|e| matches!(e, SessionEvent::PeerName { peer: 1, name } if name == "Ada")));

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut named = false;
        let mut selected = false;
        while Instant::now() < deadline && !(named && selected) {
            // The relay happens as the host polls, so the hub keeps turning.
            let _ = host.poll(&bounds());
            for event in second.poll(&bounds()) {
                match event {
                    SessionEvent::PeerName { peer: 1, name } if name == "Ada" => named = true,
                    SessionEvent::PeerSelection { peer: 1, ids: got } if got == ids => {
                        selected = true;
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(named, "the second guest learns the first one's name");
        assert!(selected, "and sees their selection, attributed to them");

        // The host's own messages arrive unwrapped and read as peer 0.
        host.send_name("Host");
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut host_named = false;
        while Instant::now() < deadline && !host_named {
            host_named = second.poll(&bounds()).iter().any(
                |e| matches!(e, SessionEvent::PeerName { peer: HOST_PEER, name } if name == "Host"),
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(host_named);

        // The first guest drops; the host announces it to the second by id and name.
        drop(first);
        let events = poll_within(&host, Duration::from_secs(2));
        assert!(events.iter().any(|e| matches!(
            e,
            SessionEvent::PeerLeft { peer: 1, name: Some(name) } if name == "Ada"
        )));
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut told = false;
        while Instant::now() < deadline && !told {
            let _ = host.poll(&bounds());
            told = second
                .poll(&bounds())
                .iter()
                .any(|e| matches!(e, SessionEvent::PeerLeft { peer: 1, .. }));
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(told, "the second guest is told the first one left");
        let _ = second_id;
    }

    #[test]
    fn the_peer_camera_travels_and_comes_back_as_vectors() {
        let port = free_port();
        let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();

        let position = geometry_math::vec::Vec3d::new(-256.0, 128.0, 96.0);
        let direction = geometry_math::vec::Vec3d::new(0.6, 0.8, 0.0);
        host.send_camera(position, direction);

        let events = poll_within(&guest, Duration::from_secs(2));
        let SessionEvent::PeerCamera {
            peer: _,
            position: got_position,
            direction: got_direction,
        } = &events[0]
        else {
            panic!("expected the camera");
        };
        assert_eq!(*got_position, position);
        assert_eq!(*got_direction, direction);
    }

    #[test]
    fn a_guest_redials_what_it_joined_and_adopts_the_world_again() {
        let port = free_port();
        let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();

        // The host goes away mid-session; the guest notices.
        drop(host);
        let events = poll_within(&guest, Duration::from_secs(2));
        assert!(matches!(events.last(), Some(SessionEvent::ConnectionLost)));

        // What main.rs then does on each side: the host listens on the same port again,
        // the guest redials the origin it kept.
        let origin = guest.origin().clone();
        drop(guest);
        let host_thread =
            std::thread::spawn(move || Session::redial(&Origin::TcpHost(port)).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Session::redial(&origin).unwrap();
        let host = host_thread.join().unwrap();

        // The rejoin behaves like a fresh join: the world is re-sent and adopted, which
        // is what bounds the cost of the outage.
        let world = build_tree(&read_map(A_BOX, None).unwrap(), &bounds()).world;
        host.send_world(&world);
        let events = poll_within(&guest, Duration::from_secs(2));
        let SessionEvent::AdoptWorld(adopted) = &events[0] else {
            panic!("expected the world after the rejoin");
        };
        assert_eq!(flatten_map(&world), flatten_map(adopted));
    }

    #[test]
    fn a_dropped_peer_is_reported_once() {
        let port = free_port();
        let host_thread = std::thread::spawn(move || Session::host(port).unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let guest = Session::join(&format!("127.0.0.1:{port}")).unwrap();
        let host = host_thread.join().unwrap();

        drop(guest);
        let events = poll_within(&host, Duration::from_secs(2));
        assert!(matches!(events.last(), Some(SessionEvent::PeerLeft { .. })));
        // A second poll does not report it again.
        assert!(host
            .poll(&bounds())
            .iter()
            .all(|e| !matches!(e, SessionEvent::PeerLeft { .. })));
    }
}

#[cfg(test)]
#[path = "regression.rs"]
mod regression;
