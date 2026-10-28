//! The 3D map view without its widget: the frame it draws each time (uploads, the tool
//! overlay, the HUD), and everything its pointer and keys do. The frontend owns the GL
//! context and the renderer, binds the framebuffer and forwards its events.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::assemble_tool::AssembleBrushTool;
use crate::camera3d::Camera3d;
use crate::canvas::Canvas;
use crate::clip_tool::ClipTool;
use crate::document::{Document, FaceAttributeTransfer, FaceRef};
use crate::extrude_tool::{self, ExtrudeMode};
use crate::grid::Grid;
use crate::handle_drag::{self, HandlePicker};
use crate::handle_tool::{HandleKind, HandleManager};
use crate::i18n::{counted, fill};
use crate::input::{Key, Modifiers};
use crate::overlay;
use crate::preferences::pref;
use crate::renderer::Renderer;
use crate::rotate_tool::RotateTool;
use crate::scale_tool::{self, AnchorPos, BBoxHandle, ProportionalAxes};
use crate::shear_tool;
use crate::tool::Tool;
use crate::view_filters::ViewFilters;
use geometry_math::abstract_line::AbstractLine;
use geometry_math::bbox::Bbox3d;
use geometry_math::ray::Ray3d;
use geometry_math::vec::{self, Vec2d, Vec3d};
use map_model::brush::Brush;
use map_model::polyhedron::FaceId;

/// Snap the two surface coordinates and solve the remaining coordinate on the face.
pub fn snap_clip_surface(grid: &Grid, surface: Vec3d, normal: Vec3d) -> Vec3d {
    let mut at = grid.snap_point(surface);
    let axis = (0..3)
        .max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))
        .unwrap();
    if normal[axis].abs() > 1e-9 {
        at[axis] += vec::dot(normal, surface - at) / normal[axis];
    }
    at
}

pub fn clip_surface_point(
    document: &Document,
    grid: &Grid,
    ray: &Ray3d,
) -> Option<(Vec3d, Vec<Vec3d>)> {
    let hit = document.pick(ray)?;
    let face = document.face_at(FaceRef {
        node: hit.node,
        face: hit.face?,
    })?;
    let normal = face.normal();
    Some((
        snap_clip_surface(grid, ray.point_at_distance(hit.distance), normal),
        vec![normal],
    ))
}

/// How much faster the camera flies with Shift held, and slower with Alt.
///
/// `FlyModeHelper::SpeedModifier`.
pub const SPEED_MODIFIER: f64 = 2.0;

/// How far the camera turns per pixel of drag, in radians.
///
/// `CameraTool3D::lookSpeedH`, which is the preference over minus fifty; the sign is what
/// makes dragging right turn the view right.
pub fn turn_per_pixel() -> (f64, f64) {
    let speed = f64::from(pref(|p| p.camera_look_speed)) / -50.0;
    let horizontal = if pref(|p| p.invert_horizontal_look) {
        -speed
    } else {
        speed
    };
    let vertical = if pref(|p| p.invert_vertical_look) {
        -speed
    } else {
        speed
    };
    (horizontal, vertical)
}

/// What the frame about to be drawn is of.
///
/// Two frames with the same key are of the same view, and may be averaged together; two
/// with different keys may not. It covers the camera, the map, the filters, the size of
/// the window and the tool's own overlay, since a handle that has moved is a different
/// picture even when nothing else has changed.
pub fn frame_key(
    camera: &Camera3d,
    document: (u64, u64),
    filters: &ViewFilters,
    width: i32,
    height: i32,
    points: &[(Vec3d, [f32; 4])],
    guides: &[(Vec3d, [f32; 4])],
) -> u64 {
    let mut key = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |value: u64| {
        key ^= value;
        key = key.wrapping_mul(0x0000_0100_0000_01b3);
    };

    for part in [
        camera.position.x(),
        camera.position.y(),
        camera.position.z(),
        camera.yaw,
        camera.pitch,
        camera.fov(),
    ] {
        mix(part.to_bits());
    }
    mix(document.0);
    mix(document.1);
    mix(width as u64);
    mix(height as u64);
    // The filters are small and comparable, so their debug form is a cheap way to take
    // all of them without listing each one and forgetting the next one added.
    for byte in format!("{filters:?}").into_bytes() {
        mix(u64::from(byte));
    }
    for marks in [points, guides] {
        mix(marks.len() as u64);
        for (position, colour) in marks {
            for part in [position.x(), position.y(), position.z()] {
                mix(part.to_bits());
            }
            for channel in colour {
                mix(u64::from(channel.to_bits()));
            }
        }
    }
    key
}

/// What picture a frame of `frame` (its [`frame_key`]) comes out as: the frame, and what
/// the renderer draws it with besides -- the settings (colours, light, axes) and the
/// materials.
///
/// A frontend whose framebuffer keeps its contents between frames can show the last
/// picture again while this stays the same, rather than drawing it again.
pub fn picture_key(frame: u64, settings: &render::RenderSettings, materials_revision: u64) -> u64 {
    let mut key = frame ^ 0x9e37_79b9_7f4a_7c15;
    let mut mix = |value: u64| {
        key ^= value;
        key = key.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(materials_revision);
    // As with the filters in `frame_key`: every knob, without listing each one.
    for byte in format!("{settings:?}").into_bytes() {
        mix(u64::from(byte));
    }
    key
}

/// Whether a kept framebuffer that holds the picture `drawn` has to be drawn again to
/// show `picture`: it holds another picture (or none), the still view is still gathering
/// samples (`refining`), or an animation plays, which moves without the key moving.
pub fn needs_draw(drawn: Option<u64>, picture: u64, refining: bool, playing: bool) -> bool {
    drawn != Some(picture) || refining || playing
}

/// How far the camera slides per pixel of drag, along each screen axis.
///
/// `CameraTool3D::panSpeedH` and `panSpeedV`, which are the preference with the two
/// invert flags applied.
pub fn pan_per_pixel() -> (f64, f64) {
    let speed = f64::from(pref(|p| p.camera_pan_speed));
    let horizontal = if pref(|p| p.invert_horizontal_pan) {
        -speed
    } else {
        speed
    };
    let vertical = if pref(|p| p.invert_vertical_pan) {
        -speed
    } else {
        speed
    };
    (horizontal, vertical)
}

/// The current handle position for drags that move one handle. Other operations draw their own
/// outlines.
pub fn dragged_handle(drag: &Drag) -> Option<Vec3d> {
    match &drag.what {
        DragKind::Handles { .. } => Some(drag.initial + drag.applied),
        DragKind::SplitHandle { at } => Some(*at + drag.applied),
        _ => None,
    }
}

/// What the HUD over the 3D view draws: the compass and the selection's sizes, already
/// projected onto the viewport. Made each GL frame and compared with the last, so the
/// HUD is only drawn again when something in it moved.
#[derive(PartialEq)]
pub struct RotationFeedback {
    pub ring: Vec<Vec2d>,
    pub sector: Vec<Vec2d>,
    pub color: [f64; 3],
    pub label: Option<(String, Vec2d)>,
}

impl RotationFeedback {
    pub fn new(
        camera: &Camera3d,
        width: f64,
        height: f64,
        gesture: &render::rotation_gizmo::RingDrag,
        radius: f64,
        angle: Option<f64>,
    ) -> Option<Self> {
        let center = gesture.center;
        let radial = gesture.reference;
        let tangent = geometry_math::vec::cross(gesture.axis, radial);
        let project = |a: f64| {
            camera.project(
                center + (radial * a.cos() + tangent * a.sin()) * radius,
                width,
                height,
            )
        };
        let ring = (0..=64)
            .map(|i| project(i as f64 * std::f64::consts::TAU / 64.0))
            .collect::<Option<Vec<_>>>()?;
        let axis =
            (0..3).max_by(|&a, &b| gesture.axis[a].abs().total_cmp(&gesture.axis[b].abs()))?;
        let color = [[1.0, 0.35, 0.16], [0.48, 0.85, 0.16], [0.18, 0.7, 1.0]][axis];
        let mut sector = Vec::new();
        let label = if let Some(angle) = angle {
            sector.push(camera.project(center, width, height)?);
            for i in 0..=64 {
                sector.push(project(angle * i as f64 / 64.0)?);
            }
            let anchor = project(angle)?;
            Some((
                format!("{}  {:.1}°", ["X", "Y", "Z"][axis], angle.to_degrees()),
                anchor,
            ))
        } else {
            None
        };
        Some(Self {
            ring,
            sector,
            color,
            label,
        })
    }

    pub fn draw(&self, cr: &mut dyn Canvas) {
        cr.save();
        let path = |cr: &mut dyn Canvas, points: &[Vec2d]| {
            cr.new_path();
            if let Some(first) = points.first() {
                cr.move_to(first.x(), first.y());
                for p in &points[1..] {
                    cr.line_to(p.x(), p.y());
                }
            }
        };
        let [r, g, b] = self.color;
        path(cr, &self.ring);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.65);
        cr.set_line_width(5.0);
        cr.stroke_preserve();
        cr.set_source_rgb(r, g, b);
        cr.set_line_width(3.0);
        cr.stroke();
        if !self.sector.is_empty() {
            path(cr, &self.sector);
            cr.close_path();
            cr.set_source_rgba(r, g, b, 0.18);
            cr.fill_preserve();
            cr.set_source_rgba(r, g, b, 0.95);
            cr.set_line_width(1.5);
            cr.stroke();
        }
        if let Some((text, anchor)) = &self.label {
            crate::view2d::draw_label(
                cr,
                text,
                *anchor,
                crate::size_labels::Alignment::Bottom,
                24.0,
                crate::view2d::LabelStyle::Picked,
            );
        }
        cr.restore();
    }
}

#[derive(Default, PartialEq)]
pub struct Hud {
    /// Hidden while an animation plays: its guides refer to the saved geometry.
    pub hidden: bool,
    pub rotation: Option<RotationFeedback>,
    pub render_aspect: Option<f64>,
    pub arrows: Vec<crate::compass::Arrow>,
    pub cameras: Vec<(u32, String, Vec2d)>,
    /// What to write, where on the viewport, which side of that point, how far off it,
    /// and in which colours.
    pub labels: Vec<(
        String,
        Vec2d,
        crate::size_labels::Alignment,
        f64,
        crate::view2d::LabelStyle,
    )>,
}

/// How large a handle is, in pixels, for picking it with a ray.
///
/// The same size the renderer draws them at, so what looks like a handle is what can be
/// grabbed.
pub fn handle_radius() -> f64 {
    f64::from(pref(|p| p.handle_radius))
}

/// How near the ray has to pass a handle to take hold of it.
///
/// The same number the handle is drawn at, as in a flat view, so that what is seen is what
/// can be clicked.
pub fn pick_radius() -> f64 {
    handle_radius() + 2.0
}

/// A drag of a handle in the 3D view.
pub struct Drag {
    what: DragKind,
    /// The surface the handle slides along.
    picker: HandlePicker,
    /// Where the handle was when the drag began.
    initial: Vec3d,
    /// Where the press landed relative to the handle, so the handle does not jump under
    /// the pointer.
    offset: Vec3d,
    /// How far the handle has been moved so far.
    applied: Vec3d,
}

pub enum DragKind {
    /// Vertex, edge or face handles: the positions they move, as they were at the press.
    Handles {
        vertices: Vec<Vec3d>,
        /// The handle the press took hold of, and whether it was picked before.
        caught: Vec3d,
        was_picked: bool,
        /// Whether the handle is to be the only one picked once the press becomes a drag.
        /// A Ctrl-press adds an unpicked handle to what is picked, which is what a
        /// Ctrl-click does, but a drag of it moves it alone (`NodeHandleToolBase::
        /// startMove`).
        narrow: bool,
    },
    /// The middle of an edge or the centre of a face, taken with Shift by the vertex tool:
    /// the drag puts a corner where it goes.
    SplitHandle { at: Vec3d },
    /// A bounding-box handle, and the box it started from.
    Scale {
        handle: BBoxHandle,
        start_bounds: Bbox3d,
    },
    /// The rotation ring about the given axis.
    Rotate {
        gesture: render::rotation_gizmo::RingDrag,
    },
    /// A face of the bounding box, slid across itself.
    Shear {
        normal: Vec3d,
        side_center: Vec3d,
        start_bounds: Bbox3d,
    },
    /// A face of a selected brush, dragged along its own normal.
    Extrude {
        /// The face's vertices when the drag began, which is how the same face is found
        /// again on every brush that shares it.
        polygon: Vec<Vec3d>,
        normal: Vec3d,
        /// The brush and face the drag began on, for snapping the distance.
        brush: Brush,
        face: FaceId,
        mode: ExtrudeMode,
    },
    /// The sweep tool's destination cap. Nothing in the document moves; the drag only
    /// changes the shared sweep transform, which the preview and the panel follow.
    SweepCap {
        /// The translation when the drag began, which the delta is added to.
        start: Vec3d,
    },
    /// Sweeping out a new brush from empty space, `DrawShapeToolController3D`. The second
    /// corner slides on the horizontal plane through the first.
    Draw {
        /// The first corner, where the press landed.
        start: Vec3d,
    },
    /// Moving the whole selection, `MoveObjectsToolController`. Slides across the ground by
    /// default, straight up with Alt, along the dominant axis with Shift; with Ctrl at the
    /// press it moves a copy.
    MoveObjects { duplicate: crate::view2d::Duplicate },
    /// A press that only clicks: it selects on release if it stays put, and a drag does
    /// nothing (`SelectionTool::mouseClick`).
    Click,
    /// The rotate tool's centre, moved on the ground or straight up with Alt
    /// (`RotateToolController`'s centre handle).
    RotationCenter,
    /// A Ctrl-drag from something unselected adds whatever it passes over to the
    /// selection, faces with Shift as well (`PaintSelectionDragTracker`). Let go where it
    /// was pressed, it is a Ctrl-click.
    PaintSelect {
        faces: bool,
        /// Whether what the press was on still has to be added, which happens once the
        /// press has become a drag.
        pending: bool,
        /// The selection at the press, which Esc puts back.
        before: crate::document::SelectionSnapshot,
        /// The edge or face handle the press was on, which a Ctrl-click picks or
        /// unpicks. Those tools do not move a handle with Ctrl held, so the drag goes
        /// to the selection (`NodeHandleToolMovePartBase::shouldStartMove`).
        handle: Option<Vec3d>,
    },
    /// The assemble tool's rectangle, drawn on the face the press landed on
    /// (`AssembleBrushToolController3D`'s `DrawFacePart`).
    AssembleFace {
        /// The plane of that face, which the rectangle's corners lie in.
        plane: geometry_math::plane::Plane3d,
        /// The assemble tool's points when the press happened.
        before: Vec<Vec3d>,
    },
    /// A copy of the assemble tool's polygon, pulled out along its normal with Shift
    /// (`DuplicateFacePart`), which makes a prism of it.
    AssembleExtrude {
        /// The polygon's corners, in order around it.
        polygon: Vec<Vec3d>,
        /// The polygon's normal, facing the camera.
        normal: Vec3d,
        /// The assemble tool's points when the press happened.
        before: Vec<Vec3d>,
    },
}

/// The box a drawn brush would fill, from the two corners a 3D draw drag has swept out.
///
/// `DrawShapeToolController3D::makeBounds`: the corners are snapped apart (the low one
/// down, the high one up), and an axis the two agree on -- the vertical, for a drag across
/// the ground -- is given one grid step, growing away from the camera so the brush stands
/// in front of it.
pub fn draw_bounds(
    start: Vec3d,
    corner: Vec3d,
    grid: &Grid,
    camera_direction: Vec3d,
) -> Option<Bbox3d> {
    let mut min = grid.snap_down_point(vec::min(start, corner));
    let mut max = grid.snap_up_point(vec::max(start, corner));
    let step = grid.actual_size();
    for axis in 0..3 {
        if max[axis] <= min[axis] {
            if camera_direction[axis] <= 0.0 {
                max[axis] = min[axis] + step;
            } else {
                min[axis] = max[axis] - step;
            }
        }
    }
    if (max - min).v.iter().any(|extent| *extent <= 0.0) {
        return None;
    }
    Some(Bbox3d::new(min, max))
}

/// [`draw_bounds`] with the modifiers of `DrawShapeToolController3D::makeBounds`: Shift
/// makes the base square, and with Alt a cube, growing away from where the drag began;
/// the box is kept inside the world.
pub fn shaped_draw_bounds(
    start: Vec3d,
    corner: Vec3d,
    grid: &Grid,
    camera_direction: Vec3d,
    modifiers: Modifiers,
    world: &Bbox3d,
) -> Option<Bbox3d> {
    let bounds = draw_bounds(start, corner, grid, camera_direction)?;
    let bounds = if modifiers.shift {
        let size = bounds.max - bounds.min;
        let (axes, rest) = if modifiers.alt {
            (Vec3d::one(), Vec3d::zero())
        } else {
            (Vec3d::new(1.0, 1.0, 0.0), Vec3d::new(0.0, 0.0, 1.0))
        };
        let longest = (size * axes).get_abs_max_component(0);
        let extent = rest * size + axes * longest;
        let direction = vec::step(start, corner);
        Bbox3d::new(
            vec::mix(bounds.min, bounds.max - extent, Vec3d::one() - direction),
            vec::mix(bounds.max, bounds.min + extent, direction),
        )
    } else {
        bounds
    };
    bounds.intersected(world)
}

/// How far the pointer must move before a press counts as a drag, in pixels.
///
/// `InputEventRecorder`'s own threshold: a hand shakes, and a click that moved a brush by
/// a grid step because of it is worse than a drag that starts three pixels late.
pub const DRAG_THRESHOLD: f64 = 3.0;

/// How far down the ray the first corner of a drawn brush sits when the pointer is over
/// nothing, `Camera::DefaultPointDistance`.
pub const DEFAULT_DRAW_DISTANCE: f64 = 256.0;

/// How often a sculpt stroke dabs again while the pointer rests, in seconds: holding the
/// button still keeps raising, as clicking again would.
pub const HOLD_DAB_INTERVAL: f64 = 0.1;

/// Whether a point on screen is inside the rectangle a lasso swept out.
pub fn inside(from: Vec2d, to: Vec2d, point: Vec2d) -> bool {
    let (left, right) = (from.x().min(to.x()), from.x().max(to.x()));
    let (top, bottom) = (from.y().min(to.y()), from.y().max(to.y()));
    point.x() >= left && point.x() <= right && point.y() >= top && point.y() <= bottom
}

/// What a lasso has caught: every brush whose corners are all drawn inside the rectangle.
///
/// A brush is enclosed only when every corner projects inside the rectangle. Corners behind the
/// camera exclude the brush.
pub fn lassoed(
    camera: &Camera3d,
    document: &Document,
    from: Vec2d,
    to: Vec2d,
    width: f64,
    height: f64,
) -> Vec<map_model::node::NodeId> {
    let left = from.x().min(to.x());
    let right = from.x().max(to.x());
    let top = from.y().min(to.y());
    let bottom = from.y().max(to.y());

    document
        .selectable_brushes()
        .into_iter()
        .filter(|(_, brush)| {
            brush.vertex_positions().iter().all(|corner| {
                camera
                    .project(*corner, width, height)
                    .is_some_and(|on_screen| {
                        on_screen.x() >= left
                            && on_screen.x() <= right
                            && on_screen.y() >= top
                            && on_screen.y() <= bottom
                    })
            })
        })
        .map(|(id, _)| id)
        .collect()
}

/// Works out what a press has hold of, if anything.
#[allow(
    clippy::too_many_arguments,
    reason = "one press consults every tool's state"
)]
pub fn begin_drag(
    camera: &Camera3d,
    viewport_height: f64,
    ray: &Ray3d,
    tool: Tool,
    document: &Document,
    handles: &mut HandleManager,
    rotate: &RotateTool,
    sweep: &crate::sweep_tool::SweepInputs,
    shift: bool,
    ctrl: bool,
    alt: bool,
) -> Option<Drag> {
    // Shift extrudes with the Select or Brush tool. A handle tool splits vertices with it,
    // and Rotate, Scale, Shear, Sweep, Clip and Assemble switch extruding off
    // (`MapViewToolBox`), Scale for its own proportional Shift.
    if shift && matches!(tool, Tool::Select | Tool::Brush) {
        if let Some(drag) = begin_extrude(camera, viewport_height, ray, document, ctrl, alt) {
            return Some(drag);
        }
    }

    match tool {
        // A fillet press rounds an edge rather than taking hold of it, which the press
        // handler has already done by the time this is reached.
        Tool::Fillet => None,

        Tool::Vertex | Tool::Edge | Tool::Face | Tool::ControlPoint => {
            let positions: Vec<Vec3d> = handles.movable().map(|handle| handle.position).collect();
            let caught = handle_drag::nearest_handle(
                camera,
                viewport_height,
                ray,
                positions,
                pick_radius(),
            )?;

            if ctrl && matches!(tool, Tool::Edge | Tool::Face) {
                return Some(Drag {
                    what: DragKind::PaintSelect {
                        faces: false,
                        pending: true,
                        before: document.selection_snapshot(),
                        handle: Some(caught),
                    },
                    picker: HandlePicker::plane(
                        geometry_math::plane::horizontal_plane(Vec3d::zero()),
                        Vec3d::zero(),
                    ),
                    initial: caught,
                    offset: Vec3d::zero(),
                    applied: Vec3d::zero(),
                });
            }

            // Dragging a handle that is not selected makes it the selection first, so a
            // drag always moves what is under the pointer. Ctrl adds it to what is picked
            // until the press becomes a drag, so a Ctrl-click still adds it.
            let was_picked = handles.is_selected(caught);
            if !was_picked {
                handles.select(caught, ctrl);
            }

            let offset = offset_of(ray, camera, viewport_height, caught);
            Some(Drag {
                what: DragKind::Handles {
                    vertices: handles.selected_vertices(),
                    caught,
                    was_picked,
                    narrow: ctrl && !was_picked,
                },
                picker: handle_drag::picker_for_move(
                    handle_drag::MoveMode::Default,
                    caught,
                    caught,
                    offset,
                ),
                initial: caught,
                offset,
                applied: Vec3d::zero(),
            })
        }

        Tool::Scale => {
            let bounds = document.selection_bounds()?;
            let candidates: Vec<(BBoxHandle, Vec3d)> = scale_tool::all_handles()
                .into_iter()
                .map(|handle| (handle, handle.position(&bounds)))
                .collect();
            let caught = handle_drag::nearest_handle(
                camera,
                viewport_height,
                ray,
                candidates.iter().map(|(_, position)| *position),
                handle_radius(),
            )?;
            let (handle, _) = candidates
                .into_iter()
                .find(|(_, position)| *position == caught)?;

            let offset = offset_of(ray, camera, viewport_height, caught);
            Some(Drag {
                what: DragKind::Scale {
                    handle,
                    start_bounds: bounds,
                },
                // A scale handle runs along the line from the handle across the box.
                picker: HandlePicker::line(handle.drag_line(&bounds), offset),
                initial: caught,
                offset,
                applied: Vec3d::zero(),
            })
        }

        Tool::Rotate => {
            let center = rotate.center()?;
            // The same constant-on-screen radius the rings are drawn at, so a press lands
            // on the ring it looks like it lands on.
            let radius = f64::from(pref(|p| p.rotate_handle_radius))
                * handle_drag::perspective_scaling_factor(camera, center, viewport_height);

            let gesture = render::rotation_gizmo::RingDrag::pick(
                camera,
                viewport_height,
                ray,
                center,
                radius,
                pick_radius().max(6.0),
            );
            let Some(gesture) = gesture else {
                // Off the rings, the centre handle: seen edge-on a ring passes through the
                // centre, and the ring wins there.
                if !shift && !ctrl {
                    if let Some(caught) = handle_drag::nearest_handle(
                        camera,
                        viewport_height,
                        ray,
                        vec![center],
                        pick_radius(),
                    ) {
                        let offset = offset_of(ray, camera, viewport_height, caught);
                        let mode = handle_drag::move_mode(alt, false, caught, caught);
                        return Some(Drag {
                            what: DragKind::RotationCenter,
                            picker: handle_drag::picker_for_move(mode, caught, caught, offset),
                            initial: caught,
                            offset,
                            applied: Vec3d::zero(),
                        });
                    }
                }
                return None;
            };
            Some(Drag {
                what: DragKind::Rotate { gesture },
                picker: HandlePicker::circle(center, gesture.axis, radius, Vec3d::zero()),
                initial: center,
                offset: Vec3d::zero(),
                applied: Vec3d::zero(),
            })
        }

        Tool::Shear => {
            let bounds = document.selection_bounds()?;
            let sides = shear_tool::handles(&bounds);
            let caught = handle_drag::nearest_handle(
                camera,
                viewport_height,
                ray,
                sides.iter().map(|(_, center)| *center),
                handle_radius(),
            )?;
            let (normal, side_center) = sides.into_iter().find(|(_, center)| *center == caught)?;

            let offset = offset_of(ray, camera, viewport_height, caught);
            let picker =
                shear_tool::picker(normal, side_center, offset, true, camera.direction(), false)?;
            Some(Drag {
                what: DragKind::Shear {
                    normal,
                    side_center,
                    start_bounds: bounds,
                },
                picker,
                initial: caught,
                offset,
                applied: Vec3d::zero(),
            })
        }

        Tool::Sweep => {
            let source = document.sweep_source()?;
            let destination = sweep.transform.destination_center(&source);
            // Only a press that landed on the cap starts a drag; otherwise the press is a
            // selection, as it is in the other view tools.
            handle_drag::pick_point_handle(
                camera,
                viewport_height,
                ray,
                destination,
                handle_radius(),
            )?;
            let offset = offset_of(ray, camera, viewport_height, destination);
            Some(Drag {
                what: DragKind::SweepCap {
                    start: sweep.transform.translation,
                },
                picker: handle_drag::picker_for_move(
                    handle_drag::MoveMode::Default,
                    destination,
                    destination,
                    offset,
                ),
                initial: destination,
                offset,
                applied: Vec3d::zero(),
            })
        }

        Tool::Select | Tool::Brush | Tool::Clip | Tool::Assemble | Tool::Paint | Tool::Sculpt => {
            None
        }
    }
}

/// What a click with these modifiers copies, if anything.
///
/// The three combinations `SetBrushFaceAttributesTool` reads, and nothing else: the test
/// is for the exact set of modifiers, so Alt with something unrelated held is not a copy.
pub fn transfer_mode(modifiers: Modifiers) -> Option<FaceAttributeTransfer> {
    let held = (modifiers.alt, modifiers.control, modifiers.shift);
    if held == (true, true, false) {
        Some(FaceAttributeTransfer::MaterialOnly)
    } else if held == (true, false, false) {
        Some(FaceAttributeTransfer::Projection)
    } else if held == (true, false, true) {
        Some(FaceAttributeTransfer::Rotation)
    } else {
        None
    }
}

/// A run of the attribute-transfer drag: the reference face, what it carries, and the
/// faces the pointer has swept across so far.
pub enum AttrStroke {
    Transfer {
        source: FaceRef,
        mode: FaceAttributeTransfer,
        touched: Vec<FaceRef>,
        mark: usize,
        revision: u64,
    },
    Paint {
        state: crate::document::MaterialStroke,
        last: (f64, f64),
    },
    /// A drag of the sculpt brush: where the last dab fell, whether Shift swapped raise
    /// and lower when it began, and how long the pointer has rested since that dab, in
    /// seconds.
    Sculpt {
        state: crate::document::SculptStroke,
        last: Vec3d,
        inverted: bool,
        rested: f64,
    },
}

/// The sculpt brush for a stroke begun with Shift held or not.
fn sculpt_brush(inverted: bool) -> crate::sculpt::SculptParameters {
    let mut parameters = crate::sculpt::parameters();
    if inverted {
        parameters.mode = parameters.mode.inverted();
    }
    parameters
}

/// The brush face the pointer is over, if any.
pub fn face_under(
    camera: &Camera3d,
    (width, height): (f64, f64),
    document: &Document,
    x: f64,
    y: f64,
) -> Option<FaceRef> {
    let ray = camera.ray_through(x, y, width, height);
    let hit = document.pick(&ray)?;
    Some(FaceRef {
        node: hit.node,
        face: hit.face?,
    })
}

/// Works out whether a Shift-press landed on a face of a selected brush.
pub fn begin_extrude(
    camera: &Camera3d,
    viewport_height: f64,
    ray: &Ray3d,
    document: &Document,
    ctrl: bool,
    alt: bool,
) -> Option<Drag> {
    // Only the selection counts, as `ResizeBrushesTool` picks it: a brush in front of the
    // selected one, or the floor a crate stands on, does not take the press.
    let (node, face_index, hit_point) = match document
        .pick_selected(ray)
        .and_then(|hit| Some((hit.node, hit.face?, ray.point_at_distance(hit.distance))))
    {
        Some(hit) => hit,
        None => proximate_face(camera, viewport_height, ray, document)?,
    };

    let brush = document.brush(node)?.clone();
    let polygon = brush.face_polygon(face_index)?;
    let normal = brush.faces()[face_index].plane.normal;
    let face = brush.geometry_face_of(face_index)?;

    let offset = offset_of(ray, camera, viewport_height, hit_point);

    Some(Drag {
        what: DragKind::Extrude {
            polygon,
            normal,
            brush,
            face,
            mode: extrude_tool::mode_for(ctrl, alt),
        },
        picker: extrude_tool::picker(hit_point, normal, offset),
        initial: hit_point,
        offset,
        applied: Vec3d::zero(),
    })
}

/// A press just off a selected brush, near one of its edges: the face behind that edge,
/// facing away from the camera, so a box's side can be dragged from its outline even where
/// the brush it stands on fills the view behind. As `ResizeBrushesTool::pickProximateFace`.
/// Returns the brush, the face and the point on the edge nearest the ray.
fn proximate_face(
    camera: &Camera3d,
    viewport_height: f64,
    ray: &Ray3d,
    document: &Document,
) -> Option<(map_model::node::NodeId, usize, Vec3d)> {
    let mut best: Option<(f64, map_model::node::NodeId, usize, Vec3d)> = None;
    for (node, brush) in document.selected_brushes_with_ids() {
        let polygons: Vec<Vec<Vec3d>> = (0..brush.faces().len())
            .map(|index| brush.face_polygon(index).unwrap_or_default())
            .collect();
        for (index, polygon) in polygons.iter().enumerate() {
            // Only a face turned away from the camera: its edges outline the brush, and it
            // is the one hidden behind it.
            if vec::dot(brush.faces()[index].plane.normal, ray.direction) <= 0.0 {
                continue;
            }
            for i in 0..polygon.len() {
                let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
                // The edge has to divide this face from one facing the camera.
                let outline = polygons.iter().enumerate().any(|(other, corners)| {
                    other != index
                        && vec::dot(brush.faces()[other].plane.normal, ray.direction) < 0.0
                        && [a, b]
                            .iter()
                            .all(|p| corners.iter().any(|q| vec::length(*q - *p) < 1e-6))
                });
                if !outline {
                    continue;
                }
                let near = nearest_on_segment(ray, a, b);
                let Some(distance) = handle_drag::pick_point_handle(
                    camera,
                    viewport_height,
                    ray,
                    near,
                    pick_radius(),
                ) else {
                    continue;
                };
                if best.as_ref().is_none_or(|(d, ..)| distance < *d) {
                    best = Some((distance, node, index, near));
                }
            }
        }
    }
    best.map(|(_, node, index, near)| (node, index, near))
}

/// The point of the segment from `a` to `b` nearest the ray.
fn nearest_on_segment(ray: &Ray3d, a: Vec3d, b: Vec3d) -> Vec3d {
    let along = b - a;
    let across = a - ray.origin;
    let (aa, ab, bb) = (
        vec::dot(along, along),
        vec::dot(along, ray.direction),
        vec::dot(ray.direction, ray.direction),
    );
    let (ad, bd) = (vec::dot(along, across), vec::dot(ray.direction, across));
    let denominator = aa * bb - ab * ab;
    let t = if denominator.abs() < 1e-12 {
        0.0
    } else {
        (ab * bd - bb * ad) / denominator
    };
    a + along * t.clamp(0.0, 1.0)
}

/// Where the press landed relative to the handle it caught.
pub fn offset_of(ray: &Ray3d, camera: &Camera3d, viewport_height: f64, handle: Vec3d) -> Vec3d {
    match handle_drag::pick_point_handle(camera, viewport_height, ray, handle, handle_radius()) {
        Some(distance) => handle - ray.point_at_distance(distance),
        None => Vec3d::zero(),
    }
}

/// Moves whatever the drag has hold of. Returns whether anything changed.
#[allow(
    clippy::too_many_arguments,
    reason = "one drag step touches every tool's state"
)]
pub fn update_drag(
    drag: &mut Drag,
    ray: &Ray3d,
    camera_direction: Vec3d,
    grid: &Grid,
    document: &mut Document,
    handles: &mut HandleManager,
    rotate: &mut RotateTool,
    sweep: &mut crate::sweep_tool::SweepInputs,
    modifiers: Modifiers,
) -> bool {
    let alt = modifiers.alt;
    let shift = modifiers.shift;
    let ctrl = modifiers.control;

    if let DragKind::Handles {
        vertices,
        caught,
        narrow,
        ..
    } = &mut drag.what
    {
        if std::mem::take(narrow) {
            handles.select(*caught, false);
            *vertices = handles.selected_vertices();
        }
    }

    match &drag.what {
        DragKind::SplitHandle { at } => {
            let at = *at;
            let current = drag.initial + drag.applied;
            let mode = handle_drag::move_mode(alt, false, drag.initial, current);
            drag.picker = handle_drag::picker_for_move(mode, drag.initial, current, drag.offset);

            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let total = if ctrl {
                grid.snap_point(proposed) - drag.initial
            } else {
                grid.snap_point(proposed - drag.initial)
            };
            if total == drag.applied {
                return false;
            }

            // Measured from the brush as it was, so the corner follows the pointer.
            document.rollback_transaction();
            if document.split_handle(at, total) == 0 {
                return false;
            }
            drag.applied = total;
            true
        }

        DragKind::Handles { vertices, .. } => {
            // Modifiers can change the drag constraint while the gesture is active.
            let current = drag.initial + drag.applied;
            let mode = handle_drag::move_mode(alt, shift, drag.initial, current);
            drag.picker = handle_drag::picker_for_move(mode, drag.initial, current, drag.offset);

            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            // `makeRelativeHandleSnapper`, or the absolute one on Ctrl, which only the vertex
            // tool allows (`VertexTool::allowAbsoluteSnapping`).
            let total = if ctrl && handles.kind() == Some(HandleKind::Vertex) {
                grid.snap_point(proposed) - drag.initial
            } else {
                grid.snap_point(proposed - drag.initial)
            };
            let step = total - drag.applied;
            if step == Vec3d::zero() {
                return false;
            }

            document.rollback_transaction();
            // A control point is not a brush vertex, so which of the two moves depends on
            // what the handles are of.
            let moved = if handles.kind() == Some(HandleKind::ControlPoint) {
                document.move_control_points(vertices, total)
            } else {
                document.move_handles(vertices, total)
            };
            if !moved {
                return false;
            }
            handles.move_selection(step);
            drag.applied = total;
            true
        }

        DragKind::Scale {
            handle,
            start_bounds,
        } => {
            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let delta = grid.snap_point(proposed) - drag.initial;

            let proportional = if shift {
                ProportionalAxes::all()
            } else {
                ProportionalAxes::none()
            };
            let anchor = if alt {
                AnchorPos::Center
            } else {
                AnchorPos::Opposite
            };
            let Some(new_bounds) = handle.drag(start_bounds, delta, proportional, anchor) else {
                return false;
            };

            document.rollback_transaction();
            let transform = geometry_math::mat_ext::scale_bbox_matrix(start_bounds, &new_bounds);
            document.transform_selection(&transform);
            true
        }

        DragKind::Shear {
            normal,
            side_center,
            start_bounds,
        } => {
            // Alt can be pressed part way through, which changes what the face slides
            // along, so the picker is rebuilt each step.
            let Some(picker) = shear_tool::picker(
                *normal,
                *side_center,
                drag.offset,
                true,
                camera_direction,
                alt,
            ) else {
                return false;
            };
            drag.picker = picker;

            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let delta = grid.snap_point(proposed - drag.initial);

            document.rollback_transaction();
            let Some(transform) =
                geometry_math::mat_ext::shear_bbox_matrix(start_bounds, *normal, delta)
            else {
                return false;
            };
            document.transform_selection(&transform);
            true
        }

        DragKind::Extrude {
            polygon,
            normal,
            brush,
            face,
            mode,
        } => {
            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let snapped = extrude_tool::snap_handle_position(
                grid,
                brush,
                *face,
                *normal,
                drag.initial,
                proposed,
            );
            let delta = snapped - drag.initial;
            if delta == Vec3d::zero() {
                return false;
            }

            // Every step starts again from the brushes as they were, so the result depends
            // only on where the pointer is and not on how it got there.
            document.rollback_transaction();
            // How far the face went, so the release can tell a drag from a click.
            drag.applied = delta;
            match mode {
                ExtrudeMode::Extrude => document.extrude_faces(polygon, delta),
                ExtrudeMode::Slide => document.slide_faces(polygon, delta),
                ExtrudeMode::Split => document.split_faces(polygon, delta),
                ExtrudeMode::Stamp => document.stamp_faces(polygon, delta),
            }
        }

        DragKind::Rotate { gesture } => {
            let Some(raw_angle) = gesture.angle(ray) else {
                return false;
            };
            let angle = rotate.set_drag_angle(grid, raw_angle);
            if (angle - drag.applied.x()).abs() < 1e-9 {
                return false;
            }
            document.rollback_transaction();
            document.rotate_selection_about(gesture.center, gesture.axis, angle.to_degrees());
            drag.applied = Vec3d::new(angle, 0.0, 0.0);
            true
        }

        DragKind::SweepCap { start } => {
            let start = *start;
            // The cap slides on the ground by default, straight up with Alt, along the
            // dominant axis with Shift -- the same move modes as a handle.
            let current = drag.initial + drag.applied;
            let mode = handle_drag::move_mode(alt, shift, drag.initial, current);
            drag.picker = handle_drag::picker_for_move(mode, drag.initial, current, drag.offset);

            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let total = if ctrl {
                grid.snap_point(proposed) - drag.initial
            } else {
                grid.snap_point(proposed - drag.initial)
            };
            if total == drag.applied {
                return false;
            }
            drag.applied = total;
            sweep.transform.translation = start + total;
            true
        }

        DragKind::MoveObjects { duplicate } => {
            let duplicate = *duplicate;
            // The modifier keys can change what the move is held to part way through:
            // Alt lifts it off the ground, Shift holds it to the axis it has gone
            // furthest in. Same as a handle drag.
            let current = drag.initial + drag.applied;
            let mode = handle_drag::move_mode(alt, shift, drag.initial, current);
            drag.picker = handle_drag::picker_for_move(mode, drag.initial, current, drag.offset);

            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            // Measured from where the drag began and snapped as one move, so snapping
            // does not eat a little of every step. Ctrl snaps the grabbed point to the
            // grid outright rather than snapping the distance.
            let total = if ctrl && duplicate == crate::view2d::Duplicate::No {
                grid.snap_point(proposed) - drag.initial
            } else {
                grid.snap_point(proposed - drag.initial)
            };
            if total == drag.applied {
                return false;
            }
            // A Ctrl-drag copies the selection on its first step and moves the copy; the
            // transaction is opened over the copy, and folded with it at the end.
            if duplicate == crate::view2d::Duplicate::Pending {
                let mark = document.edit_mark();
                document.duplicate_selection_by(Vec3d::zero());
                document.begin_transaction();
                drag.what = DragKind::MoveObjects {
                    duplicate: crate::view2d::Duplicate::Made(mark),
                };
            }

            document.rollback_transaction();
            document.translate_selection(total);
            drag.applied = total;
            true
        }

        DragKind::RotationCenter => {
            let current = drag.initial + drag.applied;
            let mode = handle_drag::move_mode(alt, shift, drag.initial, current);
            drag.picker = handle_drag::picker_for_move(mode, drag.initial, current, drag.offset);
            let Some(proposed) = drag.picker.pick(ray) else {
                return false;
            };
            let total = grid.snap_point(proposed) - drag.initial;
            if total == drag.applied {
                return false;
            }
            rotate.set_center(drag.initial + total);
            drag.applied = total;
            true
        }

        // A draw drag and the assemble tool's drags are followed in the update closure,
        // before this is reached.
        DragKind::Draw { .. }
        | DragKind::Click
        | DragKind::PaintSelect { .. }
        | DragKind::AssembleFace { .. }
        | DragKind::AssembleExtrude { .. } => false,
    }
}

/// One step of an assemble tool drag: the points the tool has once the pointer is where
/// `ray` points, or `None` if the drag cannot follow it there.
///
/// A face rectangle runs from the press to wherever the ray meets a brush
/// (`makeSurfaceHandlePicker`), and a pulled polygon goes along its normal by whole grid
/// steps (`makeRelativeLineHandleSnapper`).
pub fn assemble_drag_points(
    drag: &Drag,
    ray: &Ray3d,
    document: &Document,
    grid: &Grid,
) -> Option<Vec<Vec3d>> {
    let (mut points, before) = match &drag.what {
        DragKind::AssembleFace { plane, before } => {
            let hit = document.pick(ray)?;
            let to = ray.point_at_distance(hit.distance);
            let rectangle = crate::assemble_tool::face_rectangle(plane, drag.initial, to, grid);
            (rectangle.to_vec(), before)
        }
        DragKind::AssembleExtrude {
            polygon,
            normal,
            before,
        } => {
            let proposed = drag.picker.pick(ray)?;
            let distance = grid.snap_scalar(vec::dot(proposed - drag.initial, *normal));
            let moved = polygon.iter().map(|point| *point + *normal * distance);
            (moved.collect(), before)
        }
        _ => return None,
    };
    points.extend(before.iter().copied());
    Some(points)
}

/// Breaks a line into the dashes that draw it as a dashed one.
///
/// The overlay is drawn as GL lines, which have no dash pattern of their own, so the gaps
/// are made by leaving segments out. The dash is a fixed length in world units: a preview
/// is looked at from a few hundred units away, and a pattern in screen space would need
/// the camera, which nothing else in this list does.
pub fn dashes(from: Vec3d, to: Vec3d) -> Vec<(Vec3d, Vec3d)> {
    /// How long a dash is, and the gap after it, in world units.
    const DASH: f64 = 4.0;

    let along = to - from;
    let length = geometry_math::vec::length(along);
    if length <= DASH * 2.0 {
        return vec![(from, to)];
    }
    let step = along / length * DASH;
    let count = (length / DASH) as usize;
    (0..count)
        .step_by(2)
        .map(|index| {
            let start = from + step * index as f64;
            let end = if index + 1 < count { start + step } else { to };
            (start, end)
        })
        .collect()
}

impl Hud {
    /// Draws the HUD over a view `viewport` pixels large: the render camera's frame, the
    /// labels, the other participants' cameras, the rotate feedback and the compass.
    pub fn draw(&self, cr: &mut dyn Canvas, viewport: (f64, f64)) {
        if let Some(aspect) = self.render_aspect {
            crate::render_camera::draw_frame(cr, aspect, viewport.0, viewport.1);
        }
        for (text, anchor, alignment, gap, style) in &self.labels {
            crate::view2d::draw_label(cr, text, *anchor, *alignment, *gap, *style);
        }
        for (peer, name, anchor) in &self.cameras {
            let [r, g, b] = crate::collab::peer_color_bytes(*peer);
            cr.set_source_rgb(
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
            );
            cr.arc(anchor.x(), anchor.y(), 5.0, 0.0, std::f64::consts::TAU);
            cr.fill_preserve();
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.8);
            cr.set_line_width(1.5);
            cr.stroke();
            // Bottom aligns the label's lower edge above the camera marker.
            crate::view2d::draw_label(
                cr,
                name,
                *anchor,
                crate::size_labels::Alignment::Bottom,
                24.0,
                crate::view2d::LabelStyle::Picked,
            );
        }
        if let Some(rotation) = &self.rotation {
            rotation.draw(cr);
        }
        crate::compass::draw(cr, viewport, &self.arrows);
    }
}

/// The tools' overlay for one frame, from [`Scene3d::build_overlay`].
pub struct FrameOverlay {
    /// Handles: where, and in what colour.
    pub points: Vec<(Vec3d, [f32; 4])>,
    /// Guide lines, as pairs of ends.
    pub guides: Vec<(Vec3d, [f32; 4])>,
    /// Whether the HUD changed and has to be drawn again.
    pub hud_changed: bool,
}

/// What a 3D view shares with the rest of the editor, handed to [`Scene3d::new`].
pub struct Scene3dParts {
    pub document: Rc<RefCell<Document>>,
    pub tool: Rc<RefCell<Tool>>,
    pub handles: Rc<RefCell<HandleManager>>,
    pub rotate: Rc<RefCell<RotateTool>>,
    pub camera: Rc<RefCell<Camera3d>>,
    pub fillet: Rc<RefCell<crate::fillet_tool::FilletParameters>>,
    pub pending_shape: Rc<RefCell<Option<Bbox3d>>>,
    pub shape_parameters: Rc<RefCell<crate::shape_tool::ShapeParameters>>,
    pub clip: Rc<RefCell<ClipTool>>,
    pub assemble: Rc<RefCell<AssembleBrushTool>>,
    pub grid: Rc<RefCell<Grid>>,
    pub filters: Rc<RefCell<ViewFilters>>,
    pub sweep: Rc<RefCell<crate::sweep_tool::SweepInputs>>,
    pub paste_target: Rc<RefCell<Option<Vec3d>>>,
    /// Asks the side panels to catch up with the document.
    pub refresh_panels: Rc<dyn Fn()>,
    /// Redraws every view.
    pub redraw: Rc<dyn Fn()>,
    /// Asks this view for another frame.
    pub render: Rc<dyn Fn()>,
    /// Gives this view the keyboard focus.
    pub focus: Rc<dyn Fn()>,
}

/// One 3D view: its per-view state and the editor state it shares.
pub struct Scene3d {
    pub document: Rc<RefCell<Document>>,
    pub tool: Rc<RefCell<Tool>>,
    pub handles: Rc<RefCell<HandleManager>>,
    pub rotate: Rc<RefCell<RotateTool>>,
    pub camera: Rc<RefCell<Camera3d>>,
    pub fillet: Rc<RefCell<crate::fillet_tool::FilletParameters>>,
    pub pending_shape: Rc<RefCell<Option<Bbox3d>>>,
    pub shape_parameters: Rc<RefCell<crate::shape_tool::ShapeParameters>>,
    pub clip: Rc<RefCell<ClipTool>>,
    pub assemble: Rc<RefCell<AssembleBrushTool>>,
    pub grid: Rc<RefCell<Grid>>,
    pub filters: Rc<RefCell<ViewFilters>>,
    pub sweep: Rc<RefCell<crate::sweep_tool::SweepInputs>>,
    pub paste_target: Rc<RefCell<Option<Vec3d>>>,
    pub refresh_panels: Rc<dyn Fn()>,
    pub redraw: Rc<dyn Fn()>,
    pub render: Rc<dyn Fn()>,
    pub focus: Rc<dyn Fn()>,
    /// The animation frame the buffers were last uploaded for.
    pub rendered_frame: Rc<Cell<Option<(u32, u32)>>>,
    pub preview_motions: Rc<
        RefCell<std::collections::HashMap<map_model::node::NodeId, crate::motion_preview::Motion>>,
    >,
    pub preview_mesh: Rc<RefCell<crate::motion_preview::Mesh>>,
    pub fillet_surfaces: Rc<RefCell<crate::fillet_tool::PreviewCache>>,
    pub uploaded_surface_epoch: Rc<Cell<Option<u64>>>,
    /// What the HUD over the view draws, rebuilt each frame.
    pub hud: Rc<RefCell<Hud>>,
    /// Whether the pointer is over something selected, for the selection's size labels.
    pub hover_selected: Rc<RefCell<bool>>,
    /// The primary drag under way.
    pub drag: Rc<RefCell<Option<Drag>>>,
    /// Which document generation and revision the buffers hold.
    pub uploaded: Rc<RefCell<Option<(u64, u64)>>>,
    /// Which material collection the textures hold.
    pub uploaded_materials: Rc<RefCell<Option<u64>>>,
    /// Which filters the buffers were built under.
    pub uploaded_filters: Rc<RefCell<Option<ViewFilters>>>,
    /// Which clip the buffers show.
    pub uploaded_clip: Rc<RefCell<Option<ClipTool>>>,
    /// What the last frame was of, so a changed view restarts the accumulation.
    pub last_frame: Rc<RefCell<Option<u64>>>,
    /// The outlines of the faces a Shift-drag would extrude.
    pub extrude_preview: Rc<RefCell<Vec<Vec<Vec3d>>>>,
    /// The box an entity dragged from the browser would land in.
    pub entity_drop_preview: Rc<RefCell<Option<Bbox3d>>>,
    /// Where the pointer is.
    pub pointer: Rc<RefCell<(f64, f64)>>,
    /// Whether the pointer is over the view.
    pub pointer_inside: Rc<Cell<bool>>,
    /// The selection rectangle in viewport pixels.
    pub lasso: Rc<RefCell<Option<(Vec2d, Vec2d)>>>,
    /// The selection when a lasso began.
    pub marquee: Rc<RefCell<Option<crate::document::SelectionSnapshot>>>,
    /// The Alt-drag that copies face attributes across faces.
    pub stroke: Rc<RefCell<Option<AttrStroke>>>,
    /// The last position of the right-button look drag.
    pub look_last: Rc<RefCell<(f64, f64)>>,
    /// The last position of the middle-button pan drag.
    pub pan_last: Rc<RefCell<(f64, f64)>>,
    /// Where the primary press happened.
    pub press: Rc<Cell<(f64, f64)>>,
    /// Whether any handle was picked when a handle tool's press missed them all: a click
    /// then selects the brush under it rather than only dropping what was picked.
    pub handles_were_picked: Rc<Cell<bool>>,
    /// Where the history stood before and after the last attribute sweep was applied, so
    /// an Alt double click that follows folds the single-face copy into its own.
    pub transfer_marks: Rc<Cell<Option<(usize, usize)>>>,
}

impl Scene3d {
    /// A view of the shared editor state.
    pub fn new(parts: Scene3dParts) -> Self {
        let Scene3dParts {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
        } = parts;
        Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame: Rc::new(Cell::new(None)),
            preview_motions: Rc::new(RefCell::new(std::collections::HashMap::new())),
            preview_mesh: Rc::new(RefCell::new(crate::motion_preview::Mesh::default())),
            fillet_surfaces: Default::default(),
            uploaded_surface_epoch: Default::default(),
            hud: Rc::new(RefCell::new(Hud::default())),
            hover_selected: Rc::new(RefCell::new(false)),
            drag: Rc::new(RefCell::new(None)),
            uploaded: Rc::new(RefCell::new(None)),
            uploaded_materials: Rc::new(RefCell::new(None)),
            uploaded_filters: Rc::new(RefCell::new(None)),
            uploaded_clip: Rc::new(RefCell::new(None)),
            last_frame: Rc::new(RefCell::new(None)),
            extrude_preview: Rc::new(RefCell::new(Vec::new())),
            entity_drop_preview: Rc::new(RefCell::new(None)),
            pointer: Rc::new(RefCell::new((0.0, 0.0))),
            pointer_inside: Rc::new(Cell::new(false)),
            lasso: Rc::new(RefCell::new(None)),
            marquee: Rc::new(RefCell::new(None)),
            stroke: Rc::new(RefCell::new(None)),
            look_last: Rc::new(RefCell::new((0.0, 0.0))),
            pan_last: Rc::new(RefCell::new((0.0, 0.0))),
            press: Rc::new(Cell::new((0.0, 0.0))),
            handles_were_picked: Rc::new(Cell::new(false)),
            transfer_marks: Rc::new(Cell::new(None)),
        }
    }

    /// Cached, view-only geometry for the edge under the pointer or the pinned selection.
    pub fn fillet_surface_preview(
        &self,
        document: &Document,
        width: f64,
        height: f64,
    ) -> Option<Rc<crate::fillet_tool::SurfacePreview>> {
        if *self.tool.borrow() != Tool::Fillet {
            return None;
        }
        let mut handles = self.handles.borrow_mut();
        handles.collect(HandleKind::Edge, document.selected_brushes().into_iter());
        let (x, y) = *self.pointer.borrow();
        let camera = self.camera.borrow();
        let hovered = self
            .pointer_inside
            .get()
            .then(|| {
                handle_drag::nearest_handle(
                    &camera,
                    height,
                    &camera.ray_through(x, y, width, height),
                    handles.handles().iter().map(|h| h.position),
                    pick_radius(),
                )
            })
            .flatten();
        let edges = crate::fillet_tool::edges_to_round(document, &handles, hovered);
        self.fillet_surfaces
            .borrow_mut()
            .get(document, edges, *self.fillet.borrow())
    }

    /// A new GL context is up: every cached upload is stale, and the axes, which span
    /// the world, go up once.
    pub fn renderer_created(&self, renderer: &mut Renderer) {
        *self.uploaded.borrow_mut() = None;
        *self.uploaded_materials.borrow_mut() = None;
        *self.uploaded_filters.borrow_mut() = None;
        renderer.upload_axes(&self.document.borrow().world_bounds());
    }

    /// Whether another frame is wanted without anything changing: a still view with
    /// samples left to gather, or an animation playing.
    pub fn wants_frame(&self, renderer: &Renderer) -> bool {
        let filters = self.filters.borrow();
        let playing = filters.animation_frame.is_none()
            && filters.preview_motion
            && !self.preview_mesh.borrow().motions.is_empty();
        playing || (!filters.wireframe && renderer.wants_another_sample())
    }

    /// The first edit click stops playback, so picking uses the visible rest pose.
    /// Returns whether it stopped something, in which case the click goes no further.
    pub fn edit_press(&self) -> bool {
        let mut filters = self.filters.borrow_mut();
        if !(filters.animation_auto_key && filters.animation_frame.is_some())
            && (filters.preview_motion || filters.animation_frame.is_some())
        {
            filters.preview_motion = false;
            filters.animation_frame = None;
            drop(filters);
            (self.redraw)();
            true
        } else {
            false
        }
    }

    /// The pointer entered or moved: the tools that show hover want a frame.
    pub fn pointer_over(&self) {
        self.pointer_inside.set(true);
        let active = *self.tool.borrow();
        if matches!(
            active,
            Tool::Rotate | Tool::Scale | Tool::Shear | Tool::Clip | Tool::Sculpt
        ) || HandleKind::of(active).is_some()
        {
            (self.render)();
        }
    }

    /// The pointer left the view.
    pub fn pointer_away(&self) {
        self.pointer_inside.set(false);
        (self.render)();
    }

    /// Escape during a lasso restores the selection. Returns whether it did, in which
    /// case the frontend resets its drag.
    pub fn escape(&self) -> bool {
        let Self {
            document,
            tool,
            lasso,
            marquee,
            redraw,
            ..
        } = self;
        // Esc in the middle of a drag undoes it and ends it, as `ToolChain::cancel` does;
        // closing only the transaction left the drag going, each later step applied
        // outside it and recorded on its own.
        if let Some(ended) = self.drag.borrow_mut().take() {
            document.borrow_mut().cancel_transaction();
            // `DrawFaceDragDelegate::cancel`: the points go back to what they were.
            if let DragKind::AssembleFace { before, .. }
            | DragKind::AssembleExtrude { before, .. } = &ended.what
            {
                self.assemble.borrow_mut().set_points(before);
            }
            // Painting the selection is undone with it (`cancelTransaction`).
            if let DragKind::PaintSelect { before, .. } = &ended.what {
                document.borrow_mut().restore_selection(before);
                (self.refresh_panels)();
            }
            *self.pending_shape.borrow_mut() = None;
            (self.render)();
            redraw();
            return true;
        }
        let selecting = lasso.borrow().is_some() && *tool.borrow() == Tool::Select;
        if selecting {
            if let Some(initial) = marquee.borrow_mut().take() {
                document.borrow_mut().restore_selection(&initial);
            }
            *lasso.borrow_mut() = None;
            redraw();
            return true;
        }
        false
    }

    /// Right-drag to look around: its start.
    pub fn look_begin(&self) {
        *self.look_last.borrow_mut() = (0.0, 0.0);
    }

    /// Right-drag to look around: `(x, y)` from the press. Dragging right turns the view
    /// right, and dragging down looks down.
    pub fn look_update(&self, x: f64, y: f64) {
        let (lx, ly) = *self.look_last.borrow();
        self.camera
            .borrow_mut()
            .turn((x - lx) * turn_per_pixel().0, (y - ly) * turn_per_pixel().1);
        *self.look_last.borrow_mut() = (x, y);
        (self.render)();
    }

    /// Alt and the right button orbit, as `CameraTool3D::shouldOrbit` does: about what is
    /// under the press, or a point down the ray when it is over nothing.
    pub fn orbit_begin(&self, x: f64, y: f64, size: (f64, f64)) -> Vec3d {
        *self.look_last.borrow_mut() = (0.0, 0.0);
        let ray = self.ray(x, y, size);
        let hit = self
            .document
            .borrow()
            .pick(&ray)
            .filter(|hit| hit.distance >= 3.0)
            .map(|hit| hit.distance);
        ray.point_at_distance(hit.unwrap_or(DEFAULT_DRAW_DISTANCE))
    }

    /// The orbit: `(x, y)` from the press, at the look speed.
    pub fn orbit_update(&self, center: Vec3d, x: f64, y: f64) {
        let (lx, ly) = *self.look_last.borrow();
        self.camera.borrow_mut().orbit(
            center,
            (x - lx) * turn_per_pixel().0,
            (y - ly) * turn_per_pixel().1,
        );
        *self.look_last.borrow_mut() = (x, y);
        (self.render)();
    }

    /// Middle-drag to slide the view, as `CameraTool3D` does: its start.
    pub fn pan_begin(&self) {
        *self.pan_last.borrow_mut() = (0.0, 0.0);
    }

    /// Middle-drag: `(x, y)` from the press.
    ///
    /// `CameraEnableAltMove`: with the preference set, holding Alt turns the middle-drag
    /// into a move along the view axis rather than a pan, and `CameraAltMoveInvert` flips
    /// which way the vertical drag goes.
    pub fn pan_update(&self, x: f64, y: f64, modifiers: Modifiers) {
        let (lx, ly) = *self.pan_last.borrow();
        let alt_move = pref(|p| p.camera_enable_alt_move) && modifiers.alt;
        if alt_move {
            let speed = f64::from(pref(|p| p.camera_move_speed));
            let sign = if pref(|p| p.camera_alt_move_invert) {
                -1.0
            } else {
                1.0
            };
            self.camera
                .borrow_mut()
                .fly(-(y - ly) * speed * sign, 0.0, 0.0);
        } else {
            let (horizontal, vertical) = pan_per_pixel();
            // Dragging right slides the world right, which means the camera goes left.
            self.camera
                .borrow_mut()
                .pan(-(x - lx) * horizontal, (y - ly) * vertical);
        }
        *self.pan_last.borrow_mut() = (x, y);
        (self.render)();
    }

    /// The camera's fly keys: W, S, A and D for horizontal flight, Q for rising and X for
    /// descending. Returns whether the key was one of them, which the view keeps for
    /// itself; the flight is [`Self::fly_held`], every frame the keys are held. Control
    /// passes the window's own shortcuts through, and the arrows are the window's, which
    /// move the selection.
    pub fn key(&self, key: Key, modifiers: Modifiers) -> bool {
        !modifiers.control
            && ['w', 's', 'a', 'd', 'q', 'x']
                .into_iter()
                .any(|c| key.is(c))
    }

    /// Flies the camera for `seconds` with the fly keys held, `forward`, `right` and `up`
    /// each -1, 0 or 1: `FlyModeHelper`, which moves by time rather than by the keyboard's
    /// repeat, so the flight starts at once and runs smoothly. Shift flies faster and Alt
    /// slower.
    pub fn fly_held(&self, forward: f64, right: f64, up: f64, modifiers: Modifiers, seconds: f64) {
        if modifiers.control || (forward == 0.0 && right == 0.0 && up == 0.0) {
            return;
        }
        let modifier = if modifiers.shift {
            SPEED_MODIFIER
        } else if modifiers.alt {
            1.0 / SPEED_MODIFIER
        } else {
            1.0
        };
        // The speed preference is in units a millisecond.
        let distance = f64::from(pref(|p| p.camera_fly_move_speed)) * 1000.0 * seconds * modifier;
        self.camera
            .borrow_mut()
            .fly(forward * distance, right * distance, up * distance);
        (self.render)();
    }

    /// A ray from the pointer into the scene.
    pub fn ray(&self, x: f64, y: f64, (width, height): (f64, f64)) -> Ray3d {
        self.camera.borrow().ray_through(x, y, width, height)
    }

    /// The node under the pointer.
    pub fn pick_node(&self, x: f64, y: f64, size: (f64, f64)) -> Option<map_model::node::NodeId> {
        self.document
            .borrow()
            .pick(&self.ray(x, y, size))
            .map(|hit| hit.node)
    }

    /// Where a new thing goes: the surface under the pointer, or a set distance down the
    /// ray when it is over nothing, snapped to the grid.
    fn landing(&self, ray: &Ray3d) -> Vec3d {
        let document = self.document.borrow();
        self.grid.borrow().snap_point(match document.pick(ray) {
            Some(hit) => ray.point_at_distance(hit.distance),
            None => ray.point_at_distance(DEFAULT_DRAW_DISTANCE),
        })
    }

    /// What the right-click menu needs to know about the point under the pointer.
    pub fn probe(&self, x: f64, y: f64, size: (f64, f64)) -> crate::view2d::Probe {
        let ray = self.ray(x, y, size);
        let at = self.landing(&ray);
        let document = self.document.borrow();
        let hit = document.pick(&ray);
        crate::view2d::Probe {
            entity: hit.and_then(|hit| document.entity_around(hit.node)),
            material: hit.and_then(|hit| {
                hit.face.and_then(|face| {
                    document
                        .face_at(FaceRef {
                            node: hit.node,
                            face,
                        })
                        .map(|face| face.material_name.to_string())
                })
            }),
            at,
        }
    }

    /// A prefab dropped on the view: inserted on the surface under the pointer, or a set
    /// distance down the ray.
    pub fn drop_prefab(&self, path: &std::path::Path, x: f64, y: f64, size: (f64, f64)) -> bool {
        let at = self.landing(&self.ray(x, y, size));
        let landed = crate::assets::insert_prefab_on_grid(
            &mut self.document.borrow_mut(),
            path,
            at,
            &self.grid.borrow(),
        );
        (self.redraw)();
        (self.refresh_panels)();
        landed
    }

    /// An entity class dragged over the view: the box it would land in follows the
    /// pointer, set down on the face under it or a set distance down the ray --
    /// `CreateEntityTool::updateEntityPosition3D`.
    pub fn entity_drag_over(&self, classname: Option<&str>, x: f64, y: f64, size: (f64, f64)) {
        let landing = classname.map(|classname| {
            crate::entity_placement::drop_preview_bounds(
                &self.document.borrow(),
                &self.grid.borrow(),
                classname,
                &self.ray(x, y, size),
                false,
            )
        });
        let changed = *self.entity_drop_preview.borrow() != landing;
        if changed {
            *self.entity_drop_preview.borrow_mut() = landing;
            (self.render)();
        }
    }

    /// The entity drag left without a drop.
    pub fn entity_drag_leave(&self) {
        let showing = self.entity_drop_preview.borrow().is_some();
        if showing {
            *self.entity_drop_preview.borrow_mut() = None;
            (self.render)();
        }
    }

    /// An entity class dropped on the view: one placed there.
    pub fn drop_entity(&self, classname: &str, x: f64, y: f64, size: (f64, f64)) -> bool {
        *self.entity_drop_preview.borrow_mut() = None;
        let placed = crate::entity_placement::place_entity_on_ray(
            &self.document,
            &self.grid.borrow(),
            classname,
            &self.ray(x, y, size),
        );
        (self.redraw)();
        (self.refresh_panels)();
        placed
    }

    /// Draws a frame into the bound framebuffer, `width` by `height` logical pixels at
    /// `scale` device pixels each, uploading whatever changed first. Returns whether the
    /// HUD changed and has to be drawn again.
    pub fn render(&self, renderer: &mut Renderer, width: f64, height: f64, scale: i32) -> bool {
        self.render_kept(renderer, width, height, scale, None)
    }

    /// [`Scene3d::render`] for a framebuffer that keeps its picture between frames:
    /// `drawn` holds the [`picture_key`] of what it shows, and
    /// the drawing is skipped while the frame would be that picture again. Uploads, the
    /// overlay and the HUD still follow the scene, which is cheap next to the drawing.
    pub fn render_kept(
        &self,
        renderer: &mut Renderer,
        width: f64,
        height: f64,
        scale: i32,
        drawn: Option<&Cell<Option<u64>>>,
    ) -> bool {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;
        // The renderer draws by what it is handed, not by reading the preference
        // store itself; this is the desktop frontend filling that in each frame.
        let settings = self.begin_frame(width, height);
        renderer.set_settings(settings.clone());

        let document = document.borrow();

        // The materials first: the geometry's batches index into them.
        let materials_key = document.materials_revision();
        if *uploaded_materials.borrow() != Some(materials_key) {
            renderer.upload_materials(document.materials().materials());
            *uploaded_materials.borrow_mut() = Some(materials_key);
            // The batches were built against the collection that has just been
            // replaced, so they have to be built again.
            *uploaded.borrow_mut() = None;
        }

        // The filters change what is drawn without the document changing, so they are
        // part of what says whether the buffers are still current.
        let current_filters = *filters.borrow();
        let clip_state = (*tool.borrow() == Tool::Clip).then(|| clip.borrow().clone());
        let fillet_preview = self.fillet_surface_preview(&document, width, height);
        let surface_epoch = document.with_tool_preview_surfaces(
            &current_filters,
            clip_state.as_ref(),
            fillet_preview.as_ref(),
            |s| s.epoch(),
        );
        let key = (document.generation(), document.revision());
        if uploaded_surface_epoch.get() != Some(surface_epoch)
            || *uploaded.borrow() != Some(key)
            || *uploaded_filters.borrow() != Some(current_filters.geometry_only())
            || *uploaded_clip.borrow() != clip_state
        {
            let started = crate::profile::start();
            *preview_motions.borrow_mut() = document.preview_motions();
            *preview_mesh.borrow_mut() = document.with_tool_preview_surfaces(
                &current_filters,
                clip_state.as_ref(),
                fillet_preview.as_ref(),
                |s| s.mesh().0,
            );
            crate::profile::add_scene_build(started);
            renderer.upload(&preview_mesh.borrow().triangles);
            renderer.upload_sprites(if current_filters.show_point_entity_models {
                document.sprites()
            } else {
                Vec::new()
            });
            *uploaded.borrow_mut() = Some(key);
            *uploaded_filters.borrow_mut() = Some(current_filters.geometry_only());
            rendered_frame.set(None);
            *uploaded_clip.borrow_mut() = clip_state;
            uploaded_surface_epoch.set(Some(surface_epoch));
            renderer.restart_accumulating();
            *last_frame.borrow_mut() = None;
        }

        let playing = current_filters.animation_frame.is_none()
            && current_filters.preview_motion
            && !preview_mesh.borrow().motions.is_empty();
        document.animation_preview_time(playing);
        if let Some(frame) = current_filters.animation_frame {
            let state = (frame, current_filters.animation_fps.max(1));
            if rendered_frame.get() != Some(state) {
                renderer.upload(
                    &preview_mesh
                        .borrow()
                        .at_frame(frame as f64 / state.1 as f64),
                );
                renderer.restart_accumulating();
                rendered_frame.set(Some(state));
            }
        } else if playing {
            rendered_frame.set(None);
            let elapsed = document.animation_preview_time(true);
            renderer.upload(&preview_mesh.borrow().at(elapsed));
            renderer.restart_accumulating();
        } else {
            if rendered_frame.take().is_some() {
                renderer.upload(&preview_mesh.borrow().triangles);
                renderer.restart_accumulating();
            }
        }

        let FrameOverlay {
            points,
            guides,
            hud_changed,
        } = self.build_overlay(&document, width, height, playing);
        // Editing guides refer to the saved geometry, so hide them during playback.
        if playing {
            renderer.upload_overlay(&[], &[]);
        } else {
            renderer.upload_overlay(&points, &guides);
        }

        let (width, height) = (
            (width * f64::from(scale)).round() as i32,
            (height * f64::from(scale)).round() as i32,
        );

        // Accumulating averages this frame with the ones before it, so a frame of a
        // different view would smear across it.
        let frame = frame_key(
            &camera.borrow(),
            key,
            &current_filters,
            width,
            height,
            &points,
            &guides,
        ) ^ surface_epoch.rotate_left(29);
        if *last_frame.borrow() != Some(frame) {
            renderer.restart_accumulating();
            *last_frame.borrow_mut() = Some(frame);
        }

        if let Some(drawn) = drawn {
            let picture = picture_key(frame, &settings, document.materials_revision());
            let refining = !current_filters.wireframe && renderer.wants_another_sample();
            if !needs_draw(drawn.get(), picture, refining, playing) {
                return hud_changed;
            }
            drawn.set(Some(picture));
        }
        if current_filters.wireframe {
            renderer.draw_wireframe(&camera.borrow(), width, height);
        } else {
            renderer.draw(&camera.borrow(), width, height);
        }
        hud_changed
    }

    /// What every frame starts with, whatever draws it: the render settings from the
    /// preferences, and the camera's field of view, so a change in the dialog reaches a
    /// camera that is already looking at something.
    pub fn begin_frame(&self, width: f64, height: f64) -> render::RenderSettings {
        let camera = &self.camera;
        let render_fov = crate::render_camera::view_fov(camera, width, height);
        camera.borrow_mut().fov_degrees =
            render_fov.unwrap_or_else(|| f64::from(crate::preferences::pref(|p| p.camera_fov)));
        crate::render_settings::from_preferences()
    }

    /// The overlay of a frame, `width` by `height` logical pixels: the handles and guide
    /// lines the tools draw over the scene, in world space, and whether the HUD changed.
    /// Independent of the GPU backend: the GL renderer and the wgpu one both draw it.
    pub fn build_overlay(
        &self,
        doc: &Document,
        width: f64,
        height: f64,
        playing: bool,
    ) -> FrameOverlay {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;
        let document = doc;
        let current_filters = *filters.borrow();
        let render_aspect = crate::render_camera::view_aspect(camera);
        let mut hud_changed = false;
        // The overlay follows the tool and what it has hold of, both of which change
        // between frames without the document changing, so it is rebuilt every frame.
        let active = *tool.borrow();
        let fillet_preview = self.fillet_surface_preview(document, width, height);
        // Kept alongside the borrow below, since that shadows the name and the fillet
        // preview further down needs the handles again once it has been dropped.
        let handle_state = Rc::clone(handles);
        let mut handles = handles.borrow_mut();
        if let Some(kind) = HandleKind::of(active) {
            if kind == HandleKind::ControlPoint {
                handles.collect_control_points(document.selected_patches().into_iter());
            } else {
                handles.collect(kind, document.selected_brushes().into_iter());
            }
        }
        // The rotate rings are drawn at a constant size on screen, as
        // `RotateHandle::majorRadius` times the camera's perspective scaling --
        // sizing them to the selection instead puts a large selection's rings up
        // against the near plane, where their segments smear across the view.
        let rotate_ring_radius = rotate
            .borrow()
            .center()
            .map(|center| {
                f64::from(pref(|p| p.rotate_handle_radius))
                    * handle_drag::perspective_scaling_factor(&camera.borrow(), center, height)
            })
            .unwrap_or(0.0);
        // Labels the HUD will draw, in world space until it projects them: the
        // handle a press would take or a drag is moving, written out as its three
        // coordinates (`NodeHandleToolBase::renderHighlight`).
        let mut world_labels: Vec<(String, Vec3d)> = Vec::new();
        let overlay_started = crate::profile::start();
        let (mut points, mut guides) = overlay::build(
            document,
            active,
            &handles,
            &rotate.borrow(),
            &clip.borrow(),
            &assemble.borrow(),
            &sweep.borrow(),
            rotate_ring_radius,
        );
        drop(handles);
        if active == Tool::Clip && pointer_inside.get() {
            let (x, y) = *pointer.borrow();
            let ray = camera.borrow().ray_through(x, y, width, height);
            if let Some((at, _)) = clip_surface_point(document, &grid.borrow(), &ray) {
                if !clip.borrow().is_dragging() && clip.borrow().can_add_point(at) {
                    points.push((at, pref(|p| p.clip_handle_color).rgba_f32()));
                }
            }
        }
        // The sculpt brush's outline, lying on the surface under the pointer.
        if active == Tool::Sculpt && pointer_inside.get() {
            let (x, y) = *pointer.borrow();
            let ray = camera.borrow().ray_through(x, y, width, height);
            let parameters = crate::sculpt::parameters();
            if let (Some(at), Some(along)) = (
                document.sculpt_target(&ray),
                document.sculpt_axis(&ray, parameters.direction),
            ) {
                let colour = [0.9, 0.38, 0.0, 1.0];
                let radius = match parameters.form {
                    crate::sculpt::SculptForm::Free => {
                        parameters.free_radius(grid.borrow().actual_size())
                    }
                    _ => parameters.radius,
                };
                let ring = document.sculpt_ring(at, radius, along);
                for pair in ring.windows(2) {
                    guides.push((pair[0], colour));
                    guides.push((pair[1], colour));
                }
                points.push((at, colour));
            }
        }

        // Scale and shear use the same point picker for hover and pointer-down.
        // Recolour only the nearest handle, including handles outside geometry.
        if pointer_inside.get() && matches!(active, Tool::Scale | Tool::Shear) {
            let (x, y) = *pointer.borrow();
            let camera = camera.borrow();
            let ray = camera.ray_through(x, y, width, height);
            if let Some(position) = handle_drag::nearest_handle(
                &camera,
                height,
                &ray,
                points.iter().map(|(at, _)| *at),
                pick_radius(),
            ) {
                for (at, color) in &mut points {
                    if *at == position {
                        *color = pref(|p| p.selected_handle_color).rgba_f32();
                    }
                }
                world_labels.push((crate::size_labels::point_text(position), position));
            }
        }

        // What the pointer is over, for the tools that pick geometry: the vertex,
        // the edge or the outline of the face a press would take hold of, and for the fillet the solid outline of its result.
        // Both are worked out from the brushes as they
        // are, so nothing is changed by looking.
        if HandleKind::of(active).is_some() {
            let handles = handle_state.borrow();
            let (px, py) = *pointer.borrow();
            let ray = camera.borrow().ray_through(px, py, width, height);
            let positions: Vec<Vec3d> = handles
                .handles()
                .iter()
                .map(|handle| handle.position)
                .collect();
            let hovered = handle_drag::nearest_handle(
                &camera.borrow(),
                height,
                &ray,
                positions,
                pick_radius(),
            )
            .filter(|_| pointer_inside.get());

            // What a press would take hold of, said in numbers.
            if let Some(position) = hovered {
                for (at, color) in &mut points {
                    if *at == position {
                        *color = pref(|p| p.selected_handle_color).rgba_f32();
                    }
                }
                world_labels.push((crate::size_labels::point_text(position), position));
            }

            let hover_colour = pref(|p| p.selected_handle_color).rgba_f32();
            if let Some(handle) = hovered.and_then(|position| {
                handles
                    .handles()
                    .iter()
                    .find(|handle| geometry_math::vec::is_zero(handle.position - position, 0.001))
            }) {
                for (from, to) in crate::handle_tool::outline_of(handle) {
                    guides.push((from, hover_colour));
                    guides.push((to, hover_colour));
                }
            }

            if active == Tool::Fillet {
                let edges = crate::fillet_tool::edges_to_round(document, &handles, hovered);
                for (from, to) in &edges {
                    guides.push((*from, hover_colour));
                    guides.push((*to, hover_colour));
                }

                let preview_colour = pref(|p| p.selected_edge_color).rgba_f32();
                if let Some(preview) = fillet_preview.as_ref() {
                    for (_, brushes) in &preview.brushes {
                        for brush in brushes {
                            for (from, to) in brush.edge_positions() {
                                guides.push((from, preview_colour));
                                guides.push((to, preview_colour));
                            }
                        }
                    }
                }
            }
        }

        // The rectangle a selection drag has swept so far. The overlay draws world
        // lines, so the four corners are put a little way down the rays through them:
        // a quad in front of the camera, which is the rectangle on the screen.
        if let Some((from, to)) = *lasso.borrow() {
            let camera = camera.borrow();
            let colour = pref(|p| p.selected_edge_color).rgba_f32();
            let at = |x: f64, y: f64| {
                let ray = camera.ray_through(x, y, width, height);
                ray.origin + ray.direction * (camera.near * 4.0)
            };
            let corners = [
                at(from.x(), from.y()),
                at(to.x(), from.y()),
                at(to.x(), to.y()),
                at(from.x(), to.y()),
            ];
            for index in 0..corners.len() {
                guides.push((corners[index], colour));
                guides.push((corners[(index + 1) % corners.len()], colour));
            }
        }

        // Adjacent poses are outlines of selected animated brushes, never saved geometry.
        if let Some(frame) = current_filters.animation_frame {
            let motions = preview_motions.borrow();
            let fps = current_filters.animation_fps.max(1) as f64;
            for id in document.selection_ids() {
                let (
                    Some(brush),
                    Some(
                        crate::motion_preview::Motion::Keyframes { pivot, track, .. }
                        | crate::motion_preview::Motion::Playback {
                            pivot,
                            authored: track,
                            ..
                        },
                    ),
                ) = (document.brush(*id), motions.get(id))
                else {
                    continue;
                };
                let now = frame as f64 / fps;
                for (a, b) in current_filters.brush_edges(brush) {
                    let colour = pref(|p| p.selected_edge_color).rgba_f32();
                    guides.push((track.transform_clamped(a, *pivot, now, false), colour));
                    guides.push((track.transform_clamped(b, *pivot, now, false), colour));
                }
                // RGB axes show the animated pivot and rotation.
                let centre = track.transform_clamped(*pivot, *pivot, now, false);
                for (axis, colour) in [
                    (Vec3d::new(32.0, 0.0, 0.0), [1.0, 0.25, 0.25, 1.0]),
                    (Vec3d::new(0.0, 32.0, 0.0), [0.25, 1.0, 0.25, 1.0]),
                    (Vec3d::new(0.0, 0.0, 32.0), [0.3, 0.55, 1.0, 1.0]),
                ] {
                    guides.push((centre, colour));
                    guides.push((
                        centre + track.transform_clamped(axis, *pivot, now, true),
                        colour,
                    ));
                }
                for offset in 1..=current_filters.onion_frames {
                    for direction in [-1.0, 1.0] {
                        let time = now
                            + direction * offset as f64 * current_filters.onion_step.max(1) as f64
                                / fps;
                        if time < 0.0 || time > track.keys.last().unwrap().time {
                            continue;
                        }
                        let colour = if direction < 0.0 {
                            [0.25, 0.65, 1.0, 0.45 / offset as f32]
                        } else {
                            [1.0, 0.55, 0.2, 0.45 / offset as f32]
                        };
                        for (a, b) in current_filters.brush_edges(brush) {
                            guides.push((track.transform_clamped(a, *pivot, time, false), colour));
                            guides.push((track.transform_clamped(b, *pivot, time, false), colour));
                        }
                    }
                }
            }
        }

        // Point entities drawn as boxes: their edges in the class's colour, or the
        // selection's, over the see-through fill, as the flat views outline them.
        for entity in document.entity_box_outlines(&current_filters) {
            let colour = if entity.selected {
                pref(|p| p.selected_edge_color).rgba_f32()
            } else {
                let [r, g, b] = entity.color.map(|channel| f32::from(channel) / 255.0);
                [r, g, b, 1.0]
            };
            for (from, to) in entity.bounds.edges() {
                guides.push((from, colour));
                guides.push((to, colour));
            }
        }

        // The outline round what is selected: its edges, drawn over the surface and
        // faintly through it, so the selection reads however the light falls on the
        // faces. Blender draws the silhouette; the edges are what
        // this renderer already has a way to draw, and on a brush they come to the
        // same thing.
        if current_filters.animation_frame.is_none()
            && (pref(|p| p.selection_outline) || document.has_shape_preview())
        {
            let colour = pref(|p| {
                if document.has_shape_preview() {
                    p.resize_handle_color
                } else {
                    p.selected_edge_color
                }
            })
            .rgba_f32();
            for brush in document.selected_brushes() {
                // Do not draw the old sharp silhouette over its rounded replacement.
                if fillet_preview.as_ref().is_some_and(|preview| {
                    preview.brushes.iter().any(|(id, _)| {
                        document
                            .brush(*id)
                            .is_some_and(|source| std::ptr::eq(source, brush))
                    })
                }) {
                    continue;
                }
                for (from, to) in current_filters.brush_edges(brush) {
                    guides.push((from, colour));
                    guides.push((to, colour));
                }
            }
        }

        // Every selected face needs a perimeter: texture detail can hide its tint.
        // Painted faces retain this cue even when selection outlines are disabled.
        {
            let colour = pref(|p| p.selected_edge_color).rgba_f32();
            for selected in document.selected_faces() {
                if fillet_preview.as_ref().is_some_and(|preview| {
                    preview.brushes.iter().any(|(id, _)| *id == selected.node)
                }) {
                    continue;
                }
                if let Some(brush) = document.brush(selected.node) {
                    brush.for_each_geometry_face(|index, face, polygon| {
                        if index == selected.face
                            && current_filters.animation_frame.is_none()
                            && (face.color.is_some() || pref(|p| p.selection_outline))
                            && (current_filters.show_export_hidden || !face.export_hidden)
                        {
                            for i in 0..polygon.len() {
                                guides.push((polygon[i], colour));
                                guides.push((polygon[(i + 1) % polygon.len()], colour));
                            }
                        }
                    });
                }
            }
        }

        // The shape a create drag in a flat view is sweeping out, drawn as the
        // brushes it would make rather than as a box, so a cylinder or a flight of
        // stairs shows what it is. The drag happens in another
        // view; this one only draws what it has been handed.
        if let Some(bounds) = *pending_shape.borrow() {
            let preview = crate::shape_tool::create_brushes(
                &document.world_bounds(),
                &bounds,
                &shape_parameters.borrow(),
                map_model::shapes::DEFAULT_MATERIAL,
            )
            .unwrap_or_default();
            let colour = pref(|p| p.resize_handle_color).rgba_f32();
            for brush in &preview {
                for (from, to) in brush.edge_positions() {
                    guides.push((from, colour));
                    guides.push((to, colour));
                }
            }
        }

        // The other participants in a live session, each in the colour that person
        // draws in everywhere, so nothing of theirs reads as this editor's own
        // work: the edges of what they have selected, and their cameras drawn as
        // little frusta with a line of sight reaching into the scene -- their
        // cursors in space.
        {
            for (peer, from, to) in document.peer_edges().iter() {
                let colour = crate::collab::peer_color_f32(*peer);
                guides.push((*from, colour));
                guides.push((*to, colour));
            }
            for (peer, (position, direction)) in document.peer_cameras() {
                let colour = crate::collab::peer_color_f32(*peer);
                let forward = geometry_math::vec::normalize(*direction);
                let steady = if forward.z().abs() > 0.9 {
                    Vec3d::new(1.0, 0.0, 0.0)
                } else {
                    Vec3d::new(0.0, 0.0, 1.0)
                };
                let right =
                    geometry_math::vec::normalize(geometry_math::vec::cross(forward, steady));
                let up = geometry_math::vec::normalize(geometry_math::vec::cross(right, forward));
                // A pyramid the size of a hand: apex where they stand, base a
                // little way along their look, wider than tall like their viewport.
                let base = *position + forward * 28.0;
                let (half_width, half_height) = (16.0, 10.0);
                let corners = [
                    base + right * half_width + up * half_height,
                    base - right * half_width + up * half_height,
                    base - right * half_width - up * half_height,
                    base + right * half_width - up * half_height,
                ];
                for index in 0..corners.len() {
                    guides.push((*position, colour));
                    guides.push((corners[index], colour));
                    guides.push((corners[index], colour));
                    guides.push((corners[(index + 1) % corners.len()], colour));
                }
                guides.push((base, colour));
                guides.push((*position + forward * 96.0, colour));
            }
        }

        // Draws world-axis guides and coordinates for the active drag handle.
        if let Some(dragged) = drag.borrow().as_ref().and_then(dragged_handle) {
            let colour = pref(|p| p.handle_color);
            let mut colour = colour.rgba_f32();
            // `VertexTool::renderGuide` halves the handle colour's alpha.
            colour[3] *= 0.5;
            guides.extend(overlay::point_guide(dragged, colour));
            world_labels.push((crate::size_labels::point_text(dragged), dragged));
        }

        // Draws links between entities and their targets in the 3D view.
        if current_filters.entity_links != crate::view_filters::EntityLinkMode::None {
            for link in document.entity_links() {
                if !current_filters.entity_links.shows(&link) {
                    continue;
                }
                let colour = crate::view2d::link_colour(&link).rgba_f32();
                // What the link does, by the links touching the selection (see view2d).
                if link.touches_selection {
                    world_labels.push((link.label(), (link.from + link.to) / 2.0));
                }
                guides.push((link.from, colour));
                guides.push((link.to, colour));
                let delta = link.to - link.from;
                let length = geometry_math::vec::length(delta);
                if length > 1e-6 {
                    let u = delta / length;
                    let side = geometry_math::vec::cross(u, camera.borrow().direction());
                    let side_length = geometry_math::vec::length(side);
                    if side_length > 1e-6 {
                        let size = (camera.borrow().world_units_per_pixel(link.to, height) * 9.0)
                            .min(length * 0.25);
                        for sign in [-1.0, 1.0] {
                            guides.push((link.to, colour));
                            guides.push((
                                link.to - u * size + side / side_length * (size * 0.45 * sign),
                                colour,
                            ));
                        }
                    }
                }
            }
        }

        // And from the linked group being edited to its other copies,
        // `GroupLinkRenderer`.
        {
            let colour = pref(|p| p.linked_group_color).rgba_f32();
            for (from, to) in document.linked_group_links() {
                guides.push((from, colour));
                guides.push((to, colour));
            }
        }

        // The Shift-extrude preview: the outline of each face a Shift-drag would move,
        // in the extrude handle's colour, `ExtrudeToolController::render`.
        let extrude_colour = pref(|p| p.resize_handle_color).rgba_f32();
        for outline in extrude_preview.borrow().iter() {
            for pair in 0..outline.len() {
                guides.push((outline[pair], extrude_colour));
                guides.push((outline[(pair + 1) % outline.len()], extrude_colour));
            }
        }

        // The box an entity drag would drop into, in the handle colour, so where it
        // lands is visible before the button is let go.
        if let Some(bounds) = *entity_drop_preview.borrow() {
            let colour = pref(|p| p.handle_color).rgba_f32();
            for (from, to) in bounds.edges() {
                guides.push((from, colour));
                guides.push((to, colour));
            }
        }

        // The HUD -- the compass turned the way the camera is, `Compass3D`, and the
        // selection's sizes when a tool wants them shown,
        // `SelectionBoundsRenderer::renderSize3D`.
        {
            let camera = camera.borrow();
            let rotation = if active == Tool::Rotate {
                let dragging = drag.borrow();
                let rotating = rotate.borrow();
                if let Some(Drag {
                    what: DragKind::Rotate { gesture },
                    applied,
                    ..
                }) = dragging.as_ref()
                {
                    let angle = if applied.x() > std::f64::consts::PI {
                        applied.x() - std::f64::consts::TAU
                    } else {
                        applied.x()
                    };
                    RotationFeedback::new(
                        &camera,
                        width,
                        height,
                        gesture,
                        rotate_ring_radius,
                        Some(angle),
                    )
                } else if pointer_inside.get() && dragging.is_none() {
                    rotating.center().and_then(|center| {
                        let (x, y) = *pointer.borrow();
                        let ray = camera.ray_through(x, y, width, height);
                        render::rotation_gizmo::RingDrag::pick(
                            &camera,
                            height,
                            &ray,
                            center,
                            rotate_ring_radius,
                            pick_radius().max(6.0),
                        )
                        .and_then(|gesture| {
                            RotationFeedback::new(
                                &camera,
                                width,
                                height,
                                &gesture,
                                rotate_ring_radius,
                                None,
                            )
                        })
                    })
                } else {
                    None
                }
            } else {
                None
            };
            let mut next = Hud {
                hidden: playing,
                rotation,
                render_aspect,
                arrows: crate::compass::arrows(
                    camera.right(),
                    camera.up(),
                    camera.direction(),
                    None,
                ),
                labels: Vec::new(),
                cameras: Vec::new(),
            };
            for (peer, (position, _)) in document.peer_cameras() {
                if let Some(anchor) = camera.project(*position, width, height) {
                    if anchor.x() >= 0.0
                        && anchor.x() <= width
                        && anchor.y() >= 0.0
                        && anchor.y() <= height
                    {
                        next.cameras
                            .push((*peer, document.peer_name(*peer), anchor));
                    }
                }
            }
            next.cameras.sort_by_key(|(peer, _, _)| *peer);
            for (text, at) in &world_labels {
                if let Some(anchor) = camera.project(*at, width, height) {
                    next.labels.push((
                        text.clone(),
                        anchor,
                        crate::size_labels::Alignment::Bottom,
                        crate::view2d::HANDLE_LABEL_GAP,
                        crate::view2d::LabelStyle::Picked,
                    ));
                }
            }
            if let Some(bounds) = document.selection_bounds() {
                let (moving, rotating) = match drag.borrow().as_ref().map(|d| &d.what) {
                    Some(DragKind::MoveObjects { .. }) => (true, false),
                    Some(DragKind::Rotate { .. }) => (false, true),
                    _ => (false, false),
                };
                let hovering_selected = !moving && !rotating && *hover_selected.borrow();
                if crate::view2d::selection_guide_shown(active, hovering_selected, moving, rotating)
                {
                    // `MapView3D::renderMap` adds a `BoundsGuideRenderer` alongside
                    // the sizes: spikes off the box's corners along the world axes.
                    guides.extend(overlay::bounds_guide(
                        &bounds,
                        pref(|p| p.selection_bounds_color).rgba_f32(),
                    ));
                    for label in
                        crate::size_labels::labels_3d(&bounds, camera.position, camera.direction())
                    {
                        if let Some(anchor) = camera.project(label.base, width, height) {
                            next.labels.push((
                                label.text,
                                anchor,
                                label.alignment,
                                crate::size_labels::GAP,
                                crate::view2d::LabelStyle::Weak,
                            ));
                        }
                    }
                }
            }
            if *hud.borrow() != next {
                *hud.borrow_mut() = next;
                hud_changed = true;
            }
        }

        guides.extend(crate::render_camera::guides(document, &camera.borrow()));

        crate::profile::add_scene_build(overlay_started);
        FrameOverlay {
            points,
            guides,
            hud_changed,
        }
    }

    /// The primary press, at `(x, y)`.
    pub fn drag_begin(&self, x: f64, y: f64, modifiers: Modifiers, (width, height): (f64, f64)) {
        self.press.set((x, y));
        if matches!(*self.tool.borrow(), Tool::Paint | Tool::Sculpt) {
            self.drag.borrow_mut().take();
            self.lasso.borrow_mut().take();
            return;
        }
        // Attribute transfer keeps the source handle selected while pointing at targets.
        if transfer_mode(modifiers).is_some()
            && self.document.borrow().single_selected_face().is_some()
        {
            self.drag.borrow_mut().take();
            return;
        }
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        *marquee.borrow_mut() = Some(document.borrow().selection_snapshot());
        let ray = camera.borrow().ray_through(x, y, width, height);
        let shift = modifiers.shift;

        let ctrl = modifiers.control;
        let alt = modifiers.alt;
        // Fillet pins edge previews; clicks away from handles select source brushes.
        if *tool.borrow() == Tool::Fillet {
            let mut handles = handles.borrow_mut();
            handles.collect(
                HandleKind::Edge,
                document.borrow().selected_brushes().into_iter(),
            );
            let hit = document.borrow().pick(&ray).map(|hit| hit.node);
            // A foreground unselected brush must not be intercepted by a selected edge behind it.
            let positions: Vec<Vec3d> = if hit.is_none_or(|id| document.borrow().is_selected(id)) {
                handles
                    .handles()
                    .iter()
                    .map(|handle| handle.position)
                    .collect()
            } else {
                Vec::new()
            };
            match handle_drag::nearest_handle(
                &camera.borrow(),
                height,
                &ray,
                positions,
                pick_radius(),
            ) {
                Some(position) => {
                    let outcome = crate::fillet_tool::press(
                        document,
                        &mut handles,
                        *fillet.borrow(),
                        position,
                        ctrl,
                    );
                    crate::fillet_tool::report(&outcome);
                }
                None => crate::fillet_tool::select_brush(
                    &mut document.borrow_mut(),
                    &mut handles,
                    hit,
                    ctrl,
                ),
            }
            drop(handles);
            *drag.borrow_mut() = None;
            redraw();
            refresh_panels();
            return;
        }

        // `AssembleBrushToolController3D` takes a drag without Ctrl or Alt: from a face of
        // a brush it draws a rectangle on that face, and with Shift from the polygon the
        // points make it pulls a copy of the polygon out along its normal. What a press
        // that stays put does is decided on release (`drag_end`), where it adds a point.
        if *tool.borrow() == Tool::Assemble && !ctrl && !alt {
            let before = assemble.borrow().points().to_vec();
            let started = if shift {
                let picked = assemble.borrow().pick_polygon(&ray);
                let polygon = assemble.borrow().polygon();
                picked
                    .zip(polygon)
                    .map(|((distance, normal), (polygon, _))| {
                        let at = ray.point_at_distance(distance);
                        let line = geometry_math::line::Line3d::new(at, normal);
                        (
                            DragKind::AssembleExtrude {
                                polygon,
                                normal,
                                before,
                            },
                            HandlePicker::line(line, Vec3d::zero()),
                            at,
                        )
                    })
            } else {
                let document = document.borrow();
                document.pick(&ray).and_then(|hit| {
                    let face = document.face_at(FaceRef {
                        node: hit.node,
                        face: hit.face?,
                    })?;
                    let at = ray.point_at_distance(hit.distance);
                    let plane =
                        geometry_math::plane::Plane3d::from_anchor_and_normal(at, face.normal());
                    Some((
                        DragKind::AssembleFace { plane, before },
                        HandlePicker::plane(plane, Vec3d::zero()),
                        at,
                    ))
                })
            };
            if let Some((what, picker, initial)) = started {
                *drag.borrow_mut() = Some(Drag {
                    what,
                    picker,
                    initial,
                    offset: Vec3d::zero(),
                    applied: Vec3d::zero(),
                });
                return;
            }
        }

        // Without modifiers, Select drags an existing selection when pressed on a brush, or
        // draws from empty space. Brush always draws. The starting corner is the ray hit, or a
        // default distance along the ray when nothing is hit.
        {
            let tool_now = *tool.borrow();
            let hit = if (tool_now == Tool::Select || tool_now == Tool::Brush) && !ctrl && !alt {
                let document = document.borrow();
                document
                    .pick(&ray)
                    .map(|hit| (hit.node, ray.point_at_distance(hit.distance)))
            } else {
                None
            };

            // `MoveObjectsToolController` moves only what is already selected, even behind
            // something unselected; what a press selects is decided on release, if it
            // stays a click (`drag_end`), so a drag over anything else draws a brush.
            // It takes no Shift, and leaves a face selection alone (`MoveObjectsTool::
            // startMove`); Alt at the press lifts the move off the ground, and Ctrl moves
            // a copy (`duplicateObjects`).
            let grabbed =
                if tool_now == Tool::Select && !shift && !document.borrow().has_face_selection() {
                    let document = document.borrow();
                    document
                        .pick_selected(&ray)
                        .map(|hit| (hit.node, ray.point_at_distance(hit.distance)))
                } else {
                    None
                };
            if tool_now == Tool::Select && !shift {
                if let Some((_, hit_point)) = grabbed {
                    if !ctrl {
                        document.borrow_mut().begin_transaction();
                    }
                    let offset = offset_of(&ray, &camera.borrow(), height, hit_point);
                    let mode = handle_drag::move_mode(alt, false, hit_point, hit_point);
                    *drag.borrow_mut() = Some(Drag {
                        what: DragKind::MoveObjects {
                            duplicate: if ctrl {
                                crate::view2d::Duplicate::Pending
                            } else {
                                crate::view2d::Duplicate::No
                            },
                        },
                        picker: handle_drag::picker_for_move(mode, hit_point, hit_point, offset),
                        initial: hit_point,
                        offset,
                        applied: Vec3d::zero(),
                    });
                    redraw();
                    refresh_panels();
                    return;
                }
            }

            if (tool_now == Tool::Brush
                // `DrawShapeToolController3D` draws only while nothing is selected, and
                // takes Shift at the press for a square base.
                || (tool_now == Tool::Select && !document.borrow().has_selection()))
                && !ctrl
                && !alt
            {
                let start = hit
                    .map(|(_, point)| point)
                    .unwrap_or_else(|| ray.point_at_distance(DEFAULT_DRAW_DISTANCE));
                *drag.borrow_mut() = Some(Drag {
                    what: DragKind::Draw { start },
                    picker: HandlePicker::plane(
                        geometry_math::plane::horizontal_plane(start),
                        Vec3d::zero(),
                    ),
                    initial: start,
                    offset: Vec3d::zero(),
                    applied: Vec3d::zero(),
                });
                redraw();
                return;
            }

            // Anything else the Select tool is pressed on is a click, decided on release:
            // a plain one selects or deselects, a Ctrl one toggles what it is on. A
            // Ctrl-drag from it paints the selection, and a Ctrl-drag from empty space is
            // the marquee below.
            if tool_now == Tool::Select
                && !shift
                && !alt
                && (!ctrl || document.borrow().pick(&ray).is_some())
            {
                *drag.borrow_mut() = Some(Drag {
                    what: if ctrl {
                        DragKind::PaintSelect {
                            faces: false,
                            pending: true,
                            before: document.borrow().selection_snapshot(),
                            handle: None,
                        }
                    } else {
                        DragKind::Click
                    },
                    picker: HandlePicker::plane(
                        geometry_math::plane::horizontal_plane(Vec3d::zero()),
                        Vec3d::zero(),
                    ),
                    initial: Vec3d::zero(),
                    offset: Vec3d::zero(),
                    applied: Vec3d::zero(),
                });
                return;
            }
        }

        // Ctrl-drag from empty space extends the selection with a marquee.
        if *tool.borrow() == Tool::Select && ctrl && document.borrow().pick(&ray).is_none() {
            *lasso.borrow_mut() = Some((Vec2d::new(x, y), Vec2d::new(x, y)));
            *drag.borrow_mut() = None;
            render();
            return;
        }

        // `VertexTool`'s split: Shift and a drag from the middle of an edge or the
        // centre of a face puts a corner there and then drags that corner, which is
        // how a brush gains a vertex.
        // `MoveVertexPart::mouseClick`: Alt and Shift with exactly one vertex picked
        // snaps that vertex onto whatever handle is clicked.
        if shift && alt && *tool.borrow() == Tool::Vertex {
            let snapped = {
                let handles = handles.borrow();
                let positions: Vec<Vec3d> = handles
                    .handles()
                    .iter()
                    .map(|handle| handle.position)
                    .collect();
                let target = handle_drag::nearest_handle(
                    &camera.borrow(),
                    height,
                    &ray,
                    positions,
                    pick_radius(),
                );
                match (target, handles.selected_anchor()) {
                    (Some(target), Some(from)) if handles.selected_count() == 1 => {
                        Some((from, target - from))
                    }
                    _ => None,
                }
            };
            if let Some((from, delta)) = snapped {
                if delta != Vec3d::zero() && document.borrow_mut().move_handles(&[from], delta) {
                    handles.borrow_mut().move_selection(delta);
                    *drag.borrow_mut() = None;
                    redraw();
                    refresh_panels();
                    return;
                }
            }
        }

        let splitting = if shift && !alt && *tool.borrow() == Tool::Vertex {
            let handles_ref = handles.borrow();
            let positions: Vec<Vec3d> = handles_ref
                .splitting()
                .map(|handle| handle.position)
                .collect();
            drop(handles_ref);
            handle_drag::nearest_handle(&camera.borrow(), height, &ray, positions, pick_radius())
        } else {
            None
        };

        if let Some(at) = splitting {
            // The corner is put where the drag takes it rather than where it started,
            // since a point sitting on an edge is no corner of the hull at all.
            handles.borrow_mut().select(at, false);
            document.borrow_mut().begin_transaction();
            let offset = offset_of(&ray, &camera.borrow(), height, at);
            *drag.borrow_mut() = Some(Drag {
                what: DragKind::SplitHandle { at },
                picker: handle_drag::picker_for_move(
                    handle_drag::MoveMode::Default,
                    at,
                    at,
                    offset,
                ),
                initial: at,
                offset,
                applied: Vec3d::zero(),
            });
            redraw();
            refresh_panels();
            return;
        }

        let started = begin_drag(
            &camera.borrow(),
            height,
            &ray,
            *tool.borrow(),
            &document.borrow(),
            &mut handles.borrow_mut(),
            &rotate.borrow(),
            &sweep.borrow(),
            shift,
            ctrl,
            alt,
        );

        // A sweep-cap or rotation-centre drag changes no geometry, so it needs no
        // transaction.
        if started.as_ref().is_some_and(|d| {
            !matches!(d.what, DragKind::SweepCap { .. } | DragKind::RotationCenter)
        }) {
            document.borrow_mut().begin_transaction();
        }
        // A press with a handle tool active belongs to the handles whether or not it
        // caught one: missing sweeps a lasso over them, the way dragging in a file
        // list sweeps over files. Falling through was what made an edge impossible to
        // pick -- a near miss threw the brush selection away, and the handles with it.
        // Let go where it was pressed with nothing picked, it selects the brush under it
        // (`drag_end`).
        if started.is_none() && HandleKind::of(*tool.borrow()).is_some() {
            *lasso.borrow_mut() = Some((Vec2d::new(x, y), Vec2d::new(x, y)));
            handles_were_picked.set(handles.borrow().selected_count() > 0);
            if !ctrl {
                handles.borrow_mut().clear_selection();
            }
        }
        // A Rotate, Scale, Shear or Sweep press that missed the tool's handles is a
        // selection click, decided on release; Alt with Sweep picks faces instead.
        let missed = started.is_none()
            && matches!(
                *tool.borrow(),
                Tool::Rotate | Tool::Scale | Tool::Shear | Tool::Sweep
            )
            && !(alt && *tool.borrow() == Tool::Sweep);
        let started = if missed {
            Some(Drag {
                what: DragKind::Click,
                picker: HandlePicker::plane(
                    geometry_math::plane::horizontal_plane(Vec3d::zero()),
                    Vec3d::zero(),
                ),
                initial: Vec3d::zero(),
                offset: Vec3d::zero(),
                applied: Vec3d::zero(),
            })
        } else {
            started
        };
        // Ctrl+Shift with nothing else taking the press paints faces into the selection.
        let paints_faces = started.is_none()
            && *tool.borrow() == Tool::Select
            && shift
            && ctrl
            && !alt
            && document
                .borrow()
                .pick(&ray)
                .is_some_and(|hit| hit.face.is_some());
        let started = if paints_faces {
            Some(Drag {
                what: DragKind::PaintSelect {
                    faces: true,
                    pending: true,
                    before: document.borrow().selection_snapshot(),
                    handle: None,
                },
                picker: HandlePicker::plane(
                    geometry_math::plane::horizontal_plane(Vec3d::zero()),
                    Vec3d::zero(),
                ),
                initial: Vec3d::zero(),
                offset: Vec3d::zero(),
                applied: Vec3d::zero(),
            })
        } else {
            started
        };

        *drag.borrow_mut() = started;
        // What a press does to the handles shows in every view, not only in this
        // one: the flat views draw the same handles and count them in their status
        // line, and they only draw when they are told to.
        redraw();
        // Which handles are picked is what the fillet panel works from.
        refresh_panels();
    }

    /// The primary drag moved `(dx, dy)` from the press.
    pub fn drag_update(&self, dx: f64, dy: f64, modifiers: Modifiers, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        // A lasso is not a drag of anything, so it is followed before the rest. The
        // rectangle is read out before it is written back: an `if let` on a borrow
        // holds that borrow for the whole body, and writing to the same cell inside
        // it is what "already borrowed" means.
        let lassoing = *lasso.borrow();
        if let Some((from, _)) = lassoing {
            let to = from + Vec2d::new(dx, dy);
            *lasso.borrow_mut() = Some((from, to));
            if *tool.borrow() == Tool::Select {
                let caught = lassoed(
                    &camera.borrow(),
                    &document.borrow(),
                    from,
                    to,
                    width,
                    height,
                );
                if let Some(initial) = marquee.borrow().as_ref() {
                    let add = modifiers.control;
                    document.borrow_mut().preview_marquee(initial, &caught, add);
                }
            }
            redraw();
            return;
        }

        // `InputEventRecorder`: a press only becomes a drag once the pointer has
        // moved off the spot. Without this a click on a handle moved it by whatever
        // the grid rounded the press point to, which read as picking a handle being
        // impossible.
        if vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD
            && !drag.borrow().as_ref().is_some_and(|d| {
                matches!(d.what, DragKind::Rotate { .. }) && d.applied != Vec3d::zero()
            })
        {
            return;
        }

        let mut drag = drag.borrow_mut();
        let Some(drag) = drag.as_mut() else {
            return;
        };
        let (start_x, start_y) = press.get();
        let ray = camera
            .borrow()
            .ray_through(start_x + dx, start_y + dy, width, height);

        let camera_direction = camera.borrow().direction();

        // A draw drag changes nothing in the document yet: it only sweeps out the box
        // every view shows as the shape it would make.
        if let DragKind::Draw { start } = &drag.what {
            let start = *start;
            // Alt raises the far corner straight up, setting the height; without it the
            // corner slides on the level it is at (`DrawShapeToolController3D::
            // modifierKeyChange`).
            let current = start + drag.applied;
            drag.picker = if modifiers.alt {
                HandlePicker::line(
                    geometry_math::line::Line3d::new(current, Vec3d::new(0.0, 0.0, 1.0)),
                    Vec3d::zero(),
                )
            } else {
                HandlePicker::plane(
                    geometry_math::plane::horizontal_plane(current),
                    Vec3d::zero(),
                )
            };
            let Some(corner) = drag.picker.pick(&ray) else {
                return;
            };
            drag.applied = corner - start;
            let world = document.borrow().world_bounds();
            *pending_shape.borrow_mut() = shaped_draw_bounds(
                start,
                corner,
                &grid.borrow(),
                camera_direction,
                modifiers,
                &world,
            );
            redraw();
            return;
        }

        if let DragKind::PaintSelect { faces, pending, .. } = &mut drag.what {
            let faces = *faces;
            // What the press was on goes in first, as `SelectionTool::acceptMouseDrag`
            // selects it before the tracker follows the pointer.
            let mut rays = Vec::new();
            if std::mem::take(pending) {
                rays.push(camera.borrow().ray_through(start_x, start_y, width, height));
            }
            rays.push(ray);
            let mut document = document.borrow_mut();
            let mut changed = false;
            for ray in &rays {
                let Some(hit) = document.pick(ray) else {
                    continue;
                };
                changed |= match (faces, hit.face) {
                    (true, Some(face)) => document.paint_select_face(FaceRef {
                        node: hit.node,
                        face,
                    }),
                    (true, None) => false,
                    (false, _) => document.paint_select(hit.node),
                };
            }
            drop(document);
            if changed {
                redraw();
                render();
                refresh_panels();
            }
            return;
        }

        if matches!(
            drag.what,
            DragKind::AssembleFace { .. } | DragKind::AssembleExtrude { .. }
        ) {
            let points = assemble_drag_points(drag, &ray, &document.borrow(), &grid.borrow());
            if let Some(points) = points {
                assemble.borrow_mut().set_points(&points);
                render();
            }
            return;
        }

        if update_drag(
            drag,
            &ray,
            camera_direction,
            &grid.borrow(),
            &mut document.borrow_mut(),
            &mut handles.borrow_mut(),
            &mut rotate.borrow_mut(),
            &mut sweep.borrow_mut(),
            modifiers,
        ) {
            // The sweep panel's numbers follow the cap.
            if matches!(drag.what, DragKind::SweepCap { .. }) {
                refresh_panels();
            }
            if matches!(
                drag.what,
                DragKind::Rotate { .. } | DragKind::RotationCenter
            ) {
                redraw();
            } else {
                render();
            }
        }
    }

    /// The primary release, `(dx, dy)` from the press.
    pub fn drag_end(&self, dx: f64, dy: f64, modifiers: Modifiers, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        if let Some((from, to)) = lasso.borrow_mut().take() {
            let add = modifiers.control;

            // With a handle tool in hand the lasso sweeps up handles rather than
            // objects: `VertexToolController`'s own lasso, which is how several
            // vertices or edges are taken at once.
            if HandleKind::of(*tool.borrow()).is_some() {
                // A click on no handle with none picked selects the brush under it
                // (`NodeHandleToolControllerParts`: the chain falls through).
                if vec::distance(from, to) < DRAG_THRESHOLD && !handles_were_picked.get() {
                    let (x, y) = press.get();
                    self.click_at(1, x, y, modifiers, (width, height), true);
                    return;
                }
                if vec::distance(from, to) >= DRAG_THRESHOLD {
                    let camera = camera.borrow();
                    let mut handles = handles.borrow_mut();
                    let caught: Vec<Vec3d> = handles
                        .handles()
                        .iter()
                        .map(|handle| handle.position)
                        .filter(|position| {
                            camera
                                .project(*position, width, height)
                                .is_some_and(|on_screen| inside(from, to, on_screen))
                        })
                        .collect();
                    if !add {
                        handles.clear_selection();
                    }
                    for position in &caught {
                        if !handles.is_selected(*position) {
                            handles.select(*position, true);
                        }
                    }
                    drop(handles);
                    drop(camera);
                    crate::say!(
                        "{}",
                        fill("Selected {}", &[&counted(caught.len(), "handle")])
                    );
                    refresh_panels();
                }
                redraw();
                return;
            }

            // A drag of a pixel or two is a click that shook; the click handler has
            // already dealt with it.
            if vec::distance(from, to) >= DRAG_THRESHOLD {
                let caught = lassoed(
                    &camera.borrow(),
                    &document.borrow(),
                    from,
                    to,
                    width,
                    height,
                );
                let mut document = document.borrow_mut();
                if let Some(initial) = marquee.borrow_mut().take() {
                    document.preview_marquee(&initial, &caught, add);
                }
                crate::say!(
                    "{}",
                    fill("{} selected", &[&counted(caught.len(), "object")])
                );
                drop(document);
                refresh_panels();
            }
            render();
            return;
        }

        let ended = drag.borrow_mut().take();
        // A press that never moved past the threshold was a click: it draws and moves
        // nothing, and selects what it was on, as `SelectionTool::mouseClick` does.
        let still = vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD;
        if still
            && matches!(
                &ended,
                Some(Drag {
                    what: DragKind::Draw { .. }
                        | DragKind::MoveObjects { .. }
                        | DragKind::Click
                        | DragKind::PaintSelect { .. }
                        | DragKind::AssembleFace { .. }
                        | DragKind::AssembleExtrude { .. },
                    ..
                })
            )
        {
            *pending_shape.borrow_mut() = None;
            document.borrow_mut().cancel_transaction();
            if let Some(Drag {
                what:
                    DragKind::PaintSelect {
                        handle: Some(handle),
                        ..
                    },
                ..
            }) = &ended
            {
                handles.borrow_mut().select(*handle, true);
                redraw();
                refresh_panels();
                return;
            }
            let (x, y) = press.get();
            self.click_at(1, x, y, modifiers, (width, height), true);
            render();
            return;
        }
        // A handle let go where it was taken: Ctrl unpicks one that was picked, and a
        // plain click narrows what is picked to it (`NodeHandleToolControllerParts`).
        if let Some(Drag {
            what: DragKind::Handles {
                caught, was_picked, ..
            },
            ..
        }) = &ended
        {
            if still {
                document.borrow_mut().cancel_transaction();
                if modifiers.control && *was_picked {
                    handles.borrow_mut().select(*caught, true);
                } else if !modifiers.control && !modifiers.shift {
                    handles.borrow_mut().select(*caught, false);
                }
                redraw();
                refresh_panels();
                return;
            }
        }
        if let Some(Drag {
            what: DragKind::Draw { .. },
            ..
        }) = &ended
        {
            let bounds = pending_shape.borrow_mut().take();
            if let Some(bounds) = bounds {
                let mut document = document.borrow_mut();
                // The tool stays as it was (`DrawShapeTool`).
                match document.begin_shape_preview(bounds, *shape_parameters.borrow()) {
                    Ok(_) => {}
                    Err(error) => crate::complain!(
                        "{}",
                        fill("Could not draw the shape: {}", &[&error.to_string()])
                    ),
                }
            }
            redraw();
            refresh_panels();
            return;
        }
        if ended.is_some() {
            document.borrow_mut().commit_transaction();
            // A Ctrl-drag's copy undoes with its move.
            if let Some(Drag {
                what:
                    DragKind::MoveObjects {
                        duplicate: crate::view2d::Duplicate::Made(mark),
                    },
                ..
            }) = &ended
            {
                document.borrow_mut().fold_edits_since(*mark);
            }
            render();
        }
        // A Shift press on a selected face that never moved it was a click after all:
        // Shift-click selects the face under the pointer, as `SelectionTool` does on
        // release.
        if let Some(Drag {
            what: DragKind::Extrude { .. },
            applied,
            ..
        }) = ended
        {
            if applied == Vec3d::zero() && modifiers.shift {
                let (x, y) = press.get();
                self.click_at(1, x, y, modifiers, (width, height), true);
            }
        }
    }

    /// The primary drag was taken away without a release.
    pub fn drag_cancel(&self) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        if lasso.borrow_mut().take().is_some() {
            if *tool.borrow() == Tool::Select {
                if let Some(initial) = marquee.borrow_mut().take() {
                    document.borrow_mut().restore_selection(&initial);
                }
            }
            redraw();
        }
        // What the drag did is kept as one step, as a release would, and the transaction
        // closed so it cannot swallow later edits; a brush being drawn is dropped. Esc
        // undoes instead (`escape`).
        if drag.borrow_mut().take().is_some() {
            document.borrow_mut().commit_transaction();
            render();
        }
        *pending_shape.borrow_mut() = None;
    }

    /// A double click released at `(x, y)`: [`Document::double_click`] on what is under
    /// it, with the Select or Brush tool in hand. Alt with one face selected is the
    /// attribute copy `click` already made. Returns whether anything changed.
    pub fn double_click(
        &self,
        x: f64,
        y: f64,
        modifiers: Modifiers,
        (width, height): (f64, f64),
    ) -> bool {
        // `AssembleBrushToolController3D::mouseDoubleClick`: a plain double click adds
        // the corners of the face under it to the assemble tool's points.
        if *self.tool.borrow() == Tool::Assemble {
            if modifiers.shift || modifiers.control || modifiers.alt {
                return false;
            }
            let ray = self.camera.borrow().ray_through(x, y, width, height);
            let document = self.document.borrow();
            let corners = document.pick(&ray).and_then(|hit| {
                let face = map_model::polyhedron::FaceId(u32::try_from(hit.face?).ok()?);
                Some(document.brush(hit.node)?.geometry().face_positions(face))
            });
            drop(document);
            let mut assemble = self.assemble.borrow_mut();
            let mut changed = false;
            for corner in corners.unwrap_or_default() {
                changed |= assemble.add_point(corner);
            }
            drop(assemble);
            if changed {
                (self.render)();
            }
            return changed;
        }
        if !matches!(*self.tool.borrow(), Tool::Select | Tool::Brush) {
            return false;
        }
        if modifiers.alt && self.document.borrow().single_selected_face().is_some() {
            return false;
        }
        let ray = self.camera.borrow().ray_through(x, y, width, height);
        let mut document = self.document.borrow_mut();
        let hit = document.pick(&ray);
        let changed = document.double_click(hit, modifiers.shift, modifiers.alt, modifiers.control);
        drop(document);
        if changed {
            (self.render)();
            (self.refresh_panels)();
        }
        changed
    }

    /// Ends painting, or rolls back a live attribute sweep taken away by Esc.
    pub fn stroke_cancel(&self) {
        let transfer = matches!(*self.stroke.borrow(), Some(AttrStroke::Transfer { .. }));
        if transfer {
            if let Some(AttrStroke::Transfer { mark, revision, .. }) =
                self.stroke.borrow_mut().take()
            {
                let mut document = self.document.borrow_mut();
                if document.revision() == revision && document.edit_mark() > mark {
                    document.undo();
                }
            }
            (self.render)();
            return;
        }
        self.stroke_end();
    }

    /// A primary click, `presses` times, at `(x, y)`.
    pub fn click(&self, presses: i32, x: f64, y: f64, modifiers: Modifiers, size: (f64, f64)) {
        self.click_at(presses, x, y, modifiers, size, false);
    }

    /// [`Self::click`], or with `released` the click a press turned out to be once let
    /// go, which selects even with a tool in hand whose press the drag owns.
    fn click_at(
        &self,
        presses: i32,
        x: f64,
        y: f64,
        modifiers: Modifiers,
        (width, height): (f64, f64),
        released: bool,
    ) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        // Clicking the view takes the keyboard off whatever number field or entry
        // had it, so WASD and the arrows reach the view again.
        focus();
        // The click can arrive before the drag has stored its state.
        // Transform handles extend outside the brush: selecting here would clear
        // or replace the selection before the drag starts. These tools own the
        // primary-button press, including when their handle has not been caught.
        // A Shift press that took hold of a selected face for resizing is a click only if
        // it is let go where it was: `drag_end` selects the face then. Selecting it now
        // would throw away the very selection the resize works on.
        if (!released && matches!(*tool.borrow(), Tool::Rotate | Tool::Scale | Tool::Shear))
            || drag.borrow().is_some()
        {
            return;
        }
        let ray = camera.borrow().ray_through(x, y, width, height);

        let mut document = document.borrow_mut();

        // `SetBrushFaceAttributesTool`: with one face selected, Alt and a double click
        // copies its material -- and by the modifiers its mapping -- onto every face
        // of the brush under the pointer. The single-face and swept-across cases are
        // the drag gesture below, where a plain click is a zero-length drag.
        if let Some(mode) = transfer_mode(modifiers) {
            if presses >= 2 {
                if let (Some(source), Some(hit)) =
                    (document.single_selected_face(), document.pick(&ray))
                {
                    if let Some(face) = hit.face {
                        let target = FaceRef {
                            node: hit.node,
                            face,
                        };
                        // The first click's single-face copy and this one undo together,
                        // as `SetBrushFaceAttributesTool` undoes the first before it.
                        let folded = transfer_marks.take().filter(|(_, after)| {
                            *after == document.edit_mark()
                                || matches!(
                                    stroke.borrow().as_ref(),
                                    Some(AttrStroke::Transfer { mark, .. }) if mark == after
                                )
                        });
                        if document.copy_face_attributes(source, target, mode, true) {
                            if let Some((before, _)) = folded {
                                document.fold_edits_since(before);
                            }
                            // The sweep this press began is the double click's.
                            stroke.borrow_mut().take();
                            drop(document);
                            render();
                            return;
                        }
                    }
                }
            }
            return;
        }

        // A handle tool has already had this press, in the drag controller above: it
        // is about the handles, not about which brush is selected.
        if HandleKind::of(*tool.borrow()).is_some() {
            return;
        }

        // A press that began a sculpt stroke is the stroke's; anywhere else it selects.
        if *tool.borrow() == Tool::Sculpt
            && matches!(*stroke.borrow(), Some(AttrStroke::Sculpt { .. }))
        {
            return;
        }

        // The paint tool puts the current material on the face under the pointer: a
        // click paints one face, a double click the whole brush, and Shift takes the
        // face's material as the current one instead, the way an eyedropper does.
        if *tool.borrow() == Tool::Paint {
            if let Some(hit) = document.pick(&ray) {
                if let Some(face) = hit.face {
                    let target = FaceRef {
                        node: hit.node,
                        face,
                    };
                    if modifiers.shift {
                        if let Some(picked) =
                            document.face_at(target).map(|f| f.material_name.clone())
                        {
                            document.set_current_material(picked);
                            drop(document);
                            refresh_panels();
                            render();
                            return;
                        }
                    } else {
                        let material = document.current_material().to_string();
                        let painted = if let Some(AttrStroke::Paint { state, .. }) =
                            stroke.borrow_mut().as_mut()
                        {
                            let targets = if presses >= 2 {
                                (0..document.brush(hit.node).unwrap().geometry().face_count())
                                    .map(|face| FaceRef {
                                        node: hit.node,
                                        face,
                                    })
                                    .collect()
                            } else {
                                vec![target]
                            };
                            document.paint_material_stroke(&targets, state) > 0
                        } else if presses >= 2 {
                            document.paint_brush(hit.node, &material)
                        } else {
                            document.paint_face(target, &material)
                        };
                        if painted {
                            drop(document);
                            render();
                            return;
                        }
                    }
                }
            }
            return;
        }

        // The clip tool places its points here too, not only in the flat views. A point goes where the ray meets a surface, snapped;
        // the face's own normal is the help vector, which is what lets two points
        // placed in one view describe a plane -- `ClipToolController3D`.
        // A double click puts the plane on the face under it, and a click with any
        // modifier is a selection click (`ClipToolController` takes only plain clicks).
        let plain = !modifiers.shift && !modifiers.control && !modifiers.alt;
        if *tool.borrow() == Tool::Clip && plain && presses == 2 {
            let face = document.pick(&ray).and_then(|hit| {
                hit.face.map(|face| FaceRef {
                    node: hit.node,
                    face,
                })
            });
            if let Some(points) = face.and_then(|face| document.face_points(face)) {
                clip.borrow_mut().set_face(points);
                drop(document);
                refresh_panels();
                render();
            }
            return;
        }
        if *tool.borrow() == Tool::Clip && plain {
            if let Some((at, help_vectors)) = clip_surface_point(&document, &grid.borrow(), &ray) {
                if clip.borrow_mut().add_point(at, help_vectors) {
                    drop(document);
                    refresh_panels();
                    render();
                    return;
                }
            }
            return;
        }

        // With the assemble tool active a click places a point rather than selecting.
        // The point goes where the ray meets a surface, snapped, since building a
        // shape against the geometry already there is what the tool is for.
        if *tool.borrow() == Tool::Assemble {
            // On the face's plane, as `AssembleBrushToolController3D::mouseClick` snaps
            // it (`Grid::snap` with the face's boundary), which is the clip tool's point.
            if let Some((at, _)) = clip_surface_point(&document, &grid.borrow(), &ray) {
                if assemble.borrow_mut().add_point(at) {
                    drop(document);
                    render();
                    return;
                }
            }
            return;
        }

        // `SelectionTool`: Shift picks the individual face under the pointer rather
        // than the whole brush (`isFaceClick`), and Ctrl adds to the selection
        // (`isMultiClick`) -- so Shift+Ctrl picks several faces.
        let add_to_selection = modifiers.control;
        let pick_face = modifiers.shift;

        match (document.pick(&ray), pick_face) {
            // Shift picks a face, which only a brush has.
            (Some(hit), true) if hit.face.is_some() => {
                let face = FaceRef {
                    node: hit.node,
                    face: hit.face.expect("checked just above"),
                };
                if add_to_selection {
                    document.toggle_face_selected(face);
                } else {
                    document.select_face_only(face);
                }
            }
            (Some(hit), true) if add_to_selection => document.toggle_selected(hit.node),
            (Some(hit), true) => document.select_only(hit.node),
            (Some(hit), false) if add_to_selection => document.toggle_selected(hit.node),
            (Some(hit), false) => document.select_only(hit.node),
            (None, _) if !add_to_selection => document.clear_selection(),
            (None, _) => {}
        }
        render();
    }

    /// The face-attribute stroke's press.
    pub fn stroke_begin(&self, x: f64, y: f64, modifiers: Modifiers, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        *stroke.borrow_mut() = None;
        if *tool.borrow() == Tool::Sculpt
            && !modifiers.alt
            && filters.borrow().animation_frame.is_none()
        {
            let ray = camera.borrow().ray_through(x, y, width, height);
            let Some(at) = document.borrow().sculpt_target(&ray) else {
                return;
            };
            let parameters = crate::sculpt::parameters();
            let step = grid.borrow().actual_size();
            let state = match parameters.form {
                crate::sculpt::SculptForm::Bend => document
                    .borrow()
                    .begin_sculpt_stroke_under(&ray, parameters.direction),
                crate::sculpt::SculptForm::Blocks => {
                    Some(document.borrow().begin_block_stroke(step))
                }
                crate::sculpt::SculptForm::Free => {
                    let axis = document.borrow().sculpt_axis(&ray, parameters.direction);
                    axis.map(|along| document.borrow().begin_free_stroke(step, along))
                }
            };
            let Some(mut state) = state else {
                return;
            };
            if document
                .borrow_mut()
                .sculpt_dab(at, &sculpt_brush(modifiers.shift), &mut state)
            {
                render();
            }
            *stroke.borrow_mut() = Some(AttrStroke::Sculpt {
                state,
                last: at,
                inverted: modifiers.shift,
                rested: 0.0,
            });
            return;
        }
        if *tool.borrow() == Tool::Paint && !modifiers.shift && !modifiers.alt {
            *stroke.borrow_mut() = Some(AttrStroke::Paint {
                state: document.borrow().begin_material_stroke(),
                last: (x, y),
            });
            return;
        }
        let Some(mode) = transfer_mode(modifiers) else {
            return;
        };
        let document = document.borrow();
        let Some(source) = document.single_selected_face() else {
            return;
        };
        let mark = document.edit_mark();
        let revision = document.revision();
        drop(document);
        *stroke.borrow_mut() = Some(AttrStroke::Transfer {
            source,
            mode,
            touched: Vec::new(),
            mark,
            revision,
        });
        self.stroke_update((x, y), 0.0, 0.0, (width, height));
    }

    /// The face-attribute stroke moved; `start` is where it began.
    pub fn stroke_update(&self, start: (f64, f64), dx: f64, dy: f64, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        let mut stroke = stroke.borrow_mut();
        let Some(stroke) = stroke.as_mut() else {
            return;
        };
        let point = (start.0 + dx, start.1 + dy);
        match stroke {
            AttrStroke::Transfer {
                source,
                mode,
                touched,
                mark,
                revision,
            } => {
                if document.borrow().revision() != *revision {
                    return;
                }
                let face = face_under(
                    &camera.borrow(),
                    (width, height),
                    &document.borrow(),
                    point.0,
                    point.1,
                );
                if let Some(face) = face {
                    if !touched.contains(&face) {
                        touched.push(face);
                        let mut document = document.borrow_mut();
                        let before = document.edit_mark();
                        if document.copy_face_attributes_across(*source, &[face], *mode) > 0 {
                            let evicted = (before + 1).saturating_sub(document.edit_mark());
                            *mark = mark.saturating_sub(evicted);
                            document.fold_edits_since(*mark);
                            *revision = document.revision();
                            drop(document);
                            render();
                        }
                    }
                }
            }
            AttrStroke::Paint { state, last } => {
                if *tool.borrow() != Tool::Paint || filters.borrow().animation_frame.is_some() {
                    return;
                }
                let distance = (point.0 - last.0).hypot(point.1 - last.1);
                let steps = ((distance / 3.0).ceil() as usize).clamp(1, 128);
                let mut faces = Vec::new();
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    let at = (
                        last.0 + (point.0 - last.0) * t,
                        last.1 + (point.1 - last.1) * t,
                    );
                    if at.0 < 0.0 || at.1 < 0.0 || at.0 > width || at.1 > height {
                        continue;
                    }
                    if let Some(face) = face_under(
                        &camera.borrow(),
                        (width, height),
                        &document.borrow(),
                        at.0,
                        at.1,
                    ) {
                        if !faces.contains(&face) {
                            faces.push(face);
                        }
                    }
                }
                *last = point;
                if document.borrow_mut().paint_material_stroke(&faces, state) > 0 {
                    render();
                }
            }
            AttrStroke::Sculpt {
                state,
                last,
                inverted,
                rested,
            } => {
                if *tool.borrow() != Tool::Sculpt {
                    return;
                }
                let ray = camera.borrow().ray_through(point.0, point.1, width, height);
                let Some(at) = document.borrow().sculpt_target(&ray) else {
                    return;
                };
                let brush = sculpt_brush(*inverted);
                // Dabs a spacing apart along the way, so a quick drag leaves no gaps.
                let distance = geometry_math::vec::length(at - *last);
                let steps = (distance / brush.spacing()).floor() as usize;
                let mut changed = false;
                for i in 1..=steps.min(64) {
                    let t = i as f64 / steps as f64;
                    changed |=
                        document
                            .borrow_mut()
                            .sculpt_dab(*last + (at - *last) * t, &brush, state);
                }
                if steps > 0 {
                    *last = at;
                    *rested = 0.0;
                }
                if changed {
                    render();
                }
            }
        }
    }

    /// Keeps a sculpt stroke working while the button is held: after the pointer at
    /// `x`, `y` has rested `HOLD_DAB_INTERVAL` since the last dab, dabs again under it.
    /// `seconds` is the time since the last call. Returns whether a sculpt stroke is under
    /// way, so the caller keeps the frames coming.
    pub fn stroke_hold(&self, x: f64, y: f64, seconds: f64, (width, height): (f64, f64)) -> bool {
        let Self {
            document,
            tool,
            camera,
            render,
            stroke,
            ..
        } = self;
        let mut stroke = stroke.borrow_mut();
        let Some(AttrStroke::Sculpt {
            state,
            last,
            inverted,
            rested,
        }) = &mut *stroke
        else {
            return false;
        };
        if *tool.borrow() != Tool::Sculpt {
            return false;
        }
        // A long frame, say after a slow dab, counts as one interval, so dabs never pile up.
        *rested += seconds.min(HOLD_DAB_INTERVAL);
        if *rested < HOLD_DAB_INTERVAL {
            return true;
        }
        *rested = 0.0;
        // A free stroke keeps working the spot the last dab was aimed at, and the dab
        // lands from there on the ground along the stroke: aimed again, the pointer would
        // find the near side of what it grows and wander off towards the camera.
        let at = if state.is_free() {
            *last
        } else {
            let ray = camera.borrow().ray_through(x, y, width, height);
            let Some(at) = document.borrow().sculpt_target(&ray) else {
                return true;
            };
            at
        };
        *last = at;
        if document
            .borrow_mut()
            .sculpt_dab(at, &sculpt_brush(*inverted), state)
        {
            render();
        }
        true
    }

    /// The face-attribute stroke's release.
    pub fn stroke_end(&self) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        let Some(stroke) = stroke.borrow_mut().take() else {
            return;
        };
        if let AttrStroke::Sculpt { mut state, .. } = stroke {
            if state.is_free() {
                document.borrow_mut().end_free_stroke(&mut state);
            }
            render();
            return;
        }
        if let AttrStroke::Transfer { mark, revision, .. } = stroke {
            let document = document.borrow();
            if document.revision() == revision && document.edit_mark() > mark {
                transfer_marks.set(Some((mark, document.edit_mark())));
            }
        }
        render();
    }

    /// The pointer moved over the view.
    pub fn motion(&self, x: f64, y: f64, modifiers: Modifiers, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        *pointer.borrow_mut() = (x, y);
        // The brush outline follows the pointer.
        if *tool.borrow() == Tool::Sculpt {
            render();
        }
        let ray = camera.borrow().ray_through(x, y, width, height);
        // With Shift held, outline the faces a Shift-drag would extrude, as
        // `ExtrudeToolController::render` does with the proposed drag handles.
        let shift = modifiers.shift;
        let outlines = if shift {
            document.borrow().extrude_preview_faces(&ray)
        } else {
            Vec::new()
        };
        if *extrude_preview.borrow() != outlines {
            *extrude_preview.borrow_mut() = outlines;
            render();
        }
        let hit = document.borrow().pick(&ray);
        // Whether that is something selected, for the selection's size labels. A
        // change is worth a frame; the HUD is rebuilt from it in the render closure.
        let over = hit.is_some_and(|hit| document.borrow().is_selected(hit.node));
        if *hover_selected.borrow() != over {
            *hover_selected.borrow_mut() = over;
            render();
        }
        // Where a paste would land: the surface under the pointer, or a point out in
        // front of the camera when the pointer is over open space.
        let at = match hit {
            Some(hit) => ray.point_at_distance(hit.distance),
            None => {
                let camera = camera.borrow();
                camera.position + camera.direction() * 256.0
            }
        };
        *paste_target.borrow_mut() = Some(at);
    }

    /// The pointer left the view.
    pub fn leave(&self) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        if *hover_selected.borrow() {
            *hover_selected.borrow_mut() = false;
            render();
        }
    }

    /// A wheel notch; `dy` below zero is away from the user.
    pub fn scroll(&self, dy: f64, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        // `CameraTool3D::moveSpeed`: the preference times twenty, over a wheel
        // notch rather than a pixel of drag, so it moves a useful distance.
        let speed = f64::from(pref(|p| p.camera_move_speed)) * 20.0 * 4.0;
        let direction = if pref(|p| p.invert_mouse_wheel) {
            1.0
        } else {
            -1.0
        };
        let distance = direction * dy * speed;

        let mut camera = camera.borrow_mut();
        if pref(|p| p.camera_move_in_cursor_dir) {
            // Towards what the pointer is over rather than straight ahead, which is
            // what makes zooming in on a corner keep it in view.
            let (x, y) = *pointer.borrow();
            let height = height.max(1.0);
            let width = width.max(1.0);
            let ray = camera.ray_through(x, y, width, height);
            camera.position += ray.direction * distance;
        } else {
            camera.fly(distance, 0.0, 0.0);
        }
        render();
    }

    /// The wheel with modifiers, `dy` notches towards the user positive. Returns whether
    /// a modifier made it something other than flying: Shift zooms the field of view
    /// (`CameraTool3D`, `m_camera.zoom`), Ctrl drills the selection through what is under
    /// the pointer, and Ctrl+Alt changes the grid (`SelectionTool::mouseScroll`).
    pub fn scroll_with(&self, dy: f64, modifiers: Modifiers, size: (f64, f64)) -> bool {
        let invert = pref(|p| p.invert_mouse_wheel);
        let away = (dy < 0.0) != invert;
        if modifiers.control && modifiers.alt && !modifiers.shift {
            let mut grid = self.grid.borrow_mut();
            if away {
                grid.decrease();
            } else {
                grid.increase();
            }
            crate::say!(
                "{}",
                fill("Grid size {}", &[&grid.actual_size().to_string()])
            );
            drop(grid);
            (self.redraw)();
            (self.render)();
            return true;
        }
        if modifiers.control && !modifiers.alt && !modifiers.shift {
            let (x, y) = *self.pointer.borrow();
            let ray = self.ray(x, y, size);
            if self
                .document
                .borrow_mut()
                .drill_selection(&ray, away, false)
            {
                (self.refresh_panels)();
                (self.render)();
            }
            return true;
        }
        if modifiers.shift && !modifiers.control && !modifiers.alt {
            // Narrower towards the scene, as a zoom lens.
            let factor = if away { 1.0 / 1.08 } else { 1.08 };
            crate::preferences::edit(|p| p.camera_fov = (p.camera_fov * factor).clamp(30.0, 140.0));
            (self.render)();
            return true;
        }
        false
    }

    /// The modifiers changed under a still pointer.
    pub fn modifiers_changed(&self, state: Modifiers, (width, height): (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            document,
            tool,
            handles,
            rotate,
            camera,
            fillet,
            pending_shape,
            shape_parameters,
            clip,
            assemble,
            grid,
            filters,
            sweep,
            paste_target,
            refresh_panels,
            redraw,
            render,
            focus,
            rendered_frame,
            preview_motions,
            preview_mesh,
            fillet_surfaces,
            uploaded_surface_epoch,
            hud,
            hover_selected,
            drag,
            uploaded,
            uploaded_materials,
            uploaded_filters,
            uploaded_clip,
            last_frame,
            extrude_preview,
            entity_drop_preview,
            pointer,
            pointer_inside,
            lasso,
            marquee,
            stroke,
            look_last,
            pan_last,
            press,
            handles_were_picked,
            transfer_marks,
        } = self;

        let (x, y) = *pointer.borrow();
        let ray = camera.borrow().ray_through(x, y, width, height);
        let outlines = if state.shift {
            document.borrow().extrude_preview_faces(&ray)
        } else {
            Vec::new()
        };
        if *extrude_preview.borrow() != outlines {
            *extrude_preview.borrow_mut() = outlines;
            render();
        }
    }
}

#[cfg(test)]
mod clip_preview_tests {
    use super::*;

    #[test]
    fn clip_surface_snap_stays_on_sloped_and_offset_faces() {
        let grid = Grid::new(4);
        for normal in [
            Vec3d::new(0., 0., 1.),
            Vec3d::new(1., 0., 1.),
            Vec3d::new(0., 1., 0.),
        ] {
            let surface = Vec3d::new(21., 37., 5.);
            let at = snap_clip_surface(&grid, surface, normal);
            assert!(vec::dot(normal, at - surface).abs() < 1e-9);
            assert!(
                (0..3)
                    .filter(|axis| at[*axis] == grid.snap_point(at)[*axis])
                    .count()
                    >= 2
            );
        }
    }
}

#[cfg(test)]
mod pick_tests {
    use super::*;
    use geometry_math::vec::Vec3d;

    #[test]
    fn a_press_on_a_handle_in_the_3d_view_picks_it() {
        // The 3D view as it opens: the default camera, a pane about 435 by 315.
        let camera = crate::camera3d::Camera3d::default();
        let (width, height) = (435.0, 315.0);
        // An edge midpoint of the 128-unit box in the smoke map.
        let handle = Vec3d::new(0.0, -64.0, 16.0);

        let on_screen = camera
            .project(handle, width, height)
            .expect("the handle is in front of the camera");
        let ray = camera.ray_through(on_screen.x(), on_screen.y(), width, height);

        let caught = crate::handle_drag::nearest_handle(
            &camera,
            height,
            &ray,
            [handle, Vec3d::new(64.0, 0.0, 16.0)],
            pick_radius(),
        );
        assert_eq!(caught, Some(handle), "the press was right on it");
    }

    #[test]
    fn shift_resizes_a_crate_from_just_off_its_outline_not_the_floor_under_it() {
        let camera = crate::camera3d::Camera3d::default();
        let (width, height) = (435.0, 315.0);
        let mut document = Document::new(Default::default());
        let bounds = document.world_bounds();
        let cuboid = |min: Vec3d, max: Vec3d| {
            map_model::shapes::create_cuboid(&bounds, &Bbox3d::new(min, max), "m").unwrap()
        };
        document.add_brushes(vec![cuboid(
            Vec3d::new(-512.0, -512.0, -16.0),
            Vec3d::new(512.0, 512.0, 0.0),
        )]);
        let crate_box = cuboid(Vec3d::new(-32.0, -32.0, 0.0), Vec3d::new(32.0, 32.0, 64.0));
        document.add_brushes(vec![crate_box.clone()]);
        // The crate's corner furthest right on screen, at its top, and a little past it:
        // the ray misses the crate and meets the floor behind.
        let corner = crate_box
            .vertex_positions()
            .into_iter()
            .filter(|p| p.z() > 32.0)
            .filter_map(|p| camera.project(p, width, height))
            .max_by(|a, b| a.x().total_cmp(&b.x()))
            .unwrap();
        let ray = camera.ray_through(corner.x() + 3.0, corner.y(), width, height);
        assert!(
            document.pick_selected(&ray).is_none(),
            "the ray misses the crate"
        );
        assert!(document.pick(&ray).is_some(), "and meets the floor");
        let drag = begin_extrude(&camera, height, &ray, &document, false, false)
            .expect("Shift takes hold of the crate");
        let DragKind::Extrude { brush, .. } = drag.what else {
            panic!("not a resize");
        };
        assert_eq!(
            brush.bounds(),
            crate_box.bounds(),
            "the crate, not the floor"
        );
    }

    #[test]
    fn a_draw_across_the_ground_makes_a_grid_tall_slab_in_front_of_the_camera() {
        let grid = Grid::new(4); // 16 units
                                 // Two corners on the same z, as a horizontal-plane drag gives.
        let start = Vec3d::new(3.0, 3.0, 0.0);
        let corner = Vec3d::new(40.0, 40.0, 0.0);
        // A camera looking down: the slab grows upward.
        let bounds =
            draw_bounds(start, corner, &grid, Vec3d::new(0.0, 0.0, -1.0)).expect("a solid box");

        assert_eq!(bounds.min, Vec3d::new(0.0, 0.0, 0.0), "snapped down");
        assert_eq!(
            bounds.max,
            Vec3d::new(48.0, 48.0, 16.0),
            "snapped up, one grid tall"
        );
    }

    #[test]
    fn a_camera_looking_up_grows_the_slab_downward() {
        let grid = Grid::new(4);
        let start = Vec3d::new(0.0, 0.0, 0.0);
        let corner = Vec3d::new(32.0, 32.0, 0.0);
        let bounds =
            draw_bounds(start, corner, &grid, Vec3d::new(0.0, 0.0, 1.0)).expect("a solid box");
        assert_eq!(bounds.min.z(), -16.0, "the box is below the drag");
        assert_eq!(bounds.max.z(), 0.0);
    }

    #[test]
    fn dragging_a_selected_brush_in_the_3d_view_slides_it_across_the_ground() {
        use crate::world::WorldBounds;
        use geometry_math::bbox::Bbox3d;

        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .expect("the cube is built");
        document.add_brushes(vec![brush]);
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        document.begin_transaction();

        // The press grabbed the top face; the pointer is now straight above a point
        // 48 units along +x from it.
        let initial = Vec3d::new(32.0, 32.0, 64.0);
        let mut drag = Drag {
            what: DragKind::MoveObjects {
                duplicate: crate::view2d::Duplicate::No,
            },
            picker: handle_drag::picker_for_move(
                handle_drag::MoveMode::Default,
                initial,
                initial,
                Vec3d::zero(),
            ),
            initial,
            offset: Vec3d::zero(),
            applied: Vec3d::zero(),
        };
        let ray = geometry_math::ray::Ray3d {
            origin: Vec3d::new(80.0, 32.0, 200.0),
            direction: Vec3d::new(0.0, 0.0, -1.0),
        };

        let moved = update_drag(
            &mut drag,
            &ray,
            Vec3d::new(0.0, 0.0, -1.0),
            &Grid::new(4),
            &mut document,
            &mut HandleManager::new(),
            &mut RotateTool::new(),
            &mut crate::sweep_tool::SweepInputs::default(),
            Modifiers::default(),
        );

        assert!(moved, "the pointer moved far enough to slide the brush");
        assert_eq!(
            drag.applied,
            Vec3d::new(48.0, 0.0, 0.0),
            "snapped to the grid"
        );
        let bounds = document
            .selection_bounds()
            .expect("the brush is still selected");
        let near = |a: Vec3d, b: Vec3d| (a - b).v.iter().all(|d| d.abs() < 1e-6);
        assert!(
            near(bounds.min, Vec3d::new(48.0, 0.0, 0.0)),
            "min at {:?}",
            bounds.min
        );
        assert!(
            near(bounds.max, Vec3d::new(112.0, 64.0, 64.0)),
            "max at {:?}",
            bounds.max
        );
    }
    #[test]
    fn ring_drag_rotates_from_original_returns_to_start_and_is_one_undo() {
        use crate::world::WorldBounds;
        use geometry_math::bbox::Bbox3d;
        let mut document = Document::new(WorldBounds::default());
        let original = Bbox3d::new(Vec3d::zero(), Vec3d::new(32.0, 64.0, 16.0));
        let brush =
            map_model::shapes::create_cuboid(&document.world_bounds(), &original, "stone").unwrap();
        document.add_brushes(vec![brush]);
        let id = document.selectable_brushes()[0].0;
        document.select_only(id);
        let center = original.center();
        let mut camera = crate::camera3d::Camera3d {
            position: center + Vec3d::new(0.0, 0.0, 500.0),
            ..Default::default()
        };
        camera.look_along(center - camera.position);
        let height = 800.0;
        let radius = f64::from(pref(|p| p.rotate_handle_radius))
            * handle_drag::perspective_scaling_factor(&camera, center, height);
        let from = Vec3d::new(0.8, 0.6, 0.0) * radius;
        let ray = |point| {
            let at = camera.project(point, height, height).unwrap();
            camera.ray_through(at.x(), at.y(), height, height)
        };
        let mut rotate = RotateTool::new();
        rotate.set_center(center);
        let mut handles = HandleManager::new();
        let mut sweep = crate::sweep_tool::SweepInputs::default();
        let mut drag = begin_drag(
            &camera,
            height,
            &ray(center + from),
            Tool::Rotate,
            &document,
            &mut handles,
            &rotate,
            &sweep,
            false,
            false,
            false,
        )
        .unwrap();
        let DragKind::Rotate { gesture } = &drag.what else {
            panic!("expected rotation")
        };
        let hover = RotationFeedback::new(&camera, height, height, gesture, radius, None).unwrap();
        assert_eq!(hover.ring.len(), 65);
        assert!(hover.sector.is_empty() && hover.label.is_none());
        let feedback = RotationFeedback::new(
            &camera,
            height,
            height,
            gesture,
            radius,
            Some(-std::f64::consts::FRAC_PI_2),
        )
        .unwrap();
        assert_eq!(feedback.sector.len(), 66);
        assert_eq!(feedback.label.as_ref().unwrap().0, "Z  -90.0°");
        let endpoint = center + vec::cross(gesture.axis, gesture.reference) * -radius;
        let projected = camera.project(endpoint, height, height).unwrap();
        assert!(vec::length(*feedback.sector.last().unwrap() - projected) < 1e-6);
        let mut canvas = crate::canvas::Raster::new(600, 600).unwrap();
        feedback.draw(&mut canvas);
        document.begin_transaction();
        let grid = Grid::default();
        for angle in [90.0_f64, 0.0, -90.0] {
            let radians = angle.to_radians();
            let point = center
                + from * radians.cos()
                + vec::cross(Vec3d::new(0.0, 0.0, 1.0), from) * radians.sin();
            assert!(update_drag(
                &mut drag,
                &ray(point),
                camera.direction(),
                &grid,
                &mut document,
                &mut handles,
                &mut rotate,
                &mut sweep,
                Modifiers::default()
            ));
            let bounds = document.selection_bounds().unwrap();
            let expected = if angle == 0.0 {
                original.size()
            } else {
                Vec3d::new(64.0, 32.0, 16.0)
            };
            assert!(vec::length(bounds.size() - expected) < 1e-6);
            assert!(vec::length(bounds.center() - center) < 1e-6);
        }
        document.commit_transaction();
        document.undo().unwrap();
        let restored = document.selection_bounds().unwrap();
        assert!(vec::length(restored.min - original.min) < 1e-6);
        assert!(vec::length(restored.max - original.max) < 1e-6);
    }

    #[test]
    fn rotation_changes_brush_geometry_on_every_axis_in_oblique_and_edge_on_views() {
        use crate::world::WorldBounds;
        for index in 0..3 {
            for edge_on in [false, true] {
                let mut document = Document::new(WorldBounds::default());
                let original = Bbox3d::new(Vec3d::zero(), Vec3d::new(32.0, 64.0, 16.0));
                let brush =
                    map_model::shapes::create_cuboid(&document.world_bounds(), &original, "stone")
                        .unwrap();
                document.add_brushes(vec![brush]);
                document.select_only(document.selectable_brushes()[0].0);
                let center = original.center();
                let mut camera = Camera3d::default();
                let mut offset = Vec3d::new(-400.0, -450.0, 300.0);
                if edge_on {
                    offset[index] = 0.0;
                }
                camera.position = center + offset;
                camera.look_along(-offset);
                let height = 800.0;
                let radius = f64::from(pref(|p| p.rotate_handle_radius))
                    * camera.world_units_per_pixel(center, height);
                let mut axis = Vec3d::zero();
                axis[index] = 1.0;
                let mut rotate = RotateTool::new();
                rotate.set_center(center);
                let mut handles = HandleManager::new();
                let mut sweep = crate::sweep_tool::SweepInputs::default();
                let mut successful = 0;
                for step in 0..64 {
                    let angle = (step as f64 + 0.5) / 64.0 * std::f64::consts::TAU;
                    let mut from = Vec3d::zero();
                    from[(index + 1) % 3] = radius * angle.cos();
                    from[(index + 2) % 3] = radius * angle.sin();
                    let screen = camera.project(center + from, height, height).unwrap();
                    let ray = camera.ray_through(screen.x(), screen.y(), height, height);
                    let Some(mut drag) = begin_drag(
                        &camera,
                        height,
                        &ray,
                        Tool::Rotate,
                        &document,
                        &mut handles,
                        &rotate,
                        &sweep,
                        false,
                        false,
                        false,
                    ) else {
                        continue;
                    };
                    let DragKind::Rotate { gesture } = drag.what else {
                        unreachable!()
                    };
                    if gesture.axis != axis {
                        continue;
                    }
                    let tangent = vec::cross(axis, from);
                    let projected = camera
                        .project(center + from + tangent * 0.01, height, height)
                        .unwrap();
                    let direction = projected - screen;
                    if vec::length(direction) < 0.01 {
                        continue;
                    }
                    let to = screen + direction / vec::length(direction) * 30.0;
                    let ray = camera.ray_through(to.x(), to.y(), height, height);
                    document.begin_transaction();
                    assert!(
                        update_drag(
                            &mut drag,
                            &ray,
                            camera.direction(),
                            &Grid::default(),
                            &mut document,
                            &mut handles,
                            &mut rotate,
                            &mut sweep,
                            Modifiers::default()
                        ),
                        "axis {index}, edge_on {edge_on}, step {step}"
                    );
                    let changed = document.selection_bounds().unwrap();
                    assert!(
                        vec::length(changed.size() - original.size()) > 0.1,
                        "unchanged axis {index}, edge_on {edge_on}, step {step}, angle {}",
                        drag.applied.x()
                    );
                    document.rollback_transaction();
                    document.commit_transaction();
                    successful += 1;
                }
                assert!(
                    successful > 10,
                    "axis {index}, edge_on {edge_on}: only {successful} usable ring locations"
                );
            }
        }
    }

    #[test]
    fn foreground_brush_does_not_intercept_transform_handles() {
        let mut document = Document::new(crate::world::WorldBounds::default());
        let bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(32.0, 64.0, 16.0));
        document.add_brushes(vec![map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &bounds,
            "stone",
        )
        .unwrap()]);
        let selected = document.selectable_brushes()[0].0;
        document.select_only(selected);
        let center = bounds.center();
        let mut camera = Camera3d::default();
        let offset = Vec3d::new(-400.0, -450.0, 300.0);
        camera.position = center + offset;
        camera.look_along(-offset);
        let mut rotate = RotateTool::new();
        rotate.set_center(center);
        let sweep = crate::sweep_tool::SweepInputs::default();
        let sample = |document: &Document, tool| {
            let mut handles = HandleManager::new();
            (280..=520)
                .step_by(4)
                .flat_map(|x| (280..=520).step_by(4).map(move |y| (x, y)))
                .map(|(x, y)| {
                    let ray = camera.ray_through(x as f64, y as f64, 800.0, 800.0);
                    begin_drag(
                        &camera,
                        800.0,
                        &ray,
                        tool,
                        document,
                        &mut handles,
                        &rotate,
                        &sweep,
                        false,
                        false,
                        false,
                    )
                    .is_some()
                })
                .collect::<Vec<_>>()
        };
        let tools = [Tool::Rotate, Tool::Scale, Tool::Shear];
        let before = tools.map(|tool| sample(&document, tool));
        let occluder_center = center + offset * 0.5;
        let extent = Vec3d::new(128.0, 128.0, 128.0);
        document.add_brushes(vec![map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::new(occluder_center - extent, occluder_center + extent),
            "stone",
        )
        .unwrap()]);
        document.select_only(selected);
        assert_ne!(
            document
                .pick(&camera.ray_through(400.0, 400.0, 800.0, 800.0))
                .unwrap()
                .node,
            selected
        );
        for (tool, expected) in tools.into_iter().zip(before) {
            assert!(
                expected.iter().any(|hit| *hit),
                "{tool:?} must have reachable handles"
            );
            assert_eq!(
                sample(&document, tool),
                expected,
                "{tool:?} was intercepted"
            );
        }
    }

    #[test]
    fn fillet_surface_preview_and_overlay_share_the_pinned_result() {
        let document = Rc::new(RefCell::new(Document::new(
            crate::world::WorldBounds::default(),
        )));
        let brush = map_model::shapes::create_cuboid(
            &document.borrow().world_bounds(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .unwrap();
        document.borrow_mut().add_brushes(vec![brush]);
        document.borrow_mut().select_all();
        let tool = Rc::new(RefCell::new(Tool::Fillet));
        let nothing: Rc<dyn Fn()> = Rc::new(|| {});
        let scene = Scene3d::new(Scene3dParts {
            document: Rc::clone(&document),
            tool: Rc::clone(&tool),
            handles: Default::default(),
            rotate: Default::default(),
            camera: Default::default(),
            fillet: Default::default(),
            pending_shape: Default::default(),
            shape_parameters: Default::default(),
            clip: Default::default(),
            assemble: Default::default(),
            grid: Default::default(),
            filters: Default::default(),
            sweep: Default::default(),
            paste_target: Default::default(),
            refresh_panels: Rc::clone(&nothing),
            redraw: Rc::clone(&nothing),
            render: Rc::clone(&nothing),
            focus: nothing,
        });
        let original = document.borrow().to_map_text();
        assert!(scene
            .fillet_surface_preview(&document.borrow(), 800.0, 600.0)
            .is_none());
        let edge = scene.handles.borrow().handles()[0].position;
        scene.handles.borrow_mut().select(edge, false);
        let shown = scene
            .fillet_surface_preview(&document.borrow(), 800.0, 600.0)
            .unwrap();
        let overlay = scene.build_overlay(&document.borrow(), 800.0, 600.0, false);
        assert!(!overlay.guides.is_empty());
        let id = document.borrow().selectable_brushes()[0].0;
        let rounded_edges = shown.brushes[0].1[0].edge_positions();
        let original_edges = document.borrow().brush(id).unwrap().edge_positions();
        let stale: Vec<_> = original_edges
            .into_iter()
            .filter(|edge| !rounded_edges.contains(edge))
            .collect();
        assert!(
            stale.iter().any(
                |(a, b)| !overlay
                    .guides
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .any(|line| (line[0].0 == *a && line[1].0 == *b)
                        || (line[0].0 == *b && line[1].0 == *a))
            ),
            "old silhouette must not obscure the preview"
        );
        let again = scene
            .fillet_surface_preview(&document.borrow(), 800.0, 600.0)
            .unwrap();
        assert!(Rc::ptr_eq(&shown, &again));
        *tool.borrow_mut() = Tool::Select;
        assert!(scene
            .fillet_surface_preview(&document.borrow(), 800.0, 600.0)
            .is_none());
        assert_eq!(document.borrow().to_map_text(), original);
    }

    #[test]
    fn fillet_can_pick_and_extend_brush_selection_without_leaving_the_tool() {
        let mut d = Document::new(crate::world::WorldBounds::default());
        for x in [-96.0, 32.0] {
            d.add_brushes(vec![map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &Bbox3d::new(Vec3d::new(x, -32.0, 0.0), Vec3d::new(x + 64.0, 32.0, 64.0)),
                "stone",
            )
            .unwrap()]);
        }
        d.clear_selection();
        let before = d.to_map_text();
        let document = Rc::new(RefCell::new(d));
        let tool = Rc::new(RefCell::new(Tool::Fillet));
        let nothing: Rc<dyn Fn()> = Rc::new(|| {});
        let scene = Scene3d::new(Scene3dParts {
            document: document.clone(),
            tool: tool.clone(),
            handles: Default::default(),
            rotate: Default::default(),
            camera: Default::default(),
            fillet: Default::default(),
            pending_shape: Default::default(),
            shape_parameters: Default::default(),
            clip: Default::default(),
            assemble: Default::default(),
            grid: Default::default(),
            filters: Default::default(),
            sweep: Default::default(),
            paste_target: Default::default(),
            refresh_panels: nothing.clone(),
            redraw: nothing.clone(),
            render: nothing.clone(),
            focus: nothing,
        });
        {
            let mut camera = scene.camera.borrow_mut();
            camera.position = Vec3d::new(0.0, 0.0, 500.0);
            camera.look_along(Vec3d::new(0.0, 0.0, -1.0));
        }
        for (x, ctrl, count) in [
            (-64.0, false, 1),
            (64.0, true, 2),
            (64.0, true, 1),
            (64.0, false, 1),
        ] {
            let at = scene
                .camera
                .borrow()
                .project(Vec3d::new(x, 0.0, 64.0), 800.0, 600.0)
                .unwrap();
            let mods = Modifiers {
                control: ctrl,
                ..Default::default()
            };
            scene.drag_begin(at[0], at[1], mods, (800.0, 600.0));
            scene.click(1, at[0], at[1], mods, (800.0, 600.0));
            scene.drag_end(0.0, 0.0, mods, (800.0, 600.0));
            assert_eq!(document.borrow().selection_count(), count);
            assert_eq!(*tool.borrow(), Tool::Fillet);
        }
        scene.drag_begin(-1000.0, -1000.0, Modifiers::default(), (800.0, 600.0));
        assert!(!document.borrow().has_selection());
        assert_eq!(document.borrow().to_map_text(), before);
    }

    /// A floor and a cube on it, in a Select-tool scene seen by the default camera.
    fn floor_and_cube_scene() -> (
        Scene3d,
        Rc<RefCell<Document>>,
        Rc<RefCell<Tool>>,
        map_model::node::NodeId,
        map_model::node::NodeId,
    ) {
        let document = Rc::new(RefCell::new(Document::new(
            crate::world::WorldBounds::default(),
        )));
        let mut ids = Vec::new();
        for (min, max) in [
            (
                Vec3d::new(-512.0, -512.0, -16.0),
                Vec3d::new(512.0, 512.0, 0.0),
            ),
            (Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(64.0, 64.0, 64.0)),
        ] {
            let world = document.borrow().world_bounds();
            let brush =
                map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "stone").unwrap();
            document.borrow_mut().add_brushes(vec![brush]);
            ids.push(*document.borrow().selection_ids().iter().next().unwrap());
            document.borrow_mut().clear_selection();
        }
        let tool = Rc::new(RefCell::new(Tool::Select));
        let nothing: Rc<dyn Fn()> = Rc::new(|| {});
        let scene = Scene3d::new(Scene3dParts {
            document: Rc::clone(&document),
            tool: Rc::clone(&tool),
            handles: Default::default(),
            rotate: Default::default(),
            camera: Default::default(),
            fillet: Default::default(),
            pending_shape: Default::default(),
            shape_parameters: Default::default(),
            clip: Default::default(),
            assemble: Default::default(),
            grid: Default::default(),
            filters: Default::default(),
            sweep: Default::default(),
            paste_target: Default::default(),
            refresh_panels: Rc::clone(&nothing),
            redraw: Rc::clone(&nothing),
            render: Rc::clone(&nothing),
            focus: nothing,
        });
        (scene, document, tool, ids[0], ids[1])
    }

    /// Press, move and release as the frontend sends them: the click goes with the press.
    fn press_drag(scene: &Scene3d, at: Vec3d, by: (f64, f64)) {
        let size = (800.0, 600.0);
        let on_screen = scene.camera.borrow().project(at, size.0, size.1).unwrap();
        let (x, y) = (on_screen.x(), on_screen.y());
        let mods = Modifiers::default();
        scene.drag_begin(x, y, mods, size);
        scene.click(1, x, y, mods, size);
        for step in 1..=4 {
            let t = f64::from(step) / 4.0;
            scene.drag_update(by.0 * t, by.1 * t, mods, size);
        }
        scene.drag_end(by.0, by.1, mods, size);
    }

    /// `MoveObjectsToolController` moves only what is selected; a drag over the floor
    /// draws a brush on it rather than dragging the floor away, and a click selects.
    #[test]
    fn a_drag_over_an_unselected_brush_draws_and_a_click_selects() {
        let (scene, document, _, floor, _) = floor_and_cube_scene();
        let before = document.borrow().brush(floor).unwrap().bounds();
        press_drag(&scene, Vec3d::new(-150.0, -60.0, 0.0), (60.0, 30.0));
        assert_eq!(
            document.borrow().brush(floor).unwrap().bounds(),
            before,
            "the floor moved"
        );
        assert!(document.borrow().has_shape_preview(), "nothing was drawn");

        let (scene, document, _, floor, _) = floor_and_cube_scene();
        press_drag(&scene, Vec3d::new(-150.0, -60.0, 0.0), (0.0, 0.0));
        assert!(!document.borrow().has_shape_preview());
        assert_eq!(
            document
                .borrow()
                .selection_ids()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![floor]
        );
    }

    /// A drag on the selected cube moves it.
    #[test]
    fn a_drag_on_a_selected_brush_moves_it_in_3d() {
        let (scene, document, _, _, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        press_drag(&scene, Vec3d::new(32.0, 32.0, 64.0), (80.0, 0.0));
        let bounds = document.borrow().brush(cube).unwrap().bounds();
        assert_ne!(bounds.min, Vec3d::zero(), "the cube did not move");
        assert!(!document.borrow().has_shape_preview());
    }

    /// Press, move and release with modifiers held throughout.
    fn press_drag_with(scene: &Scene3d, at: Vec3d, by: (f64, f64), mods: Modifiers) {
        let size = (800.0, 600.0);
        let on_screen = scene.camera.borrow().project(at, size.0, size.1).unwrap();
        let (x, y) = (on_screen.x(), on_screen.y());
        scene.drag_begin(x, y, mods, size);
        scene.click(1, x, y, mods, size);
        for step in 1..=4 {
            let t = f64::from(step) / 4.0;
            scene.drag_update(by.0 * t, by.1 * t, mods, size);
        }
        scene.drag_end(by.0, by.1, mods, size);
    }

    /// A Ctrl-drag on the selected cube moves a copy, undone in one step; a Ctrl-click on
    /// the floor adds it to the selection on release.
    #[test]
    fn ctrl_drag_copies_and_ctrl_click_toggles_in_3d() {
        let (scene, document, _, floor, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        let before = document.borrow().to_map_text();
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        press_drag_with(&scene, Vec3d::new(32.0, 32.0, 64.0), (80.0, 0.0), ctrl);
        assert_eq!(
            document.borrow().brush(cube).unwrap().bounds().min,
            Vec3d::zero()
        );
        assert_eq!(
            map_model::node::brushes(document.borrow().tree_world()).len(),
            3
        );
        document.borrow_mut().undo();
        assert_eq!(document.borrow().to_map_text(), before);

        document.borrow_mut().select_only(cube);
        press_drag_with(&scene, Vec3d::new(-150.0, -60.0, 0.0), (0.0, 0.0), ctrl);
        assert!(document.borrow().is_selected(floor) && document.borrow().is_selected(cube));
    }

    /// `PaintSelectionDragTracker`: a Ctrl-drag from the unselected floor onto the cube
    /// adds both, and Esc in the middle of one puts the selection back.
    #[test]
    fn a_ctrl_drag_paints_the_selection_in_3d() {
        let (scene, document, _, floor, cube) = floor_and_cube_scene();
        let size = (800.0, 600.0);
        let project = |at: Vec3d| scene.camera.borrow().project(at, size.0, size.1).unwrap();
        let from = project(Vec3d::new(-150.0, -60.0, 0.0));
        let to = project(Vec3d::new(32.0, 32.0, 64.0));
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        press_drag_with(
            &scene,
            Vec3d::new(-150.0, -60.0, 0.0),
            (to.x() - from.x(), to.y() - from.y()),
            ctrl,
        );
        assert!(document.borrow().is_selected(floor) && document.borrow().is_selected(cube));
        assert!(!document.borrow().has_shape_preview());

        document.borrow_mut().clear_selection();
        scene.drag_begin(from.x(), from.y(), ctrl, size);
        scene.click(1, from.x(), from.y(), ctrl, size);
        scene.drag_update(to.x() - from.x(), to.y() - from.y(), ctrl, size);
        assert!(document.borrow().is_selected(cube));
        assert!(scene.escape());
        assert!(!document.borrow().has_selection());
    }

    /// `NodeHandleToolMovePartBase::shouldStartMove`: the edge tool does not move a handle
    /// with Ctrl held, so a Ctrl-drag from one paints the selection, and a Ctrl-click
    /// picks or unpicks it.
    #[test]
    fn a_ctrl_drag_from_an_edge_handle_paints_the_selection() {
        let (scene, document, tool, floor, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        *tool.borrow_mut() = Tool::Edge;
        scene.handles.borrow_mut().collect(
            HandleKind::Edge,
            document.borrow().selected_brushes().into_iter(),
        );
        let before = document.borrow().to_map_text();
        let size = (800.0, 600.0);
        let project = |at: Vec3d| scene.camera.borrow().project(at, size.0, size.1).unwrap();
        let edge = Vec3d::new(32.0, 0.0, 64.0);
        let from = project(edge);
        let to = project(Vec3d::new(-150.0, -60.0, 0.0));
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        press_drag_with(&scene, edge, (to.x() - from.x(), to.y() - from.y()), ctrl);
        assert!(document.borrow().is_selected(floor) && document.borrow().is_selected(cube));
        assert!(!scene.handles.borrow().is_selected(edge));
        document.borrow_mut().select_only(cube);
        assert_eq!(document.borrow().to_map_text(), before);

        press_drag_with(&scene, edge, (0.0, 0.0), ctrl);
        assert!(scene.handles.borrow().is_selected(edge));
        press_drag_with(&scene, edge, (0.0, 0.0), ctrl);
        assert!(!scene.handles.borrow().is_selected(edge));
    }

    /// `AssembleBrushToolController3D`: a drag on a face draws a rectangle of four points
    /// on it, snapped outwards to the grid, and a Shift-drag from that rectangle pulls a
    /// copy of it up into a box. A click adds one point.
    #[test]
    fn assemble_drags_draw_a_face_and_pull_it_into_a_solid() {
        let (scene, document, tool, _, _) = floor_and_cube_scene();
        *tool.borrow_mut() = Tool::Assemble;
        let size = (800.0, 600.0);
        let project = |at: Vec3d| scene.camera.borrow().project(at, size.0, size.1).unwrap();
        let from = project(Vec3d::new(-150.0, -60.0, 0.0));
        let to = project(Vec3d::new(-100.0, -20.0, 0.0));
        press_drag(
            &scene,
            Vec3d::new(-150.0, -60.0, 0.0),
            (to.x() - from.x(), to.y() - from.y()),
        );
        let mut points = scene.assemble.borrow().points().to_vec();
        points.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));
        assert_eq!(
            points,
            vec![
                Vec3d::new(-160.0, -64.0, 0.0),
                Vec3d::new(-160.0, -16.0, 0.0),
                Vec3d::new(-96.0, -64.0, 0.0),
                Vec3d::new(-96.0, -16.0, 0.0),
            ]
        );
        assert!(scene.assemble.borrow().polygon().is_some());
        assert!(!document.borrow().has_selection());

        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        press_drag_with(&scene, Vec3d::new(-128.0, -40.0, 0.0), (0.0, -120.0), shift);
        let hull = scene.assemble.borrow().hull().expect("a box");
        assert_eq!(hull.vertex_count(), 8);
        assert_eq!(hull.bounds().min.z(), 0.0);
        assert!(hull.bounds().max.z() > 0.0);
        assert_eq!(hull.bounds().max.z() % 16.0, 0.0);

        scene.assemble.borrow_mut().reset();
        press_drag(&scene, Vec3d::new(-150.0, -60.0, 0.0), (0.0, 0.0));
        assert_eq!(
            scene.assemble.borrow().points(),
            &[Vec3d::new(-144.0, -64.0, 0.0)]
        );
    }

    /// `AssembleBrushToolController3D::mouseDoubleClick`: a double click adds the corners
    /// of the face under it.
    #[test]
    fn an_assemble_double_click_adds_the_corners_of_a_face() {
        let (scene, _, tool, _, _) = floor_and_cube_scene();
        *tool.borrow_mut() = Tool::Assemble;
        let size = (800.0, 600.0);
        let at = scene
            .camera
            .borrow()
            .project(Vec3d::new(32.0, 32.0, 64.0), size.0, size.1)
            .unwrap();
        assert!(scene.double_click(at.x(), at.y(), Modifiers::default(), size));
        let mut points = scene.assemble.borrow().points().to_vec();
        points.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));
        assert_eq!(
            points,
            vec![
                Vec3d::new(0.0, 0.0, 64.0),
                Vec3d::new(0.0, 64.0, 64.0),
                Vec3d::new(64.0, 0.0, 64.0),
                Vec3d::new(64.0, 64.0, 64.0),
            ]
        );
    }

    /// Esc during an assemble drag puts the points back.
    #[test]
    fn escape_takes_back_an_assemble_drag() {
        let (scene, _, tool, _, _) = floor_and_cube_scene();
        *tool.borrow_mut() = Tool::Assemble;
        scene
            .assemble
            .borrow_mut()
            .add_point(Vec3d::new(0.0, 0.0, 128.0));
        let size = (800.0, 600.0);
        let at = scene
            .camera
            .borrow()
            .project(Vec3d::new(-150.0, -60.0, 0.0), size.0, size.1)
            .unwrap();
        let mods = Modifiers::default();
        scene.drag_begin(at.x(), at.y(), mods, size);
        scene.click(1, at.x(), at.y(), mods, size);
        scene.drag_update(40.0, 20.0, mods, size);
        assert_eq!(scene.assemble.borrow().points().len(), 5);
        assert!(scene.escape());
        assert_eq!(
            scene.assemble.borrow().points(),
            &[Vec3d::new(0.0, 0.0, 128.0)]
        );
    }

    /// Alt at the press lifts a move off the ground.
    #[test]
    fn alt_at_the_press_moves_up() {
        let (scene, document, _, _, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        press_drag_with(&scene, Vec3d::new(32.0, 32.0, 32.0), (0.0, -80.0), alt);
        let bounds = document.borrow().brush(cube).unwrap().bounds();
        assert!(bounds.min.z() > 0.0, "the cube did not go up: {bounds:?}");
        assert_eq!((bounds.min.x(), bounds.min.y()), (0.0, 0.0));
    }

    /// With something selected, a click on something else selects it on release, and a
    /// drag from it does nothing.
    #[test]
    fn with_a_selection_a_3d_press_elsewhere_is_a_click() {
        let (scene, document, _, floor, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        let floor_bounds = document.borrow().brush(floor).unwrap().bounds();
        press_drag(&scene, Vec3d::new(-150.0, -60.0, 0.0), (60.0, 30.0));
        assert!(document.borrow().is_selected(cube));
        assert_eq!(
            document.borrow().brush(floor).unwrap().bounds(),
            floor_bounds
        );
        press_drag(&scene, Vec3d::new(-150.0, -60.0, 0.0), (0.0, 0.0));
        assert_eq!(
            document
                .borrow()
                .selection_ids()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![floor]
        );
    }

    /// A press that misses the Scale, Shear or Rotate tool's handles is a selection
    /// click, made on release.
    #[test]
    fn a_3d_press_off_a_tools_handles_selects_on_release() {
        for active in [Tool::Scale, Tool::Shear, Tool::Rotate] {
            let (scene, document, tool, floor, cube) = floor_and_cube_scene();
            document.borrow_mut().select_only(cube);
            *tool.borrow_mut() = active;
            press_drag(&scene, Vec3d::new(-200.0, -100.0, 0.0), (0.0, 0.0));
            assert_eq!(
                document
                    .borrow()
                    .selection_ids()
                    .iter()
                    .copied()
                    .collect::<Vec<_>>(),
                vec![floor],
                "{active:?}"
            );
        }
    }

    /// `FlyModeHelper`: the fly keys move the camera by time, at the speed preference in
    /// units a millisecond, twice as fast with Shift and half with Alt.
    #[test]
    fn held_fly_keys_move_by_time() {
        let (scene, ..) = floor_and_cube_scene();
        let speed = f64::from(pref(|p| p.camera_fly_move_speed));
        let start = scene.camera.borrow().position;
        let forward = scene.camera.borrow().direction();
        scene.fly_held(1.0, 0.0, 0.0, Modifiers::default(), 0.1);
        let moved = scene.camera.borrow().position - start;
        assert!(vec::length(moved - forward * speed * 100.0) < 1e-6);
        let start = scene.camera.borrow().position;
        let fast = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        scene.fly_held(0.0, 0.0, 1.0, fast, 0.1);
        let moved = scene.camera.borrow().position - start;
        assert!((moved.z() - speed * 200.0).abs() < 1e-6);
        assert!(scene.key(Key::Letter('w'), Modifiers::default()));
    }

    /// `DrawShapeToolController3D::makeBounds`: Shift squares the base, Shift+Alt makes a
    /// cube, and the box stays inside the world.
    #[test]
    fn shift_and_alt_shape_a_3d_draw() {
        let grid = Grid::new(4);
        let down = Vec3d::new(0.3, 0.3, -1.0);
        let world = Document::new(crate::world::WorldBounds::default()).world_bounds();
        let start = Vec3d::new(0.0, 0.0, 0.0);
        let corner = Vec3d::new(64.0, 32.0, 0.0);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let square = shaped_draw_bounds(start, corner, &grid, down, shift, &world).unwrap();
        assert_eq!(square.max - square.min, Vec3d::new(64.0, 64.0, 16.0));
        let cube_mods = Modifiers {
            shift: true,
            alt: true,
            ..Modifiers::default()
        };
        let cube = shaped_draw_bounds(start, corner, &grid, down, cube_mods, &world).unwrap();
        assert_eq!(cube.max - cube.min, Vec3d::new(64.0, 64.0, 64.0));
        let far = Vec3d::new(world.max.x() + 512.0, 32.0, 0.0);
        let clipped =
            shaped_draw_bounds(start, far, &grid, down, Modifiers::default(), &world).unwrap();
        assert!(clipped.max.x() <= world.max.x());
    }

    /// Alt during a draw drag sets the height: the far corner goes straight up.
    #[test]
    fn alt_during_a_3d_draw_sets_the_height() {
        let (scene, document, ..) = floor_and_cube_scene();
        document.borrow_mut().clear_selection();
        let size = (800.0, 600.0);
        let at = scene
            .camera
            .borrow()
            .project(Vec3d::new(-150.0, -60.0, 0.0), size.0, size.1)
            .unwrap();
        scene.drag_begin(at.x(), at.y(), Modifiers::default(), size);
        scene.drag_update(60.0, 30.0, Modifiers::default(), size);
        let flat = scene.pending_shape.borrow().unwrap();
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        scene.drag_update(60.0, -60.0, alt, size);
        let tall = scene.pending_shape.borrow().unwrap();
        assert!(tall.max.z() > flat.max.z(), "{flat:?} -> {tall:?}");
        assert_eq!((tall.min.x(), tall.max.x()), (flat.min.x(), flat.max.x()));
        scene.drag_cancel();
    }

    /// The rotate tool's centre is a handle in 3D too: dragging it moves the centre.
    #[test]
    fn the_rotation_centre_drags_in_3d() {
        let (scene, document, tool, _, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        *tool.borrow_mut() = Tool::Rotate;
        let center = Vec3d::new(32.0, 32.0, 32.0);
        scene.rotate.borrow_mut().set_center(center);
        press_drag(&scene, center, (80.0, 0.0));
        let moved = scene.rotate.borrow().center().unwrap();
        assert_ne!(moved, center);
        assert_eq!(
            document.borrow().brush(cube).unwrap().bounds().min,
            Vec3d::zero()
        );
    }

    /// `SetBrushFaceAttributesTool`: an Alt double click copies the selected face onto
    /// the whole brush, and the first click's single-face copy undoes with it.
    #[test]
    fn an_alt_double_click_copy_undoes_in_one_step() {
        let (scene, document, _, floor, cube) = floor_and_cube_scene();
        document.borrow_mut().paint_brush(cube, "crate");
        let top = {
            let document = document.borrow();
            let brush = document.brush(floor).unwrap();
            (0..brush.faces().len())
                .find(|i| brush.faces()[*i].normal().z() > 0.9)
                .unwrap()
        };
        document.borrow_mut().select_face_only(FaceRef {
            node: floor,
            face: top,
        });
        let before = document.borrow().to_map_text();
        let size = (800.0, 600.0);
        let at = scene
            .camera
            .borrow()
            .project(Vec3d::new(32.0, 32.0, 64.0), size.0, size.1)
            .unwrap();
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        for presses in [1, 2] {
            scene.drag_begin(at.x(), at.y(), alt, size);
            scene.stroke_begin(at.x(), at.y(), alt, size);
            scene.click(presses, at.x(), at.y(), alt, size);
            scene.drag_end(0.0, 0.0, alt, size);
            scene.stroke_end();
        }
        let copied = document.borrow().to_map_text();
        assert_ne!(copied, before);
        assert!(
            !copied.contains("crate"),
            "every face of the cube took the floor's material"
        );
        document.borrow_mut().undo();
        assert_eq!(document.borrow().to_map_text(), before);
    }

    #[test]
    fn alt_transfer_is_live_and_undoes_or_cancels_as_one_stroke() {
        for cancel in [false, true] {
            let (scene, document, _, floor, cube) = floor_and_cube_scene();
            document.borrow_mut().paint_brush(cube, "crate");
            let source = {
                let doc = document.borrow();
                let brush = doc.brush(floor).unwrap();
                FaceRef {
                    node: floor,
                    face: (0..brush.faces().len())
                        .find(|i| brush.faces()[*i].normal().z() > 0.9)
                        .unwrap(),
                }
            };
            document.borrow_mut().select_face_only(source);
            let before = document.borrow().to_map_text();
            let mark = document.borrow().edit_mark();
            let size = (800.0, 600.0);
            let mut targets = Vec::new();
            for y in (0..600).step_by(8) {
                for x in (0..800).step_by(8) {
                    if let Some(face) = face_under(
                        &scene.camera.borrow(),
                        size,
                        &document.borrow(),
                        x as f64,
                        y as f64,
                    ) {
                        if face.node == cube && !targets.iter().any(|(f, _)| *f == face) {
                            targets.push((face, (x as f64, y as f64)));
                        }
                    }
                }
            }
            assert!(targets.len() >= 2);
            let alt = Modifiers {
                alt: true,
                ..Modifiers::default()
            };
            let start = targets[0].1;
            scene.stroke_begin(start.0, start.1, alt, size);
            assert_eq!(
                document
                    .borrow()
                    .face_at(targets[0].0)
                    .unwrap()
                    .material_name,
                "stone"
            );
            for (face, at) in &targets[1..] {
                scene.stroke_update(start, at.0 - start.0, at.1 - start.1, size);
                assert_eq!(
                    document.borrow().face_at(*face).unwrap().material_name,
                    "stone"
                );
            }
            assert_eq!(document.borrow().edit_mark(), mark + 1);
            if cancel {
                scene.stroke_cancel();
            } else {
                scene.stroke_end();
                document.borrow_mut().undo();
            }
            assert_eq!(document.borrow().to_map_text(), before);
        }
    }

    /// Esc in the middle of a 3D move undoes it and ends the drag.
    #[test]
    fn escape_ends_a_3d_move() {
        let (scene, document, _, _, cube) = floor_and_cube_scene();
        document.borrow_mut().select_only(cube);
        let size = (800.0, 600.0);
        let p = scene
            .camera
            .borrow()
            .project(Vec3d::new(32.0, 32.0, 64.0), size.0, size.1)
            .unwrap();
        scene.drag_begin(p.x(), p.y(), Modifiers::default(), size);
        scene.drag_update(80.0, 0.0, Modifiers::default(), size);
        assert!(scene.escape());
        scene.drag_update(160.0, 0.0, Modifiers::default(), size);
        scene.drag_end(160.0, 0.0, Modifiers::default(), size);
        assert_eq!(
            document.borrow().brush(cube).unwrap().bounds().min,
            Vec3d::zero()
        );
        assert!(!document.borrow().in_transaction());
    }

    #[test]
    fn transform_handle_press_does_not_clear_selection_before_drag_begins() {
        let document = Rc::new(RefCell::new(Document::new(
            crate::world::WorldBounds::default(),
        )));
        let bounds = Bbox3d::new(Vec3d::zero(), Vec3d::new(32.0, 64.0, 16.0));
        let brush =
            map_model::shapes::create_cuboid(&document.borrow().world_bounds(), &bounds, "stone")
                .unwrap();
        document.borrow_mut().add_brushes(vec![brush]);
        let id = document.borrow().selectable_brushes()[0].0;
        document.borrow_mut().select_only(id);
        let tool = Rc::new(RefCell::new(Tool::Rotate));
        let nothing: Rc<dyn Fn()> = Rc::new(|| {});
        let scene = Scene3d::new(Scene3dParts {
            document: Rc::clone(&document),
            tool: Rc::clone(&tool),
            handles: Default::default(),
            rotate: Default::default(),
            camera: Default::default(),
            fillet: Default::default(),
            pending_shape: Default::default(),
            shape_parameters: Default::default(),
            clip: Default::default(),
            assemble: Default::default(),
            grid: Default::default(),
            filters: Default::default(),
            sweep: Default::default(),
            paste_target: Default::default(),
            refresh_panels: Rc::clone(&nothing),
            redraw: Rc::clone(&nothing),
            render: Rc::clone(&nothing),
            focus: nothing,
        });
        // A frontend may dispatch the click before the drag. A ring extends beyond the
        // brush silhouette, so its press must not become an empty-space selection
        // click, even before the drag has populated its shared state.
        for active in [Tool::Rotate, Tool::Scale, Tool::Shear] {
            *tool.borrow_mut() = active;
            scene.click(1, -1000.0, -1000.0, Modifiers::default(), (640.0, 480.0));
            assert!(
                document.borrow().is_selected(id),
                "{active:?} press cleared the brush before the drag could transform it"
            );
        }
        *tool.borrow_mut() = Tool::Select;
        let before = document.borrow().to_map_text();
        let on_screen = scene
            .camera
            .borrow()
            .project(Vec3d::new(32.0, 32.0, 8.0), 640.0, 480.0)
            .expect("visible cube");
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        // A Shift drag on the selected brush resizes it and keeps it selected: the click
        // the press also sends must not trade the brush for a face first.
        scene.drag_begin(on_screen[0], on_screen[1], shift, (640.0, 480.0));
        scene.click(1, on_screen[0], on_screen[1], shift, (640.0, 480.0));
        assert!(
            document.borrow().is_selected(id),
            "the press keeps the brush"
        );
        // Along the face's normal on screen, two grid steps out.
        let out = scene
            .camera
            .borrow()
            .project(Vec3d::new(64.0, 32.0, 8.0), 640.0, 480.0)
            .expect("visible");
        let (dx, dy) = (out[0] - on_screen[0], out[1] - on_screen[1]);
        scene.drag_update(dx, dy, shift, (640.0, 480.0));
        scene.drag_end(dx, dy, shift, (640.0, 480.0));
        assert_ne!(
            document.borrow().to_map_text(),
            before,
            "the drag resized it"
        );
        assert!(
            document.borrow().is_selected(id),
            "and it is still selected"
        );
        assert!(document.borrow_mut().undo().is_some());
        // Let go where it was pressed, it is a Shift-click, which selects the face.
        scene.drag_begin(on_screen[0], on_screen[1], shift, (640.0, 480.0));
        scene.click(1, on_screen[0], on_screen[1], shift, (640.0, 480.0));
        scene.drag_end(0.0, 0.0, shift, (640.0, 480.0));
        assert_eq!(
            document.borrow().face_selection_count(),
            1,
            "a Shift-click without a drag selects the face"
        );
        assert_eq!(
            document.borrow().to_map_text(),
            before,
            "click without drag must not extrude"
        );
        *tool.borrow_mut() = Tool::Select;
        scene.click(1, -1000.0, -1000.0, Modifiers::default(), (640.0, 480.0));
        assert!(
            !document.borrow().has_selection(),
            "ordinary selection clicks must still clear the selection"
        );
    }
}

#[cfg(test)]
mod kept_frame_tests {
    //! A frontend that keeps the 3D view's framebuffer shows the last picture again until
    //! something it shows changes; each of these is one such change.
    use super::*;

    fn key_of(
        camera: &Camera3d,
        document: (u64, u64),
        filters: &ViewFilters,
        size: (i32, i32),
        points: &[(Vec3d, [f32; 4])],
    ) -> u64 {
        frame_key(camera, document, filters, size.0, size.1, points, &[])
    }

    #[test]
    fn every_change_the_view_shows_moves_the_picture() {
        let camera = Camera3d::default();
        let filters = ViewFilters::default();
        let point = [(Vec3d::new(1.0, 2.0, 3.0), [1.0, 1.0, 1.0, 1.0])];
        let settings = crate::render_settings::from_preferences();
        let base = picture_key(
            key_of(&camera, (1, 7), &filters, (640, 480), &point),
            &settings,
            3,
        );
        assert_eq!(
            base,
            picture_key(
                key_of(&camera, (1, 7), &filters, (640, 480), &point),
                &settings,
                3
            ),
            "the same scene is the same picture"
        );

        let mut moved = camera;
        moved.position = Vec3d::new(5.0, 0.0, 0.0);
        let mut turned = camera;
        turned.yaw += 0.1;
        let mut hidden = filters;
        hidden.show_brushes = false;
        let mut frame = filters;
        frame.animation_frame = Some(4);
        let hover = [(Vec3d::new(1.0, 2.0, 3.0), [1.0, 0.0, 0.0, 1.0])];
        let mut background = settings.clone();
        background.background_color[0] += 0.5;
        let mut no_accumulation = settings.clone();
        no_accumulation.accumulate = !no_accumulation.accumulate;

        let changed = [
            (
                "camera moved",
                picture_key(
                    key_of(&moved, (1, 7), &filters, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "camera turned",
                picture_key(
                    key_of(&turned, (1, 7), &filters, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "edit or selection",
                picture_key(
                    key_of(&camera, (1, 8), &filters, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "another map",
                picture_key(
                    key_of(&camera, (2, 7), &filters, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "filter",
                picture_key(
                    key_of(&camera, (1, 7), &hidden, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "timeline frame",
                picture_key(
                    key_of(&camera, (1, 7), &frame, (640, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "resized",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (641, 480), &point),
                    &settings,
                    3,
                ),
            ),
            (
                "hover highlight",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (640, 480), &hover),
                    &settings,
                    3,
                ),
            ),
            (
                "handle gone",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (640, 480), &[]),
                    &settings,
                    3,
                ),
            ),
            (
                "render settings",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (640, 480), &point),
                    &background,
                    3,
                ),
            ),
            (
                "accumulation",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (640, 480), &point),
                    &no_accumulation,
                    3,
                ),
            ),
            (
                "materials",
                picture_key(
                    key_of(&camera, (1, 7), &filters, (640, 480), &point),
                    &settings,
                    4,
                ),
            ),
        ];
        for (what, key) in changed {
            assert_ne!(key, base, "{what} must redraw the kept 3D view");
        }
    }

    #[test]
    fn a_kept_picture_is_drawn_again_only_when_it_has_to_be() {
        assert!(
            needs_draw(None, 1, false, false),
            "a new framebuffer holds nothing yet"
        );
        assert!(needs_draw(Some(1), 2, false, false), "another picture");
        assert!(
            !needs_draw(Some(1), 1, false, false),
            "the same picture is shown again"
        );
        assert!(
            needs_draw(Some(1), 1, true, false),
            "samples still to gather"
        );
        assert!(needs_draw(Some(1), 1, false, true), "an animation plays");
    }
}
