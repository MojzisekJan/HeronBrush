//! The keyframe animation editor: a mode switch, the named
//! clip library, transport and clip settings, key tools, the timeline with a key
//! inspector beside it, onion skin and pivot, and the viewport pose tools.
//!
//! The shared [`State`] holds the clip, the selection, the playhead and the timeline's
//! zoom; a checked [`Draft`] writes it to the document. Undo, Redo and edits elsewhere
//! reload the clip into the same state, so zoom, selection and playhead time survive.
use crate::{dialog, editor::Editor, theme, widgets};
use editor_core::{
    animation_curves::{Interpolation, Key},
    animation_editor::Draft,
    animation_library::{Entry, Library},
    animation_timeline::{State, TimelineGeometry, TimelineStyle},
    i18n::{tr, tr_text},
};
use eframe::egui;
use std::collections::BTreeSet;

const MODES: [&str; 3] = ["once", "loop", "pingpong"];
const INTERPOLATIONS: [Interpolation; 3] = [Interpolation::Linear, Interpolation::Hold, Interpolation::Bezier];

pub struct AnimationPage {
    pub open: bool,
    pub detached: bool,
    draft: Option<Draft>,
    state: Option<State>,
    /// The library name of the clip in `state`; a different one resets the view.
    name: String,
    clipboard: Vec<(usize, Key)>,
    playing: bool,
    direction: f64,
    last: Option<web_time::Instant>,
    status: String,
    library_name: String,
    library_status: String,
    retime_scale: f64,
    retime_offset: f64,
    retime_from_cursor: bool,
    retime_extend: bool,
    /// How many onion frames to show when onion skinning is switched back on.
    onion_count: u8,
    timeline: Timeline,
}
#[derive(Default)]
struct Timeline {
    /// Whether the timeline had keyboard focus last frame.
    focused: bool,
    pointer: Option<egui::Pos2>,
}
impl Default for AnimationPage {
    fn default() -> Self {
        Self {
            open: false,
            detached: false,
            draft: None,
            state: None,
            name: String::new(),
            clipboard: Vec::new(),
            playing: false,
            direction: 1.0,
            last: None,
            status: String::new(),
            library_name: String::new(),
            library_status: String::new(),
            retime_scale: 1.0,
            retime_offset: 0.0,
            retime_from_cursor: false,
            retime_extend: false,
            onion_count: 1,
            timeline: Timeline::default(),
        }
    }
}

/// A raised button sized to its label.
fn button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let width = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), theme::body(), theme::fg()).size().x) + 24.0;
    theme::button(ui, label, width.max(40.0), enabled)
}
/// A toggle button: accent-filled while on.
fn toggle(ui: &mut egui::Ui, label: &str, on: &mut bool) -> egui::Response {
    let width = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), theme::body(), theme::fg()).size().x) + 24.0;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(width.max(40.0), widgets::ROW_HEIGHT), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    theme::paint_button(ui, rect, &response, label, true, *on);
    response
}
/// A square button with a painted icon, for glyphs the UI font lacks.
fn icon_button(ui: &mut egui::Ui, tooltip: &str, paint: impl Fn(&egui::Painter, egui::Rect, egui::Stroke)) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, widgets::ROW_HEIGHT), egui::Sense::click());
    theme::paint_button(ui, rect, &response, "", true, false);
    paint(ui.painter(), egui::Rect::from_center_size(rect.center(), egui::vec2(14.0, 14.0)), egui::Stroke::new(1.5, theme::fg()));
    response.on_hover_text(tooltip)
}
fn dim(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(theme::dim()));
}

impl AnimationPage {
    // ----- document link -------------------------------------------------------------

    /// Reads the selection's animation when the document moved on. The same selection
    /// reloads into the existing state, keeping the view; another one starts afresh.
    fn sync(&mut self, editor: &Editor) {
        let document = editor.document.borrow();
        if self.draft.as_ref().is_some_and(|d| d.is_current(&document)) {
            return;
        }
        if self.state.as_ref().is_some_and(State::dragging) {
            return;
        }
        match Draft::read(&document) {
            Err(error) => {
                self.status = tr_text(&error);
                self.draft = None;
                self.state = None;
                self.playing = false;
            }
            Ok(draft) => {
                let name = draft.name(&document);
                let keep = self.draft.as_ref().is_some_and(|old| old.same_selection(&document));
                match (&mut self.state, keep) {
                    (Some(state), true) => {
                        state.reload(draft.clip.clone(), draft.mode.clone(), draft.pivot, name != self.name);
                    }
                    _ => {
                        let mut state = State::new(draft.clip.clone(), draft.mode.clone(), draft.pivot);
                        let f = *editor.filters.borrow();
                        if let Some(frame) = f.animation_frame {
                            state.frame = (frame as f64 / f.animation_fps.max(1) as f64).min(state.clip.duration) * state.clip.fps as f64;
                        }
                        self.state = Some(state);
                        self.playing = false;
                        self.status.clear();
                    }
                }
                if self.draft.is_none() {
                    self.status.clear();
                }
                self.name = name;
                self.draft = Some(draft);
                drop(document);
                if editor.filters.borrow().animation_frame.is_some() {
                    self.seek(editor);
                }
            }
        }
    }
    /// Shows the playhead's pose in the views: the edit-animation mode.
    fn seek(&self, editor: &Editor) {
        if let Some(state) = &self.state {
            let mut f = editor.filters.borrow_mut();
            f.animation_frame = Some(state.frame.round().max(0.0) as u32);
            f.animation_fps = state.clip.fps;
        }
        (editor.redraw)();
    }
    fn pending(&self) -> bool {
        match (&self.draft, &self.state) {
            (Some(d), Some(s)) => d.clip != s.clip || d.mode != s.mode || d.pivot != s.pivot,
            _ => false,
        }
    }
    /// Writes the state as one undoable edit, when it differs from the document (or
    /// `force`, which creates the animation entity for a blank clip).
    fn commit(&mut self, editor: &Editor, force: bool) {
        if !force && !self.pending() {
            if self.state.is_some() && self.draft.is_some() {
                self.seek(editor);
            }
            return;
        }
        let (Some(draft), Some(state)) = (&mut self.draft, &self.state) else {
            return;
        };
        draft.clip = state.clip.clone();
        draft.mode = state.mode.clone();
        draft.pivot = state.pivot;
        let result = draft.commit(&mut editor.document.borrow_mut());
        match result {
            Ok(()) => {
                self.status.clear();
                self.name = draft.name(&editor.document.borrow());
                self.seek(editor);
            }
            Err(error) => {
                self.status = tr_text(&error);
                // Forget the draft so the next frame reloads what the document holds.
                if !draft.is_current(&editor.document.borrow()) {
                    self.draft = None;
                }
            }
        }
    }
    /// Follows the views: a clicked motion-path key, or a press that moved the playhead.
    fn follow_views(&mut self, editor: &Editor) {
        let (select, seek) = {
            let mut link = editor.animation_link.borrow_mut();
            (link.select_key.take(), link.seek.take())
        };
        let Some(state) = &mut self.state else {
            return;
        };
        if let Some(time) = select {
            state.select_position_key(time);
            self.playing = false;
            self.seek(editor);
        } else if let Some((frame, fps)) = seek {
            state.cancel();
            state.frame = (frame as f64 / fps.max(1) as f64).min(state.clip.duration) * state.clip.fps as f64;
            self.playing = false;
        }
    }

    // ----- playback ---------------------------------------------------------------------

    fn set_playing(&mut self, on: bool, editor: &Editor) {
        if on && !self.playing {
            if let Some(state) = &mut self.state {
                state.start_playback();
            }
            self.direction = 1.0;
            self.last = Some(web_time::Instant::now());
            self.commit(editor, false);
        }
        self.playing = on;
        self.seek(editor);
    }
    /// Advances playback by the time since the last frame, as the clip's mode says.
    fn tick(&mut self, ctx: &egui::Context, editor: &Editor) {
        let now = web_time::Instant::now();
        let seconds = self.last.replace(now).map_or(0.0, |last| now.duration_since(last).as_secs_f64());
        self.advance(seconds, editor);
        if self.playing {
            ctx.request_repaint_after(web_time::Duration::from_millis(16));
        }
    }
    fn advance(&mut self, seconds: f64, editor: &Editor) {
        let Some(state) = &mut self.state else {
            self.playing = false;
            return;
        };
        if !self.playing || state.dragging() {
            return;
        }
        if state.advance_playback(seconds, &mut self.direction) {
            self.playing = false;
        }
        self.seek(editor);
    }

    // ----- commands ----------------------------------------------------------------------

    fn set_auto_key(&mut self, on: bool, editor: &Editor) {
        if on {
            self.playing = false;
            let blank = self.draft.as_ref().is_some_and(|d| d.blank(&editor.document.borrow()));
            if blank {
                if let Some(state) = &mut self.state {
                    state.clip.tracks = std::array::from_fn(|_| Vec::new());
                    for channel in 0..6 {
                        let _ = state.clip.insert(channel, 0.0, 0.0);
                    }
                }
                self.commit(editor, true);
            }
            self.seek(editor);
            let mut f = editor.filters.borrow_mut();
            if f.onion_frames == 0 {
                f.onion_frames = self.onion_count.clamp(1, 4);
            }
        }
        editor.filters.borrow_mut().animation_auto_key = on;
        (editor.redraw)();
    }
    fn add_key(&mut self, editor: &Editor) {
        self.playing = false;
        let Some(state) = &mut self.state else { return };
        let channel = state.active;
        let time = state.playhead_time();
        let value = state.clip.sample(channel, time);
        if let Err(error) = state.insert_key(channel, time, value) {
            self.status = tr_text(&error);
            return;
        }
        self.commit(editor, false);
        // A new key must be editable in the views at once; after the commit, so blank
        // clip initialisation cannot erase it.
        self.set_auto_key(true, editor);
    }
    fn remove(&mut self, editor: &Editor) {
        let Some(state) = &mut self.state else { return };
        if state.selected.is_empty() {
            return;
        }
        let ids = std::mem::take(&mut state.selected);
        state.clip.delete(&ids);
        self.commit(editor, false);
    }
    fn copy(&mut self) {
        if let Some(state) = &self.state {
            self.clipboard = state.clip.copy(&state.selected);
        }
    }
    fn paste(&mut self, editor: &Editor) {
        if self.clipboard.is_empty() {
            return;
        }
        let Some(state) = &mut self.state else { return };
        let time = state.playhead_time();
        match state.clip.paste(&self.clipboard, time) {
            Ok(ids) => state.selected = ids,
            Err(error) => {
                self.status = tr_text(&error);
                return;
            }
        }
        self.commit(editor, false);
    }
    /// Copy, then paste at the playhead.
    fn duplicate(&mut self, editor: &Editor) {
        self.copy();
        self.paste(editor);
    }
    fn transport(&mut self, direction: i32, editor: &Editor) {
        self.playing = false;
        let Some(state) = &mut self.state else { return };
        let time = state.playhead_time();
        let keys = || state.clip.tracks.iter().flatten().map(|k| k.time);
        let target = match direction {
            0 => 0.0,
            2 => state.clip.duration,
            -1 => keys().filter(|t| *t < time - 1e-6).fold(0.0, f64::max),
            _ => keys().filter(|t| *t > time + 1e-6).fold(state.clip.duration, f64::min),
        };
        state.frame = target * state.clip.fps as f64;
        self.seek(editor);
    }
    fn leave_animation_mode(&mut self, editor: &Editor) {
        self.playing = false;
        let mut f = editor.filters.borrow_mut();
        f.animation_frame = None;
        f.preview_motion = false;
        drop(f);
        (editor.redraw)();
    }

    // ----- keyboard -------------------------------------------------------------------

    /// Animation editing owns the clipboard and key commands while the timeline has focus
    /// or the views show a pose. Returns true when the event was used.
    pub fn key_event(&mut self, event: &egui::Event, editor: &Editor) -> bool {
        let posing = editor.filters.borrow().animation_frame.is_some();
        if !self.open || (!posing && !self.timeline.focused) {
            return false;
        }
        self.sync(editor);
        if self.state.is_none() {
            return false;
        }
        let timeline = self.timeline.focused;
        match event {
            egui::Event::Copy => self.copy(),
            egui::Event::Cut => {
                self.copy();
                self.remove(editor);
            }
            egui::Event::Paste(_) => self.paste(editor),
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                let command = modifiers.command || modifiers.ctrl;
                match key {
                    egui::Key::Z if command => {
                        if let Some(state) = &mut self.state {
                            state.cancel();
                        }
                        // Pose edits in progress are written first, so Undo takes them back.
                        self.commit(editor, false);
                        if modifiers.shift {
                            editor.document.borrow_mut().redo();
                        } else {
                            editor.document.borrow_mut().undo();
                        }
                        self.sync(editor);
                    }
                    egui::Key::D if command => self.duplicate(editor),
                    egui::Key::A if command && timeline => {
                        if let Some(state) = &mut self.state {
                            state.select_all();
                        }
                    }
                    egui::Key::Delete | egui::Key::Backspace => self.remove(editor),
                    egui::Key::ArrowLeft | egui::Key::ArrowRight if !command => {
                        if let Some(state) = &mut self.state {
                            let step = if *key == egui::Key::ArrowLeft { -1.0 } else { 1.0 };
                            state.frame = (state.frame + step).clamp(0.0, state.end_frame());
                        }
                        self.seek(editor);
                    }
                    egui::Key::Space if !command => {
                        let on = !self.playing;
                        self.set_playing(on, editor);
                    }
                    egui::Key::Escape => {
                        if timeline {
                            if let Some(state) = &mut self.state {
                                state.cancel();
                            }
                            self.timeline.pointer = None;
                        } else if editor.animation_link.borrow().dragging {
                            return false;
                        } else {
                            self.leave_animation_mode(editor);
                        }
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }

    // ----- presentation -----------------------------------------------------------------

    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        self.present(ctx, editor, None);
    }
    pub fn show_docked(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let ctx = ui.ctx().clone();
        self.present(&ctx, editor, Some(ui));
    }
    fn present(&mut self, ctx: &egui::Context, editor: &Editor, panel: Option<&mut egui::Ui>) {
        if !self.open {
            return;
        }
        self.sync(editor);
        self.follow_views(editor);
        self.tick(ctx, editor);
        let mut close = false;
        let mut detach = false;
        let docked = panel.is_some();
        let visible = if let Some(ui) = panel {
            egui::Frame::new().fill(theme::bg()).inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.body(ui, editor, true, &mut close, &mut detach);
            });
            true
        } else {
            dialog::show(ctx, "animation", tr("Keyframe animation"), [1100.0, 560.0], |ui, part| {
                if part == dialog::Part::Actions {
                    close |= dialog::button(ui, tr("Close"), true).clicked();
                } else {
                    self.body(ui, editor, false, &mut close, &mut detach);
                }
            })
        };
        if detach {
            self.detached = docked;
        }
        // Edits commit once the pointer is up and no field is being typed in, so one
        // drag or one typed value is one undo step.
        let idle = !ctx.input(|i| i.pointer.any_down()) && !ctx.text_edit_focused();
        if idle && self.pending() && !self.state.as_ref().is_some_and(State::dragging) {
            self.commit(editor, false);
        }
        self.open = visible && !close;
        if !self.open {
            if self.pending() {
                self.commit(editor, false);
            }
            self.detached = false;
            self.playing = false;
            editor.filters.borrow_mut().animation_frame = None;
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, editor: &Editor, docked: bool, close: &mut bool, detach: &mut bool) {
        // Keys reach the panel itself while the timeline has focus; otherwise the window
        // passes them on through `key_event`.
        if self.timeline.focused && ui.ctx().egui_wants_keyboard_input() && !ui.ctx().text_edit_focused() {
            for event in ui.input(|i| i.events.clone()) {
                self.key_event(&event, editor);
            }
        }
        let total = ui.available_height();
        // Header: the panel's name, the live mode and its switch.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("Keyframe animation")).font(theme::body_bold()));
            ui.add_space(12.0);
            let f = *editor.filters.borrow();
            let editing = f.animation_frame.is_some();
            let label = editor_core::animation_mode::label(&f);
            ui.label(egui::RichText::new(label).color(if editing { theme::warning() } else { theme::dim() }));
            if self.state.is_some() {
                ui.add_space(12.0);
                self.library_row(ui, editor);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if docked {
                    if icon_button(ui, tr("Close"), |p, r, stroke| {
                        let r = r.shrink(2.0);
                        p.line_segment([r.left_top(), r.right_bottom()], stroke);
                        p.line_segment([r.right_top(), r.left_bottom()], stroke);
                    })
                    .clicked()
                    {
                        *close = true;
                    }
                    if icon_button(ui, tr("Animation editor in separate window"), |p, r, stroke| {
                        let back = egui::Rect::from_min_max(r.min + egui::vec2(4.0, 0.0), r.max - egui::vec2(0.0, 4.0));
                        let front = egui::Rect::from_min_max(r.min + egui::vec2(0.0, 4.0), r.max - egui::vec2(4.0, 0.0));
                        p.rect_stroke(back, 1.0, stroke, egui::StrokeKind::Middle);
                        p.rect_filled(front, 1.0, theme::button_fill());
                        p.rect_stroke(front, 1.0, stroke, egui::StrokeKind::Middle);
                    })
                    .clicked()
                    {
                        *detach = true;
                    }
                }
                let mode = tr(if editing { "Edit map" } else { "Edit animation" });
                if button(ui, mode, self.state.is_some()).clicked() {
                    if editing {
                        self.leave_animation_mode(editor);
                    } else {
                        self.seek(editor);
                    }
                }
            });
        });
        if self.state.is_none() {
            dim(ui, &self.status);
            return;
        }
        let top = total - ui.available_height();
        egui::ScrollArea::vertical().id_salt("animation-body").auto_shrink([false, false]).show(ui, |ui| {
            // Playback, timing and the key commands share one row, wrapping when narrow.
            ui.horizontal_wrapped(|ui| {
                self.transport_row(ui, editor);
                ui.add_space(8.0);
                self.tools_row(ui, editor);
            });
            // The timeline takes what the rows above and the options row below leave.
            let above = ui.min_rect().height();
            let timeline_height = (total - top - above - (widgets::ROW_HEIGHT + 2.0 * ui.spacing().item_spacing.y) - 4.0).max(7.0 * (widgets::ROW_HEIGHT + 4.0) + 30.0);
            ui.horizontal_top(|ui| {
                let width = (ui.available_width() - 256.0).max(240.0);
                ui.allocate_ui(egui::vec2(width, timeline_height), |ui| self.timeline(ui, editor, timeline_height));
                let (line, _) = ui.allocate_exact_size(egui::vec2(9.0, timeline_height), egui::Sense::hover());
                ui.painter().vline(line.center().x, line.y_range(), egui::Stroke::new(1.0, theme::grid()));
                ui.vertical(|ui| {
                    ui.set_width(240.0);
                    self.inspector(ui);
                });
            });
            self.options_row(ui, editor);
            if !self.status.is_empty() {
                dim(ui, &self.status);
            }
        });
    }

    /// The named clips, in the header: the choice, and beside it a popover to save as,
    /// delete, make the two presets and set Play on start.
    fn library_row(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let entity = self.draft.as_ref().and_then(|d| d.entity);
        let (names, current, autoplay) = {
            let d = editor.document.borrow();
            match entity.map(|id| (id, Library::read(&d, id))) {
                Some((id, Ok((library, current)))) => {
                    let autoplay = map_model::node::entities_with_ids(d.tree_world())
                        .iter()
                        .any(|(n, e)| *n == id && e.property("start_on") != Some("0"));
                    (library.clips.keys().cloned().collect::<Vec<_>>(), current, autoplay)
                }
                _ => (Vec::new(), String::new(), false),
            }
        };
        let mut action: Option<(&str, String)> = None;
        let mut index = names.iter().position(|n| *n == current).unwrap_or(usize::MAX);
        let choices: Vec<&str> = names.iter().map(String::as_str).collect();
        if widgets::dropdown(ui, "animation-library", &mut index, &choices, 160.0) {
            if let Some(name) = names.get(index) {
                action = Some(("select", name.clone()));
            }
        }
        let more = icon_button(ui, tr("Animation"), |p, r, stroke| {
            // A stack of clips.
            for (i, inset) in [(0.0, 0.0), (1.0, 2.0), (2.0, 4.0)] {
                let y = r.top() + 3.0 + i * 4.5;
                p.line_segment([egui::pos2(r.left() + inset, y), egui::pos2(r.right() - inset, y)], stroke);
            }
            p.rect_stroke(egui::Rect::from_min_max(egui::pos2(r.left(), r.top() + 12.0), r.max), 1.5, stroke, egui::StrokeKind::Middle);
        });
        egui::Popup::from_toggle_button_response(&more).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            egui::Frame::new().inner_margin(4).show(ui, |ui| {
            ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.library_name)
                    .hint_text(tr("Animation name"))
                    .desired_width(130.0)
                    .margin(crate::widgets::body_entry_margin(ui, 6, 6))
                    .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT))
                    .vertical_align(egui::Align::Center),
            );
            if button(ui, tr("Save as"), true).clicked() {
                action = Some(("save_as", self.library_name.trim().to_owned()));
            }
            if button(ui, tr("Delete"), true).clicked() {
                if let Some(name) = names.get(index) {
                    action = Some(("delete", name.clone()));
                }
            }
            });
            ui.horizontal(|ui| {
            if button(ui, tr("Translate"), true).clicked() {
                action = Some(("translate", String::new()));
            }
            if button(ui, tr("Door animations"), true)
                .on_hover_text(tr("Create open and close clips with a 90 degree rotation around the current pivot."))
                .clicked()
            {
                action = Some(("door", String::new()));
            }
            });
            let mut on = autoplay;
            if crate::widgets::checkbox(ui, &mut on, tr("Play on start")).changed() {
                action = Some(("autoplay", if on { "1" } else { "0" }.to_owned()));
            }
            });
        });
        if !self.library_status.is_empty() {
            dim(ui, &self.library_status);
        }
        if let Some((action, name)) = action {
            if self.pending() {
                self.commit(editor, false);
            }
            let result = library_action(&mut editor.document.borrow_mut(), entity, action, &name);
            self.library_status = result.err().map(|e| tr_text(&e)).unwrap_or_default();
            (editor.redraw)();
        }
    }

    fn transport_row(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let mut play = self.playing;
        let mut step = None;
        {
            if button(ui, "|◀", true).on_hover_text(tr("Start")).clicked() {
                step = Some(0);
            }
            if button(ui, "◀|", true).on_hover_text(tr("Previous key")).clicked() {
                step = Some(-1);
            }
            toggle(ui, "▶", &mut play).on_hover_text(tr("Playback"));
            if button(ui, "|▶", true).on_hover_text(tr("Next key")).clicked() {
                step = Some(1);
            }
            if button(ui, "▶|", true).on_hover_text(tr("End")).clicked() {
                step = Some(2);
            }
            ui.add_space(8.0);
            let state = self.state.as_mut().unwrap();
            ui.label(tr("Frame"));
            let mut frame = state.frame;
            if widgets::spin_width(ui, &mut frame, 0.0..=12_000_000.0, 1.0, 0, 84.0).changed() {
                state.frame = frame.min(state.end_frame());
                step = Some(9);
            }
            ui.label("FPS");
            let mut fps = state.clip.fps as f64;
            if widgets::spin_width(ui, &mut fps, 1.0..=120.0, 1.0, 0, 72.0).changed() {
                state.set_fps(fps as u32);
            }
            ui.label(tr("Duration (s)"));
            let mut duration = state.clip.duration;
            if widgets::spin_width(ui, &mut duration, 0.001..=100000.0, 0.1, 3, 96.0).changed() {
                state.set_duration(duration);
            }
            let mut mode = MODES.iter().position(|m| *m == state.mode).unwrap_or(1);
            if widgets::dropdown(ui, "animation-mode", &mut mode, &[tr("Once"), tr("Loop"), tr("Ping-pong")], 112.0) {
                state.mode = MODES[mode].into();
            }
        }
        match step {
            Some(9) => self.seek(editor),
            Some(direction) => self.transport(direction, editor),
            None => {}
        }
        if play != self.playing {
            self.set_playing(play, editor);
        }
    }

    fn tools_row(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        #[derive(PartialEq)]
        enum Do {
            Add,
            Remove,
            Copy,
            Paste,
            Duplicate,
            Retime,
        }
        let mut command = None;
        {
            for (label, what) in [(tr("Add key"), Do::Add), (tr("Remove"), Do::Remove)] {
                if button(ui, label, true).clicked() {
                    command = Some(what);
                }
            }
            let state = self.state.as_mut().unwrap();
            let mut curves = state.curves;
            if toggle(ui, tr("Curves"), &mut curves).changed() {
                state.curves = curves;
                if curves {
                    let active = state.active;
                    let clip = &state.clip;
                    state.selected.retain(|id| clip.key(*id).is_some_and(|(c, _)| c == active));
                } else {
                    state.group_selection();
                }
                state.fit_values();
            }
            crate::widgets::checkbox(ui, &mut state.snap, tr("Snap to frames"));
            if button(ui, "−", true).clicked() {
                state.zoom(1.0 / 0.7);
            }
            if button(ui, "+", true).clicked() {
                state.zoom(0.7);
            }
            if button(ui, tr("Fit"), true).clicked() {
                state.start = 0.0;
                state.span = state.clip.duration * 1.1;
                state.fit_values();
            }
            let more = icon_button(ui, tr("Retime keys"), |p, r, stroke| {
                for dx in [-5.0, 0.0, 5.0] {
                    p.circle_filled(r.center() + egui::vec2(dx, 0.0), 1.6, stroke.color);
                }
            });
            egui::Popup::from_toggle_button_response(&more)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    // The popover's content keeps off its edge, as the dialogs' does.
                    egui::Frame::new().inner_margin(4).show(ui, |ui| {
                    // The clipboard of keys, which Cmd+C, Cmd+V and Cmd+D also reach.
                    ui.horizontal(|ui| {
                        for (label, what) in [(tr("Copy"), Do::Copy), (tr("Paste"), Do::Paste), (tr("Duplicate"), Do::Duplicate)] {
                            if button(ui, label, true).clicked() {
                                command = Some(what);
                                egui::Popup::close_all(ui.ctx());
                            }
                        }
                    });
                    ui.separator();
                    egui::Grid::new("animation-retime").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label(tr("Time scale"));
                        widgets::spin_width(ui, &mut self.retime_scale, 0.001..=1000.0, 0.1, 3, 130.0);
                        ui.end_row();
                        ui.label(tr("Offset (frames)"));
                        widgets::spin_width(ui, &mut self.retime_offset, -12_000_000.0..=12_000_000.0, 1.0, 2, 130.0);
                        ui.end_row();
                    });
                    crate::widgets::checkbox(ui, &mut self.retime_from_cursor, tr("Scale from playhead"));
                    crate::widgets::checkbox(ui, &mut self.retime_extend, tr("Extend duration"));
                    if button(ui, tr("Retime selection"), true).clicked() {
                        command = Some(Do::Retime);
                        egui::Popup::close_all(ui.ctx());
                    }
                    });
                });
        }
        match command {
            Some(Do::Add) => self.add_key(editor),
            Some(Do::Remove) => self.remove(editor),
            Some(Do::Copy) => self.copy(),
            Some(Do::Paste) => self.paste(editor),
            Some(Do::Duplicate) => self.duplicate(editor),
            Some(Do::Retime) => {
                let state = self.state.as_mut().unwrap();
                let ids = state.selected.clone();
                let pivot = if self.retime_from_cursor {
                    state.playhead_time()
                } else {
                    ids.iter().filter_map(|id| state.clip.key(*id).map(|(_, k)| k.time)).fold(f64::INFINITY, f64::min)
                };
                let offset = self.retime_offset / state.clip.fps as f64;
                match state.clip.retime(&ids, self.retime_scale, pivot, offset, self.retime_extend) {
                    Ok(()) => self.commit(editor, false),
                    Err(error) => self.status = tr_text(&error),
                }
            }
            None => {}
        }
    }

    /// The selected keys: their count, frame and value, the position at the playhead for
    /// the position lane, one interpolation for all, and the Bezier slopes.
    fn inspector(&mut self, ui: &mut egui::Ui) {
        let state = self.state.as_mut().unwrap();
        let fps = state.clip.fps as f64;
        let position_time = state.position_time();
        let single = if state.selected.len() == 1 {
            state.selected.first().and_then(|id| state.clip.key(*id)).map(|(c, k)| (c, k.clone()))
        } else {
            None
        };
        let value_rows = state.curves || state.active >= 3;
        let mut error = None;
        let label_width = 64.0;
        let field = 156.0;
        ui.add_enabled_ui(!state.selected.is_empty(), |ui| {
            let heading = if position_time.is_some() {
                tr("Position").to_owned()
            } else {
                let count = state
                    .selected
                    .iter()
                    .filter_map(|id| state.clip.key(*id))
                    .map(|(c, k)| (if !state.curves && c < 3 { 0 } else { c }, k.time.to_bits()))
                    .collect::<BTreeSet<_>>()
                    .len();
                format!("{} · {}", tr("Key"), count)
            };
            ui.label(egui::RichText::new(heading).font(theme::body_bold()));
            let row = |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.horizontal(|ui| {
                    ui.add_sized([label_width, widgets::ROW_HEIGHT], egui::Label::new(label));
                    add(ui);
                });
            };
            // Frame: moves the key, or the whole position at the playhead.
            let key_time = position_time.or(single.as_ref().map(|(_, k)| k.time));
            let mut frame = key_time.map_or(0.0, |t| t * fps);
            row(ui, tr("Frame"), &mut |ui| {
                ui.add_enabled_ui(key_time.is_some(), |ui| {
                    if widgets::spin_width(ui, &mut frame, 0.0..=12_000_000.0, 1.0, 2, field).changed() {
                        if let Some(time) = key_time {
                            let ids = state.selected.clone();
                            if let Err(e) = state.clip.move_keys(&ids, frame / fps - time, 0.0) {
                                error = Some(e);
                            }
                        }
                    }
                });
            });
            if value_rows {
                let mut value = single.as_ref().map_or(0.0, |(_, k)| k.value);
                row(ui, tr("Value"), &mut |ui| {
                    ui.add_enabled_ui(single.is_some(), |ui| {
                        if widgets::spin_width(ui, &mut value, -100000.0..=100000.0, 1.0, 3, field).changed() {
                            if let Some((_, key)) = &single {
                                let ids = state.selected.clone();
                                if let Err(e) = state.clip.move_keys(&ids, 0.0, value - key.value) {
                                    error = Some(e);
                                }
                            }
                        }
                    });
                });
            } else {
                let at = position_time.unwrap_or(state.playhead_time());
                for axis in 0..3 {
                    let mut value = state.clip.sample(axis, at);
                    row(ui, ["X", "Y", "Z"][axis], &mut |ui| {
                        ui.add_enabled_ui(position_time.is_some(), |ui| {
                            if widgets::spin_width(ui, &mut value, -100000.0..=100000.0, 1.0, 3, field).changed() {
                                let mut position: [f64; 3] = std::array::from_fn(|c| state.clip.sample(c, at));
                                position[axis] = value;
                                match state.clip.insert_position(at, position) {
                                    Ok(ids) => state.selected = ids,
                                    Err(e) => error = Some(e),
                                }
                            }
                        });
                    });
                }
            }
            // One interpolation for every selected key; blank when they differ.
            let mut modes = state.selected.iter().filter_map(|id| state.clip.key(*id).map(|(_, k)| k.interpolation));
            let shared = modes.next().filter(|first| modes.all(|m| m == *first));
            let mut index = shared.and_then(|m| INTERPOLATIONS.iter().position(|i| *i == m)).unwrap_or(usize::MAX);
            ui.horizontal(|ui| {
                ui.add_sized([label_width, widgets::ROW_HEIGHT], egui::Label::new(""));
                if widgets::dropdown(ui, "animation-interpolation", &mut index, &[tr("Linear"), tr("Hold"), tr("Bezier")], field) {
                    for id in state.selected.clone() {
                        if let Some(key) = state.clip.key_mut(id) {
                            key.interpolation = INTERPOLATIONS[index.min(2)];
                        }
                    }
                }
            });
            if value_rows {
                let (incoming_on, outgoing_on) = match &single {
                    Some((channel, key)) => {
                        let keys = &state.clip.tracks[*channel];
                        let index = keys.iter().position(|k| k.id == key.id).unwrap_or(0);
                        (
                            index > 0 && keys[index - 1].interpolation == Interpolation::Bezier,
                            key.interpolation == Interpolation::Bezier && keys.last().is_some_and(|k| k.id != key.id),
                        )
                    }
                    None => (false, false),
                };
                for (label, incoming, on) in [(tr("Incoming slope"), true, incoming_on), (tr("Outgoing slope"), false, outgoing_on)] {
                    let mut slope = single.as_ref().map_or(0.0, |(_, k)| if incoming { k.incoming } else { k.outgoing });
                    ui.label(label);
                    ui.add_enabled_ui(on, |ui| {
                        if widgets::spin_width(ui, &mut slope, -1e9..=1e9, 0.1, 3, label_width + field + 8.0).changed() {
                            if let Some(key) = single.as_ref().and_then(|(_, k)| state.clip.key_mut(k.id)) {
                                if incoming {
                                    key.incoming = slope;
                                } else {
                                    key.outgoing = slope;
                                }
                            }
                        }
                    });
                }
            }
            if button(ui, tr("Smooth tangents"), true).clicked() {
                smooth_keys(&mut state.clip, &state.selected.clone());
            }
        });
        if let Some(error) = error {
            self.status = tr_text(&error);
        }
    }

    /// Onion skin, pivot, Auto key and the pose tool.
    fn options_row(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let mut auto = editor.filters.borrow().animation_auto_key;
        let mut onion_changed = false;
        ui.horizontal_wrapped(|ui| {
            let mut f = *editor.filters.borrow();
            let mut onion = f.onion_frames > 0;
            if crate::widgets::checkbox(ui, &mut onion, tr("Onion skinning")).changed() {
                onion_changed = true;
            }
            let mut count = if f.onion_frames > 0 { f.onion_frames } else { self.onion_count } as f64;
            if widgets::spin_width(ui, &mut count, 1.0..=4.0, 1.0, 0, 100.0).changed() {
                onion_changed = true;
            }
            self.onion_count = count as u8;
            ui.label(tr("Spacing"));
            let mut spacing = f.onion_step.max(1) as f64;
            if widgets::spin_width(ui, &mut spacing, 1.0..=120.0, 1.0, 0, 110.0).changed() {
                onion_changed = true;
            }
            if onion_changed {
                f.onion_frames = if onion { self.onion_count } else { 0 };
                f.onion_step = spacing as u32;
                *editor.filters.borrow_mut() = f;
            }
            ui.add_space(12.0);
            let state = self.state.as_mut().unwrap();
            ui.label(tr("Pivot"));
            for axis in 0..3 {
                ui.label(["X", "Y", "Z"][axis]);
                widgets::spin_width(ui, &mut state.pivot[axis], -100000.0..=100000.0, 1.0, 2, 120.0);
            }
            ui.add_space(12.0);
            crate::widgets::checkbox(ui, &mut auto, tr("Auto key"));
            let mut f = editor.filters.borrow_mut();
            let mut pose = if f.animation_edit_pivot { 2 } else if f.animation_rotate { 1 } else { 0 };
            let before = pose;
            crate::widgets::radio_value(ui, &mut pose, 0, tr("Translate"));
            crate::widgets::radio_value(ui, &mut pose, 1, tr("Rotate"));
            crate::widgets::radio_value(ui, &mut pose, 2, tr("Pivot"));
            if pose != before {
                f.animation_rotate = pose == 1;
                f.animation_edit_pivot = pose == 2;
            }
        });
        if onion_changed {
            self.seek(editor);
        }
        if auto != editor.filters.borrow().animation_auto_key {
            self.set_auto_key(auto, editor);
        }
    }

    /// The shared timeline drawn and driven through egui's pointer.
    fn timeline(&mut self, ui: &mut egui::Ui, editor: &Editor, height: f32) {
        let state = self.state.as_mut().unwrap();
        let style = timeline_style();
        state.geometry = style.geometry;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click_and_drag());
        let size = (rect.width() as f64, rect.height() as f64);
        let old_frame = state.frame;
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut commit = false;
        if response.contains_pointer() {
            for (button, code) in [(egui::PointerButton::Primary, 1), (egui::PointerButton::Middle, 2)] {
                if ui.input(|i| i.pointer.button_pressed(button)) {
                    if let Some(at) = ui.input(|i| i.pointer.press_origin()).or(pointer) {
                        self.timeline.pointer = Some(at);
                        response.request_focus();
                        let extend = ui.input(|i| i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command);
                        state.begin(code, (at.x - rect.left()) as f64, (at.y - rect.top()) as f64, size.0, size.1, extend);
                    }
                }
            }
            let (scroll, control) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.ctrl || i.modifiers.command));
            if scroll != egui::Vec2::ZERO {
                state.scroll(-scroll.x as f64 / 40.0, -scroll.y as f64 / 40.0, control);
                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
            }
        }
        if let (Some(from), Some(at)) = (self.timeline.pointer, pointer) {
            state.update((at.x - from.x) as f64, (at.y - from.y) as f64, size.0, size.1);
        }
        if self.timeline.pointer.is_some() && ui.input(|i| i.pointer.any_released() || !i.pointer.any_down()) {
            commit |= state.end();
            self.timeline.pointer = None;
        }
        if ui.input(|i| !i.focused) {
            state.cancel();
            self.timeline.pointer = None;
        }
        if response.double_clicked() {
            if let Some(at) = pointer {
                match state.double_click((at.x - rect.left()) as f64, (at.y - rect.top()) as f64, size.0, size.1) {
                    Ok(changed) => commit |= changed,
                    Err(error) => self.status = tr_text(&error),
                }
            }
        }
        self.timeline.focused = response.has_focus();
        let painter = ui.painter_at(rect);
        state.draw_styled(&mut crate::egui_canvas::EguiCanvas::new(&painter, rect), size.0, size.1, &style);
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, theme::border()), egui::StrokeKind::Inside);
        let moved = state.frame != old_frame;
        response.on_hover_text(tr("Drag keys; Shift selects more. Double-click inserts a key. Ctrl+wheel zooms."));
        if moved {
            self.seek(editor);
        }
        if commit {
            self.commit(editor, false);
        }
    }
}

/// The timeline in the editor's look: a ruler with frame numbers, alternating lanes, diamond
/// keys with the accent for the selection and an accent playhead with a frame flag.
fn timeline_style() -> TimelineStyle {
    // The tokens are opaque sRGB, as the canvas takes its colours.
    let rgba = |c: egui::Color32| c.to_array().map(|v| v as f64 / 255.0);
    let alpha = |c: egui::Color32, a: f64| {
        let [r, g, b, _] = rgba(c);
        [r, g, b, a]
    };
    let dark = theme::is_dark();
    let fg = theme::fg();
    let accent = theme::accent();
    let axes = [theme::axis(0), theme::axis(1), theme::axis(2)].map(rgba);
    TimelineStyle {
        geometry: TimelineGeometry { left: 152.0, top: 24.0, pad: 12.0, right: 16.0, bottom: 0.0, curve_top: 16.0, curve_bottom: 14.0 },
        background: rgba(theme::well()),
        label_background: Some(rgba(theme::bg())),
        ruler_background: Some(rgba(theme::base())),
        track_alt: Some(alpha(fg, if dark { 0.025 } else { 0.035 })),
        divider: Some(rgba(theme::border())),
        active_row: alpha(accent, if dark { 0.16 } else { 0.12 }),
        active_row_inset: 1.0,
        active_bar: Some(rgba(accent)),
        label_text: rgba(fg),
        value_text: rgba(theme::dim()),
        values_right_aligned: true,
        font_size: 12.0,
        label_baseline: 4.0,
        swatch_top: -6.0,
        position_swatch_split: true,
        ruler_text: rgba(theme::dim()),
        ruler_font_size: 11.0,
        ruler_baseline: 16.0,
        ruler_tick: Some(alpha(theme::dim(), 0.6)),
        frame_steps: true,
        tick_spacing: 64.0,
        grid_major: alpha(fg, if dark { 0.09 } else { 0.12 }),
        grid_minor: Some(alpha(fg, if dark { 0.035 } else { 0.05 })),
        range_shade: [0.0, 0.0, 0.0, if dark { 0.28 } else { 0.07 }],
        range_shade_ruler: true,
        range_edge: Some(alpha(theme::dim(), 0.55)),
        value_grid: alpha(fg, if dark { 0.07 } else { 0.1 }),
        value_grid_text: rgba(theme::dim()),
        zero_line: Some(alpha(fg, if dark { 0.2 } else { 0.25 })),
        curve_colours: [axes[0], axes[1], axes[2], axes[0], axes[1], axes[2]],
        curve_width: 1.5,
        handle: rgba(theme::dim()),
        position_key: Some(rgba(if dark { egui::Color32::from_rgb(0xc9, 0xcd, 0xd3) } else { egui::Color32::from_rgb(0x5b, 0x61, 0x6a) })),
        key_size: 6.0,
        key_selected: rgba(theme::accent_hover()),
        key_outline: Some(rgba(if dark { theme::base() } else { egui::Color32::WHITE })),
        key_selected_outline: Some(if dark { [1.0; 4] } else { rgba(fg) }),
        selection_fill: alpha(accent, 0.14),
        selection_stroke: alpha(accent, 0.8),
        playhead: rgba(accent),
        playhead_flag: Some([1.0; 4]),
        crisp: true,
    }
}

/// The clip library's actions on the entity's named clips.
fn library_action(d: &mut editor_core::document::Document, entity: Option<map_model::node::NodeId>, action: &str, name: &str) -> Result<(), String> {
    use editor_core::animation_curves::Clip;
    if d.in_transaction() {
        return Err(tr("Finish the current drag first").into());
    }
    if action == "door" || action == "translate" {
        let mut clip = Clip { version: 1, duration: 1.0, fps: 30, tracks: std::array::from_fn(|_| vec![]) };
        let door = action == "door";
        let channel = if door { 5 } else { 0 };
        clip.insert(channel, 0.0, 0.0)?;
        clip.insert(channel, 1.0, if door { 90.0 } else { 128.0 })?;
        for key in &mut clip.tracks[channel] {
            key.interpolation = Interpolation::Bezier;
        }
        let mut library = match entity {
            Some(id) => Library::read(d, id)?.0,
            None => Library::parse("")?,
        };
        let selected = if door {
            library.clips.extend(Library::door(clip).clips);
            "open"
        } else {
            library.clips.insert("translate".into(), Entry { clip, mode: "pingpong".into() });
            "translate"
        };
        library.validate()?;
        let centre = d.selection_bounds().ok_or(tr("Select brushes first"))?.center();
        d.run_as_one_edit(|d| {
            if let Some(id) = entity.or_else(|| d.create_brush_entity("func_animate")) {
                library.write(d, id, selected);
                d.set_entity_property(id, "start_on", "0");
                let has_pivot = map_model::node::entities_with_ids(d.tree_world())
                    .iter()
                    .any(|(n, e)| *n == id && e.property("animation_pivot").is_some());
                if !has_pivot {
                    d.set_entity_property(id, "animation_pivot", &format!("{} {} {}", centre.x(), centre.y(), centre.z()));
                }
            }
        });
        return Ok(());
    }
    let id = entity.ok_or(tr("Add an animation key or choose Door animations first"))?;
    if action == "autoplay" {
        d.run_as_one_edit(|d| d.set_entity_property(id, "start_on", name));
        return Ok(());
    }
    let (mut library, current) = Library::read(d, id)?;
    if action == "select" && current == name {
        return Ok(());
    }
    let selected = library.edit(&current, action, name)?;
    d.run_as_one_edit(|d| library.write(d, id, &selected));
    Ok(())
}

fn smooth_keys(clip: &mut editor_core::animation_curves::Clip, selected: &BTreeSet<u64>) {
    for keys in &mut clip.tracks {
        let old = keys.clone();
        for (i, key) in keys.iter_mut().enumerate().filter(|(_, k)| selected.contains(&k.id)) {
            let a = &old[i.saturating_sub(1)];
            let b = &old[(i + 1).min(old.len() - 1)];
            let slope = if b.time > a.time { (b.value - a.value) / (b.time - a.time) } else { 0.0 };
            key.incoming = slope;
            key.outgoing = slope;
            key.interpolation = Interpolation::Bezier;
        }
        for i in 1..keys.len() {
            if selected.contains(&keys[i].id) {
                keys[i - 1].interpolation = Interpolation::Bezier;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The crate map, everything selected, with a position key at one second.
    fn setup() -> (egui::Context, Editor, AnimationPage) {
        let ctx = egui::Context::default();
        let editor = Editor::open(&ctx, None);
        *editor.document.borrow_mut() = editor_core::document::Document::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
            Default::default(),
        )
        .unwrap();
        editor.document.borrow_mut().select_all();
        let mut draft = Draft::read(&editor.document.borrow()).unwrap();
        draft.clip.insert_position(1.0, [64.0, 0.0, 0.0]).unwrap();
        draft.commit(&mut editor.document.borrow_mut()).unwrap();
        let mut page = AnimationPage { open: true, ..Default::default() };
        page.sync(&editor);
        assert!(page.state.is_some());
        (ctx, editor, page)
    }
    fn key(key: egui::Key, command: bool) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers { ctrl: command, ..Default::default() } }
    }
    #[test]
    fn delete_is_undoable_and_does_not_delete_brushes() {
        let (_ctx, editor, mut page) = setup();
        let before = editor.document.borrow().to_map_text();
        let count = map_model::node::brushes(editor.document.borrow().tree_world()).len();
        page.state.as_mut().unwrap().select_position_key(1.0);
        editor.filters.borrow_mut().animation_frame = Some(0);
        assert!(page.key_event(&key(egui::Key::Delete, false), &editor));
        assert!(page.state.as_ref().unwrap().clip.tracks[0].len() == 1);
        assert_eq!(map_model::node::brushes(editor.document.borrow().tree_world()).len(), count);
        editor.document.borrow_mut().undo();
        assert_eq!(editor.document.borrow().to_map_text(), before);
        editor.filters.borrow_mut().animation_frame = None;
        assert!(!page.key_event(&key(egui::Key::Delete, false), &editor));
    }
    #[test]
    fn playback_follows_the_clip_mode_and_restarts_at_the_end() {
        let (_ctx, editor, mut page) = setup();
        page.state.as_mut().unwrap().mode = "once".into();
        page.state.as_mut().unwrap().frame = 50.0;
        page.set_playing(true, &editor);
        page.advance(1.0, &editor);
        assert!(!page.playing, "once stops at the end");
        assert_eq!(page.state.as_ref().unwrap().frame, 60.0);
        assert_eq!(editor.filters.borrow().animation_frame, Some(60));
        page.set_playing(true, &editor);
        assert_eq!(page.state.as_ref().unwrap().frame, 0.0, "play at the end restarts");
        page.state.as_mut().unwrap().mode = "pingpong".into();
        page.advance(2.5, &editor);
        assert_eq!(page.state.as_ref().unwrap().frame, 60.0);
        page.advance(0.5, &editor);
        assert!(page.playing);
        assert_eq!(page.state.as_ref().unwrap().frame, 45.0, "ping-pong comes back");
        page.state.as_mut().unwrap().mode = "loop".into();
        page.state.as_mut().unwrap().frame = 55.0;
        page.direction = 1.0;
        page.advance(0.5, &editor);
        assert!((page.state.as_ref().unwrap().frame - 10.0).abs() < 1e-6);
    }
    #[test]
    fn scrubbing_previews_the_pose_without_editing_the_map() {
        let (_ctx, editor, mut page) = setup();
        let before = editor.document.borrow().to_map_text();
        assert_eq!(editor.filters.borrow().animation_frame, None);
        page.transport(1, &editor);
        assert_eq!(editor.filters.borrow().animation_frame, Some(30), "next key seeks");
        page.leave_animation_mode(&editor);
        page.transport(2, &editor);
        assert_eq!(editor.filters.borrow().animation_frame, Some(60));
        assert_eq!(editor.document.borrow().to_map_text(), before);
    }
    #[test]
    fn fps_change_keeps_the_playhead_time() {
        let (_ctx, editor, mut page) = setup();
        let state = page.state.as_mut().unwrap();
        state.frame = 30.0;
        state.set_fps(60);
        page.commit(&editor, false);
        assert_eq!(page.state.as_ref().unwrap().frame, 60.0);
        assert_eq!(editor.filters.borrow().animation_frame, Some(60));
        assert_eq!(Draft::read(&editor.document.borrow()).unwrap().clip.fps, 60);
    }
    #[test]
    fn duplicate_pastes_at_the_playhead() {
        let (_ctx, editor, mut page) = setup();
        let state = page.state.as_mut().unwrap();
        state.select_position_key(1.0);
        state.frame = 45.0;
        editor.filters.borrow_mut().animation_frame = Some(45);
        assert!(page.key_event(&key(egui::Key::D, true), &editor));
        let clip = Draft::read(&editor.document.borrow()).unwrap().clip;
        for axis in 0..3 {
            assert!(clip.tracks[axis].iter().any(|k| (k.time - 1.5).abs() < 1e-9), "axis {axis}");
        }
        assert_eq!(clip.sample(0, 1.5), 64.0);
        assert_eq!(page.state.as_ref().unwrap().selected.len(), 3);
    }
    #[test]
    fn undo_keeps_zoom_selection_and_playhead_time() {
        let (_ctx, editor, mut page) = setup();
        let state = page.state.as_mut().unwrap();
        state.select_position_key(1.0);
        state.zoom(0.5);
        state.start = 0.4;
        let (start, span, selected) = (state.start, state.span, state.selected.clone());
        state.set_fps(60);
        page.commit(&editor, false);
        editor.filters.borrow_mut().animation_frame = Some(60);
        assert!(page.key_event(&key(egui::Key::Z, true), &editor));
        let state = page.state.as_ref().unwrap();
        assert_eq!(state.clip.fps, 30, "undone");
        assert_eq!((state.start, state.span), (start, span));
        assert_eq!(state.selected, selected);
        assert_eq!(state.frame, 30.0, "same time at the old rate");
    }
    #[test]
    fn a_motion_path_click_selects_and_seeks_its_key() {
        let (_ctx, editor, mut page) = setup();
        page.playing = true;
        editor.animation_link.borrow_mut().select_key = Some(1.0);
        page.follow_views(&editor);
        let state = page.state.as_ref().unwrap();
        assert!(!page.playing);
        assert_eq!(state.frame, 30.0);
        assert_eq!(state.selected.len(), 3);
        assert_eq!(editor.filters.borrow().animation_frame, Some(30));
    }
    #[test]
    fn auto_key_enters_edit_mode_with_onion_skin_and_add_key_turns_it_on() {
        let (_ctx, editor, mut page) = setup();
        page.playing = true;
        page.set_auto_key(true, &editor);
        let f = *editor.filters.borrow();
        assert!(!page.playing && f.animation_auto_key && f.onion_frames > 0 && f.animation_frame.is_some());
        page.set_auto_key(false, &editor);
        page.state.as_mut().unwrap().frame = 15.0;
        page.add_key(&editor);
        assert!(editor.filters.borrow().animation_auto_key);
        let clip = Draft::read(&editor.document.borrow()).unwrap().clip;
        assert!(clip.tracks[0].iter().any(|k| (k.time - 0.5).abs() < 1e-9));
    }
    #[test]
    fn escape_cancels_a_timeline_drag_and_leaves_the_pose_mode_otherwise() {
        let (_ctx, editor, mut page) = setup();
        editor.filters.borrow_mut().animation_frame = Some(10);
        page.timeline.focused = true;
        assert!(page.key_event(&key(egui::Key::Escape, false), &editor));
        assert_eq!(editor.filters.borrow().animation_frame, Some(10));
        page.timeline.focused = false;
        assert!(page.key_event(&key(egui::Key::Escape, false), &editor));
        assert_eq!(editor.filters.borrow().animation_frame, None);
    }
    #[test]
    fn seeking_in_a_zoomed_timeline_uses_the_visible_time_range() {
        let (ctx, editor, mut page) = setup();
        let state = page.state.as_mut().unwrap();
        state.set_duration(4.0);
        state.start = 1.0;
        state.span = 2.0;
        let mut at = egui::Pos2::ZERO;
        for step in 0..3 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 260.0))),
                ..Default::default()
            };
            if step > 0 {
                input.events.push(egui::Event::PointerMoved(at));
                input.events.push(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: step == 1, modifiers: Default::default() });
            }
            let mut output = ctx.run_ui(input, |ui| {
                let g = timeline_style().geometry;
                at = ui.cursor().min + egui::vec2((g.left + g.pad + (ui.available_width() as f64 - g.left - g.pad - g.right) * 0.5) as f32, 12.0);
                page.timeline(ui, &editor, 220.0);
            });
            output.textures_delta.clear();
        }
        assert_eq!(page.state.as_ref().unwrap().frame, 60.0);
        assert_eq!(editor.filters.borrow().animation_frame, Some(60), "scrubbing previews");
    }
}
