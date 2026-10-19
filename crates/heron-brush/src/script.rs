//! A developer's input script: replays pointer and keyboard input inside the app and saves
//! screenshots, so a change can be looked at in the real window without driving the
//! desktop. Off unless `HERON_BRUSH_SCRIPT` names a script file.
//!
//! One step per line, coordinates in points from the window's top-left:
//!
//! ```text
//! wait 0.5                 # seconds
//! move 400 300
//! click 400 300 [right|double] [shift,alt,cmd]  # modifiers held through the click
//! drag 400 300 500 320 [right|middle]
//! scroll 400 300 -3        # lines, negative scrolls down
//! key S cmd,shift          # an egui key name, optional modifiers (cmd = Ctrl off macOS)
//! text hello world
//! size 900 600             # resizes the window's inner area
//! shot name                # writes <HERON_BRUSH_SHOTS or .>/name.png
//! burst name 8             # the 3D view's next 8 pictures as name-0-3d.png … (wgpu)
//! preview                  # turns the animation preview on or off
//! tap 400 300              # a finger: down and up
//! hold 400 300             # a finger held still for 0.7 s (a long press)
//! swipe 400 300 500 320    # one finger dragged
//! pinch 400 300 100 200    # two fingers about (400, 300), from 100 apart to 200
//! pan2 400 300 450 340     # two fingers 80 apart, moved together
//! quit
//! ```
//!
//! The finger steps send touch events alone, without the mouse egui-winit makes of a
//! finger: what reaches a map view is what `touch.rs` makes of them either way.
//!
//! While a script runs, dialogs are drawn as windows inside the main one, so every one of
//! them can be clicked and lands in the screenshots.
//!
//! With the `wgpu` feature, `shot` also writes `<name>-3d.png`, the 3D view's own picture
//! read back from the renderer: on macOS a locked screen stalls Metal's presentation, so
//! the window's screenshot never arrives (it is given up after a few seconds), while the
//! view keeps drawing.

use std::collections::VecDeque;
use std::path::PathBuf;
use web_time::{Duration, Instant};

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};

enum Step {
    Wait(f32),
    Move(Pos2),
    Click(Pos2, PointerButton, bool, Modifiers),
    Drag(Pos2, Pos2, PointerButton),
    Scroll(Pos2, f32),
    Key(Key, Modifiers),
    Text(String),
    Shot(String),
    Snapshot(String),
    Burst(String, u32),
    Preview,
    Size(f32, f32),
    /// A finger event: which finger, what it did, where.
    Touch(u64, egui::TouchPhase, Pos2),
    Quit,
}

/// The finger events of a gesture, a frame each: `fingers` paths of `frames` + 1 points.
fn finger_steps(paths: &[(u64, Pos2, Pos2)], frames: usize) -> Vec<Step> {
    use egui::TouchPhase;
    let mut steps: Vec<Step> = paths.iter().map(|(id, from, _)| Step::Touch(*id, TouchPhase::Start, *from)).collect();
    for i in 1..=frames {
        let t = i as f32 / frames as f32;
        steps.extend(paths.iter().map(|(id, from, to)| Step::Touch(*id, TouchPhase::Move, from.lerp(*to, t))));
    }
    steps.extend(paths.iter().map(|(id, _, to)| Step::Touch(*id, TouchPhase::End, *to)));
    steps
}

/// A running script.
pub struct Script {
    steps: VecDeque<Step>,
    /// Events still to deliver, each in a frame of its own after `not_before`.
    queue: VecDeque<Event>,
    not_before: Instant,
    shots: PathBuf,
    /// The window screenshot asked for, and when to stop waiting for it.
    pending_shot: Option<(String, Instant)>,
}

/// Where the 3D view should write its picture, set by a `shot` step (wgpu builds).
static VIEW_SHOT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// A `burst` step's pictures still to write: where, and how many of how many.
static BURST: std::sync::Mutex<Option<(PathBuf, u32, u32)>> = std::sync::Mutex::new(None);

/// A `preview` step asked for the animation preview to be toggled.
static SNAPSHOT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Pending document evidence requested by a developer input script.
pub fn take_snapshot() -> Option<PathBuf> { SNAPSHOT.lock().ok()?.take() }

static PREVIEW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The path a `shot` or `burst` step wants the 3D view's picture written to, taken once
/// per drawn picture.
#[cfg_attr(not(feature = "wgpu"), allow(dead_code))]
pub fn take_view_shot() -> Option<PathBuf> {
    if let Some(path) = VIEW_SHOT.lock().ok()?.take() {
        return Some(path);
    }
    let mut burst = BURST.lock().ok()?;
    let (base, next, total) = burst.as_mut()?;
    let path = PathBuf::from(format!("{}-{next}-3d.png", base.display()));
    *next += 1;
    if *next >= *total {
        *burst = None;
    }
    Some(path)
}

/// Whether a `preview` step asked for the animation preview to be toggled, taken once.
pub fn take_preview_toggle() -> bool {
    PREVIEW.swap(false, std::sync::atomic::Ordering::Relaxed)
}

fn modifiers(text: &str) -> Modifiers {
    let mut m = Modifiers::NONE;
    for part in text.split(',') {
        match part.trim() {
            "cmd" => m = m | Modifiers::COMMAND,
            "ctrl" => m = m | Modifiers::CTRL,
            "shift" => m = m | Modifiers::SHIFT,
            "alt" => m = m | Modifiers::ALT,
            _ => {}
        }
    }
    m
}

fn parse(text: &str) -> Result<VecDeque<Step>, String> {
    let mut steps: VecDeque<Step> = VecDeque::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();
        let num = |i: usize| -> Result<f32, String> {
            rest.get(i).and_then(|w| w.parse().ok()).ok_or_else(|| format!("line {}: expected a number", n + 1))
        };
        let button = |w: Option<&&str>| match w.copied() {
            Some("right") => PointerButton::Secondary,
            Some("middle") => PointerButton::Middle,
            _ => PointerButton::Primary,
        };
        steps.push_back(match command {
            "wait" => Step::Wait(num(0)?),
            "move" => Step::Move(Pos2::new(num(0)?, num(1)?)),
            "click" => {
                let held = rest[2..].iter().filter(|w| !matches!(**w, "left" | "right" | "middle" | "double")).map(|w| modifiers(w)).fold(Modifiers::NONE, |a, b| a | b);
                Step::Click(Pos2::new(num(0)?, num(1)?), button(rest.get(2)), rest.get(2) == Some(&"double"), held)
            }
            "drag" => Step::Drag(Pos2::new(num(0)?, num(1)?), Pos2::new(num(2)?, num(3)?), button(rest.get(4))),
            "scroll" => Step::Scroll(Pos2::new(num(0)?, num(1)?), num(2)?),
            "key" => {
                let name = rest.first().ok_or_else(|| format!("line {}: key name", n + 1))?;
                let key = Key::from_name(name).ok_or_else(|| format!("line {}: unknown key {name}", n + 1))?;
                Step::Key(key, rest.get(1).map(|m| modifiers(m)).unwrap_or_default())
            }
            "text" => Step::Text(rest.join(" ")),
            "snapshot" => Step::Snapshot(rest.first().copied().unwrap_or("snapshot").to_string()),
            "shot" => Step::Shot(rest.first().copied().unwrap_or("shot").to_string()),
            "burst" => Step::Burst(rest.first().copied().unwrap_or("burst").to_string(), num(1)? as u32),
            "preview" => Step::Preview,
            "size" => Step::Size(num(0)?, num(1)?),
            "tap" => {
                let at = Pos2::new(num(0)?, num(1)?);
                steps.extend(finger_steps(&[(1, at, at)], 0));
                continue;
            }
            "hold" => {
                let at = Pos2::new(num(0)?, num(1)?);
                steps.extend([Step::Touch(1, egui::TouchPhase::Start, at), Step::Wait(0.7), Step::Touch(1, egui::TouchPhase::End, at)]);
                continue;
            }
            "swipe" => {
                steps.extend(finger_steps(&[(1, Pos2::new(num(0)?, num(1)?), Pos2::new(num(2)?, num(3)?))], 12));
                continue;
            }
            "pinch" => {
                let (centre, from, to) = (Pos2::new(num(0)?, num(1)?), num(2)? / 2.0, num(3)? / 2.0);
                let side = |d: f32| egui::vec2(d, 0.0);
                steps.extend(finger_steps(&[(1, centre - side(from), centre - side(to)), (2, centre + side(from), centre + side(to))], 12));
                continue;
            }
            "pan2" => {
                let (from, to) = (Pos2::new(num(0)?, num(1)?), Pos2::new(num(2)?, num(3)?));
                let side = egui::vec2(40.0, 0.0);
                steps.extend(finger_steps(&[(1, from - side, to - side), (2, from + side, to + side)], 12));
                continue;
            }
            "quit" => Step::Quit,
            other => return Err(format!("line {}: unknown step {other}", n + 1)),
        });
    }
    Ok(steps)
}

impl Script {
    /// The script `HERON_BRUSH_SCRIPT` names, if any.
    pub fn from_env() -> Option<Script> {
        let path = std::env::var_os("HERON_BRUSH_SCRIPT")?;
        let text = std::fs::read_to_string(&path).map_err(|e| eprintln!("script: {e}")).ok()?;
        let steps = parse(&text).map_err(|e| eprintln!("script: {e}")).ok()?;
        let shots = std::env::var_os("HERON_BRUSH_SHOTS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&shots);
        Some(Script { steps, queue: VecDeque::new(), not_before: Instant::now() + Duration::from_secs(2), shots, pending_shot: None })
    }

    /// Feeds this frame's share of the script into `raw`.
    pub fn feed(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        ctx.request_repaint();
        for event in &raw.events {
            if let Event::Screenshot { image, .. } = event {
                if let Some((name, _)) = self.pending_shot.take() {
                    let path = self.shots.join(format!("{name}.png"));
                    let [w, h] = image.size;
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                    match editor_core::assets::encode_png(w as u32, h as u32, &bytes).and_then(|png| std::fs::write(&path, png).map_err(|e| e.to_string())) {
                        Ok(()) => println!("script: {}", path.display()),
                        Err(e) => eprintln!("script: {e}"),
                    }
                }
            }
        }
        // Keep desktop input out, but retain responses to screenshot/clipboard requests.
        raw.events.retain(|e| matches!(e, Event::Screenshot { .. } | Event::Paste(_)));
        // The window counts as focused, or drags that cancel on focus loss never run.
        raw.focused = true;
        if let Some((name, deadline)) = &self.pending_shot {
            if Instant::now() < *deadline {
                return;
            }
            eprintln!("script: no window screenshot for {name} (is the screen locked?)");
            self.pending_shot = None;
        }
        if Instant::now() < self.not_before {
            return;
        }
        if let Some(event) = self.queue.pop_front() {
            raw.events.push(event);
            return;
        }
        let Some(step) = self.steps.pop_front() else { return };
        let pause = Duration::from_millis(30);
        let held = match &step {
            Step::Click(.., m) => *m,
            _ => Modifiers::NONE,
        };
        let press = |pos, button, pressed| Event::PointerButton { pos, button, pressed, modifiers: held };
        match step {
            Step::Snapshot(name) => { *SNAPSHOT.lock().expect("snapshot lock") = Some(self.shots.join(format!("{name}.json"))); },
            Step::Wait(s) => self.not_before = Instant::now() + Duration::from_secs_f32(s),
            Step::Move(p) => self.queue.push_back(Event::PointerMoved(p)),
            Step::Click(p, b, double, _) => {
                self.queue.extend([Event::ModifiersChanged(held), Event::PointerMoved(p), press(p, b, true), press(p, b, false)]);
                if double {
                    self.queue.extend([press(p, b, true), press(p, b, false)]);
                }
                self.queue.push_back(Event::ModifiersChanged(Modifiers::NONE));
            }
            Step::Drag(a, b, button) => {
                self.queue.extend([Event::PointerMoved(a), press(a, button, true)]);
                for i in 1..=12 {
                    self.queue.push_back(Event::PointerMoved(a + (b - a) * (i as f32 / 12.0)));
                }
                self.queue.push_back(press(b, button, false));
            }
            Step::Scroll(p, lines) => self.queue.extend([
                Event::PointerMoved(p),
                Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, lines), modifiers: Modifiers::NONE, phase: egui::TouchPhase::Move },
            ]),
            Step::Key(key, modifiers) => self.queue.extend([
                Event::ModifiersChanged(modifiers),
                Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers },
                Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers },
                Event::ModifiersChanged(Modifiers::NONE),
            ]),
            Step::Text(text) => self.queue.push_back(Event::Text(text)),
            Step::Shot(name) => {
                if cfg!(feature = "wgpu") {
                    if let Ok(mut shot) = VIEW_SHOT.lock() {
                        *shot = Some(self.shots.join(format!("{name}-3d.png")));
                    }
                }
                self.pending_shot = Some((name, Instant::now() + Duration::from_secs(4)));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            Step::Burst(name, count) => {
                if let Ok(mut burst) = BURST.lock() {
                    *burst = (count > 0).then(|| (self.shots.join(name), 0, count));
                }
            }
            Step::Preview => PREVIEW.store(true, std::sync::atomic::Ordering::Relaxed),
            Step::Touch(id, phase, pos) => {
                self.queue.push_back(Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(id), phase, pos, force: None });
            }
            Step::Size(w, h) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
                self.not_before = Instant::now() + Duration::from_millis(400);
            }
            Step::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
        self.not_before = self.not_before.max(Instant::now() + pause);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn clipboard_response_survives_script_input_isolation() {
        use super::*;
        let mut script=Script { steps:VecDeque::new(), queue:VecDeque::new(),
            not_before:Instant::now()+Duration::from_secs(1), shots:PathBuf::new(), pending_shot:None };
        let mut raw=egui::RawInput { events:vec![Event::Paste("map contents".into()),Event::PointerMoved(Pos2::ZERO)], ..Default::default() };
        script.feed(&egui::Context::default(),&mut raw);
        assert_eq!(raw.events.len(),1);
        assert!(matches!(&raw.events[0],Event::Paste(text) if text=="map contents"));
    }
    #[test]
    fn every_step_parses() {
        let steps = super::parse("wait 1\nmove 1 2\nclick 3 4 right\nclick 3 4 shift\ndrag 1 2 3 4\nscroll 1 1 -2\nkey S cmd,shift\ntext a b\nshot x # note\nburst y 4\npreview\nsize 900 600\nquit\n").unwrap();
        assert_eq!(steps.len(), 13);
        // A tap is two finger events; a pinch two fingers down, 12 frames of both, both up.
        assert_eq!(super::parse("tap 1 2").unwrap().len(), 2);
        assert_eq!(super::parse("hold 1 2").unwrap().len(), 3);
        assert_eq!(super::parse("pinch 100 100 50 100").unwrap().len(), 2 + 24 + 2);
        assert!(super::parse("bogus").is_err());
    }
}
