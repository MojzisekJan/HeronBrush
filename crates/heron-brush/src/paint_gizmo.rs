//! Paint handles on the selected faces in the 3D view: a dot at the centre of a face in one
//! colour, which opens a colour picker, and a gradient's ends and colour stops, dragged
//! across the face and snapped to its corners, the middles of its edges, its edges and its
//! centre. Shift keeps an end's direction to 15° steps or along an edge, Alt drags without
//! snapping, and Alt+click adds a stop on the gradient's line or removes the one clicked.
//! A drag paints as it goes and undoes in one step.
use editor_core::{camera3d::Camera3d, document::{Document, FaceRef}, tool::Tool};
use eframe::egui;
use geometry_math::{plane::Plane3d, vec::{self, Vec3d}};
use map_model::brush_face::{BrushFace, ColorStop, FaceColor, GradientEnds, GradientStops, PaintOnFace, Rgb};
use std::{cell::RefCell, rc::Rc};

/// How close the pointer must come to a handle to take it, and to a corner, edge or centre
/// to snap to it, in points.
const GRAB: f32 = 9.0;
const SNAP: f32 = 10.0;
/// The most faces whose handles are drawn at once.
const MOST_FACES: usize = 64;

#[derive(Default)]
pub struct PaintGizmo {
    drag: Option<Drag>,
    picker: Option<Picker>,
}

/// One selected face as the handles see it.
struct Face {
    face: FaceRef,
    source: BrushFace,
    polygon: Vec<Vec3d>,
    centre: Vec3d,
    paint: FaceColor,
}

/// What a handle stands for.
#[derive(Clone, Copy, PartialEq)]
enum Grip {
    /// The dot of a face in one colour.
    Colour,
    /// A gradient's start or end.
    End(bool),
    /// A colour stop between the ends, by its index in the sorted stops.
    Stop(usize),
}

struct Drag {
    face: Face,
    /// Every face painted like this one when the drag began: they change together.
    faces: Vec<FaceRef>,
    grip: Grip,
    stops: Vec<ColorStop>,
    radial: bool,
    angle: f32,
    alpha: u8,
    ends: GradientEnds,
    origin: egui::Pos2,
    moved: bool,
    mark: usize,
    /// Where the handle snapped last frame, to mark it.
    snapped: Option<Vec3d>,
    /// Alt was held at the press: a click removes the stop, a drag does not snap.
    alt: bool,
}

/// The colour picker a click on a handle opened.
struct Picker {
    faces: Vec<FaceRef>,
    at: egui::Pos2,
    rgb: [u8; 3],
    alpha: f32,
    /// What it colours: the whole face, or one stop of the gradient.
    target: Target,
    mark: usize,
}

enum Target {
    Solid,
    Stop { paint: FaceColor, index: usize },
}

/// Whether the tool shows paint handles: the ones that pick or paint faces.
fn shows(tool: Tool) -> bool {
    matches!(tool, Tool::Select | Tool::Face | Tool::Paint)
}

/// A gradient's stops, whether it is radial, its angle, its alpha and its ends.
type Gradient = (Vec<ColorStop>, bool, f32, u8, Option<GradientEnds>);

/// A gradient as [`Gradient`], a two-colour one too.
fn as_gradient(paint: FaceColor) -> Option<Gradient> {
    match paint {
        FaceColor::MultiGradient { stops, radial, angle, alpha, ends } => Some((stops.as_slice().to_vec(), radial, angle, alpha, ends)),
        FaceColor::Gradient { from, to, angle, alpha } => Some((
            vec![ColorStop { position: 0.0, color: from }, ColorStop { position: 1.0, color: to }],
            false,
            angle,
            alpha,
            None,
        )),
        FaceColor::Solid { .. } => None,
    }
}

impl Face {
    fn collect(document: &Document) -> Vec<Face> {
        document
            .selected_edit_faces()
            .into_iter()
            .take(MOST_FACES)
            .filter_map(|face| {
                let brush = document.brush(face.node)?;
                let source = brush.faces().get(brush.source_face_index(face.face)?)?.clone();
                let paint = source.paint()?;
                let polygon = brush.geometry().face_positions(map_model::polyhedron::FaceId(face.face as u32));
                if polygon.len() < 3 {
                    return None;
                }
                let centre = polygon.iter().fold(Vec3d::zero(), |a, p| a + *p) / polygon.len() as f64;
                Some(Face { face, source, polygon, centre, paint })
            })
            .collect()
    }

    fn sampler(&self, paint: FaceColor) -> PaintOnFace {
        PaintOnFace::new(&self.source, &self.polygon, paint)
    }

    /// The point on the face at `fraction` of its UV bounds.
    fn point(&self, sampler: &PaintOnFace, fraction: [f32; 2]) -> Option<Vec3d> {
        let s = &self.source;
        map_model::uv::point_at_uv(&s.uv_coord_system, sampler.from_bounds(fraction), &s.plane, &s.uv_attributes)
    }

    /// Whether the face is turned towards the camera.
    fn facing(&self, camera: &Camera3d) -> bool {
        vec::dot(self.source.normal(), camera.position - self.centre) > 0.0
    }
}

/// Where a handle is drawn: its grip, its point on the screen, and its colour.
struct Handle {
    grip: Grip,
    screen: egui::Pos2,
    colour: Rgb,
}

/// The handles of `face`, on the screen of `rect`.
fn handles(face: &Face, camera: &Camera3d, rect: egui::Rect) -> Vec<Handle> {
    let size = (f64::from(rect.width()), f64::from(rect.height()));
    let screen = |p: Vec3d| camera.project(p, size.0, size.1).map(|s| egui::pos2(rect.min.x + s.x() as f32, rect.min.y + s.y() as f32));
    let mut out = Vec::new();
    match as_gradient(face.paint) {
        None => {
            let FaceColor::Solid { color, .. } = face.paint else { return out };
            if let Some(at) = screen(face.centre) {
                out.push(Handle { grip: Grip::Colour, screen: at, colour: color });
            }
        }
        Some((stops, ..)) => {
            let sampler = face.sampler(face.paint);
            let Some(ends) = sampler.ends() else { return out };
            let (Some(a), Some(b)) = (face.point(&sampler, ends.start), face.point(&sampler, ends.end)) else { return out };
            for (end, world) in [(false, a), (true, b)] {
                let colour = if end { stops.last() } else { stops.first() }.map_or(Rgb::new(0, 0, 0), |s| s.color);
                if let Some(at) = screen(world) {
                    out.push(Handle { grip: Grip::End(end), screen: at, colour });
                }
            }
            for (index, stop) in stops.iter().enumerate() {
                // A stop at an end is that end's colour: the end's handle carries it.
                if stop.position <= 0.0 || stop.position >= 1.0 {
                    continue;
                }
                let world = a + (b - a) * f64::from(stop.position);
                if let Some(at) = screen(world) {
                    out.push(Handle { grip: Grip::Stop(index), screen: at, colour: stop.color });
                }
            }
        }
    }
    out
}

/// The point nearest `p` on the segment from `a` to `b`.
fn nearest_on_segment(a: Vec3d, b: Vec3d, p: Vec3d) -> Vec3d {
    let d = b - a;
    let length = vec::dot(d, d);
    if length <= 1e-12 {
        return a;
    }
    a + d * (vec::dot(p - a, d) / length).clamp(0.0, 1.0)
}

/// Where the pointer at `at` lands on the face's plane, snapped (when `snap`) to a corner,
/// the middle of an edge or the centre within [`SNAP`] points of it, else to an edge; and
/// whether it snapped.
fn land(face: &Face, camera: &Camera3d, rect: egui::Rect, at: egui::Pos2, snap: bool) -> Option<(Vec3d, bool)> {
    land_on(&face.source.plane, &face.polygon, face.centre, camera, rect, at, snap)
}

/// [`land`] for any face, given as its plane, its corners and its centre.
pub(crate) fn land_on(plane: &Plane3d, polygon: &[Vec3d], centre: Vec3d, camera: &Camera3d, rect: egui::Rect, at: egui::Pos2, snap: bool) -> Option<(Vec3d, bool)> {
    let size = (f64::from(rect.width()), f64::from(rect.height()));
    let local = at - rect.min;
    let ray = camera.ray_through(f64::from(local.x), f64::from(local.y), size.0, size.1);
    let distance = geometry_math::intersection::intersect_ray_plane(&ray, plane)?;
    let hit = ray.origin + ray.direction * distance;
    if !snap {
        return Some((hit, false));
    }
    let screen = |p: Vec3d| camera.project(p, size.0, size.1).map(|s| egui::pos2(rect.min.x + s.x() as f32, rect.min.y + s.y() as f32));
    let near = |p: Vec3d| screen(p).is_some_and(|s| s.distance(at) <= SNAP);
    let count = polygon.len();
    let middles = (0..count).map(|i| (polygon[i] + polygon[(i + 1) % count]) * 0.5);
    let corners = polygon.iter().copied().chain(middles).chain([centre]);
    if let Some(point) = corners.filter(|p| near(*p)).min_by(|a, b| vec::length(*a - hit).total_cmp(&vec::length(*b - hit))) {
        return Some((point, true));
    }
    let edges = (0..polygon.len()).map(|i| nearest_on_segment(polygon[i], polygon[(i + 1) % polygon.len()], hit));
    if let Some(point) = edges.filter(|p| near(*p)).min_by(|a, b| vec::length(*a - hit).total_cmp(&vec::length(*b - hit))) {
        return Some((point, true));
    }
    Some((hit, false))
}

/// `point` moved so that its direction from `from` keeps to a 15° step from the face's
/// first edge, or to one of its edges, whichever is nearest.
fn constrain(face: &Face, from: Vec3d, point: Vec3d) -> Vec3d {
    let offset = point - from;
    let length = vec::length(offset);
    let normal = face.source.normal();
    let first = face.polygon[1] - face.polygon[0];
    if length < 1e-9 || vec::length(first) < 1e-9 {
        return point;
    }
    let u = vec::normalize(first);
    let v = vec::cross(normal, u);
    let steps = (0..24).map(|k| {
        let a = f64::from(k as u8) * 15f64.to_radians();
        u * a.cos() + v * a.sin()
    });
    let count = face.polygon.len();
    let edges = (0..count).map(|i| face.polygon[(i + 1) % count] - face.polygon[i]).filter(|d| vec::length(*d) > 1e-9).flat_map(|d| {
        let d = vec::normalize(d);
        [d, d * -1.0]
    });
    let direction = offset / length;
    let best = steps.chain(edges).max_by(|a, b| vec::dot(*a, direction).total_cmp(&vec::dot(*b, direction))).unwrap_or(direction);
    from + best * vec::dot(offset, best)
}

fn colour32(rgb: Rgb) -> egui::Color32 {
    egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}

impl PaintGizmo {
    /// Whether a handle is being dragged, so the view leaves the pointer to it.
    pub fn busy(&self) -> bool {
        self.drag.is_some()
    }

    /// Takes a primary press on a handle, before the view would select or drag with it.
    /// Returns whether it took the press.
    pub fn press(&mut self, ui: &egui::Ui, rect: egui::Rect, document: &Rc<RefCell<Document>>, tool: Tool, camera: &Camera3d) -> bool {
        if !shows(tool) || self.drag.is_some() || !ui.input(|i| i.pointer.primary_pressed()) {
            return false;
        }
        let Some(at) = ui.input(|i| i.pointer.press_origin()) else { return false };
        if !rect.contains(at) {
            return false;
        }
        let document_ref = document.borrow();
        let faces = Face::collect(&document_ref);
        let grabbed = faces
            .into_iter()
            .filter(|face| face.facing(camera))
            .flat_map(|face| handles(&face, camera, rect).into_iter().map(|h| (h.grip, h.screen.distance(at))).collect::<Vec<_>>().into_iter().map(move |h| (face.face, h)))
            .filter(|(_, (_, distance))| *distance <= GRAB)
            .min_by(|a, b| a.1 .1.total_cmp(&b.1 .1));
        let alt = ui.input(|i| i.modifiers.alt);
        let Some((picked, (grip, _))) = grabbed else {
            drop(document_ref);
            return alt && self.add_stop(document, camera, rect, at);
        };
        // Collected again for the one face, to own it.
        let Some(face) = Face::collect(&document_ref).into_iter().find(|f| f.face == picked) else { return false };
        let alike: Vec<FaceRef> = Face::collect(&document_ref).into_iter().filter(|f| f.paint == face.paint).map(|f| f.face).collect();
        let mark = document_ref.history_mark();
        drop(document_ref);
        self.picker = None;
        let (stops, radial, angle, alpha, ends) = match as_gradient(face.paint) {
            Some((stops, radial, angle, alpha, ends)) => (stops, radial, angle, alpha, ends),
            None => (Vec::new(), false, 0.0, face.paint.alpha(), None),
        };
        let ends = ends.or_else(|| face.sampler(face.paint).ends()).unwrap_or(GradientEnds { start: [0.0, 0.5], end: [1.0, 0.5] });
        self.drag = Some(Drag { face, faces: alike, grip, stops, radial, angle, alpha, ends, origin: at, moved: false, mark, snapped: None, alt });
        true
    }

    /// Alt+click on a gradient's line: a stop there, in the colour the gradient has there.
    /// Returns whether the click was on a line.
    fn add_stop(&mut self, document: &Rc<RefCell<Document>>, camera: &Camera3d, rect: egui::Rect, at: egui::Pos2) -> bool {
        let faces = Face::collect(&document.borrow());
        for face in faces.iter().filter(|f| f.facing(camera)) {
            let Some((mut stops, radial, angle, alpha, _)) = as_gradient(face.paint) else { continue };
            let handles = handles(face, camera, rect);
            let (Some(a), Some(b)) = (handles.iter().find(|h| h.grip == Grip::End(false)), handles.iter().find(|h| h.grip == Grip::End(true))) else { continue };
            let along = b.screen - a.screen;
            let t = ((at - a.screen).dot(along) / along.length_sq().max(1e-6)).clamp(0.0, 1.0);
            if (a.screen + along * t).distance(at) > 6.0 || stops.len() >= 8 {
                continue;
            }
            let sampler = face.sampler(face.paint);
            let Some((point, _)) = land(face, camera, rect, at, false) else { continue };
            let position = sampler.position_at_uv(face.source.uv_coords(point)).clamp(0.001, 0.999);
            let ends = sampler.ends();
            let colour = Rgb::from_f32(GradientStops::new(&stops).map_or([0.0; 3], |g| g.at(position)));
            stops.push(ColorStop { position, color: colour });
            let Some(stops) = GradientStops::new(&stops) else { continue };
            let alike: Vec<FaceRef> = faces.iter().filter(|f| f.paint == face.paint).map(|f| f.face).collect();
            let mut document = document.borrow_mut();
            let mark = document.history_mark();
            document.set_face_color_since(Some(FaceColor::MultiGradient { stops, radial, angle, alpha, ends }), &alike, mark);
            return true;
        }
        false
    }

    /// Follows a drag, opens the picker for a click, and draws the handles over the view.
    pub fn show(&mut self, ui: &egui::Ui, rect: egui::Rect, document: &Rc<RefCell<Document>>, tool: Tool, camera: &Camera3d, hidden: bool) {
        self.follow(ui, rect, document, camera);
        self.picker(ui, document);
        if !shows(tool) || hidden {
            return;
        }
        let painter = ui.painter_at(rect);
        let faces = Face::collect(&document.borrow());
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let dragged = self.drag.as_ref().filter(|d| d.moved).map(|d| (d.face.face, d.grip));
        for face in faces.iter().filter(|f| f.facing(camera)) {
            let handles = handles(face, camera, rect);
            if handles.len() >= 2 {
                if let (Some(a), Some(b)) = (handles.iter().find(|h| h.grip == Grip::End(false)), handles.iter().find(|h| h.grip == Grip::End(true))) {
                    painter.line_segment([a.screen, b.screen], egui::Stroke::new(3.0, egui::Color32::from_black_alpha(110)));
                    painter.line_segment([a.screen, b.screen], egui::Stroke::new(1.5, egui::Color32::WHITE));
                }
            }
            for handle in &handles {
                let hot = dragged == Some((face.face, handle.grip)) || pointer.is_some_and(|p| p.distance(handle.screen) <= GRAB);
                let fill = colour32(handle.colour);
                let ring = egui::Stroke::new(2.0, egui::Color32::WHITE);
                let outline = egui::Stroke::new(1.0, egui::Color32::from_black_alpha(160));
                match handle.grip {
                    Grip::Colour => {
                        let r = if hot { 8.0 } else { 7.0 };
                        painter.circle(handle.screen, r + 1.0, egui::Color32::TRANSPARENT, outline);
                        painter.circle(handle.screen, r, fill, ring);
                    }
                    Grip::End(_) => {
                        let side = if hot { 12.0 } else { 10.0 };
                        let square = egui::Rect::from_center_size(handle.screen, egui::vec2(side, side));
                        painter.rect_stroke(square.expand(1.0), 2.0, outline, egui::StrokeKind::Outside);
                        painter.rect(square, 2.0, fill, ring, egui::StrokeKind::Middle);
                    }
                    Grip::Stop(_) => {
                        let r = if hot { 6.0 } else { 5.0 };
                        painter.circle(handle.screen, r + 1.0, egui::Color32::TRANSPARENT, outline);
                        painter.circle(handle.screen, r, fill, ring);
                    }
                }
            }
        }
        if let Some(point) = self.drag.as_ref().and_then(|d| d.snapped) {
            let size = (f64::from(rect.width()), f64::from(rect.height()));
            if let Some(s) = camera.project(point, size.0, size.1) {
                let at = egui::pos2(rect.min.x + s.x() as f32, rect.min.y + s.y() as f32);
                painter.circle_stroke(at, 9.0, egui::Stroke::new(1.5, crate::theme::accent()));
            }
        }
    }

    /// Moves the dragged handle, paints, and ends the drag on release; a release that did
    /// not move opens the picker.
    fn follow(&mut self, ui: &egui::Ui, rect: egui::Rect, document: &Rc<RefCell<Document>>, camera: &Camera3d) {
        let Some(drag) = self.drag.as_mut() else { return };
        let (pointer, down, released) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.primary_down(), i.pointer.primary_released()));
        if let Some(at) = pointer {
            drag.moved |= at.distance(drag.origin) > 3.0;
        }
        if drag.moved && drag.grip != Grip::Colour {
            let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
            let end = matches!(drag.grip, Grip::End(_));
            if let Some((mut point, snapped)) = pointer.and_then(|at| land(&drag.face, camera, rect, at, !alt && !(shift && end))) {
                drag.snapped = snapped.then_some(point);
                let paint = drag.paint();
                let sampler = drag.face.sampler(paint);
                if let (true, Grip::End(which)) = (shift, drag.grip) {
                    let other = if which { drag.ends.start } else { drag.ends.end };
                    if let Some(from) = drag.face.point(&sampler, other) {
                        point = constrain(&drag.face, from, point);
                    }
                }
                let uv = drag.face.source.uv_coords(point);
                match drag.grip {
                    Grip::End(false) => drag.ends.start = sampler.to_bounds(uv),
                    Grip::End(true) => drag.ends.end = sampler.to_bounds(uv),
                    Grip::Stop(index) => {
                        let t = sampler.position_at_uv(uv);
                        if let Some(stop) = drag.stops.get_mut(index) {
                            // Kept just inside the ends, so it stays a handle of its own.
                            stop.position = if t.is_finite() { t.clamp(0.001, 0.999) } else { stop.position };
                        }
                    }
                    Grip::Colour => {}
                }
                let paint = drag.paint();
                document.borrow_mut().set_face_color_since(Some(paint), &drag.faces, drag.mark);
                ui.ctx().request_repaint();
            }
        }
        if released || !down {
            let mut drag = self.drag.take().expect("a drag");
            match (drag.moved, drag.alt, drag.grip) {
                (false, true, Grip::Stop(index)) if drag.stops.len() > 2 => {
                    drag.stops.remove(index);
                    let paint = drag.paint();
                    document.borrow_mut().set_face_color_since(Some(paint), &drag.faces, drag.mark);
                }
                (false, _, _) => self.open_picker(ui, drag),
                _ => {}
            }
        }
    }

    /// The picker for the handle `drag` clicked: the face's colour, or its stop's.
    fn open_picker(&mut self, ui: &egui::Ui, drag: Drag) {
        let at = drag.origin + egui::vec2(12.0, 12.0);
        let (rgb, target) = match drag.grip {
            Grip::Colour => {
                let FaceColor::Solid { color, .. } = drag.face.paint else { return };
                (color, Target::Solid)
            }
            grip => {
                let index = match grip {
                    Grip::End(false) => 0,
                    Grip::End(true) => drag.stops.len().saturating_sub(1),
                    Grip::Stop(index) => index,
                    Grip::Colour => unreachable!(),
                };
                let Some(stop) = drag.stops.get(index) else { return };
                (stop.color, Target::Stop { paint: drag.face.paint, index })
            }
        };
        self.picker = Some(Picker {
            faces: drag.faces,
            at,
            rgb: [rgb.r, rgb.g, rgb.b],
            alpha: f32::from(drag.alpha) / 255.0,
            target,
            mark: drag.mark,
        });
        egui::Popup::open_id(ui.ctx(), picker_id());
    }

    /// The open picker: what it picks paints live, and one undo takes it all back.
    fn picker(&mut self, ui: &egui::Ui, document: &Rc<RefCell<Document>>) {
        let Some(picker) = self.picker.as_mut() else { return };
        let mut changed = false;
        let shown = egui::Popup::new(picker_id(), ui.ctx().clone(), egui::PopupAnchor::Position(picker.at), egui::LayerId::new(egui::Order::Foreground, picker_id()))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .open_memory(None)
            .show(|ui| {
                egui::Frame::new().inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                    let alpha = matches!(picker.target, Target::Solid).then_some(&mut picker.alpha);
                    changed = crate::colour_picker::contents(ui, picker_id(), &mut picker.rgb, alpha);
                });
            });
        if shown.is_none() {
            self.picker = None;
            return;
        }
        if !changed {
            return;
        }
        let [r, g, b] = picker.rgb;
        let paint = match &picker.target {
            Target::Solid => FaceColor::solid(Rgb::new(r, g, b)).with_alpha((picker.alpha.clamp(0.0, 1.0) * 255.0).round() as u8),
            Target::Stop { paint, index } => {
                let Some((mut stops, radial, angle, alpha, ends)) = as_gradient(*paint) else { return };
                if let Some(stop) = stops.get_mut(*index) {
                    stop.color = Rgb::new(r, g, b);
                }
                let Some(stops) = GradientStops::new(&stops) else { return };
                FaceColor::MultiGradient { stops, radial, angle, alpha, ends }
            }
        };
        document.borrow_mut().set_face_color_since(Some(paint), &picker.faces, picker.mark);
    }
}

impl Drag {
    /// The gradient as the drag has it now.
    fn paint(&self) -> FaceColor {
        let stops = GradientStops::new(&self.stops).unwrap_or_else(|| {
            GradientStops::new(&[
                ColorStop { position: 0.0, color: Rgb::new(0, 0, 0) },
                ColorStop { position: 1.0, color: Rgb::new(255, 255, 255) },
            ])
            .expect("two stops")
        });
        FaceColor::MultiGradient { stops, radial: self.radial, angle: self.angle, alpha: self.alpha, ends: Some(self.ends) }
    }
}

fn picker_id() -> egui::Id {
    egui::Id::new("paint-gizmo-picker")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_point_on_a_segment_stays_on_it() {
        let (a, b) = (Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(10.0, 0.0, 0.0));
        assert_eq!(nearest_on_segment(a, b, Vec3d::new(4.0, 3.0, 0.0)), Vec3d::new(4.0, 0.0, 0.0));
        assert_eq!(nearest_on_segment(a, b, Vec3d::new(-4.0, 3.0, 0.0)), a);
        assert_eq!(nearest_on_segment(a, b, Vec3d::new(14.0, 3.0, 0.0)), b);
    }

    #[test]
    fn a_two_colour_gradient_reads_as_two_stops() {
        let (stops, radial, angle, alpha, ends) = as_gradient(FaceColor::gradient(Rgb::new(1, 2, 3), Rgb::new(4, 5, 6), 30.0)).unwrap();
        assert_eq!((stops.len(), radial, angle, alpha, ends), (2, false, 30.0, 255, None));
        assert!(as_gradient(FaceColor::solid(Rgb::new(0, 0, 0))).is_none());
    }
}
