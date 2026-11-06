//! Modal G/R/S transforms scoped to the hovered UV canvas.
use super::*;
use editor_core::uv_view::{self, UvView};
use editor_core::uv_workflow::TransformKind;
use geometry_math::vec::Vec2f;

pub(super) struct Transform {
    kind: TransformKind,
    axis: Option<usize>,
    text: String,
    view: UvView,
    pointer: (f64, f64),
    base: Vec2f,
    value: Vec2f,
    origin: Vec2f,
    precision: bool,
    face: Option<editor_core::document::FaceRef>,
}
impl Transform {
    pub(super) fn label(&self) -> String {
        let kind = match self.kind {
            TransformKind::Move => "G",
            TransformKind::Rotate => "R",
            TransformKind::Scale => "S",
        };
        let axis = self.axis.map_or("", |k| [" X", " Y"][k]);
        let value = if self.text.is_empty() {
            match self.kind {
                TransformKind::Move => format!("{:.2}, {:.2}", self.value.x(), self.value.y()),
                _ => format!("{:.3}", self.value.x()),
            }
        } else {
            self.text.clone()
        };
        format!("{kind}{axis}  {value}   · Enter / Esc")
    }
}

pub(super) fn handle(panel: &mut UvPanel, ui: &egui::Ui, document: &Rc<RefCell<Document>>) {
    let Some((viewport, rect, id, layer)) = panel.canvas_region else {
        return;
    };
    if viewport != ui.ctx().viewport_id() {
        return;
    }
    if ui.ctx().text_edit_focused() && panel.keyboard.is_none() {
        return;
    }
    if panel.keyboard.is_some() && !ui.input(|i| i.focused) {
        panel.keyboard = None;
        panel.canvas.drag_cancel(&mut document.borrow_mut());
        return;
    }
    let at = ui
        .input(|i| i.pointer.interact_pos())
        .unwrap_or(rect.center());
    let hovered = rect.contains(at) && ui.ctx().layer_id_at(at) == Some(layer);
    let events = ui.input(|i| i.events.clone());
    if panel.keyboard.is_none() {
        if !hovered || panel.canvas.dragging() || document.borrow().in_transaction() {
            return;
        }
        let kind = events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.command && !modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
                match key {
                    egui::Key::G => Some(TransformKind::Move),
                    egui::Key::R => Some(TransformKind::Rotate),
                    egui::Key::S => Some(TransformKind::Scale),
                    _ => None,
                }
            }
            _ => None,
        });
        let Some(kind) = kind else {
            return;
        };
        let face = document.borrow().face_to_edit().map(|(id, _)| id);
        if face.is_none() {
            return;
        }
        let local = (f64::from(at.x - rect.left()), f64::from(at.y - rect.top()));
        panel.canvas.prepare(
            &document.borrow(),
            f64::from(rect.width()),
            f64::from(rect.height()),
        );
        let origin = panel
            .canvas
            .pivot(&document.borrow())
            .unwrap_or(Vec2f::zero());
        panel.canvas.begin_transform(
            &mut document.borrow_mut(),
            kind,
            f64::from(rect.height()),
            local.0,
            local.1,
        );
        let value = if kind == TransformKind::Scale {
            Vec2f::new(1.0, 1.0)
        } else {
            Vec2f::zero()
        };
        panel.keyboard = Some(Transform {
            kind,
            axis: None,
            text: String::new(),
            view: panel.canvas.view,
            pointer: local,
            base: value,
            value,
            origin,
            precision: false,
            face,
        });
        panel.gesture = None;
        ui.ctx().memory_mut(|m| m.request_focus(id));
    }
    let Some(mut session) = panel.keyboard.take() else {
        return;
    };
    let mut confirm = false;
    let mut cancel = false;
    for event in &events {
        match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => match key {
                egui::Key::Escape => cancel = true,
                egui::Key::Enter => confirm = true,
                egui::Key::X if !modifiers.command && !modifiers.ctrl => session.axis = Some(0),
                egui::Key::Y if !modifiers.command && !modifiers.ctrl => session.axis = Some(1),
                egui::Key::Backspace => {
                    session.text.pop();
                }
                egui::Key::Z if modifiers.command || modifiers.ctrl => cancel = true,
                _ => {}
            },
            egui::Event::Text(text) => {
                for ch in text.chars() {
                    if ch.is_ascii_digit() || matches!(ch, '.' | ',' | '-' | '+' | '/') {
                        session.text.push(if ch == ',' { '.' } else { ch });
                    }
                }
            }
            egui::Event::PointerButton {
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            } => confirm = true,
            egui::Event::PointerButton {
                button: egui::PointerButton::Secondary,
                pressed: true,
                ..
            } => cancel = true,
            _ => {}
        }
    }
    ui.input_mut(|i| {
        i.events.retain(|e| {
            !matches!(
                e,
                egui::Event::Key { .. }
                    | egui::Event::Text(_)
                    | egui::Event::PointerButton { pressed: true, .. }
            )
        })
    });
    panel.keyboard_frame = Some(ui.ctx().cumulative_frame_nr());
    if cancel || document.borrow().face_to_edit().map(|(id, _)| id) != session.face {
        panel.canvas.drag_cancel(&mut document.borrow_mut());
        ui.ctx().request_repaint();
        return;
    }
    let local = (f64::from(at.x - rect.left()), f64::from(at.y - rect.top()));
    let held = ui.input(|i| i.modifiers);
    if held.shift != session.precision {
        session.base = session.value;
        session.pointer = local;
        session.precision = held.shift;
    }
    let precision = if session.precision { 0.1 } else { 1.0 };
    let h = f64::from(rect.height());
    let from = session
        .view
        .unproject(&document.borrow(), h, session.pointer.0, session.pointer.1);
    let to = session
        .view
        .unproject(&document.borrow(), h, local.0, local.1);
    let mut value = match session.kind {
        TransformKind::Move => session.base + (to - from) * precision as f32,
        TransformKind::Rotate => {
            let before = uv_view::angle_between(session.origin, from).unwrap_or(0.0);
            let now = uv_view::angle_between(session.origin, to).unwrap_or(before);
            Vec2f::new(
                session.base.x() + uv_view::wrap_180(now - before) * precision as f32,
                0.0,
            )
        }
        TransformKind::Scale => {
            let a = from - session.origin;
            let b = to - session.origin;
            let length = a.x() * a.x() + a.y() * a.y();
            let ratio = if length > 1e-5 {
                (a.x() * b.x() + a.y() * b.y()) / length
            } else {
                1.0 + ((local.0 - session.pointer.0) / 80.0) as f32
            };
            let ratio = 1.0 + (ratio - 1.0) * precision as f32;
            session.base * ratio
        }
    };
    let numeric = if session.text.is_empty() {
        None
    } else {
        let inverse = session.text.ends_with('/');
        session
            .text
            .trim_end_matches('/')
            .parse::<f32>()
            .ok()
            .and_then(|v| {
                let v = if inverse { 1.0 / v } else { v };
                v.is_finite().then_some(v)
            })
    };
    if let Some(number) = numeric {
        value = match session.kind {
            TransformKind::Move => {
                if session.axis == Some(1) {
                    Vec2f::new(0.0, number)
                } else {
                    Vec2f::new(number, 0.0)
                }
            }
            TransformKind::Rotate => Vec2f::new(number, 0.0),
            TransformKind::Scale => Vec2f::new(number, number),
        };
    }
    let axes = match session.axis {
        Some(0) => [true, false],
        Some(1) => [false, true],
        _ => [true, true],
    };
    if session.text.is_empty() || numeric.is_some() {
        panel.canvas.preview_transform(
            &mut document.borrow_mut(),
            session.kind,
            value,
            axes,
            numeric.is_some() || held.command || held.ctrl,
        );
        session.value = value;
    }
    if confirm {
        panel.canvas.drag_end(&mut document.borrow_mut());
    } else {
        panel.keyboard = Some(session);
    }
    ui.ctx().request_repaint();
}
