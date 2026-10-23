//! A colour button that opens a picker: a saturation/value square to drag, a hue strip whose thumb wears the
//! current colour, an opacity slider (optionally shared with a panel's own Opacity), an
//! editable `#RRGGBBAA` field that turns red while it does not parse, a Copy button, and
//! R, G and B values. The button itself is a swatch over a checkerboard, so a see-through
//! colour reads as one.
//!
//! Laid out the way Godot's editor lays its picker out: the square with the hue strip
//! beside it, then a column of channel rows with their labels lined up, then the hex code.
//!
//! Any panel can use it: [`button`] edits an `[u8; 3]` and, when given one, an alpha from
//! 0 to 1. [`open`] opens a button's picker from elsewhere (the gradient rail does this
//! when a stop marker is clicked).

use editor_core::i18n::tr;
use eframe::egui::{self, Color32, CornerRadius, Id, Mesh, Pos2, Rect, Sense, Stroke, Ui};

use crate::theme;

/// Hue in degrees, saturation and value from 0 to 1, as three 0-to-1 floats of RGB.
pub fn rgb_from_hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

/// Hue in degrees, saturation and value of three 0-to-1 floats of RGB.
pub fn hsv_from_rgb([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-8 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

/// `#RRGGBBAA`, or `#RRGGBB` with no alpha.
pub fn hex(rgb: [u8; 3], alpha: Option<f32>) -> String {
    let mut text = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
    if let Some(alpha) = alpha {
        text += &format!("{:02X}", (alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    text
}

/// Reads `#RRGGBB` or `#RRGGBBAA` (the `#` optional); the alpha is `None` when not given.
pub fn parse_hex(text: &str) -> Option<([u8; 3], Option<f32>)> {
    let text = text.trim();
    let text = text.strip_prefix('#').unwrap_or(text);
    if ![6, 8].contains(&text.len()) || !text.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    let rgb = [byte(0)?, byte(2)?, byte(4)?];
    let alpha = if text.len() == 8 {
        Some(f32::from(byte(6)?) / 255.0)
    } else {
        None
    };
    Some((rgb, alpha))
}

fn to_u8(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn to_f32(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| f32::from(v) / 255.0)
}

/// A checkerboard of `cell`-sized squares behind see-through colours.
pub fn checkerboard(painter: &egui::Painter, rect: Rect, cell: f32) {
    let (dark, light) = (Color32::from_gray(97), Color32::from_gray(143));
    painter.rect_filled(rect, 0.0, dark);
    let cols = (rect.width() / cell).ceil() as i32;
    let rows = (rect.height() / cell).ceil() as i32;
    for y in 0..rows {
        for x in 0..cols {
            if (x + y) % 2 == 1 {
                let min = rect.min + egui::vec2(x as f32 * cell, y as f32 * cell);
                let square = Rect::from_min_size(min, egui::vec2(cell, cell)).intersect(rect);
                painter.rect_filled(square, 0.0, light);
            }
        }
    }
}

/// Fills `rect` with colours from `at`, sampled on a `cols` × `rows` grid of points given
/// as 0-to-1 coordinates across the rect, and blended between them.
pub fn paint_ramp(
    painter: &egui::Painter,
    rect: Rect,
    cols: usize,
    rows: usize,
    at: impl Fn(f32, f32) -> Color32,
) {
    let mut mesh = Mesh::default();
    for j in 0..=rows {
        for i in 0..=cols {
            let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
            mesh.colored_vertex(
                rect.min + egui::vec2(u * rect.width(), v * rect.height()),
                at(u, v),
            );
        }
    }
    let stride = cols as u32 + 1;
    for j in 0..rows as u32 {
        for i in 0..cols as u32 {
            let a = j * stride + i;
            mesh.add_triangle(a, a + 1, a + stride);
            mesh.add_triangle(a + 1, a + stride + 1, a + stride);
        }
    }
    painter.add(mesh);
}

fn colour(rgb: [f32; 3], alpha: f32) -> Color32 {
    let [r, g, b] = to_u8(rgb);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The id of the picker that belongs to the button `id`.
fn popup_id(id: Id) -> Id {
    id.with("colour-picker")
}

/// Opens the picker of the button made with `id`, as clicking the button would.
pub fn open(ctx: &egui::Context, id: Id) {
    egui::Popup::open_id(ctx, popup_id(id));
    ctx.request_repaint();
}

/// What the picker remembers between frames: the hue, saturation and value it last set,
/// which keep the hue while the colour is grey and the saturation while it is black, and
/// the hex text while it is being typed.
#[derive(Clone, Default)]
struct State {
    hsv: (f32, f32, f32),
    rgb: [u8; 3],
    known: bool,
    text: Option<String>,
}

/// A swatch button that opens the picker. Edits `rgb`, and `alpha` (0 to 1) when given:
/// a panel passes its own opacity here so the picker's slider and the panel's agree.
/// The response is `changed()` when the user changed the colour or the alpha.
pub fn button(ui: &mut Ui, id: Id, rgb: &mut [u8; 3], alpha: Option<&mut f32>) -> egui::Response {
    let size = egui::vec2(36.0, 24.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let mut response = ui
        .interact(rect, id, Sense::click())
        .on_hover_text(tr("Colour"));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let inner = rect.shrink(1.0);
        checkerboard(&painter, inner, 6.0);
        let a = alpha.as_deref().copied().unwrap_or(1.0);
        painter.rect_filled(inner, 0.0, colour(to_f32(*rgb), a));
        let stroke = if response.hovered() && ui.is_enabled() {
            theme::fg().gamma_multiply(0.6)
        } else {
            theme::grid()
        };
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
        if !ui.is_enabled() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), theme::bg().gamma_multiply(0.6));
        }
    }
    let shown = egui::Popup::from_toggle_button_response(&response)
        .id(popup_id(id))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| contents(ui, id, rgb, alpha))
                .inner
        });
    if shown.is_some_and(|inner| inner.inner) {
        response.mark_changed();
    }
    response
}

/// The picker itself, for a popup of the caller's own. Returns whether the colour changed.
pub(crate) fn contents(
    ui: &mut Ui,
    id: Id,
    rgb: &mut [u8; 3],
    mut alpha: Option<&mut f32>,
) -> bool {
    let state_id = id.with("picker-state");
    let mut state: State = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    // Follow a colour changed from outside, keeping the hue of a grey and the saturation
    // of a black, which the colour alone cannot say.
    if !state.known || state.rgb != *rgb {
        let (h, s, v) = hsv_from_rgb(to_f32(*rgb));
        let (old_h, old_s, _) = state.hsv;
        let known = state.known;
        state.hsv = (
            if s < 1e-6 && known { old_h } else { h },
            if v < 1e-6 && known { old_s } else { s },
            v,
        );
        state.rgb = *rgb;
        state.known = true;
    }
    // A picker is a dense tool window: rows the height of their contents.
    ui.spacing_mut().interact_size.y = 24.0;
    ui.spacing_mut().item_spacing.y = 6.0;
    let before = (*rgb, alpha.as_deref().copied());
    let (mut h, mut s, mut v) = state.hsv;
    let mut hsv_changed = false;
    let spacing = 8.0;

    // The square and the hue strip.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        let (square, response) =
            ui.allocate_exact_size(egui::vec2(220.0, 160.0), Sense::click_and_drag());
        if let Some(p) = response
            .interact_pointer_pos()
            .filter(|_| response.is_pointer_button_down_on())
        {
            s = ((p.x - square.left()) / square.width()).clamp(0.0, 1.0);
            v = (1.0 - (p.y - square.top()) / square.height()).clamp(0.0, 1.0);
            hsv_changed = true;
        }
        let painter = ui.painter_at(square);
        let hue = colour(rgb_from_hsv(h, 1.0, 1.0), 1.0);
        let mut mesh = Mesh::default();
        mesh.colored_vertex(square.left_top(), Color32::WHITE);
        mesh.colored_vertex(square.right_top(), hue);
        mesh.colored_vertex(square.right_bottom(), Color32::BLACK);
        mesh.colored_vertex(square.left_bottom(), Color32::BLACK);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(mesh);
        let at = Pos2::new(
            square.left() + s * square.width(),
            square.top() + (1.0 - v) * square.height(),
        );
        painter.circle_stroke(at, 5.0, Stroke::new(3.0, Color32::BLACK));
        painter.circle_stroke(at, 5.0, Stroke::new(1.0, Color32::WHITE));

        let (strip, response) =
            ui.allocate_exact_size(egui::vec2(16.0, 160.0), Sense::click_and_drag());
        let response = response.on_hover_text("H (°)");
        if let Some(p) = response
            .interact_pointer_pos()
            .filter(|_| response.is_pointer_button_down_on())
        {
            h = ((p.y - strip.top()) / strip.height()).clamp(0.0, 1.0) * 359.9;
            hsv_changed = true;
        }
        paint_ramp(ui.painter(), strip, 1, 12, |_, t| {
            colour(rgb_from_hsv(t * 360.0, 1.0, 1.0), 1.0)
        });
        // The thumb wears the colour being picked.
        let y = strip.top() + h / 360.0 * strip.height();
        let thumb = Rect::from_center_size(
            Pos2::new(strip.center().x, y),
            egui::vec2(strip.width() + 6.0, 8.0),
        );
        ui.painter()
            .rect_filled(thumb, CornerRadius::same(2), colour(to_f32(*rgb), 1.0));
        ui.painter().rect_stroke(
            thumb,
            CornerRadius::same(2),
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Outside,
        );
        ui.painter().rect_stroke(
            thumb.shrink(1.0),
            CornerRadius::same(2),
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Inside,
        );
    });
    if hsv_changed {
        *rgb = to_u8(rgb_from_hsv(h, s, v));
        state.hsv = (h, s, v);
        state.rgb = *rgb;
    }

    // The channel rows, labels in one column.
    let opacity_label = tr("Opacity");
    let label_width = ["R", "G", "B", opacity_label]
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(l.to_string(), theme::body(), theme::fg())
                .size()
                .x
        })
        .fold(0.0, f32::max)
        + spacing;
    let track_width = 220.0 + spacing + 16.0 - label_width - 56.0 - 2.0 * spacing;
    let mut channels_changed = false;
    for (index, label) in ["R", "G", "B"].into_iter().enumerate() {
        let mut value = rgb[index];
        let changed = channel_row(
            ui,
            label,
            None,
            label_width,
            track_width,
            &mut value,
            0.0..=255.0,
            |t| {
                let mut c = to_f32(*rgb);
                c[index] = t;
                colour(c, 1.0)
            },
        );
        if changed {
            rgb[index] = value;
            channels_changed = true;
        }
    }
    if let Some(alpha) = alpha.as_deref_mut() {
        let mut percent = (*alpha * 100.0).round().clamp(0.0, 100.0) as u8;
        let solid = to_f32(*rgb);
        if channel_row(
            ui,
            opacity_label,
            Some(opacity_label),
            label_width,
            track_width,
            &mut percent,
            0.0..=100.0,
            |t| colour(solid, t),
        ) {
            *alpha = f32::from(percent) / 100.0;
        }
    }
    if channels_changed {
        let (nh, ns, nv) = hsv_from_rgb(to_f32(*rgb));
        state.hsv = (
            if ns < 1e-6 { h } else { nh },
            if nv < 1e-6 { s } else { ns },
            nv,
        );
        state.rgb = *rgb;
    }

    // The hex code and Copy.
    ui.horizontal(|ui| {
        let current = hex(*rgb, alpha.as_deref().copied());
        let text_id = id.with("hex");
        let focused = ui.memory(|m| m.has_focus(text_id));
        let mut text = if focused {
            state.text.clone().unwrap_or_else(|| current.clone())
        } else {
            current.clone()
        };
        let valid = parse_hex(&text).is_some();
        let mut edit = egui::TextEdit::singleline(&mut text)
            .id(text_id)
            .hint_text("#RRGGBBAA")
            .char_limit(9)
            .margin(egui::vec2(6.0, 4.0))
            .min_size(egui::vec2(0.0, 24.0))
            .align(egui::Align2::LEFT_CENTER)
            .desired_width(220.0 + spacing + 16.0 - 36.0 - spacing)
            .font(egui::TextStyle::Monospace);
        if !valid {
            edit = edit.text_color(ui.visuals().error_fg_color);
        }
        let response = ui.add(edit);
        if !valid {
            ui.painter().rect_stroke(
                response.rect,
                CornerRadius::same(3),
                Stroke::new(1.0, ui.visuals().error_fg_color),
                egui::StrokeKind::Inside,
            );
        }
        if response.changed() {
            if let Some((parsed, parsed_alpha)) = parse_hex(&text) {
                *rgb = parsed;
                if let (Some(alpha), Some(parsed_alpha)) = (alpha.as_deref_mut(), parsed_alpha) {
                    *alpha = parsed_alpha;
                }
                let (nh, ns, nv) = hsv_from_rgb(to_f32(*rgb));
                state.hsv = (if ns < 1e-6 { h } else { nh }, ns, nv);
                state.rgb = *rgb;
            }
        }
        state.text = response.has_focus().then_some(text);
        let copy = ui
            .add(egui::Button::new("").min_size(egui::vec2(36.0, response.rect.height())))
            .on_hover_text(tr("Copy"));
        copy_icon(ui.painter(), copy.rect, theme::fg());
        if copy.clicked() {
            ui.ctx().copy_text(current);
        }
    });

    ui.data_mut(|d| d.insert_temp(state_id, state));
    before != (*rgb, alpha.as_deref().copied())
}

/// One channel: its label, a track showing what the channel does to the colour, and its
/// number. Returns whether the value changed.
#[allow(clippy::too_many_arguments)]
fn channel_row(
    ui: &mut Ui,
    label: &str,
    tooltip: Option<&str>,
    label_width: f32,
    track_width: f32,
    value: &mut u8,
    range: std::ops::RangeInclusive<f32>,
    ramp: impl Fn(f32) -> Color32,
) -> bool {
    let max = *range.end();
    let mut changed = false;
    ui.horizontal(|ui| {
        let (label_rect, label_response) =
            ui.allocate_exact_size(egui::vec2(label_width, 20.0), Sense::hover());
        crate::theme::text(
            ui.painter(),
            label_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            label,
            theme::body(),
            theme::dim(),
        );
        if let Some(tip) = tooltip {
            label_response.on_hover_text(tip);
        }
        let (track, response) = ui.allocate_exact_size(
            egui::vec2(track_width.max(40.0), 20.0),
            Sense::click_and_drag(),
        );
        let response = match tooltip {
            Some(tip) => response.on_hover_text(tip),
            None => response,
        };
        if let Some(p) = response
            .interact_pointer_pos()
            .filter(|_| response.is_pointer_button_down_on())
        {
            let t = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let new = (t * max).round() as u8;
            if new != *value {
                *value = new;
                changed = true;
            }
        }
        let bar = Rect::from_center_size(track.center(), egui::vec2(track.width(), 10.0));
        checkerboard(&ui.painter_at(bar), bar, 5.0);
        paint_ramp(ui.painter(), bar, 16, 1, |t, _| ramp(t));
        ui.painter().rect_stroke(
            bar,
            CornerRadius::same(2),
            Stroke::new(1.0, theme::grid()),
            egui::StrokeKind::Outside,
        );
        let x = bar.left() + f32::from(*value) / max * bar.width();
        let thumb = Rect::from_center_size(Pos2::new(x, bar.center().y), egui::vec2(6.0, 16.0));
        ui.painter()
            .rect_filled(thumb, CornerRadius::same(2), Color32::WHITE);
        ui.painter().rect_stroke(
            thumb,
            CornerRadius::same(2),
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Outside,
        );
        let mut number = f32::from(*value);
        let number_response = ui.add_sized(
            egui::vec2(56.0, 24.0),
            egui::DragValue::new(&mut number)
                .range(range.clone())
                .speed(1.0)
                .fixed_decimals(0),
        );
        if number_response.changed() {
            *value = number.round().clamp(0.0, max) as u8;
            changed = true;
        }
    });
    changed
}

/// Two overlapping sheets, the usual Copy icon.
fn copy_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.4, color);
    let back = Rect::from_min_size(c + egui::vec2(-6.0, -6.0), egui::vec2(9.0, 10.0));
    let front = back.translate(egui::vec2(3.0, 3.0));
    painter.rect_stroke(
        back,
        CornerRadius::same(1),
        stroke,
        egui::StrokeKind::Middle,
    );
    painter.rect_filled(front, CornerRadius::same(1), theme::button_fill());
    painter.rect_stroke(
        front,
        CornerRadius::same(1),
        stroke,
        egui::StrokeKind::Middle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_hsv_round_trip() {
        for text in ["#FF000080", "#12AF34FF", "#00000000", "#FFFFFF40"] {
            let (rgb, alpha) = parse_hex(text).unwrap();
            assert_eq!(hex(rgb, alpha), text);
            let (h, s, v) = hsv_from_rgb(to_f32(rgb));
            assert_eq!(to_u8(rgb_from_hsv(h, s, v)), rgb);
        }
        assert!(parse_hex("#xyzxyz").is_none());
        assert!(parse_hex("#12345").is_none());
        assert_eq!(parse_hex("123456"), Some(([0x12, 0x34, 0x56], None)));
        assert_eq!(hex([0x12, 0x34, 0x56], Some(1.0)), "#123456FF");
    }

    /// The picker opens from [`open`], showing it changes nothing, and typing a hex code
    /// into it sets the colour and the shared alpha.
    #[test]
    fn opens_and_takes_a_typed_hex_code() {
        let ctx = egui::Context::default();
        let id = Id::unique("test-picker");
        let mut rgb = [10u8, 20, 30];
        let mut alpha = 0.5f32;
        let frame = |events: Vec<egui::Event>, rgb: &mut [u8; 3], alpha: &mut f32| {
            let mut changed = false;
            let raw = egui::RawInput {
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw, |ui| {
                changed = button(ui, id, rgb, Some(alpha)).changed();
            });
            output.textures_delta.clear();
            changed
        };
        assert!(!frame(vec![], &mut rgb, &mut alpha));
        open(&ctx, id);
        assert!(
            !frame(vec![], &mut rgb, &mut alpha),
            "showing the picker changes nothing"
        );
        assert!(egui::Popup::is_id_open(&ctx, popup_id(id)));
        assert_eq!((rgb, alpha), ([10, 20, 30], 0.5));
        ctx.memory_mut(|m| m.request_focus(id.with("hex")));
        frame(vec![], &mut rgb, &mut alpha);
        let select_all = egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
        let changed = frame(
            vec![select_all, egui::Event::Text("#FF000040".into())],
            &mut rgb,
            &mut alpha,
        );
        assert!(changed);
        assert_eq!(rgb, [255, 0, 0]);
        assert!((alpha - 64.0 / 255.0).abs() < 1e-6);
        // Half a code is an error, not a colour.
        let changed = frame(
            vec![egui::Event::Key {
                key: egui::Key::Backspace,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut rgb,
            &mut alpha,
        );
        assert!(!changed);
        assert_eq!(rgb, [255, 0, 0]);
    }
}
