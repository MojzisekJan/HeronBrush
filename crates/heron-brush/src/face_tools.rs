//! The Face tab's colour, sweep and fillet pages. Edits go through the core's document
//! methods, so each is one undoable step.
use crate::widgets::{self, icons};
use crate::{colour_picker, editor::Editor, theme};
use editor_core::i18n::{counted, fill, tr, translation_key};
use eframe::egui::{self, Color32, CornerRadius, Id, Rect, Sense, Stroke};
use map_model::brush_face::{
    ColorStop, FaceColor, GradientEnds, GradientStops, Rgb, SurfaceMaterial,
};

/// The palette: colours told apart at a glance, for marking out a level being planned.
const PALETTE: [(&str, Rgb); 12] = [
    (translation_key("White"), Rgb::new(236, 236, 236)),
    (translation_key("Light grey"), Rgb::new(170, 170, 170)),
    (translation_key("Dark grey"), Rgb::new(90, 90, 90)),
    (translation_key("Black"), Rgb::new(24, 24, 24)),
    (translation_key("Red"), Rgb::new(212, 52, 52)),
    (translation_key("Orange"), Rgb::new(226, 124, 40)),
    (translation_key("Yellow"), Rgb::new(226, 196, 52)),
    (translation_key("Green"), Rgb::new(84, 176, 84)),
    (translation_key("Teal"), Rgb::new(52, 168, 168)),
    (translation_key("Blue"), Rgb::new(56, 116, 208)),
    (translation_key("Purple"), Rgb::new(136, 88, 192)),
    (translation_key("Pink"), Rgb::new(216, 112, 168)),
];

/// How many swatches a row of the palette holds.
const SWATCHES_PER_ROW: usize = 6;

/// The gradient presets: a name, its start and its end.
const PRESETS: [(&str, Rgb, Rgb); 3] = [
    (
        translation_key("Warm gradient"),
        Rgb::new(246, 191, 77),
        Rgb::new(174, 48, 73),
    ),
    (
        translation_key("Cool gradient"),
        Rgb::new(74, 211, 195),
        Rgb::new(61, 75, 164),
    ),
    (
        translation_key("Greyscale gradient"),
        Rgb::new(24, 24, 24),
        Rgb::new(236, 236, 236),
    ),
];

/// The preview's height, and the inset of the stop rail from its sides.
const PREVIEW_HEIGHT: f32 = 72.0;
const RAIL_INSET: f32 = 8.0;
/// How near a press must land to a stop marker to take it, in points.
const HIT: f32 = 10.0;

/// The gradient being prepared. The first two stops are the two ends (the
/// "from" and "to" rows), and any further stops follow them; positions run 0 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientDraft {
    pub stops: Vec<ColorStop>,
    pub radial: bool,
    pub angle: f32,
    /// Where the gradient was placed on the face with its handles in the 3D view; `None`
    /// fits it to the face along the angle.
    pub ends: Option<GradientEnds>,
}

impl Default for GradientDraft {
    fn default() -> Self {
        Self {
            stops: vec![
                ColorStop {
                    position: 0.0,
                    color: Rgb::new(24, 24, 24),
                },
                ColorStop {
                    position: 1.0,
                    color: Rgb::new(236, 236, 236),
                },
            ],
            radial: false,
            angle: 0.0,
            ends: None,
        }
    }
}

impl GradientDraft {
    /// The most stops a gradient holds: two ends and six between.
    pub const MAX_STOPS: usize = 8;

    /// The stops, sorted, as the document stores them.
    pub fn sorted(&self) -> GradientStops {
        let stops: Vec<ColorStop> = self
            .stops
            .iter()
            .map(|s| ColorStop {
                position: s.position.clamp(0.0, 1.0),
                ..*s
            })
            .collect();
        GradientStops::new(&stops).expect("two to eight stops")
    }

    /// The paint the draft amounts to.
    pub fn paint(&self, alpha: u8) -> FaceColor {
        FaceColor::MultiGradient {
            stops: self.sorted(),
            radial: self.radial,
            angle: self.angle,
            alpha,
            ends: self.ends,
        }
    }

    /// Takes on a face's gradient: its first and last stops are the ends.
    pub fn load(&mut self, color: FaceColor) {
        match color {
            FaceColor::Gradient {
                from, to, angle, ..
            } => {
                *self = Self {
                    stops: vec![
                        ColorStop {
                            position: 0.0,
                            color: from,
                        },
                        ColorStop {
                            position: 1.0,
                            color: to,
                        },
                    ],
                    radial: false,
                    angle: angle.rem_euclid(360.0),
                    ends: None,
                };
            }
            FaceColor::MultiGradient {
                stops,
                radial,
                angle,
                ends,
                ..
            } => {
                let stops = stops.as_slice();
                let mut ordered = vec![stops[0], *stops.last().unwrap()];
                ordered.extend_from_slice(&stops[1..stops.len() - 1]);
                *self = Self {
                    stops: ordered,
                    radial,
                    angle: angle.rem_euclid(360.0),
                    ends,
                };
            }
            FaceColor::Solid { .. } => {}
        }
    }

    pub fn can_add(&self) -> bool {
        self.stops.len() < Self::MAX_STOPS
    }

    /// Adds a stop at `t`, coloured by what the gradient shows there. Returns its index.
    pub fn add_at(&mut self, t: f32) -> Option<usize> {
        if !self.can_add() {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        let color = Rgb::from_f32(self.sorted().at(t));
        self.stops.push(ColorStop { position: t, color });
        Some(self.stops.len() - 1)
    }

    /// Adds a stop in the middle of the widest gap, as "Add colour stop" does.
    pub fn add_in_widest_gap(&mut self) -> Option<usize> {
        let sorted = self.sorted();
        let pair = sorted.as_slice().windows(2).max_by(|a, b| {
            (a[1].position - a[0].position).total_cmp(&(b[1].position - b[0].position))
        })?;
        let t = (pair[0].position + pair[1].position) / 2.0;
        self.add_at(t)
    }

    /// Removes a stop. A gradient keeps two: removing an end moves the last added stop
    /// into its place, and with only the two ends nothing is removed.
    pub fn remove(&mut self, index: usize) -> bool {
        if self.stops.len() <= 2 || index >= self.stops.len() {
            return false;
        }
        if index < 2 {
            let last = self.stops.pop().unwrap();
            self.stops[index] = last;
        } else {
            self.stops.remove(index);
        }
        true
    }

    /// The stop nearest `t` if it is within [`HIT`] points on a rail `span` points long.
    pub fn hit(&self, t: f32, span: f32) -> Option<usize> {
        self.stops
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (a.position - t).abs().total_cmp(&(b.position - t).abs()))
            .filter(|(_, s)| (s.position - t).abs() * span < HIT)
            .map(|(i, _)| i)
    }

    /// Moves a stop to `t`, kept inside the rail.
    pub fn move_to(&mut self, index: usize, t: f32) {
        if let Some(stop) = self.stops.get_mut(index) {
            stop.position = t.clamp(0.0, 1.0);
        }
    }

    /// Swaps the two ends' colours and mirrors every position.
    pub fn swap(&mut self) {
        let (from, to) = (self.stops[0], self.stops[1]);
        self.stops[0] = ColorStop {
            position: 1.0 - to.position,
            color: to.color,
        };
        self.stops[1] = ColorStop {
            position: 1.0 - from.position,
            color: from.color,
        };
        for stop in &mut self.stops[2..] {
            stop.position = 1.0 - stop.position;
        }
    }

    /// A two-colour preset from end to end.
    pub fn preset(&mut self, start: Rgb, end: Rgb) {
        self.stops = vec![
            ColorStop {
                position: 0.0,
                color: start,
            },
            ColorStop {
                position: 1.0,
                color: end,
            },
        ];
    }
}

/// Where along the rail of `rect` the point `x` lies, 0 to 1, and the rail's length.
fn rail(rect: Rect, x: f32) -> (f32, f32) {
    let span = (rect.width() - 2.0 * RAIL_INSET).max(1.0);
    (
        ((x - rect.left() - RAIL_INSET) / span).clamp(0.0, 1.0),
        span,
    )
}

fn to_color32(c: [f32; 3], alpha: f32) -> Color32 {
    let [r, g, b] = Rgb::from_f32(c).to_f32().map(|v| (v * 255.0).round() as u8);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn rgb_array(c: Rgb) -> [u8; 3] {
    [c.r, c.g, c.b]
}

/// "1 face" or "5 faces".
fn counted_faces(faces: usize) -> String {
    counted(faces, "face")
}

/// A colour's name with its opacity when it is not solid: "blue" or "blue at 60%".
fn described(name: String, colour: Option<FaceColor>) -> String {
    match colour.map(|c| c.alpha()) {
        Some(alpha) if alpha != 255 => fill(
            "{} at {}%",
            &[
                &name,
                &((f32::from(alpha) / 255.0 * 100.0).round() as i32).to_string(),
            ],
        ),
        _ => name,
    }
}

/// What to call a colour: the palette's name for it, or its numbers.
fn name_of(rgb: Rgb) -> String {
    PALETTE
        .iter()
        .find(|(_, candidate)| *candidate == rgb)
        .map(|(name, _)| tr(name).to_lowercase())
        .unwrap_or_else(|| format!("{} {} {}", rgb.r, rgb.g, rgb.b))
}

/// The Colour page's status line.
pub fn colour_status(faces: usize, current: Option<FaceColor>) -> String {
    match (faces, current) {
        (0, _) => tr(
            "Click a brush to paint all of it, Shift+click a face to paint that \
                     one, or pick faces with the face tool",
        )
        .to_string(),
        (faces, None) => fill("{} to paint", &[&counted_faces(faces)]),
        (faces, Some(FaceColor::Solid { color, .. })) => fill(
            "{}, the first one is {}",
            &[&counted_faces(faces), &described(name_of(color), current)],
        ),
        (faces, Some(FaceColor::Gradient { .. } | FaceColor::MultiGradient { .. })) => fill(
            "{}, the first one wears {}",
            &[
                &counted_faces(faces),
                &described(tr("a gradient").to_string(), current),
            ],
        ),
    }
}

/// A dim, wrapping status line.
fn status(ui: &mut egui::Ui, text: &str) {
    widgets::status(ui, text);
}

/// A palette swatch: a button filled with its colour and a hairline round it.
fn swatch(ui: &mut egui::Ui, rgb: Rgb, name: &'static str, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::click());
    let response = response.on_hover_text(tr(name));
    let painter = ui.painter();
    let radius = CornerRadius::same(3);
    painter.rect_filled(rect, radius, Color32::from_rgb(rgb.r, rgb.g, rgb.b));
    let stroke = if response.hovered() && ui.is_enabled() {
        theme::fg().gamma_multiply(0.8)
    } else {
        Color32::from_gray(128).gamma_multiply(0.5)
    };
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    if !ui.is_enabled() {
        painter.rect_filled(rect, radius, theme::bg().gamma_multiply(0.6));
    }
    response
}

/// The gap between swatches.
const SWATCH_GAP: f32 = 4.0;

/// How wide each of `count` swatches is to fill `width` with gaps between.
fn swatch_width(width: f32, count: usize) -> f32 {
    ((width - SWATCH_GAP * (count as f32 - 1.0)) / count as f32).floor()
}

/// The Colour page.
pub struct ColourPage {
    last_color: Option<FaceColor>,
    /// The Mix colour.
    color: [u8; 3],
    /// The opacity the next paint goes on with, in percent.
    opacity: f64,
    gradient: GradientDraft,
    /// The stop being dragged along the rail: its index, its position when the drag
    /// started and where the pointer was.
    dragging: Option<(usize, f32, f32)>,
    /// The stop the rail's context menu is for.
    menu_stop: Option<(usize, egui::Pos2)>,
    color_session: editor_core::document::LivePaint,
    gradient_session: editor_core::document::LivePaint,
    surface_session: editor_core::document::LivePaint,
    /// A test's stand-in for the user typing an opacity, applied as the spin button would.
    #[cfg(test)]
    pub(crate) typed_opacity: Option<f64>,
}

impl Default for ColourPage {
    fn default() -> Self {
        Self {
            last_color: None,
            color: [212, 52, 52],
            opacity: 100.0,
            gradient: GradientDraft::default(),
            dragging: None,
            menu_stop: None,
            color_session: Default::default(),
            gradient_session: Default::default(),
            surface_session: Default::default(),
            #[cfg(test)]
            typed_opacity: None,
        }
    }
}

/// What a frame of the Colour page asks of the document.
enum Paint {
    /// Paint once, as its own undoable edit: a swatch, Paint, or Back to material.
    Once(Option<FaceColor>),
    /// The Mix colour, live while its picker is dragged.
    Mix,
    /// The gradient, live while its controls change.
    Gradient,
}

impl ColourPage {
    fn alpha(&self) -> u8 {
        (self.opacity / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8
    }

    pub fn show(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let picked = editor
            .document
            .borrow()
            .faces_at(&editor.handles.borrow().selected_face_centres());
        let (faces, current, surface) = {
            let d = editor.document.borrow();
            let faces = if picked.is_empty() {
                d.paintable_face_count()
            } else {
                picked.len()
            };
            let face = picked.first().and_then(|f| {
                d.brush(f.node)
                    .and_then(|b| b.source_face_index(f.face).map(|i| b.faces()[i].clone()))
            });
            let current = match &face {
                Some(face) => face.paint(),
                None => d.first_selected_face_color(),
            };
            let surface = match &face {
                Some(face) => face.surface_material,
                None => d.first_selected_surface_material(),
            }
            .unwrap_or_default();
            (faces, current, surface)
        };
        // Follow the painted face, so the controls show what is there before they are
        // used to change it; a frame that changes nothing must not erase settings being
        // prepared.
        if current != self.last_color {
            self.last_color = current;
            if let Some(colour) = current {
                self.opacity = f64::from(colour.alpha()) / 255.0 * 100.0;
                self.gradient.load(colour);
            }
        }
        status(ui, &colour_status(faces, current));

        let opacity_before = self.opacity;
        #[cfg(test)]
        if let Some(typed) = self.typed_opacity.take() {
            self.opacity = typed;
        }
        let gradient_before = self.gradient.clone();
        let mut paint = None;
        ui.add_enabled_ui(faces > 0, |ui| {
            // How opaque the paint goes on; it applies to whichever button paints next.
            widgets::row(
                ui,
                tr("Opacity"),
                Some(tr("How opaque the paint is, as a percentage")),
                |ui| {
                    widgets::spin(ui, &mut self.opacity, 0.0..=100.0, 5.0, 0)
                        .on_hover_text(tr("How opaque the paint is, as a percentage"));
                },
            );

            // The palette. A click paints; there is no separate apply.
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(SWATCH_GAP, SWATCH_GAP);
                let width = swatch_width(ui.available_width(), SWATCHES_PER_ROW);
                for chunk in PALETTE.chunks(SWATCHES_PER_ROW) {
                    ui.horizontal(|ui| {
                        for (name, rgb) in chunk {
                            if swatch(ui, *rgb, name, width).clicked() {
                                paint = Some(Paint::Once(Some(
                                    FaceColor::solid(*rgb).with_alpha(self.alpha()),
                                )));
                            }
                        }
                    });
                }
            });

            let mut alpha = (self.opacity / 100.0) as f32;
            // Anything the palette does not have.
            widgets::row(ui, tr("Mix"), None, |ui| {
                let width = ui.available_width() - 36.0 - ui.spacing().item_spacing.x;
                if theme::button(ui, tr("Paint"), width, ui.is_enabled()).clicked() {
                    let [r, g, b] = self.color;
                    paint = Some(Paint::Once(Some(
                        FaceColor::solid(Rgb::new(r, g, b)).with_alpha(self.alpha()),
                    )));
                }
                if colour_picker::button(
                    ui,
                    Id::new("colour-mix"),
                    &mut self.color,
                    Some(&mut alpha),
                )
                .changed()
                {
                    paint = Some(Paint::Mix);
                }
            });

            widgets::row(ui, tr("Gradient"), None, |ui| {
                let mut kind = usize::from(self.gradient.radial);
                let width =
                    ui.available_width() - widgets::ROW_HEIGHT - ui.spacing().item_spacing.x;
                if widgets::dropdown(
                    ui,
                    "gradient-kind",
                    &mut kind,
                    &[tr("Linear gradient"), tr("Radial gradient")],
                    width,
                ) {
                    self.gradient.radial = kind == 1;
                }
                let enabled = ui.is_enabled();
                if widgets::icon_button(
                    ui,
                    widgets::ROW_HEIGHT,
                    tr("Swap gradient colours"),
                    enabled,
                    false,
                    icons::swap,
                )
                .clicked()
                {
                    self.gradient.swap();
                }
            });
            for index in 0..self.gradient.stops.len() {
                self.stop_row(ui, index, &mut alpha);
            }
            let width = ui.available_width();
            if theme::button(
                ui,
                tr("Add colour stop"),
                width,
                ui.is_enabled() && self.gradient.can_add(),
            )
            .clicked()
            {
                self.gradient.add_in_widest_gap();
            }
            // The angle means nothing to a radial gradient.
            if !self.gradient.radial {
                widgets::row(ui, tr("Gradient angle"), None, |ui| {
                    let enabled = ui.is_enabled();
                    let step = 44.0;
                    let spin = ui.available_width() - 2.0 * (step + ui.spacing().item_spacing.x);
                    if theme::button(ui, "+45°", step, enabled).clicked() {
                        self.gradient.angle = (self.gradient.angle + 45.0).rem_euclid(360.0);
                        self.gradient.ends = None;
                    }
                    let mut angle = f64::from(self.gradient.angle);
                    let response = widgets::spin_width(ui, &mut angle, -0.1..=360.0, 0.1, 1, spin)
                        .on_hover_text(tr("Which way the blend runs, in degrees"));
                    if response.changed() {
                        // Wraps round: 360 is 0 again.
                        self.gradient.angle = (angle as f32).rem_euclid(360.0);
                        if self.gradient.angle >= 359.95 {
                            self.gradient.angle = 0.0;
                        }
                        self.gradient.ends = None;
                    }
                    if theme::button(ui, "−45°", step, enabled).clicked() {
                        self.gradient.angle = (self.gradient.angle - 45.0).rem_euclid(360.0);
                        self.gradient.ends = None;
                    }
                });
            }
            // Ends placed with the handles in the 3D view; this lets the gradient fill the
            // face again.
            if self.gradient.ends.is_some() {
                let width = ui.available_width();
                if theme::button(ui, tr("Fit to face"), width, ui.is_enabled()).clicked() {
                    self.gradient.ends = None;
                }
            }
            self.preview(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = SWATCH_GAP;
                let width = swatch_width(ui.available_width(), PRESETS.len());
                for (name, start, end) in PRESETS {
                    if swatch(ui, start, name, width).clicked() {
                        self.gradient.preset(start, end);
                    }
                }
            });
            if (alpha - (self.opacity / 100.0) as f32).abs() > 1e-6 {
                self.opacity = f64::from(alpha) * 100.0;
            }

            // Taking the colour off puts the face back to the material it never stopped wearing.
            let width = ui.available_width();
            if theme::button(ui, tr("Back to material"), width, ui.is_enabled()).clicked() {
                paint = Some(Paint::Once(None));
            }

            widgets::subheading(ui, tr("Surface material"));
            self.surface(ui, editor, &picked, surface);
        });

        let gradient_changed = self.gradient != gradient_before;
        // The opacity applies to the next paint, except that a gradient follows it live.
        let opacity_changed = self.opacity != opacity_before;
        let wears_gradient = matches!(
            current,
            Some(FaceColor::Gradient { .. } | FaceColor::MultiGradient { .. })
        );
        if paint.is_none() && (gradient_changed || (opacity_changed && wears_gradient)) {
            paint = Some(Paint::Gradient);
        }
        let Some(paint) = paint else { return };
        let mut document = editor.document.borrow_mut();
        let color = match paint {
            Paint::Once(color) => {
                if picked.is_empty() {
                    document.set_face_color(color);
                } else {
                    document.set_face_color_on(&picked, color);
                }
                color
            }
            Paint::Mix => {
                let [r, g, b] = self.color;
                let color = Some(FaceColor::solid(Rgb::new(r, g, b)).with_alpha(self.alpha()));
                document.set_face_color_live(color, &picked, &mut self.color_session);
                color
            }
            Paint::Gradient => {
                let color = Some(self.gradient.paint(self.alpha()));
                document.set_face_color_live(color, &picked, &mut self.gradient_session);
                color
            }
        };
        // What was just painted is what the controls already say, so the next frame need
        // not reload them (which would reorder the stops under a drag).
        self.last_color = color;
    }

    /// One stop's row: its colour, its position and the button that removes it.
    fn stop_row(&mut self, ui: &mut egui::Ui, index: usize, alpha: &mut f32) {
        if index >= self.gradient.stops.len() {
            return;
        }
        let can_remove = self.gradient.stops.len() > 2;
        ui.push_id(("stop", index), |ui| {
            widgets::row(ui, tr("Position (%)"), None, |ui| {
                let stop = &mut self.gradient.stops[index];
                let mut rgb = rgb_array(stop.color);
                if colour_picker::button(ui, stop_picker_id(index), &mut rgb, Some(alpha)).changed()
                {
                    stop.color = Rgb::new(rgb[0], rgb[1], rgb[2]);
                }
                let gap = ui.spacing().item_spacing.x;
                let width = ui.available_width() - widgets::ROW_HEIGHT - gap;
                let mut position = f64::from(stop.position) * 100.0;
                if widgets::spin_width(ui, &mut position, 0.0..=100.0, 1.0, 1, width)
                    .on_hover_text(tr("Position (%)"))
                    .changed()
                {
                    stop.position = (position / 100.0) as f32;
                }
                let enabled = ui.is_enabled() && can_remove;
                if widgets::icon_button(
                    ui,
                    widgets::ROW_HEIGHT,
                    tr("Remove colour stop"),
                    enabled,
                    false,
                    icons::minus,
                )
                .clicked()
                {
                    self.gradient.remove(index);
                }
            });
        });
    }

    /// The gradient drawn over a checkerboard, with the rail of stop markers under it.
    /// A click on the rail adds a stop or opens the clicked stop's picker, a right click
    /// offers to remove it, and a drag moves it.
    fn preview(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PREVIEW_HEIGHT),
            Sense::click_and_drag(),
        );
        let enabled = ui.is_enabled();
        let alpha = (self.opacity / 100.0) as f32;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            colour_picker::checkerboard(&painter, rect, 8.0);
            let stops = self.gradient.sorted();
            let (w, h) = (rect.width(), rect.height());
            if self.gradient.radial {
                // Normalised, so the ellipse fills the preview as it fills the face's UV bounds.
                colour_picker::paint_ramp(&painter, rect, 48, 16, |u, v| {
                    let t = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() / 0.5;
                    to_color32(stops.at(t), alpha)
                });
            } else {
                let radians = self.gradient.angle.to_radians();
                let (dx, dy) = (radians.cos(), -radians.sin());
                let span = ((w * dx.abs() + h * dy.abs()) / 2.0).max(1e-3);
                colour_picker::paint_ramp(&painter, rect, 48, 8, |u, v| {
                    let (x, y) = ((u - 0.5) * w, (v - 0.5) * h);
                    let t = ((x * dx + y * dy) / span + 1.0) / 2.0;
                    to_color32(stops.at(t), alpha)
                });
            }
            // The rail always runs from position 0 to 1.
            let bar = Rect::from_min_max(
                egui::pos2(rect.left() + RAIL_INSET, rect.bottom() - 18.0),
                egui::pos2(rect.right() - RAIL_INSET, rect.bottom() - 6.0),
            );
            colour_picker::paint_ramp(&painter, bar, 32, 1, |u, _| to_color32(stops.at(u), 1.0));
            for stop in &self.gradient.stops {
                let x = bar.left() + bar.width() * stop.position;
                let marker = Rect::from_min_size(
                    egui::pos2(x - 5.0, rect.bottom() - 21.0),
                    egui::vec2(10.0, 18.0),
                );
                painter.rect_filled(
                    marker,
                    0.0,
                    Color32::from_rgb(stop.color.r, stop.color.g, stop.color.b),
                );
                painter.rect_stroke(
                    marker,
                    0.0,
                    Stroke::new(3.0, Color32::BLACK),
                    egui::StrokeKind::Middle,
                );
                painter.rect_stroke(
                    marker,
                    0.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Middle,
                );
            }
            if !enabled {
                painter.rect_filled(rect, 0.0, theme::bg().gamma_multiply(0.6));
            }
        }
        if !enabled {
            return;
        }

        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(origin) = ui.input(|i| i.pointer.press_origin()) {
                let (t, span) = rail(rect, origin.x);
                self.dragging = self
                    .gradient
                    .hit(t, span)
                    .map(|i| (i, self.gradient.stops[i].position, origin.x));
            }
        }
        if let (Some((index, start, origin)), Some(pointer)) =
            (self.dragging, response.interact_pointer_pos())
        {
            if response.dragged() {
                let (_, span) = rail(rect, pointer.x);
                self.gradient
                    .move_to(index, start + (pointer.x - origin) / span);
            }
        }
        if response.drag_stopped() {
            self.dragging = None;
        }
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let (t, span) = rail(rect, pointer.x);
                match self.gradient.hit(t, span) {
                    Some(index) => colour_picker::open(ui.ctx(), stop_picker_id(index)),
                    None => {
                        self.gradient.add_at(t);
                    }
                }
            }
        }
        let menu_id = Id::new("colour-stop-menu");
        if response.secondary_clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let (t, span) = rail(rect, pointer.x);
                self.menu_stop = self.gradient.hit(t, span).map(|i| (i, pointer));
                if self.menu_stop.is_some() {
                    egui::Popup::open_id(ui.ctx(), menu_id);
                }
            }
        }
        if let Some((index, at)) = self.menu_stop {
            let shown = egui::Popup::new(
                menu_id,
                ui.ctx().clone(),
                egui::PopupAnchor::Position(at),
                ui.layer_id(),
            )
            .kind(egui::PopupKind::Menu)
            .open_memory(None)
            .show(|ui| {
                if ui.button(tr("Remove colour stop")).clicked() {
                    self.gradient.remove(index);
                    egui::Popup::close_id(ui.ctx(), menu_id);
                }
            });
            if shown.is_none() {
                self.menu_stop = None;
            }
        }
    }

    /// Roughness, metallic and emission, applied live; and the reset, which is an edit of
    /// its own.
    fn surface(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        picked: &[editor_core::document::FaceRef],
        surface: SurfaceMaterial,
    ) {
        let mut roughness = f64::from(surface.roughness);
        let mut metallic = f64::from(surface.metallic);
        let mut emission = f64::from(surface.emission) / 100.0;
        let mut changed = false;
        for (label, value, range, step, decimals, tip) in [
            (
                tr("Roughness"),
                &mut roughness,
                0.0..=100.0,
                5.0,
                0,
                tr("0 = glossy, 100 = matte"),
            ),
            (
                tr("Metallic"),
                &mut metallic,
                0.0..=100.0,
                5.0,
                0,
                tr("0 = non-metal, 100 = metal"),
            ),
            (
                tr("Emission"),
                &mut emission,
                0.0..=8.0,
                0.1,
                1,
                tr("Emits the surface colour; does not light nearby objects"),
            ),
        ] {
            widgets::row(ui, label, Some(tip), |ui| {
                changed |= widgets::spin(ui, value, range, step, decimals)
                    .on_hover_text(tip)
                    .changed();
            });
        }
        let width = ui.available_width();
        let reset = theme::button(ui, tr("Use default surface"), width, ui.is_enabled()).clicked();
        let mut document = editor.document.borrow_mut();
        if changed {
            let value = SurfaceMaterial {
                roughness: roughness.round() as u8,
                metallic: metallic.round() as u8,
                emission: (emission * 100.0).round() as u16,
            };
            document.set_surface_material_live(Some(value), picked, &mut self.surface_session);
        } else if reset {
            if picked.is_empty() {
                document.set_surface_material(None);
            } else {
                document.set_surface_material_on(picked, None);
            }
        }
    }
}

/// The picker of the stop at `index`, which the rail opens when its marker is clicked.
fn stop_picker_id(index: usize) -> Id {
    Id::new(("colour-stop", index))
}

/// The Sweep page.
pub struct SweepPage {
    /// The turn's axis, which the rotation cannot say while the angle is zero.
    axis: usize,
    /// The preview line and what it was worked out for.
    preview: Option<(
        editor_core::sweep_tool::SweepSource,
        editor_core::sweep_tool::SweepInputs,
        String,
    )>,
}

impl Default for SweepPage {
    fn default() -> Self {
        Self {
            axis: 2,
            preview: None,
        }
    }
}

/// The path modes and alignments, in the order the choosers list them.
const PATH_MODES: [(editor_core::sweep_tool::SweepPathMode, &str); 3] = [
    (
        editor_core::sweep_tool::SweepPathMode::Arc,
        translation_key("Arc"),
    ),
    (
        editor_core::sweep_tool::SweepPathMode::Straight,
        translation_key("Straight"),
    ),
    (
        editor_core::sweep_tool::SweepPathMode::SBend,
        translation_key("S-bend"),
    ),
];
const ALIGNMENTS: [(editor_core::sweep_tool::SweepAlignment, &str); 2] = [
    (
        editor_core::sweep_tool::SweepAlignment::Integer,
        translation_key("Integer"),
    ),
    (
        editor_core::sweep_tool::SweepAlignment::Free,
        translation_key("Free"),
    ),
];

/// What the sweep would make, without making it.
pub fn sweep_preview(
    source: &editor_core::sweep_tool::SweepSource,
    inputs: &editor_core::sweep_tool::SweepInputs,
    material: &str,
) -> String {
    use editor_core::sweep_tool;
    if inputs.transform.is_no_op() {
        return tr("The sweep goes nowhere").to_string();
    }
    let brushes =
        match sweep_tool::generate_checked(source, &inputs.transform, &inputs.parameters, material)
        {
            Ok(brushes) => brushes,
            Err(error) => return error,
        };
    match sweep_tool::bounds_of(&brushes) {
        Some(bounds) => {
            let size = bounds.size();
            format!(
                "{} brushes, {:.0} × {:.0} × {:.0}",
                brushes.len(),
                size.x(),
                size.y(),
                size.z()
            )
        }
        None => tr("Nothing to make: every segment is flat").to_string(),
    }
}

/// A vector's three components side by side in one row, marked X, Y and Z.
fn vector_row(
    ui: &mut egui::Ui,
    title: &str,
    values: &mut geometry_math::vec::Vec3d,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
) {
    widgets::row(ui, title, None, |ui| {
        let mut v = [values[0], values[1], values[2]];
        let responses =
            widgets::axis_spins(ui, title, &mut v, &["X", "Y", "Z"], range, step, decimals);
        if responses.iter().any(|r| r.changed()) {
            for (axis, value) in v.into_iter().enumerate() {
                values[axis] = value;
            }
        }
    });
}

impl SweepPage {
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let source = editor.document.borrow().sweep_source();
        match &source {
            None => status(ui, tr("Select faces to sweep (Shift-click)")),
            Some(source) => {
                let n = source.faces.len();
                status(
                    ui,
                    &format!("{} face{} selected", n, if n == 1 { "" } else { "s" }),
                );
            }
        }
        let mut inputs = editor.sweep.borrow_mut();
        // Where the face ends up.
        vector_row(
            ui,
            tr("Travel"),
            &mut inputs.transform.translation,
            -8192.0..=8192.0,
            8.0,
            0,
        );

        // How far it turns on the way, about which axis.
        let rotation = inputs.transform.rotation;
        let mut angle = rotation.angle().to_degrees();
        let direction = rotation.axis();
        if angle.abs() > 1e-6 {
            self.axis = (0..3)
                .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
                .unwrap_or(2);
            if direction[self.axis] < 0.0 {
                angle = -angle;
            }
        }
        let old = (angle, self.axis);
        widgets::row(ui, tr("Turn"), None, |ui| {
            widgets::dropdown(ui, "sweep-axis", &mut self.axis, &["X", "Y", "Z"], 64.0);
            let width = ui.available_width();
            widgets::spin_width(ui, &mut angle, -360.0..=360.0, 15.0, 0, width);
        });
        if old != (angle, self.axis) {
            let mut direction = geometry_math::vec::Vec3d::zero();
            direction[self.axis] = 1.0;
            inputs.transform.rotation =
                geometry_math::quat::Quatd::from_axis_angle(direction, angle.to_radians());
        }

        // How much larger or smaller it becomes.
        vector_row(
            ui,
            tr("Scale"),
            &mut inputs.transform.scale,
            0.01..=64.0,
            0.1,
            2,
        );

        // How finely it is cut up, and how often it repeats.
        widgets::row(ui, tr("Steps"), None, |ui| {
            let half = halves(ui);
            spin_usize(ui, &mut inputs.parameters.iterations, 64, half)
                .on_hover_text(tr("Iterations"));
            spin_usize(ui, &mut inputs.parameters.segments, 256, half);
        });
        widgets::row(ui, tr("Path"), None, |ui| {
            let mut alignment = ALIGNMENTS
                .iter()
                .position(|(m, _)| *m == inputs.parameters.alignment)
                .unwrap_or(0);
            let names: Vec<&str> = ALIGNMENTS.iter().map(|(_, n)| tr(n)).collect();
            let half = halves(ui);
            if widgets::dropdown(ui, "sweep-alignment", &mut alignment, &names, half) {
                inputs.parameters.alignment = ALIGNMENTS[alignment].0;
            }
            let mut path = PATH_MODES
                .iter()
                .position(|(m, _)| *m == inputs.parameters.path_mode)
                .unwrap_or(0);
            let names: Vec<&str> = PATH_MODES.iter().map(|(_, n)| tr(n)).collect();
            if widgets::dropdown(ui, "sweep-path", &mut path, &names, half) {
                inputs.parameters.path_mode = PATH_MODES[path].0;
            }
        });

        // What it would make, worked out again only when the source or settings change.
        if let Some(source) = &source {
            let stale = self
                .preview
                .as_ref()
                .is_none_or(|(s, i, _)| s != source || *i != *inputs);
            if stale {
                let text =
                    sweep_preview(source, &inputs, editor.document.borrow().current_material());
                self.preview = Some((source.clone(), *inputs, text));
            }
            if let Some((_, _, text)) = &self.preview {
                status(ui, text);
            }
        }

        let width = ui.available_width();
        if theme::button(ui, tr("Sweep"), width, source.is_some()).clicked() {
            if inputs.transform.is_no_op() {
                editor_core::say!("the sweep goes nowhere, so there is nothing to make");
            } else {
                let result = editor
                    .document
                    .borrow_mut()
                    .sweep_selection_checked(&inputs.transform, &inputs.parameters);
                match result {
                    Ok(made) => editor_core::say!("swept {made} brushes"),
                    Err(error) => editor_core::say!("{error}"),
                }
                self.preview = None;
            }
        }
    }
}

/// A spin button over a count from 1 to `max`, `width` wide.
fn spin_usize(ui: &mut egui::Ui, value: &mut usize, max: u32, width: f32) -> egui::Response {
    let mut v = f64::from(u32::try_from(*value).unwrap_or(max));
    let response = widgets::spin_width(ui, &mut v, 1.0..=f64::from(max), 1.0, 0, width);
    if response.changed() {
        *value = v.round() as usize;
    }
    response
}

/// Half the width a row has left, less the gap between two controls.
fn halves(ui: &egui::Ui) -> f32 {
    ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).floor()
}

/// What the fillet panel would round if its button were pressed now.
#[derive(Debug, Clone, PartialEq)]
pub enum FilletWork {
    Nothing,
    /// The edges picked, or the ones between picked vertices.
    Edges(Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)>),
    /// The loop around each of this many selected faces.
    FaceLoops(usize),
}

impl FilletWork {
    pub fn of(editor: &Editor) -> Self {
        let document = editor.document.borrow();
        let handles = editor.handles.borrow();
        let edges = handles.selected_edges();
        if !edges.is_empty() {
            return Self::Edges(edges);
        }
        let between = editor_core::fillet_tool::edges_between_picked_vertices(&document, &handles);
        if !between.is_empty() {
            return Self::Edges(between);
        }
        match document.face_selection_count() {
            0 => Self::Nothing,
            faces => Self::FaceLoops(faces),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Nothing => "Click an edge with the fillet tool (O), pick vertices or edges, or \
                 select faces to round their loops"
                .to_string(),
            Self::Edges(edges) => format!(
                "{} selected",
                text_plural(edges.len(), "1 edge", &format!("{} edges", edges.len()))
            ),
            Self::FaceLoops(faces) => {
                format!(
                    "The loop around {}",
                    text_plural(
                        *faces,
                        "1 selected face",
                        &format!("{faces} selected faces")
                    )
                )
            }
        }
    }
}

/// "1 brush" or "N brushes".
fn text_plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        one.to_string()
    } else {
        many.to_string()
    }
}

/// The Fillet panel.
#[derive(Default)]
pub struct FilletPanel {
    /// What the last press did, and what was picked when it did it; shown until that
    /// changes.
    result: Option<(String, FilletWork)>,
}

impl FilletPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let work = FilletWork::of(editor);
        if self.result.as_ref().is_some_and(|(_, then)| *then != work) {
            self.result = None;
        }
        match &self.result {
            Some((text, _)) => status(ui, text),
            None => status(ui, &work.describe()),
        }
        let mut settings = editor.fillet.borrow_mut();
        widgets::row(
            ui,
            tr("Radius"),
            Some(tr("How far back from the edge the rounding starts")),
            |ui| {
                widgets::number_buttons(ui, &mut settings.radius, 0.01..=1024.0);
            },
        );
        widgets::row(
            ui,
            tr("Faces"),
            Some(tr(
                "How many flat faces the rounding is made of; one is a chamfer",
            )),
            |ui| {
                widgets::number_buttons(ui, &mut settings.faces, 1.0..=64.0);
            },
        );
        let tip = crate::tool_rows::corners_tip();
        widgets::row(ui, tr("Corners"), Some(&tip), |ui| {
            let corners = &crate::tool_rows::CORNERS;
            let mut index = corners
                .iter()
                .position(|(c, _)| *c == settings.corners)
                .unwrap_or(0);
            let names: Vec<&str> = corners.iter().map(|(_, name)| tr(name)).collect();
            if widgets::dropdown(
                ui,
                "fillet-panel-corners",
                &mut index,
                &names,
                ui.available_width().clamp(96.0, 220.0),
            ) {
                settings.corners = corners[index].0;
            }
        });
        if crate::widgets::checkbox(ui, &mut settings.inner, tr("Inner L corner")).changed() {
            self.result = None;
        }
        let parameters = *settings;
        drop(settings);
        let inner = parameters.inner;
        let width = ui.available_width();
        if theme::button(
            ui,
            tr("Round edges"),
            width,
            inner || work != FilletWork::Nothing,
        )
        .clicked()
        {
            let outcome = editor_core::fillet_tool::apply(
                &editor.document,
                &mut editor.handles.borrow_mut(),
                parameters,
            );
            let text = match outcome {
                editor_core::fillet_tool::Outcome::Rounded(count) => {
                    format!("Rounded {count} brushes")
                }
                editor_core::fillet_tool::Outcome::Refused(error) => error,
                _ => tr("Nothing was rounded").to_owned(),
            };
            self.result = Some((text, FilletWork::of(editor)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(position: f32, v: u8) -> ColorStop {
        ColorStop {
            position,
            color: Rgb::new(v, v, v),
        }
    }

    #[test]
    fn rail_adds_stops_sampled_from_the_gradient_up_to_eight() {
        let mut g = GradientDraft {
            stops: vec![stop(0.0, 0), stop(1.0, 200)],
            ..Default::default()
        };
        assert_eq!(g.add_at(0.25), Some(2));
        assert_eq!(g.stops[2], stop(0.25, 50));
        for _ in 0..5 {
            assert!(g.add_at(0.5).is_some());
        }
        assert_eq!(g.stops.len(), 8);
        assert_eq!(g.add_at(0.75), None, "eight is the most");
        assert_eq!(g.add_in_widest_gap(), None);
    }

    #[test]
    fn removing_keeps_two_stops_and_an_end_takes_the_last_added() {
        let mut g = GradientDraft {
            stops: vec![stop(0.0, 0), stop(1.0, 200)],
            ..Default::default()
        };
        assert!(!g.remove(0), "the two ends stay");
        g.add_at(0.25);
        g.add_at(0.75);
        assert!(g.remove(0));
        assert_eq!(
            g.stops,
            vec![stop(0.75, 150), stop(1.0, 200), stop(0.25, 50)]
        );
        assert!(g.remove(2));
        assert_eq!(g.stops.len(), 2);
    }

    #[test]
    fn markers_are_hit_within_ten_points_and_drag_inside_the_rail() {
        let mut g = GradientDraft {
            stops: vec![stop(0.0, 0), stop(1.0, 200), stop(0.5, 100)],
            ..Default::default()
        };
        // A 400-point rail: 10 points is 0.025.
        assert_eq!(g.hit(0.52, 400.0), Some(2));
        assert_eq!(g.hit(0.98, 400.0), Some(1));
        assert_eq!(g.hit(0.3, 400.0), None);
        g.move_to(2, 0.8);
        assert_eq!(g.stops[2].position, 0.8);
        g.move_to(2, 1.7);
        assert_eq!(g.stops[2].position, 1.0);
        // The paint is sorted whatever order the rows are in.
        let FaceColor::MultiGradient { stops, .. } = g.paint(255) else {
            panic!()
        };
        assert!(stops
            .as_slice()
            .windows(2)
            .all(|p| p[0].position <= p[1].position));
    }

    #[test]
    fn swap_mirrors_and_presets_reset() {
        let mut g = GradientDraft {
            stops: vec![stop(0.1, 0), stop(0.9, 200), stop(0.3, 100)],
            ..Default::default()
        };
        g.swap();
        assert_eq!(g.stops[0].color, Rgb::new(200, 200, 200));
        assert!(
            (g.stops[0].position - 0.1).abs() < 1e-6 && (g.stops[1].position - 0.9).abs() < 1e-6
        );
        assert!((g.stops[2].position - 0.7).abs() < 1e-6);
        g.preset(Rgb::new(1, 2, 3), Rgb::new(4, 5, 6));
        assert_eq!(g.stops.len(), 2);
    }

    #[test]
    fn loading_a_face_gradient_puts_its_ends_first() {
        let mut g = GradientDraft::default();
        let stops = GradientStops::new(&[stop(0.0, 0), stop(0.4, 90), stop(1.0, 200)]).unwrap();
        g.load(FaceColor::MultiGradient {
            stops,
            radial: true,
            angle: 400.0,
            alpha: 255,
            ends: None,
        });
        assert_eq!(g.stops, vec![stop(0.0, 0), stop(1.0, 200), stop(0.4, 90)]);
        assert!(g.radial);
        assert!((g.angle - 40.0).abs() < 1e-4);
    }

    #[test]
    fn status_names_the_first_face() {
        editor_core::preferences::set_current(Default::default());
        assert_eq!(
            colour_status(
                3,
                Some(FaceColor::solid(Rgb::new(212, 52, 52)).with_alpha(153))
            ),
            "3 faces, the first one is red at 60%"
        );
        assert_eq!(colour_status(1, None), "1 face to paint");
        assert!(colour_status(2, Some(GradientDraft::default().paint(255))).contains("a gradient"));
    }
}
