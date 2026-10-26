//! The welcome screen (`welcome_window.rs`): the startup window when no map was named,
//! and File > Welcome later.
//!
//! A banner across the top, the bundled render dithered in the editor's blue
//! ([`crate::welcome_banner`]), with the logo, the name and the version; under it, on the
//! left, a menu like a game's: New map, Continue, Open map, the examples, the manual and
//! preferences and the profile for new maps; on the right, the recent maps (or the
//! bundled examples) as cards with a thumbnail, a star and a trash button; a strip of
//! shortcuts at the foot. One click opens a map. The arrow keys move between the menu and
//! the cards, Return opens, S stars and Delete moves to the trash. Thumbnails are drawn off
//! the UI thread.

use crate::{app_actions::Action, dialog, theme};
use editor_core::{
    assets, favourite_maps,
    i18n::{fill, tr},
    recent_documents,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
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
    /// One of the menu's buttons, top to bottom.
    Button(usize),
    /// A card in the grid, row by row.
    Card(usize),
}

/// What a key asked of the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Activation {
    /// Return: press the button or open the map.
    Open(Focus),
    /// S on a card: star it or take the star away.
    Star(usize),
    /// Delete on a card: move the map to the trash.
    Trash(usize),
}

/// What a menu button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    New,
    Continue,
    Open,
    /// Shows the examples, or the recent maps again.
    Switch,
    Manual,
    Preferences,
}

/// A map in the list.
struct Listed {
    path: PathBuf,
    example: bool,
}

/// Makes map thumbnails in the background: a map and when it changed in, its picture out.
type PreviewPool = crate::pool::Pool<
    (PathBuf, SystemTime, egui::Context),
    (PathBuf, SystemTime, Option<assets::Rgba>),
>;

pub struct WelcomePage {
    pub open: bool,
    /// The recent maps, starred first, then the examples not among them.
    maps: Vec<Listed>,
    games: Vec<editor_core::games::Game>,
    focus: Option<Focus>,
    last_button: usize,
    last_card: usize,
    /// The grid shows the examples rather than the recent maps.
    examples: bool,
    /// The render behind the banner, and the banner as last drawn and for what.
    source: Option<crate::welcome_banner::Source>,
    banner: Option<(crate::welcome_banner::Layout, egui::TextureHandle)>,
    refreshed: Option<Instant>,
    thumbnails: HashMap<PathBuf, Thumbnail>,
    previews: PreviewPool,
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
                let image = assets::thumbnail_cached_sized(&path, &cache, THUMBNAIL);
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
            last_card: 0,
            examples: false,
            source: crate::welcome_banner::Source::bundled(),
            banner: None,
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
        if let Some(Focus::Card(card)) = self.focus {
            if card >= self.maps.len() {
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
                        // Mipmapped, so a small card on a phone shrinks it smoothly.
                        egui::TextureOptions {
                            mipmap_mode: Some(egui::TextureFilter::Linear),
                            ..egui::TextureOptions::LINEAR
                        },
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
                [1180.0, 760.0],
                0.0,
                contents,
            )
        };
        self.open = visible && !close;
        request
    }

    /// The page: the banner, the menu and the grid of maps, the strip of shortcuts.
    fn page(&mut self, ui: &mut egui::Ui) -> Option<Request> {
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        ui.painter().rect_filled(area, 0.0, theme::bg());
        let last = recent_documents::load().into_iter().next();
        let recent = self.maps.iter().filter(|m| !m.example).count();
        let examples = self.examples || recent == 0;
        let shown: Vec<usize> = (0..self.maps.len())
            .filter(|index| self.maps[*index].example == examples)
            .collect();
        let mut items = vec![Item::New];
        if last.is_some() {
            items.push(Item::Continue);
        }
        items.extend([Item::Open, Item::Switch, Item::Manual, Item::Preferences]);
        if area.width() < NARROW {
            return self.stacked_page(ui, area, &items, last, examples, &shown);
        }

        // A low window, a phone on its side, drops the strip of shortcuts and lets the
        // banner shrink further. A touch screen has no use for the shortcuts either.
        let short = area.height() < SHORT;
        let footer_height = if short || crate::touch::enabled(ui.ctx()) {
            0.0
        } else {
            44.0
        };
        let (left, right) = if short { (24.0, 24.0) } else { (64.0, 48.0) };

        // The banner takes four tenths of the height, up to 330 points, and what sits on
        // it scales with it; in a low window it gives way first, so the menu under it fits
        // without scrolling. Down to 150 points (110 in a short window), below which the
        // menu scrolls instead.
        let menu_height = items
            .iter()
            .map(|item| match item {
                Item::New | Item::Continue => 50.0 + 8.0,
                Item::Open | Item::Switch => 42.0 + 8.0,
                // Manual and Preferences share a 40-point row.
                Item::Manual => 40.0 + 8.0,
                Item::Preferences => 0.0,
            })
            .sum::<f32>()
            // The profile: its caption, the dropdown and the line under it.
            + 92.0;
        let room = area.height() - footer_height - 12.0 - 4.0 - menu_height;
        let lowest = if short { 110.0 } else { 150.0 };
        let banner_height = (area.height() * 0.41).min(room).clamp(lowest, 330.0);
        let u = banner_height / 330.0;
        let banner = Rect::from_min_size(area.min, egui::vec2(area.width(), banner_height));
        let logo = (left + 12.0 + 60.0 * u, 172.0 * u, 60.0 * u);
        self.banner(ui, banner, logo);

        let footer = Rect::from_min_max(
            egui::pos2(area.left(), area.bottom() - footer_height),
            area.max,
        );
        let body = Rect::from_min_max(
            egui::pos2(area.left() + left, banner.bottom() + 4.0),
            egui::pos2(area.right() - right, footer.top() - 12.0),
        );
        let menu_width = 262.0_f32.min(body.width() * 0.32);
        let menu = Rect::from_min_size(body.min, egui::vec2(menu_width, body.height()));
        let grid = Rect::from_min_max(egui::pos2(menu.right() + 36.0, body.top()), body.max);
        let columns = ((grid.width() + 12.0) / 200.0).floor().clamp(2.0, 5.0) as usize;

        let activated = self.keyboard(ui, items.len(), shown.len(), columns);
        let mut request = None;
        let mut panel = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(menu)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        // Scrolls when the window is too low for the whole menu.
        let menu_request = egui::ScrollArea::vertical()
            .id_salt("welcome-menu")
            .auto_shrink([false, false])
            .show(&mut panel, |ui| {
                self.menu(ui, &items, last, examples, activated)
            })
            .inner;
        if let Some(r) = menu_request {
            request = Some(r);
        }
        let mut list = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(grid)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        heading(
            &mut list,
            examples,
            if short { 24.0 } else { 32.0 },
            shown.is_empty(),
        );
        if let Some(r) = self.grid(&mut list, &shown, columns, activated, true) {
            request = Some(r);
        }
        if footer_height > 0.0 {
            self.footer(ui, footer);
        }
        request
    }

    /// The page in a narrow window, a phone held upright: the banner, the menu and the
    /// cards one under another, scrolled together.
    fn stacked_page(
        &mut self,
        ui: &mut egui::Ui,
        area: Rect,
        items: &[Item],
        last: Option<PathBuf>,
        examples: bool,
        shown: &[usize],
    ) -> Option<Request> {
        let margin = 16.0;
        // The banner scales until the logo and the name fit across it, as they stand on
        // the wide page but closer to the edge.
        let title = ui
            .painter()
            .layout_no_wrap(
                "HERON BRUSH".into(),
                theme::display_bold(72.0),
                Color32::WHITE,
            )
            .size()
            .x;
        let u = ((area.width() - 2.0 * margin - 4.0) / (160.0 + title)).min(0.7);
        // Lower by `drop`, to keep the name clear of the version above it.
        let drop = 24.0;
        let logo = (margin + 4.0 + 60.0 * u, drop + 172.0 * u, 60.0 * u);
        let width = area.width() - 2.0 * margin;
        let columns = ((width + 12.0) / 160.0).floor().clamp(1.0, 5.0) as usize;
        let activated = self.keyboard(ui, items.len(), shown.len(), columns);
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        egui::ScrollArea::vertical()
            .id_salt("welcome-page")
            .auto_shrink([false, false])
            .show(&mut page, |ui| {
                let (banner, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), drop + 330.0 * u),
                    Sense::hover(),
                );
                self.banner(ui, banner, logo);
                let mut request = None;
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: margin as i8,
                        right: margin as i8,
                        top: 4,
                        bottom: margin as i8,
                    })
                    .show(ui, |ui| {
                        request = self.menu(ui, items, last, examples, activated);
                        ui.add_space(20.0);
                        heading(ui, examples, 24.0, shown.is_empty());
                        if let Some(r) = self.grid(ui, shown, columns, activated, false) {
                            request = Some(r);
                        }
                    });
                request
            })
            .inner
    }

    /// The banner across the top, with the logo, the name and the version on it.
    fn banner(&mut self, ui: &mut egui::Ui, rect: Rect, logo: (f32, f32, f32)) {
        let scale = ui.ctx().pixels_per_point();
        let layout = crate::welcome_banner::Layout {
            size: [
                (rect.width() * scale).round().max(1.0) as usize,
                (rect.height() * scale).round().max(1.0) as usize,
            ],
            scale,
            logo,
            page: theme::bg(),
        };
        if let Some(source) = &self.source {
            if self
                .banner
                .as_ref()
                .is_none_or(|(drawn, _)| *drawn != layout)
            {
                let image = crate::welcome_banner::draw(source, layout);
                let texture =
                    ui.ctx()
                        .load_texture("welcome-banner", image, egui::TextureOptions::NEAREST);
                self.banner = Some((layout, texture));
            }
        }
        let painter = ui.painter();
        match &self.banner {
            Some((_, texture)) => {
                painter.image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, 0.0, theme::base());
            }
        }

        // The version at the top right, below the window's own buttons where it has them.
        let top = if crate::chrome::button_style() == crate::chrome::ButtonStyle::Mac {
            rect.top() + 12.0
        } else {
            rect.top() + theme::HEADER_HEIGHT + 6.0
        };
        let version = format!("v{}", env!("CARGO_PKG_VERSION")).to_uppercase();
        let mono = egui::FontId::monospace(11.5);
        let galley = painter.layout_no_wrap(version, mono, Color32::from_rgb(0xe4, 0xe6, 0xea));
        let label = Rect::from_min_size(
            Pos2::new(rect.right() - 22.0 - galley.size().x - 20.0, top),
            galley.size() + egui::vec2(20.0, 8.0),
        );
        painter.rect_filled(label, 0.0, Color32::from_rgba_unmultiplied(20, 21, 23, 205));
        painter.rect_filled(
            Rect::from_min_size(label.min, egui::vec2(2.0, label.height())),
            0.0,
            ORANGE,
        );
        painter.galley(
            label.min + egui::vec2(10.0, 4.0),
            galley,
            Color32::from_rgb(0xe4, 0xe6, 0xea),
        );

        // The logo in two rings, the page's dark and the accent.
        let centre = rect.min + egui::vec2(logo.0, logo.1);
        let radius = logo.2;
        let (uri, bytes) = crate::chrome::LOGO;
        egui::Image::from_bytes(uri, bytes)
            .fit_to_exact_size(egui::Vec2::splat(radius * 2.0))
            .paint_at(
                ui,
                Rect::from_center_size(centre, egui::Vec2::splat(radius * 2.0)),
            );
        let painter = ui.painter();
        painter.circle_stroke(centre, radius + 1.5, Stroke::new(3.0, theme::base()));
        painter.circle_stroke(centre, radius + 4.0, Stroke::new(2.0, theme::accent()));

        // The name with a hard shadow, and "level editor" spaced out under it, scaled with
        // the logo.
        let u = radius / 60.0;
        let left = centre.x + radius + 30.0 * u;
        let title = theme::display_bold(72.0 * u);
        for (offset, colour) in [
            (
                egui::vec2(3.0, 3.0) * u,
                Color32::from_rgb(0x0b, 0x1a, 0x33),
            ),
            (egui::Vec2::ZERO, Color32::from_rgb(0xf1, 0xf2, 0xf4)),
        ] {
            painter.text(
                Pos2::new(left, centre.y - 12.0 * u) + offset,
                egui::Align2::LEFT_CENTER,
                "HERON BRUSH",
                title.clone(),
                colour,
            );
        }
        let size = 16.0 * u;
        let tagline = spaced(
            painter,
            &tr("Level editor").to_uppercase(),
            theme::display(size),
            Color32::from_rgb(0x8b, 0xba, 0xff),
            0.62 * size,
        );
        painter.galley(
            Pos2::new(left, centre.y + 30.0 * u - tagline.size().y / 2.0),
            tagline,
            Color32::WHITE,
        );
    }

    /// Moves the keyboard focus with the arrow keys; returns what Return, S or Delete
    /// asked for.
    fn keyboard(
        &mut self,
        ui: &egui::Ui,
        buttons: usize,
        cards: usize,
        columns: usize,
    ) -> Option<Activation> {
        if ui.ctx().egui_wants_keyboard_input() {
            return None;
        }
        let (up, down, left, right, enter, star, trash) = ui.input(|i| {
            let plain = i.modifiers.is_none();
            (
                plain && i.key_pressed(egui::Key::ArrowUp),
                plain && i.key_pressed(egui::Key::ArrowDown),
                plain && i.key_pressed(egui::Key::ArrowLeft),
                plain && i.key_pressed(egui::Key::ArrowRight),
                plain && i.key_pressed(egui::Key::Enter),
                plain && i.key_pressed(egui::Key::S),
                plain && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)),
            )
        });
        if let Some(Focus::Card(card)) = self.focus {
            if star {
                return Some(Activation::Star(card));
            }
            if trash {
                return Some(Activation::Trash(card));
            }
        }
        if !(up || down || left || right || enter) {
            return None;
        }
        let focus = self.focus.unwrap_or(if cards > 0 {
            Focus::Card(0)
        } else {
            Focus::Button(0)
        });
        if self.focus.is_none() && !enter {
            self.focus = Some(focus);
            return None;
        }
        if enter {
            return Some(Activation::Open(focus));
        }
        self.focus = Some(match focus {
            Focus::Button(index) => {
                self.last_button = index;
                if up {
                    Focus::Button(index.saturating_sub(1))
                } else if down {
                    Focus::Button((index + 1).min(buttons - 1))
                } else if right && cards > 0 {
                    Focus::Card(self.last_card.min(cards - 1))
                } else {
                    focus
                }
            }
            Focus::Card(card) => {
                self.last_card = card;
                if left && card % columns == 0 {
                    Focus::Button(self.last_button.min(buttons - 1))
                } else if left {
                    Focus::Card(card - 1)
                } else if right {
                    Focus::Card((card + 1).min(cards.saturating_sub(1)))
                } else if up && card >= columns {
                    Focus::Card(card - columns)
                } else if down && card + columns < cards {
                    Focus::Card(card + columns)
                } else {
                    focus
                }
            }
        });
        None
    }

    /// The menu on the left, and the profile for new maps under it.
    fn menu(
        &mut self,
        ui: &mut egui::Ui,
        items: &[Item],
        last: Option<PathBuf>,
        examples: bool,
        activated: Option<Activation>,
    ) -> Option<Request> {
        let mut request = None;
        let width = ui.available_width();
        ui.spacing_mut().item_spacing.y = 8.0;
        let mut index = 0;
        while index < items.len() {
            let item = items[index];
            let focused = |i: usize| self.focus == Some(Focus::Button(i));
            let pressed = |i: usize| activated == Some(Activation::Open(Focus::Button(i)));
            if item == Item::Manual {
                // The manual and preferences share a row, quieter than the rest.
                let half = (width - 8.0) / 2.0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (offset, label, action) in [
                        (0, tr("Manual"), Action::Manual),
                        (1, tr("Preferences"), Action::Preferences),
                    ] {
                        let at = index + offset;
                        let response = menu_button(
                            ui,
                            &label.to_uppercase(),
                            None,
                            egui::vec2(half, 40.0),
                            Look::Quiet,
                            focused(at),
                        );
                        if focused(at) {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() || pressed(at) {
                            request = Some(Request::Action(action));
                        }
                    }
                });
                index += 2;
                continue;
            }
            let (label, detail, look, height) = match item {
                Item::New => (
                    tr("New map").to_uppercase() + "  ▶",
                    None,
                    Look::Primary,
                    50.0,
                ),
                Item::Continue => {
                    let name = last
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    (tr("Continue").to_uppercase(), Some(name), Look::Plain, 50.0)
                }
                Item::Open => (tr("Open map").to_uppercase() + "…", None, Look::Plain, 42.0),
                Item::Switch => {
                    let label = if examples {
                        tr("Recent maps")
                    } else {
                        tr("Examples")
                    };
                    (label.to_uppercase(), None, Look::Plain, 42.0)
                }
                Item::Manual | Item::Preferences => unreachable!("drawn as a pair above"),
            };
            let mut response = menu_button(
                ui,
                &label,
                detail.as_deref(),
                egui::vec2(width, height),
                look,
                focused(index),
            );
            if focused(index) {
                response.scroll_to_me(None);
            }
            if let (Item::Continue, Some(last)) = (item, &last) {
                response = response.on_hover_text(home_relative(last));
            }
            if response.clicked() || pressed(index) {
                match item {
                    Item::New => request = Some(Request::Action(Action::New)),
                    Item::Continue => request = last.clone().map(Request::Map),
                    Item::Open => request = Some(Request::Action(Action::Open)),
                    Item::Switch => {
                        self.examples = !examples;
                        self.focus = None;
                    }
                    Item::Manual | Item::Preferences => {}
                }
            }
            index += 1;
        }

        // The game a new map is for, the same preference the Games pane writes.
        ui.add_space(6.0);
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
            let caption = spaced(
                ui.painter(),
                &tr("Profile").to_uppercase(),
                theme::display(12.0),
                theme::dim(),
                3.6,
            );
            let (rect, _) = ui.allocate_exact_size(caption.size(), Sense::hover());
            ui.painter().galley(rect.min, caption, theme::dim());
            let mut game = editor_core::preferences::pref(|p| p.game.clone());
            let chosen = self
                .games
                .iter()
                .position(|g| g.name == game)
                .map_or(0, |at| at + 1);
            let mut selected = chosen;
            let changed = crate::widgets::game_dropdown(
                ui,
                "welcome-game",
                &mut selected,
                &self.games,
                width,
            );
            if changed {
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
            ui.label(
                egui::RichText::new(format!(
                    "{} / {}",
                    tr("Entity definitions"),
                    tr("Materials")
                ))
                .small()
                .color(theme::dim()),
            );
        }
        request
    }

    /// The grid on the right: the recent maps, or the examples. Scrolls on its own if
    /// `scroll`, else the page around it does.
    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        shown: &[usize],
        columns: usize,
        activated: Option<Activation>,
        scroll: bool,
    ) -> Option<Request> {
        let mut request = None;
        let mut refresh = false;
        let gap = 12.0;
        let mut cards = |ui: &mut egui::Ui| {
            let width = ui.available_width();
            let card_width = ((width - gap * (columns as f32 - 1.0)) / columns as f32).floor();
            let thumb_height = (card_width * 0.45).round();
            let size = egui::vec2(card_width, thumb_height + 50.0);
            for (row, chunk) in shown.chunks(columns).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (column, &index) in chunk.iter().enumerate() {
                        let card = row * columns + column;
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
                        let image = self.thumbnails.get(&path).and_then(|s| s.image.clone());
                        let focused = self.focus == Some(Focus::Card(card));
                        let (clicked, star, trash) = map_card(
                            ui,
                            size,
                            thumb_height,
                            &path,
                            image.as_ref(),
                            favourite_maps::is_favourite(&path),
                            focused,
                        );
                        if focused {
                            ui.scroll_to_rect(ui.min_rect(), None);
                        }
                        if star || activated == Some(Activation::Star(card)) {
                            favourite_maps::toggle(&path);
                            refresh = true;
                        } else if trash || activated == Some(Activation::Trash(card)) {
                            refresh |= self.trash(path);
                        } else if clicked || activated == Some(Activation::Open(Focus::Card(card)))
                        {
                            request = Some(Request::Map(path));
                        }
                    }
                });
                ui.add_space(gap - ui.spacing().item_spacing.y);
            }
            if !self.error.is_empty() {
                ui.label(egui::RichText::new(&self.error).color(theme::error()));
            }
        };
        if scroll {
            egui::ScrollArea::vertical()
                .id_salt("welcome-maps")
                .auto_shrink([false, false])
                .show(ui, cards);
        } else {
            cards(ui);
        }
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

    /// Asks before moving `path` to the trash, or where there is no wastebasket takes it
    /// off the list. Returns whether the list changed.
    fn trash(&mut self, path: PathBuf) -> bool {
        if !TRASH_CAN {
            // No wastebasket to put it in, and a map is never deleted outright: the card
            // only leaves the list.
            recent_documents::forget(&path);
            favourite_maps::forget(&path);
            return true;
        }
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
        self.trash_answer(answer)
    }

    /// The strip of shortcuts at the foot.
    fn footer(&self, ui: &egui::Ui, rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::base());
        painter.hline(
            rect.x_range(),
            rect.top() + 0.5,
            Stroke::new(1.0, theme::border()),
        );
        let mono = egui::FontId::monospace(12.0);
        let mut job = egui::text::LayoutJob::default();
        let join = crate::menu::ctrl("Shift+J");
        for (key, label) in [
            ("B".to_string(), tr("Brush")),
            ("C".to_string(), tr("Clip")),
            ("V".to_string(), tr("Vertex")),
            (join, tr("Join brushes")),
        ] {
            for (text, colour) in [
                (format!("[{key}] "), orange_text()),
                (format!("{label}    "), theme::dim()),
            ] {
                job.append(&text, 0.0, egui::TextFormat::simple(mono.clone(), colour));
            }
        }
        let galley = painter.layout_job(job);
        painter.galley(
            Pos2::new(
                (rect.right() - 48.0 - galley.size().x).max(rect.left() + 16.0),
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            theme::dim(),
        );
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

/// Narrower than this, the page stands its parts one under another, for a phone held
/// upright.
const NARROW: f32 = 640.0;

/// Lower than this, the page drops the strip of shortcuts and the banner shrinks further,
/// for a phone on its side.
const SHORT: f32 = 480.0;

/// How large a card's thumbnail is drawn, in pixels: a card is up to a few hundred points
/// wide on a screen of two or three pixels to the point, and shows the middle of the square
/// picture. The asset panel's small [`assets::THUMBNAIL`] blew up into blocks there.
const THUMBNAIL: i32 = 512;

/// The heron's beak: the version's mark and the keys in the strip of shortcuts.
const ORANGE: Color32 = Color32::from_rgb(0xf3, 0x9a, 0x2b);

/// [`ORANGE`] as text on the page: deeper on the light scheme, to be read there.
fn orange_text() -> Color32 {
    if theme::is_dark() {
        ORANGE
    } else {
        Color32::from_rgb(0xa8, 0x55, 0x00)
    }
}

/// `text` laid out with `spacing` points between its letters, as the banner and the
/// headings spread them.
fn spaced(
    painter: &egui::Painter,
    text: &str,
    font: egui::FontId,
    colour: Color32,
    spacing: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color: colour,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    painter.layout_job(job)
}

/// The grid's heading, Recent maps or Examples, `size` points high, and a line under it
/// when there is nothing to show.
fn heading(ui: &mut egui::Ui, examples: bool, size: f32, empty: bool) {
    let text = if examples {
        tr("Examples")
    } else {
        tr("Recent maps")
    };
    let galley = spaced(
        ui.painter(),
        &text.to_uppercase(),
        theme::display_bold(size),
        theme::fg(),
        3.0 * size / 32.0,
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), galley.size().y + 10.0),
        Sense::hover(),
    );
    ui.painter().galley(rect.min, galley, theme::fg());
    if empty {
        ui.add_space(20.0);
        ui.label(egui::RichText::new(tr("No recent documents")).color(theme::dim()));
    }
}

/// The outline of `rect` with its top-left and bottom-right corners cut off by `cut`,
/// the menu's and the cards' shape.
fn chamfered(rect: Rect, top_left: f32, bottom_right: f32) -> Vec<Pos2> {
    vec![
        Pos2::new(rect.left() + top_left, rect.top()),
        Pos2::new(rect.right(), rect.top()),
        Pos2::new(rect.right(), rect.bottom() - bottom_right),
        Pos2::new(rect.right() - bottom_right, rect.bottom()),
        Pos2::new(rect.left(), rect.bottom()),
        Pos2::new(rect.left(), rect.top() + top_left),
    ]
}

/// How a menu button stands out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    /// The accent: New map.
    Primary,
    Plain,
    /// The pair at the foot: the manual and preferences.
    Quiet,
}

/// A menu button: its corners cut, its label in the heading face spaced out, and under it
/// `detail` if given.
fn menu_button(
    ui: &mut egui::Ui,
    label: &str,
    detail: Option<&str>,
    size: egui::Vec2,
    look: Look,
    focused: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hovered = response.hovered();
    let fill = match (look, hovered) {
        (Look::Primary, false) => theme::accent(),
        (Look::Primary, true) => theme::accent_hover(),
        (Look::Plain, false) => theme::surface(),
        (Look::Quiet, false) => theme::base(),
        (_, true) => theme::button_hover(),
    };
    let cut = if look == Look::Quiet { 10.0 } else { 12.0 };
    let painter = ui.painter();
    painter.add(egui::Shape::convex_polygon(
        chamfered(rect, cut, cut),
        fill,
        Stroke::NONE,
    ));
    if focused {
        painter.add(egui::Shape::closed_line(
            chamfered(rect.expand(2.0), cut + 1.0, cut + 1.0),
            Stroke::new(1.5, theme::accent_hover()),
        ));
    }
    let colour = match look {
        Look::Primary => theme::on_accent(),
        Look::Plain => theme::fg(),
        Look::Quiet => theme::dim(),
    };
    let font = match look {
        Look::Primary => theme::display_bold(18.0),
        Look::Plain => theme::display(16.0),
        Look::Quiet => theme::display(14.0),
    };
    // Spaced out while it fits; a long word, as some languages have, closes up and then
    // shrinks to fit the button.
    let room = size.x - if look == Look::Quiet { 12.0 } else { 40.0 };
    let mut text = spaced(painter, label, font.clone(), colour, 2.0);
    if text.size().x > room {
        text = spaced(painter, label, font.clone(), colour, 0.0);
        if text.size().x > room {
            let smaller = egui::FontId::new(font.size * room / text.size().x, font.family);
            text = spaced(painter, label, smaller, colour, 0.0);
        }
    }
    let detail = detail.map(|detail| {
        let small = egui::FontId::proportional(theme::text_size() - 0.5);
        let shown = middle_ellipsis(detail, |t| {
            painter
                .layout_no_wrap(t.to_string(), small.clone(), theme::dim())
                .size()
                .x
                <= size.x - 40.0
        });
        painter.layout_no_wrap(shown, small, theme::dim())
    });
    let block = text.size().y + detail.as_ref().map_or(0.0, |d| d.size().y);
    let left = if look == Look::Quiet {
        rect.center().x - text.size().x / 2.0
    } else {
        rect.left() + 20.0
    };
    let top = rect.center().y - block / 2.0;
    let text_height = text.size().y;
    painter.galley(Pos2::new(left, top), text, colour);
    if let Some(detail) = detail {
        painter.galley(Pos2::new(left, top + text_height), detail, theme::dim());
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    response
}

/// One map as a card: its thumbnail, its name and folder,
/// the accent arrow at the corner, and the star and trash buttons over the thumbnail.
/// Returns whether the card, the star or the trash was clicked.
#[allow(
    clippy::too_many_arguments,
    reason = "a card is drawn from all of these"
)]
fn map_card(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    thumb_height: f32,
    path: &Path,
    image: Option<&egui::TextureHandle>,
    starred: bool,
    focused: bool,
) -> (bool, bool, bool) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let thumb = Rect::from_min_size(rect.min, egui::vec2(rect.width(), thumb_height));
    let trash_rect = Rect::from_min_size(
        Pos2::new(thumb.right() - 30.0, thumb.top() + 4.0),
        egui::Vec2::splat(26.0),
    );
    let star_rect = trash_rect.translate(egui::vec2(-30.0, 0.0));
    let star = ui
        .interact(
            star_rect,
            ui.scope_id().with((path, "star")),
            Sense::click(),
        )
        .on_hover_text(tr("Keep at the top"));
    let trash = ui
        .interact(
            trash_rect,
            ui.scope_id().with((path, "trash")),
            Sense::click(),
        )
        .on_hover_text(tr("Move to the trash"));
    let hovered = response.hovered() || star.hovered() || trash.hovered();
    let painter = ui.painter();
    let cut = 12.0;
    painter.add(egui::Shape::convex_polygon(
        chamfered(rect, cut, 0.0),
        if hovered {
            theme::button_hover()
        } else {
            theme::surface()
        },
        Stroke::NONE,
    ));
    painter.rect_filled(thumb, 0.0, theme::base());
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
        painter.image(image.id(), thumb, uv, Color32::WHITE);
    }
    // The cut corner over the thumbnail too, in the page's colour.
    painter.add(egui::Shape::convex_polygon(
        vec![
            rect.min,
            rect.min + egui::vec2(cut, 0.0),
            rect.min + egui::vec2(0.0, cut),
        ],
        theme::bg(),
        Stroke::NONE,
    ));
    if focused {
        painter.add(egui::Shape::closed_line(
            chamfered(rect.expand(2.0), cut + 1.0, 0.0),
            Stroke::new(1.5, theme::accent_hover()),
        ));
    }

    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    let room = rect.width() - 50.0;
    let font = theme::display(13.5);
    let fits = |t: &str, font: &egui::FontId| {
        painter
            .layout_no_wrap(t.to_string(), font.clone(), theme::fg())
            .size()
            .x
            <= room
    };
    let name = middle_ellipsis(&name, |t| fits(t, &font));
    let small = egui::FontId::proportional(theme::text_size() - 1.0);
    let folder = path.parent().map(home_relative).unwrap_or_default();
    let folder = middle_ellipsis(&folder, |t| fits(t, &small));
    let name = spaced(painter, &name, font, theme::fg(), 0.5);
    painter.galley(
        Pos2::new(rect.left() + 10.0, thumb.bottom() + 8.0),
        name,
        theme::fg(),
    );
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.bottom() - 10.0),
        egui::Align2::LEFT_BOTTOM,
        folder,
        small,
        theme::dim(),
    );
    let arrow = Rect::from_min_max(rect.max - egui::Vec2::splat(32.0), rect.max);
    painter.rect_filled(
        arrow,
        0.0,
        if hovered {
            theme::accent_hover()
        } else {
            theme::accent()
        },
    );
    theme::icon_text(
        painter,
        arrow.center(),
        egui::Align2::CENTER_CENTER,
        egui_phosphor::regular::CARET_RIGHT,
        theme::icon(),
        theme::on_accent(),
    );

    for (slot, button, icon) in [(star_rect, &star, 1u8), (trash_rect, &trash, 2u8)] {
        let visible = hovered || focused || (icon == 1 && starred);
        if !visible {
            continue;
        }
        let back = if button.hovered() {
            theme::button_hover()
        } else {
            Color32::from_rgba_unmultiplied(20, 21, 23, 200)
        };
        painter.rect_filled(slot, 0.0, back);
        let colour = if button.hovered() {
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
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            path.file_name().unwrap_or_default().to_string_lossy(),
        )
    });
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
