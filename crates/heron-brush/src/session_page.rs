//! One window for hosting, joining and watching a live session: host or join, your name, how to host, what to send the guests, and the
//! buttons that start and stop it. Closing the window keeps the session running.
use crate::{dialog, editor::Editor, form, theme, widgets};
use editor_core::{
    collab::session::{Session, SessionInbox},
    i18n::{fill, tr, translation_key},
    mcp::FrontendState,
};
use eframe::egui;

/// The TCP port a local-network host listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 27960;

/// How many times a dropped guest tries to come back, two seconds apart.
const REDIAL_ATTEMPTS: u32 = 30;

enum Connected {
    Host(String, SessionInbox),
    Guest(Session),
}

/// What a connecting thread is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Connecting {
    /// Starting to host.
    Host,
    /// Joining at the user's request.
    Join,
    /// Joining again after the link dropped.
    Rejoin,
}

/// The labels and states of the two main buttons, from whether the window hosts or
/// joins and whether a session is running or connecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buttons {
    pub start: &'static str,
    pub start_enabled: bool,
    pub stop: &'static str,
    pub stop_enabled: bool,
}

/// The buttons for the state: Start session or Join while idle, Stop session or
/// Leave session while busy.
pub fn buttons(joining: bool, busy: bool) -> Buttons {
    Buttons {
        start: if joining {
            translation_key("Join")
        } else {
            translation_key("Start session")
        },
        start_enabled: !busy,
        stop: if joining {
            translation_key("Leave session")
        } else {
            translation_key("Stop session")
        },
        stop_enabled: busy,
    }
}

/// What the window and the header bar say about a session, untranslated: the window's
/// line and the indicator's label, or `None` while there is no session.
fn describe(state: FrontendState, connecting: Option<Connecting>) -> Option<(String, String)> {
    let same = |text: &'static str| Some((tr(text).to_string(), tr(text).to_string()));
    match (connecting, state) {
        (Some(Connecting::Host), _) => same(translation_key("Starting session")),
        (Some(Connecting::Join), _) => same(translation_key("Connecting")),
        (Some(Connecting::Rejoin), _) | (None, FrontendState::Reconnecting) => {
            same(translation_key("Live: reconnecting"))
        }
        (None, FrontendState::Hosting(count)) => {
            let line = if count == 0 {
                tr("Session running — waiting for guests").to_string()
            } else {
                fill("Connected guests: {}", &[&count.to_string()])
            };
            Some((line, fill("Live: hosting ({})", &[&count.to_string()])))
        }
        (None, FrontendState::Joined) => same(translation_key("Live: connected")),
        (None, FrontendState::Idle) => None,
    }
}

pub struct SessionPage {
    pub open: bool,
    was_open: bool,
    /// 0 hosts, 1 joins.
    mode: usize,
    name: String,
    internet: bool,
    port: u16,
    /// The connection details: the host's to send, or the ones a guest pasted.
    details: String,
    /// What the window says while there is no session: why it stopped or failed.
    message: String,
    pending: Option<std::sync::mpsc::Receiver<Result<Connected, String>>>,
    connecting: Option<Connecting>,
    /// What this editor joined, so a dropped link can be dialled again.
    joined: Option<String>,
    sent: Option<web_time::Instant>,
    join_revision: Option<u64>,
    state: FrontendState,
    /// The header bar's label, while a session runs or connects.
    indicator: Option<String>,
}

impl Default for SessionPage {
    fn default() -> Self {
        Self {
            open: false,
            was_open: false,
            // A page cannot host; it joins a desktop host.
            mode: if cfg!(target_arch = "wasm32") { 1 } else { 0 },
            name: String::new(),
            internet: cfg!(feature = "iroh"),
            port: DEFAULT_PORT,
            details: String::new(),
            message: String::new(),
            pending: None,
            connecting: None,
            joined: None,
            sent: None,
            join_revision: None,
            state: FrontendState::Idle,
            indicator: None,
        }
    }
}

impl SessionPage {
    /// What the header bar shows while a session runs or connects, such as "Live:
    /// hosting (2)"; `None` while there is none.
    pub fn indicator(&self) -> Option<String> {
        self.indicator.clone()
    }

    /// Whether a session runs or is connecting.
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.state != FrontendState::Idle
    }

    pub fn poll(&mut self, editor: &Editor, state: &mut editor_core::mcp::Collab) {
        if editor.document.borrow().in_transaction() {
            return;
        }
        if let Some(result) = self.pending.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.pending = None;
            let connecting = self.connecting.take();
            let result = match result {
                Ok(Connected::Host(token, inbox)) => state.host_frontend(self.name.trim().to_string(), token, inbox).map(|token| {
                    self.details = token;
                    editor_core::say!("live session ready -- connection details are in Live session. Waiting for guests");
                }),
                Ok(Connected::Guest(session)) => {
                    let changed = self.join_revision.take().is_some_and(|revision| revision != editor.document.borrow().revision());
                    if connecting == Some(Connecting::Join) && changed && !crate::app_actions::can_leave(&mut editor.document.borrow_mut()) {
                        Err(tr("Cancelled").to_owned())
                    } else {
                        let peer = session.peer().to_string();
                        state.join_frontend(&mut editor.document.borrow_mut(), self.name.trim().to_string(), session).map(|()| {
                            editor_core::say!("joined the live session at {peer}");
                        })
                    }
                }
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                editor_core::complain!("the live session could not start: {error}");
                if connecting == Some(Connecting::Rejoin) {
                    state.leave_frontend(&mut editor.document.borrow_mut());
                }
                self.idle(error);
            }
        }
        if !editor.document.borrow().in_transaction() {
            state.poll_frontend(&mut editor.document.borrow_mut());
        }
        let now = state.frontend_state();
        if now == FrontendState::Reconnecting
            && self.state != FrontendState::Reconnecting
            && self.pending.is_none()
        {
            // A guest tries to come back on its own; the world is sent again on the rejoin,
            // so a drop costs at most the edits made while apart, which are held for it.
            if let Some(target) = self.joined.clone() {
                editor_core::say!("connection lost -- trying to rejoin");
                self.connecting = Some(Connecting::Rejoin);
                self.start(move || {
                    let mut last = String::new();
                    for attempt in 0..REDIAL_ATTEMPTS {
                        if attempt > 0 {
                            std::thread::sleep(web_time::Duration::from_secs(2));
                        }
                        match connect(&target) {
                            Ok(session) => return Ok(Connected::Guest(session)),
                            Err(error) => last = error,
                        }
                    }
                    Err(last)
                });
            }
        }
        self.state = now;
        self.indicator = describe(self.state, self.connecting).map(|(_, label)| label);
        if self
            .sent
            .is_none_or(|last| last.elapsed().as_millis() >= 100)
        {
            state.frontend_presence(&editor.document.borrow(), &editor.camera.borrow());
            self.sent = Some(web_time::Instant::now());
        }
    }

    /// Back to no session, saying `message`.
    fn idle(&mut self, message: String) {
        self.pending = None;
        self.connecting = None;
        self.message = message;
        if self.mode == 0 {
            self.details.clear();
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        state: &mut editor_core::mcp::Collab,
    ) {
        if !self.open {
            self.was_open = false;
            return;
        }
        if !self.was_open {
            self.was_open = true;
            if self.name.is_empty() {
                self.name = editor_core::collab::nickname();
            }
        }
        let busy = self.busy();
        let joining = self.mode == 1;
        let labels = buttons(joining, busy);
        let mut close = false;
        let mut start = false;
        let mut stop = false;
        self.open = dialog::show(
            ctx,
            "session",
            tr("Live session"),
            [640.0, 600.0],
            |ui, part| {
                if part == dialog::Part::Actions {
                    start =
                        form::accent_button(ui, tr(labels.start), labels.start_enabled).clicked();
                    close = dialog::button(ui, tr("Close"), true).clicked();
                    stop = dialog::button(ui, tr(labels.stop), labels.stop_enabled).clicked();
                    return;
                }
                self.body(ui, busy);
            },
        ) && !close;
        if start {
            if joining {
                self.join(editor);
            } else {
                self.host();
            }
        }
        if stop {
            state.leave_frontend(&mut editor.document.borrow_mut());
            self.joined = None;
            self.state = FrontendState::Idle;
            self.indicator = None;
            self.idle(tr("Session stopped").to_string());
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, busy: bool) {
        let joining = self.mode == 1;
        egui::ScrollArea::vertical().id_salt("session").auto_shrink([false, false]).max_height(ui.available_height() - 52.0).show(ui, |ui| {
            ui.set_max_width(ui.available_width() - 10.0);
            ui.add_enabled_ui(!busy, |ui| {
                widgets::card(ui, "", None, |ui| {
                    widgets::row(ui, tr("Mode"), None, |ui| {
                        let width = ui.available_width().min(260.0);
                        let names = [tr("Host a session"), tr("Join a session")];
                        if widgets::dropdown(ui, "session-mode", &mut self.mode, &names, width) {
                            self.details.clear();
                            self.message.clear();
                        }
                    });
                    widgets::row(ui, tr("Your name"), None, |ui| {
                        let width = ui.available_width().min(260.0);
                        let hint = editor_core::collab::default_nickname();
                        if let Some(text) = widgets::field(ui, ui.scope_id().with("session-name"), &self.name, &hint, width) {
                            self.name = text;
                        }
                    });
                    if !joining {
                        let iroh = cfg!(feature = "iroh");
                        ui.add_enabled_ui(iroh, |ui| {
                            form::switch_row(ui, tr("Connect over the internet"), None, &mut self.internet);
                        });
                        ui.add_enabled_ui(!self.internet, |ui| {
                            widgets::row(ui, tr("Local network port"), None, |ui| {
                                let width = ui.available_width().min(160.0);
                                let mut port = f64::from(self.port);
                                if widgets::spin_width(ui, &mut port, 1.0..=65535.0, 1.0, 0, width).changed() {
                                    self.port = port.round() as u16;
                                }
                            });
                        });
                        if !iroh {
                            form::note(ui, tr("This build supports the local network only. Guests must be on the same network."));
                        }
                    }
                });
            });
            ui.add_space(10.0);
            form::text(
                ui,
                if joining {
                    tr("Paste the connection details from the host. Joining replaces the open map with the host's map. Save your work before joining. Closing this window keeps the connection open.")
                } else {
                    tr("Start the session, then send the connection details below to your guests. They choose Join a session in this window. Closing this window keeps the session running.")
                },
            );
            ui.add_space(10.0);
            let status = describe(self.state, self.connecting)
                .map(|(line, _)| line)
                .unwrap_or_else(|| if self.message.is_empty() { tr("Session not running").to_string() } else { self.message.clone() });
            ui.label(egui::RichText::new(status).font(theme::body_bold()).color(theme::fg()));
            ui.add_space(6.0);
            let editable = joining && !busy;
            egui::Frame::new()
                .fill(theme::entry())
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .corner_radius(egui::CornerRadius::same(theme::radius()))
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.details)
                            .id(egui::Id::unique("session-details"))
                            .font(egui::TextStyle::Monospace)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(4)
                            .interactive(editable),
                    );
                });
            if !joining {
                ui.add_space(6.0);
                let ready = !self.details.is_empty() && self.state != FrontendState::Idle;
                if form::button(ui, tr("Copy connection details"), ready).clicked() {
                    ui.ctx().copy_text(self.details.clone());
                }
            }
        });
    }

    fn host(&mut self) {
        let internet = self.internet && cfg!(feature = "iroh");
        let port = self.port;
        self.remember_name();
        self.connecting = Some(Connecting::Host);
        self.start(move || {
            let (token, inbox) = if internet {
                Session::host_iroh()
            } else {
                Session::host_on_this_network(port)
            }
            .map_err(|e| e.to_string())?;
            Ok(Connected::Host(token, inbox))
        });
    }

    fn join(&mut self, editor: &Editor) {
        let target: String = self.details.split_whitespace().collect();
        if target.is_empty() {
            self.message = tr("Paste connection details first").to_string();
            return;
        }
        if !crate::app_actions::can_leave(&mut editor.document.borrow_mut()) {
            return;
        }
        self.remember_name();
        self.join_revision = Some(editor.document.borrow().revision());
        self.joined = Some(target.clone());
        self.connecting = Some(Connecting::Join);
        self.start(move || connect(&target).map(Connected::Guest));
    }

    /// The name goes in the preferences once a session starts with it.
    fn remember_name(&mut self) {
        let name = self.name.trim().to_string();
        editor_core::preferences::edit(|p| p.collab_nickname = name);
    }

    fn start(&mut self, connect: impl FnOnce() -> Result<Connected, String> + Send + 'static) {
        self.message.clear();
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        // The web build has no threads: connecting there fails at once, on this one.
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let _ = tx.send(connect());
        });
        #[cfg(target_arch = "wasm32")]
        let _ = tx.send(connect());
    }
}

/// Joins at `target`: `host:port` over the local network, anything else over iroh.
fn connect(target: &str) -> Result<Session, String> {
    let address = target
        .rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok());
    if address {
        Session::join(target)
    } else {
        Session::join_iroh(target)
    }
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The buttons follow the mode and whether a session runs.
    #[test]
    fn buttons_follow_the_state() {
        assert_eq!(
            buttons(false, false),
            Buttons {
                start: "Start session",
                start_enabled: true,
                stop: "Stop session",
                stop_enabled: false
            }
        );
        assert_eq!(
            buttons(false, true),
            Buttons {
                start: "Start session",
                start_enabled: false,
                stop: "Stop session",
                stop_enabled: true
            }
        );
        assert_eq!(buttons(true, false).start, "Join");
        assert_eq!(buttons(true, true).stop, "Leave session");
        assert!(buttons(true, true).stop_enabled && !buttons(true, true).start_enabled);
    }

    /// The window and the header say where the session stands in words, not debug text.
    #[test]
    fn states_read_as_sentences() {
        assert_eq!(describe(FrontendState::Idle, None), None);
        assert_eq!(
            describe(FrontendState::Idle, Some(Connecting::Host))
                .unwrap()
                .0,
            "Starting session"
        );
        assert_eq!(
            describe(FrontendState::Idle, Some(Connecting::Join))
                .unwrap()
                .0,
            "Connecting"
        );
        let (line, label) = describe(FrontendState::Hosting(0), None).unwrap();
        assert_eq!(
            (line.as_str(), label.as_str()),
            ("Session running — waiting for guests", "Live: hosting (0)")
        );
        assert_eq!(
            describe(FrontendState::Hosting(2), None).unwrap().0,
            "Connected guests: 2"
        );
        assert_eq!(
            describe(FrontendState::Joined, None).unwrap().1,
            "Live: connected"
        );
        assert_eq!(
            describe(FrontendState::Reconnecting, None).unwrap().1,
            "Live: reconnecting"
        );
    }

    /// A fresh window hosts on the default port and starts idle, with nothing in the header.
    #[test]
    fn a_new_window_is_idle_on_the_default_port() {
        let page = SessionPage::default();
        assert_eq!(page.port, DEFAULT_PORT);
        assert!(!page.busy());
        assert_eq!(page.indicator(), None);
        assert_eq!(page.internet, cfg!(feature = "iroh"));
    }
}
