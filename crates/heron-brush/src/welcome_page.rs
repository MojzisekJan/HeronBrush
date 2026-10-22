//! The welcome screen (`welcome_window.rs`): the startup window when no map was named,
//! and File > Welcome later.
//!
//! On the left, the logo, name and version, the profile for new maps, Continue,
//! New map and Browse, then Preferences and Manual at the foot. On the right, the recent maps with the starred
//! ones on top (each group newest first), then the bundled examples that are not
//! already among them, each with a thumbnail, its name and path, a star and a trash
//! button. One click opens a map. The arrow keys move between the buttons, the rows and
//! a row's icons; Return activates. Thumbnails are drawn off the UI thread.

use crate::{app_actions::Action, dialog, theme};
use editor_core::{
    assets, favourite_maps,
    i18n::{fill, tr},
    recent_documents,
};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke};
use std::time::SystemTime;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use web_time::Instant;

pub enum Request {
    Action(Action),
    Map(PathBuf),
}

struct Thumbnail {
    modified: SystemTime,
    image: Option<egui::TextureHandle>,
}

/// What the keyboard is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    /// One of the action buttons: Continue (when there is a last map), New map, Browse.
    Button(usize),
    /// A map row (`0`), or its star (`1`) or trash (`2`).
    Row(usize, u8),
}

/// A map in the list.
struct Listed {
    path: PathBuf,
    example: bool,
}

pub struct WelcomePage {
    pub open: bool,
    /// The recent maps, starred first, then the examples not among them.
    maps: Vec<Listed>,
    games: Vec<editor_core::games::Game>,
    focus: Option<Focus>,
    last_button: usize,
    last_row: usize,
    refreshed: Option<Instant>,
    thumbnails: HashMap<PathBuf, Thumbnail>,
    previews: crate::pool::Pool<
        (PathBuf, SystemTime, egui::Context),
        (PathBuf, SystemTime, Option<assets::Rgba>),
    >,
    error: String,
    /// The map the trash question is about, while it waits for an in-app answer.
    trashing: Option<PathBuf>,
}

impl Default for WelcomePage {
    fn default() -> Self {
        // A preview of a big map takes a good part of a second; the one drawn last time
        // comes back from the disk cache unless the map changed.
        let cache = assets::thumbnail_cache_dir();
        let previews = crate::pool::Pool::new(
            "map previews",
            1,
            move |(path, modified, ctx): (PathBuf, SystemTime, egui::Context)| {
                let image = assets::thumbnail_cached(&path, &cache);
                ctx.request_repaint();
                (path, modified, image)
            },
        );
        Self {
            open: false,
            maps: vec![],
            games: vec![],
            focus: None,
            last_button: 0,
            last_row: 0,
            refreshed: None,
            thumbnails: HashMap::new(),
            previews,
            error: String::new(),
            trashing: None,
        }
    }
}

/// The recent maps with the starred ones moved to the front, each group keeping its
/// newest-first order (`ordered_recent`).
pub fn ordered_recent(recent: Vec<PathBuf>, starred: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let (mut favourites, rest): (Vec<PathBuf>, Vec<PathBuf>) =
        recent.into_iter().partition(|path| starred(path));
    favourites.extend(rest);
    favourites
}

/// A path with the home directory shown as `~`.
pub fn home_relative(path: &Path) -> String {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    match home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// `text` shortened in the middle with an ellipsis until `fits` says it fits.
pub fn middle_ellipsis(text: &str, fits: impl Fn(&str) -> bool) -> String {
    if fits(text) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut keep = chars.len().saturating_sub(1);
    while keep > 2 {
        let head = keep / 2;
        let tail = keep - head;
        let shortened: String = chars[..head]
            .iter()
            .chain(['…'].iter())
            .chain(chars[chars.len() - tail..].iter())
            .collect();
        if fits(&shortened) {
            return shortened;
        }
        keep -= 1;
    }
    "…".into()
}

impl WelcomePage {
    fn refresh(&mut self) {
        let recent = ordered_recent(recent_documents::load(), favourite_maps::is_favourite);
        let known: std::collections::HashSet<PathBuf> = recent
            .iter()
            .filter_map(|p| std::fs::canonicalize(p).ok())
            .collect();
        let examples = editor_core::samples::ensure()
            .into_iter()
            .filter(|p| {
                std::fs::canonicalize(p)
                    .map(|c| !known.contains(&c))
                    .unwrap_or(true)
            })
            .collect();
        let examples = ordered_recent(examples, favourite_maps::is_favourite);
        self.maps = recent
            .into_iter()
            .map(|path| Listed {
                path,
                example: false,
            })
            .chain(examples.into_iter().map(|path| Listed {
                path,
                example: true,
            }))
            .collect();
        self.games = editor_core::games::all();
        self.thumbnails
            .retain(|path, _| self.maps.iter().any(|m| &m.path == path));
        self.refreshed = Some(Instant::now());
        if let Some(Focus::Row(row, _)) = self.focus {
            if row >= self.maps.len() {
                self.focus = None;
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<Request> {
        self.present(ctx, None)
    }

    pub fn show_in(&mut self, ui: &mut egui::Ui) -> Option<Request> {
        let ctx = ui.ctx().clone();
        self.present(&ctx, Some(ui))
    }

    fn present(&mut self, ctx: &egui::Context, panel: Option<&mut egui::Ui>) -> Option<Request> {
        if !self.open {
            return None;
        }
        if self.refreshed.is_none_or(|t| t.elapsed().as_secs() >= 2) {
            self.refresh();
        }
        // The window sleeps while nothing happens; the list is read again every couple
        // of seconds while it shows.
        ctx.request_repaint_after(web_time::Duration::from_secs(2));
        while let Some((path, modified, image)) = self.previews.try_recv() {
            if let Some(slot) = self
                .thumbnails
                .get_mut(&path)
                .filter(|s| s.modified == modified)
            {
                slot.image = image.map(|image| {
                    ctx.load_texture(
                        path.display().to_string(),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.pixels,
                        ),
                        egui::TextureOptions::LINEAR,
                    )
                });
            }
        }
        let mut request = None;
        let mut close = false;
        let mut contents = |ui: &mut egui::Ui, part| {
            if part == dialog::Part::Actions {
                close = dialog::button(ui, tr("Close"), true).clicked();
                return;
            }
            request = self.page(ui);
        };
        let visible = if let Some(ui) = panel {
            contents(ui, dialog::Part::Body);
            true
        } else {
            dialog::show_with_body_margin(
                ctx,
                "welcome",
                tr("Welcome to Heron Brush"),
                [920.0, 620.0],
                0.0,
                contents,
            )
        };
        self.open = visible && !close;
        request
    }

    /// The page: the action panel and the map list side by side.
    fn page(&mut self, ui: &mut egui::Ui) -> Option<Request> {
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        let last = recent_documents::load().into_iter().next();
        let buttons = if last.is_some() { 3 } else { 2 };
        let activated = self.keyboard(ui, buttons);
        let mut request = None;

        let left = Rect::from_min_size(
            area.min,
            egui::vec2(300.0_f32.min(area.width() * 0.4), area.height()),
        );
        let right = Rect::from_min_max(egui::pos2(left.right() + 1.0, area.top()), area.max);
        ui.painter().rect_filled(left, 0.0, theme::base());
        ui.painter().vline(
            left.right() + 0.5,
            left.y_range(),
            Stroke::new(1.0, theme::border()),
        );

        let mut panel = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left.shrink2(egui::vec2(24.0, 24.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        if let Some(r) = self.actions(&mut panel, last, activated) {
            request = Some(r);
        }
        let mut list = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right.shrink2(egui::vec2(12.0, 12.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        if let Some(r) = self.map_list(&mut list, activated) {
            request = Some(r);
        }
        request
    }

    /// Moves the keyboard focus with the arrow keys; returns what Return activated.
    fn keyboard(&mut self, ui: &egui::Ui, buttons: usize) -> Option<Focus> {
        if ui.ctx().egui_wants_keyboard_input() {
            return None;
        }
        let (up, down, left, right, enter) = ui.input(|i| {
            let plain = i.modifiers.is_none();
            (
                plain && i.key_pressed(egui::Key::ArrowUp),
                plain && i.key_pressed(egui::Key::ArrowDown),
                plain && i.key_pressed(egui::Key::ArrowLeft),
                plain && i.key_pressed(egui::Key::ArrowRight),
                plain && i.key_pressed(egui::Key::Enter),
            )
        });
        if !(up || down || left || right || enter) {
            return None;
        }
        let rows = self.maps.len();
        let new_map = buttons - 2;
        let browse = buttons - 1;
        let focus = self.focus.unwrap_or(if rows > 0 {
            Focus::Row(0, 0)
        } else {
            Focus::Button(0)
        });
        if self.focus.is_none() && !enter {
            self.focus = Some(focus);
            return None;
        }
        if enter {
            return Some(focus);
        }
        self.focus = Some(match focus {
            Focus::Button(index) => {
                self.last_button = index;
                if up && new_map > 0 {
                    Focus::Button(0)
                } else if down && index < new_map {
                    Focus::Button(new_map)
                } else if right && index == new_map {
                    Focus::Button(browse)
                } else if right && rows > 0 {
                    Focus::Row(self.last_row.min(rows - 1), 0)
                } else if left && index == browse {
                    Focus::Button(new_map)
                } else {
                    focus
                }
            }
            Focus::Row(row, icon) => {
                if up {
                    Focus::Row(row.saturating_sub(1), 0)
                } else if down {
                    Focus::Row((row + 1).min(rows.saturating_sub(1)), 0)
                } else if right {
                    Focus::Row(row, (icon + 1).min(2))
                } else if left && icon > 0 {
                    Focus::Row(row, icon - 1)
                } else if left {
                    self.last_row = row;
                    Focus::Button(self.last_button.min(browse))
                } else {
                    focus
                }
            }
        });
        None
    }

    /// The left panel.
    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        last: Option<PathBuf>,
        activated: Option<Focus>,
    ) -> Option<Request> {
        let mut request = None;
        let width = ui.available_width();
        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            let (uri, bytes) = crate::chrome::LOGO;
            ui.add(egui::Image::from_bytes(uri, bytes).fit_to_exact_size(egui::Vec2::splat(72.0)));
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Heron Brush")
                    .font(egui::FontId::new(22.0, theme::bold()))
                    .color(theme::fg()),
            );
            ui.label(
                egui::RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(theme::dim()),
            );
        });
        ui.add_space(24.0);

        // The game a new map is for, the same preference the Games pane writes.
        if self.games.is_empty() {
            let folder = home_relative(&editor_core::games::user_games_directory());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(fill(
                        "No games found. Put game configurations in {}",
                        &[&folder],
                    ))
                    .color(theme::dim()),
                )
                .wrap(),
            );
        } else {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(tr("Profile")).color(theme::dim()))
                    .on_hover_text(format!(
                        "{} / {}",
                        tr("Entity definitions"),
                        tr("Materials")
                    ));
                let mut game = editor_core::preferences::pref(|p| p.game.clone());
                let chosen = self
                    .games
                    .iter()
                    .position(|g| g.name == game)
                    .map_or(0, |at| at + 1);
                let mut selected = chosen;
                if crate::widgets::game_dropdown(
                    ui,
                    "welcome-game",
                    &mut selected,
                    &self.games,
                    ui.available_width(),
                ) {
                    game = if selected == 0 {
                        String::new()
                    } else {
                        self.games[selected - 1].name.clone()
                    };
                    editor_core::preferences::edit(|p| p.game = game);
                    if let Err(e) = editor_core::preferences::current()
                        .save(&editor_core::preferences::Preferences::default_path())
                    {
                        self.error = e.to_string();
                    }
                }
            });
        }

        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "{} / {}",
                    tr("Entity definitions"),
                    tr("Materials")
                ))
                .color(theme::dim()),
            )
            .wrap(),
        );

        ui.add_space(18.0);
        let mut index = 0;
        if let Some(last) = last {
            let name = last
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| last.display().to_string());
            let response = big_button(
                ui,
                &fill("Continue {}", &[&name]),
                width,
                40.0,
                true,
                self.focus == Some(Focus::Button(0)),
            )
            .on_hover_text(home_relative(&last));
            if response.clicked() || activated == Some(Focus::Button(0)) {
                request = Some(Request::Map(last));
            }
            ui.add_space(8.0);
            index = 1;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let half = (width - 8.0) / 2.0;
            let new_map = big_button(
                ui,
                tr("New map"),
                half,
                34.0,
                false,
                self.focus == Some(Focus::Button(index)),
            )
            .on_hover_text(tr("Create a new map document"));
            if new_map.clicked() || activated == Some(Focus::Button(index)) {
                request = Some(Request::Action(Action::New));
            }
            let browse = big_button(
                ui,
                tr("Browse"),
                half,
                34.0,
                false,
                self.focus == Some(Focus::Button(index + 1)),
            )
            .on_hover_text(tr("Open an existing map document"));
            if browse.clicked() || activated == Some(Focus::Button(index + 1)) {
                request = Some(Request::Action(Action::Open));
            }
        });
        ui.add_space(22.0);

        // Secondary actions stay together at the foot of the sidebar.
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            use egui_phosphor::regular as icon;
            for (glyph, label, action) in [
                (icon::BOOK_OPEN, tr("Manual"), Action::Manual),
                (icon::GEAR_SIX, tr("Preferences"), Action::Preferences),
            ] {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::click());
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                let active = response.hovered() || response.has_focus();
                if active {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(5), theme::button_hover());
                }
                let colour = if active { theme::fg() } else { theme::dim() };
                let mut text = egui::text::LayoutJob::default();
                text.append(
                    glyph,
                    0.0,
                    egui::TextFormat {
                        font_id: theme::icon(),
                        color: colour,
                        ..Default::default()
                    },
                );
                text.append(
                    &format!("  {label}"),
                    0.0,
                    egui::TextFormat {
                        font_id: theme::body(),
                        color: colour,
                        ..Default::default()
                    },
                );
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(text));
                let position = rect.center() - galley.size() * 0.5;
                ui.painter().galley(position, galley, colour);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
                });
                if response.clicked() {
                    request = Some(Request::Action(action));
                }
            }
            ui.separator();
        });
        request
    }

    /// The right panel: the recent maps, then the examples.
    fn map_list(&mut self, ui: &mut egui::Ui, activated: Option<Focus>) -> Option<Request> {
        let mut request = None;
        let mut refresh = false;
        egui::ScrollArea::vertical()
            .id_salt("welcome-maps")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let width = ui.available_width();
                if !self.maps.iter().any(|m| !m.example) {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(tr("No recent documents")).color(theme::dim()))
                    });
                    ui.add_space(12.0);
                }
                let mut heading_done = false;
                for index in 0..self.maps.len() {
                    if self.maps[index].example && !heading_done {
                        heading_done = true;
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(tr("Examples"))
                                .font(theme::small_bold())
                                .color(theme::dim()),
                        );
                        ui.add_space(2.0);
                    }
                    let path = self.maps[index].path.clone();
                    let modified = assets::modified(&path);
                    if self
                        .thumbnails
                        .get(&path)
                        .is_none_or(|s| s.modified != modified)
                    {
                        self.thumbnails.insert(
                            path.clone(),
                            Thumbnail {
                                modified,
                                image: None,
                            },
                        );
                        self.previews
                            .send((path.clone(), modified, ui.ctx().clone()));
                    }
                    let focus = match self.focus {
                        Some(Focus::Row(row, icon)) if row == index => Some(icon),
                        _ => None,
                    };
                    let image = self.thumbnails.get(&path).and_then(|s| s.image.clone());
                    let (clicked, star, trash) = map_row(
                        ui,
                        width,
                        &path,
                        image.as_ref(),
                        favourite_maps::is_favourite(&path),
                        focus,
                    );
                    if focus.is_some() {
                        ui.scroll_to_rect(
                            ui.min_rect().with_min_y(ui.min_rect().bottom() - 70.0),
                            None,
                        );
                    }
                    let activated_icon = match activated {
                        Some(Focus::Row(row, icon)) if row == index => Some(icon),
                        _ => None,
                    };
                    if star || activated_icon == Some(1) {
                        favourite_maps::toggle(&path);
                        refresh = true;
                    } else if trash || activated_icon == Some(2) {
                        if TRASH_CAN {
                            let name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            let question = fill("Move {} to the trash?", &[&name])
                                + "\n"
                                + tr("It goes to the wastebasket, so it can be put back.");
                            let buttons = [tr("Move to the trash"), tr("Cancel")];
                            let answer = crate::file_dialog::question(
                                TRASH_QUESTION,
                                true,
                                tr("Move to the trash?"),
                                &question,
                                &buttons,
                            );
                            self.trashing = Some(path);
                            refresh |= self.trash_answer(answer);
                        } else {
                            // No wastebasket to put it in, and a map is never deleted outright:
                            // the row only leaves the list.
                            recent_documents::forget(&path);
                            favourite_maps::forget(&path);
                            refresh = true;
                        }
                    } else if clicked || activated_icon == Some(0) {
                        request = Some(Request::Map(path));
                    }
                }
                if !self.error.is_empty() {
                    ui.label(egui::RichText::new(&self.error).color(theme::error()));
                }
            });
        refresh |= self.trash_answer(crate::file_dialog::question(
            TRASH_QUESTION,
            false,
            "",
            "",
            &[],
        ));
        if refresh {
            self.refresh();
        }
        request
    }

    /// Moves the map the trash question was about to the trash if the answer was yes.
    /// Returns whether the list changed.
    fn trash_answer(&mut self, answer: Option<usize>) -> bool {
        let Some(answer) = answer else { return false };
        let Some(path) = self.trashing.take() else {
            return false;
        };
        if answer != 0 {
            return false;
        }
        match trash_file(&path) {
            Ok(()) => {
                recent_documents::forget(&path);
                favourite_maps::forget(&path);
                true
            }
            Err(e) => {
                self.error = e;
                false
            }
        }
    }
}

/// The trash question, by its key in [`crate::file_dialog::question`].
const TRASH_QUESTION: &str = "trash-map";

/// Whether the platform has a wastebasket [`trash_file`] can use.
const TRASH_CAN: bool = cfg!(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
));

/// A full-width action button; `primary` is the accent one.
fn big_button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    height: f32,
    primary: bool,
    focused: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let painter = ui.painter();
    let radius = CornerRadius::same(5);
    let fill = match (primary, response.hovered()) {
        (true, true) => theme::accent_hover(),
        (true, false) => theme::accent(),
        (false, true) => theme::button_hover(),
        (false, false) => theme::button_fill(),
    };
    painter.rect_filled(rect, radius, fill);
    if !primary {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
    }
    if focused {
        painter.rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(7),
            Stroke::new(1.5, theme::accent_hover()),
            egui::StrokeKind::Inside,
        );
    }
    let colour = if primary {
        theme::on_accent()
    } else {
        theme::fg()
    };
    let font = if primary {
        theme::body_bold()
    } else {
        theme::body()
    };
    let text = middle_ellipsis(label, |t| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(t.to_string(), font.clone(), colour)
                .size()
                .x
        }) <= width - 20.0
    });
    crate::theme::text(
        painter,
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font,
        colour,
    );
    response
}

/// One map: thumbnail, name, path and the star and trash buttons. Returns whether the
/// row, the star or the trash was clicked.
fn map_row(
    ui: &mut egui::Ui,
    width: f32,
    path: &Path,
    image: Option<&egui::TextureHandle>,
    starred: bool,
    focus: Option<u8>,
) -> (bool, bool, bool) {
    let height = 66.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let star_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 52.0, rect.center().y),
        egui::Vec2::splat(28.0),
    );
    let trash_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 20.0, rect.center().y),
        egui::Vec2::splat(28.0),
    );
    let star = ui
        .interact(star_rect, ui.id().with((path, "star")), Sense::click())
        .on_hover_text(tr("Keep at the top"));
    let trash = ui
        .interact(trash_rect, ui.id().with((path, "trash")), Sense::click())
        .on_hover_text(tr("Move to the trash"));
    let hovered = response.hovered() || star.hovered() || trash.hovered();
    let painter = ui.painter();
    if hovered || focus.is_some() {
        painter.rect_filled(rect, CornerRadius::same(6), theme::surface());
    }
    if focus == Some(0) {
        painter.rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.5, theme::accent_hover()),
            egui::StrokeKind::Inside,
        );
    }
    let thumb = Rect::from_min_size(
        Pos2::new(rect.left() + 6.0, rect.top() + 5.0),
        egui::vec2(80.0, height - 10.0),
    );
    painter.rect_filled(thumb, CornerRadius::same(4), theme::base());
    if let Some(image) = image {
        // Cropped to the slot from the middle, never stretched: the previews are rendered
        // square and the slot is wider than tall.
        let [w, h] = image.size();
        let (source, slot) = (w as f32 / h.max(1) as f32, thumb.width() / thumb.height());
        let uv = if source < slot {
            let keep = source / slot;
            Rect::from_min_max(
                Pos2::new(0.0, 0.5 - keep / 2.0),
                Pos2::new(1.0, 0.5 + keep / 2.0),
            )
        } else {
            let keep = slot / source;
            Rect::from_min_max(
                Pos2::new(0.5 - keep / 2.0, 0.0),
                Pos2::new(0.5 + keep / 2.0, 1.0),
            )
        };
        egui::Image::new(image)
            .uv(uv)
            .fit_to_exact_size(thumb.size())
            .corner_radius(4)
            .paint_at(ui, thumb);
    }
    let text_left = thumb.right() + 12.0;
    let text_room = star_rect.left() - 8.0 - text_left;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let measure = |text: &str, font: &egui::FontId| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(text.to_string(), font.clone(), theme::fg())
                .size()
                .x
        })
    };
    let bold = theme::body_bold();
    let name = middle_ellipsis(&name, |t| measure(t, &bold) <= text_room);
    let small = egui::FontId::proportional(theme::text_size() - 1.5);
    let shown = middle_ellipsis(&home_relative(path), |t| measure(t, &small) <= text_room);
    let painter = ui.painter();
    painter.rect_stroke(
        thumb,
        CornerRadius::same(4),
        Stroke::new(1.0, theme::border()),
        egui::StrokeKind::Inside,
    );
    crate::theme::text(
        painter,
        Pos2::new(text_left, rect.center().y - 9.0),
        egui::Align2::LEFT_CENTER,
        name,
        theme::body_bold(),
        theme::fg(),
    );
    crate::theme::text(
        painter,
        Pos2::new(text_left, rect.center().y + 10.0),
        egui::Align2::LEFT_CENTER,
        shown,
        small,
        theme::dim(),
    );

    for (slot, response, icon) in [(star_rect, &star, 1u8), (trash_rect, &trash, 2u8)] {
        if response.hovered() {
            painter.rect_filled(slot, CornerRadius::same(5), theme::button_hover());
        }
        if focus == Some(icon) {
            painter.rect_stroke(
                slot,
                CornerRadius::same(5),
                Stroke::new(1.5, theme::accent_hover()),
                egui::StrokeKind::Inside,
            );
        }
        let visible = hovered || focus.is_some() || (icon == 1 && starred);
        if !visible {
            continue;
        }
        let colour = if response.hovered() {
            theme::fg()
        } else {
            theme::dim()
        };
        if icon == 1 {
            draw_star(painter, slot.center(), starred, colour);
        } else {
            draw_trash(painter, slot.center(), colour);
        }
    }
    (response.clicked(), star.clicked(), trash.clicked())
}

fn draw_star(painter: &egui::Painter, c: Pos2, filled: bool, colour: Color32) {
    let points: Vec<Pos2> = (0..10)
        .map(|i| {
            let angle = std::f32::consts::PI * (i as f32) / 5.0 - std::f32::consts::FRAC_PI_2;
            let r = if i % 2 == 0 { 7.5 } else { 3.2 };
            c + egui::vec2(angle.cos() * r, angle.sin() * r + 0.5)
        })
        .collect();
    if filled {
        let gold = theme::star();
        // Concave: drawn as the centre fan so the fill stays inside the outline.
        for i in 0..10 {
            painter.add(egui::Shape::convex_polygon(
                vec![c + egui::vec2(0.0, 0.5), points[i], points[(i + 1) % 10]],
                gold,
                Stroke::NONE,
            ));
        }
    } else {
        painter.add(egui::Shape::closed_line(points, Stroke::new(1.3, colour)));
    }
}

fn draw_trash(painter: &egui::Painter, c: Pos2, colour: Color32) {
    let stroke = Stroke::new(1.3, colour);
    painter.line_segment(
        [c + egui::vec2(-6.0, -4.5), c + egui::vec2(6.0, -4.5)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(-2.0, -6.5), c + egui::vec2(2.0, -6.5)],
        stroke,
    );
    painter.add(egui::Shape::line(
        vec![
            c + egui::vec2(-4.5, -4.5),
            c + egui::vec2(-3.5, 6.5),
            c + egui::vec2(3.5, 6.5),
            c + egui::vec2(4.5, -4.5),
        ],
        stroke,
    ));
    painter.line_segment(
        [c + egui::vec2(-1.2, -2.0), c + egui::vec2(-1.0, 4.0)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(1.2, -2.0), c + egui::vec2(1.0, 4.0)],
        stroke,
    );
}

/// Use the desktop's recycle-bin service; never substitute permanent deletion.
fn trash_file(path: &Path) -> Result<(), String> {
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("gio")
        .arg("trash")
        .arg("--")
        .arg(&path)
        .output();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("osascript")
        .args(["-e", "on run argv\ntell application \"Finder\" to delete POSIX file (item 1 of argv)\nend run", "--"])
        .arg(&path)
        .output();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("powershell.exe")
        .env("HERON_TRASH_PATH", &path)
        .args(["-NoProfile", "-NonInteractive", "-Command", "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($env:HERON_TRASH_PATH, 'OnlyErrorDialogs', 'SendToRecycleBin')"])
        .output();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err(format!(
        "{}: this platform has no recycle-bin integration",
        path.display()
    ));
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        let output = result.map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starred_maps_go_first_keeping_their_order() {
        let paths: Vec<PathBuf> = ["a", "b", "c", "d"].iter().map(PathBuf::from).collect();
        let ordered = ordered_recent(paths, |p| p == Path::new("c") || p == Path::new("b"));
        assert_eq!(
            ordered,
            ["b", "c", "a", "d"]
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn long_paths_lose_their_middle() {
        let text = "~/projects/maps/very/deep/folder/crate.map";
        let short = middle_ellipsis(text, |t| t.chars().count() <= 20);
        assert_eq!(short.chars().count(), 20);
        assert!(
            short.starts_with("~/proj") && short.ends_with("crate.map") && short.contains('…'),
            "{short}"
        );
        assert_eq!(middle_ellipsis("short", |t| t.len() <= 20), "short");
    }
}
