//! What the active tool draws on top of the scene in the 3D view.
//!
//! The 2D views draw their own handles with cairo, where a handle is a few pixels on a
//! flat projection. The 3D view has no such luxury: a handle there is a point in space
//! that has to be drawn with everything else. This turns whatever the current tool is
//! working on into two lists -- points to mark, and line segments to draw -- which is all
//! the renderer needs to know about tools.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::assemble_tool::AssembleBrushTool;
use crate::clip_tool::ClipTool;
use crate::document::Document;
use crate::handle_tool::HandleManager;
use crate::rotate_tool::RotateTool;
use crate::scale_tool;
use crate::tool::Tool;

/// A point to mark, or an endpoint of a line to draw.
pub type Marked = (Vec3d, [f32; 4]);

/// Reads `Preferences::HandleColor` for unselected handles, keeping 2D and 3D views consistent.
fn handle_color() -> [f32; 4] {
    crate::preferences::pref(|p| p.handle_color).rgba_f32()
}

/// Reads `Preferences::SelectedHandleColor` for selected handles.
fn selected_color() -> [f32; 4] {
    crate::preferences::pref(|p| p.selected_handle_color).rgba_f32()
}
/// The colour of a guide line.
const GUIDE_COLOR: [f32; 4] = [0.55, 0.85, 1.0, 0.7];
/// The outline of the half the clip tool would keep.
const CLIP_KEPT_COLOR: [f32; 4] = [1.0, 0.5, 0.0, 0.95];
/// The outline of the half the clip tool would throw away, drawn faded.
const CLIP_REMOVED_COLOR: [f32; 4] = [1.0, 0.5, 0.0, 0.25];
/// The outline of the brushes a sweep would make.
const SWEEP_PREVIEW_COLOR: [f32; 4] = [0.4, 0.9, 0.5, 0.8];

/// How many segments a rotation ring is drawn with.
const RING_SEGMENTS: usize = 64;

/// Everything the active tool wants drawn: the points to mark, and the lines to draw.
///
/// The lines come out as pairs, one segment per two entries, which is what `GL_LINES`
/// wants.
///
/// `rotate_ring_radius` is the world radius the rotate tool's rings are drawn at. The
/// caller sizes it to a constant number of pixels on screen, `RotateHandle::majorRadius`
/// times the camera's perspective scaling, rather than to the selection -- a ring scaled
/// to a large selection lands near the camera and its segments cross the near plane.
#[allow(
    clippy::too_many_arguments,
    reason = "one call gathers every tool's overlay"
)]
pub fn build(
    document: &Document,
    tool: Tool,
    handles: &HandleManager,
    rotate: &RotateTool,
    clip: &ClipTool,
    assemble: &AssembleBrushTool,
    sweep: &crate::sweep_tool::SweepInputs,
    rotate_ring_radius: f64,
) -> (Vec<Marked>, Vec<Marked>) {
    let mut points = Vec::new();
    let mut lines = Vec::new();

    match tool {
        Tool::Vertex | Tool::Edge | Tool::Face | Tool::ControlPoint | Tool::Fillet => {
            for handle in handles.handles() {
                let mut color = if handles.is_selected(handle.position) {
                    selected_color()
                } else {
                    handle_color()
                };
                // A handle that would add a corner rather than move one is drawn faint:
                // the 3D view draws every point at one size, so the colour is what says
                // which of the two it is.
                if handle.splits {
                    color[3] *= 0.45;
                }
                points.push((handle.position, color));
            }
        }

        Tool::Scale => {
            if let Some(bounds) = document.selection_bounds() {
                for handle in scale_tool::all_handles() {
                    points.push((handle.position(&bounds), handle_color()));
                }
                lines.extend(box_edges(&bounds, GUIDE_COLOR));
            }
        }

        Tool::Rotate => {
            if let Some(center) = rotate.center() {
                points.push((center, selected_color()));
                let radius = rotate_ring_radius;
                // One ring per axis, in the axis colours, so which way a drag will turn is
                // readable without having to try it.
                for (axis, color) in [
                    (0, [1.0, 0.24, 0.0, 0.7]),
                    (1, [0.29, 0.58, 0.0, 0.7]),
                    (2, [0.06, 0.61, 1.0, 0.7]),
                ] {
                    lines.extend(ring(center, axis, radius, color));
                }
            }
        }

        Tool::Clip => {
            let placed = clip.points();
            for point in &placed {
                points.push((*point, selected_color()));
            }
            // The cut itself, once the points describe one.
            if clip.can_clip() && placed.len() >= 2 {
                for pair in placed.windows(2) {
                    lines.push((pair[0], selected_color()));
                    lines.push((pair[1], selected_color()));
                }
            }
            // What the clip would keep, in the outline colour, and what it would throw
            // away in the same colour faded, so the chosen side reads before it is cut.
            if clip.can_clip() {
                let bounds = document.world_bounds();
                for brush in document.selected_brushes() {
                    for kept in clip.clip_brush(&bounds, brush) {
                        for (a, b) in kept.edge_positions() {
                            lines.push((a, CLIP_KEPT_COLOR));
                            lines.push((b, CLIP_KEPT_COLOR));
                        }
                    }
                    for removed in clip.clipped_away(brush) {
                        for (a, b) in removed.edge_positions() {
                            lines.push((a, CLIP_REMOVED_COLOR));
                            lines.push((b, CLIP_REMOVED_COLOR));
                        }
                    }
                }
            }
        }

        Tool::Shear => {
            if let Some(bounds) = document.selection_bounds() {
                for (_, center) in crate::shear_tool::handles(&bounds) {
                    points.push((center, handle_color()));
                }
                lines.extend(box_edges(&bounds, GUIDE_COLOR));
            }
        }

        Tool::Assemble => {
            for point in assemble.points() {
                points.push((*point, selected_color()));
            }
            // What the points have come to so far, so the shape is visible before it is
            // committed.
            for (from, to) in assemble.edges() {
                lines.push((from, GUIDE_COLOR));
                lines.push((to, GUIDE_COLOR));
            }
        }

        Tool::Sweep => {
            // Where the sweep starts from, so it is clear which faces it will take.
            if let Some(source) = document.sweep_source() {
                points.push((source.center, selected_color()));
                for face in &source.faces {
                    for pair in face.windows(2) {
                        lines.push((pair[0], GUIDE_COLOR));
                        lines.push((pair[1], GUIDE_COLOR));
                    }
                    if let (Some(first), Some(last)) = (face.first(), face.last()) {
                        lines.push((*last, GUIDE_COLOR));
                        lines.push((*first, GUIDE_COLOR));
                    }
                }

                // The destination cap, draggable, and a preview of the brushes the sweep
                // would make so the drag is judged by the result rather than the numbers.
                let destination = sweep.transform.destination_center(&source);
                points.push((destination, selected_color()));
                lines.push((source.center, GUIDE_COLOR));
                lines.push((destination, GUIDE_COLOR));

                if !sweep.transform.is_no_op() {
                    let preview = crate::sweep_tool::generate_brushes(
                        &source,
                        &sweep.transform,
                        &sweep.parameters,
                        document.current_material(),
                    );
                    for brush in &preview {
                        for (a, b) in brush.edge_positions() {
                            lines.push((a, SWEEP_PREVIEW_COLOR));
                            lines.push((b, SWEEP_PREVIEW_COLOR));
                        }
                    }
                }
            }
        }

        // These draw nothing of their own: the paint tool acts on the face under the
        // pointer, the brush tool's own preview is the shape being dragged out, and
        // selecting draws the selection itself.
        Tool::Select | Tool::Brush | Tool::Paint | Tool::Sculpt => {}
    }

    // The leak trace is drawn whatever the tool is: it is about the level rather than
    // about what is being edited.
    if let Some(trace) = document.leak_trace() {
        let colour = crate::preferences::pref(|p| p.point_file_color).rgba_f32();
        for pair in trace.points.windows(2) {
            lines.push((pair[0], colour));
            lines.push((pair[1], colour));
        }
    }

    // The portals a compiler wrote, drawn as the outlines of the polygons they are.
    if let Some(portals) = document.portals() {
        let colour = crate::preferences::pref(|p| p.portal_file_color).rgba_f32();
        for polygon in &portals.polygons {
            for (at, point) in polygon.iter().enumerate() {
                let next = polygon[(at + 1) % polygon.len()];
                lines.push((*point, colour));
                lines.push((next, colour));
            }
        }
    }

    (points, lines)
}

/// The twelve edges of a box, as segment endpoints.
fn box_edges(bounds: &Bbox3d, color: [f32; 4]) -> Vec<Marked> {
    let mut lines = Vec::with_capacity(24);
    for (first, second) in scale_tool::all_edges() {
        let (a, b) = scale_tool::points_for_bbox_edge(bounds, first, second);
        lines.push((a, color));
        lines.push((b, color));
    }
    lines
}

/// `SpikeGuideRenderer::maxLength`: how far a guide spike reaches.
const SPIKE_LENGTH: f64 = 1024.0;

/// The spikes `BoundsGuideRenderer` draws off the selection's box: from every corner, one
/// along each axis heading away from the box, so where the selection sits against the
/// rest of the map can be read along the world's own directions. Each spike is solid
/// for three quarters of its length and then fades, `SpikeGuideRenderer::addSpike`'s
/// `blendColor` at a mix of one half.
pub fn bounds_guide(bounds: &Bbox3d, color: [f32; 4]) -> Vec<Marked> {
    use geometry_math::bbox::Corner;
    let faded = [color[0], color[1], color[2], color[3] * 0.5];
    let mut lines = Vec::with_capacity(8 * 3 * 4);
    for corners in [
        [Corner::Min, Corner::Min, Corner::Min],
        [Corner::Min, Corner::Min, Corner::Max],
        [Corner::Min, Corner::Max, Corner::Min],
        [Corner::Min, Corner::Max, Corner::Max],
        [Corner::Max, Corner::Min, Corner::Min],
        [Corner::Max, Corner::Min, Corner::Max],
        [Corner::Max, Corner::Max, Corner::Min],
        [Corner::Max, Corner::Max, Corner::Max],
    ] {
        let origin = bounds.corner_position(corners);
        for axis in 0..3 {
            let mut direction = Vec3d::zero();
            direction[axis] = if corners[axis] == Corner::Min {
                -1.0
            } else {
                1.0
            };
            let knee = origin + direction * (SPIKE_LENGTH * 0.75);
            let tip = origin + direction * SPIKE_LENGTH;
            lines.push((origin, color));
            lines.push((knee, color));
            lines.push((knee, color));
            lines.push((tip, faded));
        }
    }
    lines
}

/// The spikes `PointGuideRenderer` draws from a handle: one along each world axis, both
/// ways, so where a vertex sits relative to everything else is readable while it is
/// dragged. Same shape as the bounds guide's, since both are `SpikeGuideRenderer`.
pub fn point_guide(position: Vec3d, color: [f32; 4]) -> Vec<Marked> {
    let faded = [color[0], color[1], color[2], color[3] * 0.5];
    let mut lines = Vec::with_capacity(6 * 4);
    for axis in 0..3 {
        for sign in [-1.0, 1.0] {
            let mut direction = Vec3d::zero();
            direction[axis] = sign;
            let knee = position + direction * (SPIKE_LENGTH * 0.75);
            let tip = position + direction * SPIKE_LENGTH;
            lines.push((position, color));
            lines.push((knee, color));
            lines.push((knee, color));
            lines.push((tip, faded));
        }
    }
    lines
}

/// A circle about `center` in the plane perpendicular to `axis`, as segment endpoints.
fn ring(center: Vec3d, axis: usize, radius: f64, color: [f32; 4]) -> Vec<Marked> {
    // The two axes the ring lies in are the ones that are not its own.
    let (u, v) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };

    let point_at = |step: usize| {
        let angle = step as f64 / RING_SEGMENTS as f64 * std::f64::consts::TAU;
        let mut point = center;
        point[u] += angle.cos() * radius;
        point[v] += angle.sin() * radius;
        point
    };

    let mut lines = Vec::with_capacity(RING_SEGMENTS * 2);
    for step in 0..RING_SEGMENTS {
        lines.push((point_at(step), color));
        lines.push((point_at(step + 1), color));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_tool::HandleKind;
    use crate::world::WorldBounds;
    use map_model::shapes;

    fn document_with_a_cube() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = shapes::create_cuboid(
            &world,
            &Bbox3d::from_scalars(-32.0, 32.0),
            shapes::DEFAULT_MATERIAL,
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
        document.select_all();
        document
    }

    #[test]
    fn the_point_guide_is_a_spike_along_each_axis_both_ways() {
        let at = Vec3d::new(16.0, -8.0, 4.0);
        let lines = point_guide(at, [1.0, 1.0, 0.0, 0.5]);
        assert_eq!(lines.len(), 6 * 4);
        let mut directions = Vec::new();
        for spike in lines.chunks(4) {
            assert_eq!(spike[0].0, at, "every spike starts at the handle");
            let outward = spike[3].0 - at;
            assert!((geometry_math::vec::length(outward) - SPIKE_LENGTH).abs() < 1e-9);
            directions.push(geometry_math::vec::normalize(outward));
            assert!(spike[3].1[3] < spike[0].1[3], "the far quarter fades");
        }
        // One each way along each axis, and nothing else.
        for axis in 0..3 {
            for sign in [-1.0, 1.0] {
                let mut wanted = Vec3d::zero();
                wanted[axis] = sign;
                assert!(
                    directions
                        .iter()
                        .any(|d| geometry_math::vec::is_equal(d, &wanted, 1e-9)),
                    "no spike along {wanted:?}"
                );
            }
        }
    }

    #[test]
    fn the_bounds_guide_is_a_spike_off_every_corner_along_every_axis() {
        let bounds = Bbox3d::from_scalars(-32.0, 32.0);
        let lines = bounds_guide(&bounds, [1.0, 0.0, 0.0, 0.35]);
        // Eight corners, three axes, two segments each.
        assert_eq!(lines.len(), 8 * 3 * 4);
        for spike in lines.chunks(4) {
            let (origin, knee, tip) = (spike[0].0, spike[1].0, spike[3].0);
            // Starts on a corner and heads away from the box.
            assert!(origin.v.iter().all(|c| c.abs() == 32.0), "{origin:?}");
            let outward = tip - origin;
            assert!(
                geometry_math::vec::dot(outward, origin) > 0.0,
                "{outward:?} points in"
            );
            assert!((geometry_math::vec::length(outward) - SPIKE_LENGTH).abs() < 1e-9);
            assert!((geometry_math::vec::length(knee - origin) - SPIKE_LENGTH * 0.75).abs() < 1e-9);
            // The far quarter fades.
            assert!(spike[3].1[3] < spike[0].1[3]);
        }
    }

    fn empty() -> (HandleManager, RotateTool, ClipTool, AssembleBrushTool) {
        (
            HandleManager::new(),
            RotateTool::new(),
            ClipTool::new(),
            AssembleBrushTool::new(),
        )
    }

    #[test]
    fn the_select_tool_draws_nothing() {
        let document = document_with_a_cube();
        let (handles, rotate, clip, assemble) = empty();
        let (points, lines) = build(
            &document,
            Tool::Select,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        assert!(points.is_empty());
        assert!(lines.is_empty());
    }

    #[test]
    fn a_handle_tool_draws_one_point_per_handle() {
        let document = document_with_a_cube();
        let (mut handles, rotate, clip, assemble) = empty();
        handles.collect(HandleKind::Vertex, document.selected_brushes().into_iter());

        let (points, lines) = build(
            &document,
            Tool::Vertex,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        // Eight corners, and the middle of every edge and centre of every face beside
        // them, which are what a Shift-drag splits.
        assert_eq!(points.len(), 8 + 12 + 6);
        assert!(lines.is_empty());
    }

    #[test]
    fn a_selected_handle_is_drawn_differently() {
        let document = document_with_a_cube();
        let (mut handles, rotate, clip, assemble) = empty();
        handles.collect(HandleKind::Vertex, document.selected_brushes().into_iter());
        let first = handles.handles()[0].position;
        handles.select(first, false);

        let (points, _) = build(
            &document,
            Tool::Vertex,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        let selected = points
            .iter()
            .filter(|(_, color)| *color == selected_color())
            .count();
        assert_eq!(selected, 1);
    }

    #[test]
    fn the_scale_tool_draws_its_handles_and_the_box_they_sit_on() {
        let document = document_with_a_cube();
        let (handles, rotate, clip, assemble) = empty();

        let (points, lines) = build(
            &document,
            Tool::Scale,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        // Six sides, twelve edges and eight corners.
        assert_eq!(points.len(), 26);
        // Twelve edges, two endpoints each.
        assert_eq!(lines.len(), 24);
    }

    #[test]
    fn the_rotate_tool_draws_a_ring_per_axis_around_its_centre() {
        let document = document_with_a_cube();
        let (handles, mut rotate, clip, assemble) = empty();
        rotate.set_center(Vec3d::zero());

        let radius = 64.0;
        let (points, lines) = build(
            &document,
            Tool::Rotate,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            radius,
        );
        assert_eq!(points.len(), 1);
        assert_eq!(lines.len(), 3 * RING_SEGMENTS * 2);

        // Every point of every ring is the radius it was given from the centre.
        for (point, _) in &lines {
            geometry_math::assert_approx_eq!(geometry_math::vec::length(*point), radius, 1e-9);
        }
    }

    #[test]
    fn the_shear_tool_draws_one_handle_per_face() {
        let document = document_with_a_cube();
        let (handles, rotate, clip, assemble) = empty();

        let (points, lines) = build(
            &document,
            Tool::Shear,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        assert_eq!(points.len(), 6);
        assert_eq!(lines.len(), 24);
    }

    #[test]
    fn the_clip_tool_draws_its_points_and_the_cut_between_them() {
        let document = document_with_a_cube();
        let (handles, rotate, mut clip, assemble) = empty();
        let looking_down = vec![Vec3d::new(0.0, 0.0, -1.0)];

        clip.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down.clone());
        let (points, lines) = build(
            &document,
            Tool::Clip,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        assert_eq!(points.len(), 1);
        // One point does not describe a plane, so there is no cut to draw yet.
        assert!(lines.is_empty());

        clip.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down);
        let (points, lines) = build(
            &document,
            Tool::Clip,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        assert_eq!(points.len(), 2);
        // The cut between the two points, once.
        let cut = lines
            .iter()
            .filter(|(_, colour)| *colour == selected_color())
            .count();
        assert_eq!(cut, 2, "one segment is two endpoints");
        // And the outlines of the halves the cut would keep and throw away.
        assert!(lines.iter().any(|(_, colour)| *colour == CLIP_KEPT_COLOR));
        assert!(lines
            .iter()
            .any(|(_, colour)| *colour == CLIP_REMOVED_COLOR));
    }

    #[test]
    fn the_clip_preview_needs_something_selected_to_outline() {
        let mut document = document_with_a_cube();
        document.clear_selection();
        let (handles, rotate, mut clip, assemble) = empty();
        let looking_down = vec![Vec3d::new(0.0, 0.0, -1.0)];
        clip.add_point(Vec3d::new(-32.0, 0.0, 0.0), looking_down.clone());
        clip.add_point(Vec3d::new(32.0, 0.0, 0.0), looking_down);

        let (_, lines) = build(
            &document,
            Tool::Clip,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &Default::default(),
            64.0,
        );
        // The cut line still shows; the half outlines do not, since nothing is selected.
        assert!(lines.iter().all(|(_, colour)| *colour == selected_color()));
    }

    #[test]
    fn the_sweep_tool_marks_the_destination_and_previews_the_brushes() {
        use crate::document::FaceRef;
        use crate::sweep_tool::SweepInputs;

        let mut document = document_with_a_cube();
        let (id, brush) = map_model::node::brushes_with_ids(document.tree_world())[0];
        let top = brush
            .faces()
            .iter()
            .position(|f| f.plane.normal.z() > 0.99)
            .expect("a top face");
        document.select_face_only(FaceRef {
            node: id,
            face: top,
        });

        let (handles, rotate, clip, assemble) = empty();

        // With no transform, only the source is marked -- the destination sits on it.
        let idle = SweepInputs::default();
        let (points, _) = build(
            &document,
            Tool::Sweep,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &idle,
            64.0,
        );
        assert_eq!(points.len(), 2, "source centre and destination, coincident");

        // Move the destination up and the preview brushes appear.
        let mut inputs = SweepInputs::default();
        inputs.transform.translation = Vec3d::new(0.0, 0.0, 128.0);
        let (points, lines) = build(
            &document,
            Tool::Sweep,
            &handles,
            &rotate,
            &clip,
            &assemble,
            &inputs,
            64.0,
        );
        let destination = points
            .iter()
            .find(|(p, _)| p.z() > 60.0)
            .expect("the destination cap is up where it was dragged");
        assert_eq!(destination.1, selected_color());
        assert!(
            lines
                .iter()
                .any(|(_, colour)| *colour == SWEEP_PREVIEW_COLOR),
            "the brushes the sweep would make are outlined"
        );
    }

    #[test]
    fn a_ring_lies_in_the_plane_of_its_axis() {
        let center = Vec3d::new(8.0, 16.0, 24.0);
        for axis in 0..3 {
            for (point, _) in ring(center, axis, 32.0, GUIDE_COLOR) {
                // The ring's own axis is the one that does not move.
                geometry_math::assert_approx_eq!(point[axis], center[axis], 1e-9);
            }
        }
    }
}
