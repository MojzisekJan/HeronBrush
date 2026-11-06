//! The UV editor's canvas and controls, without a toolkit: how a material sits on a face.
//!
//! [`UvCanvas`] draws the material with the face's outline and the handles, and turns
//! pointer events in canvas pixels into edits. [`Action`] and [`Field`] are the buttons and
//! number fields beside it. The operations themselves are in [`map_model::brush_face`];
//! nothing here computes a mapping, it only decides which operation a control asks for.

use std::collections::HashMap;

use geometry_math::vec::{Vec2f, Vec3d};
use map_model::uv_alignment::{UvAxis, UvFitMode, UvPolicy, UvSign};

use crate::canvas::Canvas;
use crate::document::Document;
use crate::i18n::{tr, translation_key};
use crate::input::Modifiers;
use crate::uv_workflow::{self, GridMode, PivotMode, SnapTarget, TransformKind};

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
    anchors: Option<[Vec2f; 3]>,
    axes: [[f64; 2]; 2],
    size: (f64, f64),
    base: f64,
    /// Subdivisions of one texture repeat, for trim-sheet editing.
    pub divisions: [u32; 2],
    /// UV display and transform settings shared by docked and detached views.
    pub options: uv_workflow::Options,
    grid: Option<Vec2f>,
    highlight: Option<(Vec2f, bool, bool)>,
}
impl Default for UvView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
            face: None,
            anchors: None,
            axes: [[1.0, 0.0], [0.0, 1.0]],
            size: (0.0, 0.0),
            base: 1.0,
            divisions: [1, 1],
            options: Default::default(),
            grid: None,
            highlight: None,
        }
    }
}
impl UvView {
    pub fn sync(&mut self, document: &Document, width: f64, height: f64) {
        let Some((id, face)) = document.face_to_edit() else {
            self.face = None;
            return;
        };
        let points = document.face_to_edit_polygon().unwrap_or_default();
        let anchors = (points.len() >= 3).then(|| {
            [
                face.uv_coords(points[0]),
                face.uv_coords(points[1]),
                face.uv_coords(points[2]),
            ]
        });
        let base = canvas_scale(document, height);
        if self.face != Some(id) {
            self.axes = [[1.0, 0.0], [0.0, 1.0]];
            if let Some(uv) = anchors {
                // Frame the face's physical plane, independently of the existing
                // material rotation, scale or shear (including a fresh selection).
                let normal = face.normal();
                let (u, v) =
                    map_model::uv::UvCoordSystem::Paraxial.axes(normal, &Default::default());
                let u =
                    geometry_math::vec::normalize(u - normal * geometry_math::vec::dot(u, normal));
                let v = v - normal * geometry_math::vec::dot(v, normal);
                let v = geometry_math::vec::normalize(v - u * geometry_math::vec::dot(v, u));
                let a = points[1] - points[0];
                let b = points[2] - points[0];
                let physical = [
                    [geometry_math::vec::dot(a, u), geometry_math::vec::dot(b, u)],
                    [geometry_math::vec::dot(a, v), geometry_math::vec::dot(b, v)],
                ];
                let mapping = [
                    [
                        f64::from(uv[1].x() - uv[0].x()),
                        f64::from(uv[2].x() - uv[0].x()),
                    ],
                    [
                        f64::from(uv[1].y() - uv[0].y()),
                        f64::from(uv[2].y() - uv[0].y()),
                    ],
                ];
                if let Some(inverse) = invert_2x2(mapping) {
                    self.axes = mul_2x2(physical, inverse);
                }
            }
            let uvs: Vec<_> = points
                .iter()
                .map(|p| {
                    let at = transform(self.axes, face.uv_coords(*p));
                    Vec2f::new(at.0 as f32, at.1 as f32)
                })
                .collect();
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
                self.zoom = scale.clamp(0.02, 64.0) / base;
                let center = (lo + hi) * 0.5;
                self.pan = (
                    width / 2.0 - f64::from(center.x()) * base * self.zoom,
                    height / 2.0 - f64::from(center.y()) * base * self.zoom,
                );
            }
            self.face = Some(id);
        } else {
            // Keep the face plane fixed while its UV mapping changes. This also
            // compensates texture dimensions, window resizing, and undo/redo.
            let old_scale = self.base * self.zoom;
            self.zoom *= self.base / base;
            self.pan.0 += (width - self.size.0) * 0.5;
            self.pan.1 += (height - self.size.1) * 0.5;
            if let (Some(old), Some(new)) = (self.anchors, anchors) {
                let basis = |p: [Vec2f; 3]| {
                    [
                        [
                            f64::from(p[1].x() - p[0].x()),
                            f64::from(p[2].x() - p[0].x()),
                        ],
                        [
                            f64::from(p[1].y() - p[0].y()),
                            f64::from(p[2].y() - p[0].y()),
                        ],
                    ]
                };
                if let Some(inverse) = invert_2x2(basis(new)) {
                    let next = mul_2x2(self.axes, mul_2x2(basis(old), inverse));
                    let before = transform(self.axes, old[0]);
                    let after = transform(next, new[0]);
                    self.pan.0 += (before.0 - after.0) * old_scale;
                    self.pan.1 += (before.1 - after.1) * old_scale;
                    self.axes = next;
                }
            }
        }
        self.anchors = anchors;
        self.size = (width, height);
        self.base = base;
    }

    /// Maps UV texels into local canvas pixels.
    pub fn project(&self, document: &Document, height: f64, uv: Vec2f) -> (f64, f64) {
        let at = transform(self.axes, uv);
        let scale = canvas_scale(document, height * self.zoom);
        (at.0 * scale + self.pan.0, at.1 * scale + self.pan.1)
    }
    /// Maps local canvas pixels back into UV texels.
    pub fn unproject(&self, document: &Document, height: f64, x: f64, y: f64) -> Vec2f {
        let scale = canvas_scale(document, height * self.zoom);
        let inverse = invert_2x2(self.axes).unwrap_or([[1.0, 0.0], [0.0, 1.0]]);
        let (x, y) = ((x - self.pan.0) / scale, (y - self.pan.1) / scale);
        Vec2f::new(
            (inverse[0][0] * x + inverse[0][1] * y) as f32,
            (inverse[1][0] * x + inverse[1][1] * y) as f32,
        )
    }
    fn axes_pixels(&self, document: &Document, height: f64) -> [[f64; 2]; 2] {
        let scale = canvas_scale(document, height * self.zoom);
        self.axes.map(|row| row.map(|v| v * scale))
    }

    pub fn zoom_at(&mut self, x: f64, y: f64, delta: f64) {
        // Limits are in face-plane pixels per world unit, independent of the
        // selected texture's dimensions and the current panel height.
        let next = (self.zoom * self.base * (-delta * 0.15).exp()).clamp(0.02, 64.0) / self.base;
        let ratio = next / self.zoom;
        self.pan = (x - (x - self.pan.0) * ratio, y - (y - self.pan.1) * ratio);
        self.zoom = next;
    }
}

fn transform(matrix: [[f64; 2]; 2], uv: Vec2f) -> (f64, f64) {
    (
        matrix[0][0] * f64::from(uv.x()) + matrix[0][1] * f64::from(uv.y()),
        matrix[1][0] * f64::from(uv.x()) + matrix[1][1] * f64::from(uv.y()),
    )
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
    /// Move just one component of the pivot by dragging its red axis.
    OriginAxis {
        horizontally: bool,
        vertically: bool,
    },
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
    drag_view: UvView,
    drag_origin: Option<Vec2f>,
    drag_scale: Vec2f,
    grid_hit: Option<Vec2f>,
    drag_face: Option<map_model::brush_face::BrushFace>,
}

impl UvCanvas {
    /// The currently chosen transform anchor, in texture pixels.
    pub fn pivot(&self, document: &Document) -> Option<Vec2f> {
        match self.view.options.pivot {
            PivotMode::Origin => document.face_to_edit().map(|_| Vec2f::zero()),
            PivotMode::Cursor => handle_origin(document, self.origin),
            PivotMode::Center | PivotMode::Individual => handle_origin(document, self.origin),
        }
    }

    /// Synchronizes the view and pivot before input or drawing.
    pub fn prepare(&mut self, document: &Document, width: f64, height: f64) {
        if self.view.face != document.face_to_edit().map(|(id, _)| id) {
            self.origin = None;
        }
        let old_origin = self.origin.map(|uv| {
            let at = transform(self.view.axes, uv);
            let scale = self.view.base * self.view.zoom;
            (
                at.0 * scale + self.view.pan.0 + (width - self.view.size.0) * 0.5,
                at.1 * scale + self.view.pan.1 + (height - self.view.size.1) * 0.5,
            )
        });
        self.view.sync(document, width, height);
        if let Some((x, y)) = old_origin {
            self.origin = Some(self.view.unproject(document, height, x, y));
        }
    }

    /// Paints the canvas, `width` by `height` pixels.
    pub fn draw(&mut self, document: &Document, cr: &mut dyn Canvas, width: f64, height: f64) {
        self.prepare(document, width, height);
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
        if let Some(at) = self.pivot(document) {
            let (x, y) = view.project(document, height, at);
            let axes = view.axes_pixels(document, height);
            cr.set_source_rgba(1.0, 0.25, 0.2, 0.65);
            cr.set_line_width(1.0);
            for (&axis_x, &axis_y) in axes[0].iter().zip(&axes[1]) {
                let length = axis_x.hypot(axis_y).max(1e-9);
                let dx = axis_x / length * (width + height);
                let dy = axis_y / length * (width + height);
                cr.move_to(x - dx, y - dy);
                cr.line_to(x + dx, y + dy);
            }
            cr.stroke();
            draw_handles(cr, (x, y));
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
        self.view.grid = Some(self.stripe(document));
        self.drag_view = self.view;
        self.drag_face = document.face_to_edit().map(|(_, face)| face.clone());
        self.drag_origin = self.pivot(document);
        self.drag_scale = document
            .face_to_edit()
            .map(|(_, f)| f.uv_attributes.scale)
            .unwrap_or(Vec2f::new(1.0, 1.0));
        self.start = (x, y);
        self.grid_hit = None;
        let hit = self.view.unproject(document, height, x, y);
        let grip = match self.drag_origin {
            Some(at) if shear => {
                let parallel = document.face_to_edit().is_some_and(|(_, f)| {
                    matches!(
                        f.uv_coord_system,
                        map_model::uv::UvCoordSystem::Parallel { .. }
                    )
                });
                let held = self.grid_grip(document, height, x, y);
                let hit = Vec2f::new(
                    (hit.x() - at.x()) * self.drag_scale.x(),
                    (hit.y() - at.y()) * self.drag_scale.y(),
                );
                match held {
                    Some(Grip::Scale {
                        horizontally,
                        vertically,
                    }) if parallel
                        && hit.x().abs() >= SHEAR_MIN_OFFSET
                        && hit.y().abs() >= SHEAR_MIN_OFFSET =>
                    {
                        self.initial_hit = hit;
                        Grip::Shear {
                            horizontally,
                            vertically,
                        }
                    }
                    _ => Grip::Nothing,
                }
            }
            Some(at) => {
                let origin = self.view.project(document, height, at);
                let handle = grip_at(origin, x, y);
                if handle != Grip::Offset {
                    handle
                } else if let Some(axis) =
                    origin_axis_grip(origin, self.view.axes_pixels(document, height), x, y)
                {
                    axis
                } else if let Some(grip) = self.grid_grip(document, height, x, y) {
                    let stripe = self.stripe(document);
                    let mut caught = Vec2f::new(
                        (hit.x() / stripe.x()).round() * stripe.x(),
                        (hit.y() / stripe.y()).round() * stripe.y(),
                    );
                    let Grip::Scale {
                        mut horizontally,
                        mut vertically,
                    } = grip
                    else {
                        unreachable!()
                    };
                    horizontally &= (caught.x() - at.x()).abs() > 1e-3;
                    vertically &= (caught.y() - at.y()).abs() > 1e-3;
                    if !horizontally {
                        caught.v[0] = hit.x();
                    }
                    if !vertically {
                        caught.v[1] = hit.y();
                    }
                    self.grid_hit = Some(caught);
                    if horizontally || vertically {
                        Grip::Scale {
                            horizontally,
                            vertically,
                        }
                    } else {
                        Grip::Offset
                    }
                } else {
                    Grip::Offset
                }
            }
            None => Grip::Nothing,
        };
        self.grip = Some(grip);
        if !matches!(grip, Grip::Nothing | Grip::Origin | Grip::OriginAxis { .. }) {
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
        let control = control || !self.view.options.snap;
        let Some(held) = self.grip else { return false };
        if self.drag_view.face != document.face_to_edit().map(|(id, _)| id) {
            self.drag_cancel(document);
            return false;
        }
        let (start_x, start_y) = self.start;
        if held == Grip::Nothing {
            return false;
        }
        let Some(handle) = self.drag_origin else {
            return false;
        };
        let from = self.drag_view.unproject(document, height, start_x, start_y);
        let to = self
            .drag_view
            .unproject(document, height, start_x + dx, start_y + dy);

        match held {
            Grip::Offset => {
                // The whole drag is measured from where it began, so the rounding to a
                // texel does not eat a little of every step.
                let total = if control {
                    to - from
                } else {
                    self.snap_offset(document, height, to - from)
                };
                let step = total - self.applied;
                if step == Vec2f::zero() {
                    return false;
                }
                self.applied = total;
                document.edit_faces(|face| face.move_uv(-step));
            }

            Grip::Origin | Grip::OriginAxis { .. } => {
                let mut target = handle + (to - from);
                if !control {
                    target = self.snap_point(document, height, target, true);
                }
                if let Grip::OriginAxis {
                    horizontally,
                    vertically,
                } = held
                {
                    if !horizontally {
                        target.v[0] = handle.x();
                    }
                    if !vertically {
                        target.v[1] = handle.y();
                    }
                }
                self.origin = Some(target);
                self.view.options.pivot = PivotMode::Cursor;
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
                let total = if control {
                    raw
                } else {
                    if self.view.options.snap_target == SnapTarget::Vertex {
                        snap_rotation_to_edges(document, applied_so_far, raw)
                    } else {
                        let step = self.view.options.rotation_step.max(0.1);
                        (raw / step).round() * step
                    }
                };
                let step = total - applied_so_far;
                if step == 0.0 {
                    return false;
                }
                self.applied.v[0] = total;
                uv_workflow::transform(
                    document,
                    uv_workflow::Transform::Rotate(step),
                    self.view.options.pivot,
                    handle,
                );
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
                let current = Vec2f::new(
                    (to.x() - handle.x()) * self.drag_scale.x(),
                    (to.y() - handle.y()) * self.drag_scale.y(),
                );
                let zoom = canvas_scale(document, height * self.drag_view.zoom) as f32;
                let delta = current - initial;
                let factors = Vec2f::new(-delta.y() / initial.x(), -delta.x() / initial.y());
                let factors = if control {
                    factors
                } else {
                    match (&self.drag_face, document.face_to_edit_polygon()) {
                        (Some(face), Some(polygon)) => {
                            snap_shear_for_face(face, &polygon, factors, current, zoom)
                        }
                        _ => factors,
                    }
                };
                let total = Vec2f::new(
                    if horizontally { factors.x() } else { 0.0 },
                    if vertically { factors.y() } else { 0.0 },
                );
                let previous = self.applied;
                if total == previous || invert_2x2(shear_rows(total)).is_none() {
                    return false;
                }
                // Undo the previous shear with its inverse before applying the new one,
                // since consecutive shear matrices do not add linearly.
                let Some(undo) = invert_2x2(shear_rows(previous)) else {
                    return false;
                };
                let rows = mul_2x2(shear_rows(total), undo);
                self.applied = total;
                document.edit_faces_with_polygon(|face, polygon| {
                    let pivot = if self.view.options.pivot == PivotMode::Individual {
                        uv_workflow::center(face, polygon)
                    } else {
                        handle
                    };
                    face.skew_uv_about(pivot, rows);
                });
            }

            Grip::Nothing => return false,

            Grip::Scale {
                horizontally,
                vertically,
            } => {
                let caught = self.grid_hit.unwrap_or(from);
                let started = caught - handle;
                let target = caught + (to - from);
                let target = if control {
                    target
                } else {
                    self.snap_point(document, height, target, false)
                };
                let now = target - handle;
                let ratio = |before: f32, after: f32, enabled: bool| {
                    if !enabled || before.abs() < 1e-3 {
                        1.0
                    } else {
                        let value = (after / before).clamp(-20.0, 20.0);
                        if value.abs() < 1e-4 {
                            1e-4_f32.copysign(value)
                        } else {
                            value
                        }
                    }
                };
                let total = Vec2f::new(
                    ratio(started.x(), now.x(), horizontally),
                    ratio(started.y(), now.y(), vertically),
                );
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
                uv_workflow::transform(
                    document,
                    uv_workflow::Transform::Scale(step),
                    self.view.options.pivot,
                    handle,
                );
            }
        }
        true
    }

    /// Begins a modal keyboard transform using the same transaction as mouse drags.
    pub fn begin_transform(
        &mut self,
        document: &mut Document,
        kind: TransformKind,
        height: f64,
        x: f64,
        y: f64,
    ) {
        self.drag_begin(document, height, x, y, false);
        if self.drag_origin.is_none() {
            return;
        }
        if !self.transaction {
            document.begin_transaction();
            self.transaction = true;
        }
        self.grip = Some(match kind {
            TransformKind::Move => Grip::Offset,
            TransformKind::Rotate => Grip::Rotate,
            TransformKind::Scale => Grip::Scale {
                horizontally: true,
                vertically: true,
            },
        });
        self.grid_hit = None;
        self.applied = if kind == TransformKind::Scale {
            Vec2f::new(1.0, 1.0)
        } else {
            Vec2f::zero()
        };
    }

    /// Applies the absolute preview value from the start of a keyboard operation.
    /// Numeric values can bypass snapping; changing the axis removes the prior other-axis preview.
    pub fn preview_transform(
        &mut self,
        document: &mut Document,
        kind: TransformKind,
        raw: Vec2f,
        axes: [bool; 2],
        bypass: bool,
    ) -> bool {
        if self.drag_view.face != document.face_to_edit().map(|(id, _)| id) {
            self.drag_cancel(document);
            return false;
        }
        let Some(origin) = self.drag_origin else {
            return false;
        };
        let snap = self.view.options.snap && !bypass;
        let height = self.view.size.1;
        match kind {
            TransformKind::Move => {
                let mut total = if snap {
                    self.snap_offset(document, height, raw)
                } else {
                    raw
                };
                for (k, &enabled) in axes.iter().enumerate() {
                    if !enabled {
                        total.v[k] = 0.0;
                    }
                }
                let step = total - self.applied;
                if step == Vec2f::zero() {
                    return false;
                }
                self.applied = total;
                document.edit_faces(|f| f.move_uv(-step));
            }
            TransformKind::Rotate => {
                let total = if snap {
                    if self.view.options.snap_target == SnapTarget::Vertex {
                        snap_rotation_to_edges(document, self.applied.x(), raw.x())
                    } else {
                        let step = self.view.options.rotation_step.max(0.1);
                        (raw.x() / step).round() * step
                    }
                } else {
                    raw.x()
                };
                let step = total - self.applied.x();
                if step == 0.0 {
                    return false;
                }
                self.applied.v[0] = total;
                uv_workflow::transform(
                    document,
                    uv_workflow::Transform::Rotate(step),
                    self.view.options.pivot,
                    origin,
                );
            }
            TransformKind::Scale => {
                let mut total = raw;
                if snap {
                    let from =
                        self.drag_view
                            .unproject(document, height, self.start.0, self.start.1);
                    let started = from - origin;
                    let target = origin + Vec2f::new(started.x() * raw.x(), started.y() * raw.y());
                    let target = self.snap_point(document, height, target, false) - origin;
                    for k in 0..2 {
                        if started[k].abs() > 1e-4 {
                            total.v[k] = target[k] / started[k];
                        }
                    }
                }
                for (k, &enabled) in axes.iter().enumerate() {
                    if !enabled {
                        total.v[k] = 1.0;
                    }
                    if !total[k].is_finite() || total[k].abs() < 1e-4 {
                        return false;
                    }
                }
                let previous = self.applied;
                let step = Vec2f::new(total.x() / previous.x(), total.y() / previous.y());
                if step == Vec2f::new(1.0, 1.0) {
                    return false;
                }
                self.applied = total;
                uv_workflow::transform(
                    document,
                    uv_workflow::Transform::Scale(step),
                    self.view.options.pivot,
                    origin,
                );
            }
        }
        true
    }

    /// Ctrl/Cmd-drag anywhere rotates; chosen only when a gesture begins.
    pub fn rotate_drag_begin(&mut self, document: &mut Document, height: f64, x: f64, y: f64) {
        self.drag_begin(document, height, x, y, false);
        if self.drag_origin.is_some() {
            if !self.transaction {
                document.begin_transaction();
                self.transaction = true;
            }
            self.grip = Some(Grip::Rotate);
        }
    }

    fn stripe(&self, document: &Document) -> Vec2f {
        grid_stripe(document, self.view.size.1, self.view)
    }

    fn grid_grip(&self, document: &Document, height: f64, x: f64, y: f64) -> Option<Grip> {
        if !self.view.options.show_grid || self.view.options.opacity < 0.01 {
            return None;
        }
        let uv = self.view.unproject(document, height, x, y);
        let stripe = self.stripe(document);
        let axes = self.view.axes_pixels(document, height);
        let det = (axes[0][0] * axes[1][1] - axes[0][1] * axes[1][0]).abs();
        let near = |k: usize| {
            let distance = (uv[k] - (uv[k] / stripe[k]).round() * stripe[k]).abs() as f64;
            distance * det / axes[0][1 - k].hypot(axes[1][1 - k]).max(1e-9) <= SHEAR_GRIP_DISTANCE
        };
        let (horizontally, vertically) = (near(0), near(1));
        (horizontally || vertically).then_some(Grip::Scale {
            horizontally,
            vertically,
        })
    }
    fn snap_point(&self, document: &Document, height: f64, raw: Vec2f, origin: bool) -> Vec2f {
        match self.view.options.snap_target {
            SnapTarget::Pixel => return Vec2f::new(raw.x().round(), raw.y().round()),
            SnapTarget::Grid => {
                let stripe = self.stripe(document);
                return Vec2f::new(
                    (raw.x() / stripe.x()).round() * stripe.x(),
                    (raw.y() / stripe.y()).round() * stripe.y(),
                );
            }
            SnapTarget::Vertex => {}
        }
        let Some((_, face)) = document.face_to_edit() else {
            return raw;
        };
        // Convert current corners to the UV frame captured at the beginning of the drag.
        let mut points: Vec<_> = document
            .face_to_edit_polygon()
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                let uv = face.uv_coords(p);
                let (x, y) = self.view.project(document, height, uv);
                self.drag_view.unproject(document, height, x, y)
            })
            .collect();
        if origin && !points.is_empty() {
            points.push(
                points.iter().copied().fold(Vec2f::zero(), |a, b| a + b) / points.len() as f32,
            );
            let stripe = self.stripe(document);
            points.push(Vec2f::new(
                (raw.x() / stripe.x()).round() * stripe.x(),
                (raw.y() / stripe.y()).round() * stripe.y(),
            ));
        }
        let axes = self.drag_view.axes_pixels(document, height);
        let mut result = raw;
        for k in 0..2 {
            if let Some(value) = points
                .iter()
                .map(|p| p[k])
                .min_by(|a, b| (a - raw[k]).abs().total_cmp(&(b - raw[k]).abs()))
            {
                if f64::from((value - raw[k]).abs()) * axes[0][k].hypot(axes[1][k]) <= 8.0 {
                    result.v[k] = value;
                }
            }
        }
        result
    }
    fn snap_offset(&self, document: &Document, height: f64, raw: Vec2f) -> Vec2f {
        match self.view.options.snap_target {
            SnapTarget::Pixel => return Vec2f::new(raw.x().round(), raw.y().round()),
            SnapTarget::Grid => {
                let stripe = self.stripe(document);
                return Vec2f::new(
                    (raw.x() / stripe.x()).round() * stripe.x(),
                    (raw.y() / stripe.y()).round() * stripe.y(),
                );
            }
            SnapTarget::Vertex => {}
        }
        let Some((_, face)) = document.face_to_edit() else {
            return raw;
        };
        let stripe = self.stripe(document);
        let axes = self.drag_view.axes_pixels(document, height);
        let mut best = Vec2f::new(f32::INFINITY, f32::INFINITY);
        for p in document.face_to_edit_polygon().unwrap_or_default() {
            let uv = face.uv_coords(p) + self.applied - raw;
            for k in 0..2 {
                let correction = uv[k] - (uv[k] / stripe[k]).round() * stripe[k];
                if correction.abs() < best[k].abs() {
                    best.v[k] = correction;
                }
            }
        }
        let mut result = Vec2f::new(raw.x().round(), raw.y().round());
        for k in 0..2 {
            if f64::from(best[k].abs()) * axes[0][k].hypot(axes[1][k]) <= 8.0 {
                result.v[k] = raw[k] + best[k];
            }
        }
        result
    }

    /// Which grid line the pointer can scale, for cursor feedback.
    pub fn grid_handle(&self, document: &Document, height: f64, x: f64, y: f64) -> Option<Grip> {
        let at = self.pivot(document)?;
        let origin = self.view.project(document, height, at);
        if grip_at(origin, x, y) != Grip::Offset
            || origin_axis_grip(origin, self.view.axes_pixels(document, height), x, y).is_some()
        {
            return None;
        }
        let Grip::Scale {
            mut horizontally,
            mut vertically,
        } = self.grid_grip(document, height, x, y)?
        else {
            return None;
        };
        let hit = self.view.unproject(document, height, x, y);
        let stripe = self.stripe(document);
        horizontally &= ((hit.x() / stripe.x()).round() * stripe.x() - at.x()).abs() > 1e-3;
        vertically &= ((hit.y() / stripe.y()).round() * stripe.y() - at.y()).abs() > 1e-3;
        (horizontally || vertically).then_some(Grip::Scale {
            horizontally,
            vertically,
        })
    }

    /// Highlights exactly the line that can be grabbed, retaining it during a drag.
    pub fn hover(&mut self, document: &Document, height: f64, pointer: Option<(f64, f64)>) {
        self.view.highlight = None;
        if let (
            Some(Grip::Scale {
                horizontally,
                vertically,
            }),
            Some(hit),
        ) = (self.grip, self.grid_hit)
        {
            self.view.highlight = Some((hit, horizontally, vertically));
            return;
        }
        if let Some((x, y)) = pointer {
            if let Some(Grip::Scale {
                horizontally,
                vertically,
            }) = self.grid_handle(document, height, x, y)
            {
                let hit = self.view.unproject(document, height, x, y);
                let stripe = self.stripe(document);
                self.view.highlight = Some((
                    Vec2f::new(
                        (hit.x() / stripe.x()).round() * stripe.x(),
                        (hit.y() / stripe.y()).round() * stripe.y(),
                    ),
                    horizontally,
                    vertically,
                ));
            }
        }
    }

    /// The drag ended: what it did becomes one step of the history.
    pub fn drag_end(&mut self, document: &mut Document) {
        self.grip = None;
        self.view.grid = None;
        self.view.highlight = None;
        if std::mem::take(&mut self.transaction) {
            document.commit_transaction();
        }
    }

    /// The drag was cancelled: what it did is taken back. Returns whether the document
    /// changed.
    pub fn drag_cancel(&mut self, document: &mut Document) -> bool {
        self.grip = None;
        self.view.grid = None;
        self.view.highlight = None;
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
        self.view = UvView {
            divisions: self.view.divisions,
            options: self.view.options,
            ..UvView::default()
        };
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
    (
        translation_key("Align"),
        Action::Align,
        translation_key(
            "Turn the material along one of the face's edges; press again for the next edge, Shift for the last",
        ),
    ),
    (
        "◧",
        Action::Justify(UvAxis::U, UvSign::Minus),
        translation_key("Push the material against the face's left edge"),
    ),
    (
        "◨",
        Action::Justify(UvAxis::U, UvSign::Plus),
        translation_key("Push it against the right edge"),
    ),
    (
        "⬒",
        Action::Justify(UvAxis::V, UvSign::Minus),
        translation_key("Push it against the top edge"),
    ),
    (
        "⬓",
        Action::Justify(UvAxis::V, UvSign::Plus),
        translation_key("Push it against the bottom edge"),
    ),
    (
        translation_key("Fit U"),
        Action::FitAxis(UvAxis::U),
        translation_key(
            "Scale across so a whole number of copies covers the face; press again for the next count, Shift for the last, Ctrl to keep a trim sheet's subdivision",
        ),
    ),
    (
        translation_key("Fit V"),
        Action::FitAxis(UvAxis::V),
        translation_key("The same down the face"),
    ),
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

    /// What the edited face has in this field. A face with no surface attributes of its
    /// own shows the ones its texture gives it, which is what the compiler will use.
    pub fn value(self, document: &Document, face: &map_model::brush_face::BrushFace) -> f64 {
        let attributes = face.uv_attributes;
        let surface = surface_of(document, face);
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
    /// the lot, which is what the format writes, starting from what its texture gives it:
    /// once a face line carries flags, `qbsp` no longer reads the `.wal`'s, so a sky face
    /// given a light value from zeroes would stop being sky.
    pub fn set(self, document: &mut Document, value: f64) {
        let defaults = surface_defaults(document);
        document.edit_faces(|face| self.apply(face, value, &defaults));
    }

    /// Sets the field on one face. `defaults` are the surface attributes each material's
    /// texture gives it, by material name.
    fn apply(
        self,
        face: &mut map_model::brush_face::BrushFace,
        value: f64,
        defaults: &HashMap<String, map_model::uv::SurfaceAttributes>,
    ) {
        // Reject invalid edits here so both numeric controls and scripted edits keep
        // the previous mapping. Negative scales remain valid for mirroring.
        if !value.is_finite()
            || !(value as f32).is_finite()
            || (matches!(self, Field::ScaleX | Field::ScaleY) && value as f32 == 0.0)
        {
            return;
        }
        let offset = face.uv_attributes.offset;
        let scale = face.uv_attributes.scale;
        let mut surface = face
            .surface_attributes
            .or_else(|| defaults.get(face.material_name.as_ref()).copied())
            .unwrap_or_default();
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
    }
}

/// Sets several fields on the given faces, selected or not, as one undo step: what the
/// panel's fields do one at a time, for MCP's `set_face_attributes`. Returns how many
/// faces changed.
pub fn set_on(
    document: &mut Document,
    faces: &[crate::document::FaceRef],
    values: &[(Field, f64)],
) -> usize {
    let defaults = surface_defaults(document);
    document.edit_faces_on(faces, |face| {
        let before = (face.uv_attributes, face.surface_attributes);
        for (field, value) in values {
            field.apply(face, *value, &defaults);
        }
        before != (face.uv_attributes, face.surface_attributes)
    })
}

/// The surface attributes each loaded material's texture gives a face, by name.
fn surface_defaults(document: &Document) -> HashMap<String, map_model::uv::SurfaceAttributes> {
    document
        .materials()
        .materials()
        .iter()
        .filter_map(|m| Some((m.name.clone(), m.surface_defaults?)))
        .collect()
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

/// The surface attributes a face carries: its own, or else the ones its texture's header
/// gives it, or none.
pub fn surface_of(
    document: &Document,
    face: &map_model::brush_face::BrushFace,
) -> map_model::uv::SurfaceAttributes {
    face.surface_attributes
        .or_else(|| {
            document
                .materials()
                .get(&face.material_name)
                .and_then(|material| material.surface_defaults)
        })
        .unwrap_or_default()
}

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

fn origin_axis_grip(origin: (f64, f64), axes: [[f64; 2]; 2], x: f64, y: f64) -> Option<Grip> {
    let distance = |k: usize| {
        ((x - origin.0) * axes[1][k] - (y - origin.1) * axes[0][k]).abs()
            / axes[0][k].hypot(axes[1][k]).max(1e-9)
    };
    if distance(0) <= 5.0 {
        Some(Grip::OriginAxis {
            horizontally: true,
            vertically: false,
        })
    } else if distance(1) <= 5.0 {
        Some(Grip::OriginAxis {
            horizontally: false,
            vertically: true,
        })
    } else {
        None
    }
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
    if ((x - origin.0).hypot(y - origin.1) - ROTATE_RADIUS).abs() <= HANDLE_RADIUS + 2.0 {
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
    snap_shear_for_face(face, &polygon, factors, offset, zoom)
}

fn snap_shear_for_face(
    face: &map_model::brush_face::BrushFace,
    polygon: &[Vec3d],
    factors: Vec2f,
    offset: Vec2f,
    zoom: f32,
) -> Vec2f {
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
    if !m.iter().flatten().all(|value| value.is_finite()) || !det.is_finite() || det.abs() < 1e-12 {
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

/// Visible editing lines remain within reach even when a texture repeat is
/// larger than the viewport. Picking and snapping use these same subdivisions.
fn grid_stripe(document: &Document, height: f64, view: UvView) -> Vec2f {
    if let Some(stripe) = view.grid {
        return stripe;
    }
    let (w, h) = material_size(document);
    let axes = view.axes_pixels(document, height.max(1.0));
    let det = (axes[0][0] * axes[1][1] - axes[0][1] * axes[1][0]).abs();
    let mut stripe = Vec2f::new(
        (w / view.divisions[0].max(1) as f64) as f32,
        (h / view.divisions[1].max(1) as f64) as f32,
    );
    if view.options.grid == GridMode::Pixel {
        stripe = Vec2f::new(1.0, 1.0);
    }
    if view.options.grid == GridMode::Fixed {
        return stripe;
    }
    for k in 0..2 {
        let pixels_per_texel = det / axes[0][1 - k].hypot(axes[1][1 - k]).max(1e-12);
        for _ in 0..40 {
            let gap = f64::from(stripe[k]) * pixels_per_texel;
            if gap > 80.0 && view.options.grid == GridMode::Dynamic {
                stripe.v[k] *= 0.5;
            } else if gap < 16.0 {
                stripe.v[k] *= 2.0;
            } else {
                break;
            }
        }
    }
    stripe
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
            cr.paint_tiled_image_affine(
                &face.material_name,
                material.width,
                material.height,
                &pixels[..(material.width * material.height * 4) as usize],
                view.pan,
                view.axes_pixels(document, height),
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

    // Draw the UV grid in the stable face-plane view, including trim subdivisions.
    let stripe = grid_stripe(document, height, view);
    let stripe = [f64::from(stripe.x()), f64::from(stripe.y())];
    let corners = [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)]
        .map(|(x, y)| view.unproject(document, height, x, y));
    for k in 0..2 {
        let lo = corners
            .iter()
            .map(|p| p[k] as f64)
            .fold(f64::INFINITY, f64::min);
        let hi = corners
            .iter()
            .map(|p| p[k] as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let other_lo = corners
            .iter()
            .map(|p| p[1 - k])
            .fold(f32::INFINITY, f32::min);
        let other_hi = corners
            .iter()
            .map(|p| p[1 - k])
            .fold(f32::NEG_INFINITY, f32::max);
        let first = (lo / stripe[k]).floor();
        let count = ((hi - lo) / stripe[k]).ceil() as usize + 2;
        if count > 2000 || !view.options.show_grid || view.options.opacity < 0.01 {
            continue;
        }
        for n in 0..count {
            let mut a = Vec2f::zero();
            let mut b = Vec2f::zero();
            a.v[k] = ((first + n as f64) * stripe[k]) as f32;
            b.v[k] = a[k];
            a.v[1 - k] = other_lo;
            b.v[1 - k] = other_hi;
            let a = view.project(document, height, a);
            let b = view.project(document, height, b);
            let coordinate = ((first + n as f64) * stripe[k]) as f32;
            let highlighted = view
                .highlight
                .is_some_and(|(hit, u, v)| [u, v][k] && (hit[k] - coordinate).abs() < 0.01);
            // Dark halo plus an opaque light stroke remain visible on both bright
            // and dark textures. Cyan identifies the actual draggable line.
            cr.set_source_rgba(0.0, 0.0, 0.0, view.options.opacity);
            cr.set_line_width(if highlighted { 5.0 } else { 3.0 });
            cr.move_to(a.0, a.1);
            cr.line_to(b.0, b.1);
            cr.stroke();
            if highlighted {
                cr.set_source_rgb(0.15, 0.9, 1.0);
            } else {
                cr.set_source_rgba(0.88, 0.9, 0.93, view.options.opacity);
            }
            cr.set_line_width(if highlighted { 2.5 } else { 1.25 });
            cr.move_to(a.0, a.1);
            cr.line_to(b.0, b.1);
            cr.stroke();
        }
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
        let (x, y) = view.project(document, height, uv);
        if index == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.close_path();
    if view.options.stretch {
        if let Some(metrics) = uv_workflow::metrics(face) {
            let ratio = if view.options.area_stretch {
                let r = metrics.density / view.options.density.max(1e-4);
                r.max(1.0 / r)
            } else {
                metrics.stretch
            };
            let t = (ratio - 1.0).clamp(0.0, 1.0);
            cr.set_source_rgba(
                0.1 + 0.85 * t,
                0.45 * (1.0 - t) + 0.1 * t,
                0.95 * (1.0 - t) + 0.05 * t,
                view.options.opacity * 0.45,
            );
            cr.fill_preserve();
        }
    }
    if view.options.show_outline {
        cr.set_source_rgba(1.0, 0.65, 0.2, view.options.opacity);
        cr.set_line_width(1.5);
        cr.stroke();
    } else {
        cr.new_path();
    }
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

    fn one_face() -> Document {
        let mut d = cube_with_a_selected_face();
        let face = d.face_to_edit().unwrap().0;
        d.select_face_only(face);
        d.edit_faces(|f| {
            f.uv_attributes.scale = Vec2f::new(1.0, 1.0);
            f.uv_attributes.offset = Vec2f::new(13.0, 19.0);
        });
        d
    }

    #[test]
    fn large_texture_has_visible_highlighted_scale_lines_that_really_scale() {
        let mut d = one_face();
        let directory =
            std::env::temp_dir().join(format!("heron-uv-visible-grid-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("bright.png"),
            crate::assets::encode_png(512, 512, &vec![255; 512 * 512 * 4]).unwrap(),
        )
        .unwrap();
        d.load_materials(&directory).unwrap();
        d.edit_faces(|f| {
            f.material_name = "bright".into();
            f.uv_attributes.scale = Vec2f::new(0.25, 0.25);
        });
        let mut canvas = UvCanvas::default();
        canvas.prepare(&d, 316.0, 160.0);
        let stripe = canvas.stripe(&d);
        assert!(
            stripe.x() < 512.0,
            "a whole tile is too large to give reachable scale lines"
        );
        let mut picked = None;
        for n in -5..=5 {
            for y in [12.0, 25.0, 40.0, 60.0, 100.0, 125.0, 145.0] {
                let v = canvas.view.unproject(&d, 160.0, 0.0, y).y();
                let (x, y) = canvas
                    .view
                    .project(&d, 160.0, Vec2f::new(n as f32 * stripe.x(), v));
                if x > 10.0
                    && x < 306.0
                    && canvas.grid_handle(&d, 160.0, x, y)
                        == Some(Grip::Scale {
                            horizontally: true,
                            vertically: false,
                        })
                {
                    picked = Some((x, y));
                    break;
                }
            }
            if picked.is_some() {
                break;
            }
        }
        let (x, y) = picked.expect("reachable single-axis scale line inside small UV panel");
        canvas.hover(&d, 160.0, Some((x, y)));
        assert!(canvas.view.highlight.is_some());
        let mut raster = Raster::new(316, 160).unwrap();
        canvas.draw(&d, &mut raster, 316.0, 160.0);
        let image = raster.into_rgba();
        let at = ((y as usize) * 316 + x as usize) * 4;
        let pixel = &image.pixels[at..at + 4];
        assert!(
            pixel[0] < 150 && pixel[1] > 150 && pixel[2] > 180,
            "line must be visibly cyan even over a white texture: {pixel:?}"
        );
        let before = d.face_to_edit().unwrap().1.uv_attributes.scale;
        canvas.drag_begin(&mut d, 160.0, x, y, false);
        assert!(canvas.drag_update(&mut d, 160.0, 15.0, 0.0, true));
        canvas.prepare(&d, 316.0, 160.0);
        assert_eq!(
            canvas.stripe(&d),
            stripe,
            "grid subdivision must not jump during scale"
        );
        canvas.drag_end(&mut d);
        let after = d.face_to_edit().unwrap().1.uv_attributes.scale;
        assert_ne!(before.x(), after.x());
        assert_eq!(before.y(), after.y());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn affine_raster_texture_preserves_rotation_and_repeating_colors() {
        let mut raster = Raster::new(16, 16).unwrap();
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        raster.paint_tiled_image_affine(
            "corners",
            2,
            2,
            &pixels,
            (8.0, 0.0),
            [[0.0, -4.0], [4.0, 0.0]],
        );
        let image = raster.into_rgba();
        let pixel = |x: usize, y: usize| &image.pixels[(y * 16 + x) * 4..(y * 16 + x + 1) * 4];
        assert_eq!(pixel(6, 2), &[255, 0, 0, 255]);
        assert_eq!(pixel(6, 6), &[0, 255, 0, 255]);
        assert_eq!(pixel(2, 2), &[0, 0, 255, 255]);
        assert_eq!(pixel(14, 10), pixel(6, 2));
    }

    #[test]
    fn fresh_face_view_is_independent_of_texture_scale_and_rotation() {
        let mut d = one_face();
        let mut before = UvView::default();
        before.sync(&d, 500.0, 260.0);
        let polygon = d.face_to_edit_polygon().unwrap();
        let expected: Vec<_> = polygon
            .iter()
            .map(|p| before.project(&d, 260.0, d.face_to_edit().unwrap().1.uv_coords(*p)))
            .collect();
        d.edit_faces(|f| {
            f.scale_uv(Vec2f::new(2.0, 0.6));
            f.rotate_uv(37.0);
        });
        let mut after = UvView::default();
        after.sync(&d, 500.0, 260.0);
        for (p, expected) in polygon.iter().zip(expected) {
            let actual = after.project(&d, 260.0, d.face_to_edit().unwrap().1.uv_coords(*p));
            assert!((actual.0 - expected.0).abs() < 0.003 && (actual.1 - expected.1).abs() < 0.003);
        }
    }

    #[test]
    fn face_outline_stays_put_through_mapping_changes_material_sizes_and_undo() {
        let mut d = one_face();
        d.edit_faces(|f| {
            let (u_axis, v_axis) = f.uv_coord_system.axes(f.normal(), &f.uv_attributes);
            f.uv_coord_system = map_model::uv::UvCoordSystem::Parallel { u_axis, v_axis };
        });
        let mut view = UvView::default();
        view.sync(&d, 500.0, 260.0);
        let positions = |d: &Document, v: UvView| {
            d.face_to_edit_polygon()
                .unwrap()
                .iter()
                .map(|p| v.project(d, 260.0, d.face_to_edit().unwrap().1.uv_coords(*p)))
                .collect::<Vec<_>>()
        };
        let expected = positions(&d, view);
        let check = |d: &Document, v: UvView| {
            for (a, b) in expected.iter().zip(positions(d, v)) {
                assert!(
                    (a.0 - b.0).abs() < 0.003 && (a.1 - b.1).abs() < 0.003,
                    "{a:?} != {b:?}"
                );
            }
        };
        for operation in [
            (|f: &mut map_model::brush_face::BrushFace| {
                f.scale_uv_about(Vec2f::new(4.0, 7.0), Vec2f::new(1.7, 0.6))
            }) as fn(&mut map_model::brush_face::BrushFace),
            |f| f.rotate_uv_about(Vec2f::new(4.0, 7.0), 37.0),
            |f| f.skew_uv_about(Vec2f::new(4.0, 7.0), [[1.0, 0.25], [0.15, 1.0]]),
            |f| f.move_uv(Vec2f::new(17.0, -39.0)),
            |f| f.flip_uv(true),
        ] {
            d.edit_faces(operation);
            view.sync(&d, 500.0, 260.0);
            check(&d, view);
            d.undo();
            view.sync(&d, 500.0, 260.0);
            check(&d, view);
            d.redo();
            view.sync(&d, 500.0, 260.0);
            check(&d, view);
        }
        // Changing to a texture with a different height must preserve camera scale.
        let directory =
            std::env::temp_dir().join(format!("heron-uv-camera-material-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("tall.png"),
            crate::assets::encode_png(16, 512, &vec![255; 16 * 512 * 4]).unwrap(),
        )
        .unwrap();
        d.load_materials(&directory).unwrap();
        let _ = std::fs::remove_dir_all(&directory);
        d.edit_faces(|f| f.material_name = "tall".into());
        view.sync(&d, 500.0, 260.0);
        check(&d, view);
        let before = view.project(&d, 260.0, Vec2f::zero());
        view.sync(&d, 540.0, 300.0);
        let after = view.project(&d, 300.0, Vec2f::zero());
        assert!((after.0 - before.0 - 20.0).abs() < 0.003);
        assert!((after.1 - before.1 - 20.0).abs() < 0.003);
    }

    #[test]
    fn grid_line_scales_one_axis_snaps_to_corner_and_is_one_undo() {
        let mut d = one_face();
        let before = d.to_map_text();
        let mut canvas = UvCanvas::default();
        let mut raster = Raster::new(500, 260).unwrap();
        canvas.draw(&d, &mut raster, 500.0, 260.0);
        let (_, face) = d.face_to_edit().unwrap();
        let corners: Vec<_> = d
            .face_to_edit_polygon()
            .unwrap()
            .iter()
            .map(|p| face.uv_coords(*p))
            .collect();
        let hi = corners
            .iter()
            .map(|p| p.x())
            .fold(f32::NEG_INFINITY, f32::max);
        let lo = corners.iter().map(|p| p.x()).fold(f32::INFINITY, f32::min);
        let origin = Vec2f::new(lo - 64.0, -79.0);
        canvas.origin = Some(origin);
        let start = Vec2f::new(0.0, canvas.stripe(&d).y() * 0.4);
        let (x, y) = canvas.view.project(&d, 260.0, start);
        let scale = canvas_scale(&d, 260.0 * canvas.view.zoom);
        canvas.drag_begin(&mut d, 260.0, x, y, false);
        assert_eq!(
            canvas.grip,
            Some(Grip::Scale {
                horizontally: true,
                vertically: false
            })
        );
        canvas.drag_update(&mut d, 260.0, (hi as f64) * scale + 2.0, 0.0, false);
        canvas.draw(&d, &mut raster, 500.0, 260.0);
        // Same pointer position after a paint is stable, rather than accumulating scale.
        canvas.drag_update(&mut d, 260.0, (hi as f64) * scale + 2.0, 0.0, false);
        canvas.drag_end(&mut d);
        let face = d.face_to_edit().unwrap().1;
        let expected = (hi - origin.x()) / (start.x() - origin.x());
        assert!(
            (face.uv_attributes.scale.x() - expected).abs() < 1e-4,
            "{:?} expected {expected}",
            face.uv_attributes.scale
        );
        assert_eq!(face.uv_attributes.scale.y(), 1.0);
        let after = d.to_map_text();
        assert_ne!(before, after);
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.to_map_text(), after);
    }

    #[test]
    fn trim_grid_can_be_picked_and_pivot_moves_without_editing_the_map() {
        let mut d = one_face();
        let before = d.to_map_text();
        let mut canvas = UvCanvas::default();
        canvas.view.divisions = [4, 2];
        let mut raster = Raster::new(500, 260).unwrap();
        canvas.draw(&d, &mut raster, 500.0, 260.0);
        let stripe = canvas.stripe(&d);
        let p = canvas
            .view
            .project(&d, 260.0, Vec2f::new(stripe.x(), stripe.y() * 0.33));
        assert!(canvas.grid_grip(&d, 260.0, p.0, p.1).is_some());
        let origin = handle_origin(&d, None).unwrap();
        let p = canvas.view.project(&d, 260.0, origin);
        canvas.drag_begin(&mut d, 260.0, p.0, p.1, false);
        assert_eq!(canvas.grip, Some(Grip::Origin));
        canvas.drag_update(&mut d, 260.0, 25.0, 15.0, true);
        canvas.drag_end(&mut d);
        assert_ne!(canvas.origin, Some(origin));
        assert_eq!(d.to_map_text(), before);
        canvas.reset_view();
        assert_eq!(canvas.view.divisions, [4, 2]);
    }

    #[test]
    fn origin_axis_drag_changes_only_one_component_and_has_no_history() {
        let mut d = one_face();
        let before = d.to_map_text();
        let mut canvas = UvCanvas::default();
        canvas.prepare(&d, 500.0, 260.0);
        let origin = handle_origin(&d, None).unwrap();
        canvas.origin = Some(origin);
        let p = canvas.view.project(&d, 260.0, origin);
        canvas.drag_begin(&mut d, 260.0, p.0 + 123.0, p.1, false);
        assert_eq!(
            canvas.grip,
            Some(Grip::OriginAxis {
                horizontally: true,
                vertically: false
            })
        );
        canvas.drag_update(&mut d, 260.0, 27.0, 19.0, true);
        canvas.drag_end(&mut d);
        assert_eq!(canvas.origin.unwrap().y(), origin.y());
        assert_ne!(canvas.origin.unwrap().x(), origin.x());
        assert_eq!(d.to_map_text(), before);
    }

    #[test]
    fn scale_snap_can_be_disabled_with_control() {
        let mut d = one_face();
        let mut canvas = UvCanvas::default();
        canvas.prepare(&d, 500.0, 260.0);
        canvas.origin = Some(Vec2f::new(-100.0, -79.0));
        let p = canvas
            .view
            .project(&d, 260.0, Vec2f::new(0.0, canvas.stripe(&d).y() * 0.4));
        let edge = d
            .face_to_edit_polygon()
            .unwrap()
            .iter()
            .map(|p| d.face_to_edit().unwrap().1.uv_coords(*p).x())
            .fold(f32::NEG_INFINITY, f32::max);
        let scale = canvas_scale(&d, 260.0 * canvas.view.zoom);
        canvas.drag_begin(&mut d, 260.0, p.0, p.1, false);
        canvas.drag_update(&mut d, 260.0, f64::from(edge) * scale + 2.0, 0.0, true);
        let expected = (edge + 2.0 / scale as f32 + 100.0) / 100.0;
        assert!((d.face_to_edit().unwrap().1.uv_attributes.scale.x() - expected).abs() < 1e-4);
        canvas.drag_cancel(&mut d);
    }

    #[test]
    fn shear_is_disabled_for_standard_projection() {
        let mut d = one_face();
        let before = d.to_map_text();
        let mut canvas = UvCanvas::default();
        let mut raster = Raster::new(500, 260).unwrap();
        canvas.draw(&d, &mut raster, 500.0, 260.0);
        let stripe = canvas.stripe(&d);
        let p = canvas
            .view
            .project(&d, 260.0, Vec2f::new(stripe.x(), stripe.y() * 0.4));
        canvas.drag_begin(&mut d, 260.0, p.0, p.1, true);
        assert_eq!(canvas.grip, Some(Grip::Nothing));
        assert!(!canvas.drag_update(&mut d, 260.0, 20.0, 15.0, false));
        canvas.drag_end(&mut d);
        assert_eq!(d.to_map_text(), before);
    }

    #[test]
    fn scale_fields_reject_zero_and_nonfinite_edits_without_history_changes() {
        let mut document = cube_with_a_selected_face();
        Field::ScaleX.set(&mut document, -0.5);
        Field::ScaleY.set(&mut document, 0.75);
        let before = document.to_map_text();
        let revision = document.revision();
        for value in [
            0.0,
            -0.0,
            f64::INFINITY,
            f64::NAN,
            f64::MAX,
            f64::MIN_POSITIVE,
        ] {
            Field::ScaleX.set(&mut document, value);
            Field::ScaleY.set(&mut document, value);
        }
        assert_eq!(document.to_map_text(), before);
        assert_eq!(document.revision(), revision);
    }

    #[test]
    fn invalid_shear_steps_are_not_invertible() {
        for matrix in [
            [[1.0, 1.0], [1.0, 1.0]],
            [[1.0, f64::INFINITY], [0.0, 1.0]],
            [[1.0, f64::NAN], [0.0, 1.0]],
        ] {
            assert!(invert_2x2(matrix).is_none());
        }
    }

    /// A face on a `.wal` that is sky by its header shows the sky flag, and giving it a
    /// light value keeps the flag: once the face line carries flags `qbsp` stops reading
    /// the texture's, so starting from zeroes would quietly turn the sky into a wall.
    #[test]
    fn surface_fields_start_from_the_texture_s_own_flags() {
        let root = std::env::temp_dir().join(format!("heron-uv-wal-flags-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("pics")).expect("pics");
        std::fs::create_dir_all(root.join("textures/e1")).expect("textures");
        let mut pcx = vec![0; 128];
        pcx[..4].copy_from_slice(&[10, 5, 1, 8]);
        pcx[8] = 1;
        pcx[65] = 1;
        pcx[66] = 2;
        pcx.extend_from_slice(&[1, 0xc1, 255, 12]);
        pcx.extend_from_slice(&[0; 768]);
        std::fs::write(root.join("pics/colormap.pcx"), pcx).expect("palette");
        let mut wal = vec![0; 100];
        wal[32..36].copy_from_slice(&8u32.to_le_bytes());
        wal[36..40].copy_from_slice(&8u32.to_le_bytes());
        for level in 0..4 {
            let offset = wal.len() as u32;
            wal[40 + level * 4..44 + level * 4].copy_from_slice(&offset.to_le_bytes());
            wal.extend(vec![1; (8 >> level) * (8 >> level)]);
        }
        wal[88..92].copy_from_slice(&4i32.to_le_bytes());
        std::fs::write(root.join("textures/e1/sky.wal"), wal).expect("wal");

        let mut document = Document::new(WorldBounds::default());
        document
            .load_materials(root.join("textures"))
            .expect("materials");
        let world = document.world_bounds();
        let brush =
            map_model::shapes::create_cuboid(&world, &Bbox3d::from_scalars(-32.0, 32.0), "e1/sky")
                .expect("a cube");
        document.add_brushes(vec![brush]);
        document.select_all();

        let face = document.face_to_edit().expect("a face").1.clone();
        assert_eq!(Field::SurfaceFlags.value(&document, &face), 4.0);
        Field::SurfaceValue.set(&mut document, 300.0);
        let face = document.face_to_edit().expect("a face").1;
        assert_eq!(
            face.surface_attributes,
            Some(map_model::uv::SurfaceAttributes {
                content_flags: 0,
                surface_flags: 4,
                value: 300,
                color: None,
            })
        );
        let _ = std::fs::remove_dir_all(&root);
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
        assert!((view.zoom * view.base - 0.02).abs() < 1e-12);
        view.zoom_at(0.0, 0.0, -1e6);
        assert!((view.zoom * view.base - 64.0).abs() < 1e-12);
    }

    #[test]
    fn offset_moves_texture_and_keeps_outline_fixed_including_undo() {
        let mut document = cube_with_a_selected_face();
        let mut view = UvView::default();
        view.sync(&document, 400.0, 200.0);
        let point = document.face_to_edit_polygon().unwrap()[0];
        let position = |d: &Document, v: UvView| {
            let uv = d.face_to_edit().unwrap().1.uv_coords(point);
            v.project(d, 200.0, uv)
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
    #[test]
    fn selectable_pixel_grid_and_rotation_snapping_apply_and_cancel_cleanly() {
        use crate::uv_workflow::{SnapTarget, TransformKind};
        for mode in [SnapTarget::Pixel, SnapTarget::Grid] {
            let mut d = cube_with_a_selected_face();
            let before = d.to_map_text();
            let mut canvas = UvCanvas::default();
            canvas.prepare(&d, 320.0, 240.0);
            canvas.view.options.snap_target = mode;
            let stripe = canvas.stripe(&d);
            let raw = if mode == SnapTarget::Pixel {
                Vec2f::new(1.2, 3.7)
            } else {
                stripe * 0.6
            };
            let expected = if mode == SnapTarget::Pixel {
                Vec2f::new(1.0, 4.0)
            } else {
                stripe
            };
            let offset = d.face_to_edit().unwrap().1.uv_attributes.offset;
            canvas.begin_transform(&mut d, TransformKind::Move, 240.0, 100.0, 100.0);
            canvas.preview_transform(&mut d, TransformKind::Move, raw, [true, true], false);
            let actual = d.face_to_edit().unwrap().1.uv_attributes.offset;
            assert!(geometry_math::vec::length(actual - (offset - expected)) < 0.001);
            canvas.drag_cancel(&mut d);
            assert_eq!(before, d.to_map_text());
            canvas.view.options.snap = false;
            canvas.begin_transform(&mut d, TransformKind::Move, 240.0, 100.0, 100.0);
            canvas.preview_transform(&mut d, TransformKind::Move, raw, [true, true], false);
            assert!(
                geometry_math::vec::length(
                    d.face_to_edit().unwrap().1.uv_attributes.offset - (offset - raw)
                ) < 0.001
            );
            canvas.drag_cancel(&mut d);
            canvas.view.options.snap = true;
            canvas.view.options.snap_target = SnapTarget::Grid;
            canvas.view.options.rotation_step = 15.0;
            let rotation = d.face_to_edit().unwrap().1.uv_attributes.rotation;
            canvas.begin_transform(&mut d, TransformKind::Rotate, 240.0, 100.0, 100.0);
            canvas.preview_transform(
                &mut d,
                TransformKind::Rotate,
                Vec2f::new(22.0, 0.0),
                [true, true],
                false,
            );
            assert!(
                (wrap_180(d.face_to_edit().unwrap().1.uv_attributes.rotation - rotation - 15.0))
                    .abs()
                    < 0.001
            );
            canvas.drag_cancel(&mut d);
            assert_eq!(before, d.to_map_text());
        }
    }
}
