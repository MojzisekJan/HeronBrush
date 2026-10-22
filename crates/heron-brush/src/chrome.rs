//! The title bar and the tool strip above the views.
//!
//! The window is undecorated (on macOS its native title bar is hidden behind the
//! content), so the app draws a compact title bar of its own: the menu bar at its start,
//! the map's title in the middle, the header indicators (engine live link, live session,
//! MCP) and the window buttons at its end. Dragging the bar moves the window and a double
//! click maximises it. The tool strip under it carries every tool, the flips, the
//! Lock/UV toggles, the active tool's settings, and the grid size and grid menu at its
//! end. Every action the bars offer goes out as an [`Action`] for the app to run.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::i18n::{tr, translation_key};
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Ui};

use crate::app_actions::Action;
use crate::menu::{self, Entry};
use crate::theme;

pub use editor_core::tool::Tool;

/// A tool's icon, as egui loads it.
fn tool_icon(tool: Tool) -> (String, &'static [u8]) {
    (
        format!("bytes://tool-{}.svg", tool.name().replace(' ', "-")),
        tool.icon_svg().as_bytes(),
    )
}

const FLIP_HORIZONTAL: (&str, &[u8]) = (
    "bytes://flip-horizontal.svg",
    include_bytes!("../assets/tools/flip-horizontal.svg"),
);
const FLIP_VERTICAL: (&str, &[u8]) = (
    "bytes://flip-vertical.svg",
    include_bytes!("../assets/tools/flip-vertical.svg"),
);
/// The application's logo, at the start of the title bar and on the welcome screen.
pub const LOGO: (&str, &[u8]) = (
    "bytes://heron-logo.svg",
    include_bytes!("../assets/logo.svg"),
);

/// `grid::MIN_SIZE..=grid::MAX_SIZE`, as powers of two.
pub const GRID_EXPONENTS: std::ops::RangeInclusive<i32> =
    editor_core::grid::MIN_SIZE..=editor_core::grid::MAX_SIZE;

/// The menu bar's menus, in order.
const MENUS: [&str; 8] = [
    translation_key("File"),
    translation_key("Edit"),
    translation_key("View"),
    translation_key("Geometry"),
    translation_key("Animation"),
    translation_key("Map"),
    translation_key("Collaboration"),
    translation_key("Help"),
];

/// A popup the bars have open, and the button it hangs from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    /// One of [`MENUS`].
    Menu(usize),
    /// Every menu, each as a submenu: the menu bar folded into one button where the
    /// window is too narrow for it (a phone).
    AllMenus,
    /// The grid size chooser.
    GridSize,
    /// The grid and view toggles.
    GridSettings,
}

/// What the header shows at its end while something runs in the background. A click
/// on one runs its action.
pub struct Indicator {
    pub label: String,
    pub tooltip: String,
    pub action: Action,
}

/// What the bars need to know about the rest of the window this frame.
#[derive(Default)]
pub struct Status {
    /// The map's title, for the middle of the title bar.
    pub title: String,
    /// A point file is loaded and can be read again.
    pub can_reload_point: bool,
    /// A point file is loaded.
    pub point_loaded: bool,
    /// A portal file is loaded and can be read again.
    pub can_reload_portal: bool,
    /// A portal file is loaded.
    pub portal_loaded: bool,
    /// There is something for Repeat to do again.
    pub can_repeat: bool,
    /// The keyframe animation panel is open.
    pub animation_open: bool,
    pub uv_window_open: bool,
    /// The inspector column is showing.
    pub inspector_shown: bool,
    /// The background work to show at the header's end.
    pub indicators: Vec<Indicator>,
}

/// The bars' view of the editor: the tool in hand, the grid and the document, all shared
/// with the views, and the window-level switches.
pub struct Chrome {
    /// The tool in hand, shared with the views.
    pub tool: Rc<RefCell<Tool>>,
    /// The grid, shared with the views.
    pub grid: Rc<RefCell<editor_core::grid::Grid>>,
    /// The document, for the flips.
    pub document: Rc<RefCell<editor_core::document::Document>>,
    /// The clip tool, which leaving the tool resets.
    pub clip: Rc<RefCell<editor_core::clip_tool::ClipTool>>,
    /// Which shape the brush tool draws.
    pub shape_parameters: Rc<RefCell<editor_core::shape_tool::ShapeParameters>>,
    /// What the fillet rounds by.
    pub fillet: Rc<RefCell<editor_core::fillet_tool::FilletParameters>>,
    /// The handles the fillet's Apply rounds.
    pub handles: Rc<RefCell<editor_core::handle_tool::HandleManager>>,
    /// The 3D view in a window of its own.
    pub detach_3d: bool,
    /// What a click in the bars asked for, for the app to run this frame.
    pub action: Option<Action>,
    pub filters: Rc<RefCell<editor_core::view_filters::ViewFilters>>,
    /// The popup open, and the rectangle of the button it hangs from.
    pub popup: Option<(Popup, Rect)>,
    /// Whether the fillet's settings window is open; it closes with the tool.
    pub fillet_window: bool,
    /// Whether the sculpt brush's settings window is open; it closes with the tool.
    pub sculpt_window: bool,
}

/// `format_size`: whole sizes without a fraction, the small ones as they are.
pub fn format_size(size: f64) -> String {
    if size >= 1.0 {
        format!("{size:.0}")
    } else {
        format!("{size}")
    }
}

/// A shortcut without the command key, as the menus print it.
fn plain(keys: &str) -> String {
    let (modifiers, key) = keys.rsplit_once('+').map_or(("", keys), |(m, k)| (m, k));
    let key = menu::key_label(key);
    if cfg!(target_os = "macos") {
        format!("{}{key}", if modifiers.contains("Alt") { "⌥" } else { "" })
    } else if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{modifiers}+{key}")
    }
}

fn shortcut(id: &str) -> String {
    editor_core::preferences::pref(|prefs| {
        let bindings =
            editor_core::custom_shortcuts::load(&prefs.keyboard_bindings).unwrap_or_default();
        let command = editor_core::custom_shortcuts::COMMANDS
            .iter()
            .find(|command| command.id == id)
            .unwrap();
        let keys = bindings
            .get(id)
            .map(String::as_str)
            .unwrap_or_else(|| editor_core::custom_shortcuts::default_chord(command, prefs.keymap));
        if let Some(keys) = keys.strip_prefix("Ctrl+") {
            menu::ctrl(keys)
        } else {
            plain(keys)
        }
    })
}

/// Every menu of the menu bar, with what is checked and what is enabled this frame.
pub fn menus(
    status: &Status,
    detached: bool,
    grid: &editor_core::grid::Grid,
    filters: &editor_core::view_filters::ViewFilters,
) -> Vec<Vec<Entry<Action>>> {
    use egui_phosphor::regular as icon;
    use menu::ctrl;
    let item = |label: &'static str, action| Entry::item(label, action);
    vec![
        // File
        vec![
            item(tr("Welcome"), Action::Welcome).icon(icon::HOUSE),
            Entry::Separator,
            item(tr("New map"), Action::New)
                .icon(icon::FILE_PLUS)
                .keys(&shortcut("new")),
            item(tr("Open Map"), Action::Open)
                .icon(icon::FOLDER_OPEN)
                .keys(&shortcut("open")),
            Entry::Separator,
            item(tr("Save"), Action::Save)
                .icon(icon::FLOPPY_DISK)
                .keys(&shortcut("save")),
            item(tr("Save As"), Action::SaveAs)
                .icon(icon::FLOPPY_DISK_BACK)
                .keys(&shortcut("save_as")),
            item(tr("Revert to saved map"), Action::Revert).icon(icon::ARROW_COUNTER_CLOCKWISE),
            Entry::Separator,
            item(tr("Export"), Action::Export)
                .icon(icon::EXPORT)
                .keys(&shortcut("export")),
            #[cfg(not(target_arch = "wasm32"))]
            item(tr("Build and run"), Action::Build)
                .icon(icon::PLAY)
                .keys(&shortcut("build")),
            #[cfg(target_arch = "wasm32")]
            item(tr("Play in a new tab"), Action::Playtest).icon(icon::PLAY),
            item(tr("Render image"), Action::Render).icon(icon::IMAGE),
            Entry::Separator,
            item(tr("Plugins"), Action::Plugins).icon(icon::PUZZLE_PIECE),
            item(tr("Preferences"), Action::Preferences)
                .icon(icon::GEAR_SIX)
                .keys(&shortcut("preferences")),
        ],
        // Edit
        vec![
            item(tr("Undo"), Action::Undo)
                .icon(icon::ARROW_U_UP_LEFT)
                .keys(&shortcut("undo")),
            item(tr("Redo"), Action::Redo)
                .icon(icon::ARROW_U_UP_RIGHT)
                .keys(&shortcut("redo")),
            Entry::Separator,
            item(tr("Cut"), Action::Cut)
                .icon(icon::SCISSORS)
                .keys(&shortcut("cut")),
            item(tr("Copy"), Action::Copy)
                .icon(icon::COPY)
                .keys(&shortcut("copy")),
            item(tr("Paste"), Action::Paste)
                .icon(icon::CLIPBOARD_TEXT)
                .keys(&shortcut("paste")),
            item(tr("Duplicate"), Action::Duplicate)
                .icon(icon::COPY_SIMPLE)
                .keys(&shortcut("duplicate")),
            item(tr("Delete"), Action::Delete)
                .icon(icon::TRASH)
                .keys(&shortcut("delete")),
            Entry::Separator,
            item(tr("Select all"), Action::SelectAll)
                .icon(icon::SELECTION_ALL)
                .keys(&shortcut("select_all")),
            Entry::Separator,
            item(tr("Repeat last commands"), Action::Repeat)
                .icon(icon::REPEAT)
                .keys(&shortcut("repeat"))
                .enabled(status.can_repeat),
            item(tr("Forget repeatable commands"), Action::ClearRepeat)
                .icon(icon::BROOM)
                .enabled(status.can_repeat),
        ],
        // View
        vec![
            item(tr("UV editor in separate window"), Action::UvEditor)
                .checked(status.uv_window_open),
            item(tr("3D view in separate window"), Action::Detach3d)
                .checked(detached)
                .enabled(!cfg!(target_os = "android")),
            item(tr("Go to"), Action::GoTo).icon(icon::CROSSHAIR),
            item(tr("Entity links"), Action::EntityGraph).icon(icon::FLOW_ARROW),
            Entry::Separator,
            item(tr("Show inspector"), Action::ToggleInspector).checked(status.inspector_shown),
            item(tr("Next view"), Action::NextView)
                .icon(icon::SQUARES_FOUR)
                .keys(&plain("Space")),
            Entry::Separator,
            item(tr("Snap to grid"), Action::SnapGrid)
                .keys(&plain("Alt+0"))
                .checked(grid.snap()),
            item(tr("Show grid"), Action::ShowGrid)
                .keys("0")
                .checked(grid.visible()),
            Entry::Separator,
            item(tr("Preview animations"), Action::PreviewAnimations)
                .keys(&shortcut("preview"))
                .checked(filters.preview_motion),
            item(tr("Show omitted faces"), Action::ShowOmitted).checked(filters.show_export_hidden),
        ],
        // Geometry
        vec![
            item(tr("Import model"), Action::ImportModel)
                .icon(icon::CUBE)
                .enabled(!cfg!(target_arch = "wasm32")),
            item(tr("Insert 3D text"), Action::Text).icon(icon::TEXT_T),
            item(tr("Join corner"), Action::Corner).icon(icon::ANGLE),
            item(tr("Array"), Action::Array).icon(icon::DOTS_NINE),
            item(tr("Split into columns"), Action::SplitColumns).icon(icon::MOUNTAINS),
            Entry::Separator,
            item(tr("Convert faces to patches"), Action::Patches).icon(icon::BEZIER_CURVE),
            item(tr("Move selection out of its entity"), Action::Structural)
                .icon(icon::ARROW_SQUARE_OUT),
            item(tr("Sync linked groups"), Action::Sync).icon(icon::ARROWS_CLOCKWISE),
        ],
        // Animation
        vec![
            item(tr("Keyframe animation"), Action::Animation).checked(status.animation_open),
            item(tr("Preview animations"), Action::PreviewAnimations)
                .keys(&shortcut("preview"))
                .checked(filters.preview_motion),
            Entry::Separator,
            item(tr("Place camera from 3D view"), Action::PlaceCamera)
                .icon(icon::VIDEO_CAMERA)
                .keys(&ctrl("Alt+0")),
        ],
        // Map
        vec![
            item(tr("Load point file"), Action::LoadPoint).icon(icon::PATH),
            item(tr("Reload point file"), Action::ReloadPoint)
                .icon(icon::ARROW_CLOCKWISE)
                .enabled(status.can_reload_point),
            item(tr("Unload point file"), Action::UnloadPoint)
                .icon(icon::X_CIRCLE)
                .enabled(status.point_loaded),
            Entry::Separator,
            item(tr("Load portal file"), Action::LoadPortal).icon(icon::DOOR_OPEN),
            item(tr("Reload portal file"), Action::ReloadPortal)
                .icon(icon::ARROW_CLOCKWISE)
                .enabled(status.can_reload_portal),
            item(tr("Unload portal file"), Action::UnloadPortal)
                .icon(icon::X_CIRCLE)
                .enabled(status.portal_loaded),
        ],
        // Collaboration
        vec![
            item(tr("Live session"), Action::Session).icon(icon::USERS_THREE),
            item(tr("Serve for an agent (MCP)"), Action::Mcp).icon(icon::ROBOT),
        ],
        // Help
        vec![
            item(tr("Manual"), Action::Manual).icon(icon::BOOK_OPEN),
            item(tr("Application updates"), Action::Updates).icon(icon::ARROW_CIRCLE_DOWN),
            Entry::Separator,
            item(tr("About"), Action::About).icon(icon::INFO),
        ],
    ]
}

/// The grid menu: the same toggles in both editor windows (`grid_settings_menu`).
pub fn grid_settings(
    grid: &editor_core::grid::Grid,
    filters: &editor_core::view_filters::ViewFilters,
    animation_open: bool,
) -> Vec<Entry<Action>> {
    vec![
        Entry::item(tr("Snap to grid"), Action::SnapGrid).checked(grid.snap()),
        Entry::item(tr("Show grid"), Action::ShowGrid).checked(grid.visible()),
        Entry::Separator,
        Entry::item(tr("Preview animations"), Action::PreviewAnimations)
            .keys(&shortcut("preview"))
            .checked(filters.preview_motion),
        Entry::item(tr("Show omitted faces"), Action::ShowOmitted)
            .checked(filters.show_export_hidden),
        Entry::item(tr("Keyframe animation"), Action::Animation).checked(animation_open),
    ]
}

/// The grid sizes, the current one checked.
pub fn grid_sizes(grid: &editor_core::grid::Grid) -> Vec<Entry<Action>> {
    GRID_EXPONENTS
        .map(|exponent| {
            Entry::item(grid_label(2f64.powi(exponent)), Action::GridSize(exponent))
                .checked(exponent == grid.size())
        })
        .collect()
}

/// "Grid 16".
pub fn grid_label(size: f64) -> String {
    format!("{} {}", tr("Grid"), format_size(size))
}

impl Chrome {
    /// Picks a tool: leaving the clip tool forgets what was
    /// placed, and pressing the active tool again returns to selection.
    pub fn pick(&self, tool: Tool) {
        let current = *self.tool.borrow();
        let next = if current == tool && tool != Tool::Select {
            Tool::Select
        } else {
            tool
        };
        if current == Tool::Clip && next != Tool::Clip {
            self.clip.borrow_mut().reset();
        }
        *self.tool.borrow_mut() = next;
    }

    /// Opens `popup` hanging from `from`, or closes it if it is the one open.
    fn toggle(&mut self, popup: Popup, from: Rect) {
        self.popup = match self.popup {
            Some((open, _)) if open == popup => None,
            _ => Some((popup, from)),
        };
    }

    /// The title bar across `rect`.
    pub fn header(&mut self, ui: &mut Ui, rect: Rect, status: &Status) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::panel_base());
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::border()),
        );
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        title_bar_drag(ui, rect, "headerbar", maximized);

        // The logo, then the menus.
        let left = rect.left() + traffic_light_inset();
        let logo = Rect::from_center_size(
            Pos2::new(left + 20.0, rect.center().y),
            egui::Vec2::splat(18.0),
        );
        egui::Image::from_bytes(LOGO.0, LOGO.1).paint_at(ui, logo);
        let mut x = left + 36.0;
        let open_menu = match self.popup {
            Some((Popup::Menu(index), _)) => Some(index),
            _ => None,
        };
        // Too narrow for the menu bar (a phone): one button holds every menu.
        let bar_width: f32 = MENUS
            .iter()
            .map(|name| text_width(ui, tr(name), theme::body()) + 18.0)
            .sum();
        let folded = x + bar_width > rect.right() - 160.0;
        if folded {
            let slot =
                Rect::from_min_size(Pos2::new(x, rect.center().y - 14.0), egui::vec2(36.0, 28.0));
            let response = ui.interact(slot, ui.id().with("menu-bar-folded"), Sense::click());
            let open = matches!(self.popup, Some((Popup::AllMenus, _)));
            if response.clicked() {
                self.toggle(Popup::AllMenus, slot);
            }
            let fill = if open {
                theme::surface()
            } else if response.hovered() {
                theme::button_hover()
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(slot, CornerRadius::same(4), fill);
            for dy in [-5.0, 0.0, 5.0] {
                let y = slot.center().y + dy;
                ui.painter().hline(
                    (slot.center().x - 8.0)..=(slot.center().x + 8.0),
                    y,
                    Stroke::new(1.6, theme::fg()),
                );
            }
            x = slot.right() + 4.0;
        }
        for (index, name) in MENUS.iter().enumerate().filter(|_| !folded) {
            let label = tr(name);
            let width = text_width(ui, label, theme::body()) + 16.0;
            let slot = Rect::from_min_size(
                Pos2::new(x, rect.center().y - 12.0),
                egui::vec2(width, 24.0),
            );
            let response = ui.interact(slot, ui.id().with(("menu-bar", index)), Sense::click());
            let open = open_menu == Some(index);
            // With a menu open, pointing at another title opens that one, as menu bars do.
            if response.clicked() || (open_menu.is_some() && !open && response.hovered()) {
                self.toggle(Popup::Menu(index), slot);
            }
            let fill = if open {
                theme::surface()
            } else if response.hovered() {
                theme::button_hover()
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(slot, CornerRadius::same(4), fill);
            let colour = if open || response.hovered() {
                theme::fg()
            } else {
                theme::fg().gamma_multiply(0.85)
            };
            crate::theme::text(
                ui.painter(),
                slot.center(),
                egui::Align2::CENTER_CENTER,
                label,
                theme::body(),
                colour,
            );
            x += width + 2.0;
        }
        let menus_end = x;

        // The end, right to left: the window buttons, the inspector's button on a touch
        // screen, then the indicators.
        let mut right = window_buttons(ui, rect, maximized) - 8.0;
        if crate::touch::enabled(ui.ctx()) {
            let slot = Rect::from_min_size(
                Pos2::new(right - 36.0, rect.center().y - 14.0),
                egui::vec2(36.0, 28.0),
            );
            let response = ui
                .interact(slot, ui.id().with("inspector-toggle"), Sense::click())
                .on_hover_text(tr("Show inspector"));
            if response.clicked() {
                self.action = Some(Action::ToggleInspector);
            }
            let on = status.inspector_shown;
            let fill = if on {
                theme::surface()
            } else if response.hovered() {
                theme::button_hover()
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(slot, CornerRadius::same(4), fill);
            // A window with its right-hand column filled in while it shows.
            let icon = Rect::from_center_size(slot.center(), egui::vec2(18.0, 14.0));
            let colour = if on {
                theme::accent_text()
            } else {
                theme::fg()
            };
            ui.painter().rect_stroke(
                icon,
                CornerRadius::same(2),
                Stroke::new(1.4, colour),
                egui::StrokeKind::Inside,
            );
            let column = Rect::from_min_max(Pos2::new(icon.right() - 7.0, icon.top()), icon.max);
            if on {
                ui.painter()
                    .rect_filled(column, CornerRadius::same(2), colour);
            } else {
                ui.painter()
                    .vline(column.left(), icon.y_range(), Stroke::new(1.4, colour));
            }
            right = slot.left() - 6.0;
        }
        for (index, indicator) in status.indicators.iter().enumerate() {
            let width = text_width(ui, &indicator.label, theme::small_bold()) + 20.0;
            let slot = Rect::from_min_size(
                Pos2::new(right - width, rect.center().y - 11.0),
                egui::vec2(width, 22.0),
            );
            let response = ui
                .interact(slot, ui.id().with(("indicator", index)), Sense::click())
                .on_hover_text(&indicator.tooltip);
            let fill = if response.hovered() {
                theme::accent_hover()
            } else {
                theme::accent()
            };
            ui.painter().rect_filled(slot, CornerRadius::same(11), fill);
            ui.painter().circle_filled(
                Pos2::new(slot.left() + 9.0, slot.center().y),
                3.0,
                Color32::from_rgb(0x9b, 0xf0, 0xb4),
            );
            crate::theme::text(
                ui.painter(),
                Pos2::new(slot.left() + 16.0, slot.center().y),
                egui::Align2::LEFT_CENTER,
                &indicator.label,
                theme::small_bold(),
                theme::on_accent(),
            );
            if response.clicked() {
                self.action = Some(indicator.action);
            }
            right = slot.left() - 6.0;
        }

        // The title, centred on the window unless that would run into either end.
        let galley = ui
            .painter()
            .layout_no_wrap(status.title.clone(), theme::body(), theme::dim());
        let room = (menus_end + 16.0)..=(right - 16.0);
        if room.end() - room.start() > 80.0 {
            let width = galley.size().x.min(room.end() - room.start());
            let left = (rect.center().x - width / 2.0).clamp(*room.start(), room.end() - width);
            let clip = Rect::from_min_max(
                Pos2::new(left, rect.top()),
                Pos2::new(left + width, rect.bottom()),
            );
            ui.painter().with_clip_rect(clip).galley(
                Pos2::new(
                    left,
                    crate::theme::galley_top(ui.ctx(), &galley, rect.center().y),
                ),
                galley,
                theme::dim(),
            );
        }
    }

    /// Shows the open popup, if any. Call after both bars.
    pub fn popups(&mut self, ui: &mut Ui, status: &Status) {
        let Some((popup, from)) = self.popup else {
            return;
        };
        let ctx = ui.ctx().clone();
        let entries = match popup {
            Popup::Menu(index) => menus(
                status,
                self.detach_3d,
                &self.grid.borrow(),
                &self.filters.borrow(),
            )
            .swap_remove(index),
            Popup::AllMenus => MENUS
                .iter()
                .zip(menus(
                    status,
                    self.detach_3d,
                    &self.grid.borrow(),
                    &self.filters.borrow(),
                ))
                .map(|(name, entries)| Entry::Sub {
                    label: tr(name).to_string(),
                    entries,
                })
                .collect(),
            Popup::GridSize => grid_sizes(&self.grid.borrow()),
            Popup::GridSettings => grid_settings(
                &self.grid.borrow(),
                &self.filters.borrow(),
                status.animation_open,
            ),
        };
        // A menu drops from its title; the grid popups hang right-aligned from their
        // buttons at the strip's end.
        let (at, pivot) = match popup {
            Popup::Menu(_) | Popup::AllMenus => (
                Pos2::new(from.left(), from.bottom() + 6.0),
                egui::Align2::LEFT_TOP,
            ),
            _ => (
                Pos2::new(from.right(), from.bottom() + 4.0),
                egui::Align2::RIGHT_TOP,
            ),
        };
        if let Some(action) = menu::popup_aligned(
            &ctx,
            Id::new(("bar-popup", format!("{popup:?}"))),
            at,
            pivot,
            &entries,
        ) {
            self.action = Some(action);
            self.popup = None;
            return;
        }
        if menu::pressed_outside(&ctx, Some(from))
            || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.popup = None;
        }
    }

    /// The tool strip across `rect`.
    pub fn toolbar(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::panel_layer());
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::border()),
        );

        let cy = rect.center().y;
        let step = theme::TOOL_BUTTON + 2.0;
        let mut x = rect.left() + 6.0;
        for (index, tool) in Tool::ESSENTIAL
            .into_iter()
            .chain(
                Tool::ALL
                    .into_iter()
                    .filter(|t| !Tool::ESSENTIAL.contains(t)),
            )
            .enumerate()
        {
            if index == Tool::ESSENTIAL.len() {
                x = separator(ui, x, rect);
            }
            let slot = Rect::from_min_size(
                Pos2::new(x, cy - theme::TOOL_BUTTON / 2.0),
                egui::Vec2::splat(theme::TOOL_BUTTON),
            );
            self.tool_button(ui, slot, tool);
            x += step;
        }
        x = separator(ui, x, rect);
        for (icon, tip, axis) in [
            (
                FLIP_HORIZONTAL,
                tr("Flip horizontally"),
                geometry_math::util::Axis::X,
            ),
            (
                FLIP_VERTICAL,
                tr("Flip vertically"),
                geometry_math::util::Axis::Z,
            ),
        ] {
            let slot = Rect::from_min_size(
                Pos2::new(x, cy - theme::TOOL_BUTTON / 2.0),
                egui::Vec2::splat(theme::TOOL_BUTTON),
            );
            let response = ui
                .interact(slot, ui.id().with(icon.0), Sense::click())
                .on_hover_text(tip);
            if response.clicked() {
                self.document.borrow_mut().flip_selection(axis);
            }
            paint_tool(ui, slot, (icon.0.to_string(), icon.1), &response, false);
            x += step;
        }
        x = separator(ui, x, rect);

        // The locks are preferences, which the preference pane and the U key also change.
        use editor_core::preferences::{edit, pref};
        let locks: [(
            &str,
            &str,
            bool,
            fn(&mut editor_core::preferences::Preferences, bool),
        ); 2] = [
            (
                tr("Lock"),
                tr("Keep the mapping when a brush moves"),
                pref(|p| p.alignment_lock),
                |p, on| p.alignment_lock = on,
            ),
            (
                tr("UV"),
                tr("Keep the mapping when a brush turns (U)"),
                pref(|p| p.uv_lock),
                |p, on| p.uv_lock = on,
            ),
        ];
        for (label, tip, on, set) in locks {
            let width = text_width(ui, label, theme::small_bold()) + 18.0;
            let slot = Rect::from_min_size(Pos2::new(x, cy - 11.0), egui::vec2(width, 22.0));
            let response = ui
                .interact(slot, ui.id().with(("lock", label)), Sense::click())
                .on_hover_text(tip);
            if response.clicked() {
                edit(|p| set(p, !on));
            }
            toggle_pill(ui, slot, &response, label, on);
            x += width + 4.0;
        }
        if pref(|p| p.show_simulation_preview_button) {
            let running = self.filters.borrow().preview_motion;
            let label = tr(if running {
                "Stop animation preview"
            } else {
                "Start animation preview"
            });
            let width = text_width(ui, label, theme::small_bold()) + 38.0;
            let slot = Rect::from_min_size(Pos2::new(x, cy - 11.0), egui::vec2(width, 22.0));
            let response = ui
                .interact(slot, ui.id().with("simulation-preview"), Sense::click())
                .on_hover_text(tr("Preview animations"));
            if response.clicked() {
                self.action = Some(Action::PreviewAnimations);
            }
            paint_flat(ui, slot, &response, running);
            let colour = if running {
                theme::accent_text()
            } else {
                theme::fg()
            };
            let center = Pos2::new(slot.left() + 12.0, cy);
            if running {
                ui.painter().rect_filled(
                    Rect::from_center_size(center, egui::vec2(9.0, 9.0)),
                    0.0,
                    colour,
                );
            } else {
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        center + egui::vec2(-4.0, -5.0),
                        center + egui::vec2(-4.0, 5.0),
                        center + egui::vec2(5.0, 0.0),
                    ],
                    colour,
                    Stroke::NONE,
                ));
            }
            crate::theme::text(
                ui.painter(),
                Pos2::new(slot.left() + 25.0, cy),
                egui::Align2::LEFT_CENTER,
                label,
                theme::small_bold(),
                colour,
            );
            x += width + 4.0;
        }
        x = separator(ui, x - 4.0, rect);

        // The end, right to left: the grid menu, the grid size, the active tool's name.
        let mut right = rect.right() - 6.0;
        let slot = Rect::from_min_size(Pos2::new(right - 26.0, cy - 12.0), egui::vec2(26.0, 24.0));
        let response = ui
            .interact(slot, ui.id().with("grid-menu"), Sense::click())
            .on_hover_text(tr("Grid settings"));
        let open = matches!(self.popup, Some((Popup::GridSettings, _)));
        paint_flat(ui, slot, &response, open);
        draw_grid_icon(ui.painter(), slot.center(), theme::fg());
        if response.clicked() {
            self.toggle(Popup::GridSettings, slot);
        }
        right = slot.left() - 4.0;

        let label = grid_label(self.grid.borrow().actual_size());
        let width = text_width(ui, &label, theme::body()) + 30.0;
        let slot =
            Rect::from_min_size(Pos2::new(right - width, cy - 12.0), egui::vec2(width, 24.0));
        let response = ui
            .interact(slot, ui.id().with("grid-size"), Sense::click())
            .on_hover_text(tr("Grid size (+ and -)"));
        let open = matches!(self.popup, Some((Popup::GridSize, _)));
        paint_flat(ui, slot, &response, open);
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
        chevron(
            ui.painter(),
            Pos2::new(slot.right() - 11.0, cy),
            theme::dim(),
        );
        if response.clicked() {
            self.toggle(Popup::GridSize, slot);
        }
        right = slot.left() - 10.0;

        let tool = *self.tool.borrow();
        let name = tr(tool.name());
        let name_width = text_width(ui, name, theme::body());
        crate::theme::text(
            ui.painter(),
            Pos2::new(right, cy),
            egui::Align2::RIGHT_CENTER,
            name,
            theme::body(),
            theme::dim(),
        );
        right -= name_width + 12.0;

        // The active tool's settings fill the rest of the row.
        let area = Rect::from_min_max(
            Pos2::new(x, rect.top() + 3.0),
            Pos2::new(right.max(x), rect.bottom() - 3.0),
        );
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        row.set_clip_rect(area);
        row.spacing_mut().item_spacing.x = 4.0;
        row.spacing_mut().interact_size.y = 24.0;
        row.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
        // Keep header controls at full height even when touch mode uses solid bars.
        row.spacing_mut().scroll.floating = true;
        row.spacing_mut().scroll.floating_allocated_width = 0.0;
        if tool == Tool::Brush {
            row.horizontal(|row| {
                if let Some(action) = crate::tool_rows::shape_row(row, &self.shape_parameters) {
                    self.action = Some(action);
                }
                if self.document.borrow().has_shape_preview() && area.width() >= 480.0 {
                    if area.width() >= 620.0 {
                        row.colored_label(theme::dim(), tr("Shape preview"));
                    }
                    if row.button(tr("Confirm")).clicked() {
                        self.document.borrow_mut().finish_shape_preview();
                    }
                    if row.button(tr("Cancel")).clicked() {
                        self.document.borrow_mut().cancel_shape_preview();
                    }
                }
            });
        } else {
            self.document.borrow_mut().finish_shape_preview();
            let mut apply = false;
            if tool == Tool::Fillet {
                row.horizontal(|row| {
                    let bar = crate::tool_rows::fillet_bar(row, self.fillet_window);
                    self.fillet_window ^= bar.settings;
                    apply |= bar.apply;
                });
            } else {
                self.fillet_window = false;
            }
            if tool == Tool::Sculpt {
                row.horizontal(|row| {
                    self.sculpt_window ^= crate::tool_rows::sculpt_row(row, self.sculpt_window);
                });
            } else {
                self.sculpt_window = false;
            }
            // The sculpt brush's direction and the columns' shape, in a window like the
            // fillet's, so the strip keeps to what changes while sculpting.
            if self.sculpt_window {
                let mut split = false;
                let mut close = false;
                self.sculpt_window = crate::dialog::show_tool(
                    ui.ctx(),
                    "sculpt-settings",
                    tr("Sculpt"),
                    [420.0, 200.0],
                    |ui, part| match part {
                        crate::dialog::Part::Body => split |= crate::tool_rows::sculpt_controls(ui),
                        crate::dialog::Part::Actions => {
                            close |= crate::dialog::button(ui, tr("Close"), true).clicked();
                        }
                    },
                ) && !close;
                if split {
                    self.action = Some(Action::SplitColumns);
                }
            }
            // The fillet's settings in a window of their own, beside the views it works in.
            if self.fillet_window {
                let fillet = Rc::clone(&self.fillet);
                let mut close = false;
                self.fillet_window = crate::dialog::show_tool(
                    ui.ctx(),
                    "fillet-settings",
                    tr("Fillet"),
                    [320.0, 250.0],
                    |ui, part| match part {
                        crate::dialog::Part::Body => crate::tool_rows::fillet_controls(ui, &fillet),
                        crate::dialog::Part::Actions => {
                            apply |= crate::dialog::accent_button(ui, tr("Apply"), true).clicked();
                            close |= crate::dialog::button(ui, tr("Close"), true).clicked();
                        }
                    },
                ) && !close;
            }
            if apply {
                let outcome = editor_core::fillet_tool::apply(
                    &self.document,
                    &mut self.handles.borrow_mut(),
                    *self.fillet.borrow(),
                );
                editor_core::fillet_tool::report(&outcome);
            }
        }
    }

    fn tool_button(&mut self, ui: &mut Ui, slot: Rect, tool: Tool) {
        let response = ui
            .interact(slot, ui.id().with(("tool", tool.name())), Sense::click())
            .on_hover_text(tr(tool.tooltip()));
        if response.clicked() {
            self.pick(tool);
        }
        let active = *self.tool.borrow() == tool;
        paint_tool(ui, slot, tool_icon(tool), &response, active);
    }
}

/// The width `text` takes in `font`.
pub fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, theme::fg())
            .size()
            .x
    })
}

/// Lets a title bar move the window and maximise it on a double click. A browser tab has
/// no window of its own to move.
pub fn title_bar_drag(ui: &Ui, rect: Rect, id: &str, maximized: bool) {
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let bar = ui.interact(rect, ui.id().with(id), Sense::click_and_drag());
    if bar.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if bar.double_clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

/// The room the native red, yellow and green buttons take at the start of a title bar
/// on macOS, where the window keeps them; nothing elsewhere.
pub fn traffic_light_inset() -> f32 {
    if button_style() == ButtonStyle::Mac {
        72.0
    } else {
        0.0
    }
}

/// Whose window buttons the title bars draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// The window's own red, yellow and green buttons.
    Mac,
    /// Windows 11's.
    Windows,
    /// GNOME's.
    Gnome,
    /// None: a phone or tablet, where the app has the whole screen and the system its
    /// own way back out, and a browser tab, which the browser closes.
    Nothing,
}

/// The system's button style. `HERON_BRUSH_WINDOW_BUTTONS=windows|gnome|mac` overrides it,
/// so each can be looked at on any system.
pub fn button_style() -> ButtonStyle {
    static STYLE: std::sync::OnceLock<ButtonStyle> = std::sync::OnceLock::new();
    *STYLE.get_or_init(
        || match std::env::var("HERON_BRUSH_WINDOW_BUTTONS").as_deref() {
            Ok("windows") => ButtonStyle::Windows,
            Ok("gnome") => ButtonStyle::Gnome,
            Ok("mac") => ButtonStyle::Mac,
            Ok("none") => ButtonStyle::Nothing,
            _ if cfg!(any(target_os = "android", target_arch = "wasm32")) => ButtonStyle::Nothing,
            _ if cfg!(target_os = "macos") => ButtonStyle::Mac,
            _ if cfg!(target_os = "windows") => ButtonStyle::Windows,
            _ => ButtonStyle::Gnome,
        },
    )
}

/// Minimise, maximise and close at the end of a title bar, drawn as the system draws
/// them: Windows 11's wide flat buttons with a red close, or GNOME's round ones on Linux
/// and the other Unixes. Returns where they start. On macOS the window keeps its own red,
/// yellow and green buttons at the start, so nothing is drawn.
pub fn window_buttons(ui: &Ui, rect: Rect, maximized: bool) -> f32 {
    let style = button_style();
    if matches!(style, ButtonStyle::Mac | ButtonStyle::Nothing) {
        return rect.right();
    }
    let windows = style == ButtonStyle::Windows;
    let (width, gap, margin) = if windows {
        (46.0, 0.0, 0.0)
    } else {
        (24.0, 10.0, 10.0)
    };
    let mut right = rect.right() - margin;
    for (index, kind) in ["close", "maximize", "minimize"].into_iter().enumerate() {
        let slot = if windows {
            Rect::from_min_max(
                Pos2::new(right - width, rect.top()),
                Pos2::new(right, rect.bottom() - 1.0),
            )
        } else {
            Rect::from_center_size(
                Pos2::new(right - width / 2.0, rect.center().y),
                egui::Vec2::splat(width),
            )
        };
        right -= width + gap;
        let response = ui.interact(slot, ui.id().with(("window-button", index)), Sense::click());
        let close = kind == "close";
        let down = response.is_pointer_button_down_on();
        let p = ui.painter();
        let mut colour = theme::fg();
        if windows {
            if close && response.hovered() {
                p.rect_filled(
                    slot,
                    0.0,
                    if down {
                        Color32::from_rgb(0xb2, 0x29, 0x1b)
                    } else {
                        Color32::from_rgb(0xc4, 0x2b, 0x1c)
                    },
                );
                colour = Color32::WHITE;
            } else if response.hovered() {
                p.rect_filled(
                    slot,
                    0.0,
                    theme::fg().gamma_multiply(if down { 0.06 } else { 0.1 }),
                );
            }
        } else {
            let alpha = if down {
                0.28
            } else if response.hovered() {
                0.2
            } else {
                0.1
            };
            p.circle_filled(
                slot.center(),
                width / 2.0,
                theme::fg().gamma_multiply(alpha),
            );
        }
        let c = slot.center();
        // Windows draws its glyphs 10 px wide and hairline thin, GNOME 8 px and bolder.
        let (half, stroke) = if windows {
            (5.0, Stroke::new(1.0, colour))
        } else {
            (4.0, Stroke::new(1.5, colour))
        };
        match kind {
            "close" => {
                p.line_segment(
                    [c + egui::vec2(-half, -half), c + egui::vec2(half, half)],
                    stroke,
                );
                p.line_segment(
                    [c + egui::vec2(-half, half), c + egui::vec2(half, -half)],
                    stroke,
                );
            }
            "maximize" if maximized => {
                let r = half - 1.0;
                p.rect_stroke(
                    Rect::from_center_size(c + egui::vec2(-1.5, 1.5), egui::Vec2::splat(2.0 * r)),
                    1.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
                p.line_segment(
                    [
                        c + egui::vec2(-r + 1.5, -r - 1.5),
                        c + egui::vec2(r + 1.5, -r - 1.5),
                    ],
                    stroke,
                );
                p.line_segment(
                    [
                        c + egui::vec2(r + 1.5, -r - 1.5),
                        c + egui::vec2(r + 1.5, r - 1.5),
                    ],
                    stroke,
                );
            }
            "maximize" => {
                p.rect_stroke(
                    Rect::from_center_size(
                        c,
                        egui::Vec2::splat(if windows {
                            2.0 * half
                        } else {
                            2.0 * half - 1.0
                        }),
                    ),
                    1.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
            _ => {
                let y = if windows { 0.0 } else { half - 1.0 };
                p.line_segment([c + egui::vec2(-half, y), c + egui::vec2(half, y)], stroke);
            }
        }
        if response.clicked() {
            ui.ctx().send_viewport_cmd(match kind {
                "close" => egui::ViewportCommand::Close,
                "maximize" => egui::ViewportCommand::Maximized(!maximized),
                _ => egui::ViewportCommand::Minimized(true),
            });
        }
    }
    right - gap
}

/// A thin vertical rule between toolbar groups; returns where the next group starts.
fn separator(ui: &Ui, x: f32, rect: Rect) -> f32 {
    let x = x + 5.0;
    ui.painter().vline(
        x + 0.5,
        (rect.center().y - 10.0)..=(rect.center().y + 10.0),
        Stroke::new(1.0, theme::border()),
    );
    x + 7.0
}

/// A flat button's background: nothing at rest, a fill under the pointer or while open.
fn paint_flat(ui: &Ui, slot: Rect, response: &egui::Response, open: bool) {
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

/// A tool button: the icon on nothing at rest, on a fill under the pointer, and on an
/// accent tint with an accent edge when it is the tool in hand.
fn paint_tool(
    ui: &Ui,
    slot: Rect,
    (uri, bytes): (String, &'static [u8]),
    response: &egui::Response,
    active: bool,
) {
    let painter = ui.painter();
    let radius = CornerRadius::same(5);
    if active {
        painter.rect_filled(slot, radius, theme::accent().gamma_multiply(0.28));
        painter.rect_stroke(
            slot,
            radius,
            Stroke::new(1.0, theme::accent()),
            egui::StrokeKind::Inside,
        );
    } else if response.is_pointer_button_down_on() {
        painter.rect_filled(slot, radius, theme::button_pressed());
    } else if response.hovered() {
        painter.rect_filled(slot, radius, theme::button_hover());
    }
    let rect = Rect::from_center_size(slot.center(), egui::Vec2::splat(theme::TOOL_ICON));
    let alpha = if active || response.hovered() {
        255
    } else {
        205
    };
    egui::Image::from_bytes(uri, bytes)
        .tint(Color32::from_white_alpha(alpha))
        .paint_at(ui, rect);
}

/// A small on/off pill: accent when on, outlined when off.
pub fn toggle_pill(ui: &Ui, slot: Rect, response: &egui::Response, label: &str, on: bool) {
    let painter = ui.painter();
    let radius = CornerRadius::same(theme::radius());
    if on {
        painter.rect_filled(
            slot,
            radius,
            if response.hovered() {
                theme::accent_hover()
            } else {
                theme::accent()
            },
        );
    } else {
        painter.rect_filled(
            slot,
            radius,
            if response.hovered() {
                theme::button_hover()
            } else {
                theme::button_fill()
            },
        );
        painter.rect_stroke(
            slot,
            radius,
            Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
    }
    let colour = if on { theme::on_accent() } else { theme::fg() };
    crate::theme::text(
        &painter,
        slot.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::small_bold(),
        colour,
    );
}

/// A drop-down's chevron centred on `c`.
pub fn chevron(painter: &egui::Painter, c: Pos2, colour: Color32) {
    painter.add(egui::Shape::line(
        vec![
            c + egui::vec2(-3.5, -1.5),
            c + egui::vec2(0.0, 2.0),
            c + egui::vec2(3.5, -1.5),
        ],
        Stroke::new(1.4, colour),
    ));
}

/// Four squares: the grid menu's glyph.
pub fn draw_grid_icon(painter: &egui::Painter, c: Pos2, color: Color32) {
    for (dx, dy) in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
        painter.rect_stroke(
            Rect::from_center_size(c + egui::vec2(dx, dy), egui::vec2(5.0, 5.0)),
            1.0,
            Stroke::new(1.2, color),
            egui::StrokeKind::Middle,
        );
    }
}

#[cfg(test)]
mod shape_preview_tests {
    use super::*;
    fn text_at(output: &egui::FullOutput, label: &str) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("toolbar did not show {label}"))
    }
    fn frame(
        ctx: &egui::Context,
        chrome: &mut Chrome,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1800.0, 200.0))),
                events,
                ..Default::default()
            },
            |ui| {
                chrome.toolbar(
                    ui,
                    Rect::from_min_size(Pos2::ZERO, egui::vec2(1800.0, theme::TOOLBAR_HEIGHT)),
                );
            },
        );
        output.textures_delta.clear();
        output
    }
    fn click(ctx: &egui::Context, chrome: &mut Chrome, at: Pos2) {
        frame(
            ctx,
            chrome,
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
        frame(
            ctx,
            chrome,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    #[test]
    fn shape_preview_toolbar_confirms_and_cancels_without_losing_the_map() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let editor = crate::editor::Editor::open(&ctx, None);
        *editor.tool.borrow_mut() = Tool::Brush;
        let parameters = editor_core::shape_tool::ShapeParameters {
            kind: editor_core::shape_tool::ShapeKind::Cone,
            ..Default::default()
        };
        *editor.shape_parameters.borrow_mut() = parameters;
        let bounds = geometry_math::bbox::Bbox3d::from_scalars(-64.0, 64.0);
        let before = editor.document.borrow().to_map_text();
        let mut chrome = Chrome {
            fillet_window: false,
            sculpt_window: false,
            tool: editor.tool.clone(),
            grid: editor.grid.clone(),
            document: editor.document.clone(),
            clip: editor.clip.clone(),
            shape_parameters: editor.shape_parameters.clone(),
            fillet: editor.fillet.clone(),
            handles: editor.handles.clone(),
            detach_3d: false,
            action: None,
            filters: editor.filters.clone(),
            popup: None,
        };
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(bounds, parameters)
            .unwrap();
        let output = frame(&ctx, &mut chrome, vec![]);
        assert!(editor.document.borrow().has_shape_preview());
        click(&ctx, &mut chrome, text_at(&output, tr("Shape settings")));
        assert_eq!(chrome.action.take(), Some(Action::ShapeSettings));
        assert!(editor.document.borrow().has_shape_preview());
        click(&ctx, &mut chrome, text_at(&output, tr("Cancel")));
        assert!(!editor.document.borrow().has_shape_preview());
        assert_eq!(editor.document.borrow().to_map_text(), before);
        editor
            .document
            .borrow_mut()
            .begin_shape_preview(bounds, parameters)
            .unwrap();
        let output = frame(&ctx, &mut chrome, vec![]);
        let inserted = editor.document.borrow().to_map_text();
        click(&ctx, &mut chrome, text_at(&output, tr("Confirm")));
        assert!(!editor.document.borrow().has_shape_preview());
        assert_eq!(editor.document.borrow().to_map_text(), inserted);
    }
}
