//! Session-scoped `_tb_id` generation.
//!
//! Sequential file-local IDs alone can collide when independent collaborators create nodes. Session
//! prefixes distinguish their new nodes during merging.
//!
//! An [`IdSource`] stamps every id with a random per-session prefix in the high 32 bits,
//! so ids coined in different sessions do not collide even with no server to coordinate
//! them. The value stays a plain decimal integer, which is how other
//! editors parse `_tb_id` back.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// A random prefix fixed for the lifetime of the process.
///
/// [`crate::node::NodeId`] uses it so that ids minted here never equal ids minted in
/// another editor's process -- which is what lets a live editing session apply another
/// person's `Added` / `Changed` / `Removed` edits by the node id they carry.
pub fn session_prefix() -> u32 {
    static PREFIX: OnceLock<u32> = OnceLock::new();
    *PREFIX.get_or_init(random_prefix)
}

/// A source of `_tb_id` values for one editing session.
#[derive(Debug)]
pub struct IdSource {
    /// The session prefix, already shifted into the high 32 bits.
    prefix: u64,
    /// The next low-32-bit sequence number.
    next: AtomicU64,
}

impl IdSource {
    /// A source with a random session prefix.
    pub fn new() -> Self {
        Self::with_prefix(random_prefix())
    }

    /// A source with the given session prefix. For tests, and for a session that wants a
    /// prefix it chose itself.
    pub fn with_prefix(prefix: u32) -> Self {
        Self {
            prefix: u64::from(prefix) << 32,
            next: AtomicU64::new(1),
        }
    }

    /// The session prefix.
    pub fn prefix(&self) -> u32 {
        (self.prefix >> 32) as u32
    }

    /// A fresh id, as the decimal string `_tb_id` holds. Never `"0"`.
    pub fn fresh(&self) -> String {
        let seq = self.next.fetch_add(1, Ordering::Relaxed);
        (self.prefix | seq).to_string()
    }
}

impl Default for IdSource {
    fn default() -> Self {
        Self::new()
    }
}

/// A random 32-bit value, seeded from `RandomState` (ASLR plus a per-thread counter) and
/// the wall clock. Good enough for a prefix: it does not need to be unpredictable, only
/// unlikely to match another session's.
fn random_prefix() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    let nanos = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    hasher.write_u64(nanos);
    (hasher.finish() >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sequential_within_a_prefix() {
        let source = IdSource::with_prefix(3);
        let base = u64::from(3u32) << 32;
        assert_eq!(source.fresh(), (base | 1).to_string());
        assert_eq!(source.fresh(), (base | 2).to_string());
        assert_eq!(source.fresh(), (base | 3).to_string());
    }

    #[test]
    fn two_sessions_do_not_share_a_prefix_in_practice() {
        // A weak check: two fresh sources almost never pick the same prefix, and their
        // ids therefore never collide.
        let a = IdSource::new();
        let b = IdSource::new();
        assert_ne!(a.prefix(), b.prefix());
        assert_ne!(a.fresh(), b.fresh());
    }

    #[test]
    fn a_fresh_id_is_never_zero() {
        assert_ne!(IdSource::with_prefix(0).fresh(), "0");
    }

    #[test]
    fn a_prefixed_id_does_not_look_like_a_small_file_id() {
        let id: u64 = IdSource::new().fresh().parse().unwrap();
        assert!(
            id > u64::from(u32::MAX),
            "{id} should be well past any hand-written id"
        );
    }
}
