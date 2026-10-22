//! The Face tab's material browser: loading collections, a filtered grid of thumbnails, the current material, and putting
//! it on faces, selecting by it, or replacing it across the map.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use editor_core::document::Document;
use editor_core::i18n::{counted, fill, tr, translation_key};
use eframe::egui;
use map_model::material::Material;

use crate::dialog::{self, Part};
use crate::{file_dialog::FileDialog, theme, widgets};

/// How large a thumbnail is drawn, in points.
const THUMBNAIL: f32 = 72.0;

/// The longest side of a thumbnail's texture, in pixels: a cell is 72 points, so this is
/// sharp up to 2x and a fraction of the full image's upload.
const THUMBNAIL_PIXELS: u32 = 144;
/// How many thumbnail textures are kept; the least recently drawn go first.
const THUMBNAILS_KEPT: usize = 400;
/// How long a frame may spend making thumbnails before leaving the rest to the next one.
const THUMBNAIL_BUDGET: web_time::Duration = web_time::Duration::from_millis(4);

/// The thumbnails the grid has drawn: small textures made when a cell first scrolls into
/// view, and dropped least recently used first. Made again when the collection changes.
///
/// A material read lazily from a file is decoded and shrunk on a worker thread (decoding
/// a texture takes milliseconds to tens of them, which the UI thread must not wait for);
/// one whose pixels are in memory already (a `.wad`'s, a model skin) is shrunk here, a
/// few per frame so scrolling never stalls.
#[derive(Default)]
pub struct Thumbnails {
    /// The collection they were made from (`Document::materials_revision`).
    revision: u64,
    textures: HashMap<String, (egui::TextureHandle, u64)>,
    /// Names sent to the workers and not back yet.
    requested: std::collections::HashSet<String>,
    workers: Option<ThumbnailWorkers>,
    /// Counts frames, for least recently used.
    frame: u64,
    /// When this frame's thumbnail work began, and whether some were left for later.
    started: Option<web_time::Instant>,
    pub deferred: bool,
    /// How many have been made, for the tests.
    pub made: usize,
}

impl Thumbnails {
    /// Starts a frame of the grid for the collection at `revision`.
    fn begin(&mut self, ctx: &egui::Context, revision: u64) {
        if revision != self.revision {
            self.textures.clear();
            self.requested.clear();
            self.revision = revision;
        }
        // What the workers finished since the last frame.
        while let Some((at, name, image)) = self.workers.as_ref().and_then(|w| w.pool.try_recv()) {
            if at != self.revision || !self.requested.remove(&name) {
                continue;
            }
            if let Some(image) = image {
                let texture = ctx.load_texture(
                    format!("material-thumbnail:{name}"),
                    image,
                    thumbnail_options(),
                );
                self.textures.insert(name, (texture, self.frame + 1));
                self.made += 1;
            }
        }
        self.frame += 1;
        self.started = None;
        self.deferred = false;
    }

    /// Ends the frame: drops the least recently drawn beyond the limit.
    fn end(&mut self, ctx: &egui::Context) {
        if self.textures.len() > THUMBNAILS_KEPT {
            let mut ages: Vec<(u64, String)> = self
                .textures
                .iter()
                .map(|(name, (_, used))| (*used, name.clone()))
                .collect();
            ages.sort();
            for (_, name) in ages.into_iter().take(self.textures.len() - THUMBNAILS_KEPT) {
                self.textures.remove(&name);
            }
        }
        if self.deferred {
            ctx.request_repaint();
        }
    }

    /// Whether thumbnails are still being made on the workers.
    #[cfg(test)]
    pub fn waiting(&self) -> usize {
        self.requested.len()
    }

    /// The material's thumbnail, made now if this frame still has time for it.
    fn get(&mut self, ctx: &egui::Context, material: &Material) -> Option<egui::TextureHandle> {
        if let Some((texture, used)) = self.textures.get_mut(&material.name) {
            *used = self.frame;
            return Some(texture.clone());
        }
        if self.requested.contains(&material.name) {
            return None;
        }
        if material.resident_pixels().is_none() {
            if self.requested.insert(material.name.clone()) {
                let revision = self.revision;
                self.workers
                    .get_or_insert_with(|| ThumbnailWorkers::new(ctx))
                    .request(revision, material.clone());
            }
            return None;
        }
        let started = *self.started.get_or_insert_with(web_time::Instant::now);
        if started.elapsed() > THUMBNAIL_BUDGET {
            self.deferred = true;
            return None;
        }
        let image = thumbnail_image(material)?;
        let texture = ctx.load_texture(
            format!("material-thumbnail:{}", material.name),
            image,
            thumbnail_options(),
        );
        self.textures
            .insert(material.name.clone(), (texture.clone(), self.frame));
        self.made += 1;
        Some(texture)
    }

    /// How many are kept now.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.textures.len()
    }
}

fn thumbnail_options() -> egui::TextureOptions {
    egui::TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Linear,
        ..Default::default()
    }
}

/// A few threads that decode and shrink lazily read materials for the grid, and wake the
/// UI when one is done. A thumbnail asked for in a collection that has since been replaced
/// is still made but dropped on arrival.
struct ThumbnailWorkers {
    pool: crate::pool::Pool<(u64, Material), (u64, String, Option<egui::ColorImage>)>,
}

impl ThumbnailWorkers {
    fn new(ctx: &egui::Context) -> Self {
        let ctx = ctx.clone();
        let pool = crate::pool::Pool::new(
            "thumbnails",
            2,
            move |(revision, material): (u64, Material)| {
                let image = thumbnail_image(&material);
                ctx.request_repaint();
                (revision, material.name.clone(), image)
            },
        );
        ThumbnailWorkers { pool }
    }

    fn request(&self, revision: u64, material: Material) {
        self.pool.send((revision, material));
    }
}

/// A material's image shrunk to at most [`THUMBNAIL_PIXELS`] on its longer side, each
/// pixel the average of a few samples spread over the source pixels it covers (up to
/// four by four), so a 2048-pixel texture costs a quarter of a million reads rather than
/// four million, and an upload of 80 KB rather than 16 MB.
///
/// Decodes the image if its pixels are not in memory (so call it off the UI thread for a
/// lazily read material), and holds them only while shrinking.
pub fn thumbnail_image(material: &Material) -> Option<egui::ColorImage> {
    let (w, h) = (material.width as usize, material.height as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let pixels = material.pixels()?;
    let longest = w.max(h);
    if longest <= THUMBNAIL_PIXELS as usize {
        return Some(egui::ColorImage::from_rgba_unmultiplied(
            [w, h],
            &pixels[..w * h * 4],
        ));
    }
    let scale = longest as f64 / f64::from(THUMBNAIL_PIXELS);
    let (tw, th) = (
        ((w as f64 / scale).round() as usize).max(1),
        ((h as f64 / scale).round() as usize).max(1),
    );
    let k = (scale.ceil() as usize).clamp(1, 4);
    let mut out = Vec::with_capacity(tw * th * 4);
    for y in 0..th {
        for x in 0..tw {
            let mut sum = [0u32; 4];
            for sy in 0..k {
                let py = (((y as f64 + (sy as f64 + 0.5) / k as f64) * scale) as usize).min(h - 1);
                for sx in 0..k {
                    let px =
                        (((x as f64 + (sx as f64 + 0.5) / k as f64) * scale) as usize).min(w - 1);
                    let at = (py * w + px) * 4;
                    for c in 0..4 {
                        sum[c] += u32::from(pixels[at + c]);
                    }
                }
            }
            let n = (k * k) as u32;
            out.extend(sum.map(|c| ((c + n / 2) / n) as u8));
        }
    }
    Some(egui::ColorImage::from_rgba_unmultiplied([tw, th], &out))
}

/// The browser's own state.
#[derive(Default)]
pub struct MaterialPage {
    filter: String,
    /// The grid's thumbnails.
    pub thumbnails: Thumbnails,
    /// The materials the filter matched (indices into the collection), with the filter and
    /// collection they were matched in, and how many the collection shows at all: matching
    /// ten thousand names every frame was a frame's worth of work on its own.
    matched: (String, u64, Vec<usize>, usize),
    /// Scroll the grid to the current material on the next frame: "Reveal in Material
    /// Browser".
    pub reveal: bool,
    /// The Replace dialog, while it is open.
    replace: Option<Replace>,
}

impl MaterialPage {
    /// Draws the browser.
    pub fn show(&mut self, ui: &mut egui::Ui, document: &Rc<RefCell<Document>>) {
        let loads = [
            (tr("Load folder"), "", true),
            (tr("Load .wad"), "", true),
            (tr("Reload"), "", true),
        ];
        let clicked = widgets::segmented(ui, "material-loads", &loads);
        if clicked == Some(2) {
            document.borrow_mut().reload_game_assets();
        }
        {
            if let Some(path) = FileDialog::new()
                .set_title(tr("Material folder"))
                .pick_folder(clicked == Some(0))
            {
                match document.borrow_mut().load_materials(&path) {
                    Ok(warnings) => {
                        for warning in warnings {
                            editor_core::complain!("materials: {warning}");
                        }
                    }
                    Err(error) => editor_core::complain!("{}: {error}", path.display()),
                }
            }
            // A `.wad` is how the Quake lineage ships its textures, and a map names the ones
            // it uses in worldspawn's `wad` property; those are loaded when the map opens.
            // This is for one that is not named there.
            let picked = FileDialog::new()
                .set_title(tr("Texture package"))
                .add_filter(tr("Texture packages"), &["wad", "WAD"])
                .pick_file(clicked == Some(1));
            if let Some(path) = picked {
                match document.borrow_mut().load_wad(&path) {
                    Ok(warnings) => {
                        for warning in warnings {
                            editor_core::complain!("wad: {warning}");
                        }
                    }
                    Err(error) => editor_core::complain!("{}: {error}", path.display()),
                }
            }
        }

        // Each collection can be removed on its own; nothing loaded is nothing to fold out.
        let sources: Vec<(String, String)> = document
            .borrow()
            .material_sources()
            .iter()
            .map(|source| {
                (
                    source.name().to_string(),
                    source.path().display().to_string(),
                )
            })
            .collect();
        if !sources.is_empty() {
            let count = sources.len().to_string();
            widgets::section_noted(
                ui,
                "material-collections",
                tr("Collections"),
                Some(&count),
                false,
                |ui| {
                    removable_list(
                        ui,
                        "material-sources",
                        &sources,
                        tr("Stop using this collection"),
                        |at| {
                            document.borrow_mut().remove_material_source(at);
                        },
                    );
                },
            );
        }

        widgets::search(ui, &mut self.filter, tr("Filter materials"));

        let current = document.borrow().current_material().to_string();
        let mut picked = None;
        let (shown, total) = {
            let document = document.borrow();
            let revision = document.materials_revision();
            let all = document.materials().materials();
            if self.matched.0 != self.filter
                || self.matched.1 != revision
                || self.matched.2.iter().any(|&i| i >= all.len())
            {
                let needle = self.filter.to_lowercase();
                // The `#`-prefixed names are the editor's own -- a model's skin, which the
                // renderer needs in the collection and no face can wear.
                let visible = all
                    .iter()
                    .filter(|material| !material.name.starts_with('#'))
                    .count();
                let matched: Vec<usize> = all
                    .iter()
                    .enumerate()
                    .filter(|(_, material)| !material.name.starts_with('#'))
                    .filter(|(_, material)| {
                        needle.is_empty()
                            || material.name.to_lowercase().contains(&needle)
                            || (material.path.starts_with("builtin/special")
                                && format!("special/{}", material.name).contains(&needle))
                    })
                    .map(|(index, _)| index)
                    .collect();
                self.matched = (self.filter.clone(), revision, matched, visible);
            }
            let matched = std::mem::take(&mut self.matched.2);
            let visible = self.matched.3;
            self.thumbnails.begin(ui.ctx(), revision);
            widgets::list(ui, 0.0, |ui| {
                if matched.is_empty() {
                    widgets::empty(
                        ui,
                        if visible == 0 {
                            tr("No materials loaded")
                        } else {
                            tr("No matching materials")
                        },
                    );
                    return;
                }
                let gap = 4.0;
                let (per_line, cell) = widgets::grid_cells(
                    ui.available_width() - ui.spacing().scroll.allocated_width(),
                    THUMBNAIL + 12.0,
                    gap,
                );
                let lines = matched.len().div_ceil(per_line);
                let line_height = THUMBNAIL + 44.0;
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("material-grid")
                    .min_scrolled_height(180.0)
                    .max_height(360.0);
                // "Reveal in Material Browser": only the lines in view are laid out, so
                // the current one is scrolled to by where its line is.
                if std::mem::take(&mut self.reveal) {
                    if let Some(at) = matched.iter().position(|&i| all[i].name == current) {
                        area = area.vertical_scroll_offset(
                            ((at / per_line) as f32 * (line_height + gap) - 120.0).max(0.0),
                        );
                    }
                }
                ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                // Only the lines in view are laid out and only their thumbnails made.
                area.show_rows(ui, line_height, lines, |ui, range| {
                    for line in range {
                        ui.horizontal(|ui| {
                            for &index in matched.iter().skip(line * per_line).take(per_line) {
                                let material = &all[index];
                                if self.cell(ui, material, material.name == current, cell) {
                                    picked = Some(material.name.clone());
                                }
                            }
                        });
                    }
                });
            });
            self.thumbnails.end(ui.ctx());
            let shown = matched.len();
            self.matched.2 = matched;
            (shown, visible)
        };
        if let Some(name) = picked {
            let changed = {
                let mut document = document.borrow_mut();
                document.set_current_material(name);
                document.apply_current_material()
            };
            if changed > 0 {
                editor_core::say!("put the material on {changed} faces");
            }
            ui.ctx().request_repaint();
        }
        let current = document.borrow().current_material().to_string();

        widgets::status(
            ui,
            &fill(
                "{} · {} of {} shown",
                &[&current, &shown.to_string(), &total.to_string()],
            ),
        );

        // Putting it on, selecting by it (whole brushes, or only the faces wearing it), and
        // replacing it.
        ui.horizontal(|ui| {
            let gap = ui.spacing().item_spacing.x;
            let width = ui.available_width() - 2.0 * gap;
            let side = (width * 0.28).floor();
            if theme::button(ui, tr("Apply to selection"), width - 2.0 * side, true)
                .on_hover_text(tr("Put this material on every selected face"))
                .clicked()
            {
                let changed = document.borrow_mut().apply_current_material();
                editor_core::say!("put the material on {changed} faces");
            }
            let select = theme::button(ui, tr("Select"), side, true);
            egui::Popup::menu(&select).show(|ui| {
                if ui
                    .button(tr("Select brushes with it"))
                    .on_hover_text(tr("Select every brush with this material on a face"))
                    .clicked()
                {
                    document.borrow_mut().select_brushes_with_material(&current);
                    let count = document.borrow().selection_count();
                    editor_core::say!("selected {count} brushes wearing {current}");
                    ui.close();
                }
                if ui
                    .button(tr("Select faces with it"))
                    .on_hover_text(tr("Select every face wearing this material"))
                    .clicked()
                {
                    let count = document.borrow_mut().select_faces_with_material(&current);
                    editor_core::say!("selected {count} faces wearing {current}");
                    ui.close();
                }
            });
            if theme::button(ui, tr("Replace"), side, true)
                .on_hover_text(tr(
                    "Swap one material for another across the map, or the selection",
                ))
                .clicked()
            {
                self.replace = Some(Replace::default());
            }
        });

        if let Some(replace) = &mut self.replace {
            if !replace.show(ui.ctx(), document) {
                self.replace = None;
            }
        }
    }

    /// One cell of the grid: the image over its name and size. Returns whether it was
    /// clicked.
    fn cell(&mut self, ui: &mut egui::Ui, material: &Material, current: bool, width: f32) -> bool {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, THUMBNAIL + 44.0), egui::Sense::click());
        widgets::paint_list_row(ui, rect, current, response.hovered());
        let image = egui::Rect::from_min_size(
            egui::pos2(rect.center().x - THUMBNAIL / 2.0, rect.top() + 4.0),
            egui::Vec2::splat(THUMBNAIL),
        );
        if let Some(texture) = self.thumbnails.get(ui.ctx(), material) {
            let size = egui::vec2(material.width as f32, material.height as f32);
            let fit = THUMBNAIL / size.x.max(size.y).max(1.0);
            let drawn = egui::Rect::from_center_size(image.center(), size * fit);
            ui.painter().image(
                texture.id(),
                drawn,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.painter()
                .rect_filled(image, egui::CornerRadius::same(4), theme::entry());
        }
        let display_name = if material.path.starts_with("builtin/special") {
            format!("special/{}", material.name)
        } else {
            material.name.clone()
        };
        let caption = egui::FontId::proportional(theme::text_size() * 0.8);
        let name = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 3.0, image.bottom() + 2.0),
            egui::pos2(rect.right() - 3.0, image.bottom() + 20.0),
        );
        let (galley, _) = widgets::truncated(
            ui,
            &display_name,
            caption.clone(),
            theme::fg(),
            name.width(),
        );
        ui.painter().galley(
            egui::pos2(
                name.center().x - galley.size().x / 2.0,
                crate::theme::galley_top(ui.ctx(), &galley, name.center().y),
            ),
            galley,
            theme::fg(),
        );
        ui.painter().text(
            egui::pos2(rect.center().x, name.bottom() + 9.0),
            egui::Align2::CENTER_CENTER,
            format!("{}×{}", material.width, material.height),
            caption,
            theme::dim(),
        );
        response
            .on_hover_text(format!(
                "{}\n{}",
                material.name,
                tr("Put this material on every selected face")
            ))
            .clicked()
    }
}

/// The document and collection revisions, then the material names in use and all of them.
type MaterialNames = ((u64, u64), Vec<String>, Vec<String>);

/// The Replace Material dialog: a material in use on the left, what to put in its place
/// on the right, across the selection when there is one and the map otherwise.
#[derive(Default)]
struct Replace {
    subject: Option<String>,
    replacement: Option<String>,
    subject_filter: String,
    with_filter: String,
    /// What the last replace did, until the picks change.
    said: Option<String>,
    /// The names in use and all the names, with the document and collection revisions they
    /// were read at: finding what the map wears walks every face.
    names: Option<MaterialNames>,
}

impl Replace {
    /// Shows the dialog; false once it is closed.
    fn show(&mut self, ctx: &egui::Context, document: &Rc<RefCell<Document>>) -> bool {
        let key = {
            let document = document.borrow();
            (document.revision(), document.materials_revision())
        };
        if self.names.as_ref().is_none_or(|(at, _, _)| *at != key) {
            let document = document.borrow();
            let all: Vec<String> = document
                .materials()
                .materials()
                .iter()
                // A `#` name is the editor's own -- a model's skin -- and no face wears
                // one, so it is not something to swap to.
                .filter(|material| !material.name.starts_with('#'))
                .map(|material| material.name.clone())
                .collect();
            self.names = Some((key, document.materials_in_use(), all));
        }
        let (_, in_use, all) = self.names.take().expect("just read");
        let scope = {
            let document = document.borrow();
            if document.has_selection() || document.has_face_selection() {
                tr("the selection")
            } else {
                tr("the map")
            }
        };
        let (status, count) = match (self.subject.as_deref(), self.replacement.as_deref()) {
            (Some(from), Some(to)) if from != to => {
                let count = document.borrow().replaceable_face_count(from);
                (
                    fill(
                        "{} across {} wear \u{201c}{}\u{201d}",
                        &[&counted(count, "face"), scope, from],
                    ),
                    count,
                )
            }
            (Some(_), Some(_)) => (
                tr("Pick a different material to replace it with").to_string(),
                0,
            ),
            _ => (fill("Pick a material to replace, across {}", &[scope]), 0),
        };
        let status = self.said.clone().unwrap_or(status);

        let mut open = true;
        let still = dialog::show_modal(
            ctx,
            "replace-material",
            tr("Replace Material"),
            [520.0, 420.0],
            |ui, part| match part {
                Part::Body => {
                    let column = ((ui.available_width() - 8.0) / 2.0).floor();
                    let status_height = 24.0;
                    let height = (ui.available_height() - status_height).max(120.0);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for (label, filter, names, pick) in [
                            (
                                translation_key("Replace"),
                                &mut self.subject_filter,
                                &in_use,
                                &mut self.subject,
                            ),
                            (
                                translation_key("With"),
                                &mut self.with_filter,
                                &all,
                                &mut self.replacement,
                            ),
                        ] {
                            ui.allocate_ui(egui::vec2(column, height), |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(column);
                                    theme::heading(ui, tr(label));
                                    ui.add_space(2.0);
                                    widgets::search(ui, filter, tr("Filter materials"));
                                    ui.add_space(4.0);
                                    let needle = filter.to_lowercase();
                                    let well = (ui.available_height() - 6.0).max(80.0);
                                    widgets::list(ui, well, |ui| {
                                        let shown: Vec<&String> = names
                                            .iter()
                                            .filter(|name| {
                                                needle.is_empty()
                                                    || name.to_lowercase().contains(&needle)
                                            })
                                            .collect();
                                        // Only the rows in view are laid out.
                                        egui::ScrollArea::vertical()
                                            .id_salt(label)
                                            .auto_shrink([false, false])
                                            .max_height(well)
                                            .show_rows(ui, 26.0, shown.len(), |ui, range| {
                                                for name in shown[range].iter().copied() {
                                                    let picked =
                                                        pick.as_deref() == Some(name.as_str());
                                                    let (rect, response) = widgets::list_row(
                                                        ui,
                                                        26.0,
                                                        picked,
                                                        egui::Sense::click(),
                                                    );
                                                    let (galley, cut) = widgets::truncated(
                                                        ui,
                                                        name,
                                                        theme::body(),
                                                        theme::fg(),
                                                        rect.width() - 16.0,
                                                    );
                                                    ui.painter().galley(
                                                        egui::pos2(
                                                            rect.left() + 8.0,
                                                            crate::theme::galley_top(
                                                                ui.ctx(),
                                                                &galley,
                                                                rect.center().y,
                                                            ),
                                                        ),
                                                        galley,
                                                        theme::fg(),
                                                    );
                                                    let response = if cut {
                                                        response.on_hover_text(name)
                                                    } else {
                                                        response
                                                    };
                                                    if response.clicked() {
                                                        *pick = Some(name.clone());
                                                        self.said = None;
                                                    }
                                                }
                                            });
                                    });
                                });
                            });
                        }
                    });
                    ui.add_space(4.0);
                    widgets::status(ui, &status);
                }
                Part::Actions => {
                    if dialog::accent_button(ui, tr("Replace"), count > 0).clicked() {
                        if let (Some(from), Some(to)) =
                            (self.subject.clone(), self.replacement.clone())
                        {
                            let changed = document.borrow_mut().replace_material(&from, &to);
                            self.said = Some(fill("Replaced {}", &[&counted(changed, "face")]));
                            if changed > 0 {
                                // The material is gone from the map, so it is no longer one to
                                // pick on the left.
                                self.subject = None;
                            }
                        }
                    }
                    if dialog::button(ui, tr("Close"), true).clicked() {
                        open = false;
                    }
                }
            },
        );
        self.names = Some((key, in_use, all));
        open && still
    }
}

/// A short list of named things, each with a button that stops using it; the full text
/// of each shows on hover. `remove` gets the index of the one removed.
pub fn removable_list(
    ui: &mut egui::Ui,
    id: &str,
    items: &[(String, String)],
    tip: &str,
    mut remove: impl FnMut(usize),
) {
    widgets::list(ui, 0.0, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(id)
            .max_height(140.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (at, (name, full)) in items.iter().enumerate() {
                    let (row, response) =
                        widgets::list_row(ui, widgets::LIST_ROW, false, egui::Sense::hover());
                    let (galley, _) = widgets::truncated(
                        ui,
                        name,
                        theme::body(),
                        theme::fg(),
                        row.width() - 36.0,
                    );
                    ui.painter().galley(
                        egui::pos2(
                            row.left() + 6.0,
                            crate::theme::galley_top(ui.ctx(), &galley, row.center().y),
                        ),
                        galley,
                        theme::fg(),
                    );
                    response.on_hover_text(full);
                    let slot = egui::Rect::from_min_size(
                        egui::pos2(row.right() - 24.0, row.center().y - 11.0),
                        egui::vec2(22.0, 22.0),
                    );
                    let clicked = ui
                        .scope_builder(
                            egui::UiBuilder::new().max_rect(slot).id_salt((id, at)),
                            |ui| {
                                widgets::icon_button(
                                    ui,
                                    22.0,
                                    tip,
                                    true,
                                    false,
                                    widgets::icons::minus,
                                )
                            },
                        )
                        .inner
                        .clicked();
                    if clicked {
                        remove(at);
                    }
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, count: usize, colour: u8) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("heron-material-page-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..count {
            let pixels: Vec<u8> = (0..64 * 64)
                .flat_map(|_| [colour, (i % 256) as u8, 0, 255])
                .collect();
            std::fs::write(
                dir.join(format!("t{i:04}.png")),
                editor_core::assets::encode_png(64, 64, &pixels).unwrap(),
            )
            .unwrap();
        }
        dir
    }

    fn frame(ctx: &egui::Context, page: &mut MaterialPage, document: &Rc<RefCell<Document>>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 900.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| page.show(ui, document));
        output.textures_delta.clear();
    }

    #[test]
    fn clicking_a_material_textures_the_selected_face_and_is_one_undo() {
        exercise_material_click(false);
    }
    #[test]
    fn clicking_a_material_targets_a_face_handle_instead_of_the_whole_brush() {
        exercise_material_click(true);
    }
    fn exercise_material_click(face_tool: bool) {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let directory = folder(
            if face_tool {
                "apply-handle-click"
            } else {
                "apply-click"
            },
            2,
            180,
        );
        let document = Rc::new(RefCell::new(Document::new(Default::default())));
        let face = {
            let mut d = document.borrow_mut();
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
                "stone",
            )
            .unwrap();
            d.add_brushes(vec![brush]);
            d.select_all();
            d.set_face_color(Some(map_model::brush_face::FaceColor::solid(
                map_model::brush_face::Rgb::new(120, 160, 200),
            )));
            let id = d.selectable_brushes()[0].0;
            let face = editor_core::document::FaceRef { node: id, face: 0 };
            if face_tool {
                d.select_only(id);
                let mut handles = editor_core::handle_tool::HandleManager::new();
                handles.collect(
                    editor_core::handle_tool::HandleKind::Face,
                    d.selected_brushes().into_iter(),
                );
                let points = d
                    .brush(id)
                    .unwrap()
                    .geometry()
                    .face_positions(map_model::polyhedron::FaceId(face.face as u32));
                let center = points
                    .iter()
                    .copied()
                    .fold(geometry_math::vec::Vec3d::zero(), |a, b| a + b)
                    / points.len() as f64;
                handles.select(center, false);
                let picked = d.faces_at(&handles.selected_face_centres());
                d.set_face_edit_targets(picked);
            } else {
                d.select_face_only(face);
            }
            d.load_materials(&directory).unwrap();
            face
        };
        let before = document.borrow().to_map_text();
        let original = document.borrow().face_at(face).unwrap().clone();
        let mut page = MaterialPage {
            filter: "t000".into(),
            ..Default::default()
        };
        let draw = |page: &mut MaterialPage, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| page.show(ui, &document),
            );
            output.textures_delta.clear();
            output
        };
        draw(&mut page, vec![]);
        let output = draw(&mut page, vec![]);
        let at = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing material {label}"))
        };
        let click = |page: &mut MaterialPage, pos| {
            draw(
                page,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            draw(
                page,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        };
        click(&mut page, at("t0000"));
        assert_eq!(document.borrow().current_material(), "t0000");
        let painted = document.borrow().face_at(face).unwrap().clone();
        assert_eq!(painted.material_name, "t0000");
        assert!(painted.color.is_none());
        assert_eq!(painted.uv_attributes, original.uv_attributes);
        assert_eq!(painted.uv_coord_system, original.uv_coord_system);
        let untouched = editor_core::document::FaceRef {
            node: face.node,
            face: 1,
        };
        assert_eq!(
            document.borrow().face_at(untouched).unwrap().material_name,
            "stone"
        );
        assert!(document
            .borrow()
            .face_at(untouched)
            .unwrap()
            .color
            .is_some());
        let after = document.borrow().to_map_text();
        assert_ne!(after, before);
        assert!(document
            .borrow()
            .triangles(&Default::default())
            .iter()
            .any(|t| t.material.is_some() && !t.painted));
        click(&mut page, at("t0000"));
        document.borrow_mut().undo().unwrap();
        assert_eq!(document.borrow().to_map_text(), before);
        document.borrow_mut().redo().unwrap();
        assert_eq!(document.borrow().to_map_text(), after);
        document.borrow_mut().clear_selection();
        let unchanged = document.borrow().to_map_text();
        click(&mut page, at("t0001"));
        assert_eq!(document.borrow().current_material(), "t0001");
        assert_eq!(
            document.borrow().to_map_text(),
            unchanged,
            "without selection only the current material changes"
        );
        settle(&ctx, &mut page, &document);
        let _ = std::fs::remove_dir_all(directory);
    }

    /// Frames until the workers have made every thumbnail asked for.
    fn settle(ctx: &egui::Context, page: &mut MaterialPage, document: &Rc<RefCell<Document>>) {
        let start = web_time::Instant::now();
        frame(ctx, page, document);
        while page.thumbnails.waiting() > 0 {
            assert!(start.elapsed().as_secs() < 30, "the thumbnails never came");
            std::thread::sleep(web_time::Duration::from_millis(5));
            frame(ctx, page, document);
        }
        frame(ctx, page, document);
    }

    /// Only the cells in view get thumbnails (made on the workers from the lazily read
    /// files), and a reloaded collection gets new ones rather than showing the old
    /// pictures.
    #[test]
    fn thumbnails_are_made_for_what_is_in_view_and_again_after_a_reload() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let dir = folder("lazy", 300, 10);
        let document = Rc::new(RefCell::new(Document::new(Default::default())));
        document.borrow_mut().load_materials(&dir).unwrap();
        let mut page = MaterialPage::default();
        frame(&ctx, &mut page, &document);
        assert_eq!(
            page.thumbnails.made, 0,
            "nothing is decoded on the UI thread"
        );
        settle(&ctx, &mut page, &document);
        let made = page.thumbnails.made;
        assert!(made > 0, "the cells in view have thumbnails");
        assert!(
            made < 60,
            "only the cells in view have thumbnails, not all 300 ({made})"
        );
        assert_eq!(page.thumbnails.len(), made);
        let before = page
            .thumbnails
            .textures
            .get("t0000")
            .map(|(t, _)| t.id())
            .expect("the first cell is in view");

        // The same folder with new pixels, loaded again.
        folder("lazy", 300, 200);
        document.borrow_mut().load_materials(&dir).unwrap();
        settle(&ctx, &mut page, &document);
        let after = page
            .thumbnails
            .textures
            .get("t0000")
            .map(|(t, _)| t.id())
            .expect("made again");
        assert_ne!(before, after, "a reloaded material gets a new thumbnail");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A big image shrinks to the thumbnail size, keeping its shape and its colours.
    #[test]
    fn a_thumbnail_is_small_and_keeps_the_picture() {
        let (w, h) = (1024u32, 512u32);
        let pixels: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                if (i % w) < w / 2 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                }
            })
            .collect();
        let material = Material::rgba("big".into(), "big.png".into(), w, h, pixels);
        let image = thumbnail_image(&material).unwrap();
        assert_eq!(
            image.size,
            [THUMBNAIL_PIXELS as usize, THUMBNAIL_PIXELS as usize / 2]
        );
        assert_eq!(image.pixels[0], egui::Color32::from_rgb(255, 0, 0));
        assert_eq!(
            image.pixels[image.size[0] - 1],
            egui::Color32::from_rgb(0, 0, 255)
        );
        let small = Material::rgba("small".into(), "small.png".into(), 2, 2, vec![9; 16]);
        assert_eq!(thumbnail_image(&small).unwrap().size, [2, 2]);
    }
}
