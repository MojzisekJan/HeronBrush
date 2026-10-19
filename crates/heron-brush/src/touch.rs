//! Touch gestures over the map views.
//!
//! egui-winit turns the first finger into a mouse with its left button down, which
//! would make every touch in a view a click there: a one-finger orbit would select what
//! it started on, and a pinch would drag. So before egui sees a frame's input
//! ([`Gestures::filter`], from the app's `raw_input_hook`), a touch that lands in a map
//! view is taken out of the mouse stream and read as a gesture instead:
//!
//! - a tap is a click (press and release where the finger went down);
//! - a long press (half a second without moving) is a right-click, which opens the
//!   context menu;
//! - one finger dragging orbits the 3D view, and in a 2D view is a left-button drag
//!   (moving the selection, drawing a brush, a marquee), as with a mouse;
//! - two fingers pan and pinch-zoom either kind of view. A second finger during a 2D
//!   drag cancels the drag rather than committing it.
//!
//! A tablet has no Shift, Ctrl or Alt, and the editor leans on them (Shift-drag resizes a
//! brush, Alt-click copies a material…), so [`modifier_strip`] puts the three keys over
//! the views as toggles. While one is on, every gesture carries it, and one finger in the
//! 3D view drags as a mouse does instead of orbiting, so a face can be taken hold of.
//!
//! Orbit, pan and zoom are not mouse gestures, so they reach the views as a [`Motion`]
//! they take with [`take_motion`]. Touches anywhere else (the inspector, menus, dialogs)
//! go on to egui untouched.

use std::collections::BTreeMap;

use eframe::egui::{self, Event, PointerButton, Pos2, Rect, TouchPhase, Vec2};

/// How far a finger may wander and still be a tap or a long press, in points.
const SLOP: f32 = 10.0;
/// How long a still finger takes to become a right-click, in seconds.
const LONG_PRESS: f64 = 0.5;

/// Whether the editor is driven by touch: on Android, with `HERON_BRUSH_TOUCH=1`, or once
/// a touch has been seen. Larger hit areas and the touch layout follow it.
pub fn enabled(ctx: &egui::Context) -> bool {
    static FORCED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let forced = *FORCED.get_or_init(|| cfg!(target_os = "android") || std::env::var_os("HERON_BRUSH_TOUCH").is_some_and(|v| !v.is_empty() && v != "0"));
    forced || ctx.data(|d| d.get_temp::<bool>(seen_id()).unwrap_or(false))
}

fn seen_id() -> egui::Id {
    egui::Id::new("touch-seen")
}

fn views_id() -> egui::Id {
    egui::Id::new("touch-views")
}

fn motion_id() -> egui::Id {
    egui::Id::new("touch-motion")
}

/// Tells the gestures where the map views are this frame, by view number (0 the 3D view).
pub fn set_views(ctx: &egui::Context, views: &[(usize, Rect)]) {
    ctx.data_mut(|d| d.insert_temp(views_id(), views.to_vec()));
}

fn modifiers_id() -> egui::Id {
    egui::Id::new("touch-modifiers")
}

fn strip_id() -> egui::Id {
    egui::Id::new("touch-modifier-strip")
}

/// The keys the strip holds down.
pub fn held_modifiers(ctx: &egui::Context) -> egui::Modifiers {
    ctx.data(|d| d.get_temp::<egui::Modifiers>(modifiers_id())).unwrap_or_default()
}

/// Shift, Ctrl and Alt as toggles in the bottom-left corner of `area` (the map views),
/// on a touch screen only. A key stays held until it is tapped again.
pub fn modifier_strip(ctx: &egui::Context, area: Rect) {
    if !enabled(ctx) {
        return;
    }
    let mut held = held_modifiers(ctx);
    let response = egui::Area::new(strip_id())
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(area.left() + 10.0, area.bottom() - 58.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(crate::theme::panel())
                .stroke(egui::Stroke::new(1.0, crate::theme::border()))
                .corner_radius(8)
                .inner_margin(4)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (label, key) in [("Shift", &mut held.shift), ("Ctrl", &mut held.ctrl), ("Alt", &mut held.alt)] {
                            let button = egui::Button::new(egui::RichText::new(label).size(15.0))
                                .min_size(egui::vec2(64.0, 40.0))
                                .selected(*key);
                            if ui.add(button).clicked() {
                                *key = !*key;
                            }
                        }
                    });
                });
        })
        .response;
    held.command = held.ctrl;
    ctx.data_mut(|d| {
        d.insert_temp(modifiers_id(), held);
        d.insert_temp(strip_id(), response.rect);
    });
}

/// What two fingers (or one, orbiting) did to a view this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    /// One finger's movement, in points: the 3D view orbits by it.
    pub orbit: Vec2,
    /// The two fingers' centre's movement, in points.
    pub pan: Vec2,
    /// How much further apart the fingers are than last frame (1 is no change).
    pub zoom: f32,
    /// The fingers' centre, where a 2D view zooms about.
    pub centre: Pos2,
    /// Whether this is the gesture's first motion, for a view that takes its pivot then.
    pub started: bool,
}

/// The motion for view `view` this frame, if a gesture moved it. Taking it clears it.
pub fn take_motion(ctx: &egui::Context, view: usize) -> Option<Motion> {
    ctx.data_mut(|d| {
        let motions = d.get_temp_mut_or_default::<Vec<(usize, Motion)>>(motion_id());
        let at = motions.iter().position(|(v, _)| *v == view)?;
        Some(motions.remove(at).1)
    })
}

fn add_motion(ctx: &egui::Context, view: usize, add: Motion) {
    ctx.data_mut(|d| {
        let motions = d.get_temp_mut_or_default::<Vec<(usize, Motion)>>(motion_id());
        match motions.iter_mut().find(|(v, _)| *v == view) {
            Some((_, motion)) => {
                motion.orbit += add.orbit;
                motion.pan += add.pan;
                motion.zoom *= add.zoom;
                motion.centre = add.centre;
                motion.started |= add.started;
            }
            None => motions.push((view, add)),
        }
    });
}

/// What the fingers are doing.
#[derive(Clone, Debug, PartialEq)]
enum State {
    /// No touch in a view.
    Idle,
    /// One finger down in `view`, not yet a tap, a long press or a drag.
    Pending { finger: u64, view: usize, start: Pos2, at: Pos2, since: f64 },
    /// One finger dragging: a left-button drag in a 2D view, an orbit in the 3D view.
    Drag { finger: u64, view: usize, at: Pos2 },
    /// Two fingers panning and pinching.
    Pinch { view: usize, centre: Pos2, distance: f32 },
    /// The gesture is over (a long press fired, a pinch lost a finger); the rest of the
    /// touch is ignored until every finger is up.
    Spent,
}

/// The touch gestures, fed each frame's raw input.
pub struct Gestures {
    state: State,
    /// Where each finger down is.
    fingers: BTreeMap<u64, Pos2>,
    /// Fingers that came down outside the views, whose touch egui handles itself.
    passed: std::collections::BTreeSet<u64>,
    /// Events for the next frame: the pointer leaving after a tap or a drag, which in the
    /// same frame would leave the release without a position.
    next: Vec<Event>,
    /// A frame that must look unfocused to the views, so a drag cancels.
    unfocus: bool,
    /// The strip's keys as egui last heard of them.
    sent: egui::Modifiers,
}

impl Default for Gestures {
    fn default() -> Self {
        Self { state: State::Idle, fingers: BTreeMap::new(), passed: Default::default(), next: Vec::new(), unfocus: false, sent: egui::Modifiers::NONE }
    }
}

impl Gestures {
    /// Reads the touches of a frame's input before egui does: those in the map views
    /// become gestures, with their emulated mouse events replaced; the rest pass.
    pub fn filter(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let views = ctx.data(|d| d.get_temp::<Vec<(usize, Rect)>>(views_id())).unwrap_or_default();
        let now = raw.time.unwrap_or(0.0);
        // Fingers hold only what the strip holds; a keyboard's own reach egui as ever.
        let modifiers = held_modifiers(ctx);
        // Said again every frame a key is held, since anything else that reports the
        // keyboard (a real one, the dev script) would otherwise let go of it.
        if modifiers.any() || modifiers != self.sent {
            raw.events.push(Event::ModifiersChanged(modifiers));
            self.sent = modifiers;
        }
        let strip = ctx.data(|d| d.get_temp::<Rect>(strip_id()));
        let mut out = std::mem::take(&mut self.next);
        self.unfocus = false;
        let mut motions = Vec::new();
        // egui-winit follows each touch event with the mouse events it makes of it, so
        // those after the touch that ended a gesture are still that gesture's.
        let mut ended_gesture = false;
        for event in std::mem::take(&mut raw.events) {
            match event {
                Event::Touch { id, phase, pos, .. } => {
                    ctx.data_mut(|d| d.insert_temp(seen_id(), true));
                    out.push(event.clone());
                    let before = self.state != State::Idle;
                    // A finger on the strip is the strip's, not a view's.
                    let on_strip = strip.is_some_and(|strip| strip.contains(pos));
                    if on_strip && phase == TouchPhase::Start {
                        self.passed.insert(id.0);
                    }
                    self.touch(id.0, phase, pos, now, &views, modifiers, &mut out, &mut motions);
                    ended_gesture = before && self.state == State::Idle;
                }
                // The mouse egui-winit makes of a finger in a view is replaced by the
                // gesture's own.
                Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone if self.state != State::Idle || ended_gesture => {}
                other => out.push(other),
            }
        }
        self.long_press(ctx, now, modifiers, &mut out);
        raw.events = out;
        if self.unfocus {
            raw.focused = false;
        }
        for (view, motion) in motions {
            add_motion(ctx, view, motion);
        }
        if !self.next.is_empty() || self.unfocus {
            ctx.request_repaint();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn touch(&mut self, finger: u64, phase: TouchPhase, pos: Pos2, now: f64, views: &[(usize, Rect)], modifiers: egui::Modifiers, out: &mut Vec<Event>, motions: &mut Vec<(usize, Motion)>) {
        match phase {
            TouchPhase::Start => {
                self.fingers.insert(finger, pos);
                match self.state.clone() {
                    State::Idle => {
                        let view = views.iter().find(|(_, rect)| rect.contains(pos)).map(|(view, _)| *view);
                        match view {
                            Some(view) if self.passed.is_empty() && !self.passed.contains(&finger) => self.state = State::Pending { finger, view, start: pos, at: pos, since: now },
                            _ => {
                                self.passed.insert(finger);
                            }
                        }
                    }
                    State::Pending { view, .. } => self.state = self.pinch(view, motions),
                    State::Drag { view, at, .. } => {
                        if view != 0 || modifiers.any() {
                            // The drag so far is undone: the views cancel a drag when the
                            // window loses focus, and the release then does nothing.
                            self.unfocus = true;
                            out.push(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers });
                            self.next.push(Event::PointerGone);
                        }
                        self.state = self.pinch(view, motions);
                    }
                    State::Pinch { .. } | State::Spent => {}
                }
            }
            TouchPhase::Move => {
                if let Some(at) = self.fingers.get_mut(&finger) {
                    *at = pos;
                }
                match self.state.clone() {
                    State::Pending { finger: first, view, start, since, .. } if first == finger => {
                        if pos.distance(start) > SLOP {
                            self.state = State::Drag { finger, view, at: pos };
                            if view == 0 && !modifiers.any() {
                                motions.push((view, Motion { orbit: pos - start, zoom: 1.0, centre: pos, started: true, ..Default::default() }));
                            } else {
                                out.push(Event::PointerMoved(start));
                                out.push(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers });
                                out.push(Event::PointerMoved(pos));
                            }
                        } else {
                            self.state = State::Pending { finger, view, start, at: pos, since };
                        }
                    }
                    State::Drag { finger: first, view, at } if first == finger => {
                        if view == 0 && !modifiers.any() {
                            motions.push((view, Motion { orbit: pos - at, zoom: 1.0, centre: pos, ..Default::default() }));
                        } else {
                            out.push(Event::PointerMoved(pos));
                        }
                        self.state = State::Drag { finger, view, at: pos };
                    }
                    State::Pinch { view, centre, distance } => {
                        if let Some((now_centre, now_distance)) = self.two_fingers() {
                            let zoom = if distance > 1.0 { now_distance / distance } else { 1.0 };
                            motions.push((view, Motion { pan: now_centre - centre, zoom, centre: now_centre, ..Default::default() }));
                            self.state = State::Pinch { view, centre: now_centre, distance: now_distance };
                        }
                    }
                    _ => {}
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                self.fingers.remove(&finger);
                self.passed.remove(&finger);
                let cancelled = phase == TouchPhase::Cancel;
                match self.state.clone() {
                    State::Pending { finger: first, start, .. } if first == finger => {
                        if !cancelled {
                            out.push(Event::PointerMoved(start));
                            out.push(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers });
                            out.push(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: false, modifiers });
                            self.next.push(Event::PointerGone);
                        }
                        self.state = State::Spent;
                    }
                    State::Drag { finger: first, view, at } if first == finger => {
                        if view != 0 || modifiers.any() {
                            if cancelled {
                                self.unfocus = true;
                            }
                            out.push(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers });
                            self.next.push(Event::PointerGone);
                        }
                        self.state = State::Spent;
                    }
                    State::Pinch { .. } => self.state = State::Spent,
                    _ => {}
                }
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                    self.passed.clear();
                }
            }
        }
    }

    /// Two fingers in `view`: a pinch from where they are now, which the view hears of
    /// at once (a motion of nothing that starts the gesture).
    fn pinch(&self, view: usize, motions: &mut Vec<(usize, Motion)>) -> State {
        match self.two_fingers() {
            Some((centre, distance)) => {
                motions.push((view, Motion { zoom: 1.0, centre, started: true, ..Default::default() }));
                State::Pinch { view, centre, distance }
            }
            None => State::Spent,
        }
    }

    /// The first two fingers' centre and distance apart.
    fn two_fingers(&self) -> Option<(Pos2, f32)> {
        let mut fingers = self.fingers.values();
        let (a, b) = (*fingers.next()?, *fingers.next()?);
        Some((a.lerp(b, 0.5), a.distance(b)))
    }

    /// A finger held still for long enough is a right-click where it is; until then the
    /// frame after the wait is asked for, since nothing else may come to wake egui.
    fn long_press(&mut self, ctx: &egui::Context, now: f64, modifiers: egui::Modifiers, out: &mut Vec<Event>) {
        let State::Pending { at, since, .. } = self.state else { return };
        let held = now - since;
        if held >= LONG_PRESS {
            out.push(Event::PointerMoved(at));
            out.push(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed: true, modifiers });
            out.push(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed: false, modifiers });
            self.next.push(Event::PointerGone);
            self.state = State::Spent;
        } else {
            ctx.request_repaint_after(web_time::Duration::from_secs_f64(LONG_PRESS - held + 0.01));
        }
    }
}

/// Larger hit areas and handles for fingers: egui's interaction radius and scroll bars.
/// The editor's own controls are sized for fingers on Android only (theme::CONTROL_HEIGHT);
/// see docs/android.md.
pub fn style(ctx: &egui::Context) {
    if !enabled(ctx) {
        return;
    }
    let done = egui::Id::new("touch-style");
    if ctx.data(|d| d.get_temp::<bool>(done).unwrap_or(false)) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
    ctx.all_styles_mut(|style| {
        style.interaction.interact_radius = 12.0;
        style.spacing.scroll.bar_width = 12.0;
        style.spacing.scroll.floating = false;
        style.spacing.combo_height = 320.0;
        // Fingers rest; a tooltip would cover what they are about to press.
        style.interaction.tooltip_delay = 1.0;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: u64, phase: TouchPhase, x: f32, y: f32) -> Event {
        Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(id), phase, pos: egui::pos2(x, y), force: None }
    }

    /// Runs one frame of `events` (as egui-winit sends them, mouse emulation and all)
    /// through the gestures at `time`; returns what egui would see.
    fn frame(ctx: &egui::Context, gestures: &mut Gestures, time: f64, events: Vec<Event>) -> Vec<Event> {
        let mut raw = egui::RawInput { time: Some(time), events, focused: true, ..Default::default() };
        gestures.filter(ctx, &mut raw);
        raw.events.into_iter().filter(|e| !matches!(e, Event::Touch { .. })).collect()
    }

    fn setup() -> (egui::Context, Gestures) {
        let ctx = egui::Context::default();
        set_views(&ctx, &[(0, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0))), (1, Rect::from_min_max(egui::pos2(400.0, 0.0), egui::pos2(800.0, 300.0)))]);
        (ctx, Gestures::default())
    }

    fn primary(pressed: bool, x: f32, y: f32) -> Event {
        Event::PointerButton { pos: egui::pos2(x, y), button: PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    #[test]
    fn a_tap_in_a_view_is_a_click_where_the_finger_went_down() {
        let (ctx, mut gestures) = setup();
        let down = frame(&ctx, &mut gestures, 0.0, vec![touch(1, TouchPhase::Start, 100.0, 100.0), Event::PointerMoved(egui::pos2(100.0, 100.0)), primary(true, 100.0, 100.0)]);
        assert!(down.is_empty(), "nothing yet: it may become a drag or a long press");
        let up = frame(&ctx, &mut gestures, 0.1, vec![touch(1, TouchPhase::Move, 103.0, 101.0), touch(1, TouchPhase::End, 103.0, 101.0), primary(false, 103.0, 101.0), Event::PointerGone]);
        assert_eq!(up, vec![Event::PointerMoved(egui::pos2(100.0, 100.0)), primary(true, 100.0, 100.0), primary(false, 100.0, 100.0)]);
        assert_eq!(frame(&ctx, &mut gestures, 0.2, vec![]), vec![Event::PointerGone]);
        assert_eq!(gestures.state, State::Idle);
    }

    #[test]
    fn a_long_press_is_a_right_click_and_nothing_else() {
        let (ctx, mut gestures) = setup();
        frame(&ctx, &mut gestures, 0.0, vec![touch(1, TouchPhase::Start, 500.0, 100.0)]);
        assert!(frame(&ctx, &mut gestures, 0.3, vec![]).is_empty());
        let held = frame(&ctx, &mut gestures, 0.6, vec![]);
        let secondary: Vec<bool> = held.iter().filter_map(|e| match e {
            Event::PointerButton { button: PointerButton::Secondary, pressed, .. } => Some(*pressed),
            _ => None,
        }).collect();
        assert_eq!(secondary, [true, false]);
        frame(&ctx, &mut gestures, 0.65, vec![]);
        let lifted = frame(&ctx, &mut gestures, 0.9, vec![touch(1, TouchPhase::End, 500.0, 100.0), primary(false, 500.0, 100.0)]);
        assert!(lifted.is_empty(), "lifting after a long press clicks nothing: {lifted:?}");
    }

    #[test]
    fn one_finger_orbits_the_3d_view_without_clicking() {
        let (ctx, mut gestures) = setup();
        frame(&ctx, &mut gestures, 0.0, vec![touch(1, TouchPhase::Start, 100.0, 100.0), primary(true, 100.0, 100.0)]);
        let moved = frame(&ctx, &mut gestures, 0.05, vec![touch(1, TouchPhase::Move, 130.0, 100.0), Event::PointerMoved(egui::pos2(130.0, 100.0))]);
        assert!(moved.is_empty());
        let motion = take_motion(&ctx, 0).expect("an orbit");
        assert_eq!(motion.orbit, egui::vec2(30.0, 0.0));
        assert!(motion.started);
        frame(&ctx, &mut gestures, 0.1, vec![touch(1, TouchPhase::Move, 140.0, 90.0)]);
        assert_eq!(take_motion(&ctx, 0).map(|m| (m.orbit, m.started)), Some((egui::vec2(10.0, -10.0), false)));
        let up = frame(&ctx, &mut gestures, 0.2, vec![touch(1, TouchPhase::End, 140.0, 90.0), primary(false, 140.0, 90.0)]);
        assert!(up.is_empty(), "{up:?}");
    }

    #[test]
    fn one_finger_in_a_2d_view_drags_like_the_left_button() {
        let (ctx, mut gestures) = setup();
        frame(&ctx, &mut gestures, 0.0, vec![touch(1, TouchPhase::Start, 500.0, 100.0)]);
        let moved = frame(&ctx, &mut gestures, 0.05, vec![touch(1, TouchPhase::Move, 540.0, 100.0)]);
        assert_eq!(moved, vec![Event::PointerMoved(egui::pos2(500.0, 100.0)), primary(true, 500.0, 100.0), Event::PointerMoved(egui::pos2(540.0, 100.0))]);
        let up = frame(&ctx, &mut gestures, 0.1, vec![touch(1, TouchPhase::End, 540.0, 100.0)]);
        assert_eq!(up, vec![primary(false, 540.0, 100.0)]);
    }

    #[test]
    fn a_second_finger_cancels_a_2d_drag_and_pinches() {
        let (ctx, mut gestures) = setup();
        frame(&ctx, &mut gestures, 0.0, vec![touch(1, TouchPhase::Start, 500.0, 100.0)]);
        frame(&ctx, &mut gestures, 0.05, vec![touch(1, TouchPhase::Move, 540.0, 100.0)]);
        let mut raw = egui::RawInput { time: Some(0.1), events: vec![touch(2, TouchPhase::Start, 640.0, 100.0)], focused: true, ..Default::default() };
        gestures.filter(&ctx, &mut raw);
        assert!(!raw.focused, "the views cancel the drag on a frame without focus");
        let mut raw = egui::RawInput { time: Some(0.15), events: vec![touch(2, TouchPhase::Move, 740.0, 100.0)], focused: true, ..Default::default() };
        gestures.filter(&ctx, &mut raw);
        assert!(raw.focused);
        let motion = take_motion(&ctx, 1).expect("a pinch");
        assert_eq!(motion.zoom, 2.0);
        assert_eq!(motion.pan, egui::vec2(50.0, 0.0));
    }

    /// The whole way from fingers to a flat view of the sample crate: a tap selects, a
    /// one-finger drag moves the selection as one undo step, a second finger cancels a
    /// drag, a pinch zooms without touching the map, and a long press opens the menu.
    #[test]
    fn fingers_drive_a_flat_view() {
        editor_core::preferences::set_current(Default::default());
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = crate::editor::Editor::open(&ctx, None);
        *editor.document.borrow_mut() = editor_core::document::Document::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"), Default::default()).unwrap();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut pane = crate::view2d::Pane2d::new(&editor, editor_core::camera2d::ViewPlane::Top);
        let mut gestures = Gestures::default();
        let mut time = 0.0;
        let mut run = |pane: &mut crate::view2d::Pane2d, events: Vec<Event>, seconds: f64| {
            time += seconds;
            set_views(&ctx, &[(1, rect)]);
            let mut raw = egui::RawInput { screen_rect: Some(rect), time: Some(time), events, focused: true, ..Default::default() };
            gestures.filter(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| pane.show(ui, rect));
            output.textures_delta.clear();
        };
        run(&mut pane, vec![], 0.0);
        run(&mut pane, vec![], 0.016);
        let centre = editor.document.borrow().bounds().unwrap().center();
        let on_crate = pane.flat.state.borrow().camera.world_to_screen(centre, (800.0, 600.0));
        let (x, y) = (on_crate.x() as f32, on_crate.y() as f32);
        let text = || editor.document.borrow().to_map_text();
        let original = text();

        run(&mut pane, vec![touch(1, TouchPhase::Start, x, y)], 0.016);
        run(&mut pane, vec![touch(1, TouchPhase::End, x, y)], 0.08);
        run(&mut pane, vec![], 0.016);
        assert!(editor.document.borrow().has_selection(), "a tap on the crate selects");
        assert_eq!(text(), original, "a tap moves nothing");

        run(&mut pane, vec![touch(1, TouchPhase::Start, x, y)], 0.016);
        for step in 1..=6 {
            run(&mut pane, vec![touch(1, TouchPhase::Move, x + 40.0 * step as f32, y)], 0.016);
        }
        run(&mut pane, vec![touch(1, TouchPhase::End, x + 240.0, y)], 0.016);
        run(&mut pane, vec![], 0.016);
        let moved = text();
        assert_ne!(moved, original, "a one-finger drag moves the selection");
        editor.document.borrow_mut().undo();
        assert_eq!(text(), original, "and is one undo step");

        run(&mut pane, vec![touch(1, TouchPhase::Start, x, y)], 0.016);
        for step in 1..=4 {
            run(&mut pane, vec![touch(1, TouchPhase::Move, x + 10.0 * step as f32, y)], 0.016);
        }
        let zoom = pane.flat.state.borrow().camera.zoom;
        run(&mut pane, vec![touch(2, TouchPhase::Start, x + 140.0, y)], 0.016);
        run(&mut pane, vec![], 0.016);
        assert_eq!(text(), original, "a second finger takes the drag back");
        run(&mut pane, vec![touch(2, TouchPhase::Move, x + 240.0, y)], 0.016);
        run(&mut pane, vec![touch(1, TouchPhase::End, x + 40.0, y), touch(2, TouchPhase::End, x + 240.0, y)], 0.016);
        run(&mut pane, vec![], 0.016);
        assert_eq!(text(), original, "a pinch edits nothing");
        let zoomed = pane.flat.state.borrow().camera.zoom;
        assert!((zoomed / zoom - 2.0).abs() < 0.01, "the fingers twice as far apart zoom twice as far in: {zoom} -> {zoomed}");

        assert!(pane.context_click.is_none());
        run(&mut pane, vec![touch(1, TouchPhase::Start, x, y)], 0.016);
        run(&mut pane, vec![], 0.3);
        run(&mut pane, vec![], 0.3);
        assert!(pane.context_click.take().is_some(), "a long press is a right-click");
        run(&mut pane, vec![touch(1, TouchPhase::End, x, y)], 0.016);
        run(&mut pane, vec![], 0.016);
        assert!(pane.context_click.is_none());
    }

    #[test]
    fn touches_outside_the_views_reach_egui_as_they_are() {
        let (ctx, mut gestures) = setup();
        let events = vec![touch(1, TouchPhase::Start, 100.0, 500.0), Event::PointerMoved(egui::pos2(100.0, 500.0)), primary(true, 100.0, 500.0)];
        let seen = frame(&ctx, &mut gestures, 0.0, events);
        assert_eq!(seen, vec![Event::PointerMoved(egui::pos2(100.0, 500.0)), primary(true, 100.0, 500.0)]);
        // A second finger in a view while the first is in the inspector is no gesture.
        frame(&ctx, &mut gestures, 0.1, vec![touch(2, TouchPhase::Start, 100.0, 100.0)]);
        assert_eq!(gestures.state, State::Idle);
    }
}
