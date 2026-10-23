//! The UV editor's canvas and controls, without a toolkit: how a material sits on a face.
//!
//! [`UvCanvas`] draws the material with the face's outline and the handles, and turns
//! pointer events in canvas pixels into edits. [`Action`] and [`Field`] are the buttons and
//! number fields beside it. The operations themselves are in [`map_model::brush_face`];
//! nothing here computes a mapping, it only decides which operation a control asks for.

use geometry_math::vec::{Vec2f, Vec3d};
use map_model::uv_alignment::{UvAxis, UvFitMode, UvPolicy, UvSign};

use crate::canvas::Canvas;
use crate::document::Document;
use crate::i18n::{tr, translation_key};
use crate::input::Modifiers;

/// How many pixels the canvas is tall.
pub const CANVAS_HEIGHT: i32 = 160;

/// How large the handles are drawn, and how near the pointer has to be to catch one.
pub const HANDLE_RADIUS: f64 = 4.0;

/// How far from the origin the rotation handle sits, in canvas pixels.
///
/// Far enough that a small movement of the pointer is a small change of angle, which is
/// what makes a rotation handle usable at all.
pub const ROTATE_RADIUS: f64 = 44.0;

/// How far from the origin the scale handles sit, in canvas pixels.
pub const SCALE_RADIUS: f64 = 26.0;

/// Navigation is local to the UV view and never edits the document.
#[derive(Clone, Copy, Debug)]
pub struct UvView {
    pub zoom: f64,
    pub pan: (f64, f64),
    face: Option<crate::document::FaceRef>,
    offset: Vec2f,
}
impl Default for UvView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
            face: None,
            offset: Vec2f::zero(),
        }
    }
}
impl UvView {
    pub fn sync(&mut self, document: &Document, width: f64, height: f64) {
        let Some((id, face)) = document.face_to_edit() else {
            self.face = None;
            return;
        };
        if self.face != Some(id) {
            let points = document.face_to_edit_polygon().unwrap_or_default();
            let uvs: Vec<_> = points.iter().map(|p| face.uv_coords(*p)).collect();
            if !uvs.is_empty() {
                let (mut lo, mut hi) = (uvs[0], uvs[0]);
                for uv in &uvs {
                    for k in 0..2 {
                        lo[k] = lo[k].min(uv[k]);
                        hi[k] = hi[k].max(uv[k]);
                    }
                }
                let base = canvas_scale(document, height);
                let scale = ((width - 60.0).max(1.0) / f64::from((hi.x() - lo.x()).max(1.0)))
                    .min((height - 60.0).max(1.0) / f64::from((hi.y() - lo.y()).max(1.0)));
                self.zoom = (scale / base).clamp(0.02, 64.0);
                let center = (lo + hi) * 0.5;
                self.pan = (
                    width / 2.0 - f64::from(center.x()) * base * self.zoom,
                    height / 2.0 - f64::from(center.y()) * base * self.zoom,
                );
            }
            self.face = Some(id);
        } else {
            let delta = self.offset - face.uv_attributes.offset;
            let scale = canvas_scale(document, height * self.zoom);
            self.pan.0 += f64::from(delta.x()) * scale;
            self.pan.1 += f64::from(delta.y()) * scale;
        }
        self.offset = face.uv_attributes.offset;
    }
    pub fn zoom_at(&mut self, x: f64, y: f64, delta: f64) {
        let next = (self.zoom * (-delta * 0.15).exp()).clamp(0.02, 64.0);
        let ratio = next / self.zoom;
        self.pan = (x - (x - self.pan.0) * ratio, y - (y - self.pan.1) * ratio);
        self.zoom = next;
    }
}

/// How near, in canvas pixels, the pointer has to be to a tile edge for an Alt-drag to
/// take it as a shear handle: `UvViewHelper::pickUvGrid`'s `maxDistance`.
pub const SHEAR_GRIP_DISTANCE: f64 = 5.0;

/// Minimum distance from a UV axis for a stable shear drag. Handles too close to an axis would
/// amplify small movements into extreme shear.
pub const SHEAR_MIN_OFFSET: f32 = 6.0;

/// What a drag on the canvas has hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grip {
    /// An Alt-press outside the shear handles performs no drag.
    Nothing,
    /// Nothing in particular, so the drag shifts the material.
    Offset,
    /// A tile edge, held with Alt, so the drag skews the material: `UvShearTool`. A
    /// vertical edge is the X handle, a horizontal one the Y handle, and a corner both.
    Shear {
        horizontally: bool,
        vertically: bool,
    },
    /// The point the other two work about.
    Origin,
    /// The ring, so the drag turns the material.
    Rotate,
    /// One of the four corners, so the drag scales it.
    Scale {
        horizontally: bool,
        vertically: bool,
    },
}

/// The canvas's state between events: the view, the origin handle and the drag in
/// progress.
#[derive(Default)]
pub struct UvCanvas {
    /// Zoom and pan, local to the canvas.
    pub view: UvView,
    /// Where rotation and scaling happen about, in texels: `UvOriginTool`'s handle, which
    /// starts at the middle of the face and can be dragged anywhere.
    pub origin: Option<Vec2f>,
    /// What the drag in progress has hold of, if one is.
    grip: Option<Grip>,
    /// How much of the drag has reached the document: an offset, an angle in `x`, a
    /// scale, or shear factors, by grip.
    applied: Vec2f,
    /// Where a shear drag began, relative to the origin handle and in the face's own
    /// units along the UV axes: `UvShearDragTracker::m_initialHit`.
    initial_hit: Vec2f,
    /// Where the drag began, in canvas pixels less the pan.
    start: (f64, f64),
    /// Whether the drag opened a transaction.
    transaction: bool,
    /// The pan a navigation drag started from.
    pan_start: (f64, f64),
}

impl UvCanvas {
    /// Paints the canvas, `width` by `height` pixels.
    pub fn draw(&mut self, document: &Document, cr: &mut dyn Canvas, width: f64, height: f64) {
        if self.view.face != document.face_to_edit().map(|(id, _)| id) {
            self.origin = None;
        }
        self.view.sync(document, width, height);
        let view = self.view;
        draw(document, cr, width, height, view);
        if document
            .face_to_edit()
            .is_some_and(|(_, face)| face.export_hidden)
        {
            cr.save();
            cr.set_source_rgba(1.0, 0.55, 0.1, 0.28);
            cr.set_line_width(1.0);
            for n in -20..40 {
                let x = n as f64 * 16.0;
                cr.move_to(x, 0.0);
                cr.line_to(x + height, height);
            }
            cr.stroke();
            cr.restore();
        }

        // The handles go on top of the face's outline, so they are findable whatever the
        // material behind them looks like.
        if let Some(at) = handle_origin(document, self.origin) {
            let (x, y) = to_canvas(document, height * view.zoom, at);
            draw_handles(cr, (x + view.pan.0, y + view.pan.1));
        }
    }

    /// A press that starts a drag at `(x, y)` on a canvas `height` pixels tall. `shear`
    /// is an Alt-press, or a Shift-middle one: it catches a tile edge and skews the
    /// material, and anywhere else does nothing.
    pub fn drag_begin(
        &mut self,
        document: &mut Document,
        height: f64,
        x: f64,
        y: f64,
        shear: bool,
    ) {
        self.applied = Vec2f::zero();
        let camera = self.view;
        let height = height * camera.zoom;
        let (x, y) = (x - camera.pan.0, y - camera.pan.1);
        self.start = (x, y);
        let grip = match handle_origin(document, self.origin) {
            Some(at) if shear => {
                // `UvShearTool::acceptMouseDrag`: Alt and a tile edge under the pointer,
                // far enough from the origin's own axes to be safe.
                let held = shear_grip_at(document, height, x, y);
                let hit = unscaled_offset(document, from_canvas(document, height, x, y) - at);
                if held.is_some()
                    && hit.x().abs() >= SHEAR_MIN_OFFSET
                    && hit.y().abs() >= SHEAR_MIN_OFFSET
                {
                    self.initial_hit = hit;
                    held.unwrap_or(Grip::Nothing)
                } else {
                    Grip::Nothing
                }
            }
            Some(at) => grip_at(to_canvas(document, height, at), x, y),
            None => Grip::Offset,
        };
        self.grip = Some(grip);
        if !matches!(grip, Grip::Nothing | Grip::Origin) {
            document.begin_transaction();
            self.transaction = true;
        }
    }

    /// The drag has moved `(dx, dy)` pixels from where it began. `control` turns the
    /// shear's snapping off. Returns whether the document changed.
    pub fn drag_update(
        &mut self,
        document: &mut Document,
        height: f64,
        dx: f64,
        dy: f64,
        control: bool,
    ) -> bool {
        let Some(held) = self.grip else { return false };
        let camera = self.view;
        let height = height * camera.zoom;
        let (start_x, start_y) = self.start;
        if held == Grip::Nothing {
            return false;
        }
        let Some(handle) = handle_origin(document, self.origin) else {
            return false;
        };
        let from = from_canvas(document, height, start_x, start_y);
        let to = from_canvas(document, height, start_x + dx, start_y + dy);

        match held {
            Grip::Offset => {
                // The whole drag is measured from where it began, so the rounding to a
                // texel does not eat a little of every step.
                let total = to - from;
                let step = total - self.applied;
                if step == Vec2f::zero() {
                    return false;
                }
                self.applied = total;
                document.edit_faces(|face| face.move_uv(-step));
            }

            Grip::Origin => {
                // The origin is not part of the mapping, so moving it changes nothing
                // about the face; it only says what the other two work about.
                self.origin = Some(handle + (to - from));
                self.start = (start_x + dx, start_y + dy);
                return false;
            }

            Grip::Rotate => {
                // The angle from the handle to where the pointer is, less the angle it
                // started at: `UvRotateTool`'s drag.
                let Some(started) = angle_between(handle, from) else {
                    return false;
                };
                let Some(now) = angle_between(handle, to) else {
                    return false;
                };
                // `UvRotateToolController::snapAngle`: when the turn would bring a face
                // edge parallel to the U or V axis, hold it there, so lining a material
                // up with a wall is a matter of turning until it clicks.
                let raw = now - started;
                let applied_so_far = self.applied.x();
                let total = snap_rotation_to_edges(document, applied_so_far, raw);
                let step = total - applied_so_far;
                if step == 0.0 {
                    return false;
                }
                self.applied.v[0] = total;
                document.edit_faces(|face| face.rotate_uv_about(handle, step));
            }

            Grip::Shear {
                horizontally,
                vertically,
            } => {
                // `UvShearDragTracker::update`: the factors come from how far the pointer
                // has moved across each axis, over how far from the origin the handle
                // started along the other, so a handle far out shears gently and one
                // close in shears fast.
                let initial = self.initial_hit;
                let current = unscaled_offset(document, to - handle);
                let zoom = canvas_scale(document, height) as f32;
                let delta = current - initial;
                let factors = Vec2f::new(-delta.y() / initial.x(), -delta.x() / initial.y());
                let factors = if control {
                    factors
                } else {
                    snap_shear_factors(document, factors, current, zoom)
                };
                let total = Vec2f::new(
                    if horizontally { factors.x() } else { 0.0 },
                    if vertically { factors.y() } else { 0.0 },
                );
                let previous = self.applied;
                if total == previous {
                    return false;
                }
                // Undo the previous shear with its inverse before applying the new one,
                // since consecutive shear matrices do not add linearly.
                let Some(undo) = invert_2x2(shear_rows(previous)) else {
                    return false;
                };
                let rows = mul_2x2(shear_rows(total), undo);
                self.applied = total;
                document.edit_faces(|face| face.skew_uv_about(handle, rows));
            }

            Grip::Nothing => return false,

            Grip::Scale { .. } => {
                // How much further from the handle the pointer is than it was, per axis. A
                // scale is a ratio, so this is a division rather than the difference the
                // other drags use.
                let started = from - handle;
                let now = to - handle;
                let ratio = |before: f32, after: f32| {
                    if before.abs() < 1e-3 {
                        1.0
                    } else {
                        (after / before).clamp(0.05, 20.0)
                    }
                };
                let total = Vec2f::new(ratio(started.x(), now.x()), ratio(started.y(), now.y()));
                let previous = if self.applied == Vec2f::zero() {
                    Vec2f::new(1.0, 1.0)
                } else {
                    self.applied
                };
                let step = Vec2f::new(total.x() / previous.x(), total.y() / previous.y());
                if step == Vec2f::new(1.0, 1.0) {
                    return false;
                }
                self.applied = total;
                document.edit_faces(|face| face.scale_uv_about(handle, step));
            }
        }
        true
    }

    /// The drag ended: what it did becomes one step of the history.
    pub fn drag_end(&mut self, document: &mut Document) {
        self.grip = None;
        if std::mem::take(&mut self.transaction) {
            document.commit_transaction();
        }
    }

    /// The drag was cancelled: what it did is taken back. Returns whether the document
    /// changed.
    pub fn drag_cancel(&mut self, document: &mut Document) -> bool {
        self.grip = None;
        let open = std::mem::take(&mut self.transaction);
        if open {
            document.cancel_transaction();
        }
        open
    }

    /// Whether a drag is in progress.
    pub fn dragging(&self) -> bool {
        self.grip.is_some()
    }

    /// The wheel turned `dy` notches with the pointer at `(x, y)`.
    pub fn scroll(&mut self, x: f64, y: f64, dy: f64) {
        self.view.zoom_at(x, y, dy);
    }

    /// A middle or right drag began, which moves the view.
    pub fn pan_begin(&mut self) {
        self.pan_start = self.view.pan;
    }

    /// The navigation drag has moved `(dx, dy)` pixels from where it began.
    pub fn pan_update(&mut self, dx: f64, dy: f64) {
        let (x, y) = self.pan_start;
        self.view.pan = (x + dx, y + dy);
    }

    /// A double right-click: the view frames the face again.
    pub fn reset_view(&mut self) {
        self.view = UvView::default();
    }
}

/// What a button beside the canvas does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Default alignment, keeping the axes the face has.
    Reset,
    /// Default alignment on the world-aligned axes.
    ResetToWorld,
    /// Stretch one copy of the material over the face.
    Fit,
    /// Mirror across, or down when false.
    Flip { horizontally: bool },
    /// A quarter turn: `UvEditor::rotateUvCCWClicked` / `rotateUvCWClicked`, which the ring
    /// and the ±15° keys make fiddly to land exactly.
    Rotate { counter_clockwise: bool },
    /// Turn the material along one of the face's edges.
    Align,
    /// Push the material against one of the face's sides.
    Justify(UvAxis, UvSign),
    /// Scale to a whole number of copies along one axis.
    FitAxis(UvAxis),
}

/// The first row of buttons: label, action and tooltip.
pub const BUTTONS: [(&str, Action, &str); 7] = [
    (
        translation_key("Reset"),
        Action::Reset,
        translation_key("Default alignment, keeping the axes the face has"),
    ),
    (
        translation_key("Reset to world"),
        Action::ResetToWorld,
        translation_key(
            "Default alignment on the world-aligned axes, dropping a Valve face's free axes",
        ),
    ),
    (translation_key("Fit"), Action::Fit, ""),
    (
        translation_key("Flip H"),
        Action::Flip { horizontally: true },
        "",
    ),
    (
        translation_key("Flip V"),
        Action::Flip {
            horizontally: false,
        },
        "",
    ),
    (
        translation_key("⟲ 90°"),
        Action::Rotate {
            counter_clockwise: true,
        },
        translation_key("Rotate the material 90° counter-clockwise"),
    ),
    (
        translation_key("⟳ 90°"),
        Action::Rotate {
            counter_clockwise: false,
        },
        translation_key("Rotate the material 90° clockwise"),
    ),
];

/// The alignment buttons, which cycle through their answers when pressed again; Shift
/// steps backwards, and Ctrl asks a fit to keep a trim sheet's subdivision.
pub const ALIGNMENT_BUTTONS: [(&str, Action, &str); 7] = [
    (translation_key("Align"), Action::Align, translation_key("Turn the material along one of the face's edges; press again for the next edge, Shift for the last")),
    ("◧", Action::Justify(UvAxis::U, UvSign::Minus), translation_key("Push the material against the face's left edge")),
    ("◨", Action::Justify(UvAxis::U, UvSign::Plus), translation_key("Push it against the right edge")),
    ("⬒", Action::Justify(UvAxis::V, UvSign::Minus), translation_key("Push it against the top edge")),
    ("⬓", Action::Justify(UvAxis::V, UvSign::Plus), translation_key("Push it against the bottom edge")),
    (translation_key("Fit U"), Action::FitAxis(UvAxis::U), translation_key("Scale across so a whole number of copies covers the face; press again for the next count, Shift for the last, Ctrl to keep a trim sheet's subdivision")),
    (translation_key("Fit V"), Action::FitAxis(UvAxis::V), translation_key("The same down the face")),
];

/// The size of the edited face's material, in texels, or the default when it is not
/// loaded.
fn material_size_vec(document: &Document) -> Vec2f {
    document
        .face_to_edit()
        .and_then(|(_, face)| document.materials().get(&face.material_name))
        .map(|material| Vec2f::new(material.width as f32, material.height as f32))
        .unwrap_or(crate::commands::DEFAULT_MATERIAL_SIZE)
}

/// Does what a button asks to the selected faces.
pub fn run(document: &mut Document, action: Action, modifiers: Modifiers) {
    // `FaceAttribsEditor`: Shift steps backwards through the answers, and Ctrl asks a fit
    // to keep a trim sheet's subdivision.
    let policy = if modifiers.shift {
        UvPolicy::Prev
    } else {
        UvPolicy::Next
    };
    let mode = if modifiers.control {
        UvFitMode::TrimSheet
    } else {
        UvFitMode::FitToFace
    };
    match action {
        Action::Reset => document.edit_faces(|face| face.reset_uv()),
        Action::ResetToWorld => document.edit_faces(|face| face.reset_uv_to_paraxial()),
        Action::Flip { horizontally } => document.edit_faces(|face| face.flip_uv(horizontally)),
        Action::Rotate { counter_clockwise } => {
            let degrees = if counter_clockwise { 90.0 } else { -90.0 };
            document.edit_faces(|face| face.rotate_uv(degrees));
        }
        // Fitting and aligning need the face's own outline and the size of what is being
        // fitted to it, neither of which the face knows on its own.
        Action::Fit | Action::Align | Action::Justify(..) | Action::FitAxis(_) => {
            let Some(polygon) = document.face_to_edit_polygon() else {
                return;
            };
            let size = material_size_vec(document);
            document.edit_faces(|face| match action {
                Action::Fit => face.fit_uv_to_face(&polygon, size),
                Action::Align => {
                    if let Some(angle) = map_model::uv_alignment::align(face, &polygon, policy) {
                        // A parallel system keeps its rotation in its axes, so the turn has
                        // to go through `rotate_uv` rather than the attribute alone.
                        face.rotate_uv(angle - face.uv_attributes.rotation);
                    }
                }
                Action::Justify(axis, sign) => {
                    if let Some(offset) =
                        map_model::uv_alignment::justify(face, &polygon, size, axis, sign, policy)
                    {
                        let was = face.uv_attributes.offset;
                        face.uv_attributes.offset = match axis {
                            UvAxis::U => Vec2f::new(offset, was.y()),
                            UvAxis::V => Vec2f::new(was.x(), offset),
                        };
                    }
                }
                Action::FitAxis(axis) => {
                    if let Some(scale) =
                        map_model::uv_alignment::fit(face, &polygon, size, axis, policy, mode)
                    {
                        let was = face.uv_attributes.scale;
                        face.uv_attributes.scale = match axis {
                            UvAxis::U => Vec2f::new(scale, was.y()),
                            UvAxis::V => Vec2f::new(was.x(), scale),
                        };
                    }
                }
                _ => {}
            });
        }
    }
}

/// One of the number fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    OffsetX,
    OffsetY,
    ScaleX,
    ScaleY,
    Rotation,
    ContentFlags,
    SurfaceFlags,
    SurfaceValue,
}

impl Field {
    /// The smallest and largest value the field takes, how far a step moves it, and how
    /// many decimals it shows.
    pub fn range(self) -> (f64, f64, f64, usize) {
        const INT: (f64, f64, f64, usize) = (-2_147_483_648.0, 2_147_483_647.0, 1.0, 0);
        match self {
            Field::OffsetX | Field::OffsetY => (-4096.0, 4096.0, 1.0, 0),
            Field::ScaleX | Field::ScaleY => (-64.0, 64.0, 0.05, 3),
            Field::Rotation => (-360.0, 360.0, 5.0, 1),
            Field::ContentFlags | Field::SurfaceFlags | Field::SurfaceValue => INT,
        }
    }

    /// What the edited face has in this field. A face with no surface attributes shows
    /// zeroes.
    pub fn value(self, face: &map_model::brush_face::BrushFace) -> f64 {
        let attributes = face.uv_attributes;
        let surface = face.surface_attributes.unwrap_or_default();
        match self {
            Field::OffsetX => f64::from(attributes.offset.x()),
            Field::OffsetY => f64::from(attributes.offset.y()),
            Field::ScaleX => f64::from(attributes.scale.x()),
            Field::ScaleY => f64::from(attributes.scale.y()),
            Field::Rotation => f64::from(attributes.rotation),
            Field::ContentFlags => f64::from(surface.content_flags),
            Field::SurfaceFlags => f64::from(surface.surface_flags),
            Field::SurfaceValue => f64::from(surface.value),
        }
    }

    /// Sets the field on every selected face. Setting a surface attribute gives the face
    /// the lot, which is what the format writes.
    pub fn set(self, document: &mut Document, value: f64) {
        document.edit_faces(|face| {
            let offset = face.uv_attributes.offset;
            let scale = face.uv_attributes.scale;
            let mut surface = face.surface_attributes.unwrap_or_default();
            match self {
                Field::OffsetX => face.uv_attributes.offset = Vec2f::new(value as f32, offset.y()),
                Field::OffsetY => face.uv_attributes.offset = Vec2f::new(offset.x(), value as f32),
                Field::ScaleX => face.uv_attributes.scale = Vec2f::new(value as f32, scale.y()),
                Field::ScaleY => face.uv_attributes.scale = Vec2f::new(scale.x(), value as f32),
                Field::Rotation => face.uv_attributes.rotation = value as f32,
                Field::ContentFlags | Field::SurfaceFlags | Field::SurfaceValue => {
                    match self {
                        Field::ContentFlags => surface.content_flags = value as i32,
                        Field::SurfaceFlags => surface.surface_flags = value as i32,
                        _ => surface.value = value as i32,
                    }
                    face.surface_attributes = Some(surface);
                }
            }
        });
    }
}

/// The fields in the order the panel lays them out, with their row labels: offset,
/// scale, rotation, the order the `.map` file writes them; then what a Quake 2 or Quake
/// 3 map carries on a face besides its mapping.
pub const ROWS: [(&str, &[Field]); 6] = [
    (translation_key("Offset"), &[Field::OffsetX, Field::OffsetY]),
    (translation_key("Scale"), &[Field::ScaleX, Field::ScaleY]),
    (translation_key("Angle"), &[Field::Rotation]),
    (translation_key("Content flags"), &[Field::ContentFlags]),
    (translation_key("Surface flags"), &[Field::SurfaceFlags]),
    (translation_key("Surface value"), &[Field::SurfaceValue]),
];

/// `FaceAttribsEditor`'s flag list: a tick per named bit, as the mask and the name.
/// `names[i]` names bit `1 << i`; an empty name is a bit the configuration does not use
/// and gets no tick.
pub fn flag_ticks(names: &[String]) -> Vec<(i32, &str)> {
    names
        .iter()
        .enumerate()
        .filter(|(bit, name)| !name.is_empty() && *bit < 31)
        .map(|(bit, name)| (1i32 << bit, name.as_str()))
        .collect()
}

/// The line under the canvas: the material's name and size.
pub fn status(document: &Document) -> String {
    match document.face_to_edit() {
        Some((_, face)) => {
            let size = document
                .materials()
                .get(&face.material_name)
                .map(|material| format!("{}×{}", material.width, material.height))
                .unwrap_or_else(|| "not loaded".to_string());
            format!("{} · {size}", face.material_name)
        }
        None => tr("No face selected").to_string(),
    }
}

/// Marks the selected faces to be left out of exports, or not.
pub fn set_export_hidden(document: &mut Document, hidden: bool) {
    document.edit_faces(|face| face.export_hidden = hidden);
}

/// Marks the faces that other brushes cover as left out of exports, and says how many.
pub fn omit_covered(document: &mut Document) -> String {
    let count = document.omit_covered_faces();
    format!("{}: {}", tr("Omit covered faces"), count)
}

/// How near, in degrees of the drag, an edge has to come to an axis before the turn is
/// held there.
pub const EDGE_SNAP_DEGREES: f32 = 4.0;

/// Holds a rotation drag where a face edge would line up with the U or V axis.
///
/// `UvRotateToolController::snapAngle`. The face is already turned by `applied`; `raw` is
/// where the drag would leave it. For each edge, the further turn that would bring it
/// parallel to an axis is found, and if that is within [`EDGE_SNAP_DEGREES`] of where the
/// drag is the total is pulled onto it. Which way a turn moves a UV angle is measured
/// from a probe rather than assumed, since it depends on the coordinate system.
pub fn snap_rotation_to_edges(document: &Document, applied: f32, raw: f32) -> f32 {
    let Some((_, face)) = document.face_to_edit() else {
        return raw;
    };
    let Some(polygon) = document.face_to_edit_polygon() else {
        return raw;
    };
    if polygon.len() < 2 {
        return raw;
    }

    let edge_angle = |face: &map_model::brush_face::BrushFace, a: Vec3d, b: Vec3d| -> Option<f32> {
        let d = face.uv_coords(b) - face.uv_coords(a);
        (d.x().abs() > 1e-4 || d.y().abs() > 1e-4).then(|| d.y().atan2(d.x()).to_degrees())
    };

    // The sign of how a positive turn moves a UV-space angle.
    let mut probe = face.clone();
    probe.rotate_uv(10.0);
    let (Some(before), Some(after)) = (
        edge_angle(face, polygon[0], polygon[1]),
        edge_angle(&probe, polygon[0], polygon[1]),
    ) else {
        return raw;
    };
    let sign = wrap_180(after - before).signum();
    if sign == 0.0 {
        return raw;
    }

    let want = raw - applied;
    let mut best = raw;
    let mut best_gap = EDGE_SNAP_DEGREES;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let Some(theta) = edge_angle(face, a, b) else {
            continue;
        };
        // The further turn `d` that makes this edge axis-aligned: theta + sign*d ≡ 0 (90).
        let base = -theta / sign;
        let cand = base + 90.0 * ((want - base) / 90.0).round();
        let gap = (cand - want).abs();
        if gap < best_gap {
            best_gap = gap;
            best = applied + cand;
        }
    }
    best
}

/// Brings an angle in degrees into `(-180, 180]`.
pub fn wrap_180(angle: f32) -> f32 {
    let a = angle % 360.0;
    if a > 180.0 {
        a - 360.0
    } else if a <= -180.0 {
        a + 360.0
    } else {
        a
    }
}

/// The angle from one point to another, in degrees, or nothing if they are the same
/// point and there is no direction between them.
pub fn angle_between(from: Vec2f, to: Vec2f) -> Option<f32> {
    let delta = to - from;
    if delta.x().abs() < 1e-4 && delta.y().abs() < 1e-4 {
        return None;
    }
    Some(delta.y().atan2(delta.x()).to_degrees())
}

/// Where the handles sit, in texels: wherever they were dragged to, or the middle of the
/// face until they have been.
pub fn handle_origin(document: &Document, origin: Option<Vec2f>) -> Option<Vec2f> {
    if let Some(at) = origin {
        return Some(at);
    }

    // `UvViewHelper::resetOrigin`: the middle of the face, which is where a rotation is
    // usually wanted.
    let (_, face) = document.face_to_edit()?;
    let polygon = document.face_to_edit_polygon()?;
    if polygon.is_empty() {
        return None;
    }
    let total = polygon
        .iter()
        .fold(Vec2f::zero(), |sum, corner| sum + face.uv_coords(*corner));
    Some(total / polygon.len() as f32)
}

/// Where a point in texels falls on the canvas.
pub fn to_canvas(document: &Document, height: f64, uv: Vec2f) -> (f64, f64) {
    let scale = canvas_scale(document, height);
    (f64::from(uv.x()) * scale, f64::from(uv.y()) * scale)
}

/// Where a point on the canvas falls in texels.
pub fn from_canvas(document: &Document, height: f64, x: f64, y: f64) -> Vec2f {
    let scale = canvas_scale(document, height);
    Vec2f::new((x / scale) as f32, (y / scale) as f32)
}

/// Draws the origin, the ring the rotation runs on, and the four scale corners.
pub fn draw_handles(cr: &mut dyn Canvas, (x, y): (f64, f64)) {
    // The ring first, so the handles on it sit on top.
    cr.set_source_rgba(0.55, 0.85, 1.0, 0.55);
    cr.set_line_width(1.0);
    cr.arc(x, y, ROTATE_RADIUS, 0.0, std::f64::consts::TAU);
    cr.stroke();

    let dot = |cr: &mut dyn Canvas, at: (f64, f64), colour: (f64, f64, f64)| {
        cr.set_source_rgb(colour.0, colour.1, colour.2);
        cr.arc(at.0, at.1, HANDLE_RADIUS, 0.0, std::f64::consts::TAU);
        cr.fill_preserve();
        cr.set_source_rgb(0.1, 0.1, 0.12);
        cr.set_line_width(1.0);
        cr.stroke();
    };

    // The scale corners.
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        dot(
            cr,
            (x + dx * SCALE_RADIUS, y + dy * SCALE_RADIUS),
            (0.55, 0.85, 1.0),
        );
    }
    // The rotation handle, on the ring.
    dot(cr, (x + ROTATE_RADIUS, y), (0.4, 1.0, 0.5));
    // And the origin itself, which the other two work about.
    dot(cr, (x, y), (1.0, 0.65, 0.2));
}

/// What a press at the given canvas point catches.
pub fn grip_at(origin: (f64, f64), x: f64, y: f64) -> Grip {
    let near = |at: (f64, f64)| {
        let (dx, dy) = (x - at.0, y - at.1);
        (dx * dx + dy * dy).sqrt() <= HANDLE_RADIUS + 2.0
    };

    if near(origin) {
        return Grip::Origin;
    }
    if near((origin.0 + ROTATE_RADIUS, origin.1)) {
        return Grip::Rotate;
    }
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        if near((origin.0 + dx * SCALE_RADIUS, origin.1 + dy * SCALE_RADIUS)) {
            return Grip::Scale {
                horizontally: true,
                vertically: true,
            };
        }
    }
    Grip::Offset
}

/// The tile edge, if any, an Alt-press at the given canvas point takes as a shear
/// handle: `UvViewHelper::pickUvGrid`, whose grid is one line per material repeat.
pub fn shear_grip_at(document: &Document, height: f64, x: f64, y: f64) -> Option<Grip> {
    let scale = canvas_scale(document, height);
    let (width, material_height) = material_size(document);
    let uv = from_canvas(document, height, x, y);
    let near = |coordinate: f32, stripe: f64| {
        let coordinate = f64::from(coordinate);
        let nearest = (coordinate / stripe).round() * stripe;
        (coordinate - nearest).abs() * scale <= SHEAR_GRIP_DISTANCE
    };
    let horizontally = near(uv.x(), width);
    let vertically = near(uv.y(), material_height);
    (horizontally || vertically).then_some(Grip::Shear {
        horizontally,
        vertically,
    })
}

/// The material's size in texels, or the default when it is not loaded.
pub fn material_size(document: &Document) -> (f64, f64) {
    document
        .face_to_edit()
        .and_then(|(_, face)| document.materials().get(&face.material_name))
        .map(|material| (f64::from(material.width), f64::from(material.height)))
        .unwrap_or((
            f64::from(crate::commands::DEFAULT_MATERIAL_SIZE.x()),
            f64::from(crate::commands::DEFAULT_MATERIAL_SIZE.y()),
        ))
}

/// Converts a texel displacement into face-space units by removing the UV mapping scale.
pub fn unscaled_offset(document: &Document, texels: Vec2f) -> Vec2f {
    let scale = document
        .face_to_edit()
        .map(|(_, face)| face.uv_attributes.scale)
        .unwrap_or(Vec2f::new(1.0, 1.0));
    Vec2f::new(
        texels.x() * map_model::uv::safe_scale(scale.x()),
        texels.y() * map_model::uv::safe_scale(scale.y()),
    )
}

/// `snapShearFactors`: when a shear would bring a face edge parallel to the U or V axis,
/// hold it there. `offset` is where the pointer is relative to the origin, unscaled, and
/// `zoom` how many pixels a unit is, which together say how far a pixel of drag moves
/// the factor and so how wide the snap should be.
pub fn snap_shear_factors(document: &Document, factors: Vec2f, offset: Vec2f, zoom: f32) -> Vec2f {
    let Some((_, face)) = document.face_to_edit() else {
        return factors;
    };
    let Some(polygon) = document.face_to_edit_polygon() else {
        return factors;
    };
    // `getEdgeVectorsUv`: each edge of the face in unscaled UV coordinates.
    let (u_axis, v_axis) = face
        .uv_coord_system
        .axes(face.normal(), &face.uv_attributes);
    let edges: Vec<Vec2f> = polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(from, to)| {
            let d = *to - *from;
            Vec2f::new(
                geometry_math::vec::dot(d, u_axis) as f32,
                geometry_math::vec::dot(d, v_axis) as f32,
            )
        })
        .collect();

    let snap = |factor: f32, orthogonal: f32, axis: usize| -> f32 {
        // `getSnappedShearFactors`: the factor that would lay each edge along the axis.
        let nearest = edges
            .iter()
            .filter(|e| e[axis].abs() > 1e-4)
            .map(|e| -e[1 - axis] / e[axis])
            .min_by(|a, b| (a - factor).abs().total_cmp(&(b - factor).abs()));
        let threshold = 10.0 / orthogonal.abs() / zoom;
        match nearest {
            Some(snapped) if (snapped - factor).abs() < threshold => snapped,
            _ => factor,
        }
    };
    Vec2f::new(
        snap(factors.x(), offset.x(), 0),
        snap(factors.y(), offset.y(), 1),
    )
}

/// Builds the UV shear matrix from two factors.
pub fn shear_rows(factors: Vec2f) -> [[f64; 2]; 2] {
    [[1.0, f64::from(factors.x())], [f64::from(factors.y()), 1.0]]
}

pub fn invert_2x2(m: [[f64; 2]; 2]) -> Option<[[f64; 2]; 2]> {
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if det.abs() < 1e-12 {
        return None;
    }
    Some([
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ])
}

pub fn mul_2x2(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}

/// How many canvas pixels one texel takes.
pub fn canvas_scale(document: &Document, height: f64) -> f64 {
    let material_height = document
        .face_to_edit()
        .and_then(|(_, face)| document.materials().get(&face.material_name))
        .map(|material| f64::from(material.height))
        .unwrap_or(f64::from(crate::commands::DEFAULT_MATERIAL_SIZE.y()));
    // Two copies of the material fit in the canvas, so the face's outline has somewhere to
    // sit when it spans more than one.
    height / (material_height * 2.0).max(1.0)
}

/// Draws the material with the face's outline where it falls on it.
pub fn draw(document: &Document, cr: &mut dyn Canvas, width: f64, height: f64, view: UvView) {
    cr.set_source_rgb(0.12, 0.12, 0.14);
    cr.paint();

    let Some((_, face)) = document.face_to_edit() else {
        cr.set_source_rgb(0.5, 0.5, 0.55);
        // The message in the middle of the canvas, as an empty list shows its own.
        let text = tr("No face selected");
        let extents = cr.text_extents(text, 10.0);
        let x = ((width - extents.width) / 2.0 - extents.x_bearing).max(8.0);
        cr.move_to(x, height / 2.0 - extents.y_bearing - extents.height / 2.0);
        crate::view2d::draw_text(cr, text, 10.0);
        return;
    };

    let scale = canvas_scale(document, height * view.zoom);
    let material = document.materials().get(&face.material_name);
    let (material_width, material_height) = material
        .map(|material| (f64::from(material.width), f64::from(material.height)))
        .unwrap_or((
            f64::from(crate::commands::DEFAULT_MATERIAL_SIZE.x()),
            f64::from(crate::commands::DEFAULT_MATERIAL_SIZE.y()),
        ));

    // The material, tiled across the canvas the way it tiles across the face.
    if let Some(material) = material {
        // Decoded on demand; the canvas keeps its own copy (keyed by name), so this
        // decodes once, not per frame, while the canvas has it.
        if let Some(pixels) = (material.width > 0 && material.height > 0)
            .then(|| material.pixels())
            .flatten()
        {
            cr.paint_tiled_image(
                &face.material_name,
                material.width,
                material.height,
                &pixels[..(material.width * material.height * 4) as usize],
                view.pan,
                scale,
            );
        }
    } else {
        cr.set_source_rgb(0.2, 0.2, 0.24);
        cr.rectangle(
            view.pan.0,
            view.pan.1,
            material_width * scale,
            material_height * scale,
        );
        cr.fill();
    }

    // The material-repeat grid supplies handles for shear drags.
    let (stripe_x, stripe_y) = (material_width * scale, material_height * scale);
    if stripe_x >= 1.0 && stripe_y >= 1.0 {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
        cr.set_line_width(1.0);
        let mut x = view.pan.0.rem_euclid(stripe_x);
        while x <= width {
            cr.move_to(x.round() + 0.5, 0.0);
            cr.line_to(x.round() + 0.5, height);
            x += stripe_x;
        }
        let mut y = view.pan.1.rem_euclid(stripe_y);
        while y <= height {
            cr.move_to(0.0, y.round() + 0.5);
            cr.line_to(width, y.round() + 0.5);
            y += stripe_y;
        }
        cr.stroke();
    }

    // The face's outline, where it falls on the material.
    let Some(polygon) = document.face_to_edit_polygon() else {
        return;
    };
    if polygon.is_empty() {
        return;
    }

    cr.set_source_rgb(1.0, 0.65, 0.2);
    cr.set_line_width(1.5);
    for (index, corner) in polygon.iter().enumerate() {
        let uv = face.uv_coords(*corner);
        let x = f64::from(uv.x()) * scale + view.pan.0;
        let y = f64::from(uv.y()) * scale + view.pan.1;
        if index == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.close_path();
    cr.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Raster;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;

    fn cube_with_a_selected_face() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::from_scalars(-32.0, 32.0),
            map_model::shapes::DEFAULT_MATERIAL,
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
        document.select_all();
        document
    }

    #[test]
    fn face_only_uv_drags_undo_redo_and_cancel_as_one_edit() {
        let operations: [fn(&mut map_model::brush_face::BrushFace); 4] = [
            |f| f.move_uv(Vec2f::new(7., -3.)),
            |f| f.rotate_uv_about(Vec2f::zero(), 15.),
            |f| f.scale_uv_about(Vec2f::zero(), Vec2f::new(1.5, 0.75)),
            |f| f.skew_uv_about(Vec2f::zero(), [[1., 0.2], [0., 1.]]),
        ];
        for (index, operation) in operations.into_iter().enumerate() {
            let mut document = cube_with_a_selected_face();
            let reference = document.face_to_edit().unwrap().0;
            document.select_face_only(reference);
            assert!(document.selection_ids().is_empty());
            // Shear requires Valve's explicit axes; Standard mappings cannot express it.
            document.edit_faces(|f| {
                let (u_axis, v_axis) = f.uv_coord_system.axes(f.normal(), &f.uv_attributes);
                f.uv_coord_system = map_model::uv::UvCoordSystem::Parallel { u_axis, v_axis };
            });
            let state = |d: &Document| {
                let b = d.brush(reference.node).unwrap();
                b.faces()
                    .iter()
                    .map(|f| (f.uv_attributes, f.uv_coord_system, f.primitive_uv.clone()))
                    .collect::<Vec<_>>()
            };
            let before = state(&document);
            document.begin_transaction();
            document.edit_faces(operation);
            document.edit_faces(operation);
            assert!(document.commit_transaction(), "operation {index}");
            let after = state(&document);
            assert_ne!(after, before);
            document.undo();
            assert_eq!(state(&document), before);
            document.redo();
            assert_eq!(state(&document), after);
            document.begin_transaction();
            document.edit_faces(operation);
            document.cancel_transaction();
            assert_eq!(state(&document), after);
        }
    }

    #[test]
    fn uv_navigation_keeps_the_cursor_anchor_and_does_not_edit_the_map() {
        let document = cube_with_a_selected_face();
        let mut view = UvView::default();
        view.sync(&document, 400.0, 200.0);
        let before = view;
        let pointer = (123.0, 87.0);
        let anchor = (
            (pointer.0 - view.pan.0) / view.zoom,
            (pointer.1 - view.pan.1) / view.zoom,
        );
        view.zoom_at(pointer.0, pointer.1, -3.0);
        assert!((view.pan.0 + anchor.0 * view.zoom - pointer.0).abs() < 1e-9);
        assert!((view.pan.1 + anchor.1 * view.zoom - pointer.1).abs() < 1e-9);
        view.zoom_at(pointer.0, pointer.1, 3.0);
        assert!((view.zoom - before.zoom).abs() < 1e-9);
        assert!((view.pan.0 - before.pan.0).abs() < 1e-9);
        view.zoom_at(0.0, 0.0, 1e6);
        assert_eq!(view.zoom, 0.02);
        view.zoom_at(0.0, 0.0, -1e6);
        assert_eq!(view.zoom, 64.0);
    }

    #[test]
    fn offset_moves_texture_and_keeps_outline_fixed_including_undo() {
        let mut document = cube_with_a_selected_face();
        let mut view = UvView::default();
        view.sync(&document, 400.0, 200.0);
        let point = document.face_to_edit_polygon().unwrap()[0];
        let position = |d: &Document, v: UvView| {
            let uv = d.face_to_edit().unwrap().1.uv_coords(point);
            let (x, y) = to_canvas(d, 200.0 * v.zoom, uv);
            (x + v.pan.0, y + v.pan.1)
        };
        let before = position(&document, view);
        let pan = view.pan;
        document.begin_transaction();
        document.edit_faces(|f| f.move_uv(Vec2f::new(-4.0, -2.0)));
        document.edit_faces(|f| f.move_uv(Vec2f::new(-6.0, -3.0)));
        document.commit_transaction();
        view.sync(&document, 400.0, 200.0);
        assert_eq!(position(&document, view), before);
        assert!(view.pan.0 > pan.0 && view.pan.1 > pan.1);
        document.undo();
        view.sync(&document, 400.0, 200.0);
        assert_eq!(position(&document, view), before);
        assert_eq!(view.pan, pan);
    }

    #[test]
    fn a_shear_near_an_edge_direction_snaps_onto_it() {
        // The cube's faces are square to the axes, so the edges lie along U and V and
        // the factor that lays an edge along an axis is nought.
        let document = cube_with_a_selected_face();
        let near = snap_shear_factors(
            &document,
            Vec2f::new(0.05, 0.0),
            Vec2f::new(20.0, 20.0),
            2.0,
        );
        assert_eq!(near, Vec2f::new(0.0, 0.0));
        // Further out than the threshold -- ten over the offset over the zoom -- it is
        // left alone.
        let far = snap_shear_factors(&document, Vec2f::new(0.6, 0.0), Vec2f::new(20.0, 20.0), 2.0);
        assert_eq!(far, Vec2f::new(0.6, 0.0));
    }

    #[test]
    fn a_shear_undone_by_its_inverse_is_the_identity() {
        let rows = shear_rows(Vec2f::new(0.4, -0.3));
        let undo = invert_2x2(rows).expect("a shear inverts");
        let product = mul_2x2(rows, undo);
        for (row, expected) in product.iter().zip([[1.0, 0.0], [0.0, 1.0]]) {
            for (value, want) in row.iter().zip(expected) {
                assert!((value - want).abs() < 1e-12, "{product:?}");
            }
        }
    }

    #[test]
    fn a_tile_edge_is_a_shear_handle_and_open_material_is_not() {
        let document = cube_with_a_selected_face();
        let height = f64::from(CANVAS_HEIGHT);
        let scale = canvas_scale(&document, height);
        let (width, material_height) = material_size(&document);
        // On the vertical edge of the first repeat, midway up a tile.
        let on_x = shear_grip_at(
            &document,
            height,
            width * scale,
            material_height * scale / 2.0,
        );
        assert_eq!(
            on_x,
            Some(Grip::Shear {
                horizontally: true,
                vertically: false
            })
        );
        // On the corner of the first repeat: both.
        let corner = shear_grip_at(&document, height, width * scale, material_height * scale);
        assert_eq!(
            corner,
            Some(Grip::Shear {
                horizontally: true,
                vertically: true
            })
        );
        // In the middle of a tile: nothing.
        let open = shear_grip_at(
            &document,
            height,
            width * scale / 2.0,
            material_height * scale / 2.0,
        );
        assert_eq!(open, None);
    }

    #[test]
    fn a_small_turn_from_square_snaps_back_to_the_axis() {
        let document = cube_with_a_selected_face();
        // A cube face's edges start axis-aligned, so a turn of a couple of degrees is held
        // at zero.
        assert_eq!(snap_rotation_to_edges(&document, 0.0, 2.5), 0.0);
        assert_eq!(snap_rotation_to_edges(&document, 0.0, -3.0), 0.0);
    }

    #[test]
    fn a_turn_past_the_snap_range_is_left_alone() {
        let document = cube_with_a_selected_face();
        assert_eq!(snap_rotation_to_edges(&document, 0.0, 20.0), 20.0);
    }

    #[test]
    fn a_turn_near_a_quarter_is_pulled_onto_it() {
        let document = cube_with_a_selected_face();
        // The other edge of the same face is at ninety degrees, so a near-quarter turn
        // lands there.
        let snapped = snap_rotation_to_edges(&document, 0.0, 87.5);
        assert!((snapped - 90.0).abs() < 1e-3, "{snapped}");
    }

    #[test]
    fn nothing_selected_means_nothing_to_snap_to() {
        let mut document = cube_with_a_selected_face();
        document.clear_selection();
        assert_eq!(snap_rotation_to_edges(&document, 0.0, 2.0), 2.0);
    }

    #[test]
    fn an_offset_drag_is_one_step_of_the_history() {
        let mut document = cube_with_a_selected_face();
        let mut canvas = UvCanvas::default();
        let mut raster = Raster::new(400, 200).unwrap();
        canvas.draw(&document, &mut raster, 400.0, 200.0);
        let before = document.face_to_edit().unwrap().1.uv_attributes;
        // Far from the handles in the middle of the face, so the drag shifts the material.
        canvas.drag_begin(&mut document, 200.0, 5.0, 5.0, false);
        assert!(canvas.drag_update(&mut document, 200.0, 20.0, 10.0, false));
        assert!(canvas.drag_update(&mut document, 200.0, 40.0, 10.0, false));
        canvas.drag_end(&mut document);
        assert_ne!(document.face_to_edit().unwrap().1.uv_attributes, before);
        document.undo();
        assert_eq!(document.face_to_edit().unwrap().1.uv_attributes, before);
    }

    #[test]
    fn flag_ticks_skip_unused_bits() {
        let names = vec!["solid".to_string(), String::new(), "detail".to_string()];
        assert_eq!(flag_ticks(&names), vec![(1, "solid"), (4, "detail")]);
    }
}
