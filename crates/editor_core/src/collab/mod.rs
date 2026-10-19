//! Live collaborative editing: two people editing one map at once.
//!
//! - [`wire`] turns an [`Edit`](crate::history::Edit) into a line of JSON and back.
//! - [`net`] is the transport: one editor hosts, the other joins, and each sends its
//!   edits to the other over TCP.
//!
//! `main.rs` hosts the GUI session; MCP uses the same session and document operations.
//! TCP accepts multiple peers, and the optional `iroh` transport reaches other networks.
//! The host orders edits and echoes guest submissions as acknowledgements. Conflicting
//! changes to the same node use the last edit in host order; they are not field-merged.
//! Guests retain unacknowledged edits for replay after adopting a reconnect snapshot.
//! GUI reconnect is automatic; a headless MCP guest explicitly joins again.

// `wire`'s per-type `to_json` / `from_json` and a couple of re-exports are exercised only
// by the tests; `Session` on the live path goes through `WireMessage`.
#![allow(dead_code, unused_imports)]

pub mod net;
pub mod session;
pub mod wire;

pub use net::{Link, Role};
pub use session::{Origin, Session, SessionEvent, HOST_PEER};

/// The colour one participant draws in -- their selection's edges, their camera -- the
/// same on every screen, because a participant's id is the same everywhere: the host is
/// 0 and guests are numbered as they join. Amber first, as the two-person sessions
/// always were, then round a small palette.
pub fn peer_color_bytes(peer: u32) -> [u8; 3] {
    const PALETTE: [[u8; 3]; 6] = [
        [0xF5, 0xA6, 0x23], // amber
        [0x2E, 0xB8, 0xD5], // azure
        [0xB8, 0x6F, 0xE0], // violet
        [0x5F, 0xC1, 0x5A], // green
        [0xE8, 0x63, 0x8B], // rose
        [0x3F, 0xD0, 0xA4], // teal
    ];
    PALETTE[(peer as usize) % PALETTE.len()]
}

/// [`peer_color_bytes`] as the 3D overlay wants it.
pub fn peer_color_f32(peer: u32) -> [f32; 4] {
    let [r, g, b] = peer_color_bytes(peer);
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ]
}
pub use wire::{WireEdit, WireError, WireMessage, WireWorld};

/// The name to show the other person: the `collab_nickname` preference, or the system
/// user name when that is blank, or `"Someone"` as a last resort.
pub fn nickname() -> String {
    let set = crate::preferences::pref(|p| p.collab_nickname.clone());
    if !set.trim().is_empty() {
        return set;
    }
    default_nickname()
}

/// The fallback name, shown as placeholder text in the preferences and used when the
/// nickname is left blank.
pub fn default_nickname() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Someone".to_string())
}
