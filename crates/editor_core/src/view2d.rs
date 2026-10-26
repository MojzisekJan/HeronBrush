//! A flat (2D) map view without its widget: its camera and state, everything it draws,
//! and the picking its tools use. Drawing goes through [`Canvas`], so the window and an
//! offscreen [`crate::canvas::Raster`] draw the same view; the frontend wires its input to the tools.

use std::cell::RefCell;
use std::rc::Rc;

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::{self, Vec2d, Vec3d};

use crate::camera2d::{self, Camera2d};
use crate::canvas::Canvas;
use crate::clip_tool::ClipTool;
use crate::document::{Document, FaceRef};
use crate::extrude_tool;
use crate::grid::Grid;
use crate::handle_drag::HandlePicker;
use crate::handle_tool::{HandleKind, HandleManager};
use crate::i18n::{counted, fill};
use crate::preferences::{pref, Color};
use crate::rotate_tool::RotateTool;
use crate::scale_tool::{self, AnchorPos, BBoxHandle, ProportionalAxes};
use crate::shape_tool::ShapeParameters;
use crate::shear_tool;
use crate::tool::Tool;
use crate::view_filters::{EntityLinkMode, ViewFilters};
use crate::world::WorldBounds;
use map_model::brush::Brush;

/// The colour one world axis is drawn in.
///
/// `Preferences::axisColor`, which picks between the three by index.
pub fn axis_colour(axis: usize) -> Color {
    match axis {
        0 => pref(|p| p.x_axis_color),
        1 => pref(|p| p.y_axis_color),
        _ => pref(|p| p.z_axis_color),
    }
}

/// Sets the source to a colour from the preferences.
pub fn set_colour(cr: &mut dyn Canvas, colour: Color) {
    let (r, g, b, a) = colour.rgba_f64();
    cr.set_source_rgba(r, g, b, a);
}

/// Sets the source to the ink the overlay text is written in.
///
/// The text sits straight on the view, so it takes its lightness from what the view is
/// cleared to rather than from a colour of its own: pale ink over a dark background,
/// dark ink over a light one. `faint` is the quieter grey the undo line uses.
pub fn set_ink(cr: &mut dyn Canvas, faint: bool) {
    let (r, g, b) = match (pref(|p| p.background_color).is_dark(), faint) {
        (true, false) => (0.75, 0.75, 0.78),
        (true, true) => (0.55, 0.55, 0.6),
        (false, false) => (0.18, 0.18, 0.22),
        (false, true) => (0.38, 0.38, 0.43),
    };
    cr.set_source_rgb(r, g, b);
}

/// Brings the other views into line with the one that just moved.
///
/// The views share whichever axes they both show, so panning or zooming one has to move
/// the rest — that is what keeps the three panes reading as one map rather than three.
/// The "Link 2D views" preference turns this off, and each view then keeps its own.
pub fn link_cameras(
    master: &Rc<RefCell<ViewportState>>,
    cameras: &Rc<RefCell<Vec<Rc<RefCell<ViewportState>>>>>,
) {
    if !crate::preferences::pref(|p| p.link_2d_cameras) {
        return;
    }
    // The master's camera is copied out first, so the others can be borrowed mutably
    // without the master's borrow still being held.
    let leader = master.borrow().camera;

    let cameras = cameras.borrow();
    let mut others: Vec<std::cell::RefMut<'_, ViewportState>> = cameras
        .iter()
        .filter(|state| !Rc::ptr_eq(state, master))
        .map(|state| state.borrow_mut())
        .collect();

    let mut borrowed: Vec<&mut Camera2d> =
        others.iter_mut().map(|state| &mut state.camera).collect();
    camera2d::update_cameras(&leader, &mut borrowed);
}

/// The per-view state a viewport draws from, shared between the widget and its input
/// handlers.
///
/// The grid is *not* part of this: it is shared by every view, so it lives in its own
/// handle that the window's shortcuts also hold.
pub struct ViewportState {
    /// The camera for this view.
    pub camera: Camera2d,
    /// The pointer position in viewport pixels, while the pointer is over this view.
    pub cursor: Option<Vec2d>,
    /// The extent of the world this view outlines.
    pub bounds: WorldBounds,
    /// Which document this view has already pointed itself at.
    pub framed_generation: Option<u64>,
    /// The box a create drag on this view is sweeping out.
    pub creating: Option<geometry_math::bbox::Bbox3d>,
    /// The box a create drag on *another* view (the 3D one, say) is sweeping out, so this
    /// view shows the shape too. Set from the shared `pending_shape` each frame.
    pub creating_elsewhere: Option<geometry_math::bbox::Bbox3d>,
    /// The rectangle a selection drag has swept out, in viewport pixels.
    pub lasso: Option<(Vec2d, Vec2d)>,
    /// The bounding-box handle a scale drag has hold of, so the view can show it and the
    /// anchor across the box from it.
    pub scaling: Option<BBoxHandle>,
    /// Whether a rotation drag is under way, so the view can show how far it has turned.
    pub rotating: bool,
    /// A selection drag forces the selection guide to remain visible.
    pub moving: bool,
    /// Where the handle a drag has hold of is now: `NodeHandleToolBase`'s
    /// `m_dragHandlePosition`, which is what the guide spikes come out of.
    pub dragging_handle: Option<Vec3d>,
    /// Bounds preview for an entity dragged from the browser.
    pub entity_drop: Option<geometry_math::bbox::Bbox3d>,
}

/// Points the view at the whole document when a new one is opened.
///
/// This runs from the draw function rather than on a resize, because a view has no size
/// until it is first drawn, and a document can be opened at any time after that.
pub fn frame_new_document(state: &mut ViewportState, document: &Document, viewport: (f64, f64)) {
    if state.framed_generation == Some(document.generation()) {
        return;
    }
    // A viewport with no area yet cannot be framed into; try again on the next draw.
    if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
        return;
    }

    if let Some(bounds) = document.bounds() {
        state.camera.frame(&bounds, viewport);
    }
    state.framed_generation = Some(document.generation());
}

pub fn draw(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    draw_base(state, grid, document, filters, cr, viewport);
    draw_over_base(state, document, filters, cr, viewport);
}

/// Whether the brushes are drawn the animation preview's way, posed or with hidden faces
/// left out, rather than from the edge cache.
fn animated_brushes(filters: &ViewFilters) -> bool {
    filters.animation_frame.is_some() || filters.preview_motion || !filters.show_export_hidden
}

/// What the bottom of the picture depends on: the camera, the view's size, the grid, the
/// preferences, the filters that decide whether brush edges are drawn, and the
/// unselected brushes' edges. A view that keeps the picture [`draw_base`] made can draw
/// it again unchanged while this stays the same.
pub fn base_key(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    filters: &ViewFilters,
    viewport: (f64, f64),
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let camera = &state.camera;
    (camera.plane as u8).hash(&mut h);
    for v in [
        camera.position.x(),
        camera.position.y(),
        camera.position.z(),
        camera.zoom,
        viewport.0,
        viewport.1,
        state.bounds.half_extent(),
        grid.actual_size(),
    ] {
        v.to_bits().hash(&mut h);
    }
    grid.visible().hash(&mut h);
    crate::preferences::version().hash(&mut h);
    let edges = filters.show_brushes && filters.show_edges && !animated_brushes(filters);
    edges.hash(&mut h);
    if edges {
        document.generation().hash(&mut h);
        document.static_edges_generation().hash(&mut h);
    }
    h.finish()
}

/// The bottom of the picture, which only changes when [`base_key`] does: the background,
/// the grid, the axes, the world's bounds and the edges of every brush that is not
/// selected.
pub fn draw_base(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // Background.
    set_colour(cr, pref(|p| p.background_color));
    cr.paint();

    draw_grid(state, grid, cr, viewport);
    draw_axes(state, cr, viewport);
    draw_world_bounds(state, cr, viewport);
    if filters.show_brushes && filters.show_edges && !animated_brushes(filters) {
        draw_static_brushes(state, document, cr, viewport);
    }
}

/// Everything [`draw`] puts over [`draw_base`]: the selection, the entities, bounds and
/// links, the tool's own marks and the view's name.
pub fn draw_over_base(
    state: &ViewportState,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    document.animation_preview_time(filters.preview_motion && filters.animation_frame.is_none());
    if filters.show_brushes && filters.show_edges {
        if animated_brushes(filters) {
            draw_animation_brushes(state, document, filters, cr, viewport);
        } else {
            draw_selected_brushes(state, document, cr, viewport);
        }
    }
    if filters.show_point_entities {
        draw_entities(state, document, filters, cr, viewport);
    }
    if filters.animation_frame.is_none() && filters.show_group_bounds {
        draw_group_bounds(state, document, cr, viewport);
    }
    if filters.animation_frame.is_none() && filters.show_brush_entity_bounds {
        draw_brush_entity_bounds(state, document, cr, viewport);
    }
    if filters.animation_frame.is_none() {
        draw_entity_links(state, document, filters, cr, viewport);
        draw_linked_group_links(state, document, cr, viewport);
    }
    draw_leak_trace(state, document, cr, viewport);
    draw_portals(state, document, cr, viewport);
    draw_creation_box(state, cr, viewport);
    draw_entity_drop(state, cr, viewport);
    draw_lasso(state, cr);

    // The view name, so the three panes are told apart.
    set_ink(cr, false);
    cr.move_to(8.0, 18.0);
    draw_text(cr, crate::i18n::tr(state.camera.plane.name()), 12.0);
    if let Some(frame) = filters.animation_frame {
        cr.move_to(8.0, 34.0);
        draw_text(
            cr,
            &format!("{} {} · Esc", crate::i18n::tr("Frame"), frame),
            12.0,
        );
    }
}

/// Draws `text` with its baseline at the context's current point, through the canvas's
/// own text shaping.
///
/// A canvas must fall back to other fonts for scripts its first face lacks -- the Chinese
/// view names, for one -- rather than draw them as nothing. `size` is in pixels, matching
/// what `set_font_size` took.
pub fn draw_text(cr: &mut dyn Canvas, text: &str, size: f64) {
    cr.show_text(text, size);
}

/// Which colours a label over the map is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    /// Selection-size labels use a faint info-overlay background.
    Weak,
    /// A note about what is picked: the coordinates of the handle a drag has hold of,
    /// `SelectedInfoOverlayTextColor` on `SelectedInfoOverlayBackgroundColor`.
    Picked,
}

impl LabelStyle {
    /// The ink and the box, in that order.
    fn colours(self) -> (Color, Color) {
        match self {
            LabelStyle::Weak => {
                let background = pref(|p| p.info_overlay_background_color);
                (
                    pref(|p| p.info_overlay_text_color),
                    background.with_alpha(pref(|p| p.weak_info_overlay_background_alpha)),
                )
            }
            LabelStyle::Picked => (
                pref(|p| p.selected_info_overlay_text_color),
                pref(|p| p.selected_info_overlay_background_color),
            ),
        }
    }
}

/// Draws a label beside `anchor`, on the side `alignment` says, over a box in the info
/// overlay colours: `RenderService::renderString` with a background, placed by
/// `TextAnchor3D::offset`.
///
/// `gap` offsets a label from its anchor: eight pixels for sizes and sixteen for handle
/// coordinates.
pub fn draw_label(
    cr: &mut dyn Canvas,
    text: &str,
    anchor: Vec2d,
    alignment: crate::size_labels::Alignment,
    gap: f64,
    style: LabelStyle,
) {
    use crate::size_labels::Alignment;

    let extents = cr.text_extents(text, 11.0);
    let (width, height) = (extents.x_advance, extents.line_height);

    // Where the box's top left goes: centred on the anchor across the axis the label is
    // offset along, and a gap away from it.
    let (x, y) = match alignment {
        Alignment::Top => (anchor.x() - width / 2.0, anchor.y() + gap),
        Alignment::Bottom => (anchor.x() - width / 2.0, anchor.y() - gap - height),
        Alignment::Left => (anchor.x() + gap, anchor.y() - height / 2.0),
        Alignment::Right => (anchor.x() - gap - width, anchor.y() - height / 2.0),
    };
    let (x, y) = (x.round(), y.round());

    let padding = 2.0;
    let (ink, box_colour) = style.colours();
    set_colour(cr, box_colour);
    cr.rectangle(
        x - padding,
        y - padding,
        width + 2.0 * padding,
        height + 2.0 * padding,
    );
    cr.fill();

    set_colour(cr, ink);
    cr.move_to(x, y + extents.baseline);
    cr.show_text(text, 11.0);
}

/// Whether the selection's sizes are shown, `RenderContext::showSelectionGuide`.
///
/// Selection guides appear on selected-object hover and during movement or rotation. Scale, shear
/// and clip tools draw their own guides.
pub fn selection_guide_shown(
    tool: Tool,
    hovering_selected: bool,
    moving: bool,
    rotating: bool,
) -> bool {
    match tool {
        Tool::Select | Tool::Brush => hovering_selected || moving,
        Tool::Rotate => hovering_selected || rotating,
        _ => false,
    }
}

/// Draws the selection's extents beside the box, `SelectionBoundsRenderer::renderSize2D`.
pub fn draw_size_labels(
    state: &ViewportState,
    document: &Document,
    tool: Tool,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let Some(bounds) = document.selection_bounds() else {
        return;
    };
    let hovering_selected = !state.moving
        && !state.rotating
        && state.cursor.is_some_and(|cursor| {
            let ray = state.camera.ray_through(cursor, viewport, PICK_REACH);
            document.pick_selected_flat(&ray).is_some()
        });
    if !selection_guide_shown(tool, hovering_selected, state.moving, state.rotating) {
        return;
    }
    for label in crate::size_labels::labels_2d(&bounds, state.camera.direction()) {
        let anchor = state.camera.world_to_screen(label.base, viewport);
        draw_label(
            cr,
            &label.text,
            anchor,
            label.alignment,
            crate::size_labels::GAP,
            LabelStyle::Weak,
        );
    }
}

/// Draws the compass in the corner: the two axes this view shows, `Compass2D`.
pub fn draw_compass(state: &ViewportState, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    let plane = state.camera.plane;
    let origin = plane.unproject(Vec2d::zero(), 0.0);
    let right = plane.unproject(Vec2d::new(1.0, 0.0), 0.0) - origin;
    let up = plane.unproject(Vec2d::new(0.0, 1.0), 0.0) - origin;
    let arrows =
        crate::compass::arrows(right, up, plane.view_direction(), Some(plane.depth_axis()));
    crate::compass::draw(cr, viewport, &arrows);
}

/// How far above a handle its coordinates are written: `RenderService::renderString`'s
/// extra offset of sixteen for a string placed at a point.
pub const HANDLE_LABEL_GAP: f64 = 16.0;

/// How near the pointer has to be, on screen, to take hold of a clip point.
pub const CLIP_HANDLE_RADIUS: f64 = 4.0;

/// How large a vertex, edge or face handle is drawn, and how near the pointer has to be
/// to take hold of it.
pub fn handle_radius() -> f64 {
    f64::from(pref(|p| p.handle_radius))
}

/// How large a handle is drawn, and how near the pointer has to be to take hold of one.
///
/// Drawing and hit testing share the handle radius, with additional screen-space padding to make
/// small handles easier to pick.
pub fn pick_radius() -> f64 {
    handle_radius() + 2.0
}

/// Draws the handles the current handle tool offers.
pub fn draw_handles(
    state: &ViewportState,
    handles: &HandleManager,
    hovered: Option<Vec3d>,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // The spikes out of the handle a drag has hold of, along the world axes:
    // `PointGuideRenderer` through `renderDragGuide`. Two of the six run into the
    // screen in a flat view and come to nothing, which is right -- that axis is the one
    // this view cannot show.
    if let Some(dragged) = state.dragging_handle {
        // `VertexTool::renderGuide` halves the handle colour's alpha.
        let colour = pref(|p| p.handle_color);
        let colour = colour.with_alpha(colour.a * 0.5);
        cr.set_line_width(1.0);
        // A spike is two segments, the far one fading out, so each is stroked in its own
        // colour: the average of what `SpikeGuideRenderer` gives its two ends.
        for pair in crate::overlay::point_guide(dragged, colour.rgba_f32()).chunks(2) {
            let alpha = (pair[0].1[3] + pair[1].1[3]) / 2.0;
            set_colour(cr, colour.with_alpha(alpha));
            let from = state.camera.world_to_screen(pair[0].0, viewport);
            let to = state.camera.world_to_screen(pair[1].0, viewport);
            cr.move_to(from.x(), from.y());
            cr.line_to(to.x(), to.y());
            cr.stroke();
        }
    }

    // What the pointer is over, marked before the handles so the handles sit on top of
    // it: the edge or the face outline the handle stands for, in the picked colour.
    if let Some(handle) = hovered.and_then(|position| {
        handles
            .handles()
            .iter()
            .find(|handle| vec::is_zero(handle.position - position, 0.001))
    }) {
        set_colour(cr, pref(|p| p.selected_handle_color));
        cr.set_line_width(2.0);
        for (from, to) in crate::handle_tool::outline_of(handle) {
            let from = state.camera.world_to_screen(from, viewport);
            let to = state.camera.world_to_screen(to, viewport);
            cr.move_to(from.x(), from.y());
            cr.line_to(to.x(), to.y());
        }
        cr.stroke();
    }

    for handle in handles.handles() {
        let on_screen = state
            .camera
            .plane_to_screen(state.camera.plane.project(handle.position), viewport);

        let over = hovered.is_some_and(|position| vec::is_zero(handle.position - position, 0.001));
        if handles.is_selected(handle.position) || over {
            set_colour(cr, pref(|p| p.selected_handle_color));
        } else {
            set_colour(cr, pref(|p| p.handle_color));
        }
        // Hovered handles grow to indicate the click target. Selected handles are red, unselected
        // ones yellow, and vertex-insertion handles smaller.
        let radius = if over {
            pick_radius() + 1.5
        } else {
            pick_radius()
        };
        let radius = if handle.splits { radius * 0.55 } else { radius };
        cr.new_sub_path();
        cr.arc(
            on_screen.x(),
            on_screen.y(),
            radius,
            0.0,
            std::f64::consts::TAU,
        );
        cr.fill_preserve();
        cr.set_source_rgb(0.1, 0.1, 0.12);
        cr.set_line_width(1.0);
        cr.stroke();
    }
    // The handle a press would take or a drag is moving, written out as its three
    // coordinates: `NodeHandleToolBase::renderHighlight`, which draws the position beside
    // the handle it highlights.
    if let Some(at) = state.dragging_handle.or(hovered) {
        let anchor = state.camera.world_to_screen(at, viewport);
        draw_label(
            cr,
            &crate::size_labels::point_text(at),
            anchor,
            crate::size_labels::Alignment::Bottom,
            HANDLE_LABEL_GAP,
            LabelStyle::Picked,
        );
    }
}

/// Marks the edge a fillet press would round, and outlines what it would leave.
///
/// The outline is dashed, as a preview of something that has not happened is drawn
/// everywhere else, and it is worked out from the brushes as they are -- looking changes
/// nothing.
pub fn draw_fillet_preview(
    state: &ViewportState,
    document: &Document,
    handles: &HandleManager,
    hovered: Option<Vec3d>,
    parameters: crate::fillet_tool::FilletParameters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let edges = crate::fillet_tool::edges_to_round(document, handles, hovered);
    if edges.is_empty() {
        return;
    }

    let project = |point: Vec3d| state.camera.world_to_screen(point, viewport);

    // The edge itself, in the colour a picked handle wears.
    set_colour(cr, pref(|p| p.selected_handle_color));
    cr.set_line_width(2.5);
    for (from, to) in &edges {
        let (from, to) = (project(*from), project(*to));
        cr.move_to(from.x(), from.y());
        cr.line_to(to.x(), to.y());
    }
    cr.stroke();

    // What the rounding would leave.
    let outline = crate::fillet_tool::preview_edges(document, &edges, parameters);
    if outline.is_empty() {
        return;
    }
    set_colour(cr, pref(|p| p.handle_color));
    cr.set_line_width(1.0);
    cr.set_dash(&[4.0, 3.0], 0.0);
    for (from, to) in outline {
        let (from, to) = (project(from), project(to));
        cr.move_to(from.x(), from.y());
        cr.line_to(to.x(), to.y());
    }
    cr.stroke();
    cr.set_dash(&[], 0.0);
}

/// Says what the handle tool would do, so the keyboard actions have a visible subject.
pub fn draw_handle_status(handles: &HandleManager, cr: &mut dyn Canvas) {
    let Some(kind) = handles.kind() else {
        return;
    };
    let count = handles.selected_count();
    let total = handles.handles().len();

    // In the edges' colour, which follows the scheme: the handles' yellow does not read
    // on the light scheme's grey.
    set_colour(cr, pref(|p| p.edge_color));
    cr.move_to(8.0, 60.0);
    draw_text(
        cr,
        &format!(
            "{} · {count} of {total} selected",
            kind.action_name(count.max(1))
        ),
        10.0,
    );
}

/// Draws the handles the scale tool offers on the selection's bounding box.
pub fn draw_scale_handles(
    state: &ViewportState,
    box_: &Bbox3d,
    dragged: Option<BBoxHandle>,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // `ScaleTool::dragAnchor`: while a handle is being dragged, the one across the box
    // from it is what stays put, and showing it says which way the box will grow.
    let anchor = dragged.map(|handle| handle.opposite());

    for handle in scale_tool::all_handles() {
        let on_screen = state
            .camera
            .plane_to_screen(state.camera.plane.project(handle.position(box_)), viewport);

        if Some(handle) == anchor {
            set_colour(cr, pref(|p| p.scale_handle_color));
        } else if Some(handle) == dragged {
            set_colour(cr, pref(|p| p.selected_handle_color));
        } else {
            set_colour(cr, pref(|p| p.handle_color));
        }
        cr.rectangle(
            on_screen.x() - handle_radius(),
            on_screen.y() - handle_radius(),
            handle_radius() * 2.0,
            handle_radius() * 2.0,
        );
        cr.fill_preserve();
        cr.set_source_rgb(0.1, 0.1, 0.12);
        cr.set_line_width(1.0);
        cr.stroke();
    }
}

/// Draws the handles the shear tool offers: one at the middle of each face.
pub fn draw_shear_handles(
    state: &ViewportState,
    box_: &Bbox3d,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    for (_, center) in shear_tool::handles(box_) {
        let on_screen = state
            .camera
            .plane_to_screen(state.camera.plane.project(center), viewport);

        set_colour(cr, pref(|p| p.shear_outline_color));
        cr.rectangle(
            on_screen.x() - handle_radius(),
            on_screen.y() - handle_radius(),
            handle_radius() * 2.0,
            handle_radius() * 2.0,
        );
        cr.fill_preserve();
        cr.set_source_rgb(0.1, 0.1, 0.12);
        cr.set_line_width(1.0);
        cr.stroke();
    }
}

/// Works out whether a Shift-press in a flat view landed on a silhouette edge.
///
/// `ExtrudeTool::pick2D`. In a flat view a face is seen edge-on, so there is nothing to
/// click on; the silhouette edges are what there is, and grabbing one drags the face
/// behind it.
pub fn begin_extrude_2d(
    document: &Document,
    ray: &geometry_math::ray::Ray3d,
    view_direction: Vec3d,
    ctrl: bool,
    alt: bool,
) -> Option<Extruding> {
    if !ctrl && !alt {
        if let Some((brush, corner)) = document.extrude_corner_2d(ray) {
            let (first, normal) = corner.faces[0];
            return Some(Extruding {
                polygon: brush.face_polygon(first)?,
                face: brush.geometry_face_of(first)?,
                second_polygon: Some(brush.face_polygon(corner.faces[1].0)?),
                brush,
                normal,
                handle: corner.handle,
                picker: extrude_tool::corner_picker(&corner, view_direction, ray),
                mode: extrude_tool::ExtrudeMode::Extrude,
            });
        }
    }
    // The nearest silhouette edge of any selected brush.
    let (brush, edge) = document
        .selected_brushes()
        .into_iter()
        .filter_map(|brush| {
            extrude_tool::find_closest_horizon_edge(brush, ray).map(|edge| (brush.clone(), edge))
        })
        .min_by(|(_, a), (_, b)| {
            a.from_ray
                .partial_cmp(&b.from_ray)
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let polygon = brush.face_polygon(edge.face_index)?;
    let face = brush.geometry_face_of(edge.face_index)?;

    Some(Extruding {
        second_polygon: None,
        polygon,
        brush,
        face,
        normal: edge.normal,
        handle: edge.handle,
        picker: extrude_tool::horizon_picker(&edge, view_direction, ray),
        mode: extrude_tool::mode_for(ctrl, alt),
    })
}

/// Returns the scale handle nearest the cursor, within reach.
///
/// A 2D view flattens the box, so several handles land on the same pixel; the nearest one
/// wins, and among equals the first, which is a side before an edge before a corner --
/// the order `allSides`, `allEdges` and `allCorners` are gathered in.
pub fn pick_scale_handle(
    state: &ViewportState,
    box_: &Bbox3d,
    viewport: (f64, f64),
    cursor: Vec2d,
) -> Option<BBoxHandle> {
    let camera = state.camera;
    scale_tool::all_handles()
        .into_iter()
        .map(|handle| {
            let on_screen =
                camera.plane_to_screen(camera.plane.project(handle.position(box_)), viewport);
            (handle, vec::squared_distance(on_screen, cursor))
        })
        .filter(|(_, distance)| *distance <= (handle_radius() + 2.0).powi(2))
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(handle, _)| handle)
}

/// How large the rotation ring is drawn, in pixels.
pub fn rotate_ring_radius() -> f64 {
    f64::from(pref(|p| p.rotate_handle_radius))
}

/// Draws the rotation ring and its centre.
pub fn draw_rotation_handle(
    state: &ViewportState,
    center: Vec3d,
    angle: Option<f64>,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let on_screen = state
        .camera
        .plane_to_screen(state.camera.plane.project(center), viewport);

    set_colour(cr, pref(|p| p.handle_color));
    cr.set_line_width(1.5);
    // `new_sub_path` before a full circle: without it, `arc` draws a line from wherever
    // the last path left its point to the start of the ring -- a stray spoke across the
    // view.
    cr.new_sub_path();
    cr.arc(
        on_screen.x(),
        on_screen.y(),
        rotate_ring_radius(),
        0.0,
        std::f64::consts::TAU,
    );
    cr.stroke();

    // While the ring is being dragged, a wedge from where the drag started shows how far
    // it has turned, the way `renderAngleIndicator` does.
    if let Some(angle) = angle {
        set_colour(cr, pref(|p| p.rotate_handle_color).with_alpha(0.35));
        cr.move_to(on_screen.x(), on_screen.y());
        // The screen's Y runs down where the world's second axis runs up, so a turn that
        // is counter-clockwise in the world is clockwise on screen.
        cr.arc_negative(
            on_screen.x(),
            on_screen.y(),
            rotate_ring_radius(),
            0.0,
            -angle,
        );
        cr.close_path();
        cr.fill();

        set_colour(cr, pref(|p| p.selected_handle_color));
        cr.move_to(on_screen.x() + 8.0, on_screen.y() - 8.0);
        draw_text(cr, &format!("{:.1}°", angle.to_degrees()), 10.0);
    }

    // The centre itself, which can be dragged to turn about somewhere else.
    set_colour(cr, pref(|p| p.selected_handle_color));
    cr.new_sub_path();
    cr.arc(
        on_screen.x(),
        on_screen.y(),
        handle_radius(),
        0.0,
        std::f64::consts::TAU,
    );
    cr.fill();
}

/// Handles a press while a handle tool is active.
///
/// Returns the drag mode: taking hold of a handle starts moving it, and a press that
/// reaches no handle drops what was picked and begins a lasso over the handles, the way
/// dragging in a file list sweeps over files (`VertexToolController`'s own lasso).
#[allow(
    clippy::too_many_arguments,
    reason = "a press is decided by the view, the handles, the pointer and the modifiers"
)]
pub fn begin_handle_drag(
    state: &ViewportState,
    handles: &mut HandleManager,
    document: &Rc<RefCell<Document>>,
    viewport: (f64, f64),
    cursor: Vec2d,
    add_to_selection: bool,
    split: bool,
    snap: bool,
    splitting: &mut Option<Vec3d>,
    grabbed: &mut Option<Vec3d>,
) -> DragMode {
    // `VertexTool`'s split: Shift and a drag from the middle of an edge or the centre of a
    // face puts a corner there and drags it, which is how a brush gains a vertex. The
    // corner appears where the drag takes it, since one sitting on an edge is no corner of
    // the hull at all.
    // `MoveVertexPart::mouseClick`: Alt and Shift with exactly one vertex picked snaps
    // that vertex onto whatever handle is clicked, which is how two corners are brought
    // together.
    if snap {
        let camera = state.camera;
        let project =
            |position: Vec3d| camera.plane_to_screen(camera.plane.project(position), viewport);
        if let (Some(target), Some(from)) = (
            handles.pick_any(cursor, pick_radius(), project),
            handles.selected_anchor(),
        ) {
            let delta = target - from;
            if delta != Vec3d::zero() && document.borrow_mut().move_handles(&[from], delta) {
                handles.move_selection(delta);
                return DragMode::Idle;
            }
        }
    }

    if split {
        let camera = state.camera;
        let project =
            |position: Vec3d| camera.plane_to_screen(camera.plane.project(position), viewport);
        if let Some(position) = handles.pick_split(cursor, pick_radius(), project) {
            handles.clear_selection();
            let _ = document;
            *splitting = Some(position);
            return DragMode::SplittingHandle;
        }
    }

    match pick_handle(state, handles, viewport, cursor) {
        Some(position) => {
            // Dragging a handle that is not selected makes it the selection first, so a
            // drag always moves what is under the pointer.
            if !handles.is_selected(position) {
                handles.select(position, add_to_selection);
            }
            // Which handle the drag has hold of, for the guide that comes out of it.
            *grabbed = Some(position);
            DragMode::MovingHandle
        }
        None => {
            if !add_to_selection {
                handles.clear_selection();
            }
            DragMode::LassoingHandles
        }
    }
}

/// The handle under the cursor, if the press is near enough to one.
pub fn pick_handle(
    state: &ViewportState,
    handles: &HandleManager,
    viewport: (f64, f64),
    cursor: Vec2d,
) -> Option<Vec3d> {
    let camera = state.camera;
    let project =
        |position: Vec3d| camera.plane_to_screen(camera.plane.project(position), viewport);
    handles.pick_orthographic(cursor, pick_radius(), camera.direction(), project)
}

/// Handles a press while the clip tool is active.
///
/// `AddClipPointPart` and `MoveClipPointPart`: a press on an existing point takes hold of
/// it, and a press anywhere else places a new one.
pub fn begin_clip(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    clip: &mut ClipTool,
    viewport: (f64, f64),
    cursor: Vec2d,
) -> DragMode {
    let Some(position) = clip_point_under_cursor(state, grid, document, viewport, cursor) else {
        return DragMode::Idle;
    };

    // How far the handle radius reaches in world units, since the points are compared in
    // world space but the pointer is near them on screen.
    let reach = CLIP_HANDLE_RADIUS * 2.0 / state.camera.zoom;
    if clip.begin_drag_point(position, reach).is_some() {
        return DragMode::ClippingPoint;
    }

    // The help vector is the direction this view looks, which is what lets two points
    // placed here describe a plane at all.
    let help_vectors = vec![state.camera.direction()];
    if clip.add_point(position, help_vectors) {
        // Placing a point starts dragging it, so it can be nudged before the button comes
        // up. `AddClipPointPart::startDrag` does the same.
        clip.begin_drag_point(position, reach);
        return DragMode::ClippingPoint;
    }

    DragMode::Idle
}

/// Returns the world point under the pointer for a clip point, snapped to the grid.
///
/// `PartDelegate2D::doGetNewClipPointPositionAndHitPoint`: a 2D view says nothing about
/// the axis it looks down, so the point is put on the plane through the middle of the
/// selection, facing the view. Without a selection there is nothing to clip and so
/// nothing to place.
pub fn clip_point_under_cursor(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    viewport: (f64, f64),
    cursor: Vec2d,
) -> Option<Vec3d> {
    let center = document.selection_bounds()?.center();
    let on_plane = state.camera.screen_to_plane(cursor, viewport);
    let depth = center[state.camera.plane.depth_axis()];
    Some(grid.snap_point(state.camera.plane.unproject(on_plane, depth)))
}

/// The next point uses exactly the same placement and picking rules as a click.
pub fn clip_preview_point(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    clip: &ClipTool,
    viewport: (f64, f64),
) -> Option<Vec3d> {
    if clip.is_dragging() {
        return None;
    }
    let point = clip_point_under_cursor(state, grid, document, viewport, state.cursor?)?;
    let reach = CLIP_HANDLE_RADIUS * 2.0 / state.camera.zoom;
    if clip
        .points()
        .iter()
        .any(|p| vec::squared_distance(*p, point) <= reach * reach)
    {
        return None;
    }
    clip.can_add_point(point).then_some(point)
}

/// Whether the cursor is on the sweep tool's destination cap.
pub fn sweep_cap_under_cursor(
    state: &ViewportState,
    document: &Document,
    sweep: &crate::sweep_tool::SweepInputs,
    viewport: (f64, f64),
    cursor: Vec2d,
) -> Option<()> {
    let source = document.sweep_source()?;
    let destination = sweep.transform.destination_center(&source);
    let on_screen = state
        .camera
        .plane_to_screen(state.camera.plane.project(destination), viewport);
    (vec::distance(on_screen, cursor) <= handle_radius() + 2.0).then_some(())
}

/// Where a sweep-cap drag has moved the destination to, keeping the depth the view does
/// not show and turning the two it does into a fresh translation.
pub fn sweep_cap_translation(
    state: &ViewportState,
    document: &Document,
    grid: &Grid,
    viewport: (f64, f64),
    cursor: Vec2d,
    current: Vec3d,
) -> Option<Vec3d> {
    let source = document.sweep_source()?;
    let destination = source.center + current;
    let depth = destination[state.camera.plane.depth_axis()];
    let on_plane = state.camera.screen_to_plane(cursor, viewport);
    let moved = grid.snap_point(state.camera.plane.unproject(on_plane, depth));
    Some(moved - source.center)
}

/// Draws the sweep tool's source and destination caps, the line between them, and an
/// outline of the brushes the sweep would make.
pub fn draw_sweep(
    state: &ViewportState,
    document: &Document,
    sweep: &crate::sweep_tool::SweepInputs,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let Some(source) = document.sweep_source() else {
        return;
    };
    let destination = sweep.transform.destination_center(&source);

    if !sweep.transform.is_no_op() {
        let preview: Vec<(Vec3d, Vec3d)> = crate::sweep_tool::generate_brushes(
            &source,
            &sweep.transform,
            &sweep.parameters,
            document.current_material(),
        )
        .iter()
        .flat_map(|brush| brush.edge_positions())
        .collect();
        stroke_edges(
            state,
            &preview,
            cr,
            viewport,
            Color::rgba(0.4, 0.9, 0.5, 0.8),
            1.0,
        );
    }

    let from = state.camera.world_to_screen(source.center, viewport);
    let to = state.camera.world_to_screen(destination, viewport);
    set_colour(cr, Color::rgba(1.0, 0.65, 0.2, 0.8));
    cr.set_line_width(1.0);
    cr.move_to(from.x(), from.y());
    cr.line_to(to.x(), to.y());
    cr.stroke();
    for point in [from, to] {
        cr.arc(
            point.x(),
            point.y(),
            CLIP_HANDLE_RADIUS,
            0.0,
            std::f64::consts::TAU,
        );
        cr.fill();
    }
}

/// Draws the clip points, the line the cut would follow, and the outlines of the halves
/// it would keep and throw away.
pub fn draw_clip(
    state: &ViewportState,
    grid: &Grid,
    document: &Document,
    clip: &ClipTool,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // The halves, before the points, so the cut line and the handles sit on top.
    if clip.can_clip() {
        let bounds = document.world_bounds();
        let mut kept = Vec::new();
        let mut removed = Vec::new();
        for brush in document.selected_brushes() {
            for half in clip.clip_brush(&bounds, brush) {
                kept.extend(half.edge_positions());
            }
            for half in clip.clipped_away(brush) {
                removed.extend(half.edge_positions());
            }
        }
        let colour = pref(|p| p.clip_handle_color);
        stroke_edges(state, &removed, cr, viewport, colour.with_alpha(0.25), 1.0);
        stroke_edges(state, &kept, cr, viewport, colour.with_alpha(0.9), 1.5);
    }

    if let Some(point) = clip_preview_point(state, grid, document, clip, viewport) {
        let p = state.camera.world_to_screen(point, viewport);
        set_colour(cr, pref(|p| p.clip_handle_color));
        // Cairo connects arc() to the previous current point unless the path is cleared.
        cr.new_path();
        cr.arc(p.x(), p.y(), CLIP_HANDLE_RADIUS, 0.0, std::f64::consts::TAU);
        cr.fill_preserve();
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.65);
        cr.set_line_width(1.0);
        cr.stroke();
    }

    let points = clip.points();
    if points.is_empty() {
        return;
    }

    let screen: Vec<Vec2d> = points
        .iter()
        .map(|point| {
            state
                .camera
                .plane_to_screen(state.camera.plane.project(*point), viewport)
        })
        .collect();

    // The cut itself, once there is a plane, drawn through the points that describe it.
    if clip.can_clip() && screen.len() >= 2 {
        set_colour(cr, pref(|p| p.clip_face_color).with_alpha(1.0));
        cr.set_line_width(1.5);
        cr.move_to(screen[0].x(), screen[0].y());
        for point in &screen[1..] {
            cr.line_to(point.x(), point.y());
        }
        cr.stroke();
    }

    set_colour(cr, pref(|p| p.clip_handle_color));
    for point in &screen {
        cr.new_path();
        cr.arc(
            point.x(),
            point.y(),
            CLIP_HANDLE_RADIUS,
            0.0,
            std::f64::consts::TAU,
        );
        cr.fill();
    }
}

/// Draws the rectangle a selection drag has swept so far.
///
/// A dashed outline in the selection's own colour, which is what says it is a selection
/// being swept rather than geometry being made.
pub fn draw_lasso(state: &ViewportState, cr: &mut dyn Canvas) {
    let Some((from, to)) = state.lasso else {
        return;
    };

    set_colour(cr, pref(|p| p.selected_edge_color));
    cr.set_line_width(1.0);
    cr.set_dash(&[4.0, 3.0], 0.0);
    cr.rectangle(
        from.x().min(to.x()),
        from.y().min(to.y()),
        (to.x() - from.x()).abs(),
        (to.y() - from.y()).abs(),
    );
    cr.stroke();
    cr.set_dash(&[], 0.0);
}

/// Draws the box a create drag is sweeping out.
///
/// What is drawn is the box that would be made, not the rectangle the pointer swept: the
/// two differ by the snapping, and the box the drag will actually leave behind is the one
/// worth showing. It is held in world units and projected here, so the
/// outline lands on the grid lines it will land on.
pub fn draw_creation_box(state: &ViewportState, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    let Some(bounds) = state.creating.or(state.creating_elsewhere) else {
        return;
    };

    let corners = bounds
        .vertices()
        .map(|corner| state.camera.world_to_screen(corner, viewport));
    let left = corners.iter().map(|c| c.x()).fold(f64::INFINITY, f64::min);
    let right = corners
        .iter()
        .map(|c| c.x())
        .fold(f64::NEG_INFINITY, f64::max);
    let top = corners.iter().map(|c| c.y()).fold(f64::INFINITY, f64::min);
    let bottom = corners
        .iter()
        .map(|c| c.y())
        .fold(f64::NEG_INFINITY, f64::max);

    set_colour(cr, pref(|p| p.resize_handle_color));
    cr.set_line_width(1.0);
    cr.rectangle(left, top, right - left, bottom - top);
    cr.stroke();
}

/// Outlines the box an entity drag would drop into, so where it lands reads before the
/// button is let go.
///
/// Shows the prospective entity bounds while dragging and creates the entity only when dropped.
pub fn draw_entity_drop(state: &ViewportState, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    let Some(bounds) = state.entity_drop else {
        return;
    };

    set_colour(cr, pref(|p| p.handle_color));
    cr.set_line_width(1.0);
    cr.set_dash(&[4.0, 3.0], 0.0);
    for (from, to) in bounds.edges() {
        let from = state.camera.world_to_screen(from, viewport);
        let to = state.camera.world_to_screen(to, viewport);
        cr.move_to(from.x(), from.y());
        cr.line_to(to.x(), to.y());
    }
    cr.stroke();
    cr.set_dash(&[], 0.0);
}

/// Outlines the faces a Shift-drag would extrude, in the extrude-handle colour.
///
/// `ExtrudeToolController::render`, for a flat view. A coplanar face seen edge-on
/// projects to a line, which is the silhouette edge the drag would grab.
pub fn draw_extrude_preview(
    state: &ViewportState,
    outlines: &[Vec<Vec3d>],
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    if outlines.is_empty() {
        return;
    }
    set_colour(cr, pref(|p| p.resize_handle_color));
    cr.set_line_width(2.0);
    for outline in outlines {
        for (index, point) in outline.iter().enumerate() {
            let screen = state.camera.world_to_screen(*point, viewport);
            if index == 0 {
                cr.move_to(screen.x(), screen.y());
            } else {
                cr.line_to(screen.x(), screen.y());
            }
        }
        cr.close_path();
    }
    cr.stroke();
}

pub fn draw_grid(state: &ViewportState, grid: &Grid, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    if !grid.visible() {
        return;
    }

    let (w, h) = viewport;
    let camera = &state.camera;

    // Coarsen the grid as it is zoomed out so lines never crowd into a solid fill.
    let mut spacing = grid.actual_size();
    while spacing * camera.zoom < 4.0 {
        spacing *= 2.0;
    }

    let min = camera.screen_to_plane(Vec2d::new(0.0, h), viewport);
    let max = camera.screen_to_plane(Vec2d::new(w, 0.0), viewport);

    cr.set_line_width(1.0);

    let mut x = (min.x() / spacing).floor() * spacing;
    while x <= max.x() {
        // Every 8th line is brighter, giving a readable major/minor grid.
        let major = (x / (spacing * 8.0)).fract() == 0.0;
        set_grid_color(cr, major);
        let sx = camera
            .plane_to_screen(Vec2d::new(x, 0.0), viewport)
            .x()
            .floor()
            + 0.5;
        cr.move_to(sx, 0.0);
        cr.line_to(sx, h);
        cr.stroke();
        x += spacing;
    }

    let mut y = (min.y() / spacing).floor() * spacing;
    while y <= max.y() {
        let major = (y / (spacing * 8.0)).fract() == 0.0;
        set_grid_color(cr, major);
        let sy = camera
            .plane_to_screen(Vec2d::new(0.0, y), viewport)
            .y()
            .floor()
            + 0.5;
        cr.move_to(0.0, sy);
        cr.line_to(w, sy);
        cr.stroke();
        y += spacing;
    }
}

pub fn set_grid_color(cr: &mut dyn Canvas, major: bool) {
    if major {
        let colour = pref(|p| p.grid_color_2d);
        set_colour(cr, colour.with_alpha(colour.a * pref(|p| p.grid_alpha)));
    } else {
        let colour = pref(|p| p.grid_color_2d);
        set_colour(
            cr,
            colour.with_alpha(colour.a * pref(|p| p.grid_alpha) * 0.5),
        );
    }
}

pub fn draw_axes(state: &ViewportState, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    let (w, h) = viewport;
    let origin = state.camera.plane_to_screen(Vec2d::zero(), viewport);

    cr.set_line_width(1.0);

    set_colour(cr, axis_colour(state.camera.plane.shown_axes()[0]));
    cr.move_to(0.0, origin.y().floor() + 0.5);
    cr.line_to(w, origin.y().floor() + 0.5);
    cr.stroke();

    set_colour(cr, axis_colour(state.camera.plane.shown_axes()[1]));
    cr.move_to(origin.x().floor() + 0.5, 0.0);
    cr.line_to(origin.x().floor() + 0.5, h);
    cr.stroke();
}

/// How far the world extends in the direction a 2D view looks.
///
/// A pick ray starts this far back, so that everything the view can see is in front of
/// it whatever its depth.
pub const PICK_REACH: f64 = 1.0e6;

/// What a lasso between two viewport points has caught.
///
/// `Lasso::selectedNodes`: an object is caught when what the view shows of it lies inside
/// the rectangle. A flat view shows two of the three axes, so this is the brush's own
/// corners put on the screen and asked whether they are inside -- every corner, since a
/// lasso that caught a brush by one corner would catch the whole level from a distance.
pub fn lassoed(
    camera: &Camera2d,
    document: &Document,
    from: Vec2d,
    to: Vec2d,
    viewport: (f64, f64),
) -> Vec<map_model::node::NodeId> {
    let left = from.x().min(to.x());
    let right = from.x().max(to.x());
    let top = from.y().min(to.y());
    let bottom = from.y().max(to.y());

    let mut caught = Vec::new();
    for (id, brush) in document.selectable_brushes() {
        let inside = brush.vertex_positions().iter().all(|corner| {
            let on_screen = camera.world_to_screen(*corner, viewport);
            on_screen.x() >= left
                && on_screen.x() <= right
                && on_screen.y() >= top
                && on_screen.y() <= bottom
        });
        if inside {
            caught.push(id);
        }
    }
    caught
}

/// The box a drag in a flat view sweeps out from `from` to `to`, in viewport pixels:
/// `DrawShapeToolController2D::makeBounds`.
///
/// Snapped outwards to the grid, with the depth this view cannot speak for taken from the
/// reference bounds. With `square`, the two visible axes are made equal, the longer one
/// winning, growing away from where the drag began; with `cube` as well, the depth too.
/// Kept inside the world.
///
/// Returns `None` if what is left is too flat to be a solid.
#[allow(
    clippy::too_many_arguments,
    reason = "a drawn box depends on all of these"
)]
pub fn swept_bounds(
    camera: &Camera2d,
    grid: &Grid,
    from: Vec2d,
    to: Vec2d,
    viewport: (f64, f64),
    document: &Document,
    square: bool,
    cube: bool,
) -> Option<geometry_math::bbox::Bbox3d> {
    let initial = camera
        .plane
        .unproject(camera.screen_to_plane(from, viewport), 0.0);
    let current = camera
        .plane
        .unproject(camera.screen_to_plane(to, viewport), 0.0);

    // Snap outwards, then take the depth from the reference bounds along the axis this
    // view cannot speak for.
    let snapped_min = grid.snap_down_point(vec::min(initial, current));
    let snapped_max = grid.snap_up_point(vec::max(initial, current));
    let reference = document.reference_bounds();
    let factors = vec::abs(camera.direction().get_abs_max_component_axis(0));
    let mut min = vec::mix(snapped_min, reference.min, factors);
    let mut max = vec::mix(snapped_max, reference.max, factors);

    if square {
        let view_axis = vec::abs(camera.direction());
        let ortho_axes = Vec3d::one() - view_axis;
        let longest = ((max - min) * ortho_axes).get_abs_max_component(0);
        let extent = if cube {
            Vec3d::one() * longest
        } else {
            view_axis * (max - min) + ortho_axes * longest
        };
        // Growing away from where the drag started, so the corner under the pointer when
        // it went down stays put.
        let drag_direction = vec::step(initial, current);
        let grown_min = vec::mix(min, max - extent, Vec3d::one() - drag_direction);
        let grown_max = vec::mix(max, min + extent, drag_direction);
        min = grown_min;
        max = grown_max;
    }

    // A box with no extent on some axis is not a solid.
    let size = max - min;
    if size.v.iter().any(|extent| *extent <= 0.0) {
        return None;
    }
    geometry_math::bbox::Bbox3d::new(min, max).intersected(&document.world_bounds())
}

/// What a drag is doing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DragMode {
    /// The press hit nothing, so nothing is happening.
    #[default]
    Idle,
    /// The press took hold of a clip point, so the drag moves it.
    ClippingPoint,
    /// The press took hold of a vertex, edge or face handle, so the drag moves it.
    MovingHandle,
    /// The press took hold of a split handle with Shift, so the drag adds a corner there.
    SplittingHandle,
    /// The press took hold of a bounding-box handle, so the drag scales the selection.
    Scaling,
    /// The press took hold of the rotation ring, so the drag turns the selection.
    Rotating,
    /// The press took hold of the rotation centre, so the drag moves it.
    MovingRotationCenter,
    /// The press took hold of a face of the bounding box, so the drag shears it.
    Shearing,
    /// The press took hold of a silhouette edge, so the drag extrudes the face behind it.
    Extruding,
    /// The press hit a selected brush, so the drag moves the selection.
    Moving,
    /// The press hit nothing and the drag is sweeping out a new brush.
    Creating,
    /// The press hit nothing and the drag is sweeping out a selection.
    ///
    /// Ctrl-drag from empty space selects multiple objects with a marquee.
    Lassoing,
    /// Sweeping a rectangle over a tool's handles, with a handle tool in hand.
    LassoingHandles,
    /// A Ctrl-drag from something unselected, adding whatever it passes over to the
    /// selection (`PaintSelectionDragTracker`).
    PaintingSelection,
    /// [`Self::PaintingSelection`] with Shift, which adds faces.
    PaintingFaces,
    /// The press took hold of the sweep tool's destination cap, so the drag moves it.
    MovingSweepCap,
}

/// What a drag from empty space does in a flat view.
///
/// Dragging from empty space draws a brush without changing tools; Ctrl-drag selects with a
/// marquee. A drag that merely clears an existing selection does not draw, except when the Brush
/// tool is explicitly active.
pub fn empty_space_drag_mode(had_selection: bool, add_to_selection: bool, tool: Tool) -> DragMode {
    if add_to_selection {
        DragMode::Lassoing
    } else if had_selection && tool != Tool::Brush {
        DragMode::Idle
    } else {
        DragMode::Creating
    }
}

/// What a drag has done so far.
#[derive(Debug, Clone, Default)]
pub struct DragState {
    /// Where the press happened, in viewport pixels.
    pub origin: Vec2d,
    /// Where the pointer is now, in viewport pixels. Only meaningful while creating.
    pub current: Vec2d,
    /// How far the selection has already been moved.
    pub applied: Vec3d,
    /// Whether the first successful movement has included vertices hidden in this view.
    pub vertex_stacks_expanded: bool,
    /// What this drag is doing.
    pub mode: DragMode,
    /// The bounding-box handle being dragged, and the box it started from.
    ///
    /// A scale is applied from the box the drag began with rather than from the last
    /// step, for the same reason a move is measured from the press: applying each step to
    /// the result of the last would compound the snapping.
    pub scaling: Option<(BBoxHandle, Bbox3d)>,
    /// The face being sheared: its normal, where its middle was, and the box it came from.
    pub shearing: Option<(Vec3d, Vec3d, Bbox3d)>,
    /// The face being extruded, and how far it may go.
    pub extruding: Option<Extruding>,
    /// Where a split drag took hold, which is where the new corner is measured from.
    pub splitting: Option<Vec3d>,
    /// What the press would select if it ends as a click rather than a drag. Selection
    /// waits for the release, as in `SelectionTool`, so a drag can still draw a brush over
    /// an unselected one, or pull a face of the selected brush, instead.
    pub click: Option<Click>,
    /// A Ctrl-drag's copy of the selection: wanted when the move begins, then made, with
    /// where the history stood before it, so the copy and its move undo as one.
    pub duplicate: Duplicate,
    /// A handle a Ctrl-press added to what is picked, to be the only one picked once the
    /// press becomes a drag (`NodeHandleToolBase::startMove`). Let go where it was, the
    /// press was a Ctrl-click, which adds it.
    pub narrow_to: Option<Vec3d>,
}

/// Where a Ctrl-drag's duplicate stands (`MoveObjectsTool::m_duplicateObjects`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Duplicate {
    /// A plain move.
    #[default]
    No,
    /// The copy is made on the first step that moves.
    Pending,
    /// The copy is made; the history stood at this mark before it.
    Made(usize),
}

/// What a press that never became a drag selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Only this object, or with Ctrl, this object toggled.
    Object {
        node: map_model::node::NodeId,
        toggle: bool,
    },
    /// Only this face, or with Ctrl, this face toggled.
    Face { face: FaceRef, toggle: bool },
    /// Nothing: a click on empty space deselects.
    Nothing,
    /// A Ctrl-click on a handle picks it or takes it out of what is picked.
    UnpickHandle(Vec3d),
}

/// A drag that is stretching a face out of a brush.
#[derive(Debug, Clone)]
pub struct Extruding {
    /// The other face when resizing a corner.
    pub second_polygon: Option<Vec<Vec3d>>,
    /// The face's vertices when the drag began, which is how the same face is found on
    /// every brush that shares it.
    pub polygon: Vec<Vec3d>,
    /// The brush and face the drag began on, for snapping the distance.
    pub brush: Brush,
    pub face: map_model::polyhedron::FaceId,
    pub normal: Vec3d,
    /// Where the handle started, and the surface it slides on.
    pub handle: Vec3d,
    pub picker: HandlePicker,
    pub mode: extrude_tool::ExtrudeMode,
}

/// Draws the edges of every brush in the document.
pub fn draw_animation_brushes(
    state: &ViewportState,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let frame = document.animation_wireframe(filters);
    for (past, offset, edges) in &frame.ghosts {
        let alpha = 0.45 / *offset as f32;
        let color = if *past {
            Color::rgba(0.25, 0.65, 1.0, alpha)
        } else {
            Color::rgba(1.0, 0.55, 0.2, alpha)
        };
        stroke_edges(state, edges, cr, viewport, color, 1.0);
    }
    stroke_edges(
        state,
        &frame.locked,
        cr,
        viewport,
        pref(|p| p.locked_edge_color),
        1.0,
    );
    stroke_edges(
        state,
        &frame.plain,
        cr,
        viewport,
        pref(|p| p.edge_color),
        1.0,
    );
    for (peer, edges) in &frame.peers {
        let [r, g, b] = crate::collab::peer_color_bytes(*peer);
        stroke_edges(state, edges, cr, viewport, Color::bytes(r, g, b), 2.0);
    }
    stroke_edges(
        state,
        &frame.selected,
        cr,
        viewport,
        pref(|p| p.selected_edge_color),
        2.0,
    );
}

pub fn draw_brushes(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    draw_static_brushes(state, document, cr, viewport);
    draw_selected_brushes(state, document, cr, viewport);
}

/// The selected brushes' edges, in the highlight colour.
pub fn draw_selected_brushes(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    stroke_edges(
        state,
        &document.selected_edges(),
        cr,
        viewport,
        pref(|p| {
            if document.has_shape_preview() {
                p.resize_handle_color
            } else {
                p.selected_edge_color
            }
        }),
        2.0,
    );
}

/// The edges of every brush that is not selected here: locked, plain and other people's.
pub fn draw_static_brushes(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // Locked first, then unselected, so the highlight is drawn over anything it overlaps.
    stroke_edges(
        state,
        &document.locked_edges(),
        cr,
        viewport,
        pref(|p| p.locked_edge_color),
        1.0,
    );
    stroke_edges(
        state,
        &document.edges(),
        cr,
        viewport,
        pref(|p| p.edge_color),
        1.0,
    );
    // The other participants' selections in a live session, each in the colour that
    // person draws in everywhere, so it does not read as this editor's. Drawn under the
    // local selection, which wins where they overlap.
    {
        let peer_edges = document.peer_edges();
        let mut peers: Vec<u32> = peer_edges.iter().map(|(peer, _, _)| *peer).collect();
        peers.sort_unstable();
        peers.dedup();
        for peer in peers {
            let owned: Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> = peer_edges
                .iter()
                .filter(|(who, _, _)| *who == peer)
                .map(|(_, from, to)| (*from, *to))
                .collect();
            let [r, g, b] = crate::collab::peer_color_bytes(peer);
            stroke_edges(
                state,
                &owned,
                cr,
                viewport,
                crate::preferences::Color::bytes(r, g, b),
                2.0,
            );
        }
    }
}

/// Keeps the picture of a flat view's bottom layer ([`draw_base`]) between frames.
pub trait BaseKeeper {
    /// Puts the bottom layer down: what it kept, if `key` is the key it was drawn
    /// under, or else what `draw` draws now (keeping that).
    fn base(&mut self, key: u64, draw: &dyn Fn(&mut dyn Canvas));
}

/// Draws each point entity as the box it occupies.
pub fn draw_entities(
    state: &ViewportState,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // Each entity's class, looked up rather than asked of the document per entity, which
    // walked the whole tree twice for every box.
    let classnames: std::collections::HashMap<map_model::node::NodeId, &str> =
        map_model::node::point_entities_by_layer(document.tree_world())
            .into_iter()
            .filter_map(|(id, entity, _)| entity.classname().map(|name| (id, name)))
            .collect();
    for entity in document.entity_boxes() {
        let min = state.camera.world_to_screen(entity.bounds.min, viewport);
        let max = state.camera.world_to_screen(entity.bounds.max, viewport);

        if entity.selected {
            set_colour(cr, pref(|p| p.selected_edge_color));
            cr.set_line_width(2.0);
        } else {
            let [r, g, b] = entity.color;
            cr.set_source_rgb(
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
            );
            cr.set_line_width(1.0);
        }

        cr.rectangle(
            min.x().min(max.x()),
            min.y().min(max.y()),
            (max.x() - min.x()).abs(),
            (max.y() - min.y()).abs(),
        );
        cr.stroke();

        if let Some(classname) = classnames.get(&entity.node) {
            crate::entity_icons::draw(
                cr,
                classname,
                (min.x() + max.x()) * 0.5 - 10.,
                (min.y() + max.y()) * 0.5 - 10.,
                20.,
            );
        }

        // `ViewEditor`'s "Show entity classnames": with nothing but a box to go on, the
        // class is the only thing saying what an entity is.
        if filters.show_entity_classnames {
            if let Some(classname) = classnames.get(&entity.node) {
                cr.move_to(min.x().min(max.x()), min.y().min(max.y()) - 3.0);
                draw_text(cr, classname, 10.0);
            }
        }
    }
}

/// Draws the outline of every group, so what belongs together looks like it does.
pub fn draw_group_bounds(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    for group in document.group_bounds() {
        let min = state.camera.world_to_screen(group.bounds.min, viewport);
        let max = state.camera.world_to_screen(group.bounds.max, viewport);

        if group.selected {
            set_colour(cr, pref(|p| p.selected_edge_color).with_alpha(0.8));
        } else if group.linked {
            set_colour(cr, pref(|p| p.linked_group_color).with_alpha(0.5));
        } else {
            set_colour(cr, pref(|p| p.group_color).with_alpha(0.5));
        }
        cr.set_line_width(1.0);
        // Dashed, so a group's outline is not mistaken for a brush's edge.
        cr.set_dash(&[4.0, 4.0], 0.0);
        cr.rectangle(
            min.x().min(max.x()),
            min.y().min(max.y()),
            (max.x() - min.x()).abs(),
            (max.y() - min.y()).abs(),
        );
        cr.stroke();
        cr.set_dash(&[], 0.0);
    }
}

/// Draws a dashed box around each brush entity's brushes.
///
/// `Preferences::ShowBrushEntityBounds`: a brush entity's brushes look like world geometry
/// otherwise, and which brushes belong to which door is worth being able to see.
pub fn draw_brush_entity_bounds(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    for (bounds, selected) in document.brush_entity_bounds() {
        let min = state.camera.world_to_screen(bounds.min, viewport);
        let max = state.camera.world_to_screen(bounds.max, viewport);

        if selected {
            set_colour(cr, pref(|p| p.selected_edge_color).with_alpha(0.6));
        } else {
            set_colour(cr, pref(|p| p.edge_color).with_alpha(0.35));
        }
        cr.set_line_width(1.0);
        cr.set_dash(&[2.0, 3.0], 0.0);
        cr.rectangle(
            min.x().min(max.x()),
            min.y().min(max.y()),
            (max.x() - min.x()).abs(),
            (max.y() - min.y()).abs(),
        );
        cr.stroke();
        cr.set_dash(&[], 0.0);
    }
}

/// Draws the portals a compiler cut the world into.
///
/// Draws loaded portal polygons as outlines in flat views.
pub fn draw_portals(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let Some(portals) = document.portals() else {
        return;
    };

    set_colour(cr, pref(|p| p.portal_file_color));
    cr.set_line_width(1.0);
    for polygon in &portals.polygons {
        for (index, point) in polygon.iter().enumerate() {
            let on_screen = state
                .camera
                .plane_to_screen(state.camera.plane.project(*point), viewport);
            if index == 0 {
                cr.move_to(on_screen.x(), on_screen.y());
            } else {
                cr.line_to(on_screen.x(), on_screen.y());
            }
        }
        cr.close_path();
    }
    cr.stroke();
}

/// Draws the way out of the level the last leak check found.
///
/// `MapViewBase::renderPointFile` draws the compiler's `.pts` file in the same colour;
/// there is no compiler here, so this is the editor's own trace.
pub fn draw_leak_trace(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let Some(trace) = document.leak_trace() else {
        return;
    };
    if trace.points.len() < 2 {
        return;
    }

    set_colour(cr, pref(|p| p.point_file_color));
    cr.set_line_width(2.0);
    for (index, point) in trace.points.iter().enumerate() {
        let on_screen = state
            .camera
            .plane_to_screen(state.camera.plane.project(*point), viewport);
        if index == 0 {
            cr.move_to(on_screen.x(), on_screen.y());
        } else {
            cr.line_to(on_screen.x(), on_screen.y());
        }
    }
    cr.stroke();
}

/// Draws the box the whole selection fills.
///
/// `Preferences::SelectionBoundsColor`. It is what says how large a selection is at a
/// glance, and what a scale or a rotation is about to work on.
pub fn draw_selection_bounds(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    // One brush is its own outline; the box only says something once there is more than
    // one thing in the selection.
    if document.selection_count() < 2 {
        return;
    }
    let Some(bounds) = document.selection_bounds() else {
        return;
    };

    let min = state.camera.world_to_screen(bounds.min, viewport);
    let max = state.camera.world_to_screen(bounds.max, viewport);

    set_colour(cr, pref(|p| p.selection_bounds_color));
    cr.set_line_width(1.0);
    cr.rectangle(
        min.x().min(max.x()),
        min.y().min(max.y()),
        (max.x() - min.x()).abs(),
        (max.y() - min.y()).abs(),
    );
    cr.stroke();
}

/// Draws a line from the linked group being edited to each of its other copies.
///
/// `GroupLinkRenderer`, in `Preferences::LinkedGroupColor`. It is what says where the
/// copies of what is under the pointer are, when they are off the edge of the view.
pub fn draw_linked_group_links(
    state: &ViewportState,
    document: &Document,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    let links = document.linked_group_links();
    if links.is_empty() {
        return;
    }
    set_colour(cr, pref(|p| p.linked_group_color));
    cr.set_line_width(1.0);
    for (from, to) in links {
        let from = state.camera.world_to_screen(from, viewport);
        let to = state.camera.world_to_screen(to, viewport);
        cr.move_to(from.x(), from.y());
        cr.line_to(to.x(), to.y());
    }
    cr.stroke();
}

/// What a link is drawn in: `EntityLinkRenderer`'s two colours, and a third for a link
/// that removes its target (`killtarget`).
pub fn link_colour(link: &crate::view_filters::EntityLink) -> Color {
    if link.kills() {
        pref(|p| p.kill_entity_link_color)
    } else if link.touches_selection {
        pref(|p| p.selected_entity_link_color)
    } else {
        pref(|p| p.entity_link_color)
    }
}

/// Draws a line from each entity to whatever it targets.
pub fn draw_entity_links(
    state: &ViewportState,
    document: &Document,
    filters: &ViewFilters,
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
) {
    if filters.entity_links == EntityLinkMode::None {
        return;
    }

    cr.set_line_width(1.0);
    for link in document.entity_links() {
        if !filters.entity_links.shows(&link) {
            continue;
        }

        let from = state.camera.world_to_screen(link.from, viewport);
        let to = state.camera.world_to_screen(link.to, viewport);

        // `addLink`: a link with a selected end is drawn in the other colour, since that
        // is the one being looked at.
        set_colour(cr, link_colour(&link));
        cr.move_to(from.x(), from.y());
        cr.line_to(to.x(), to.y());
        let delta = to - from;
        let length = (delta.x() * delta.x() + delta.y() * delta.y()).sqrt();
        if length > 12.0 {
            let u = delta / length;
            let side = Vec2d::new(-u.y(), u.x());
            for sign in [-1.0, 1.0] {
                let tip = to - u * 9.0 + side * (4.0 * sign);
                cr.move_to(to.x(), to.y());
                cr.line_to(tip.x(), tip.y());
            }
        }
        cr.stroke();
        // What the link does, by the links that touch the selection only: written by
        // every link, a map with many reads as a wall of labels.
        if link.touches_selection && length > 24.0 {
            let middle = (from + to) / 2.0;
            draw_label(
                cr,
                &link.label(),
                middle,
                crate::size_labels::Alignment::Top,
                2.0,
                LabelStyle::Weak,
            );
            cr.set_line_width(1.0);
        }
    }
}

/// Strokes a set of world-space edges in the given colour.
pub fn stroke_edges(
    state: &ViewportState,
    edges: &[(Vec3d, Vec3d)],
    cr: &mut dyn Canvas,
    viewport: (f64, f64),
    colour: Color,
    width: f64,
) {
    if edges.is_empty() {
        return;
    }

    set_colour(cr, colour);
    cr.set_line_width(width);

    let mut segments = edges.iter().filter_map(|(start, end)| {
        let from = state.camera.world_to_screen(*start, viewport);
        let to = state.camera.world_to_screen(*end, viewport);
        // An edge running along the view axis projects to a point; skip it rather than
        // asking cairo to stroke a zero-length line.
        if (from - to) == Vec2d::zero() {
            return None;
        }
        // Nor one wholly off one side of the view: zoomed in on a big map, almost every
        // edge is, and each still cost the backend a path and a stroke.
        let margin = width + 2.0;
        if (from.x() < -margin && to.x() < -margin)
            || (from.y() < -margin && to.y() < -margin)
            || (from.x() > viewport.0 + margin && to.x() > viewport.0 + margin)
            || (from.y() > viewport.1 + margin && to.y() > viewport.1 + margin)
        {
            return None;
        }
        Some((from, to))
    });
    #[cfg(feature = "test-support")]
    let legacy = crate::bench_support::legacy_line_paths();
    #[cfg(not(feature = "test-support"))]
    let legacy = false;
    if legacy {
        for (from, to) in segments {
            cr.move_to(from.x(), from.y());
            cr.line_to(to.x(), to.y());
        }
        cr.stroke();
    } else {
        cr.stroke_segments(&mut segments);
    }
}

/// Draws the edge of the editable world as a rectangle.
///
/// The world is a cube, so seen down any of the three axes it is a square; drawing its
/// projected corners rather than an arbitrary four of its eight vertices is what keeps it
/// from collapsing into a diagonal.
pub fn draw_world_bounds(state: &ViewportState, cr: &mut dyn Canvas, viewport: (f64, f64)) {
    let e = state.bounds.half_extent();
    let corners = [
        Vec2d::new(-e, -e),
        Vec2d::new(e, -e),
        Vec2d::new(e, e),
        Vec2d::new(-e, e),
    ];

    let screen: Vec<Vec2d> = corners
        .iter()
        .map(|c| state.camera.plane_to_screen(*c, viewport))
        .collect();

    set_colour(cr, pref(|p| p.soft_map_bounds_color).with_alpha(0.6));
    cr.set_line_width(1.0);
    cr.move_to(screen[0].x(), screen[0].y());
    for p in &screen[1..] {
        cr.line_to(p.x(), p.y());
    }
    cr.close_path();
    cr.stroke();
}

/// The distance in pixels a press has to travel before it counts as a drag.
pub const DRAG_THRESHOLD: f64 = 3.0;

pub use crate::input::Modifiers;

/// The primary-button drag of a flat view: select, move, lasso, draw a brush, and every
/// tool's own drag (clip points, handles, rotate, scale, shear, extrude, sweep caps).
///
/// The frontend forwards its drag gesture: [`Self::begin`] at the press, [`Self::update`]
/// with the offset from the press, [`Self::end`] at the release, [`Self::cancel`] if the
/// gesture is taken away, and Escape to [`Self::escape`].
pub struct SelectAndMove {
    pub state: Rc<RefCell<ViewportState>>,
    pub grid: Rc<RefCell<Grid>>,
    pub document: Rc<RefCell<Document>>,
    pub shape_parameters: Rc<RefCell<ShapeParameters>>,
    pub tool: Rc<RefCell<Tool>>,
    pub clip: Rc<RefCell<ClipTool>>,
    pub handles: Rc<RefCell<HandleManager>>,
    pub rotate: Rc<RefCell<RotateTool>>,
    pub fillet: Rc<RefCell<crate::fillet_tool::FilletParameters>>,
    pub pending_shape: Rc<RefCell<Option<Bbox3d>>>,
    /// Redraws every view, the 3D one included.
    pub redraw_everywhere: Rc<dyn Fn()>,
    pub sweep: Rc<RefCell<crate::sweep_tool::SweepInputs>>,
    /// Asks the side panels to catch up.
    pub refresh_panels: Rc<dyn Fn()>,
    /// Redraws the flat views.
    pub redraw: Rc<dyn Fn()>,
    /// Gives the view the keyboard focus, so the grid and nudge keys reach it.
    pub focus: Rc<dyn Fn()>,
    /// What the drag started from, so each update can be expressed as an absolute offset
    /// from the press rather than accumulated from the previous update. Accumulating would
    /// let the grid snapping round away a little of every step.
    pub drag: Rc<RefCell<DragState>>,
    /// The selection when the press happened, for a lasso to preview against.
    pub marquee: Rc<RefCell<Option<crate::document::SelectionSnapshot>>>,
}

impl SelectAndMove {
    /// The press, at `(x, y)` in view pixels.
    pub fn begin(&self, x: f64, y: f64, modifiers: Modifiers, viewport: (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            state,
            grid,
            document,
            shape_parameters,
            tool,
            clip,
            handles,
            rotate,
            fillet,
            pending_shape,
            redraw_everywhere,
            sweep,
            refresh_panels,
            redraw,
            focus,
            drag,
            marquee,
        } = self;

        *marquee.borrow_mut() = Some(document.borrow().selection_snapshot());
        // Take the keyboard off any number field the pointer was last in, so the
        // grid and nudge keys reach the window again.
        focus();

        // Painting has its own stroke: the paint gesture handles it, and a
        // press here must not also select or start moving the brush under it.
        if *tool.borrow() == Tool::Paint {
            *drag.borrow_mut() = DragState {
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                mode: DragMode::Idle,
                ..DragState::default()
            };
            return;
        }

        // The clip tool places and drags its points with plain presses; with a modifier
        // the press is a selection click (`ClipToolController`).
        if *tool.borrow() == Tool::Clip && (modifiers.shift || modifiers.control || modifiers.alt) {
            *drag.borrow_mut() = DragState {
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                mode: DragMode::Idle,
                click: self.selection_click(x, y, modifiers, viewport),
                ..DragState::default()
            };
            return;
        }
        if *tool.borrow() == Tool::Clip {
            let mode = begin_clip(
                &state.borrow(),
                &grid.borrow(),
                &document.borrow(),
                &mut clip.borrow_mut(),
                viewport,
                Vec2d::new(x, y),
            );
            *drag.borrow_mut() = DragState {
                narrow_to: None,
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                applied: Vec3d::zero(),
                vertex_stacks_expanded: false,
                mode,
                scaling: None,
                shearing: None,
                extruding: None,
                splitting: None,
                click: None,
                duplicate: Duplicate::No,
            };
            redraw();
            return;
        }

        if *tool.borrow() == Tool::Rotate {
            let mut rotate = rotate.borrow_mut();
            if rotate.center().is_none() {
                rotate.reset_center(&document.borrow(), &grid.borrow());
            }
            let mode = match rotate.center() {
                None => DragMode::Idle,
                Some(center) => {
                    let camera = state.borrow().camera;
                    let on_screen = camera.plane_to_screen(camera.plane.project(center), viewport);
                    let reach = vec::distance(on_screen, Vec2d::new(x, y));
                    let plain = !modifiers.shift && !modifiers.control && !modifiers.alt;
                    if plain && reach <= handle_radius() + 2.0 {
                        DragMode::MovingRotationCenter
                    } else if plain && reach <= rotate_ring_radius() + handle_radius() + 2.0 {
                        // On or inside the ring turns the selection
                        // (`RotateHandleController` takes a plain drag on its handle).
                        document.borrow_mut().begin_transaction();
                        state.borrow_mut().rotating = true;
                        DragMode::Rotating
                    } else {
                        DragMode::Idle
                    }
                }
            };
            drop(rotate);
            // A press away from the ring is a selection click.
            let click = if mode == DragMode::Idle {
                self.selection_click(x, y, modifiers, viewport)
            } else {
                None
            };
            *drag.borrow_mut() = DragState {
                narrow_to: None,
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                applied: Vec3d::zero(),
                vertex_stacks_expanded: false,
                mode,
                scaling: None,
                shearing: None,
                extruding: None,
                splitting: None,
                click,
                duplicate: Duplicate::No,
            };
            redraw();
            return;
        }

        // The sweep tool's destination cap: a press on it drags it, a press anywhere
        // else does nothing (the faces to sweep are chosen with Alt-click).
        if *tool.borrow() == Tool::Sweep {
            let mode = sweep_cap_under_cursor(
                &state.borrow(),
                &document.borrow(),
                &sweep.borrow(),
                viewport,
                Vec2d::new(x, y),
            )
            .map_or(DragMode::Idle, |()| DragMode::MovingSweepCap);
            // Away from the cap a plain or Ctrl press is a selection click; Alt picks
            // the faces to sweep.
            let click = (mode == DragMode::Idle && !modifiers.alt)
                .then(|| self.selection_click(x, y, modifiers, viewport))
                .flatten();
            *drag.borrow_mut() = DragState {
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                mode,
                click,
                ..DragState::default()
            };
            redraw();
            return;
        }

        // Shift extrudes with the Select or Brush tool. A handle tool splits vertices with
        // it, and Rotate, Scale, Shear, Sweep and Clip switch extruding off
        // (`MapViewToolBox`), Scale for its own proportional Shift.
        if modifiers.shift && matches!(*tool.borrow(), Tool::Select | Tool::Brush) {
            let camera = state.borrow().camera;
            let ray = camera.ray_through(Vec2d::new(x, y), viewport, PICK_REACH);
            let started = {
                let document = document.borrow();
                // `ExtrudeTool::pick2D`: over a selected brush only an edge within the
                // handle radius is taken; outside it, the nearest one at any distance.
                let inside = document.pick_selected_flat(&ray).is_some();
                begin_extrude_2d(
                    &document,
                    &ray,
                    camera.direction(),
                    modifiers.control,
                    modifiers.alt,
                )
                .filter(|extruding| {
                    !inside
                        || vec::distance(
                            camera.world_to_screen(extruding.handle, viewport),
                            Vec2d::new(x, y),
                        ) <= handle_radius() + 2.0
                })
            };
            if let Some(extruding) = started {
                document.borrow_mut().begin_transaction();
                // A Shift-click that never moves still selects the face under it, but
                // only on release: selecting it now would drop the brushes the drag pulls.
                let click = document
                    .borrow()
                    .pick_flat(&ray)
                    .and_then(|hit| {
                        hit.face.map(|face| FaceRef {
                            node: hit.node,
                            face,
                        })
                    })
                    .map(|face| Click::Face {
                        face,
                        toggle: modifiers.control,
                    });
                *drag.borrow_mut() = DragState {
                    narrow_to: None,
                    origin: Vec2d::new(x, y),
                    current: Vec2d::new(x, y),
                    applied: Vec3d::zero(),
                    vertex_stacks_expanded: false,
                    mode: DragMode::Extruding,
                    scaling: None,
                    shearing: None,
                    extruding: Some(extruding),
                    splitting: None,
                    click,
                    duplicate: Duplicate::No,
                };
                redraw();
                return;
            }
        }

        if *tool.borrow() == Tool::Shear {
            let camera = state.borrow().camera;
            let caught = document.borrow().selection_bounds().and_then(|bounds| {
                let cursor = Vec2d::new(x, y);
                shear_tool::handles(&bounds)
                    .into_iter()
                    .map(|(normal, center)| {
                        let on_screen =
                            camera.plane_to_screen(camera.plane.project(center), viewport);
                        (normal, center, vec::squared_distance(on_screen, cursor))
                    })
                    .filter(|(_, _, distance)| *distance <= (handle_radius() + 2.0).powi(2))
                    .min_by(|(_, _, a), (_, _, b)| {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(normal, center, _)| (normal, center, bounds))
            });

            if caught.is_some() {
                document.borrow_mut().begin_transaction();
            }
            *drag.borrow_mut() = DragState {
                narrow_to: None,
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                applied: Vec3d::zero(),
                vertex_stacks_expanded: false,
                mode: if caught.is_some() {
                    DragMode::Shearing
                } else {
                    DragMode::Idle
                },
                scaling: None,
                shearing: caught,
                extruding: None,
                splitting: None,
                click: caught
                    .is_none()
                    .then(|| self.selection_click(x, y, modifiers, viewport))
                    .flatten(),
                duplicate: Duplicate::No,
            };
            redraw();
            return;
        }

        if *tool.borrow() == Tool::Scale {
            let box_ = document.borrow().selection_bounds();
            let picked = box_.and_then(|box_| {
                pick_scale_handle(&state.borrow(), &box_, viewport, Vec2d::new(x, y))
                    .map(|handle| (handle, box_))
            });
            if picked.is_some() {
                // One drag is one step in the history, however many edits it makes.
                document.borrow_mut().begin_transaction();
            }
            state.borrow_mut().scaling = picked.map(|(handle, _)| handle);
            *drag.borrow_mut() = DragState {
                narrow_to: None,
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                applied: Vec3d::zero(),
                vertex_stacks_expanded: false,
                mode: if picked.is_some() {
                    DragMode::Scaling
                } else {
                    DragMode::Idle
                },
                scaling: picked,
                shearing: None,
                extruding: None,
                splitting: None,
                click: picked
                    .is_none()
                    .then(|| self.selection_click(x, y, modifiers, viewport))
                    .flatten(),
                duplicate: Duplicate::No,
            };
            redraw();
            return;
        }

        if let Some(kind) = HandleKind::of(*tool.borrow()) {
            // Ctrl adds to what is picked, as it does for objects.
            let add_to_selection = modifiers.control;
            let mut handles = handles.borrow_mut();
            handles.collect(kind, document.borrow().selected_brushes().into_iter());

            // Fillet pins a preview edge without dragging or editing the source.
            if *tool.borrow() == Tool::Fillet {
                match pick_handle(&state.borrow(), &handles, viewport, Vec2d::new(x, y)) {
                    Some(position) => {
                        let outcome = crate::fillet_tool::press(
                            document,
                            &mut handles,
                            *fillet.borrow(),
                            position,
                            add_to_selection,
                        );
                        crate::fillet_tool::report(&outcome);
                    }
                    None => {
                        let ray = state.borrow().camera.ray_through(
                            Vec2d::new(x, y),
                            viewport,
                            PICK_REACH,
                        );
                        let hit = document.borrow().pick_flat(&ray).map(|hit| hit.node);
                        crate::fillet_tool::select_brush(
                            &mut document.borrow_mut(),
                            &mut handles,
                            hit,
                            add_to_selection,
                        );
                    }
                }
                drop(handles);
                *drag.borrow_mut() = DragState {
                    origin: Vec2d::new(x, y),
                    current: Vec2d::new(x, y),
                    mode: DragMode::Idle,
                    ..DragState::default()
                };
                redraw();
                refresh_panels();
                return;
            }

            let mut splitting = None;
            let mut grabbed = None;
            let picked_one = handles.selected_count() == 1;
            let had_picked = handles.selected_count() > 0;
            // `NodeHandleToolControllerParts`: a Ctrl-click on a picked handle unpicks it,
            // on release, so a Ctrl-drag from it still moves what is picked.
            let under = if add_to_selection && !modifiers.shift {
                pick_handle(&state.borrow(), &handles, viewport, Vec2d::new(x, y))
            } else {
                None
            };
            // The edge and face tools do not move a handle with Ctrl held, so a Ctrl-drag
            // from one paints the selection (`NodeHandleToolMovePartBase::shouldStartMove`).
            if let Some(position) =
                under.filter(|_| matches!(*tool.borrow(), Tool::Edge | Tool::Face))
            {
                drop(handles);
                *drag.borrow_mut() = DragState {
                    origin: Vec2d::new(x, y),
                    current: Vec2d::new(x, y),
                    mode: DragMode::PaintingSelection,
                    click: Some(Click::UnpickHandle(position)),
                    ..DragState::default()
                };
                return;
            }
            let unpick = under.filter(|position| handles.is_selected(*position));
            let unpicked = under.filter(|position| !handles.is_selected(*position));
            let mode = begin_handle_drag(
                &state.borrow(),
                &mut handles,
                document,
                viewport,
                Vec2d::new(x, y),
                add_to_selection,
                // Splitting is the vertex tool's, and Shift is what asks for it --
                // unless Alt is held as well, which asks for a snap instead.
                *tool.borrow() == Tool::Vertex && modifiers.shift && !modifiers.alt,
                *tool.borrow() == Tool::Vertex && modifiers.shift && modifiers.alt && picked_one,
                &mut splitting,
                &mut grabbed,
            );
            drop(handles);
            if matches!(mode, DragMode::MovingHandle | DragMode::SplittingHandle) {
                document.borrow_mut().begin_transaction();
            }
            // Which handle the guide comes out of, once there is a drag to guide.
            state.borrow_mut().dragging_handle = match mode {
                DragMode::MovingHandle => grabbed,
                DragMode::SplittingHandle => splitting,
                _ => None,
            };
            *drag.borrow_mut() = DragState {
                origin: Vec2d::new(x, y),
                current: Vec2d::new(x, y),
                applied: Vec3d::zero(),
                vertex_stacks_expanded: false,
                mode,
                scaling: None,
                shearing: None,
                extruding: None,
                splitting,
                // A click on no handle with none picked falls through to selecting the
                // brush under it (`deselectAll` otherwise, which the press has done).
                click: match unpick {
                    Some(position) => Some(Click::UnpickHandle(position)),
                    None if mode == DragMode::LassoingHandles && !had_picked => {
                        self.selection_click(x, y, modifiers, viewport)
                    }
                    None => None,
                },
                duplicate: Duplicate::No,
                narrow_to: unpicked.filter(|_| mode == DragMode::MovingHandle),
            };
            redraw();
            // Which handles are picked is what the fillet panel works from,
            // so it hears about a press the way the sweep panel hears about a
            // dragged cap.
            refresh_panels();
            return;
        }

        let ray = state
            .borrow()
            .camera
            .ray_through(Vec2d::new(x, y), viewport, PICK_REACH);

        let mut document = document.borrow_mut();
        // In an orthographic view, depth must not prevent dragging the
        // selected geometry. Modified clicks still pick the nearest object.
        // Shift is about the face under the pointer; otherwise what is selected is taken
        // first even behind something that is not, as `transitivelySelected` does.
        let hit = if modifiers.shift {
            document.pick_flat(&ray)
        } else {
            document
                .pick_selected_flat(&ray)
                .or_else(|| document.pick_flat(&ray))
        };
        // Whether anything was selected before the click below clears it: a drag
        // from empty space draws a brush, but not on the same drag that deselects.
        let had_selection = document.has_selection();
        // `SelectionTool`: Shift picks the face under the pointer rather than the
        // whole brush (`isFaceClick`), and Ctrl adds to the selection
        // (`isMultiClick`) -- so Shift+Ctrl picks several faces.
        let add_to_selection = modifiers.control;
        let pick_face = modifiers.shift;

        // `SelectionTool::mouseClick`: what a press selects is decided on release, if it
        // stays a click, so the press is free to start a drag instead.
        // `MoveObjectsToolController` moves only what is already selected, and a drag over
        // anything else draws a brush (`DrawShapeToolController2D`), square with Shift.
        let click = match hit {
            // Shift picks a face, which only a brush has.
            Some(hit) if pick_face && hit.face.is_some() => Some(Click::Face {
                face: FaceRef {
                    node: hit.node,
                    face: hit.face.expect("checked just above"),
                },
                toggle: add_to_selection,
            }),
            Some(hit) => Some(Click::Object {
                node: document
                    .pick_flat(&ray)
                    .map_or(hit.node, |nearest| nearest.node),
                toggle: add_to_selection,
            }),
            None => None,
        };
        // `DrawShapeToolController2D` draws only while nothing is selected; with a
        // selection, a press elsewhere is a click, deselecting or selecting on release.
        let draws = !had_selection || *tool.borrow() == Tool::Brush;
        let click = match click {
            None if !add_to_selection && !draws => Some(Click::Nothing),
            other => other,
        };
        if hit.is_none() && !add_to_selection && draws {
            document.clear_selection();
        }
        // `MoveObjectsToolController`: a press on something selected moves it, and with
        // Ctrl moves a copy of it (`MoveObjectsTool::duplicateObjects`).
        let grabbed_selected =
            hit.is_some_and(|hit| !modifiers.shift && document.is_selected(hit.node));
        let duplicate = grabbed_selected && modifiers.control;

        *drag.borrow_mut() = DragState {
            narrow_to: None,
            origin: Vec2d::new(x, y),
            current: Vec2d::new(x, y),
            applied: Vec3d::zero(),
            vertex_stacks_expanded: false,
            scaling: None,
            shearing: None,
            extruding: None,
            splitting: None,
            mode: match hit {
                Some(_) if grabbed_selected => {
                    // The whole drag is one step in the history. Without this every
                    // motion event recorded its own, and a drag across the window
                    // left a hundred of them to undo one at a time. A Ctrl-drag opens
                    // it once the copy is made.
                    if !duplicate {
                        document.begin_transaction();
                    }
                    DragMode::Moving
                }
                // Ctrl toggles on release, and a Ctrl-drag paints the selection.
                Some(_) if add_to_selection && pick_face => DragMode::PaintingFaces,
                Some(_) if add_to_selection => DragMode::PaintingSelection,
                Some(_) if !draws => DragMode::Idle,
                Some(_) => DragMode::Creating,
                None => empty_space_drag_mode(had_selection, add_to_selection, *tool.borrow()),
            },
            click,
            duplicate: if duplicate {
                Duplicate::Pending
            } else {
                Duplicate::No
            },
        };
        state.borrow_mut().moving = drag.borrow().mode == DragMode::Moving;

        redraw();
    }
    /// What a press that missed a tool's handles selects if it stays a click: the chain
    /// falls through to `SelectionTool`, which only clicks while these tools are in hand
    /// (`MapViewToolBox` switches moving and drawing off).
    fn selection_click(
        &self,
        x: f64,
        y: f64,
        modifiers: Modifiers,
        viewport: (f64, f64),
    ) -> Option<Click> {
        let ray = self
            .state
            .borrow()
            .camera
            .ray_through(Vec2d::new(x, y), viewport, PICK_REACH);
        let hit = self.document.borrow().pick_flat(&ray);
        match hit {
            Some(hit) if modifiers.shift => hit.face.map(|face| Click::Face {
                face: FaceRef {
                    node: hit.node,
                    face,
                },
                toggle: modifiers.control,
            }),
            Some(hit) => Some(Click::Object {
                node: hit.node,
                toggle: modifiers.control,
            }),
            None if modifiers.control => None,
            None => Some(Click::Nothing),
        }
    }

    /// The pointer moved; `(dx, dy)` is the offset from the press.
    pub fn update(&self, dx: f64, dy: f64, modifiers: Modifiers, viewport: (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            state,
            grid,
            document,
            shape_parameters,
            tool,
            clip,
            handles,
            rotate,
            fillet,
            pending_shape,
            redraw_everywhere,
            sweep,
            refresh_panels,
            redraw,
            focus,
            drag,
            marquee,
        } = self;

        let mut drag = drag.borrow_mut();
        drag.current = drag.origin + Vec2d::new(dx, dy);

        // `InputEventRecorder`: a press becomes a drag only once the pointer has moved
        // off the spot, so a click on a handle picks it rather than moving it by
        // whatever the grid rounded the press point to.
        if drag.mode == DragMode::MovingHandle
            && drag.applied == Vec3d::zero()
            && vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD
        {
            return;
        }

        if drag.mode == DragMode::ClippingPoint {
            let state = state.borrow();
            if let Some(position) = clip_point_under_cursor(
                &state,
                &grid.borrow(),
                &document.borrow(),
                viewport,
                drag.current,
            ) {
                let help_vectors = vec![state.camera.direction()];
                clip.borrow_mut().drag_point(position, help_vectors);
            }
            drop(state);
            redraw();
            return;
        }

        if drag.mode == DragMode::MovingRotationCenter {
            let state_ref = state.borrow();
            // The centre keeps its depth; only the two axes the view shows can move.
            let center = rotate.borrow().center();
            if let Some(center) = center {
                let camera = state_ref.camera;
                let on_plane = camera.screen_to_plane(drag.current, viewport);
                let depth = center[camera.plane.depth_axis()];
                let moved = grid
                    .borrow()
                    .snap_point(camera.plane.unproject(on_plane, depth));
                drop(state_ref);
                rotate.borrow_mut().set_center(moved);
                drop(drag);
                redraw();
            }
            return;
        }

        if drag.mode == DragMode::MovingSweepCap {
            let moved = sweep_cap_translation(
                &state.borrow(),
                &document.borrow(),
                &grid.borrow(),
                viewport,
                drag.current,
                sweep.borrow().transform.translation,
            );
            if let Some(translation) = moved {
                if sweep.borrow().transform.translation != translation {
                    sweep.borrow_mut().transform.translation = translation;
                    drop(drag);
                    refresh_panels();
                    redraw();
                }
            }
            return;
        }

        if drag.mode == DragMode::Rotating {
            let camera = state.borrow().camera;
            let mut rotate = rotate.borrow_mut();
            let Some(center) = rotate.center() else {
                return;
            };

            // Both ends of the drag are put on the plane the ring lies in, which is
            // the one the view draws; the axis is what the view looks along.
            let depth = center[camera.plane.depth_axis()];
            let from = camera
                .plane
                .unproject(camera.screen_to_plane(drag.origin, viewport), depth);
            let to = camera
                .plane
                .unproject(camera.screen_to_plane(drag.current, viewport), depth);
            let axis = -camera.direction();

            let Some(angle) = rotate.measure(&grid.borrow(), axis, from, to) else {
                return;
            };
            drop(rotate);

            // `RotateTool::applyRotation`: roll back and turn the original again, so
            // the result depends only on where the pointer is and not on how it got
            // there.
            let mut document = document.borrow_mut();
            document.rollback_transaction();
            document.rotate_selection_about(center, axis, angle.to_degrees());
            drop(document);
            drop(drag);
            redraw();
            return;
        }

        if drag.mode == DragMode::Extruding {
            let Some(extruding) = drag.extruding.clone() else {
                return;
            };
            let camera = state.borrow().camera;
            let ray = camera.ray_through(drag.current, viewport, PICK_REACH);

            let Some(proposed) = extruding.picker.pick(&ray) else {
                return;
            };
            let grid = *grid.borrow();
            let snapped = if extruding.second_polygon.is_some() {
                extruding.handle + grid.snap_point(proposed - extruding.handle)
            } else {
                extrude_tool::snap_handle_position(
                    &grid,
                    &extruding.brush,
                    extruding.face,
                    extruding.normal,
                    extruding.handle,
                    proposed,
                )
            };
            let delta = snapped - extruding.handle;

            // Every step starts again from the brushes as they were.
            let mut document = document.borrow_mut();
            document.rollback_transaction();
            match extruding.mode {
                extrude_tool::ExtrudeMode::Extrude => {
                    if let Some(second) = &extruding.second_polygon {
                        document.extrude_face_planes(&[
                            (extruding.polygon.clone(), delta),
                            (second.clone(), delta),
                        ])
                    } else {
                        document.extrude_faces(&extruding.polygon, delta)
                    }
                }
                extrude_tool::ExtrudeMode::Slide => document.slide_faces(&extruding.polygon, delta),
                extrude_tool::ExtrudeMode::Split => document.split_faces(&extruding.polygon, delta),
                extrude_tool::ExtrudeMode::Stamp => document.stamp_faces(&extruding.polygon, delta),
            };
            drop(document);
            drop(drag);
            redraw();
            return;
        }

        if drag.mode == DragMode::Shearing {
            let Some((normal, side_center, start_bounds)) = drag.shearing else {
                return;
            };
            let camera = state.borrow().camera;

            // The face can only slide across the screen, which is what the picker
            // works out from the view direction.
            let Some(picker) = shear_tool::picker(
                normal,
                side_center,
                Vec3d::zero(),
                false,
                camera.direction(),
                false,
            ) else {
                return;
            };

            let ray = camera.ray_through(drag.current, viewport, PICK_REACH);
            let Some(proposed) = picker.pick(&ray) else {
                return;
            };
            let delta = grid.borrow().snap_point(proposed - side_center);

            let mut document = document.borrow_mut();
            document.rollback_transaction();
            if let Some(transform) =
                geometry_math::mat_ext::shear_bbox_matrix(&start_bounds, normal, delta)
            {
                document.transform_selection(&transform);
            }
            drop(document);
            drop(drag);
            redraw();
            return;
        }

        if drag.mode == DragMode::Scaling {
            let Some((handle, start_box)) = drag.scaling else {
                return;
            };
            let camera = state.borrow().camera;

            let from = camera.screen_to_plane(drag.origin, viewport);
            let to = camera.screen_to_plane(drag.current, viewport);
            let asked = camera.plane.unproject(to - from, 0.0);

            // The handle is snapped where it lands, so a dragged side comes to rest on
            // a grid line whatever the box started at.
            let grid = *grid.borrow();
            let anchor = handle.position(&start_box);
            let delta = grid.snap_point(anchor + asked) - anchor;

            let flags = modifiers;
            // Shift scales the other axes alike, except the one this view looks along
            // (`ScaleToolController`'s special case for 2D).
            let proportional = if flags.shift {
                let depth = state.borrow().camera.plane.depth_axis();
                ProportionalAxes::new(depth != 0, depth != 1, depth != 2)
            } else {
                ProportionalAxes::none()
            };
            let anchor_pos = if flags.alt {
                AnchorPos::Center
            } else {
                AnchorPos::Opposite
            };

            let Some(new_box) = handle.drag(&start_box, delta, proportional, anchor_pos) else {
                return;
            };

            // Roll back and scale the original box again, rather than scaling what
            // the last step left, so the steps do not compound.
            let mut document = document.borrow_mut();
            document.rollback_transaction();
            let transform = geometry_math::mat_ext::scale_bbox_matrix(&start_box, &new_box);
            document.transform_selection(&transform);
            drop(document);
            drop(drag);
            redraw();
            return;
        }

        if drag.mode == DragMode::MovingHandle {
            let camera = state.borrow().camera;

            // The pointer's travel, in world units, the same way a move drag measures
            // it: a 2D view can only move things in the two axes it shows.
            let from = camera.screen_to_plane(drag.origin, viewport);
            let to = camera.screen_to_plane(drag.current, viewport);
            let asked = camera.plane.unproject(to - from, 0.0);

            let grid = *grid.borrow();
            let mut handles = handles.borrow_mut();
            if let Some(position) = drag.narrow_to.take() {
                handles.select(position, false);
            }
            if handles.selected_vertices().is_empty() {
                return;
            }

            // Relative snapping rounds the drag delta, preserving the initial off-grid offset.
            // Ctrl snaps the dragged handle itself to the grid, which only the vertex tool
            // allows (`VertexTool::allowAbsoluteSnapping`).
            let absolute = modifiers.control && handles.kind() == Some(HandleKind::Vertex);
            let grabbed = state
                .borrow()
                .dragging_handle
                .map(|at| at - drag.applied)
                .or_else(|| handles.selected_anchor());
            let total = if let (true, Some(anchor)) = (absolute, grabbed) {
                grid.snap_point(anchor + asked) - anchor
            } else {
                grid.snap_point(asked)
            };

            let step = total - drag.applied;
            if step == Vec3d::zero() {
                return;
            }
            let before_expansion = (!drag.vertex_stacks_expanded).then(|| handles.clone());
            if !drag.vertex_stacks_expanded {
                handles.expand_vertex_stacks(
                    camera.plane.depth_axis(),
                    pref(|p| p.drag_occluded_vertices_2d),
                );
            }
            let vertices = handles.selected_vertices();
            // A control point is not a brush vertex, so which of the two moves
            // depends on what the handles are of.
            let moved = if handles.kind() == Some(HandleKind::ControlPoint) {
                document.borrow_mut().move_control_points(&vertices, step)
            } else {
                document.borrow_mut().move_handles(&vertices, step)
            };
            if moved {
                drag.vertex_stacks_expanded = true;
                handles.move_selection(step);
                drag.applied = total;
                drop(handles);
                // The guide follows the moving handle.
                let mut state = state.borrow_mut();
                state.dragging_handle = state.dragging_handle.map(|at| at + step);
                drop(state);
                drop(drag);
                redraw();
            } else if let Some(before) = before_expansion {
                *handles = before;
            }
            return;
        }

        if drag.mode == DragMode::SplittingHandle {
            let camera = state.borrow().camera;
            let from = camera.screen_to_plane(drag.origin, viewport);
            let to = camera.screen_to_plane(drag.current, viewport);
            let asked = camera.plane.unproject(to - from, 0.0);
            let total = grid.borrow().snap_point(asked);
            if total == drag.applied {
                return;
            }

            let Some(at) = drag.splitting else {
                return;
            };

            let mut document = document.borrow_mut();
            // Each step is measured from the brush as it was, so the corner follows
            // the pointer rather than compounding.
            document.rollback_transaction();
            if document.split_handle(at, total) > 0 {
                drag.applied = total;
                state.borrow_mut().dragging_handle = Some(at + total);
                // The new corner is what is picked, so it can be moved again, or
                // rounded, without hunting for it.
                let mut handles = handles.borrow_mut();
                handles.clear_selection();
                handles.select(at + total, false);
            }
            drop(document);
            drop(drag);
            redraw();
            return;
        }

        if matches!(
            drag.mode,
            DragMode::PaintingSelection | DragMode::PaintingFaces
        ) {
            // Still a click until the pointer has moved off the spot.
            if drag.click.is_some() && vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD {
                return;
            }
            let faces = drag.mode == DragMode::PaintingFaces;
            let ray = state
                .borrow()
                .camera
                .ray_through(drag.current, viewport, PICK_REACH);
            let mut document = document.borrow_mut();
            // What the press was on goes in first, as `SelectionTool::acceptMouseDrag`
            // selects it before the tracker follows the pointer; the press is no longer
            // a click.
            let mut changed = match drag.click.take() {
                Some(Click::Object { node, .. }) if !faces => document.paint_select(node),
                Some(Click::Face { face, .. }) if faces => document.paint_select_face(face),
                Some(Click::UnpickHandle(_)) => {
                    let ray = state
                        .borrow()
                        .camera
                        .ray_through(drag.origin, viewport, PICK_REACH);
                    document
                        .pick_flat(&ray)
                        .is_some_and(|hit| document.paint_select(hit.node))
                }
                _ => false,
            };
            if let Some(hit) = document.pick_flat(&ray) {
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
            drop(drag);
            if changed {
                redraw_everywhere();
                redraw();
                refresh_panels();
            }
            return;
        }

        if matches!(drag.mode, DragMode::Lassoing | DragMode::LassoingHandles) {
            state.borrow_mut().lasso = Some((drag.origin, drag.current));
            if drag.mode == DragMode::Lassoing {
                let camera = state.borrow().camera;
                let caught = lassoed(
                    &camera,
                    &document.borrow(),
                    drag.origin,
                    drag.current,
                    viewport,
                );
                if let Some(initial) = marquee.borrow().as_ref() {
                    let add = modifiers.control;
                    document.borrow_mut().preview_marquee(initial, &caught, add);
                }
            }
            drop(drag);
            redraw_everywhere();
            redraw();
            return;
        }

        // A press that has not moved past the threshold is still a click: no box yet
        // (`DrawShapeToolController2D` starts only on a real drag).
        if drag.mode == DragMode::Creating && vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD {
            state.borrow_mut().creating = None;
            *pending_shape.borrow_mut() = None;
            return;
        }
        if drag.mode == DragMode::Creating {
            // Show the box that would be made -- snapped, and with the depth this
            // view cannot speak for -- rather than the rectangle the pointer swept.
            // Nothing is committed until the drag ends.
            let camera = state.borrow().camera;
            let square = modifiers.shift;
            let bounds = swept_bounds(
                &camera,
                &grid.borrow(),
                drag.origin,
                drag.current,
                viewport,
                &document.borrow(),
                square,
                square && modifiers.alt,
            );
            state.borrow_mut().creating = bounds;
            // The 3D view draws the same box as the shape it would make, so it is
            // shared rather than kept in this view's own state.
            *pending_shape.borrow_mut() = bounds;
            drop(drag);
            redraw_everywhere();
            return;
        }
        if drag.mode != DragMode::Moving {
            return;
        }
        let camera = state.borrow().camera;

        // Both ends of the drag are converted to world space and subtracted there, so
        // the delta is in world units whatever the zoom. A 2D view can only move
        // things in the two axes it shows.
        let from = camera.screen_to_plane(drag.origin, viewport);
        let to = camera.screen_to_plane(drag.origin + Vec2d::new(dx, dy), viewport);
        let mut asked = camera.plane.unproject(to - from, 0.0);
        // `MoveHandleDragTracker`: Shift keeps the move to the axis it goes furthest along.
        if modifiers.shift {
            let longest = (0..3)
                .max_by(|a, b| asked[*a].abs().total_cmp(&asked[*b].abs()))
                .unwrap_or(0);
            for axis in 0..3 {
                if axis != longest {
                    asked[axis] = 0.0;
                }
            }
        }

        let grid = *grid.borrow();
        let mut document = document.borrow_mut();
        // A Ctrl-drag copies the selection on its first real step and moves the copy.
        if drag.duplicate == Duplicate::Pending {
            if vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD {
                return;
            }
            let mark = document.edit_mark();
            document.duplicate_selection_by(Vec3d::zero());
            document.begin_transaction();
            drag.duplicate = Duplicate::Made(mark);
        }
        let Some(bounds) = document.selection_bounds() else {
            return;
        };

        // The move is measured from the selection's reference point — its snapped
        // centre — and asked of the grid, which drops any part of it that would go
        // against the drag. Measuring the whole move from where the drag began, rather
        // than accumulating each step, is what keeps snapping from eating a little of
        // every one. Ctrl snaps the point that was grabbed to the grid instead, wherever
        // it lands (`MoveHandleDragTracker`'s absolute snapping).
        let total = if modifiers.control && drag.duplicate == Duplicate::No {
            let grabbed = camera.plane.unproject(from, 0.0);
            let mut total = grid.snap_point(grabbed + asked) - grabbed;
            total[camera.plane.depth_axis()] = 0.0;
            total
        } else {
            let reference = grid.reference_point(&bounds);
            grid.move_delta_for_point(reference, asked)
        };

        let step = total - drag.applied;
        if step == Vec3d::zero() {
            return;
        }
        if document.translate_selection(step) {
            drag.applied = total;
            drop(document);
            redraw();
        }
    }
    /// The release; `(dx, dy)` is the offset from the press.
    pub fn end(&self, dx: f64, dy: f64, modifiers: Modifiers, viewport: (f64, f64)) {
        #[allow(unused_variables)]
        let Self {
            state,
            grid,
            document,
            shape_parameters,
            tool,
            clip,
            handles,
            rotate,
            fillet,
            pending_shape,
            redraw_everywhere,
            sweep,
            refresh_panels,
            redraw,
            focus,
            drag,
            marquee,
        } = self;

        state.borrow_mut().creating = None;
        // Whatever the release means further down, the selection is no longer being
        // dragged: a click that never moved ends here too.
        state.borrow_mut().moving = false;
        *pending_shape.borrow_mut() = None;

        let mut drag = drag.borrow_mut();

        // A press that never became a drag is a click, and selects what it was on.
        if let Some(click) = drag.click {
            if vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD {
                drag.mode = DragMode::Idle;
                drop(drag);
                let mut document = document.borrow_mut();
                // A Shift-press opened the extrude's transaction.
                document.cancel_transaction();
                match click {
                    Click::Object { node, toggle: true } => document.toggle_selected(node),
                    Click::Object {
                        node,
                        toggle: false,
                    } => document.select_only(node),
                    Click::Face { face, toggle: true } => document.toggle_face_selected(face),
                    Click::Face {
                        face,
                        toggle: false,
                    } => document.select_face_only(face),
                    Click::Nothing => document.clear_selection(),
                    Click::UnpickHandle(position) => handles.borrow_mut().select(position, true),
                }
                drop(document);
                redraw_everywhere();
                redraw();
                refresh_panels();
                return;
            }
        }

        if matches!(
            drag.mode,
            DragMode::Moving
                | DragMode::MovingHandle
                | DragMode::SplittingHandle
                | DragMode::Scaling
                | DragMode::Rotating
                | DragMode::Shearing
                | DragMode::Extruding
        ) {
            // Defer reducing an existing multi-selection until release: pressing a
            // selected vertex must still allow dragging the whole selection.
            if drag.mode == DragMode::MovingHandle
                && !drag.vertex_stacks_expanded
                && vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD
                && !(modifiers.control || modifiers.shift)
            {
                let mut handles = handles.borrow_mut();
                if handles.kind() == Some(HandleKind::Vertex) {
                    if let Some(position) = state.borrow().dragging_handle {
                        handles.select(position, false);
                    }
                }
            }
            // The whole drag becomes one step in the history, a Ctrl-drag's copy with it.
            let committed = document.borrow_mut().commit_transaction();
            if let Duplicate::Made(mark) = drag.duplicate {
                document.borrow_mut().fold_edits_since(mark);
            }
            if committed && drag.mode == DragMode::Rotating {
                crate::say!(
                    "{}",
                    fill(
                        "Rotated by {}°",
                        &[&format!("{:.1}", rotate.borrow().angle().to_degrees())]
                    )
                );
            }
            drag.mode = DragMode::Idle;
            drag.scaling = None;
            drag.shearing = None;
            drag.extruding = None;
            let mut state = state.borrow_mut();
            state.lasso = None;
            state.scaling = None;
            state.rotating = false;
            state.moving = false;
            state.dragging_handle = None;
            drop(state);
            redraw();
            return;
        }
        if drag.mode == DragMode::MovingRotationCenter {
            drag.mode = DragMode::Idle;
            return;
        }
        if drag.mode == DragMode::ClippingPoint {
            clip.borrow_mut().end_drag_point();
            drag.mode = DragMode::Idle;
            redraw();
            return;
        }
        let mode = std::mem::take(&mut drag.mode);

        if mode == DragMode::LassoingHandles {
            let swept = state.borrow().lasso;
            state.borrow_mut().lasso = None;
            let Some((from, to)) = swept else {
                return;
            };
            if vec::distance(from, to) < DRAG_THRESHOLD {
                redraw();
                return;
            }

            // The handles inside the rectangle, which is `VertexToolController`'s
            // lasso: several vertices or edges taken in one sweep.
            let camera = state.borrow().camera;
            let add = modifiers.control;
            let mut handles = handles.borrow_mut();
            let caught: Vec<Vec3d> = handles
                .handles()
                .iter()
                .map(|handle| handle.position)
                .filter(|position| {
                    let on_screen =
                        camera.plane_to_screen(camera.plane.project(*position), viewport);
                    let (left, right) = (from.x().min(to.x()), from.x().max(to.x()));
                    let (top, bottom) = (from.y().min(to.y()), from.y().max(to.y()));
                    on_screen.x() >= left
                        && on_screen.x() <= right
                        && on_screen.y() >= top
                        && on_screen.y() <= bottom
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
            crate::say!(
                "{}",
                fill("Selected {}", &[&counted(caught.len(), "handle")])
            );
            redraw();
            refresh_panels();
            return;
        }

        if mode == DragMode::Lassoing {
            let swept = state.borrow().lasso;
            state.borrow_mut().lasso = None;
            let Some((from, to)) = swept else {
                return;
            };

            // A drag of a pixel or two is a click that shook, and clicking empty
            // space clears the selection -- which is what the press handler already
            // did, so there is nothing left to do.
            if vec::distance(from, to) < 3.0 {
                redraw();
                return;
            }
            let camera = state.borrow().camera;
            let add = modifiers.control;
            let caught = lassoed(&camera, &document.borrow(), from, to, viewport);

            let mut document = document.borrow_mut();
            if let Some(initial) = marquee.borrow_mut().take() {
                document.preview_marquee(&initial, &caught, add);
            }
            crate::say!(
                "{}",
                fill("{} selected", &[&counted(caught.len(), "object")])
            );
            drop(document);
            redraw_everywhere();
            redraw();
            refresh_panels();
            return;
        }

        if mode != DragMode::Creating || vec::length(Vec2d::new(dx, dy)) < DRAG_THRESHOLD {
            return;
        }
        let camera = state.borrow().camera;
        let grid = *grid.borrow();

        let square = modifiers.shift;

        let Some(bounds) = swept_bounds(
            &camera,
            &grid,
            drag.origin,
            drag.origin + Vec2d::new(dx, dy),
            viewport,
            &document.borrow(),
            square,
            square && modifiers.alt,
        ) else {
            return;
        };

        let mut document = document.borrow_mut();
        // The tool stays as it was (`DrawShapeTool`): drawing with Select leaves Select in
        // hand, and the frontend makes the shape at once.
        match document.begin_shape_preview(bounds, *shape_parameters.borrow()) {
            Ok(_) => {}
            Err(error) => crate::complain!(
                "{}",
                fill("Could not draw the shape: {}", &[&error.to_string()])
            ),
        }
        drop(document);
        redraw();
    }
    /// The gesture was taken away without a release: the pointer leaving the window,
    /// another grab taking over, focus going elsewhere. `drag-end` does not always come
    /// then, and a transaction left open would swallow every later edit, applied and not
    /// recorded. What the drag did is kept as one step, as a release would; a brush being
    /// drawn is dropped.
    pub fn lose(&self) {
        self.end_gesture(true);
    }

    /// Esc: the gesture is undone and ended, as a tracker's `cancel` does.
    pub fn cancel(&self) {
        self.end_gesture(false);
    }

    fn end_gesture(&self, keep: bool) {
        #[allow(unused_variables)]
        let Self {
            state,
            grid,
            document,
            shape_parameters,
            tool,
            clip,
            handles,
            rotate,
            fillet,
            pending_shape,
            redraw_everywhere,
            sweep,
            refresh_panels,
            redraw,
            focus,
            drag,
            marquee,
        } = self;

        let mut drag = drag.borrow_mut();
        if drag.mode == DragMode::Idle {
            return;
        }
        let painting = matches!(
            drag.mode,
            DragMode::PaintingSelection | DragMode::PaintingFaces
        );
        if drag.mode == DragMode::Lassoing || (painting && !keep) {
            // Esc while painting the selection undoes it, as `cancelTransaction` does.
            if let Some(initial) = marquee.borrow_mut().take() {
                document.borrow_mut().restore_selection(&initial);
            }
        } else if painting {
            marquee.borrow_mut().take();
        } else if keep {
            document.borrow_mut().commit_transaction();
        } else {
            // A tracker's `cancel` undoes the gesture (`cancelTransaction`).
            document.borrow_mut().cancel_transaction();
        }
        drag.mode = DragMode::Idle;
        drag.scaling = None;
        drag.shearing = None;
        drag.extruding = None;
        drag.click = None;
        *pending_shape.borrow_mut() = None;
        let mut state = state.borrow_mut();
        state.creating = None;
        state.lasso = None;
        state.scaling = None;
        state.rotating = false;
        state.moving = false;
        state.dragging_handle = None;
        drop(state);
        drop(drag);
        redraw_everywhere();
        redraw();
    }
    /// Escape during a lasso restores the selection. Returns whether it did, in which
    /// case the frontend resets its gesture.
    pub fn escape(&self) -> bool {
        #[allow(unused_variables)]
        let Self {
            state,
            grid,
            document,
            shape_parameters,
            tool,
            clip,
            handles,
            rotate,
            fillet,
            pending_shape,
            redraw_everywhere,
            sweep,
            refresh_panels,
            redraw,
            focus,
            drag,
            marquee,
        } = self;

        // Esc ends whatever drag is under way and undoes it, as `ToolChain::cancel` does;
        // a lasso puts the selection back.
        if drag.borrow().mode == DragMode::Idle {
            return false;
        }
        self.cancel();
        true
    }
}

/// One flat view's drawing: its own state and everything it draws from.
#[derive(Clone)]
pub struct FlatView {
    pub state: Rc<RefCell<ViewportState>>,
    /// The outlines of the faces a Shift-drag would extrude, kept current by the pointer
    /// motion and drawn on top -- `ExtrudeToolController::render`.
    pub extrude_preview: Rc<RefCell<Vec<Vec<Vec3d>>>>,
    pub grid: Rc<RefCell<Grid>>,
    pub document: Rc<RefCell<Document>>,
    pub tool: Rc<RefCell<Tool>>,
    pub clip: Rc<RefCell<ClipTool>>,
    pub handles: Rc<RefCell<HandleManager>>,
    pub rotate: Rc<RefCell<RotateTool>>,
    pub filters: Rc<RefCell<ViewFilters>>,
    pub sweep: Rc<RefCell<crate::sweep_tool::SweepInputs>>,
    pub fillet: Rc<RefCell<crate::fillet_tool::FilletParameters>>,
    pub pending_shape: Rc<RefCell<Option<Bbox3d>>>,
    /// Every flat view's state, so a pan or a zoom here brings the others into line.
    pub cameras: Rc<RefCell<Vec<Rc<RefCell<ViewportState>>>>>,
    /// Where a paste should land, shared with the 3D view.
    pub paste_target: Rc<RefCell<Option<Vec3d>>>,
    /// Which flat view the pointer was last over, for the window's arrow keys.
    pub active_plane: Rc<RefCell<Option<crate::camera2d::ViewPlane>>>,
    /// Redraws the flat views.
    pub redraw: Rc<dyn Fn()>,
    /// Redraws every view, the 3D one included.
    pub redraw_everywhere: Rc<dyn Fn()>,
    /// Asks the side panels to catch up.
    pub refresh_panels: Rc<dyn Fn()>,
}

/// What the pointer is over, for the right-click menu.
pub struct Probe {
    /// The entity under the pointer, if any.
    pub entity: Option<crate::document::EntityUnder>,
    /// The material of the face under the pointer, if any.
    pub material: Option<String>,
    /// Where the pointer is on the view's plane, snapped to the grid.
    pub at: Vec3d,
}

impl FlatView {
    /// A ray from the pointer into the view.
    pub fn ray(&self, x: f64, y: f64, viewport: (f64, f64)) -> geometry_math::ray::Ray3d {
        self.state
            .borrow()
            .camera
            .ray_through(Vec2d::new(x, y), viewport, PICK_REACH)
    }

    /// Where the pointer is on the view's plane, the axis it looks along taken as zero,
    /// snapped to the grid.
    pub fn snapped_point(&self, x: f64, y: f64, viewport: (f64, f64)) -> Vec3d {
        let state = self.state.borrow();
        let on_plane = state.camera.screen_to_plane(Vec2d::new(x, y), viewport);
        self.grid
            .borrow()
            .snap_point(state.camera.plane.unproject(on_plane, 0.0))
    }

    /// The node under the pointer.
    pub fn pick_node(
        &self,
        x: f64,
        y: f64,
        viewport: (f64, f64),
    ) -> Option<map_model::node::NodeId> {
        self.document
            .borrow()
            .pick_flat(&self.ray(x, y, viewport))
            .map(|hit| hit.node)
    }

    /// Zooms about the pointer, as `MapViewBase::doMouseScroll` does: what is under the
    /// cursor stays under it. `dy` below zero zooms in.
    ///
    /// The position the motion handler keeps stands in for the pointer; with the pointer
    /// outside the view -- a scroll on a view that only has the keyboard focus -- the
    /// centre of the view does.
    pub fn scroll(&self, dy: f64, viewport: (f64, f64)) {
        let anchor = self
            .state
            .borrow()
            .cursor
            .unwrap_or_else(|| Vec2d::new(viewport.0 / 2.0, viewport.1 / 2.0));
        let factor = if dy < 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.state
            .borrow_mut()
            .camera
            .zoom_at(factor, anchor, viewport);
        link_cameras(&self.state, &self.cameras);
        (self.redraw)();
    }

    /// Zooms by `factor` (above one zooms in) keeping what is under `(x, y)` there: a
    /// pinch, which zooms by however far the fingers spread rather than a notch at a time.
    pub fn zoom_about(&self, factor: f64, x: f64, y: f64, viewport: (f64, f64)) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        self.state
            .borrow_mut()
            .camera
            .zoom_at(factor, Vec2d::new(x, y), viewport);
        link_cameras(&self.state, &self.cameras);
        (self.redraw)();
    }

    /// Pans by a pointer movement of `(dx, dy)` pixels (a middle-button drag).
    pub fn pan(&self, dx: f64, dy: f64) {
        self.state.borrow_mut().camera.pan_by_pixels(dx, dy);
        link_cameras(&self.state, &self.cameras);
        (self.redraw)();
    }

    /// The pointer moved over the view.
    pub fn motion(&self, x: f64, y: f64, modifiers: Modifiers, viewport: (f64, f64)) {
        let mut state = self.state.borrow_mut();
        state.cursor = Some(Vec2d::new(x, y));
        *self.active_plane.borrow_mut() = Some(state.camera.plane);
        // Where a paste would land: the pointer on this view's plane, with the axis the
        // view looks along left for `paste_at` to keep as it was.
        let on_plane = state.camera.screen_to_plane(Vec2d::new(x, y), viewport);
        let mut at = state.camera.plane.unproject(on_plane, 0.0);
        at[state.camera.plane.depth_axis()] = f64::NAN;
        *self.paste_target.borrow_mut() = Some(at);
        drop(state);
        // With Shift held, outline the faces a Shift-drag would extrude, as
        // `ExtrudeToolController::render` draws the proposed drag handles.
        self.update_extrude_preview(Vec2d::new(x, y), modifiers, viewport);
        (self.redraw)();
    }

    /// The pointer left the view.
    pub fn leave(&self) {
        self.state.borrow_mut().cursor = None;
        if !self.extrude_preview.borrow().is_empty() {
            self.extrude_preview.borrow_mut().clear();
        }
        // What the pointer was over is drawn from `cursor` -- the selection's sizes, a
        // hovered handle -- so the view has to be drawn again without it.
        (self.redraw)();
    }

    /// Pressing or releasing Shift with the pointer held still also changes whether the
    /// extrude outline shows -- `ExtrudeToolController::modifierKeyChange`.
    pub fn modifiers_changed(&self, modifiers: Modifiers, viewport: (f64, f64)) {
        let cursor = self.state.borrow().cursor;
        if let Some(cursor) = cursor {
            self.update_extrude_preview(cursor, modifiers, viewport);
        }
    }

    /// Rebuilds the Shift-extrude outline and redraws if it changed.
    fn update_extrude_preview(&self, cursor: Vec2d, modifiers: Modifiers, viewport: (f64, f64)) {
        let ray = self
            .state
            .borrow()
            .camera
            .ray_through(cursor, viewport, PICK_REACH);
        let allow_corner = !(modifiers.control || modifiers.alt);
        let outlines = if modifiers.shift {
            let document = self.document.borrow();
            if let Some((brush, corner)) = allow_corner
                .then(|| document.extrude_corner_2d(&ray))
                .flatten()
            {
                corner
                    .faces
                    .iter()
                    .filter_map(|(index, _)| brush.face_polygon(*index))
                    .collect()
            } else {
                document.extrude_preview_faces_2d(&ray)
            }
        } else {
            Vec::new()
        };
        if *self.extrude_preview.borrow() != outlines {
            *self.extrude_preview.borrow_mut() = outlines;
            (self.redraw)();
        }
    }

    /// A primary-button click, `presses` times: clicks that are about a face.
    ///
    /// `AddClipPointPart::doMouseDoubleClick` clips along the face double-clicked, and the
    /// paint tool puts the current material on the one clicked -- a single click for the
    /// face the ray enters through, a double click for the whole brush, Shift to take the
    /// face's material as the current one.
    pub fn face_click(
        &self,
        presses: i32,
        x: f64,
        y: f64,
        modifiers: Modifiers,
        viewport: (f64, f64),
    ) {
        let active = *self.tool.borrow();
        if active != Tool::Clip && active != Tool::Paint {
            return;
        }
        let ray = self.ray(x, y, viewport);
        let mut document = self.document.borrow_mut();
        let Some(hit) = document.pick_flat(&ray) else {
            return;
        };
        let Some(index) = hit.face else {
            return;
        };
        let face = FaceRef {
            node: hit.node,
            face: index,
        };

        if active == Tool::Clip {
            if presses != 2 {
                return;
            }
            let Some(points) = document.face_points(face) else {
                return;
            };
            self.clip.borrow_mut().set_face(points);
            drop(document);
            (self.redraw)();
            return;
        }

        // The paint tool.
        if modifiers.shift {
            if let Some(picked) = document.face_at(face).map(|f| f.material_name.clone()) {
                document.set_current_material(picked);
                drop(document);
                (self.refresh_panels)();
            }
            return;
        }
        let material = document.current_material().to_string();
        let painted = if presses >= 2 {
            document.paint_brush(hit.node, &material)
        } else {
            document.paint_face(face, &material)
        };
        if painted {
            drop(document);
            (self.redraw)();
        }
    }

    /// The wheel with modifiers, `dy` notches towards the user positive, at `(x, y)`.
    /// Returns whether a modifier made it something other than a zoom: Ctrl drills the
    /// selection through what is under the pointer, smallest first, and Ctrl+Alt changes
    /// the grid (`SelectionTool::mouseScroll`).
    pub fn scroll_with(
        &self,
        dy: f64,
        modifiers: Modifiers,
        x: f64,
        y: f64,
        viewport: (f64, f64),
    ) -> bool {
        if !modifiers.control || modifiers.shift {
            return false;
        }
        let away = (dy < 0.0) != pref(|p| p.invert_mouse_wheel);
        if modifiers.alt {
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
            (self.redraw_everywhere)();
            return true;
        }
        let ray = self.ray(x, y, viewport);
        if self.document.borrow_mut().drill_selection(&ray, away, true) {
            (self.redraw_everywhere)();
            (self.refresh_panels)();
        }
        true
    }

    /// A double click released at `(x, y)`: [`Document::double_click`] on what is under
    /// it, with the Select or Brush tool in hand. Returns whether anything changed.
    pub fn double_click(&self, x: f64, y: f64, modifiers: Modifiers, viewport: (f64, f64)) -> bool {
        if !matches!(*self.tool.borrow(), Tool::Select | Tool::Brush) {
            return false;
        }
        let ray = self.ray(x, y, viewport);
        let mut document = self.document.borrow_mut();
        let hit = document.pick_flat(&ray);
        let changed = document.double_click(hit, modifiers.shift, modifiers.alt, modifiers.control);
        drop(document);
        if changed {
            (self.redraw_everywhere)();
            (self.refresh_panels)();
        }
        changed
    }

    pub fn face_under_pointer(&self, x: f64, y: f64, viewport: (f64, f64)) -> Option<FaceRef> {
        let hit = self
            .document
            .borrow()
            .pick_flat(&self.ray(x, y, viewport))?;
        Some(FaceRef {
            node: hit.node,
            face: hit.face?,
        })
    }

    /// One sample of a held-button material stroke in a flat view.
    pub fn paint_material_stroke_at(
        &self,
        presses: i32,
        x: f64,
        y: f64,
        viewport: (f64, f64),
        stroke: &mut crate::document::MaterialStroke,
    ) {
        if *self.tool.borrow() != Tool::Paint || self.filters.borrow().animation_frame.is_some() {
            return;
        }
        if x < 0.0 || y < 0.0 || x > viewport.0 || y > viewport.1 {
            return;
        }
        let ray = self.ray(x, y, viewport);
        let mut document = self.document.borrow_mut();
        let Some(hit) = document.pick_flat(&ray) else {
            return;
        };
        let Some(face) = hit.face else {
            return;
        };
        let targets = if presses >= 2 {
            (0..document.brush(hit.node).unwrap().geometry().face_count())
                .map(|face| FaceRef {
                    node: hit.node,
                    face,
                })
                .collect()
        } else {
            vec![FaceRef {
                node: hit.node,
                face,
            }]
        };
        let changed = document.paint_material_stroke(&targets, stroke) > 0;
        drop(document);
        if changed {
            (self.redraw_everywhere)();
        }
    }

    /// What the right-click menu needs to know about the point under the pointer.
    pub fn probe(&self, x: f64, y: f64, viewport: (f64, f64)) -> Probe {
        let ray = self.ray(x, y, viewport);
        let at = self.snapped_point(x, y, viewport);
        let document = self.document.borrow();
        let hit = document.pick_flat(&ray);
        Probe {
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

    /// A prefab from the asset library dropped on the view: inserted where it landed.
    pub fn drop_prefab(
        &self,
        path: &std::path::Path,
        x: f64,
        y: f64,
        viewport: (f64, f64),
    ) -> bool {
        let at = self.snapped_point(x, y, viewport);
        let landed = crate::assets::insert_prefab_on_grid(
            &mut self.document.borrow_mut(),
            path,
            at,
            &self.grid.borrow(),
        );
        (self.redraw_everywhere)();
        (self.refresh_panels)();
        landed
    }

    /// A class from the entity browser dragged over the view: outlines the box a drop
    /// would leave. A flat view looks down an axis and its ray meets nothing, so the
    /// entity goes on a plane facing the viewer through whatever is selected --
    /// `updateEntityPosition2D`.
    pub fn entity_drag_over(&self, classname: Option<&str>, x: f64, y: f64, viewport: (f64, f64)) {
        let landing = classname.map(|classname| {
            crate::entity_placement::drop_preview_bounds(
                &self.document.borrow(),
                &self.grid.borrow(),
                classname,
                &self.ray(x, y, viewport),
                true,
            )
        });
        let changed = self.state.borrow().entity_drop != landing;
        if changed {
            self.state.borrow_mut().entity_drop = landing;
            (self.redraw)();
        }
    }

    /// The entity drag left the view without a drop.
    pub fn entity_drag_leave(&self) {
        let showing = self.state.borrow().entity_drop.is_some();
        if showing {
            self.state.borrow_mut().entity_drop = None;
            (self.redraw)();
        }
    }

    /// A class from the entity browser dropped on the view: one placed there.
    pub fn drop_entity(&self, classname: &str, x: f64, y: f64, viewport: (f64, f64)) -> bool {
        self.state.borrow_mut().entity_drop = None;
        let placed = crate::entity_placement::place_entity_on_reference_plane(
            &self.document,
            &self.grid.borrow(),
            classname,
            &self.ray(x, y, viewport),
        );
        (self.redraw_everywhere)();
        (self.refresh_panels)();
        placed
    }

    /// Draws the view `viewport` pixels large.
    pub fn draw(&self, cr: &mut dyn Canvas, viewport: (f64, f64)) {
        self.draw_keeping_base(cr, None, viewport);
    }

    /// [`FlatView::draw`], with the bottom of the picture ([`draw_base`]) handed to
    /// `keeper` with its [`base_key`], so a view that kept the last one can draw that
    /// again instead. Without a keeper it is drawn into `cr` like the rest.
    pub fn draw_keeping_base(
        &self,
        cr: &mut dyn Canvas,
        keeper: Option<&mut dyn BaseKeeper>,
        viewport: (f64, f64),
    ) {
        #[allow(unused_variables)]
        let Self {
            state,
            extrude_preview,
            grid,
            document,
            tool,
            clip,
            handles,
            rotate,
            filters,
            sweep,
            fillet,
            pending_shape,
            ..
        } = self;

        let document = document.borrow();
        frame_new_document(&mut state.borrow_mut(), &document, viewport);
        // A draw drag started in another view (the 3D one) shows here too, through the
        // shared box. This view's own drag fills `creating` instead.
        {
            let mut state = state.borrow_mut();
            state.creating_elsewhere = match state.creating {
                Some(_) => None,
                None => *pending_shape.borrow(),
            };
        }
        {
            let (state, grid, filters) = (state.borrow(), grid.borrow(), filters.borrow());
            match keeper {
                Some(keeper) => {
                    let key = base_key(&state, &grid, &document, &filters, viewport);
                    keeper.base(key, &|cr: &mut dyn Canvas| {
                        draw_base(&state, &grid, &document, &filters, cr, viewport)
                    });
                }
                None => draw_base(&state, &grid, &document, &filters, cr, viewport),
            }
            draw_over_base(&state, &document, &filters, cr, viewport);
        }
        crate::animation_mode::draw(cr, &filters.borrow(), viewport.1);
        if filters.borrow().animation_frame.is_some() {
            let f = *filters.borrow();
            let time = f.animation_frame.unwrap() as f64 / f.animation_fps.max(1) as f64;
            if let Ok(pose) = crate::animation_pose::Pose::selected(&document, time) {
                if let Some(path) = document.animation_path(&pose).as_ref() {
                    path.draw(cr, time, f.animation_fps, |p| {
                        Some(state.borrow().camera.world_to_screen(p, viewport))
                    });
                }
                if f.animation_edit_pivot || *tool.borrow() == Tool::Rotate || f.animation_rotate {
                    let p = state
                        .borrow()
                        .camera
                        .world_to_screen(pose.centre(), viewport);
                    cr.save();
                    cr.set_source_rgb(1.0, 0.6, 0.15);
                    cr.set_line_width(1.5);
                    if f.animation_edit_pivot {
                        cr.set_source_rgb(1.0, 0.85, 0.15);
                        cr.rectangle(p.x() - 5.0, p.y() - 5.0, 10.0, 10.0);
                    } else {
                        cr.arc(p.x(), p.y(), 5.0, 0.0, std::f64::consts::TAU);
                    }
                    cr.move_to(p.x() - 9.0, p.y());
                    cr.line_to(p.x() + 9.0, p.y());
                    cr.move_to(p.x(), p.y() - 9.0);
                    cr.line_to(p.x(), p.y() + 9.0);
                    cr.stroke();
                    cr.restore();
                }
            }
            draw_compass(&state.borrow(), cr, viewport);
            return;
        }
        draw_selection_bounds(&state.borrow(), &document, cr, viewport);
        draw_size_labels(&state.borrow(), &document, *tool.borrow(), cr, viewport);
        if *tool.borrow() == Tool::Clip {
            draw_clip(
                &state.borrow(),
                &grid.borrow(),
                &document,
                &clip.borrow(),
                cr,
                viewport,
            );
        }
        if *tool.borrow() == Tool::Sweep {
            draw_sweep(&state.borrow(), &document, &sweep.borrow(), cr, viewport);
        }
        if *tool.borrow() == Tool::Scale {
            if let Some(box_) = document.selection_bounds() {
                let dragged = state.borrow().scaling;
                draw_scale_handles(&state.borrow(), &box_, dragged, cr, viewport);
            }
        }
        if *tool.borrow() == Tool::Shear {
            if let Some(box_) = document.selection_bounds() {
                draw_shear_handles(&state.borrow(), &box_, cr, viewport);
            }
        }
        if *tool.borrow() == Tool::Rotate {
            let mut rotate = rotate.borrow_mut();
            // The centre follows the selection until someone drags it somewhere else.
            if rotate.center().is_none() {
                rotate.reset_center(&document, &grid.borrow());
            }
            if let Some(center) = rotate.center() {
                let turning = state.borrow().rotating.then(|| rotate.display_angle());
                draw_rotation_handle(&state.borrow(), center, turning, cr, viewport);
            }
        }
        if let Some(kind) = HandleKind::of(*tool.borrow()) {
            // The handles follow the selection and the brushes' own shape, both of
            // which change under the tool, so they are gathered afresh each frame.
            let mut handles = handles.borrow_mut();
            if kind == HandleKind::ControlPoint {
                handles.collect_control_points(document.selected_patches().into_iter());
            } else {
                handles.collect(kind, document.selected_brushes().into_iter());
            }
            let hovered = state
                .borrow()
                .cursor
                .and_then(|cursor| pick_handle(&state.borrow(), &handles, viewport, cursor));
            draw_handles(&state.borrow(), &handles, hovered, cr, viewport);
            draw_handle_status(&handles, cr);

            // The fillet: the edge a press would round, marked, and a dashed outline
            // of what it would leave.
            if *tool.borrow() == Tool::Fillet {
                draw_fillet_preview(
                    &state.borrow(),
                    &document,
                    &handles,
                    hovered,
                    *fillet.borrow(),
                    cr,
                    viewport,
                );
            }
        }
        draw_extrude_preview(&state.borrow(), &extrude_preview.borrow(), cr, viewport);
        draw_compass(&state.borrow(), cr, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera2d::ViewPlane;
    use crate::world::WorldBounds;

    /// Zooming one flat view zooms the others while "Link 2D views" is on, and leaves
    /// them alone while it is off.
    #[test]
    fn the_link_2d_views_preference_decides_whether_views_follow() {
        let bounds = WorldBounds::default();
        let state = |plane| {
            Rc::new(RefCell::new(ViewportState {
                camera: Camera2d::new(plane, bounds.half_extent()),
                cursor: None,
                bounds,
                framed_generation: None,
                creating: None,
                creating_elsewhere: None,
                lasso: None,
                scaling: None,
                rotating: false,
                moving: false,
                dragging_handle: None,
                entity_drop: None,
            }))
        };
        let (top, front) = (state(ViewPlane::Top), state(ViewPlane::Front));
        let cameras = Rc::new(RefCell::new(vec![Rc::clone(&top), Rc::clone(&front)]));
        let before = front.borrow().camera.zoom;
        crate::preferences::edit(|p| p.link_2d_cameras = false);
        top.borrow_mut().camera.zoom = before * 3.0;
        link_cameras(&top, &cameras);
        assert_eq!(front.borrow().camera.zoom, before);
        crate::preferences::edit(|p| p.link_2d_cameras = true);
        link_cameras(&top, &cameras);
        assert_eq!(front.borrow().camera.zoom, before * 3.0);
    }

    #[test]
    fn animation_wireframe_renders_all_orthographic_planes() {
        let mut d = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d::from_scalars(0.0, 16.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 128 96 64 0 0 0");
        let original = d.to_map_text();
        let filters = ViewFilters {
            animation_frame: Some(30),
            animation_fps: 30,
            onion_frames: 1,
            onion_step: 15,
            ..Default::default()
        };
        let mut canvas = crate::canvas::Raster::new(960, 320).unwrap();
        let cr = &mut canvas;
        cr.set_source_rgb(0.08, 0.08, 0.08);
        cr.paint();
        for (i, plane) in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side]
            .into_iter()
            .enumerate()
        {
            let bounds = WorldBounds::default();
            let mut state = ViewportState {
                camera: Camera2d::new(plane, bounds.half_extent()),
                cursor: None,
                bounds,
                framed_generation: None,
                creating: None,
                creating_elsewhere: None,
                lasso: None,
                scaling: None,
                rotating: false,
                moving: false,
                dragging_handle: None,
                entity_drop: None,
            };
            state.camera.position = Vec3d::new(72.0, 56.0, 40.0);
            state.camera.zoom = 2.5;
            cr.save();
            cr.translate(i as f64 * 320.0, 0.0);
            cr.rectangle(0.0, 0.0, 320.0, 320.0);
            cr.clip();
            draw_animation_brushes(&state, &d, &filters, cr, (320.0, 320.0));
            let pose = crate::animation_pose::Pose::selected(&d, 1.0).unwrap();
            if let Some(path) = d.animation_path(&pose).as_ref() {
                path.draw(cr, 1.0, 30, |p| {
                    Some(state.camera.world_to_screen(p, (320.0, 320.0)))
                });
            }
            cr.restore();
        }
        canvas
            .into_rgba()
            .save_png(&std::env::temp_dir().join("orthographic-animation.png"))
            .unwrap();
        assert_eq!(d.to_map_text(), original);
    }

    #[test]
    fn clip_mesh_ghosts_only_the_removed_half_and_preserves_document() {
        let mut document = two_cubes();
        document.select_all();
        let filters = crate::view_filters::ViewFilters::default();
        let original = document.preview_mesh(&filters);
        let revision = document.revision();
        let mut clip = ClipTool::new();
        clip.add_point(Vec3d::new(32., 0., 0.), vec![Vec3d::new(0., 0., 1.)]);
        clip.add_point(Vec3d::new(32., 64., 0.), vec![Vec3d::new(0., 0., 1.)]);
        let preview = document.preview_mesh_with_clip(&filters, Some(&clip));
        assert!(preview.triangles.iter().any(|t| t.alpha == 0.25));
        assert!(preview.triangles.iter().any(|t| t.alpha == 1.0));
        assert!(preview.triangles.iter().all(|t| {
            t.positions.iter().all(|p| p.x() <= 32. + 1e-6)
                || t.positions.iter().all(|p| p.x() >= 32. - 1e-6)
        }));
        clip.toggle_side(); // Both halves survive.
        assert!(document
            .preview_mesh_with_clip(&filters, Some(&clip))
            .triangles
            .iter()
            .all(|t| t.alpha == 1.0));
        assert_eq!(document.revision(), revision);
        let restored = document.preview_mesh(&filters);
        assert_eq!(restored.triangles.len(), original.triangles.len());
        assert!(restored.triangles.iter().all(|t| t.alpha == 1.0));
    }

    #[test]
    fn clip_hover_matches_the_first_and_second_clicked_grid_points() {
        let mut document = two_cubes();
        document.select_all();
        let grid = Grid::new(4);
        let viewport = (640., 480.);
        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let bounds = WorldBounds::default();
            let mut state = ViewportState {
                camera: Camera2d::new(plane, bounds.half_extent()),
                cursor: Some(Vec2d::new(323., 211.)),
                bounds,
                framed_generation: None,
                creating: None,
                creating_elsewhere: None,
                lasso: None,
                scaling: None,
                rotating: false,
                moving: false,
                dragging_handle: None,
                entity_drop: None,
            };
            state.camera.zoom = 2.;
            let mut clip = ClipTool::default();
            let first = clip_preview_point(&state, &grid, &document, &clip, viewport).unwrap();
            assert_eq!(first, grid.snap_point(first));
            // The empty tool must already draw the hover point.
            let mut canvas = crate::canvas::Raster::new(640, 480).unwrap();
            // Other viewport drawing can leave a current point behind.
            canvas.move_to(0.0, 0.0);
            draw_clip(&state, &grid, &document, &clip, &mut canvas, viewport);
            let center = state.camera.world_to_screen(first, viewport);
            let stride = 640 * 4;
            let pixels = canvas.into_rgba().pixels;
            assert!(pixels.iter().any(|b| *b != 0));
            for y in 0..480usize {
                for x in 0..640usize {
                    if (x as f64 - center.x()).hypot(y as f64 - center.y())
                        > CLIP_HANDLE_RADIUS + 2.0
                    {
                        assert_eq!(
                            &pixels[y * stride + x * 4..y * stride + x * 4 + 4],
                            &[0; 4],
                            "clip hover painted outside its handle at {x}, {y}"
                        );
                    }
                }
            }
            begin_clip(
                &state,
                &grid,
                &document,
                &mut clip,
                viewport,
                state.cursor.unwrap(),
            );
            assert_eq!(clip.points(), vec![first]);
            assert!(clip_preview_point(&state, &grid, &document, &clip, viewport).is_none());
            clip.end_drag_point();
            state.cursor = Some(Vec2d::new(419., 211.));
            let second = clip_preview_point(&state, &grid, &document, &clip, viewport).unwrap();
            begin_clip(
                &state,
                &grid,
                &document,
                &mut clip,
                viewport,
                state.cursor.unwrap(),
            );
            assert_eq!(clip.points(), vec![first, second]);
            clip.end_drag_point();
            state.cursor = None;
            assert!(clip_preview_point(&state, &grid, &document, &clip, viewport).is_none());
        }
    }

    #[test]
    fn corner_extrude_is_only_used_without_ctrl_or_alt() {
        let mut document = two_cubes();
        document.select_all();
        let direction = Vec3d::new(0.0, 0.0, -1.0);
        let ray = geometry_math::ray::Ray3d::new(Vec3d::new(600.0, 600.0, 200.0), direction);
        let corner = begin_extrude_2d(&document, &ray, direction, false, false).unwrap();
        assert!(corner.second_polygon.is_some());
        for (ctrl, alt) in [(true, false), (false, true), (true, true)] {
            let single = begin_extrude_2d(&document, &ray, direction, ctrl, alt).unwrap();
            assert!(single.second_polygon.is_none());
            assert_eq!(single.mode, extrude_tool::mode_for(ctrl, alt));
        }
    }

    /// A flat view's drag handler over `document`, looking down from the top.
    fn top_view(document: Document) -> (SelectAndMove, Camera2d) {
        let bounds = WorldBounds::default();
        let mut camera = Camera2d::new(ViewPlane::Top, bounds.half_extent());
        camera.zoom = 1.;
        let state = ViewportState {
            camera,
            cursor: None,
            bounds,
            framed_generation: None,
            creating: None,
            creating_elsewhere: None,
            lasso: None,
            scaling: None,
            rotating: false,
            moving: false,
            dragging_handle: None,
            entity_drop: None,
        };
        let drag = SelectAndMove {
            state: Rc::new(RefCell::new(state)),
            grid: Rc::new(RefCell::new(Grid::new(4))),
            document: Rc::new(RefCell::new(document)),
            shape_parameters: Rc::new(RefCell::new(ShapeParameters::default())),
            tool: Rc::new(RefCell::new(Tool::Select)),
            clip: Rc::new(RefCell::new(ClipTool::new())),
            handles: Rc::new(RefCell::new(HandleManager::new())),
            rotate: Rc::new(RefCell::new(RotateTool::new())),
            fillet: Rc::new(RefCell::new(crate::fillet_tool::FilletParameters::default())),
            pending_shape: Rc::new(RefCell::new(None)),
            redraw_everywhere: Rc::new(|| {}),
            sweep: Rc::new(RefCell::new(crate::sweep_tool::SweepInputs::default())),
            refresh_panels: Rc::new(|| {}),
            redraw: Rc::new(|| {}),
            focus: Rc::new(|| {}),
            drag: Rc::new(RefCell::new(DragState::default())),
            marquee: Rc::new(RefCell::new(None)),
        };
        (drag, camera)
    }

    /// A floor under everything and a cube standing on it.
    fn floor_and_cube() -> (Document, map_model::node::NodeId, map_model::node::NodeId) {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let mut ids = Vec::new();
        for (min, max) in [
            (
                Vec3d::new(-512.0, -512.0, -16.0),
                Vec3d::new(512.0, 512.0, 0.0),
            ),
            (Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(64.0, 64.0, 64.0)),
        ] {
            let brush = map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "stone")
                .expect("a cuboid");
            document.add_brushes(vec![brush]);
            ids.push(
                *document
                    .selection_ids()
                    .iter()
                    .next()
                    .expect("the new brush is selected"),
            );
            document.clear_selection();
        }
        (document, ids[0], ids[1])
    }

    fn drag_from(
        drag: &SelectAndMove,
        from: Vec2d,
        by: Vec2d,
        modifiers: Modifiers,
        viewport: (f64, f64),
    ) {
        drag.begin(from.x(), from.y(), modifiers, viewport);
        let steps = 4;
        for i in 1..=steps {
            let t = f64::from(i) / f64::from(steps);
            drag.update(by.x() * t, by.y() * t, modifiers, viewport);
        }
        drag.end(by.x(), by.y(), modifiers, viewport);
    }

    /// `MoveObjectsToolController` only moves what is selected; a drag that starts over
    /// an unselected brush -- the floor, in a top view, is under everything -- draws a
    /// new brush, as one over empty space does.
    #[test]
    fn a_drag_over_an_unselected_brush_draws_rather_than_moves_it() {
        let (document, floor, _) = floor_and_cube();
        let floor_bounds = document.brush(floor).unwrap().bounds();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(-200.0, -200.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(-100.0, -100.0, 0.0), viewport);
        drag_from(&drag, from, to - from, Modifiers::default(), viewport);
        let document = drag.document.borrow();
        assert_eq!(
            document.brush(floor).unwrap().bounds(),
            floor_bounds,
            "the floor moved"
        );
        assert!(document.has_shape_preview(), "no brush was drawn");
    }

    /// A click on an unselected brush still selects it.
    #[test]
    fn a_click_on_an_unselected_brush_selects_it() {
        let (document, floor, _) = floor_and_cube();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let at = camera.world_to_screen(Vec3d::new(-200.0, -200.0, 0.0), viewport);
        drag_from(
            &drag,
            at,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        let document = drag.document.borrow();
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![floor]
        );
        assert!(!document.has_shape_preview());
    }

    /// A drag on a selected brush moves it, even with the floor in front of it.
    #[test]
    fn a_drag_on_a_selected_brush_moves_it() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(96.0, 32.0, 0.0), viewport);
        drag_from(&drag, from, to - from, Modifiers::default(), viewport);
        let bounds = drag.document.borrow().brush(cube).unwrap().bounds();
        assert_eq!(bounds.min.x(), 64.0);
    }

    /// `ExtrudeTool`: Shift-dragging a selected brush's edge in a flat view pulls its face.
    #[test]
    fn shift_dragging_a_selected_brushs_edge_resizes_it() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(64.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(96.0, 32.0, 0.0), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        drag_from(&drag, from, to - from, shift, viewport);
        let bounds = drag.document.borrow().brush(cube).unwrap().bounds();
        assert_eq!((bounds.min.x(), bounds.max.x()), (0.0, 96.0));
    }

    fn entity_origin(document: &Document, id: map_model::node::NodeId) -> Vec3d {
        match &map_model::node::find_node(document.tree_world(), id)
            .unwrap()
            .kind
        {
            map_model::node::NodeKind::Entity(entity) => entity.origin().unwrap(),
            _ => panic!("not an entity"),
        }
    }

    /// A click on empty space only deselects; it draws no one-cell brush.
    #[test]
    fn a_click_on_empty_space_draws_nothing() {
        let (document, _, _) = floor_and_cube();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let at = camera.world_to_screen(Vec3d::new(900.0, 900.0, 0.0), viewport);
        drag_from(
            &drag,
            at,
            Vec2d::new(1.0, 1.0),
            Modifiers::default(),
            viewport,
        );
        assert!(!drag.document.borrow().has_shape_preview());
        assert_eq!(*drag.tool.borrow(), Tool::Select);
    }

    /// `SelectionTool::mouseClick` selects what is under the click even when something
    /// selected is under it too: the cube on a selected floor.
    #[test]
    fn a_click_on_a_cube_over_a_selected_floor_selects_the_cube() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(floor);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let at = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        drag_from(
            &drag,
            at,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        let document = drag.document.borrow();
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![cube]
        );
    }

    /// Shift-drag over the floor with nothing selected draws a brush, as without Shift.
    #[test]
    fn a_shift_drag_over_an_unselected_brush_draws() {
        let (document, floor, _) = floor_and_cube();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(-200.0, -200.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(-100.0, -150.0, 0.0), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        drag_from(&drag, from, to - from, shift, viewport);
        let document = drag.document.borrow();
        assert!(document.has_shape_preview());
        assert!(!document.is_selected(floor));
    }

    /// A dragged entity follows the pointer, and the move is one undo step.
    #[test]
    fn a_dragged_entity_moves_once_and_undoes() {
        let (mut document, _, _) = floor_and_cube();
        let entity =
            document.create_point_entity("info_player_start", Vec3d::new(200.0, 200.0, 0.0));
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(200.0, 200.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(264.0, 200.0, 0.0), viewport);
        drag_from(&drag, from, to - from, Modifiers::default(), viewport);
        let mut document = drag.document.borrow_mut();
        assert_eq!(
            entity_origin(&document, entity),
            Vec3d::new(264.0, 200.0, 0.0)
        );
        document.undo();
        assert_eq!(
            entity_origin(&document, entity),
            Vec3d::new(200.0, 200.0, 0.0)
        );
    }

    /// Esc in the middle of a move puts the selection back and ends the drag.
    #[test]
    fn escape_ends_a_move_and_puts_it_back() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(96.0, 32.0, 0.0), viewport);
        let by = to - from;
        drag.begin(from.x(), from.y(), Modifiers::default(), viewport);
        drag.update(by.x(), by.y(), Modifiers::default(), viewport);
        assert!(drag.escape());
        drag.update(by.x() * 2.0, by.y(), Modifiers::default(), viewport);
        drag.end(by.x() * 2.0, by.y(), Modifiers::default(), viewport);
        let document = drag.document.borrow();
        assert_eq!(document.brush(cube).unwrap().bounds().min.x(), 0.0);
        assert!(!document.in_transaction());
    }

    /// Shift in the Scale tool is proportional scaling, not an extrude.
    #[test]
    fn shift_in_the_scale_tool_does_not_extrude() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Scale;
        let at = camera.world_to_screen(Vec3d::new(64.0, 32.0, 0.0), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        drag.begin(at.x(), at.y(), shift, viewport);
        assert_ne!(drag.drag.borrow().mode, DragMode::Extruding);
        drag.cancel();
    }

    /// `DrawShapeToolController2D` draws only while nothing is selected: with the cube
    /// selected, a drag over the floor does nothing, and a click on the floor selects it.
    #[test]
    fn with_a_selection_a_drag_elsewhere_draws_nothing_and_a_click_selects() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(cube);
        let floor_bounds = document.brush(floor).unwrap().bounds();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(-200.0, -200.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(-100.0, -100.0, 0.0), viewport);
        drag_from(&drag, from, to - from, Modifiers::default(), viewport);
        {
            let document = drag.document.borrow();
            assert!(!document.has_shape_preview());
            assert_eq!(document.brush(floor).unwrap().bounds(), floor_bounds);
            assert!(document.is_selected(cube));
        }
        drag_from(
            &drag,
            from,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        let document = drag.document.borrow();
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![floor]
        );
        // And a click on empty space deselects, on release.
        drop(document);
        let empty = camera.world_to_screen(Vec3d::new(900.0, 900.0, 0.0), viewport);
        drag_from(
            &drag,
            empty,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        assert!(!drag.document.borrow().has_selection());
    }

    fn picked(
        node: map_model::node::NodeId,
        face: Option<usize>,
    ) -> Option<crate::document::Picked> {
        Some(crate::document::Picked {
            node,
            face,
            distance: 0.0,
        })
    }

    /// `SelectionTool::mouseDoubleClick`: a double click selects what shares the hit's
    /// container, opens a closed group and closes it again from outside.
    #[test]
    fn a_double_click_selects_siblings_and_opens_and_closes_groups() {
        let (mut document, floor, cube) = floor_and_cube();
        assert!(document.double_click(picked(cube, Some(0)), false, false, false));
        let mut selected: Vec<_> = document.selection_ids().iter().copied().collect();
        selected.sort();
        let mut both = vec![floor, cube];
        both.sort();
        assert_eq!(selected, both);

        document.select_only(cube);
        let group = document.group_selection("crate").expect("a group");
        document.clear_selection();
        assert!(document.double_click(picked(cube, Some(0)), false, false, false));
        assert_eq!(document.open_group(), Some(group));
        // Inside the open group, the cube is its only sibling.
        assert!(document.double_click(picked(cube, Some(0)), false, false, false));
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![cube]
        );
        // On empty space it closes.
        assert!(document.double_click(None, false, false, false));
        assert_eq!(document.open_group(), None);
    }

    /// Shift selects every face of the brush, Shift+Alt the faces in the clicked one's
    /// plane that touch it across edges.
    #[test]
    fn shift_double_clicks_select_faces() {
        let (mut document, floor, cube) = floor_and_cube();
        document.double_click(picked(cube, Some(0)), true, false, false);
        assert_eq!(document.selected_faces().len(), 6);
        assert!(document
            .selected_faces()
            .iter()
            .all(|face| face.node == cube));

        // A second cube beside the first shares an edge on top; the floor's top is in
        // another plane.
        let world = document.world_bounds();
        let beside = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::new(64.0, 0.0, 0.0), Vec3d::new(128.0, 64.0, 64.0)),
            "stone",
        )
        .unwrap();
        document.add_brushes(vec![beside]);
        let beside = *document.selection_ids().iter().next().unwrap();
        let top = |document: &Document, id| {
            let brush = document.brush(id).unwrap();
            (0..brush.faces().len())
                .find(|i| brush.faces()[*i].normal().z() > 0.9)
                .unwrap()
        };
        let cube_top = top(&document, cube);
        document.double_click(picked(cube, Some(cube_top)), true, true, false);
        let mut nodes: Vec<_> = document
            .selected_faces()
            .iter()
            .map(|face| face.node)
            .collect();
        nodes.sort();
        let mut want = vec![cube, beside];
        want.sort();
        assert_eq!(nodes, want);
        assert!(!nodes.contains(&floor));
    }

    /// `MoveObjectsTool::duplicateObjects`: a Ctrl-drag on the selected cube moves a copy,
    /// and the copy and its move undo together.
    #[test]
    fn a_ctrl_drag_moves_a_copy_that_undoes_in_one_step() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let before = document.to_map_text();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(160.0, 32.0, 0.0), viewport);
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        drag_from(&drag, from, to - from, ctrl, viewport);
        let mut document = drag.document.borrow_mut();
        assert_eq!(
            document.brush(cube).unwrap().bounds().min.x(),
            0.0,
            "the original moved"
        );
        assert_eq!(map_model::node::brushes(document.tree_world()).len(), 3);
        let copy = *document.selection_ids().iter().next().unwrap();
        assert_ne!(copy, cube);
        assert_eq!(document.brush(copy).unwrap().bounds().min.x(), 128.0);
        document.undo();
        assert_eq!(document.to_map_text(), before);
    }

    /// `MoveHandleDragTracker`: Shift holds a move to the axis it goes furthest along.
    #[test]
    fn a_shift_move_keeps_to_one_axis() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(128.0, 64.0, 0.0), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        // Shift at the press is about faces, so it is held only once the move is under way.
        drag.begin(from.x(), from.y(), Modifiers::default(), viewport);
        let by = to - from;
        drag.update(by.x(), by.y(), shift, viewport);
        drag.end(by.x(), by.y(), shift, viewport);
        let bounds = drag.document.borrow().brush(cube).unwrap().bounds();
        assert_eq!((bounds.min.x(), bounds.min.y()), (96.0, 0.0));
    }

    /// A press that misses the Scale, Shear or Rotate tool's handles is a selection
    /// click: the floor gets selected on release.
    #[test]
    fn a_press_off_a_tools_handles_selects_on_release() {
        for active in [Tool::Scale, Tool::Shear, Tool::Rotate] {
            let (mut document, floor, cube) = floor_and_cube();
            document.select_only(cube);
            let viewport = (800., 600.);
            let (drag, camera) = top_view(document);
            *drag.tool.borrow_mut() = active;
            let at = camera.world_to_screen(Vec3d::new(-400.0, -400.0, 0.0), viewport);
            drag_from(
                &drag,
                at,
                Vec2d::new(0.0, 0.0),
                Modifiers::default(),
                viewport,
            );
            let document = drag.document.borrow();
            assert_eq!(
                document.selection_ids().iter().copied().collect::<Vec<_>>(),
                vec![floor],
                "{active:?}"
            );
        }
    }

    /// With the vertex tool and nothing picked, a click on a brush selects it; a Ctrl-click
    /// on a picked vertex unpicks it.
    #[test]
    fn handle_tool_clicks_select_brushes_and_unpick_handles() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Vertex;
        let at = camera.world_to_screen(Vec3d::new(-400.0, -400.0, 0.0), viewport);
        drag_from(
            &drag,
            at,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        assert_eq!(
            drag.document
                .borrow()
                .selection_ids()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![floor]
        );

        drag.document.borrow_mut().select_only(cube);
        let corner = camera.world_to_screen(Vec3d::new(64.0, 64.0, 0.0), viewport);
        drag_from(
            &drag,
            corner,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        assert!(
            drag.handles.borrow().selected_count() > 0,
            "the corner was not picked"
        );
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        drag_from(&drag, corner, Vec2d::new(0.0, 0.0), ctrl, viewport);
        assert_eq!(drag.handles.borrow().selected_count(), 0);
        assert!(drag.document.borrow().is_selected(cube));
    }

    /// `PaintSelectionDragTracker`: a Ctrl-drag from the unselected floor over the cube
    /// adds both and draws nothing; Esc in the middle of one puts the selection back.
    #[test]
    fn a_ctrl_drag_paints_the_selection_in_a_flat_view() {
        let (document, floor, cube) = floor_and_cube();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(-200.0, -200.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        drag_from(&drag, from, to - from, ctrl, viewport);
        {
            let document = drag.document.borrow();
            assert!(document.is_selected(floor) && document.is_selected(cube));
            assert!(!document.has_shape_preview());
        }

        drag.document.borrow_mut().clear_selection();
        drag.begin(from.x(), from.y(), ctrl, viewport);
        let by = to - from;
        drag.update(by.x(), by.y(), ctrl, viewport);
        assert!(drag.document.borrow().is_selected(cube));
        assert!(drag.escape());
        assert!(!drag.document.borrow().has_selection());
    }

    /// `NodeHandleToolBase::startMove`: a Ctrl-drag of a vertex that is not picked moves
    /// that vertex alone, though a Ctrl-click would have added it to what is picked.
    #[test]
    fn a_ctrl_drag_of_an_unpicked_vertex_moves_only_it() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Vertex;
        let picked = camera.world_to_screen(Vec3d::new(64.0, 64.0, 0.0), viewport);
        drag_from(
            &drag,
            picked,
            Vec2d::new(0.0, 0.0),
            Modifiers::default(),
            viewport,
        );
        assert!(drag.handles.borrow().selected_count() > 0);

        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        let from = camera.world_to_screen(Vec3d::new(0.0, 0.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(-16.0, 0.0, 0.0), viewport);
        drag_from(&drag, from, to - from, ctrl, viewport);
        let vertices = drag
            .document
            .borrow()
            .brush(cube)
            .unwrap()
            .vertex_positions();
        assert!(
            vertices.contains(&Vec3d::new(64.0, 64.0, 0.0)),
            "the picked corner moved"
        );
        assert!(
            vertices.contains(&Vec3d::new(-16.0, 0.0, 0.0)),
            "the dragged corner stayed"
        );
    }

    /// `NodeHandleToolMovePartBase::shouldStartMove`: the edge tool does not move a handle
    /// with Ctrl held, so a Ctrl-drag from one paints the selection, and a Ctrl-click
    /// picks or unpicks it.
    #[test]
    fn a_ctrl_drag_from_an_edge_handle_paints_the_selection_in_a_flat_view() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(cube);
        let before = document.to_map_text();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Edge;
        let edge = Vec3d::new(32.0, 0.0, 0.0);
        let from = camera.world_to_screen(edge, viewport);
        let to = camera.world_to_screen(Vec3d::new(-150.0, -60.0, 0.0), viewport);
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        drag_from(&drag, from, to - from, ctrl, viewport);
        assert!(drag.document.borrow().is_selected(floor));
        assert!(drag.document.borrow().is_selected(cube));
        assert_eq!(drag.handles.borrow().selected_count(), 0);
        drag.document.borrow_mut().select_only(cube);
        assert_eq!(drag.document.borrow().to_map_text(), before);

        drag_from(&drag, from, Vec2d::new(0.0, 0.0), ctrl, viewport);
        assert_eq!(drag.handles.borrow().selected_count(), 1);
        drag_from(&drag, from, Vec2d::new(0.0, 0.0), ctrl, viewport);
        assert_eq!(drag.handles.borrow().selected_count(), 0);
    }

    /// `SelectionTool::drillSelection`: Ctrl and the wheel move the selection to the next
    /// thing under the pointer and back. In a flat view the smaller looks nearer.
    #[test]
    fn the_wheel_drills_the_selection_through_what_is_under_the_pointer() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(cube);
        let ray = geometry_math::ray::Ray3d::new(
            Vec3d::new(32.0, 32.0, 1000.0),
            Vec3d::new(0.0, 0.0, -1.0),
        );
        assert!(document.drill_selection(&ray, true, true));
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![floor]
        );
        assert!(
            !document.drill_selection(&ray, true, true),
            "nothing past the floor"
        );
        assert!(document.drill_selection(&ray, false, true));
        assert_eq!(
            document.selection_ids().iter().copied().collect::<Vec<_>>(),
            vec![cube]
        );
    }

    /// Shift+Alt draws a cube in a flat view: the depth as long as the longer side.
    #[test]
    fn shift_alt_draws_a_cube_in_a_flat_view() {
        let (document, _, _) = floor_and_cube();
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let from = camera.world_to_screen(Vec3d::new(-256.0, -256.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(-128.0, -192.0, 0.0), viewport);
        let document = drag.document.borrow();
        let bounds = swept_bounds(
            &camera,
            &Grid::new(4),
            from,
            to,
            viewport,
            &document,
            true,
            true,
        )
        .unwrap();
        assert_eq!(bounds.max - bounds.min, Vec3d::new(128.0, 128.0, 128.0));
    }

    /// `ScaleToolController`'s 2D case: Shift scales the visible axes alike and leaves
    /// the one the view looks along.
    #[test]
    fn proportional_scaling_in_a_flat_view_leaves_the_depth() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Scale;
        let from = camera.world_to_screen(Vec3d::new(64.0, 32.0, 0.0), viewport);
        let to = camera.world_to_screen(Vec3d::new(128.0, 32.0, 0.0), viewport);
        drag.begin(from.x(), from.y(), Modifiers::default(), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let by = to - from;
        drag.update(by.x(), by.y(), shift, viewport);
        drag.end(by.x(), by.y(), shift, viewport);
        let bounds = drag.document.borrow().brush(cube).unwrap().bounds();
        assert_eq!(bounds.max.x() - bounds.min.x(), 128.0);
        assert_eq!(bounds.max.y() - bounds.min.y(), 128.0);
        assert_eq!((bounds.min.z(), bounds.max.z()), (0.0, 64.0));
    }

    /// `ExtrudeTool::pick2D`: over a selected brush, Shift takes an edge only within the
    /// handle radius; in the middle of the brush it is a face click instead.
    #[test]
    fn shift_in_the_middle_of_a_selected_brush_does_not_extrude() {
        let (mut document, _, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        let at = camera.world_to_screen(Vec3d::new(32.0, 32.0, 0.0), viewport);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        drag.begin(at.x(), at.y(), shift, viewport);
        assert_ne!(drag.drag.borrow().mode, DragMode::Extruding);
        drag.end(0.0, 0.0, shift, viewport);
        assert_eq!(drag.document.borrow().selected_faces().len(), 1);
    }

    /// The clip tool places points with plain clicks only; Ctrl-click selects.
    #[test]
    fn a_modified_click_with_the_clip_tool_selects_instead_of_clipping() {
        let (mut document, floor, cube) = floor_and_cube();
        document.select_only(cube);
        let viewport = (800., 600.);
        let (drag, camera) = top_view(document);
        *drag.tool.borrow_mut() = Tool::Clip;
        let at = camera.world_to_screen(Vec3d::new(-300.0, -300.0, 0.0), viewport);
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        drag_from(&drag, at, Vec2d::new(0.0, 0.0), ctrl, viewport);
        assert!(!drag.clip.borrow().has_points());
        assert!(
            drag.document.borrow().is_selected(floor) && drag.document.borrow().is_selected(cube)
        );
    }

    /// Join brushes leaves everything it leaves selected, a brush with nothing to merge
    /// with included.
    #[test]
    fn join_keeps_what_it_leaves_selected() {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        for (min, max) in [
            (Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(64.0, 64.0, 64.0)),
            (Vec3d::new(64.0, 0.0, 0.0), Vec3d::new(128.0, 64.0, 64.0)),
            (
                Vec3d::new(512.0, 512.0, 0.0),
                Vec3d::new(576.0, 576.0, 64.0),
            ),
        ] {
            let brush =
                map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "stone").unwrap();
            document.add_brushes(vec![brush]);
        }
        document.select_all();
        assert_eq!(document.join_brushes(), Ok(1));
        assert_eq!(map_model::node::brushes(document.tree_world()).len(), 2);
        assert_eq!(document.selection_count(), 2);
    }

    /// Two cubes, one at the origin and one well away from it.
    fn two_cubes() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        for at in [0.0, 512.0] {
            let brush = map_model::shapes::create_cuboid(
                &world,
                &geometry_math::bbox::Bbox3d::new(
                    Vec3d::new(at, at, 0.0),
                    Vec3d::new(at + 64.0, at + 64.0, 64.0),
                ),
                "stone",
            )
            .expect("a cube");
            document.add_brushes(vec![brush]);
        }
        document.clear_selection();
        document
    }

    /// The rectangle on screen that a world box projects to, with a little air.
    fn rectangle_around(
        camera: &Camera2d,
        min: Vec3d,
        max: Vec3d,
        viewport: (f64, f64),
    ) -> (Vec2d, Vec2d) {
        let corners = [
            camera.world_to_screen(min, viewport),
            camera.world_to_screen(max, viewport),
        ];
        let left = corners[0].x().min(corners[1].x()) - 4.0;
        let right = corners[0].x().max(corners[1].x()) + 4.0;
        let top = corners[0].y().min(corners[1].y()) - 4.0;
        let bottom = corners[0].y().max(corners[1].y()) + 4.0;
        (Vec2d::new(left, top), Vec2d::new(right, bottom))
    }

    #[test]
    fn a_lasso_catches_what_lies_inside_it_and_nothing_else() {
        let document = two_cubes();
        let camera = Camera2d::new(ViewPlane::Top, 16384.0);
        let viewport = (800.0, 600.0);

        let (from, to) = rectangle_around(
            &camera,
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(64.0, 64.0, 64.0),
            viewport,
        );
        let caught = lassoed(&camera, &document, from, to, viewport);
        assert_eq!(caught.len(), 1, "the cube at the origin, and only that one");
    }

    #[test]
    fn a_brush_hanging_out_of_the_lasso_is_not_caught() {
        let document = two_cubes();
        let camera = Camera2d::new(ViewPlane::Top, 16384.0);
        let viewport = (800.0, 600.0);

        // A rectangle containing only half the cube must not select the whole brush.
        let (from, to) = rectangle_around(
            &camera,
            Vec3d::new(-64.0, -64.0, 0.0),
            Vec3d::new(32.0, 32.0, 64.0),
            viewport,
        );
        assert!(lassoed(&camera, &document, from, to, viewport).is_empty());
    }

    #[test]
    fn a_lasso_over_everything_catches_everything() {
        let document = two_cubes();
        let camera = Camera2d::new(ViewPlane::Top, 16384.0);
        let viewport = (800.0, 600.0);
        let whole = (Vec2d::new(-1e6, -1e6), Vec2d::new(1e6, 1e6));
        assert_eq!(
            lassoed(&camera, &document, whole.0, whole.1, viewport).len(),
            2
        );
    }

    #[test]
    fn an_empty_space_drag_draws_a_brush_without_a_tool_in_hand() {
        // Nothing selected, no Ctrl: the select tool draws, just like the brush tool.
        assert_eq!(
            empty_space_drag_mode(false, false, Tool::Select),
            DragMode::Creating
        );
        assert_eq!(
            empty_space_drag_mode(false, false, Tool::Brush),
            DragMode::Creating
        );
    }

    #[test]
    fn ctrl_drag_from_empty_space_is_the_marquee() {
        assert_eq!(
            empty_space_drag_mode(false, true, Tool::Select),
            DragMode::Lassoing
        );
    }

    #[test]
    fn a_plain_drag_that_only_deselects_does_not_also_draw() {
        // Something was selected and the press cleared it: the select tool stops there,
        // but the brush tool still draws.
        assert_eq!(
            empty_space_drag_mode(true, false, Tool::Select),
            DragMode::Idle
        );
        assert_eq!(
            empty_space_drag_mode(true, false, Tool::Brush),
            DragMode::Creating
        );
    }
}
