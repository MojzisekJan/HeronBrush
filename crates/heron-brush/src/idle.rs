//! When the window wakes up on its own.
//!
//! egui draws a frame for every input event and for every `request_repaint`; with nothing
//! happening it sleeps. The editor used to wake every second regardless, for the autosave
//! timer and the background links, and each of those frames drew the 3D view again: on a
//! software renderer most of a core while the window sat idle. The window now wakes only
//! for what is actually waiting on a clock.

use web_time::Duration;

/// How often the live link is looked at while it runs.
pub const LIVE_LINK: Duration = Duration::from_millis(100);
/// How often a collaboration session, or the MCP server, is looked at while it runs.
pub const SESSION: Duration = Duration::from_millis(20);
/// How often a session that is still connecting is looked at.
pub const CONNECTING: Duration = Duration::from_millis(100);

/// What is waiting on a clock this frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct Waiting {
    /// When the next autosave is due ([`editor_core::autosave::Autosaver::next_due`]).
    pub autosave: Option<Duration>,
    /// A live link runs.
    pub live_link: bool,
    /// A collaboration session runs, or the MCP server listens.
    pub session: bool,
    /// A collaboration session is connecting.
    pub connecting: bool,
}

/// How long the window may sleep before it has to draw a frame on its own, or `None` to
/// sleep until the next event.
pub fn next_wake(waiting: &Waiting) -> Option<Duration> {
    [
        waiting.autosave,
        waiting.live_link.then_some(LIVE_LINK),
        waiting.session.then_some(SESSION),
        waiting.connecting.then_some(CONNECTING),
    ]
    .into_iter()
    .flatten()
    .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_idle_window_sleeps_until_the_next_event() {
        assert_eq!(next_wake(&Waiting::default()), None);
    }

    #[test]
    fn each_clock_wakes_the_window_and_the_soonest_wins() {
        let autosave = Waiting {
            autosave: Some(Duration::from_secs(540)),
            ..Default::default()
        };
        assert_eq!(next_wake(&autosave), Some(Duration::from_secs(540)));
        assert_eq!(
            next_wake(&Waiting {
                live_link: true,
                ..autosave
            }),
            Some(LIVE_LINK)
        );
        assert_eq!(
            next_wake(&Waiting {
                session: true,
                ..Default::default()
            }),
            Some(SESSION)
        );
        assert_eq!(
            next_wake(&Waiting {
                connecting: true,
                ..Default::default()
            }),
            Some(CONNECTING)
        );
        assert_eq!(
            next_wake(&Waiting {
                live_link: true,
                session: true,
                connecting: true,
                autosave: Some(Duration::ZERO)
            }),
            Some(Duration::ZERO)
        );
    }
}
