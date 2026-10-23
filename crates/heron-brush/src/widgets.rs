//! Small editing controls the panels share.

use eframe::egui;

/// A text field that commits on Enter or on leaving it, so
/// an edit reaches the history once rather than once per letter, and drops the edit on
/// Escape. Returns the new text when it was committed and differs from `value`.
pub fn field(
    ui: &mut egui::Ui,
    id: egui::Id,
    value: &str,
    hint: &str,
    width: f32,
) -> Option<String> {
    let mut text = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| value.to_string());
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .hint_text(hint)
            .desired_width(width)
            .margin(crate::widgets::body_entry_margin(ui, 6, 6))
            .min_size(egui::vec2(0.0, row_height(ui))),
    );
    underline(ui, &response);
    if response.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text));
        None
    } else if response.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(id));
        let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
        (!escaped && text != value).then_some(text)
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
        None
    }
}

/// Whether the keyboard is what the user is moving around with: true from a Tab or an
/// arrow key until the next press of the pointer. Focus is drawn only then, so a click
/// does not leave a ring behind, as the web's `:focus-visible`.
pub fn keyboard_navigating(ctx: &egui::Context) -> bool {
    let id = egui::Id::unique("keyboard-navigating");
    let (key, pointer) = ctx.input(|i| {
        let key = i.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Tab
                        | egui::Key::ArrowUp
                        | egui::Key::ArrowDown
                        | egui::Key::ArrowLeft
                        | egui::Key::ArrowRight
                        | egui::Key::Home
                        | egui::Key::End,
                    pressed: true,
                    ..
                }
            )
        });
        (key, i.pointer.any_pressed())
    });
    let mut on = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if key {
        on = true;
    }
    if pointer {
        on = false;
    }
    ctx.data_mut(|d| d.insert_temp(id, on));
    on
}

/// A list row's keyboard focus: a click gives it the keyboard, so the arrows of
/// [`list_keys`] work from there, and while the keyboard is in use the row with it is
/// ringed in the accent.
pub fn row_focus(ui: &egui::Ui, response: &egui::Response) {
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus() && keyboard_navigating(ui.ctx()) {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            CornerRadius::same(theme::radius()),
            Stroke::new(2.0, theme::accent()),
            egui::StrokeKind::Inside,
        );
    }
}

/// Keyboard for a list where one row is picked, such as the manual's chapters: while one
/// of `rows` has the keyboard, Up and Down move to the row before or after, and Home and
/// End to the first and last, taking the keyboard along. Returns the row moved to, for the
/// caller to pick. egui's own arrow navigation is held back, or it would move the keyboard
/// a second time.
pub fn list_keys(ui: &egui::Ui, rows: &[egui::Response]) -> Option<usize> {
    keys_along(ui, rows, egui::Key::ArrowUp, egui::Key::ArrowDown)
}

/// [`list_keys`] for a row of tabs: Left and Right move along it.
pub fn tab_keys(ui: &egui::Ui, tabs: &[egui::Response]) -> Option<usize> {
    keys_along(ui, tabs, egui::Key::ArrowLeft, egui::Key::ArrowRight)
}

/// [`list_keys`] with `back` and `on` for the keys that move to the previous and next.
fn keys_along(
    ui: &egui::Ui,
    rows: &[egui::Response],
    back: egui::Key,
    on: egui::Key,
) -> Option<usize> {
    let focused = rows.iter().position(egui::Response::has_focus)?;
    let none = egui::Modifiers::NONE;
    let (up, down, home, end) = ui.input_mut(|i| {
        (
            i.consume_key(none, back),
            i.consume_key(none, on),
            i.consume_key(none, egui::Key::Home),
            i.consume_key(none, egui::Key::End),
        )
    });
    let next = if up {
        focused.checked_sub(1)
    } else if down {
        (focused + 1 < rows.len()).then_some(focused + 1)
    } else if home {
        Some(0)
    } else if end {
        rows.len().checked_sub(1)
    } else {
        None
    };
    if up || down || home || end {
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    }
    let next = next.filter(|n| *n != focused)?;
    rows[next].request_focus();
    rows[next].scroll_to_me(None);
    Some(next)
}

/// The inner margin of a one-line entry [`row_height`] tall, `left` and `right` wide, that
/// puts the capitals of `font` on the entry's middle.
pub fn entry_margin(ui: &egui::Ui, font: &egui::FontId, left: i8, right: i8) -> egui::Margin {
    egui_native_look::entry_margin(ui, font, row_height(ui), left, right)
}

/// [`entry_margin`] for body text, the font text edits use.
pub fn body_entry_margin(ui: &egui::Ui, left: i8, right: i8) -> egui::Margin {
    entry_margin(ui, &theme::body(), left, right)
}

/// A button with an icon from `egui_phosphor` and a name, in the accent while `on`, as
/// tall as the controls around it. An empty name leaves the icon alone on a square button.
pub fn icon_label_button(ui: &mut egui::Ui, icon: &str, name: &str, on: bool) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    let gap = if name.is_empty() { 0.0 } else { 6.0 };
    let text_width = if name.is_empty() {
        0.0
    } else {
        crate::chrome::text_width(ui, name, theme::body())
    };
    let icon_width = crate::chrome::text_width(ui, icon, theme::icon());
    let width = (icon_width + gap + text_width + 20.0).max(height);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    theme::paint_button(ui, rect, &response, "", true, on);
    let colour = if on { theme::on_accent() } else { theme::fg() };
    let left = rect.center().x - (icon_width + gap + text_width) / 2.0;
    theme::icon_text(
        ui.painter(),
        egui::pos2(left, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        theme::icon(),
        colour,
    );
    if !name.is_empty() {
        theme::text(
            ui.painter(),
            egui::pos2(left + icon_width + gap, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            theme::body(),
            colour,
        );
    }
    response
}

/// WinUI's underline along a text entry's bottom edge; nothing in the other looks.
pub fn underline(ui: &egui::Ui, response: &egui::Response) {
    egui_native_look::underline(ui.painter(), response.rect, response.has_focus());
}

/// What a drag out of a browser carries, so a view can tell an entity class from a prefab.
#[derive(Debug, Clone)]
pub enum Dragged {
    /// An entity class from the entity browser.
    Class(String),
    /// A prefab `.map` from the asset library.
    Prefab(std::path::PathBuf),
}

use eframe::egui::{Color32, CornerRadius, Rect, Sense, Stroke};

use crate::theme;

/// The height of entries, spins, drop-downs and buttons.
pub const ROW_HEIGHT: f32 = theme::CONTROL_HEIGHT;

/// The height of the controls in `ui`: [`ROW_HEIGHT`] in panels and dialogs, less where a
/// strip such as the tool bar's lowers `interact_size`, so these controls stay as tall as
/// egui's buttons and the native look's widgets beside them.
pub fn row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y
}

/// An on/off switch in the look of the system ([`egui_native_look::Switch`]). The value
/// has changed when `response.changed()`.
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    ui.add(egui_native_look::Switch::new(on))
}

pub use egui_native_look::radio_value;

/// A check box with its label in the look of the system ([`egui_native_look::Checkbox`]).
pub fn checkbox(
    ui: &mut egui::Ui,
    checked: &mut bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    ui.add(egui_native_look::Checkbox::new(checked, text))
}

/// A number field: drag it sideways to scrub, click to type (Enter or leaving commits),
/// the arrow keys and the ‹ › steppers at its ends step by `step`, and the value stays
/// inside `range`. Shows `decimals` digits. Takes the available width, at least 96 px.
pub fn spin(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
) -> egui::Response {
    let width = ui.available_width().clamp(96.0, 220.0);
    spin_width(ui, value, range, step, decimals, width)
}

/// [`spin`] with an explicit width.
pub fn spin_width(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    width: f32,
) -> egui::Response {
    // In a scope, so the layout moves past the whole control: placing the entry inside it
    // would otherwise leave the cursor at the entry's end, under the − and + buttons.
    ui.scope(|ui| spin_body(ui, value, range, step, decimals, width, None))
        .inner
}

/// A spin led by a small coloured caption inside the field, as an engine inspector marks
/// a vector's components, and without the steppers, which a narrow field has no room
/// for: it still scrubs, types and steps with the arrow keys. `caption` is the text and
/// its colour.
pub fn spin_captioned(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    width: f32,
    caption: (&str, Color32),
) -> egui::Response {
    ui.scope(|ui| spin_body(ui, value, range, step, decimals, width, Some(caption)))
        .inner
}

/// Side-by-side spins sharing the available width, one per value, each captioned X, Y,
/// Z (or `captions`) in the axis colours. Returns each field's response.
pub fn axis_spins(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    values: &mut [f64],
    captions: &[&str],
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
) -> Vec<egui::Response> {
    let n = values.len().max(1) as f32;
    let gap = 4.0;
    let each = ((ui.available_width() - gap * (n - 1.0)) / n)
        .floor()
        .max(40.0);
    let salt = egui::Id::unique(id_salt);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        values
            .iter_mut()
            .enumerate()
            .map(|(i, value)| {
                let caption = captions.get(i).copied().unwrap_or("");
                ui.push_id(salt.with(i), |ui| {
                    spin_captioned(
                        ui,
                        value,
                        range.clone(),
                        step,
                        decimals,
                        each,
                        (caption, theme::axis(i)),
                    )
                })
                .inner
            })
            .collect()
    })
    .inner
}

fn spin_body(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    width: f32,
    caption: Option<(&str, Color32)>,
) -> egui::Response {
    let before = *value;
    let stepper = if caption.is_some() { 0.0 } else { 16.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_height(ui)), Sense::hover());
    let radius = CornerRadius::same(theme::radius());
    let enabled = ui.is_enabled();
    let hovered = ui.rect_contains_pointer(rect);
    // WinUI's entry turns solid while it is typed in; whether it is, is last frame's news.
    let typing = theme::win()
        && ui.ctx().memory(|m| m.focused()).is_some_and(|f| {
            ui.ctx()
                .read_response(f)
                .is_some_and(|r| rect.contains_rect(r.rect))
        });
    ui.painter().rect_filled(
        rect,
        radius,
        if typing {
            theme::entry_focused()
        } else {
            theme::entry()
        },
    );
    let mut inner = rect.shrink2(egui::vec2(stepper, 0.0));
    if let Some((text, colour)) = caption {
        let colour = if enabled { colour } else { theme::disabled() };
        let font = egui::FontId::new(theme::text_size() * 0.85, theme::bold());
        crate::theme::text(
            ui.painter(),
            egui::pos2(rect.left() + 7.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            font,
            colour,
        );
        inner.min.x += 16.0;
        inner.max.x -= 4.0;
    }
    // A child Ui, so placing the entry does not move the parent's cursor back inside the
    // space already allocated, which made the next widget in a row overlap this one. The
    // number sits on the field's own fill, so the field reads as one piece end to end.
    let mut entry = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    {
        let w = &mut entry.visuals_mut().widgets;
        for state in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            state.bg_fill = Color32::TRANSPARENT;
            state.weak_bg_fill = Color32::TRANSPARENT;
            state.bg_stroke = Stroke::NONE;
        }
        // While typing, the number is a text edit; its own focus frame and fill would
        // draw a second box inside the field, which already lights its edge. The frame
        // goes with a zero width, but the colour stays: egui draws selected text in it,
        // and a selected number with no colour was invisible.
        entry.visuals_mut().selection.stroke = Stroke::new(0.0, theme::fg());
        entry.visuals_mut().extreme_bg_color = Color32::TRANSPARENT;
    }
    let mut response = entry.add_sized(
        inner.size(),
        egui::DragValue::new(value)
            .range(range.clone())
            .speed(step.max(f64::EPSILON) * 0.25)
            .fixed_decimals(decimals)
            .update_while_editing(false),
    );
    let focused = response.has_focus();
    if theme::win() {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::control_border()),
            egui::StrokeKind::Inside,
        );
        let (width, colour) = if focused {
            (2.0, theme::accent())
        } else {
            (1.0, theme::control_strong())
        };
        let y = rect.bottom() - width / 2.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 2.0, y),
                egui::pos2(rect.right() - 2.0, y),
            ],
            Stroke::new(width, colour),
        );
    } else if theme::mac() {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::control_border()),
            egui::StrokeKind::Inside,
        );
        if focused {
            theme::focus_ring(ui.painter(), rect, radius);
        }
    } else {
        let stroke = if focused {
            theme::accent()
        } else if hovered {
            theme::grid()
        } else {
            theme::border()
        };
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
    }
    if focused {
        let (up, down) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if up {
            *value += step;
        }
        if down {
            *value -= step;
        }
    }
    let left = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + stepper, rect.max.y));
    let right = Rect::from_min_max(egui::pos2(rect.max.x - stepper, rect.min.y), rect.max);
    let steppers = if stepper > 0.0 {
        vec![(left, -1.0_f64, "minus"), (right, 1.0_f64, "plus")]
    } else {
        Vec::new()
    };
    for (r, sign, id) in steppers {
        let at_end = if sign < 0.0 {
            *value <= *range.start()
        } else {
            *value >= *range.end()
        };
        let live = enabled && !at_end;
        let button = ui.interact(
            r,
            response.id.with(id),
            if live { Sense::click() } else { Sense::hover() },
        );
        if hovered || focused {
            let color = if !live {
                theme::disabled()
            } else if button.hovered() {
                theme::fg()
            } else {
                theme::dim()
            };
            let c = r.center();
            let d = 3.0 * sign as f32;
            ui.painter().add(egui::Shape::line(
                vec![
                    c + egui::vec2(-d * 0.5, -4.0),
                    c + egui::vec2(d * 0.5, 0.0),
                    c + egui::vec2(-d * 0.5, 4.0),
                ],
                Stroke::new(1.5, color),
            ));
        }
        if button.clicked() {
            *value += sign * step;
        }
    }
    *value = value.clamp(*range.start(), *range.end());
    if *value != before {
        response.mark_changed();
    }
    response
}

/// [`spin`] over an integer.
#[allow(dead_code)]
pub fn spin_int<T>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    step: T,
) -> egui::Response
where
    T: Copy + Into<f64> + TryFrom<i64>,
{
    let mut v: f64 = (*value).into();
    let response = spin(
        ui,
        &mut v,
        (*range.start()).into()..=(*range.end()).into(),
        step.into(),
        0,
    );
    if response.changed() {
        if let Ok(new) = T::try_from(v.round() as i64) {
            *value = new;
        }
    }
    response
}

/// A drop-down: the current choice with a chevron, opening a list. Returns true when the
/// choice changed.
pub fn dropdown(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: &mut usize,
    choices: &[&str],
    width: f32,
) -> bool {
    dropdown_with_icons(ui, id, selected, choices, &[], width)
}

/// Game choices share the same icons on the welcome screen and in preferences.
pub fn game_dropdown(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: &mut usize,
    games: &[editor_core::games::Game],
    width: f32,
) -> bool {
    let mut names = vec![editor_core::i18n::tr("None")];
    names.extend(games.iter().map(|game| game.display_name()));
    let mut icons = vec![None];
    icons.extend(games.iter().map(|game| {
        let path = game.icon.as_ref()?;
        let key = egui::Id::unique(("game-icon", path));
        if let Some(cached) = ui
            .ctx()
            .data_mut(|data| data.get_temp::<Option<egui::TextureHandle>>(key))
        {
            return cached;
        }
        let loaded = editor_core::files::read(path)
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|image| {
                let image = image.thumbnail(64, 64).to_rgba8();
                ui.ctx().load_texture(
                    path.display().to_string(),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        ui.ctx()
            .data_mut(|data| data.insert_temp(key, loaded.clone()));
        loaded
    }));
    dropdown_with_icons(ui, id, selected, &names, &icons, width)
}

fn dropdown_with_icons(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: &mut usize,
    choices: &[&str],
    icons: &[Option<egui::TextureHandle>],
    width: f32,
) -> bool {
    let before = *selected;
    let text = choices.get(*selected).copied().unwrap_or("");
    // As tall as the controls around it: ROW_HEIGHT in panels, less in the tool strip.
    let height = row_height(ui);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let enabled = ui.is_enabled();
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Id::unique(&id).with("dropdown"));
    let painter = ui.painter();
    let arrows = egui_native_look::paint_popup_button(painter, rect, &response, enabled, open);
    let color = if enabled {
        theme::fg()
    } else {
        theme::disabled()
    };
    let icon = icons.get(*selected).and_then(Option::as_ref);
    let inset = if let Some(icon) = icon {
        let size = icon.size_vec2();
        let size = size * (20.0 / size.x.max(size.y));
        let at =
            egui::Rect::from_center_size(egui::pos2(rect.left() + 18.0, rect.center().y), size);
        painter.image(
            icon.id(),
            at,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        34.0
    } else {
        8.0
    };
    let mut job = egui::text::LayoutJob::simple(
        text.to_string(),
        theme::body(),
        color,
        (rect.width() - inset - arrows).max(0.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    painter.galley(
        egui::pos2(
            rect.min.x + inset,
            crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
        ),
        galley,
        color,
    );
    // Under the control, or over it when the window has more room there, and only as tall
    // as that room, so the list never ends up beside the control.
    let screen = ui.ctx().content_rect();
    let (below, above) = (
        screen.bottom() - rect.bottom() - 12.0,
        rect.top() - screen.top() - 12.0,
    );
    let (align, room) = if below >= 200.0 || below >= above {
        (egui::RectAlign::BOTTOM_START, below)
    } else {
        (egui::RectAlign::TOP_START, above)
    };
    let menu_width = if icons.is_empty() {
        width
    } else {
        choices
            .iter()
            .map(|text| {
                painter
                    .layout_no_wrap((*text).to_string(), theme::body(), color)
                    .size()
                    .x
                    + 52.0
            })
            .fold(width, f32::max)
            .min(screen.width())
    };
    egui::Popup::menu(&response)
        .id(egui::Id::unique(&id).with("dropdown"))
        .width(menu_width)
        .align(align)
        .align_alternatives(&[])
        .show(|ui| {
            // As wide as the control at least, one line per choice, whole rows to click.
            ui.set_min_width(width - 2.0 * ui.spacing().menu_margin.leftf());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::ScrollArea::vertical()
                .max_height(room.clamp(80.0, 360.0))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        for (i, choice) in choices.iter().enumerate() {
                            let response = if let Some(Some(icon)) = icons.get(i) {
                                let image = egui::Image::new((icon.id(), icon.size_vec2()))
                                    .max_size(egui::vec2(20.0, 20.0));
                                ui.add(egui::Button::selectable(i == *selected, (image, *choice)))
                            } else {
                                ui.selectable_label(i == *selected, *choice)
                            };
                            if response.clicked() {
                                *selected = i;
                            }
                        }
                    });
                });
        });
    *selected != before
}

/// A section card: a subtly raised block with a small title, optionally led by an
/// accent-coloured icon drawn by `icon`. An empty title with no icon draws no title row.
pub fn card<R>(
    ui: &mut egui::Ui,
    title: &str,
    icon: Option<fn(&egui::Painter, egui::Rect, Color32)>,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(theme::surface())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() || icon.is_some() {
                ui.horizontal(|ui| {
                    if let Some(icon) = icon {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                        icon(ui.painter(), r, theme::accent());
                    }
                    ui.label(
                        egui::RichText::new(title)
                            .font(theme::body_bold())
                            .color(theme::fg()),
                    );
                });
                ui.add_space(4.0);
            }
            contents(ui)
        })
        .inner
}

/// The share of a row's width its label takes, as in an engine's inspector, so the
/// controls of consecutive rows line up in one column.
pub const LABEL_SHARE: f32 = 0.45;

/// A labelled row: the label in the left column (dimmed, ellipsized, its full text and
/// `tooltip` on hover) and the control filling the right column.
pub fn row<R>(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: Option<&str>,
    control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = ui.available_width();
    let label_w = (width * LABEL_SHARE).floor();
    ui.horizontal(|ui| {
        let height = row_height(ui);
        ui.set_min_height(height);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(label_w, height), Sense::hover());
        let galley = ui.painter().layout(
            label.to_string(),
            theme::body(),
            theme::dim(),
            f32::INFINITY,
        );
        let clipped = galley.size().x > rect.width() - 6.0;
        let text = if clipped {
            let mut job = egui::text::LayoutJob::simple_singleline(
                label.to_string(),
                theme::body(),
                theme::dim(),
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 6.0);
            ui.painter().layout_job(job)
        } else {
            galley
        };
        ui.painter().galley(
            egui::pos2(
                rect.min.x,
                crate::theme::galley_top(ui.ctx(), &text, rect.center().y),
            ),
            text,
            theme::dim(),
        );
        match (tooltip, clipped) {
            (Some(tip), true) => {
                response.on_hover_text(format!("{label}\n\n{tip}"));
            }
            (Some(tip), false) => {
                response.on_hover_text(tip);
            }
            (None, true) => {
                response.on_hover_text(label);
            }
            (None, false) => {}
        }
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), height),
            egui::Layout::left_to_right(egui::Align::Center),
            control,
        )
        .inner
    })
    .inner
}

/// A label `width` wide and as tall as the controls, its text at the left end and on the
/// row's middle, for a column of labels beside a column of controls. A text too long for
/// the column ends in an ellipsis and shows in full on hover.
pub fn column_label(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, row_height(ui)), Sense::hover());
    let colour = ui.visuals().text_color();
    let (galley, cut) = truncated(ui, text, theme::body(), colour, width);
    ui.painter().galley(
        egui::pos2(
            rect.left(),
            crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
        ),
        galley,
        colour,
    );
    if cut {
        response.on_hover_text(text)
    } else {
        response
    }
}

/// One line of text cut short with an ellipsis where it would pass `width`, and whether
/// it was cut.
pub fn truncated(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> (std::sync::Arc<egui::Galley>, bool) {
    let full = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), color);
    if full.size().x <= width {
        return (full, false);
    }
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
    (ui.painter().layout_job(job), true)
}

/// The height of a section's header.
pub const SECTION_HEIGHT: f32 = 26.0;

/// A collapsible section of an inspector tab: a full-width header with a chevron and a
/// bold title, the body under it, and the same spacing everywhere. Its open state is
/// remembered under `id`. Returns what `contents` returned while open.
pub fn section<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    default_open: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    section_noted(ui, id, title, None, default_open, contents)
}

/// Opens the [`section`] drawn next in `ui` under `id`, as when something in it is to
/// be revealed.
pub fn open_section(ui: &egui::Ui, id: impl std::hash::Hash + std::fmt::Debug) {
    let id = ui.make_persistent_id(("section", id));
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
    state.set_open(true);
    state.store(ui.ctx());
}

/// [`section`] with a dim note at the header's right end, such as a count.
pub fn section_noted<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    note: Option<&str>,
    default_open: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let id = ui.make_persistent_id(("section", id));
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        default_open,
    );
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SECTION_HEIGHT),
        Sense::click(),
    );
    if response.clicked() {
        state.toggle(ui);
    }
    let openness = state.openness(ui.ctx());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if response.hovered() {
            theme::section_hover()
        } else {
            theme::section()
        };
        painter.rect_filled(rect, CornerRadius::same(theme::radius()), fill);
        // The chevron turns from pointing right to pointing down as the body opens.
        let c = egui::pos2(rect.left() + 12.0, rect.center().y);
        let rot = egui::emath::Rot2::from_angle(openness * std::f32::consts::FRAC_PI_2);
        let points: Vec<egui::Pos2> = [
            egui::vec2(-2.0, -4.0),
            egui::vec2(2.0, 0.0),
            egui::vec2(-2.0, 4.0),
        ]
        .iter()
        .map(|v| c + rot * *v)
        .collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.5, theme::dim())));
        let mut right = rect.right() - 8.0;
        if let Some(note) = note {
            let (galley, _) = truncated(ui, note, theme::body(), theme::dim(), rect.width() * 0.45);
            right -= galley.size().x;
            painter.galley(
                egui::pos2(
                    right,
                    crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
                ),
                galley,
                theme::dim(),
            );
            right -= 8.0;
        }
        let left = rect.left() + 24.0;
        let (galley, cut) = truncated(ui, title, theme::body_bold(), theme::fg(), right - left);
        painter.galley(
            egui::pos2(
                left,
                crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
            ),
            galley,
            theme::fg(),
        );
        if cut {
            response.on_hover_text(title);
        }
    }
    let body = state.show_body_unindented(ui, |ui| {
        ui.add_space(2.0);
        let inner = contents(ui);
        ui.add_space(4.0);
        inner
    });
    ui.add_space(4.0);
    body.map(|b| b.inner)
}

/// A small dim heading inside a section, for a group of rows of its own.
pub fn subheading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), Sense::hover());
    let font = egui::FontId::new(theme::text_size() * 0.85, theme::bold());
    let galley = ui
        .painter()
        .layout_no_wrap(title.to_string(), font, theme::dim());
    let y = rect.center().y;
    let text_end = rect.left() + galley.size().x;
    ui.painter().galley(
        egui::pos2(rect.left(), y - galley.size().y / 2.0),
        galley,
        theme::dim(),
    );
    if text_end + 8.0 < rect.right() {
        ui.painter().line_segment(
            [egui::pos2(text_end + 8.0, y), egui::pos2(rect.right(), y)],
            Stroke::new(1.0, theme::border()),
        );
    }
}

/// A dim, wrapping line of status text.
pub fn status(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(theme::dim())).wrap());
}

/// What an empty list or panel says: dim text in the middle of the width.
pub fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.vertical_centered(|ui| {
        ui.add(
            egui::Label::new(egui::RichText::new(text).color(theme::dim()))
                .wrap()
                .halign(egui::Align::Center),
        );
    });
    ui.add_space(6.0);
}

/// A filter entry the width of the column, with a magnifier in front of the text.
pub fn search(ui: &mut egui::Ui, text: &mut String, hint: &str) -> egui::Response {
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(ui.available_width())
            .margin(crate::widgets::body_entry_margin(ui, 26, 6))
            .min_size(egui::vec2(0.0, row_height(ui))),
    );
    let r = Rect::from_center_size(
        egui::pos2(response.rect.left() + 14.0, response.rect.center().y),
        egui::vec2(14.0, 14.0),
    );
    icons::search(ui.painter(), r, theme::dim());
    underline(ui, &response);
    response
}

/// A small flat button showing an icon, its tooltip the label it stands for. `on` marks
/// a toggle that is switched on.
pub fn icon_button(
    ui: &mut egui::Ui,
    size: f32,
    tip: &str,
    enabled: bool,
    on: bool,
    icon: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size, size),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let painter = ui.painter();
    let radius = CornerRadius::same(theme::radius());
    let hovered = enabled && response.hovered();
    if on {
        painter.rect_filled(
            rect,
            radius,
            theme::accent().gamma_multiply(if hovered { 0.5 } else { 0.35 }),
        );
    } else if hovered {
        painter.rect_filled(
            rect,
            radius,
            if response.is_pointer_button_down_on() {
                theme::button_pressed()
            } else {
                theme::button_hover()
            },
        );
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::accent_hover()),
            egui::StrokeKind::Inside,
        );
    }
    let colour = if !enabled {
        theme::disabled()
    } else if on || hovered {
        theme::fg()
    } else {
        theme::dim()
    };
    icon(
        painter,
        Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)),
        colour,
    );
    response.on_hover_text(tip).on_disabled_hover_text(tip)
}

/// A thin vertical rule between groups of icon buttons in a toolbar.
pub fn toolbar_gap(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(9.0, row_height(ui)), Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top() + 7.0),
            egui::pos2(rect.center().x, rect.bottom() - 7.0),
        ],
        Stroke::new(1.0, theme::border()),
    );
}

/// A segmented button group: equal-width buttons joined into one strip the width of the
/// column, each `(label, tooltip, enabled)`. A label too long for its segment ends in an
/// ellipsis and shows in full on hover. Returns the index of the one clicked.
pub fn segmented(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    items: &[(&str, &str, bool)],
) -> Option<usize> {
    let width = ui.available_width();
    segmented_width(ui, id, items, width)
}

/// [`segmented`] at an explicit width.
pub fn segmented_width(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    items: &[(&str, &str, bool)],
    width: f32,
) -> Option<usize> {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_height(ui)), Sense::hover());
    let n = items.len().max(1);
    let each = rect.width() / n as f32;
    let id = ui.scope_id().with(("segmented", egui::Id::unique(id)));
    let r = theme::radius();
    let mut clicked = None;
    for (i, (label, tip, enabled)) in items.iter().enumerate() {
        let slot = Rect::from_min_size(
            egui::pos2(rect.left() + each * i as f32, rect.top()),
            egui::vec2(each, rect.height()),
        );
        let response = ui.interact(
            slot,
            id.with(i),
            if *enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let radius = CornerRadius {
            nw: if i == 0 { r } else { 0 },
            sw: if i == 0 { r } else { 0 },
            ne: if i + 1 == n { r } else { 0 },
            se: if i + 1 == n { r } else { 0 },
        };
        // A disabled segment keeps the strip's fill, so the strip reads as one piece.
        let fill = if !enabled {
            theme::button_fill()
        } else if response.is_pointer_button_down_on() {
            theme::button_pressed()
        } else if response.hovered() {
            theme::button_hover()
        } else {
            theme::button_fill()
        };
        ui.painter().rect_filled(slot, radius, fill);
        let colour = if *enabled {
            theme::fg()
        } else {
            theme::disabled()
        };
        let (galley, cut) = truncated(ui, label, theme::body(), colour, slot.width() - 10.0);
        ui.painter().galley(
            egui::pos2(
                slot.center().x - galley.size().x / 2.0,
                crate::theme::galley_top(ui.ctx(), &galley, slot.center().y),
            ),
            galley,
            colour,
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                slot.shrink(1.0),
                radius,
                Stroke::new(1.0, theme::accent_hover()),
                egui::StrokeKind::Inside,
            );
        }
        let hover = match (cut, tip.is_empty()) {
            (true, false) => format!("{label}\n\n{tip}"),
            (true, true) => label.to_string(),
            (false, _) => tip.to_string(),
        };
        let response = if hover.is_empty() {
            response
        } else {
            response
                .on_hover_text(&hover)
                .on_disabled_hover_text(&hover)
        };
        if response.clicked() {
            clicked = Some(i);
        }
    }
    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        CornerRadius::same(r),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    for i in 1..n {
        let x = (rect.left() + each * i as f32).round() + 0.5;
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 1.0),
                egui::pos2(x, rect.bottom() - 1.0),
            ],
            Stroke::new(1.0, theme::border()),
        );
    }
    clicked
}

/// A segmented choice: equal-width segments, `width` wide, in the look of the system
/// ([`egui_native_look::Segmented`]), each `(label, tooltip)`. Returns true when
/// the choice changed.
pub fn segmented_choice(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    items: &[(&str, &str)],
    selected: &mut usize,
    width: f32,
) -> bool {
    let labels: Vec<&str> = items.iter().map(|(label, _)| *label).collect();
    let tips: Vec<&str> = items.iter().map(|(_, tip)| *tip).collect();
    ui.add(
        egui_native_look::Segmented::new(id, selected, &labels)
            .tooltips(&tips)
            .width(width),
    )
    .changed()
}

/// A row with its label taking the width and a switch at the right end; a click on the
/// label works the switch too. The value has changed when `response.changed()`.
pub fn toggle_row(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: Option<&str>,
    on: &mut bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(width, row_height(ui)), Sense::click());
    let knob = Rect::from_min_max(
        egui::pos2(
            rect.right() - egui_native_look::switch_width(theme::look()),
            rect.top(),
        ),
        rect.max,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(knob)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let switched = switch(&mut child, on);
    if response.clicked() && ui.is_enabled() {
        *on = !*on;
        response.mark_changed();
    }
    let colour = if ui.is_enabled() {
        theme::fg()
    } else {
        theme::disabled()
    };
    let (galley, cut) = truncated(
        ui,
        label,
        theme::body(),
        colour,
        rect.width() - egui_native_look::switch_width(theme::look()) - 12.0,
    );
    ui.painter().galley(
        egui::pos2(
            rect.left(),
            crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
        ),
        galley,
        colour,
    );
    let response = response | switched;
    match (tooltip, cut) {
        (Some(tip), true) => response.on_hover_text(format!("{label}\n\n{tip}")),
        (Some(tip), false) => response.on_hover_text(tip),
        (None, true) => response.on_hover_text(label),
        (None, false) => response,
    }
}

/// The height of a list's row.
pub const LIST_ROW: f32 = 26.0;

/// How many cells of at least `min` wide fit a line of `width` with `gap` between, and
/// how wide each is when they share the line exactly, so a grid has no ragged edge.
pub fn grid_cells(width: f32, min: f32, gap: f32) -> (usize, f32) {
    let per_line = (((width + gap) / (min + gap)).floor() as usize).max(1);
    (
        per_line,
        ((width - gap * (per_line as f32 - 1.0)) / per_line as f32).floor(),
    )
}

/// A list's well: a sunken frame the width of the column, at least `min_height` tall.
pub fn list<R>(ui: &mut egui::Ui, min_height: f32, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme::well())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(CornerRadius::same(theme::radius()))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(min_height);
            ui.spacing_mut().item_spacing.y = 0.0;
            contents(ui)
        })
        .inner
}

/// The sense of a list cell that can be dragged onto a view (an asset, an entity class).
///
/// On a touch screen a finger dragging over a list means scrolling it, as everywhere
/// else on a tablet: a cell that took drags would leave only the scroll bar to scroll
/// with. There a cell takes taps only, and a double tap places what it holds.
pub fn drag_source_sense(ctx: &egui::Context) -> egui::Sense {
    if crate::touch::enabled(ctx) {
        egui::Sense::click()
    } else {
        egui::Sense::click_and_drag()
    }
}

/// One row of a list: the full width and `height` tall, painted as picked or under the
/// pointer. Draw the row's content into the returned rectangle.
pub fn list_row(
    ui: &mut egui::Ui,
    height: f32,
    picked: bool,
    sense: Sense,
) -> (Rect, egui::Response) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), sense);
    paint_list_row(ui, rect, picked, response.hovered());
    (rect, response)
}

/// The background of a list row, picked or under the pointer.
pub fn paint_list_row(ui: &egui::Ui, rect: Rect, picked: bool, hovered: bool) {
    let radius = CornerRadius::same(if theme::mac() {
        5
    } else if theme::win() {
        4
    } else {
        3
    });
    if picked {
        ui.painter()
            .rect_filled(rect, radius, theme::row_selected());
    } else if hovered {
        ui.painter().rect_filled(rect, radius, theme::row_hover());
    }
}

/// Small line icons, each drawn into a square `rect` in `color`.
pub mod icons {
    use eframe::egui::{self, Color32, CornerRadius, Painter, Pos2, Rect, Stroke};

    fn s(color: Color32) -> Stroke {
        Stroke::new(1.5, color)
    }

    /// `x` and `y` on a 16-unit grid over the rectangle.
    fn at(rect: Rect, x: f32, y: f32) -> Pos2 {
        rect.min + egui::vec2(x, y) * (rect.width() / 16.0)
    }

    pub fn plus(p: &Painter, r: Rect, c: Color32) {
        p.line_segment([at(r, 8.0, 3.0), at(r, 8.0, 13.0)], s(c));
        p.line_segment([at(r, 3.0, 8.0), at(r, 13.0, 8.0)], s(c));
    }

    pub fn minus(p: &Painter, r: Rect, c: Color32) {
        p.line_segment([at(r, 3.0, 8.0), at(r, 13.0, 8.0)], s(c));
    }

    pub fn search(p: &Painter, r: Rect, c: Color32) {
        p.circle_stroke(at(r, 6.5, 6.5), r.width() * 4.5 / 16.0, s(c));
        p.line_segment([at(r, 10.0, 10.0), at(r, 14.0, 14.0)], s(c));
    }

    pub fn eye(p: &Painter, r: Rect, c: Color32) {
        let center = r.center();
        let k = r.width() / 16.0;
        let upper: Vec<Pos2> = (0..=12)
            .map(|i| {
                let t = i as f32 / 12.0 * std::f32::consts::PI;
                center + egui::vec2(-7.0 * k * t.cos(), -4.5 * k * t.sin())
            })
            .collect();
        let lower: Vec<Pos2> = upper
            .iter()
            .map(|q| Pos2::new(q.x, 2.0 * center.y - q.y))
            .collect();
        p.add(egui::Shape::line(upper, s(c)));
        p.add(egui::Shape::line(lower, s(c)));
        p.circle_filled(center, 2.2 * k, c);
    }

    pub fn lock(p: &Painter, r: Rect, c: Color32) {
        let k = r.width() / 16.0;
        p.rect_filled(
            Rect::from_min_max(at(r, 3.5, 7.0), at(r, 12.5, 14.0)),
            CornerRadius::same(2),
            c,
        );
        let center = at(r, 8.0, 7.0);
        let shackle: Vec<Pos2> = (0..=10)
            .map(|i| {
                let t = i as f32 / 10.0 * std::f32::consts::PI;
                center + egui::vec2(-3.0 * k * t.cos(), -3.5 * k * t.sin())
            })
            .collect();
        p.add(egui::Shape::line(shackle, s(c)));
    }

    /// An open padlock: a property linked groups may overwrite.
    pub fn unlock(p: &Painter, r: Rect, c: Color32) {
        let k = r.width() / 16.0;
        p.rect_stroke(
            Rect::from_min_max(at(r, 3.5, 7.0), at(r, 12.5, 14.0)),
            CornerRadius::same(2),
            s(c),
            egui::StrokeKind::Inside,
        );
        let center = at(r, 10.5, 5.5);
        let shackle: Vec<Pos2> = (0..=8)
            .map(|i| {
                let t = i as f32 / 10.0 * std::f32::consts::PI;
                center + egui::vec2(-3.0 * k * t.cos(), -3.5 * k * t.sin())
            })
            .collect();
        p.add(egui::Shape::line(shackle, s(c)));
    }

    /// A page with an arrow leaving it: left out of exports.
    pub fn export(p: &Painter, r: Rect, c: Color32) {
        p.add(egui::Shape::line(
            vec![
                at(r, 5.0, 7.0),
                at(r, 3.0, 7.0),
                at(r, 3.0, 14.0),
                at(r, 13.0, 14.0),
                at(r, 13.0, 7.0),
                at(r, 11.0, 7.0),
            ],
            s(c),
        ));
        p.line_segment([at(r, 8.0, 10.0), at(r, 8.0, 2.0)], s(c));
        p.add(egui::Shape::line(
            vec![at(r, 5.0, 5.0), at(r, 8.0, 2.0), at(r, 11.0, 5.0)],
            s(c),
        ));
    }

    /// An arrow dropping into a tray: move the selection here.
    pub fn move_here(p: &Painter, r: Rect, c: Color32) {
        p.add(egui::Shape::line(
            vec![
                at(r, 3.0, 9.0),
                at(r, 3.0, 14.0),
                at(r, 13.0, 14.0),
                at(r, 13.0, 9.0),
            ],
            s(c),
        ));
        p.line_segment([at(r, 8.0, 2.0), at(r, 8.0, 10.5)], s(c));
        p.add(egui::Shape::line(
            vec![at(r, 5.0, 7.5), at(r, 8.0, 10.5), at(r, 11.0, 7.5)],
            s(c),
        ));
    }

    /// A dashed selection frame round a dot: select what is on it.
    pub fn select(p: &Painter, r: Rect, c: Color32) {
        let corners = [
            at(r, 2.5, 2.5),
            at(r, 13.5, 2.5),
            at(r, 13.5, 13.5),
            at(r, 2.5, 13.5),
        ];
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            p.extend(egui::Shape::dashed_line(&[a, b], s(c), 2.0, 2.0));
        }
        p.circle_filled(r.center(), r.width() * 2.0 / 16.0, c);
    }

    /// Two opposed arrows: swap.
    pub fn swap(p: &Painter, r: Rect, c: Color32) {
        for (y, dir) in [(5.0, 1.0), (11.0, -1.0)] {
            let (a, b) = (at(r, 8.0 - 6.0 * dir, y), at(r, 8.0 + 6.0 * dir, y));
            p.line_segment([a, b], s(c));
            p.add(egui::Shape::line(
                vec![
                    at(r, 8.0 + 3.0 * dir, y - 3.0),
                    b,
                    at(r, 8.0 + 3.0 * dir, y + 3.0),
                ],
                s(c),
            ));
        }
    }

    /// A circled "i": more about this.
    pub fn info(p: &Painter, r: Rect, c: Color32) {
        p.circle_stroke(r.center(), r.width() * 6.0 / 16.0, Stroke::new(1.2, c));
        p.circle_filled(at(r, 8.0, 5.2), r.width() * 0.9 / 16.0, c);
        p.line_segment([at(r, 8.0, 7.2), at(r, 8.0, 11.5)], Stroke::new(1.4, c));
    }

    /// A square with the half towards `side` filled: left, right, top or bottom.
    pub fn justify(p: &Painter, r: Rect, c: Color32, side: usize) {
        let square = Rect::from_min_max(at(r, 2.0, 2.0), at(r, 14.0, 14.0));
        let half = match side {
            0 => Rect::from_min_max(square.min, egui::pos2(square.center().x, square.max.y)),
            1 => Rect::from_min_max(egui::pos2(square.center().x, square.min.y), square.max),
            2 => Rect::from_min_max(square.min, egui::pos2(square.max.x, square.center().y)),
            _ => Rect::from_min_max(egui::pos2(square.min.x, square.center().y), square.max),
        };
        p.rect_filled(half, 0.0, c);
        p.rect_stroke(square, 1.0, Stroke::new(1.5, c), egui::StrokeKind::Inside);
    }
}

/// Visible one-unit steps beside the entry; all edits share the preview update.
pub fn number_buttons<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    number_buttons_step(ui, value, range, 1.0)
}

/// Number buttons whose increments and drag speed follow a discrete generator step.
pub fn number_buttons_step<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
) -> bool {
    ui.add(
        egui_native_look::NumberField::new(value)
            .range(range)
            .step(step),
    )
    .changed()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<R>(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        mut add: impl FnMut(&mut egui::Ui) -> R,
    ) -> R {
        let mut result = None;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(340.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| result = Some(add(ui)));
        output.textures_delta.clear();
        result.unwrap()
    }

    fn click(at: egui::Pos2) -> Vec<Vec<egui::Event>> {
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        vec![
            vec![egui::Event::PointerMoved(at)],
            vec![press(true)],
            vec![press(false)],
        ]
    }

    /// The check box, the radio button and the segmented choice of the current look
    /// (`HERON_BRUSH_LOOK` picks another) draw and take a click.
    #[test]
    fn marks_and_choices_take_clicks_in_this_look() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut checked = false;
        for events in click(egui::pos2(6.0, 10.0)) {
            frame(&ctx, events, |ui| checkbox(ui, &mut checked, "Snap"));
        }
        assert!(checked, "the check box did not take the click");
        let mut chosen = 0;
        for events in click(egui::pos2(6.0, 10.0)) {
            frame(&ctx, events, |ui| radio_value(ui, &mut chosen, 1, "Rotate"));
        }
        assert_eq!(chosen, 1, "the radio button did not take the click");
        let mut segment = 0;
        for events in click(egui::pos2(250.0, 10.0)) {
            frame(&ctx, events, |ui| {
                segmented_choice(
                    ui,
                    "axis",
                    &[("X", ""), ("Y", ""), ("Z", "")],
                    &mut segment,
                    300.0,
                )
            });
        }
        assert_eq!(segment, 2, "the segmented choice did not take the click");
    }

    #[test]
    fn grid_cells_fill_the_line_exactly() {
        let (n, w) = grid_cells(312.0, 84.0, 4.0);
        assert_eq!(n, 3);
        let used = n as f32 * w + (n as f32 - 1.0) * 4.0;
        assert!(used <= 312.0 && used > 309.0, "{used}");
        assert_eq!(
            grid_cells(40.0, 84.0, 4.0).0,
            1,
            "a narrow line still holds one"
        );
    }

    /// A segment clicks only when enabled, and a section's body goes when its header is
    /// clicked.
    #[test]
    fn segments_click_and_sections_fold() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let items = [("A", "", true), ("B", "", false), ("C", "", true)];
        let mut clicked = Vec::new();
        // Each segment is a third of the strip at the top.
        for x in [50.0, 170.0, 290.0] {
            for events in click(egui::pos2(x, 10.0)) {
                if let Some(i) = frame(&ctx, events, |ui| segmented(ui, "strip", &items)) {
                    clicked.push(i);
                }
            }
        }
        assert_eq!(clicked, vec![0, 2], "the disabled one does nothing");

        let body = |ui: &mut egui::Ui| section(ui, "fold", "Title", true, |_| ()).is_some();
        assert!(frame(&ctx, vec![], body));
        for events in click(egui::pos2(100.0, 10.0)) {
            frame(&ctx, events, body);
        }
        // The fold animates; let it finish.
        for _ in 0..40 {
            frame(&ctx, vec![], body);
            std::thread::sleep(web_time::Duration::from_millis(10));
        }
        assert!(!frame(&ctx, vec![], body), "a click on the header folds it");
    }

    #[test]
    fn a_click_on_a_toggle_rows_label_switches_it() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut on = false;
        let mut changed = false;
        for events in click(egui::pos2(40.0, 10.0)) {
            changed |= frame(&ctx, events, |ui| {
                toggle_row(ui, "Wireframe", None, &mut on).changed()
            });
        }
        assert!(on && changed);
    }
}
