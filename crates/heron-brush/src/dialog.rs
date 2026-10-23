//! Dialogs as their own windows: a title, a body, and a strip of buttons along the bottom under a separator.

use eframe::egui;

use crate::theme;

/// Which part of a dialog the contents closure is asked to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The body.
    Body,
    /// The buttons along the bottom, laid out right to left, so the first one added is
    /// the rightmost.
    Actions,
}

/// Shows a dialog window this frame. `contents` is called once for each [`Part`]. Returns
/// false once the window's close button has been pressed.
pub fn show(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui, Part),
) -> bool {
    show_with_body_margin(ctx, id, title, size, 12.0, contents)
}

/// [`show`] for a tool's settings, such as the fillet's: the window stays above the main
/// window while Heron Brush is the app in front, so a click in the views does not bury it,
/// and drops back among the other apps' windows when another app comes to the front.
pub fn show_tool(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui, Part),
) -> bool {
    show_window(ctx, id, title, size, 12.0, true, contents)
}

/// A dialog whose body provides its own spacing, such as a full-width sidebar.
pub fn show_with_body_margin(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    body_margin: f32,
    contents: impl FnMut(&mut egui::Ui, Part),
) -> bool {
    show_window(ctx, id, title, size, body_margin, false, contents)
}

fn show_window(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    body_margin: f32,
    on_top: bool,
    mut contents: impl FnMut(&mut egui::Ui, Part),
) -> bool {
    let placement_id = egui::Id::unique(("dialog-placement", id));
    let frame = ctx.cumulative_frame_nr();
    let previous = ctx.data_mut(|d| d.get_temp::<Placement>(placement_id));
    let opening =
        previous.is_none_or(|p| frame.wrapping_sub(p.last_frame) > 1 || p.opened_frame == frame);
    let opened_frame = if opening {
        frame
    } else {
        previous.unwrap().opened_frame
    };
    let mut pending = opening || previous.is_some_and(|p| p.pending);
    let size = egui::vec2(size[0], size[1]).min(dialog_room(ctx));
    let open = if ctx.embed_viewports() {
        // Draw directly: show_viewport_immediate already embeds its own Window,
        // which would otherwise leave an extra window behind this dialog.
        embedded(
            ctx,
            id,
            title,
            size,
            body_margin,
            opening,
            &mut pending,
            &mut contents,
        )
    } else {
        let viewport = egui::ViewportId::from_hash_of(id);
        let level = if on_top {
            tool_level(ctx, viewport)
        } else {
            egui::WindowLevel::Normal
        };
        let parent = ctx.input(|i| i.viewport().inner_rect.or(i.viewport().outer_rect));
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(size)
            .with_min_inner_size(size * 0.6)
            .with_window_level(level);
        if opening {
            if let Some(parent) = parent {
                builder = builder.with_position(parent.center() - size * 0.5);
            }
        }
        ctx.show_viewport_immediate(viewport, builder, |ui, class| {
            if class == egui::ViewportClass::EmbeddedWindow {
                return embedded(
                    ui.ctx(),
                    id,
                    title,
                    size,
                    body_margin,
                    opening,
                    &mut pending,
                    &mut contents,
                );
            }
            // Once decorations are measured, center the actual outer rectangle too.
            // This runs only on creation, so dragging an open dialog stays possible.
            if pending {
                if let (Some(parent), Some(outer)) = (parent, ui.input(|i| i.viewport().outer_rect))
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            parent.center() - outer.size() * 0.5,
                        ));
                    pending = false;
                } else if parent.is_none() {
                    pending = false;
                }
            }
            egui::Panel::bottom(egui::Id::unique((id, "actions")))
                .frame(egui::Frame::new().fill(theme::bg()).inner_margin(12.0))
                .show_separator_line(true)
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        contents(ui, Part::Actions)
                    });
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(theme::bg())
                        .inner_margin(body_margin),
                )
                .show(ui, |ui| contents(ui, Part::Body));
            !ui.input(|i| i.viewport().close_requested())
        })
    };
    ctx.data_mut(|d| {
        if open {
            d.insert_temp(
                placement_id,
                Placement {
                    last_frame: frame,
                    opened_frame,
                    pending,
                },
            );
        } else {
            d.remove::<Placement>(placement_id);
        }
    });
    if pending {
        ctx.request_repaint();
    }
    open
}

/// The level a tool window `viewport` takes: floating while any of Heron Brush's windows
/// has the keyboard, normal once none has had it for a moment (focus passing from one
/// window to another leaves none focused for a frame or two). Tells the window when its
/// level changes.
fn tool_level(ctx: &egui::Context, viewport: egui::ViewportId) -> egui::WindowLevel {
    const GRACE: f64 = 0.3;
    let (now, focused) = ctx.input(|i| {
        (
            i.time,
            i.raw.viewports.values().any(|v| v.focused == Some(true)),
        )
    });
    let id = egui::Id::unique(("tool-window-level", viewport));
    let (mut last_focused, was) = ctx
        .data(|d| d.get_temp::<(f64, bool)>(id))
        .unwrap_or((now, true));
    if focused {
        last_focused = now;
    } else if was {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(GRACE));
    }
    let on_top = focused || now - last_focused < GRACE;
    ctx.data_mut(|d| d.insert_temp(id, (last_focused, on_top)));
    let level = if on_top {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    };
    if on_top != was {
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::WindowLevel(level));
    }
    level
}

#[derive(Clone, Copy)]
struct Placement {
    last_frame: u64,
    opened_frame: u64,
    pending: bool,
}

fn dialog_room(ctx: &egui::Context) -> egui::Vec2 {
    (ctx.content_rect().size() - egui::vec2(24.0, 48.0)).max(egui::vec2(64.0, 64.0))
}

// Both callers are in `show_window` and pass its own arguments straight through; a struct
// for them would be built only to be taken apart here.
#[allow(clippy::too_many_arguments)]
fn embedded(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: egui::Vec2,
    body_margin: f32,
    opening: bool,
    pending: &mut bool,
    contents: &mut impl FnMut(&mut egui::Ui, Part),
) -> bool {
    let mut open = true;
    let room = dialog_room(ctx);
    let size = size.min(room);
    let measured = ctx.memory(|m| m.area_rect(egui::Id::unique(id)));
    let mut window = egui::Window::new(title);
    if opening || *pending {
        let actual = measured.map(|r| r.size()).unwrap_or(size).min(room);
        // egui's title-bar drag restores its saved position during begin;
        // disable movement only for this placement pass so it accepts the new center.
        window = window
            .current_pos(ctx.content_rect().center() - actual * 0.5)
            .movable(false);
        *pending = measured.is_none();
        ctx.request_repaint();
    }
    // The window draws its own header rather than egui's small title bar: the title in
    // the body's bold face on the middle of a bar as tall as the main window's, and a
    // close button with room to aim at.
    let frame = egui::Frame::window(&ctx.global_style())
        .fill(theme::bg())
        .inner_margin(0);
    window
        .id(egui::Id::unique(id))
        .title_bar(false)
        .collapsible(false)
        .frame(frame)
        .default_size(size)
        .max_size(room)
        .constrain(true)
        .show(ctx, |ui| {
            if header(ui, id, title) {
                open = false;
            }
            egui::Frame::new().inner_margin(body_margin).show(ui, |ui| {
                let actions = theme::CONTROL_HEIGHT + 2.0 * ui.spacing().item_spacing.y + 8.0;
                let body = (ui.available_height() - actions).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_height(body);
                        contents(ui, Part::Body);
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), theme::CONTROL_HEIGHT),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| contents(ui, Part::Actions),
                );
            });
        });
    open
}

/// A dialog window's header: its title centred on a bar the width of the window, a
/// line under it, and a close button at its end. The bar is left to the window, so a
/// drag on it moves the dialog. Returns whether the close button was clicked.
fn header(ui: &mut egui::Ui, id: &str, title: &str) -> bool {
    let (bar, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::HEADER_HEIGHT + 4.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let corner = ui.visuals().window_corner_radius;
    painter.rect_filled(
        bar,
        egui::CornerRadius {
            nw: corner.nw,
            ne: corner.ne,
            sw: 0,
            se: 0,
        },
        theme::surface(),
    );
    painter.hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        egui::Stroke::new(1.0, theme::border()),
    );
    let close = egui::Rect::from_center_size(
        egui::pos2(bar.right() - 22.0, bar.center().y),
        egui::Vec2::splat(26.0),
    );
    let title_room = (bar.width() - 2.0 * 52.0).max(0.0);
    let (galley, _) =
        crate::widgets::truncated(ui, title, theme::body_bold(), theme::fg(), title_room);
    let top = theme::galley_top(ui.ctx(), &galley, bar.center().y);
    painter.galley(
        egui::pos2(bar.center().x - galley.size().x / 2.0, top),
        galley,
        theme::fg(),
    );
    let response = ui
        .interact(close, egui::Id::unique((id, "close")), egui::Sense::click())
        .on_hover_text(editor_core::i18n::tr("Close"));
    let painter = ui.painter();
    if response.hovered() {
        let fill = if response.is_pointer_button_down_on() {
            theme::button_pressed()
        } else {
            theme::button_hover()
        };
        painter.rect_filled(close, egui::CornerRadius::same(theme::radius()), fill);
    }
    let colour = if response.hovered() {
        theme::fg()
    } else {
        theme::dim()
    };
    let x = close.shrink(8.5);
    let stroke = egui::Stroke::new(1.5, colour);
    painter.line_segment([x.left_top(), x.right_bottom()], stroke);
    painter.line_segment([x.right_top(), x.left_bottom()], stroke);
    response.clicked()
}

/// [`show`] for a modal dialog: while it is open the
/// main window takes no clicks or drags, and a press there brings the dialog forward
/// instead. Call it from the main window's pass.
pub fn show_modal(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    contents: impl FnMut(&mut egui::Ui, Part),
) -> bool {
    block_main_window(ctx, id);
    show(ctx, id, title, size, contents)
}

/// Covers the main window with an invisible modal layer for the dialog `id`.
fn block_main_window(ctx: &egui::Context, id: &str) {
    if ctx.embed_viewports() {
        // The dialog is a floating window inside the main one, which the layer would
        // block too.
        return;
    }
    let blocker = egui::Modal::new(egui::Id::unique((id, "modal-blocker")))
        .backdrop_color(egui::Color32::TRANSPARENT)
        .frame(egui::Frame::NONE)
        .show(ctx, |_| {});
    // A press on the main window, or the main window getting the keyboard back, hands it
    // to the dialog, which keeps it while it is open.
    let main_focused = ctx.input(|i| i.focused);
    if blocker.backdrop_response.is_pointer_button_down_on() || main_focused {
        ctx.send_viewport_cmd_to(
            egui::ViewportId::from_hash_of(id),
            egui::ViewportCommand::Focus,
        );
    }
}

/// Whether Escape or Enter was pressed in the dialog this frame, for dialogs that close on
/// one and run their default button on the other.
/// Enter in a multi-line field does not count.
pub fn keys(ui: &egui::Ui) -> (bool, bool) {
    ui.input(|i| {
        (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Enter) && i.modifiers.is_none(),
        )
    })
}

/// A dialog button, sized to its label.
pub fn button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let width = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_string(), theme::body(), theme::fg())
            .size()
            .x
    }) + 32.0;
    theme::button(ui, label, width.max(90.0), enabled)
}

/// A dialog's default action: [`button`] in the accent, as wide as an ordinary one.
pub fn accent_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    crate::form::accent_button(ui, label, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        id: Option<&str>,
        size: [f32; 2],
        events: Vec<egui::Event>,
    ) -> Option<egui::Rect> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(100.0, 50.0),
                    egui::vec2(1200.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                if let Some(id) = id {
                    show(ui.ctx(), id, id, size, |ui, part| match part {
                        Part::Body => {
                            ui.label("Content");
                        }
                        Part::Actions => {
                            button(ui, "Close", true);
                        }
                    });
                }
            },
        );
        output.textures_delta.clear();
        id.and_then(|id| ctx.memory(|m| m.area_rect(egui::Id::unique(id))))
    }
    #[test]
    fn dialogs_center_on_open_and_reopen_but_stay_movable() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        theme::install(&ctx);
        for (id, size) in [
            ("about", [460.0, 360.0]),
            ("updates", [600.0, 420.0]),
            ("manual", [1100.0, 760.0]),
        ] {
            frame(&ctx, Some(id), size, vec![]);
            let initial = frame(&ctx, Some(id), size, vec![]).unwrap();
            assert!(
                initial.center().distance(egui::pos2(700.0, 500.0)) < 2.0,
                "{id}: {initial:?}"
            );
            // Register the movable title bar after the initial placement pass.
            frame(&ctx, Some(id), size, vec![]);
            let at = egui::pos2(initial.center().x, initial.top() + 10.0);
            frame(
                &ctx,
                Some(id),
                size,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            let to = at + egui::vec2(30.0, 25.0);
            frame(&ctx, Some(id), size, vec![egui::Event::PointerMoved(to)]);
            let moved = frame(
                &ctx,
                Some(id),
                size,
                vec![egui::Event::PointerButton {
                    pos: to,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            )
            .unwrap();
            assert!(
                moved.center().distance(initial.center()) > 10.0,
                "dialog could not move: {id}"
            );
            let kept = frame(&ctx, Some(id), size, vec![]).unwrap();
            assert!(
                kept.center().distance(moved.center()) < 2.0,
                "dialog jumped back while open"
            );
            frame(&ctx, None, size, vec![]);
            let reopened = frame(&ctx, Some(id), size, vec![]).unwrap();
            assert!(
                reopened.center().distance(initial.center()) < 2.0,
                "did not recenter: {id} {reopened:?}"
            );
            frame(&ctx, None, size, vec![]);
        }
    }
}
