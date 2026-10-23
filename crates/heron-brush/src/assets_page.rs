//! The inspector's Assets tab, over the core's `assets`: the library
//! folders, a filter, a grid of thumbnails, Insert, dragging onto a view, and saving the
//! selection as an asset.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;
use web_time::Instant;

use editor_core::assets::{self, Prefab, Rgba, THUMBNAIL};
use editor_core::document::Document;
use editor_core::grid::Grid;
use editor_core::i18n::{fill, tr};
use eframe::egui;
use geometry_math::vec::Vec3d;

use crate::theme;
use crate::widgets::{self, Dragged};

/// A thumbnail, as far as it has got.
enum Thumbnail {
    /// Being decoded on the worker.
    Loading,
    /// Ready to draw.
    Ready(egui::TextureHandle),
    /// Nothing could be made.
    None,
}

/// The Assets tab's own state.
pub struct AssetsPage {
    filter: String,
    /// The prefab picked in the grid.
    selected: Option<PathBuf>,
    /// Whether the grid was the last thing clicked, so the arrows and Enter work it.
    grid_focused: bool,
    /// The prefabs found, and what the files looked like when they were found, so the
    /// grid is rebuilt only when files change.
    prefabs: Vec<Prefab>,
    signature: Vec<(PathBuf, SystemTime)>,
    scanned: Option<Instant>,
    thumbnails: HashMap<PathBuf, Thumbnail>,
    /// Paths to decode, with the context to wake once each is ready, and the decoded
    /// thumbnails coming back.
    previews: crate::pool::Pool<(PathBuf, egui::Context), (PathBuf, Option<Rgba>)>,
}

/// How often the asset folders are looked at again while the tab shows.
const RESCAN_SECS: u64 = 3;

impl Default for AssetsPage {
    fn default() -> Self {
        // A few workers share the queue, rather than a thread per library entry: drawing
        // a prefab's preview takes milliseconds, and a library of hundreds queued behind
        // one worker took seconds to fill in. Only cells in view ask, and what was drawn
        // before comes back from the disk cache.
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get().clamp(2, 8) / 2);
        let cache = assets::thumbnail_cache_dir();
        let previews = crate::pool::Pool::new(
            "prefab previews",
            workers,
            move |(path, ctx): (PathBuf, egui::Context)| {
                let image = assets::thumbnail_cached(&path, &cache);
                ctx.request_repaint();
                (path, image)
            },
        );
        Self {
            filter: String::new(),
            selected: None,
            grid_focused: false,
            prefabs: Vec::new(),
            signature: Vec::new(),
            scanned: None,
            thumbnails: HashMap::new(),
            previews,
        }
    }
}

impl AssetsPage {
    /// Draws the tab.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        document: &Rc<RefCell<Document>>,
        grid: &Rc<RefCell<Grid>>,
    ) {
        let width = ui.available_width();
        self.rescan(false);
        // The window sleeps while nothing happens; the folders are looked at again
        // every few seconds while the tab shows.
        ui.ctx()
            .request_repaint_after(web_time::Duration::from_secs(RESCAN_SECS));
        while let Some((path, image)) = self.previews.try_recv() {
            let thumbnail = match image {
                Some(image) => {
                    let pixels = egui::ColorImage::from_rgba_unmultiplied(
                        [image.width as usize, image.height as usize],
                        &image.pixels,
                    );
                    Thumbnail::Ready(ui.ctx().load_texture(
                        path.display().to_string(),
                        pixels,
                        egui::TextureOptions::LINEAR,
                    ))
                }
                None => Thumbnail::None,
            };
            self.thumbnails.insert(path, thumbnail);
            ui.ctx().request_repaint();
        }

        let add = theme::button(ui, tr("Add folder"), width, true).clicked();
        if let Some(path) = crate::file_dialog::FileDialog::new()
            .set_title(tr("Asset folder"))
            .pick_folder(add)
        {
            let mut library = assets::folders();
            if !library.contains(&path) {
                library.push(path);
                assets::set_folders(&library);
            }
            self.rescan(true);
        }
        let library = assets::folders();
        if !library.is_empty() {
            widgets::section_noted(
                ui,
                "asset-folders",
                tr("Folders"),
                Some(&library.len().to_string()),
                false,
                |ui| {
                    let items: Vec<(String, String)> = library
                        .iter()
                        .map(|folder| (folder.display().to_string(), folder.display().to_string()))
                        .collect();
                    crate::material_page::removable_list(
                        ui,
                        "asset-folders",
                        &items,
                        tr("Stop using this folder"),
                        |at| {
                            let mut library = library.clone();
                            library.remove(at);
                            assets::set_folders(&library);
                            self.rescan(true);
                        },
                    );
                },
            );
        }
        widgets::search(ui, &mut self.filter, tr("Filter assets"));

        // Typing only filters what is there: no scans, decoding or map parsing.
        let shown = self.shown();
        let mut insert = false;
        let mut scroll_to_selected = false;
        let grid_area = widgets::list(ui, 0.0, |ui| {
            if shown.is_empty() {
                let text = if library.is_empty() {
                    tr("No folders yet. Add one above.")
                } else if self.prefabs.is_empty() {
                    tr("No assets")
                } else {
                    tr("No matching assets")
                };
                widgets::empty(ui, text);
            }
            egui::ScrollArea::vertical()
                .id_salt("asset-grid")
                .max_height(360.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // The keyboard, while the grid has it: arrows move the pick a cell or a line,
                    // Enter inserts it.
                    let gap = 4.0;
                    let (per_line, cell_width) =
                        widgets::grid_cells(ui.available_width(), THUMBNAIL as f32 + 12.0, gap);
                    let cell = egui::vec2(cell_width, THUMBNAIL as f32 + 30.0);
                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                    if self.grid_focused && ui.memory(|m| m.focused().is_none()) {
                        let step = ui.input(|i| {
                            if i.key_pressed(egui::Key::ArrowLeft) {
                                -1
                            } else if i.key_pressed(egui::Key::ArrowRight) {
                                1
                            } else if i.key_pressed(egui::Key::ArrowUp) {
                                -(per_line as isize)
                            } else if i.key_pressed(egui::Key::ArrowDown) {
                                per_line as isize
                            } else {
                                0
                            }
                        });
                        // Taken here, so the tools' own Enter does not see it too.
                        let enter = self.selected.is_some()
                            && ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            });
                        if step != 0 {
                            self.step_selection(&shown, step);
                            scroll_to_selected = true;
                        }
                        if enter {
                            insert = true;
                        }
                    }
                    for line in shown.chunks(per_line) {
                        // A line scrolled out of sight takes its room and nothing else: no layout,
                        // no painting and no thumbnail asked for.
                        let line_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), cell.y),
                        );
                        if !ui.clip_rect().intersects(line_rect) {
                            if scroll_to_selected
                                && line.iter().any(|&index| {
                                    self.selected.as_ref() == Some(&self.prefabs[index].path)
                                })
                            {
                                ui.scroll_to_rect(line_rect, None);
                            }
                            ui.allocate_space(egui::vec2(ui.available_width(), cell.y));
                            continue;
                        }
                        ui.horizontal(|ui| {
                            for &index in line {
                                let prefab = &self.prefabs[index];
                                let picked = self.selected.as_ref() == Some(&prefab.path);
                                let (rect, response) = ui.allocate_exact_size(
                                    cell,
                                    widgets::drag_source_sense(ui.ctx()),
                                );
                                widgets::paint_list_row(ui, rect, picked, response.hovered());
                                if picked && scroll_to_selected {
                                    ui.scroll_to_rect(rect, None);
                                }
                                let image = egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.center().x - THUMBNAIL as f32 / 2.0,
                                        rect.top() + 4.0,
                                    ),
                                    egui::Vec2::splat(THUMBNAIL as f32),
                                );
                                match self.thumbnails.get(&prefab.path) {
                                    Some(Thumbnail::Ready(texture)) => {
                                        paint_thumbnail(ui.painter(), texture, image)
                                    }
                                    Some(_) => {
                                        ui.painter().rect_filled(
                                            image,
                                            egui::CornerRadius::same(4),
                                            theme::entry(),
                                        );
                                    }
                                    None => {
                                        ui.painter().rect_filled(
                                            image,
                                            egui::CornerRadius::same(4),
                                            theme::entry(),
                                        );
                                        self.thumbnails
                                            .insert(prefab.path.clone(), Thumbnail::Loading);
                                        self.previews.send((prefab.path.clone(), ui.ctx().clone()));
                                    }
                                }
                                let name = egui::Rect::from_min_max(
                                    egui::pos2(rect.left() + 3.0, image.bottom() + 2.0),
                                    egui::pos2(rect.right() - 3.0, rect.bottom()),
                                );
                                let (galley, _) = widgets::truncated(
                                    ui,
                                    &prefab.name,
                                    theme::body(),
                                    theme::fg(),
                                    name.width(),
                                );
                                ui.painter().galley(
                                    egui::pos2(
                                        name.center().x - galley.size().x / 2.0,
                                        crate::theme::galley_top(
                                            ui.ctx(),
                                            &galley,
                                            name.center().y,
                                        ),
                                    ),
                                    galley,
                                    theme::fg(),
                                );
                                let response =
                                    response.on_hover_text(prefab.path.display().to_string());
                                // Drag the cell onto a view to drop the prefab there; its thumbnail
                                // follows the pointer as the drag icon.
                                response.dnd_set_drag_payload(Dragged::Prefab(prefab.path.clone()));
                                if response.dragged() {
                                    if let (Some(Thumbnail::Ready(texture)), Some(at)) = (
                                        self.thumbnails.get(&prefab.path),
                                        ui.ctx().pointer_interact_pos(),
                                    ) {
                                        let layer = egui::LayerId::new(
                                            egui::Order::Tooltip,
                                            egui::Id::unique("asset-drag-icon"),
                                        );
                                        let half = THUMBNAIL as f32 / 2.0;
                                        paint_thumbnail(
                                            &ui.ctx().layer_painter(layer),
                                            texture,
                                            egui::Rect::from_center_size(
                                                at,
                                                egui::Vec2::splat(half * 2.0),
                                            ),
                                        );
                                    }
                                }
                                if response.clicked() || response.drag_started() {
                                    self.selected = Some(prefab.path.clone());
                                    self.grid_focused = true;
                                }
                                if response.double_clicked() {
                                    self.selected = Some(prefab.path.clone());
                                    insert = true;
                                }
                            }
                        });
                    }
                })
        });
        // The grid keeps the keyboard until a press lands somewhere else.
        if ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|at| !grid_area.inner_rect.contains(at))
        }) {
            self.grid_focused = false;
        }
        // An empty library says so in the grid.
        if !library.is_empty() {
            widgets::status(
                ui,
                &fill(
                    "{} of {} assets shown",
                    &[&shown.len().to_string(), &self.prefabs.len().to_string()],
                ),
            );
        }

        let width = ui.available_width();
        let can_insert = self.selected.is_some();
        if theme::button(ui, tr("Insert"), width, can_insert)
            .on_hover_text(tr("Add this asset where the pointer last was"))
            .clicked()
        {
            insert = true;
        }
        if insert {
            self.insert(document, grid);
        }
        let save = theme::button(ui, tr("Save selection as asset"), width, true)
            .on_hover_text(tr("Write the selected brushes to the library as a .map"))
            .clicked();
        self.save_selection(document, save);
    }

    /// The prefabs the filter lets through, as indices. A pick the filter hides is
    /// dropped, so nothing hidden stays selected.
    fn shown(&mut self) -> Vec<usize> {
        let query = self.filter.to_lowercase();
        let shown: Vec<usize> = (0..self.prefabs.len())
            .filter(|&i| self.prefabs[i].name.to_lowercase().contains(&query))
            .collect();
        if self
            .selected
            .as_ref()
            .is_some_and(|path| !shown.iter().any(|&i| &self.prefabs[i].path == path))
        {
            self.selected = None;
        }
        shown
    }

    /// Moves the pick `step` cells through what is shown, stopping at the ends; with
    /// nothing picked the first cell is.
    fn step_selection(&mut self, shown: &[usize], step: isize) {
        if shown.is_empty() {
            return;
        }
        let at = self
            .selected
            .as_ref()
            .and_then(|path| shown.iter().position(|&i| &self.prefabs[i].path == path));
        let next = match at {
            None => 0,
            Some(at) => {
                let moved = at as isize + step;
                if moved < 0 || moved >= shown.len() as isize {
                    at
                } else {
                    moved as usize
                }
            }
        };
        self.selected = Some(self.prefabs[shown[next]].path.clone());
    }

    /// Looks the folders over again when asked, or every few seconds, and rebuilds the grid
    /// only when the files changed.
    fn rescan(&mut self, now: bool) {
        if !now
            && self
                .scanned
                .is_some_and(|at| at.elapsed().as_secs() < RESCAN_SECS)
        {
            return;
        }
        self.scanned = Some(Instant::now());
        let all: Vec<Prefab> = assets::folders()
            .iter()
            .flat_map(|folder| assets::scan(folder))
            .collect();
        let signature: Vec<(PathBuf, SystemTime)> = all
            .iter()
            .flat_map(|prefab| {
                std::iter::once(prefab.path.clone())
                    .chain(assets::thumbnail_candidates(&prefab.path))
                    .map(|path| {
                        let when = assets::modified(&path);
                        (path, when)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        if signature != self.signature {
            self.signature = signature;
            self.prefabs = all;
            self.thumbnails.clear();
        }
    }

    /// Inserts the picked prefab where the selection is, or at the origin -- not where the
    /// pointer last hovered a view, which reaching this button means crossing. Dragging a
    /// cell onto a view is how a place is chosen.
    fn insert(&mut self, document: &Rc<RefCell<Document>>, grid: &Rc<RefCell<Grid>>) {
        let Some(path) = self.selected.clone() else {
            return;
        };
        let at = {
            let document = document.borrow();
            let at = document
                .selection_bounds()
                .map_or(Vec3d::zero(), |bounds| bounds.center());
            let at = if (0..3).all(|axis| at[axis].is_finite()) {
                at
            } else {
                Vec3d::zero()
            };
            grid.borrow().snap_point(at)
        };
        assets::insert_prefab_on_grid(&mut document.borrow_mut(), &path, at, &grid.borrow());
        self.rescan(true);
    }

    /// Saves the selection as an asset map, where the chooser says; `open` asks where.
    /// Called every frame, since an in-app chooser answers frames later.
    fn save_selection(&mut self, document: &Rc<RefCell<Document>>, open: bool) {
        if open && document.borrow().copy_selection().is_none() {
            editor_core::complain!("{}", tr("Select something to save as an asset"));
            return;
        }
        let mut dialog = crate::file_dialog::FileDialog::new()
            .id("save-asset")
            .set_title(tr("Save asset"))
            .set_file_name("asset.map");
        if let Some(folder) = assets::folders().first() {
            dialog = dialog.set_directory(folder);
        }
        let Some(mut path) = dialog.save_file(open) else {
            return;
        };
        let Some(text) = document.borrow().copy_selection() else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("map");
        }
        match std::fs::write(&path, &text) {
            Ok(()) => editor_core::say!("{}", fill("Saved {}", &[&path.display().to_string()])),
            Err(error) => editor_core::complain!(
                "{}",
                fill(
                    "Could not save {}: {}",
                    &[&path.display().to_string(), &error.to_string()]
                )
            ),
        }
        self.rescan(true);
    }
}

/// A thumbnail fitted into `rect`, keeping its shape.
fn paint_thumbnail(painter: &egui::Painter, texture: &egui::TextureHandle, rect: egui::Rect) {
    let size = texture.size_vec2();
    let fit = rect.width().min(rect.height()) / size.x.max(size.y);
    let drawn = egui::Rect::from_center_size(rect.center(), size * fit);
    painter.image(
        texture.id(),
        drawn,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(names: &[&str]) -> AssetsPage {
        AssetsPage {
            prefabs: names
                .iter()
                .map(|n| Prefab {
                    name: n.to_string(),
                    path: PathBuf::from(format!("/lib/{n}.map")),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn filtering_out_the_picked_asset_unpicks_it() {
        let mut page = page(&["crate", "barrel", "crate_small"]);
        page.selected = Some(PathBuf::from("/lib/barrel.map"));
        page.filter = "bar".into();
        assert_eq!(page.shown(), vec![1]);
        assert!(page.selected.is_some(), "still shown, still picked");
        page.filter = "crate".into();
        assert_eq!(page.shown(), vec![0, 2]);
        assert_eq!(page.selected, None, "a hidden asset cannot stay picked");
    }

    #[test]
    fn arrows_walk_the_shown_cells_and_stop_at_the_ends() {
        let mut page = page(&["a", "b", "c", "d", "e"]);
        let shown = page.shown();
        page.step_selection(&shown, 1);
        assert_eq!(
            page.selected,
            Some(PathBuf::from("/lib/a.map")),
            "the first arrow picks the first cell"
        );
        page.step_selection(&shown, 3);
        assert_eq!(page.selected, Some(PathBuf::from("/lib/d.map")));
        page.step_selection(&shown, 3);
        assert_eq!(
            page.selected,
            Some(PathBuf::from("/lib/d.map")),
            "no line below"
        );
        page.step_selection(&shown, -1);
        assert_eq!(page.selected, Some(PathBuf::from("/lib/c.map")));
    }
}
