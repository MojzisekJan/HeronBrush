//! The 3D view in a window of its own (`detached_view.rs`).
//!
//! The main window's 3D view moves into the window, so the camera, the GPU resources and
//! the animation gizmo are the same ones and putting it back keeps the pose. Its header
//! carries the shading, the grid size and grid menu, and a menu with the view toggles,
//! the camera presets, full screen, Place camera, Render image and Return to main
//! window. The keyframe panel docks under it while it is apart.

use editor_core::i18n::tr;
use editor_core::view_filters::ViewFilters;
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke};
use geometry_math::vec::Vec3d;

use crate::app_actions::Action;
use crate::chrome::{self, text_width};
use crate::menu::{self, Entry};
use crate::{context_menu, theme, App, DetachedWindow};

/// The header's height.
const HEADER: f32 = 34.0;

/// A popup open from the header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    Menu,
    GridSize,
    GridSettings,
}

/// How the faces are drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shading {
    Materials,
    Solid,
    Wireframe,
}

impl Shading {
    fn of(filters: &ViewFilters) -> Self {
        if filters.wireframe {
            Shading::Wireframe
        } else if filters.textured_faces {
            Shading::Materials
        } else {
            Shading::Solid
        }
    }
    fn apply(self, filters: &mut ViewFilters) {
        filters.wireframe = self == Shading::Wireframe;
        filters.textured_faces = self == Shading::Materials;
    }
    fn label(self) -> &'static str {
        match self {
            Shading::Materials => tr("Materials"),
            Shading::Solid => tr("Solid view"),
            Shading::Wireframe => tr("Wireframe"),
        }
    }
}

/// What the window's menu asks for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Do {
    Shading(Shading),
    /// One of [`FILTERS`], toggled.
    Filter(usize),
    /// Frames the selection.
    Focus,
    /// Looks along one of [`LOOKS`] at the selection.
    Look(usize),
    Fullscreen,
    App(Action),
}

type Read = fn(&ViewFilters) -> bool;
type Write = fn(&mut ViewFilters, bool);

/// The View submenu's toggles, in menu order.
fn filters() -> [(&'static str, Read, Write); 7] {
    [
        (
            tr("Show omitted faces"),
            |f| f.show_export_hidden,
            |f, on| f.show_export_hidden = on,
        ),
        (
            tr("Preview animations"),
            |f| f.preview_motion,
            |f, on| f.preview_motion = on,
        ),
        (tr("Edges"), |f| f.show_edges, |f, on| f.show_edges = on),
        (
            tr("Group outlines"),
            |f| f.show_group_bounds,
            |f, on| f.show_group_bounds = on,
        ),
        (
            tr("Point entities"),
            |f| f.show_point_entities,
            |f, on| f.show_point_entities = on,
        ),
        (
            tr("Entity models"),
            |f| f.show_point_entity_models,
            |f, on| f.show_point_entity_models = on,
        ),
        (
            tr("Brush entity outlines"),
            |f| f.show_brush_entity_bounds,
            |f, on| f.show_brush_entity_bounds = on,
        ),
    ]
}

/// The camera presets: Front, Side, Top.
fn looks() -> [(&'static str, Vec3d); 3] {
    [
        (tr("Front"), Vec3d::new(0.0, 1.0, 0.0)),
        (tr("Side"), Vec3d::new(-1.0, 0.0, 0.0)),
        (tr("Top"), Vec3d::new(0.0, 0.0, -1.0)),
    ]
}

/// The window's menu.
pub fn entries(filters_now: &ViewFilters, has_selection: bool, fullscreen: bool) -> Vec<Entry<Do>> {
    use egui_phosphor::regular as icon;
    let shading = Shading::of(filters_now);
    let mut entries: Vec<Entry<Do>> = [Shading::Materials, Shading::Solid, Shading::Wireframe]
        .into_iter()
        .map(|mode| Entry::item(mode.label(), Do::Shading(mode)).checked(shading == mode))
        .collect();
    entries.push(Entry::sub(
        tr("View"),
        filters()
            .iter()
            .enumerate()
            .map(|(index, (label, read, _))| {
                Entry::item(*label, Do::Filter(index)).checked(read(filters_now))
            })
            .collect(),
    ));
    entries.push(Entry::Separator);
    entries.push(
        Entry::item(tr("Focus on selection"), Do::Focus)
            .icon(icon::FRAME_CORNERS)
            .enabled(has_selection),
    );
    for (index, (label, _)) in looks().iter().enumerate() {
        entries.push(Entry::item(*label, Do::Look(index)));
    }
    entries.push(Entry::Separator);
    entries.push(
        Entry::item(
            tr("Place camera from 3D view"),
            Do::App(Action::PlaceCamera),
        )
        .icon(icon::VIDEO_CAMERA)
        .keys(&menu::ctrl("Alt+0")),
    );
    entries.push(Entry::item(tr("Full screen (F11)"), Do::Fullscreen).checked(fullscreen));
    entries.push(Entry::item(tr("Render image"), Do::App(Action::Render)).icon(icon::IMAGE));
    entries.push(
        Entry::item(tr("Return to main window"), Do::App(Action::Detach3d))
            .icon(icon::ARROW_SQUARE_IN),
    );
    entries
}

/// Points `camera` along a preset, or frames the selection (`detached_view.rs`).
pub fn aim(
    camera: &mut editor_core::camera3d::Camera3d,
    bounds: Option<geometry_math::bbox::Bbox3d>,
    direction: Option<Vec3d>,
    size: (f64, f64),
) {
    if let Some(direction) = direction {
        let pivot = bounds
            .map(|b| b.center())
            .unwrap_or(camera.position + camera.direction() * 256.0);
        let distance = geometry_math::vec::length(camera.position - pivot).max(16.0);
        camera.look_along(direction);
        camera.position = pivot - camera.direction() * distance;
    }
    if let Some(bounds) = bounds {
        camera.frame_in_viewport(&bounds, size.0, size.1);
    }
}

impl App {
    /// Moves the 3D view between the windows as the switch asks, and draws its window.
    pub(crate) fn detached_window(&mut self, ctx: &egui::Context) {
        if self.chrome.detach_3d && self.detached.is_none() {
            if let Some(view) = self.view3d.take() {
                self.detached = Some(DetachedWindow {
                    view,
                    menu: None,
                    popup: None,
                });
            }
        }
        if !self.chrome.detach_3d {
            if let Some(detached) = self.detached.take() {
                self.view3d = Some(detached.view);
                self.window.borrow_mut().panes.focused = Some(0);
            }
            return;
        }
        let edges = self.picked_edges();
        let Some(detached) = &mut self.detached else {
            return;
        };
        let title = format!(
            "{} — {}",
            tr("3D view"),
            self.editor.document.borrow().window_title()
        );
        let mut viewport_size = (1.0, 1.0);
        let mut close = false;
        let mut input = None;
        let mut menu_action = None;
        let mut picked: Option<Do> = None;
        let editor = &self.editor;
        let animation = &mut self.animation;
        let builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1100.0, 760.0])
            .with_app_id(crate::APP_ID);
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("heron-3d-window"),
            builder,
            |ui, class| {
                let area = if class == egui::ViewportClass::EmbeddedWindow {
                    ui.allocate_exact_size(egui::vec2(980.0, 600.0), Sense::hover())
                        .0
                } else {
                    ui.max_rect()
                };
                ui.painter().rect_filled(area, 0.0, theme::bg());
                let header = Rect::from_min_size(area.min, egui::vec2(area.width(), HEADER));
                let mut body =
                    Rect::from_min_max(egui::pos2(area.left(), header.bottom()), area.max);
                let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if let Some(chosen) =
                    header_bar(ui, header, detached, editor, fullscreen, animation.open)
                {
                    picked = Some(chosen);
                }
                if animation.open && !animation.detached {
                    let height = (body.height() * 0.4).clamp(180.0, 340.0);
                    let rect = Rect::from_min_max(
                        egui::pos2(body.left(), body.bottom() - height),
                        body.max,
                    );
                    let mut panel = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    panel.set_clip_rect(rect);
                    animation.show_docked(&mut panel, editor);
                    ui.painter().hline(
                        rect.x_range(),
                        rect.top() + 0.5,
                        Stroke::new(1.0, theme::border()),
                    );
                    body.max.y = rect.top();
                }
                viewport_size = (f64::from(body.width()), f64::from(body.height()));
                detached.view.show(ui, body);
                if let Some((at, probe)) = detached.view.context_click.take() {
                    let mut sections =
                        editor_core::context_menu::build(&editor.document.borrow(), &probe);
                    sections.splice(0..0, editor_core::context_menu::edge_section(edges));
                    detached.menu = (!sections.is_empty()).then_some(context_menu::Open {
                        at,
                        sections,
                        frame: ui.ctx().cumulative_frame_nr(),
                    });
                }
                if let Some(menu) = &detached.menu {
                    let (chosen, dismiss) = context_menu::show(ui.ctx(), menu);
                    menu_action = chosen;
                    if dismiss {
                        detached.menu = None;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::F11) && i.modifiers.is_none()) {
                    picked = Some(Do::Fullscreen);
                }
                if detached.menu.is_none() && detached.popup.is_none() && ui.input(|i| i.focused) {
                    // Asked outside `input`, which holds the context's lock: asking inside it
                    // deadlocked the moment the detached window had the keyboard.
                    let field_focused = ui.ctx().egui_wants_keyboard_input();
                    input = Some(ui.input(|i| (i.events.clone(), i.modifiers, field_focused)));
                }
                if picked == Some(Do::Fullscreen) {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
                }
                if ui.input(|i| i.viewport().close_requested()) {
                    close = true;
                }
            },
        );
        self.window.borrow_mut().perspective_size = Some(viewport_size);
        if let Some((events, modifiers, field_focused)) = input {
            let scene = self
                .detached
                .as_ref()
                .map(|d| std::rc::Rc::clone(&d.view.scene));
            let main_focus = self.window.borrow().panes.focused;
            self.window.borrow_mut().panes.focused = Some(0);
            self.key_events(events, modifiers, scene, field_focused);
            self.window.borrow_mut().panes.focused = main_focus;
        }
        if let Some(action) = menu_action {
            self.run_context_action(ctx, action);
        }
        match picked {
            Some(Do::Shading(mode)) => mode.apply(&mut self.editor.filters.borrow_mut()),
            Some(Do::Filter(index)) => {
                let (_, read, write) = filters()[index];
                let mut filters = self.editor.filters.borrow_mut();
                let on = !read(&filters);
                write(&mut filters, on);
            }
            Some(Do::Focus) => aim(
                &mut self.editor.camera.borrow_mut(),
                self.editor.document.borrow().selection_bounds(),
                None,
                viewport_size,
            ),
            Some(Do::Look(index)) => aim(
                &mut self.editor.camera.borrow_mut(),
                self.editor.document.borrow().selection_bounds(),
                Some(looks()[index].1),
                viewport_size,
            ),
            Some(Do::App(action)) => self.action(ctx, action),
            Some(Do::Fullscreen) | None => {}
        }
        if picked.is_some() {
            ctx.request_repaint();
        }
        // Closing the window puts the view back into the main window.
        if close {
            self.chrome.detach_3d = false;
            if let Some(detached) = self.detached.take() {
                self.view3d = Some(detached.view);
            }
        }
    }
}

/// The header: the shading at the start; the grid size, grid menu and the menu at the end.
fn header_bar(
    ui: &mut egui::Ui,
    rect: Rect,
    detached: &mut DetachedWindow,
    editor: &crate::editor::Editor,
    fullscreen: bool,
    animation_open: bool,
) -> Option<Do> {
    menu::begin_frame(ui.ctx());
    ui.painter().rect_filled(rect, 0.0, theme::base());
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, theme::border()),
    );
    let cy = rect.center().y;
    let mut picked = None;

    // The shading as a segmented control.
    let current = Shading::of(&editor.filters.borrow());
    let mut x = rect.left() + 8.0;
    let modes = [Shading::Materials, Shading::Solid, Shading::Wireframe];
    let widths: Vec<f32> = modes
        .iter()
        .map(|m| text_width(ui, m.label(), theme::body()) + 20.0)
        .collect();
    let total: f32 = widths.iter().sum();
    let frame = Rect::from_min_size(Pos2::new(x, cy - 12.0), egui::vec2(total, 24.0));
    ui.painter().rect_filled(
        frame,
        CornerRadius::same(theme::radius()),
        theme::button_fill(),
    );
    ui.painter().rect_stroke(
        frame,
        CornerRadius::same(theme::radius()),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    for (mode, width) in modes.into_iter().zip(widths) {
        let slot = Rect::from_min_size(Pos2::new(x, cy - 12.0), egui::vec2(width, 24.0));
        let response = ui.interact(
            slot,
            Id::unique(("detached-shading", mode.label())),
            Sense::click(),
        );
        let on = mode == current;
        if on {
            ui.painter()
                .rect_filled(slot.shrink(2.0), CornerRadius::same(3), theme::accent());
        } else if response.hovered() {
            ui.painter().rect_filled(
                slot.shrink(2.0),
                CornerRadius::same(3),
                theme::button_hover(),
            );
        }
        crate::theme::text(
            ui.painter(),
            slot.center(),
            egui::Align2::CENTER_CENTER,
            mode.label(),
            theme::body(),
            if on { theme::on_accent() } else { theme::fg() },
        );
        if response.clicked() {
            picked = Some(Do::Shading(mode));
        }
        x += width;
    }

    // The end, right to left.
    let mut right = rect.right() - 8.0;
    let slot = Rect::from_min_size(Pos2::new(right - 28.0, cy - 12.0), egui::vec2(28.0, 24.0));
    let response = ui
        .interact(slot, Id::unique("detached-menu"), Sense::click())
        .on_hover_text(tr("3D view"));
    flat(
        ui,
        slot,
        &response,
        detached.popup.is_some_and(|(p, _)| p == Popup::Menu),
    );
    for dy in [-4.0, 0.0, 4.0] {
        ui.painter().hline(
            (slot.center().x - 6.0)..=(slot.center().x + 6.0),
            slot.center().y + dy,
            Stroke::new(1.4, theme::fg()),
        );
    }
    if response.clicked() {
        toggle(detached, Popup::Menu, slot);
    }
    right = slot.left() - 4.0;
    let slot = Rect::from_min_size(Pos2::new(right - 26.0, cy - 12.0), egui::vec2(26.0, 24.0));
    let response = ui
        .interact(slot, Id::unique("detached-grid-menu"), Sense::click())
        .on_hover_text(tr("Grid settings"));
    flat(
        ui,
        slot,
        &response,
        detached
            .popup
            .is_some_and(|(p, _)| p == Popup::GridSettings),
    );
    chrome::draw_grid_icon(ui.painter(), slot.center(), theme::fg());
    if response.clicked() {
        toggle(detached, Popup::GridSettings, slot);
    }
    right = slot.left() - 4.0;
    let label = chrome::grid_label(editor.grid.borrow().actual_size());
    let width = text_width(ui, &label, theme::body()) + 30.0;
    let slot = Rect::from_min_size(Pos2::new(right - width, cy - 12.0), egui::vec2(width, 24.0));
    let response = ui
        .interact(slot, Id::unique("detached-grid-size"), Sense::click())
        .on_hover_text(tr("Grid size (+ and -)"));
    flat(
        ui,
        slot,
        &response,
        detached.popup.is_some_and(|(p, _)| p == Popup::GridSize),
    );
    ui.painter().rect_stroke(
        slot,
        CornerRadius::same(theme::radius()),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    crate::theme::text(
        ui.painter(),
        Pos2::new(slot.left() + 9.0, cy),
        egui::Align2::LEFT_CENTER,
        &label,
        theme::body(),
        theme::fg(),
    );
    chrome::chevron(
        ui.painter(),
        Pos2::new(slot.right() - 11.0, cy),
        theme::dim(),
    );
    if response.clicked() {
        toggle(detached, Popup::GridSize, slot);
    }

    // The open popup.
    if let Some((popup, from)) = detached.popup {
        let ctx = ui.ctx().clone();
        let chosen = match popup {
            Popup::Menu => {
                let entries = entries(
                    &editor.filters.borrow(),
                    editor.document.borrow().selection_bounds().is_some(),
                    fullscreen,
                );
                menu::popup_aligned(
                    &ctx,
                    Id::unique("detached-popup-menu"),
                    Pos2::new(from.right(), from.bottom() + 4.0),
                    egui::Align2::RIGHT_TOP,
                    &entries,
                )
            }
            Popup::GridSize | Popup::GridSettings => {
                let entries = if popup == Popup::GridSize {
                    chrome::grid_sizes(&editor.grid.borrow())
                } else {
                    chrome::grid_settings(
                        &editor.grid.borrow(),
                        &editor.filters.borrow(),
                        animation_open,
                    )
                };
                menu::popup_aligned(
                    &ctx,
                    Id::unique(("detached-popup", format!("{popup:?}"))),
                    Pos2::new(from.right(), from.bottom() + 4.0),
                    egui::Align2::RIGHT_TOP,
                    &entries,
                )
                .map(Do::App)
            }
        };
        if let Some(chosen) = chosen {
            detached.popup = None;
            picked = Some(chosen);
        } else if menu::pressed_outside(&ctx, Some(from))
            || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            detached.popup = None;
        }
    }
    picked
}

fn toggle(detached: &mut DetachedWindow, popup: Popup, from: Rect) {
    detached.popup = match detached.popup {
        Some((open, _)) if open == popup => None,
        _ => Some((popup, from)),
    };
}

fn flat(ui: &egui::Ui, slot: Rect, response: &egui::Response, open: bool) {
    let fill = if open {
        theme::button_pressed()
    } else if response.hovered() {
        theme::button_hover()
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(slot, CornerRadius::same(theme::radius()), fill);
}
