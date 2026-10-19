//! What the editor has to say, where it can be read.
//!
//! `say!` and `complain!` send messages to the terminal and keep them here. A frontend's
//! output pane reads what was said so far with [`lines`] and hears each new line through
//! the listener it sets with [`set_listener`].

use std::cell::RefCell;

/// The maximum number of retained output lines.
pub const KEPT: usize = 500;

thread_local! {
    /// The lines said so far, and who to tell about the next one.
    static LOG: RefCell<Log> = const { RefCell::new(Log::new()) };
}

/// What has been said, and who is listening.
struct Log {
    /// The lines, oldest first.
    lines: Vec<(Kind, String)>,
    /// The frontend's pane, once there is one.
    listener: Option<Box<dyn Fn(Kind, &str)>>,
}

impl Log {
    const fn new() -> Self {
        Self { lines: Vec::new(), listener: None }
    }
}

/// What kind of line it is, which is what decides how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Something that happened.
    Said,
    /// Something that did not.
    Complaint,
}

/// Says something, in the console and on standard output.
///
/// Takes the arguments `println!` does.
#[macro_export]
macro_rules! say {
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        println!("{line}");
        $crate::console::record_said(line);
    }};
}

/// Complains about something, in the console and on standard error.
#[macro_export]
macro_rules! complain {
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        eprintln!("{line}");
        $crate::console::record_complaint(line);
    }};
}

/// Keeps a line that something happened. Called by `say!`.
pub fn record_said(line: String) {
    record(Kind::Said, line);
}

/// Keeps a line that something did not. Called by `complain!`.
pub fn record_complaint(line: String) {
    record(Kind::Complaint, line);
}

fn record(kind: Kind, line: String) {
    // The listener is taken out while it runs, so a pane that says something while
    // showing a line does not find the log already borrowed.
    let listener = LOG.with(|log| {
        let mut log = log.borrow_mut();
        if log.lines.len() >= KEPT {
            log.lines.remove(0);
        }
        log.lines.push((kind, line.clone()));
        log.listener.take()
    });
    if let Some(listener) = listener {
        listener(kind, &line);
        LOG.with(|log| {
            let mut log = log.borrow_mut();
            if log.listener.is_none() {
                log.listener = Some(listener);
            }
        });
    }
}

/// Every line kept so far, oldest first.
pub fn lines() -> Vec<(Kind, String)> {
    LOG.with(|log| log.borrow().lines.clone())
}

/// Hands each new line to `listener` as well as keeping it.
pub fn set_listener(listener: impl Fn(Kind, &str) + 'static) {
    LOG.with(|log| log.borrow_mut().listener = Some(Box::new(listener)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_said_before_there_is_a_panel_is_kept_for_it() {
        record_said("something happened".to_string());
        assert!(lines().iter().any(|(kind, line)| *kind == Kind::Said && line == "something happened"));
    }

    #[test]
    fn only_so_many_lines_are_kept() {
        for line in 0..(KEPT + 50) {
            record_said(format!("line {line}"));
        }
        assert!(lines().len() <= KEPT);
    }

    #[test]
    fn a_listener_hears_new_lines() {
        let heard = std::rc::Rc::new(RefCell::new(Vec::new()));
        let sink = std::rc::Rc::clone(&heard);
        set_listener(move |kind, line| sink.borrow_mut().push((kind, line.to_string())));
        record_complaint("it broke".into());
        assert_eq!(heard.borrow().as_slice(), &[(Kind::Complaint, "it broke".to_string())]);
    }
}
