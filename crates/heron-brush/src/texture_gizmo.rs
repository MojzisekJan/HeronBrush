//! Texture handles on the face being edited in the 3D view, for a face that wears its
//! material rather than a paint. One tile of the material is outlined on the face (or a
//! block of tiles, or part of one, so it stays a usable size on the screen), with a
//! handle at its corner that moves the material, one at the next corner along U that
//! turns it about that corner, and three that scale it: the far corner, and the middles
//! of the far edges for one axis.
//!
//! Moving and scaling snap the handle to the face's corners, the middles of its edges,
//! its edges and its centre; turning snaps to the face's edges. Shift keeps a move to one
//! axis, a turn to 15° steps and a corner's scale even; Alt does not snap. Every selected
//! face changes, as the UV editor changes them, and a drag undoes in one step. Escape
//! takes it back.
use crate::paint_gizmo::land_on;
use editor_core::{
    camera3d::Camera3d,
    document::{Document, FaceRef},
    tool::Tool,
};
use eframe::egui;
use geometry_math::vec::{self, Vec2f, Vec3d};
use map_model::brush_face::BrushFace;
use std::{cell::RefCell, rc::Rc};

/// How close the pointer must come to a handle to take it, in points.
const GRAB: f32 = 9.0;
/// How near a turn has to come to lining the material up with an edge to stop there.
const EDGE_SNAP_DEGREES: f64 = 4.0;
/// How long, on the screen, the outlined block's sides try to be, in points.
const SHORTEST: f32 = 56.0;

#[derive(Default)]
pub struct TextureGizmo {
    drag: Option<Drag>,
    /// The block the last drag worked on, which the handles keep to while the same face
    /// is being edited, so they stay on the tile that was moved rather than jump to
    /// whichever is under the centre now.
    kept: Option<Kept>,
}

#[derive(Clone, Copy)]
struct Kept {
    face: FaceRef,
    corner: Vec2f,
    size: Vec2f,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Grip {
    Move,
    Turn,
    /// Scales along U, along V, or both.
    Scale {
        u: bool,
        v: bool,
    },
}

/// The face being edited, as it was when the handles were placed.
struct Face {
    reference: FaceRef,
    source: BrushFace,
    polygon: Vec<Vec3d>,
    centre: Vec3d,
    /// The corner of the outlined block, in texels, and the size of its sides.
    corner: Vec2f,
    size: Vec2f,
}

struct Drag {
    face: Face,
    grip: Grip,
    origin: egui::Pos2,
    moved: bool,
    snapped: Option<Vec3d>,
    /// What the drag has done, for the note beside the pointer.
    note: String,
}

/// Whether the tool shows texture handles: the same tools as the paint handles.
fn shows(tool: Tool) -> bool {
    matches!(tool, Tool::Select | Tool::Face | Tool::Paint)
}

impl Face {
    /// The face being edited, when it is picked, wears its material and faces the camera,
    /// outlined at the `kept` block when that is of this face.
    fn find(
        document: &Document,
        camera: &Camera3d,
        rect: egui::Rect,
        kept: Option<Kept>,
    ) -> Option<Face> {
        let (reference, source) = document.face_to_edit()?;
        if source.paint().is_some() || !document.selected_edit_faces().contains(&reference) {
            return None;
        }
        let polygon = document.face_to_edit_polygon()?;
        if polygon.len() < 3 {
            return None;
        }
        let centre = polygon.iter().fold(Vec3d::zero(), |a, p| a + *p) / polygon.len() as f64;
        if vec::dot(source.normal(), camera.position - centre) <= 0.0 {
            return None;
        }
        let tile = document
            .materials()
            .get(&source.material_name)
            .map(|m| Vec2f::new(m.width as f32, m.height as f32))
            .filter(|t| t.x() > 0.0 && t.y() > 0.0)
            .unwrap_or(editor_core::commands::DEFAULT_MATERIAL_SIZE);
        let mut face = Face {
            reference,
            source: source.clone(),
            polygon,
            centre,
            corner: Vec2f::zero(),
            size: tile,
        };
        match kept.filter(|k| k.face == reference) {
            Some(kept) => (face.corner, face.size) = (kept.corner, kept.size),
            None => face.place(camera, rect, tile),
        }
        Some(face)
    }

    /// Picks how many tiles the outline spans and which block it is: the one under the
    /// face's centre, with sides no shorter than [`SHORTEST`] and, where that allows, no
    /// longer than the face is across.
    fn place(&mut self, camera: &Camera3d, rect: egui::Rect, tile: Vec2f) {
        let across = self
            .polygon
            .iter()
            .filter_map(|p| screen(camera, rect, *p))
            .collect::<Vec<_>>();
        let across = egui::Rect::from_points(&across).size().length();
        let middle = self.source.uv_coords(self.centre);
        let side = |count: f32| {
            let size = tile * count;
            let a = self.at(middle);
            let (b, c) = (
                self.at(middle + Vec2f::new(size.x(), 0.0)),
                self.at(middle + Vec2f::new(0.0, size.y())),
            );
            match (
                a.and_then(|a| screen(camera, rect, a)),
                b.and_then(|b| screen(camera, rect, b)),
                c.and_then(|c| screen(camera, rect, c)),
            ) {
                (Some(a), Some(b), Some(c)) => a.distance(b).min(a.distance(c)),
                _ => SHORTEST,
            }
        };
        let mut count = 1.0f32;
        while side(count) < SHORTEST && count < 256.0 {
            count *= 2.0;
        }
        while side(count / 2.0) >= SHORTEST && side(count) > across * 0.9 && count > 1.0 / 16.0 {
            count /= 2.0;
        }
        self.size = tile * count;
        self.corner = Vec2f::new(
            (middle.x() / self.size.x()).floor() * self.size.x(),
            (middle.y() / self.size.y()).floor() * self.size.y(),
        );
    }

    /// The point of the face at texel `uv`.
    fn at(&self, uv: Vec2f) -> Option<Vec3d> {
        let s = &self.source;
        map_model::uv::point_at_uv(&s.uv_coord_system, uv, &s.plane, &s.uv_attributes)
    }

    /// Where a handle sits, in texels.
    fn uv(&self, grip: Grip) -> Vec2f {
        let (w, h) = (self.size.x(), self.size.y());
        let offset = match grip {
            Grip::Move => Vec2f::zero(),
            Grip::Turn => Vec2f::new(w, 0.0),
            Grip::Scale { u: true, v: true } => Vec2f::new(w, h),
            Grip::Scale { u: true, v: false } => Vec2f::new(w, h / 2.0),
            Grip::Scale { .. } => Vec2f::new(w / 2.0, h),
        };
        self.corner + offset
    }

    /// The outline's corners, in order around it.
    fn outline(&self) -> [Option<Vec3d>; 4] {
        let (w, h) = (self.size.x(), self.size.y());
        [
            Vec2f::zero(),
            Vec2f::new(w, 0.0),
            Vec2f::new(w, h),
            Vec2f::new(0.0, h),
        ]
        .map(|o| self.at(self.corner + o))
    }
}

const GRIPS: [Grip; 5] = [
    Grip::Move,
    Grip::Turn,
    Grip::Scale { u: true, v: true },
    Grip::Scale { u: true, v: false },
    Grip::Scale { u: false, v: true },
];

fn screen(camera: &Camera3d, rect: egui::Rect, p: Vec3d) -> Option<egui::Pos2> {
    camera
        .project(p, f64::from(rect.width()), f64::from(rect.height()))
        .map(|s| egui::pos2(rect.min.x + s.x() as f32, rect.min.y + s.y() as f32))
}

/// The angle from `a` to `b` about `normal`, in degrees.
fn angle_about(a: Vec3d, b: Vec3d, normal: Vec3d) -> f64 {
    vec::dot(vec::cross(a, b), normal)
        .atan2(vec::dot(a, b))
        .to_degrees()
}

fn wrap_180(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// The direction of U on `face` at texel `at`, in the world.
fn u_direction(face: &BrushFace, at: Vec2f) -> Option<Vec3d> {
    let point = |uv: Vec2f| {
        map_model::uv::point_at_uv(&face.uv_coord_system, uv, &face.plane, &face.uv_attributes)
    };
    Some(point(at + Vec2f::new(1.0, 0.0))? - point(at)?)
}

/// The turn, about texel `pivot`, that brings U on `face` round by `want` degrees in the
/// world. A paraxial face turns its material in the axis plane and then projects it, so
/// the angle on a sloped face is not the angle given: a few corrections close the gap.
fn turn_for(face: &BrushFace, pivot: Vec2f, want: f64) -> f32 {
    let normal = face.normal();
    let Some(before) = u_direction(face, pivot) else {
        return want as f32;
    };
    let achieved = |degrees: f32| {
        let mut probe = face.clone();
        probe.rotate_uv_about(pivot, degrees);
        u_direction(&probe, pivot).map_or(0.0, |after| angle_about(before, after, normal))
    };
    let sign = if achieved(10.0) < 0.0 { -1.0 } else { 1.0 };
    let mut degrees = (want * sign) as f32;
    for _ in 0..4 {
        let gap = wrap_180(want - achieved(degrees));
        if gap.abs() < 1e-3 {
            break;
        }
        degrees += (gap * sign) as f32;
    }
    degrees
}

impl TextureGizmo {
    /// Follows a drag, or takes a primary press on a handle before the view would select
    /// or drag with it. Returns whether the pointer is the handles'.
    pub fn update(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        document: &Rc<RefCell<Document>>,
        tool: Tool,
        camera: &Camera3d,
        hovered: bool,
    ) -> bool {
        if self.drag.is_some() {
            self.follow(ui, rect, document, camera);
            return true;
        }
        if !shows(tool) || !hovered || !ui.input(|i| i.pointer.primary_pressed()) {
            return false;
        }
        let Some(at) = ui.input(|i| i.pointer.press_origin()) else {
            return false;
        };
        let mut document = document.borrow_mut();
        let Some(face) = Face::find(&document, camera, rect, self.kept) else {
            return false;
        };
        let grabbed = GRIPS
            .iter()
            .filter_map(|grip| {
                Some((
                    *grip,
                    screen(camera, rect, face.at(face.uv(*grip))?)?.distance(at),
                ))
            })
            .filter(|(_, distance)| *distance <= GRAB)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((grip, _)) = grabbed else {
            return false;
        };
        document.begin_transaction();
        self.drag = Some(Drag {
            face,
            grip,
            origin: at,
            moved: false,
            snapped: None,
            note: String::new(),
        });
        true
    }

    fn follow(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        document: &Rc<RefCell<Document>>,
        camera: &Camera3d,
    ) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            document.borrow_mut().cancel_transaction();
            self.drag = None;
            return;
        }
        let (pointer, down, shift, alt) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
                i.modifiers.shift,
                i.modifiers.alt,
            )
        });
        if let Some(at) = pointer {
            drag.moved |= at.distance(drag.origin) > 3.0;
        }
        if drag.moved {
            let face = &drag.face;
            let snap = !alt && !(shift && drag.grip == Grip::Turn);
            if let Some((point, snapped)) = pointer.and_then(|at| {
                land_on(
                    &face.source.plane,
                    &face.polygon,
                    face.centre,
                    camera,
                    rect,
                    at,
                    snap,
                )
            }) {
                drag.snapped = snapped.then_some(point);
                let (edit, note) = drag.grip.edit(face, point, shift, !alt);
                drag.note = note;
                let mut document = document.borrow_mut();
                document.rollback_transaction();
                document.edit_faces(|f| edit.apply(f));
                ui.ctx().request_repaint();
            }
        }
        if !down {
            document.borrow_mut().commit_transaction();
            let face = self.drag.take().expect("a drag").face;
            self.kept = Some(Kept {
                face: face.reference,
                corner: face.corner,
                size: face.size,
            });
        }
    }

    /// Draws the outline and the handles over the view.
    pub fn show(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        document: &Rc<RefCell<Document>>,
        tool: Tool,
        camera: &Camera3d,
        hidden: bool,
    ) {
        if !shows(tool) || hidden {
            return;
        }
        // During a drag the handles follow the face as it is now, in the block they began in.
        let kept = self
            .drag
            .as_ref()
            .map(|d| Kept {
                face: d.face.reference,
                corner: d.face.corner,
                size: d.face.size,
            })
            .or(self.kept);
        let face = Face::find(&document.borrow(), camera, rect, kept);
        let Some(face) = face else { return };
        let painter = ui.painter_at(rect);
        let dark = egui::Stroke::new(3.0, egui::Color32::from_black_alpha(110));
        let light = egui::Stroke::new(1.25, egui::Color32::from_white_alpha(220));
        let corners: Vec<egui::Pos2> = face
            .outline()
            .iter()
            .filter_map(|p| p.and_then(|p| screen(camera, rect, p)))
            .collect();
        if corners.len() == 4 {
            for i in 0..4 {
                painter.line_segment([corners[i], corners[(i + 1) % 4]], dark);
            }
            for i in 0..4 {
                painter.line_segment([corners[i], corners[(i + 1) % 4]], light);
            }
        }
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let accent = crate::theme::accent();
        let outline = egui::Stroke::new(1.0, egui::Color32::from_black_alpha(160));
        for grip in GRIPS {
            let Some(at) = face.at(face.uv(grip)).and_then(|p| screen(camera, rect, p)) else {
                continue;
            };
            let hot = self.drag.as_ref().is_some_and(|d| d.grip == grip)
                || (self.drag.is_none() && pointer.is_some_and(|p| p.distance(at) <= GRAB));
            let grow = if hot { 2.0 } else { 0.0 };
            match grip {
                Grip::Move => {
                    let square =
                        egui::Rect::from_center_size(at, egui::vec2(11.0 + grow, 11.0 + grow));
                    painter.rect_stroke(
                        square.expand(1.0),
                        2.0,
                        outline,
                        egui::StrokeKind::Outside,
                    );
                    painter.rect(
                        square,
                        2.0,
                        accent,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );
                }
                Grip::Turn => {
                    painter.circle(at, 7.0 + grow, egui::Color32::TRANSPARENT, outline);
                    painter.circle(
                        at,
                        6.0 + grow,
                        egui::Color32::WHITE,
                        egui::Stroke::new(2.0, accent),
                    );
                }
                Grip::Scale { .. } => {
                    let side = if grip == (Grip::Scale { u: true, v: true }) {
                        9.0
                    } else {
                        7.0
                    } + grow;
                    let square = egui::Rect::from_center_size(at, egui::vec2(side, side));
                    painter.rect_stroke(
                        square.expand(1.0),
                        1.0,
                        outline,
                        egui::StrokeKind::Outside,
                    );
                    painter.rect(
                        square,
                        1.0,
                        egui::Color32::WHITE,
                        egui::Stroke::new(1.5, accent),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
        if let Some(drag) = &self.drag {
            if let Some(at) = drag.snapped.and_then(|p| screen(camera, rect, p)) {
                painter.circle_stroke(at, 9.0, egui::Stroke::new(1.5, accent));
            }
            if let (Some(at), false) = (pointer, drag.note.is_empty()) {
                let galley = painter.layout_no_wrap(
                    drag.note.clone(),
                    crate::theme::body(),
                    egui::Color32::WHITE,
                );
                let place = egui::Rect::from_min_size(at + egui::vec2(14.0, 14.0), galley.size())
                    .expand(4.0);
                painter.rect_filled(place, 4.0, egui::Color32::from_black_alpha(170));
                painter.galley(
                    place.min + egui::vec2(4.0, 4.0),
                    galley,
                    egui::Color32::WHITE,
                );
            }
        }
    }

    /// Whether a handle is being dragged.
    pub fn busy(&self) -> bool {
        self.drag.is_some()
    }
}

/// A change to a face's mapping, worked out on the face that was grabbed and applied to
/// every selected face.
#[derive(Clone, Copy, Debug)]
enum Edit {
    Move(Vec2f),
    Turn { pivot: Vec2f, degrees: f32 },
    Scale { pivot: Vec2f, factors: Vec2f },
}

impl Edit {
    fn apply(self, face: &mut BrushFace) {
        match self {
            Edit::Move(delta) => face.move_uv(delta),
            Edit::Turn { pivot, degrees } => face.rotate_uv_about(pivot, degrees),
            Edit::Scale { pivot, factors } => face.scale_uv_about(pivot, factors),
        }
    }
}

impl Grip {
    /// What dragging this handle of `face` to `point` does, and a note of it.
    fn edit(self, face: &Face, point: Vec3d, shift: bool, snap: bool) -> (Edit, String) {
        let source = &face.source;
        let pivot = face.corner;
        let to = source.uv_coords(point);
        match self {
            Grip::Move => {
                // The corner goes where the pointer is: the material shifts by the
                // difference, measured in the face's mapping as it was.
                let mut delta = face.uv(Grip::Move) - to;
                if shift {
                    if delta.x().abs() > delta.y().abs() {
                        delta.v[1] = 0.0;
                    } else {
                        delta.v[0] = 0.0;
                    }
                }
                (
                    Edit::Move(delta),
                    format!("{:+.0}, {:+.0}", -delta.x(), -delta.y()),
                )
            }
            Grip::Turn => {
                let normal = source.normal();
                let (Some(from), Some(corner)) = (face.at(face.uv(Grip::Turn)), face.at(pivot))
                else {
                    return (
                        Edit::Turn {
                            pivot,
                            degrees: 0.0,
                        },
                        String::new(),
                    );
                };
                let (start, now) = (from - corner, point - corner);
                if vec::length(now) < 1e-9 {
                    return (
                        Edit::Turn {
                            pivot,
                            degrees: 0.0,
                        },
                        String::new(),
                    );
                }
                let mut want = angle_about(start, now, normal);
                if shift {
                    // Steps of 15° from the face's first edge.
                    let first = face.polygon[1] - face.polygon[0];
                    let base = angle_about(start, first, normal);
                    want = base + ((want - base) / 15.0).round() * 15.0;
                } else if snap {
                    // The nearest turn that lines U up with an edge, or with its square.
                    let count = face.polygon.len();
                    let best = (0..count)
                        .map(|i| face.polygon[(i + 1) % count] - face.polygon[i])
                        .filter(|e| vec::length(*e) > 1e-9)
                        .flat_map(|e| {
                            let along = angle_about(start, e, normal);
                            (0..4).map(move |k| along + f64::from(k) * 90.0)
                        })
                        .map(|a| (a, wrap_180(a - want).abs()))
                        .min_by(|a, b| a.1.total_cmp(&b.1));
                    if let Some((angle, _)) = best.filter(|(_, gap)| *gap <= EDGE_SNAP_DEGREES) {
                        want += wrap_180(angle - want);
                    }
                }
                let want = wrap_180(want);
                (
                    Edit::Turn {
                        pivot,
                        degrees: turn_for(source, pivot, want),
                    },
                    format!("{want:.1}°"),
                )
            }
            Grip::Scale { u, v } => {
                let reach = face.uv(self) - pivot;
                let now = to - pivot;
                let ratio = |now: f32, reach: f32| {
                    if reach.abs() < 1e-6 {
                        1.0
                    } else {
                        (now / reach).clamp(0.02, 50.0)
                    }
                };
                let mut factors = Vec2f::new(
                    if u { ratio(now.x(), reach.x()) } else { 1.0 },
                    if v { ratio(now.y(), reach.y()) } else { 1.0 },
                );
                if u && v && shift {
                    // Even: how far along the diagonal the pointer has come.
                    let along = (now.x() * reach.x() + now.y() * reach.y())
                        / (reach.x() * reach.x() + reach.y() * reach.y()).max(1e-6);
                    factors = Vec2f::new(along.clamp(0.02, 50.0), along.clamp(0.02, 50.0));
                }
                let scale = source.uv_attributes.scale * factors;
                let note = match (u, v) {
                    (true, true) => format!("{:.3} × {:.3}", scale.x(), scale.y()),
                    (true, false) => format!("U {:.3}", scale.x()),
                    _ => format!("V {:.3}", scale.y()),
                };
                (Edit::Scale { pivot, factors }, note)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::brush_face::BrushFace;

    /// A face on the floor, 128 units square, with the material in its default place.
    fn floor() -> Face {
        let source = BrushFace::from_points(
            [
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
            ],
            "stone",
            map_model::uv::UvCoordSystem::Paraxial,
            Default::default(),
        )
        .expect("a face");
        let polygon = vec![
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(128.0, 0.0, 0.0),
            Vec3d::new(128.0, 128.0, 0.0),
            Vec3d::new(0.0, 128.0, 0.0),
        ];
        Face {
            reference: FaceRef {
                node: map_model::node::NodeId(0),
                face: 0,
            },
            source,
            polygon,
            centre: Vec3d::new(64.0, 64.0, 0.0),
            corner: Vec2f::zero(),
            size: Vec2f::new(64.0, 64.0),
        }
    }

    fn edited(face: &Face, grip: Grip, point: Vec3d, shift: bool) -> BrushFace {
        let mut out = face.source.clone();
        grip.edit(face, point, shift, true).0.apply(&mut out);
        out
    }

    fn near(a: Vec2f, b: Vec2f) -> bool {
        (a - b).x().abs() < 1e-2 && (a - b).y().abs() < 1e-2
    }

    #[test]
    fn the_corner_handle_carries_the_material_with_it() {
        let face = floor();
        let corner = face.at(face.uv(Grip::Move)).unwrap();
        let point = Vec3d::new(10.0, 20.0, 0.0) + corner;
        let after = edited(&face, Grip::Move, point, false);
        assert!(near(after.uv_coords(point), face.uv(Grip::Move)));
    }

    #[test]
    fn the_turn_handle_brings_u_round_to_the_pointer() {
        let face = floor();
        let corner = face.at(face.corner).unwrap();
        let from = face.at(face.uv(Grip::Turn)).unwrap() - corner;
        // A quarter turn about the face's normal.
        let point = corner + vec::cross(face.source.normal(), from);
        let after = edited(&face, Grip::Turn, point, false);
        let along = after.uv_coords(point) - after.uv_coords(corner);
        assert!(along.x() > 1.0 && along.y().abs() < 1e-2, "{along:?}");
        assert!(near(after.uv_coords(corner), face.corner));
    }

    #[test]
    fn the_far_corner_scales_the_tile_to_reach_the_pointer() {
        let face = floor();
        let corner = face.at(face.corner).unwrap();
        let far = face.at(face.uv(Grip::Scale { u: true, v: true })).unwrap();
        let point = corner + (far - corner) * 1.5;
        let after = edited(&face, Grip::Scale { u: true, v: true }, point, false);
        assert!(near(
            after.uv_coords(point),
            face.uv(Grip::Scale { u: true, v: true })
        ));
        assert!(near(after.uv_coords(corner), face.corner));
    }

    #[test]
    fn an_edge_handle_scales_one_axis() {
        let face = floor();
        let grip = Grip::Scale { u: true, v: false };
        let corner = face.at(face.corner).unwrap();
        let handle = face.at(face.uv(grip)).unwrap();
        let after = edited(&face, grip, corner + (handle - corner) * 2.0, false);
        let scale = after.uv_attributes.scale;
        assert!((scale.x() - face.source.uv_attributes.scale.x() * 2.0).abs() < 1e-3);
        assert!((scale.y() - face.source.uv_attributes.scale.y()).abs() < 1e-6);
    }

    #[test]
    fn a_turn_on_a_sloped_face_lands_on_the_angle_asked() {
        let source = BrushFace::from_points(
            [
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.7),
                Vec3d::new(1.0, 0.0, 0.0),
            ],
            "stone",
            map_model::uv::UvCoordSystem::Paraxial,
            Default::default(),
        )
        .expect("a face");
        let normal = source.normal();
        let before = u_direction(&source, Vec2f::zero()).unwrap();
        for want in [30.0, -75.0, 140.0] {
            let mut after = source.clone();
            after.rotate_uv_about(Vec2f::zero(), turn_for(&source, Vec2f::zero(), want));
            let got = angle_about(before, u_direction(&after, Vec2f::zero()).unwrap(), normal);
            assert!(wrap_180(got - want).abs() < 0.05, "{want} turned {got}");
        }
    }
}
