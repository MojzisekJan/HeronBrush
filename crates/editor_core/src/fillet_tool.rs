//! The fillet tool: what a click on an edge does, and the settings it rounds by.
//!
//! A click selects the preview edges. Radius/segments remain editable until Apply or Enter.
//!
//! Ctrl is the same as everywhere else: it adds to the selection instead of doing
//! anything. That is how several edges, on one brush or across several, are gathered
//! before rounding them together, which is not the same as rounding each in turn -- the
//! corner where two of them meet comes out mitred rather than cut into.
//!
//! The rounding itself is [`map_model::fillet`], reached through the document so that it is one
//! undoable edit.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;
use crate::handle_tool::HandleManager;
use crate::i18n::{counted, fill, tr};

/// What a fillet is made of, shared by the panel that sets it and the click that uses it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilletParameters {
    /// How far back from the edge the rounding starts.
    pub radius: f64,
    /// Fill the inside of two overlapping rectangular L members.
    pub inner: bool,
    /// How many flat faces the rounding is made of. One is a chamfer.
    pub faces: usize,
    /// What happens where two or three rounded edges meet at a vertex.
    pub corners: map_model::fillet::Corners,
    /// Use the fillet grid independently of the editor movement grid.
    pub snap: bool,
    pub grid: f64,
}

impl Default for FilletParameters {
    /// A radius of one grid step at the size most maps are built at, enough faces to read
    /// as round rather than as a cut corner, and the mitre a corner gets on its own.
    fn default() -> Self {
        Self {
            radius: 8.0,
            inner: false,
            faces: 4,
            corners: map_model::fillet::Corners::Mitred,
            snap: false,
            grid: 1.0,
        }
    }
}

impl FilletParameters {
    /// The shape to hand the rounding.
    pub fn shape(self) -> map_model::fillet::Fillet {
        map_model::fillet::Fillet {
            radius: self.radius,
            faces: self.faces,
            corners: self.corners,
            snap: self.snap.then_some(self.grid),
        }
    }
}

/// What a press on an edge handle with the fillet tool did, in words for the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The press reached no edge.
    Missed,
    /// The edge was added to, or taken out of, the selection.
    Picked,
    /// Edges on this many brushes were rounded.
    Rounded(usize),
    /// The rounding could not be done, and nothing changed.
    Refused(String),
}

/// Select source brushes without leaving Fillet when the click missed its edge handles.
/// Ctrl toggles brushes and keeps pinned edges that still belong to the selection.
pub fn select_brush(
    document: &mut Document,
    handles: &mut HandleManager,
    hit: Option<map_model::node::NodeId>,
    additive: bool,
) {
    let hit = hit.filter(|id| document.brush(*id).is_some());
    match hit {
        Some(id) if additive => document.toggle_selected(id),
        Some(id) => {
            document.select_only(id);
            handles.clear_selection();
        }
        None if !additive => {
            document.clear_selection();
            handles.clear_selection();
        }
        None => {}
    }
    handles.collect(
        crate::handle_tool::HandleKind::Edge,
        document.selected_brushes().into_iter(),
    );
}

/// Handles a press on an edge handle with the fillet tool active.
///
/// A plain press pins an edge preview and preserves a picked multi-edge set.
/// Ctrl toggles edges; Apply or Enter commits the current preview.
pub fn press(
    _document: &Rc<RefCell<Document>>,
    handles: &mut HandleManager,
    parameters: FilletParameters,
    position: geometry_math::vec::Vec3d,
    add_to_selection: bool,
) -> Outcome {
    if add_to_selection {
        handles.select(position, true);
        return Outcome::Picked;
    }

    // A click pins the preview; only Apply/Enter changes the source geometry.
    if parameters.inner {
        return Outcome::Picked;
    }
    if !handles.is_selected(position) {
        handles.select(position, false);
    }
    if handles.selected_edges().is_empty() {
        Outcome::Missed
    } else {
        Outcome::Picked
    }
}

/// Rounds whatever is picked, and says what happened.
///
/// What the panel's button does and what Return does: the picked edges, or the edges
/// between the picked vertices, or -- with neither -- the loop around each selected face.
/// Returns whether anything was rounded.
pub fn apply(
    document: &Rc<RefCell<Document>>,
    handles: &mut HandleManager,
    parameters: FilletParameters,
) -> Outcome {
    if parameters.inner {
        return apply_inner(document, handles, parameters);
    }
    let edges = edges_to_round(&document.borrow(), handles, None);
    let outcome = if edges.is_empty() {
        if document.borrow().face_selection_count() == 0 {
            return Outcome::Missed;
        }
        document.borrow_mut().fillet_face_loops(&parameters.shape())
    } else {
        document
            .borrow_mut()
            .fillet_edges(&edges, &parameters.shape())
    };

    match outcome {
        Ok(0) => Outcome::Refused("nothing was rounded".to_string()),
        Ok(count) => {
            // What was rounded is gone, so what was picked names nothing now.
            handles.clear_selection();
            Outcome::Rounded(count)
        }
        Err(why) => Outcome::Refused(why.to_string()),
    }
}

fn inner_options(parameters: FilletParameters) -> crate::corner_join::Rounding {
    crate::corner_join::Rounding {
        inner: Some((parameters.radius, parameters.faces)),
        snap: parameters.snap.then_some(parameters.grid),
        ..Default::default()
    }
}

fn apply_inner(
    document: &Rc<RefCell<Document>>,
    handles: &mut HandleManager,
    parameters: FilletParameters,
) -> Outcome {
    let revision = document.borrow().revision();
    let result = document
        .borrow_mut()
        .join_corner_options(revision, inner_options(parameters));
    match result {
        Ok(()) => {
            handles.clear_selection();
            Outcome::Rounded(2)
        }
        Err(error) => Outcome::Refused(error),
    }
}

/// Says what a press did, the way the other tools report themselves.
pub fn report(outcome: &Outcome) {
    match outcome {
        Outcome::Picked => {}
        Outcome::Missed => crate::say!(
            "{}",
            tr("Nothing to round. Pick edges or vertices, or select faces to round their loops")
        ),
        Outcome::Rounded(count) => crate::say!(
            "{}",
            fill("Rounded edges on {}", &[&counted(*count, "brush")])
        ),
        Outcome::Refused(why) => {
            crate::say!("{}", fill("The edge was not rounded: {}", &[why]))
        }
    }
}

/// The edges a press would round, given what is picked and what the pointer is over.
///
/// The picked edges, or -- with none picked -- the one under the pointer, which is what
/// makes the preview follow the pointer before anything has been clicked.
pub fn edges_to_round(
    document: &Document,
    handles: &HandleManager,
    hovered: Option<geometry_math::vec::Vec3d>,
) -> Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> {
    let picked = handles.selected_edges();
    if !picked.is_empty() {
        return picked;
    }
    // Vertices picked with the vertex tool say which edges to round as well: the ones
    // running between them. Two neighbouring vertices name one edge, and the eight of a
    // cube name all twelve.
    let between = edges_between_picked_vertices(document, handles);
    if !between.is_empty() {
        return between;
    }
    let Some(hovered) = hovered else {
        return Vec::new();
    };
    handles
        .handles()
        .iter()
        .find(|handle| geometry_math::vec::is_zero(handle.position - hovered, 0.001))
        .and_then(|handle| match handle.vertices.as_slice() {
            [from, to] => Some(vec![(*from, *to)]),
            _ => None,
        })
        .unwrap_or_default()
}

/// The edges whose two ends are both picked with the vertex tool.
///
/// A vertex is not an edge, so picking vertices says nothing on its own; what it does say
/// is which edges lie between them, which is what a fillet can act on. Nothing comes back
/// unless the vertex tool's handles are what is picked, and unless two or more are.
pub fn edges_between_picked_vertices(
    document: &Document,
    handles: &HandleManager,
) -> Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> {
    if handles.kind() != Some(crate::handle_tool::HandleKind::Vertex)
        || handles.selected_count() < 2
    {
        return Vec::new();
    }

    let mut edges = Vec::new();
    for brush in document.selected_brushes() {
        for (from, to) in brush.edge_positions() {
            if handles.is_selected(from)
                && handles.is_selected(to)
                && !edges.iter().any(|(a, b)| same_edge((*a, *b), (from, to)))
            {
                edges.push((from, to));
            }
        }
    }
    edges
}

/// Whether two edges are the same one, whichever way round each is given.
fn same_edge(
    one: (geometry_math::vec::Vec3d, geometry_math::vec::Vec3d),
    other: (geometry_math::vec::Vec3d, geometry_math::vec::Vec3d),
) -> bool {
    let same = |a: geometry_math::vec::Vec3d, b: geometry_math::vec::Vec3d| {
        geometry_math::vec::is_zero(a - b, 0.001)
    };
    (same(one.0, other.0) && same(one.1, other.1)) || (same(one.0, other.1) && same(one.1, other.0))
}

/// The outline a fillet would leave: the edges of the brushes it would make.
///
/// Worked out from the brushes as they are, without changing anything, so a view can draw
/// it while the pointer is still moving. A brush the rounding would fail on -- too large a
/// radius -- contributes nothing, which is what makes the preview disappear at the radius
/// the press would refuse.
pub fn preview_edges(
    document: &Document,
    edges: &[(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)],
    parameters: FilletParameters,
) -> Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> {
    if parameters.inner {
        return document
            .preview_corner_join_options(inner_options(parameters))
            .map(|draft| {
                draft
                    .join
                    .after
                    .iter()
                    .chain(&draft.join.added)
                    .flat_map(|brush| brush.edge_positions())
                    .collect()
            })
            .unwrap_or_default();
    }
    let selected = document.selected_faces();
    let owned_edges: Vec<_> = if edges.is_empty() {
        selected
            .iter()
            .filter_map(|f| {
                document.brush(f.node).and_then(|b| {
                    b.geometry_face_of(f.face)
                        .map(|face| map_model::fillet::face_edges(b, face))
                })
            })
            .flatten()
            .collect()
    } else {
        Vec::new()
    };
    let edges = if edges.is_empty() {
        owned_edges.as_slice()
    } else {
        edges
    };
    if edges.is_empty() {
        return Vec::new();
    }
    let mut brushes = document.selected_brushes();
    if !selected.is_empty() {
        for face in &selected {
            if let Some(brush) = document.brush(face.node) {
                if !brushes.iter().any(|b| std::ptr::eq(*b, brush)) {
                    brushes.push(brush);
                }
            }
        }
    }
    let mut outline = Vec::new();
    for brush in brushes {
        let mine: Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> = edges
            .iter()
            .copied()
            .filter(|(from, to)| map_model::fillet::has_edge(brush, *from, *to))
            .collect();
        if mine.is_empty() {
            continue;
        }
        if let Ok(rounded) = map_model::fillet::fillet_edges(brush, &mine, &parameters.shape()) {
            outline.extend(rounded.edge_positions());
        }
    }
    outline
}

/// Temporary replacement surfaces, never inserted into the map or its history.
#[derive(Default)]
pub struct SurfacePreview {
    pub brushes: Vec<(map_model::node::NodeId, Vec<map_model::brush::Brush>)>,
}

/// What a preview was made from: document generation and revision, parameters and edges.
type PreviewKey = (
    u64,
    u64,
    FilletParameters,
    Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)>,
);

#[derive(Default)]
pub struct PreviewCache {
    key: Option<PreviewKey>,
    value: Option<Rc<SurfacePreview>>,
}

impl PreviewCache {
    pub fn get(
        &mut self,
        document: &Document,
        edges: Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)>,
        parameters: FilletParameters,
    ) -> Option<Rc<SurfacePreview>> {
        let key = (
            document.generation(),
            document.revision(),
            parameters,
            edges,
        );
        if self.key.as_ref() != Some(&key) {
            self.value = surface_preview(document, &key.3, parameters)
                .ok()
                .filter(|p| !p.brushes.is_empty())
                .map(Rc::new);
            self.key = Some(key);
        }
        self.value.clone()
    }
}

/// Uses the same all-or-nothing geometry as Apply, including face loops and inner fills.
pub fn surface_preview(
    document: &Document,
    edges: &[(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)],
    parameters: FilletParameters,
) -> Result<SurfacePreview, String> {
    if parameters.inner {
        let draft = document.preview_corner_join_options(inner_options(parameters))?;
        let [a, b] = draft.join.after;
        let mut first = vec![a];
        first.extend(draft.join.added);
        return Ok(SurfacePreview {
            brushes: vec![(draft.ids[0], first), (draft.ids[1], vec![b])],
        });
    }
    let mut result = SurfacePreview::default();
    let faces = document.selected_faces();
    for (id, brush) in map_model::node::brushes_with_ids(document.tree_world()) {
        let mine: Vec<_> = if edges.is_empty() {
            faces
                .iter()
                .filter(|f| f.node == id)
                .filter_map(|f| brush.geometry_face_of(f.face))
                .flat_map(|face| map_model::fillet::face_edges(brush, face))
                .collect()
        } else if document.selection_ids().contains(&id) {
            edges
                .iter()
                .copied()
                .filter(|(a, b)| map_model::fillet::has_edge(brush, *a, *b))
                .collect()
        } else {
            Vec::new()
        };
        if mine.is_empty() {
            continue;
        }
        let rounded = map_model::fillet::fillet_edges(brush, &mine, &parameters.shape())
            .map_err(|e| e.to_string())?;
        result.brushes.push((id, vec![rounded]));
    }
    Ok(result)
}

#[cfg(test)]
mod vertex_tests {
    use super::*;
    use crate::handle_tool::{HandleKind, HandleManager};
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;

    /// One cube from -32 to 32, selected, with its vertex handles collected.
    fn cube_with_vertex_handles() -> (Document, HandleManager) {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(-32.0, -32.0, -32.0),
                Vec3d::new(32.0, 32.0, 32.0),
            ),
            "stone",
        )
        .expect("a cube");
        document.add_brushes(vec![brush]);
        document.select_all();

        let mut handles = HandleManager::new();
        handles.collect(HandleKind::Vertex, document.selected_brushes().into_iter());
        (document, handles)
    }

    #[test]
    fn two_picked_vertices_name_the_edge_between_them() {
        let (document, mut handles) = cube_with_vertex_handles();
        handles.select(Vec3d::new(-32.0, -32.0, 32.0), false);
        handles.select(Vec3d::new(32.0, -32.0, 32.0), true);

        let edges = edges_between_picked_vertices(&document, &handles);
        assert_eq!(edges.len(), 1, "the one edge running between them");
        assert!(edges_to_round(&document, &handles, None).len() == 1);
    }

    #[test]
    fn two_vertices_that_are_not_neighbours_name_no_edge() {
        let (document, mut handles) = cube_with_vertex_handles();
        // Opposite corners of the cube: nothing runs between them.
        handles.select(Vec3d::new(-32.0, -32.0, -32.0), false);
        handles.select(Vec3d::new(32.0, 32.0, 32.0), true);
        assert!(edges_between_picked_vertices(&document, &handles).is_empty());
    }

    #[test]
    fn every_vertex_picked_names_every_edge() {
        let (document, mut handles) = cube_with_vertex_handles();
        handles.select_all();
        assert_eq!(edges_between_picked_vertices(&document, &handles).len(), 12);
    }

    #[test]
    fn one_vertex_names_nothing_and_edge_handles_are_left_to_themselves() {
        let (document, mut handles) = cube_with_vertex_handles();
        handles.select(Vec3d::new(-32.0, -32.0, 32.0), false);
        assert!(edges_between_picked_vertices(&document, &handles).is_empty());

        // With the edge tool's own handles the edges are picked directly, and this has
        // nothing to add.
        let mut edges = HandleManager::new();
        edges.collect(HandleKind::Edge, document.selected_brushes().into_iter());
        edges.select_all();
        assert!(edges_between_picked_vertices(&document, &edges).is_empty());
        assert_eq!(edges.selected_edges().len(), 12);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_tool::HandleKind;
    use crate::world::WorldBounds;
    use geometry_math::vec::Vec3d;

    /// Two boxes side by side, both selected, with the edge handles collected.
    fn two_boxes() -> (Rc<RefCell<Document>>, HandleManager) {
        let bounds = WorldBounds::default();
        let world = geometry_math::bbox::Bbox3d::new(
            Vec3d::fill(-bounds.half_extent()),
            Vec3d::fill(bounds.half_extent()),
        );
        let box_at = |x: f64| {
            map_model::shapes::create_cuboid(
                &world,
                &geometry_math::bbox::Bbox3d::new(
                    Vec3d::new(x, -16.0, -16.0),
                    Vec3d::new(x + 32.0, 16.0, 16.0),
                ),
                "stone",
            )
            .expect("the box is built")
        };

        let mut document = Document::new(bounds);
        document.add_brushes(vec![box_at(-16.0), box_at(64.0)]);
        document.select_all();

        let mut handles = HandleManager::new();
        handles.collect(HandleKind::Edge, document.selected_brushes().into_iter());
        (Rc::new(RefCell::new(document)), handles)
    }

    #[test]
    fn brush_selection_while_fillet_is_active_keeps_valid_pinned_edges() {
        let (document, mut handles) = two_boxes();
        let ids: Vec<_> = document
            .borrow()
            .selectable_brushes()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let before = document.borrow().to_map_text();
        document.borrow_mut().clear_selection();
        handles.clear();
        select_brush(
            &mut document.borrow_mut(),
            &mut handles,
            Some(ids[0]),
            false,
        );
        assert_eq!(document.borrow().selection_count(), 1);
        let edge = handles.handles()[0].position;
        handles.select(edge, false);
        select_brush(&mut document.borrow_mut(), &mut handles, Some(ids[1]), true);
        assert_eq!(document.borrow().selection_count(), 2);
        assert!(handles.is_selected(edge));
        select_brush(&mut document.borrow_mut(), &mut handles, Some(ids[1]), true);
        assert_eq!(document.borrow().selection_count(), 1);
        assert!(handles.is_selected(edge));
        select_brush(&mut document.borrow_mut(), &mut handles, None, true);
        assert_eq!(document.borrow().selection_count(), 1);
        assert!(handles.is_selected(edge));
        select_brush(
            &mut document.borrow_mut(),
            &mut handles,
            Some(ids[1]),
            false,
        );
        assert_eq!(document.borrow().selection_count(), 1);
        assert!(!handles.is_selected(edge));
        select_brush(&mut document.borrow_mut(), &mut handles, None, false);
        assert_eq!(document.borrow().selection_count(), 0);
        assert_eq!(handles.selected_count(), 0);
        assert_eq!(document.borrow().to_map_text(), before);
    }

    /// The handle in the middle of the given edge.
    fn handle_at(handles: &HandleManager, from: Vec3d, to: Vec3d) -> Vec3d {
        let wanted = (from + to) / 2.0;
        handles
            .handles()
            .iter()
            .map(|handle| handle.position)
            .find(|position| geometry_math::vec::is_zero(*position - wanted, 0.001))
            .expect("that edge has a handle")
    }

    #[test]
    fn a_plain_press_pins_an_editable_preview_until_apply() {
        let (document, mut handles) = two_boxes();
        let position = handle_at(
            &handles,
            Vec3d::new(16.0, -16.0, 16.0),
            Vec3d::new(16.0, 16.0, 16.0),
        );

        let before = document.borrow().to_map_text();
        let outcome = press(
            &document,
            &mut handles,
            FilletParameters::default(),
            position,
            false,
        );
        assert_eq!(outcome, Outcome::Picked);
        assert_eq!(document.borrow().to_map_text(), before);
        let settings = FilletParameters {
            radius: 4.0,
            ..Default::default()
        };
        assert!(!preview_edges(&document.borrow(), &handles.selected_edges(), settings).is_empty());
        assert_eq!(document.borrow().to_map_text(), before);
        assert_eq!(
            apply(&document, &mut handles, settings),
            Outcome::Rounded(1)
        );

        let document = document.borrow();
        let faces: Vec<usize> = document
            .selected_brushes()
            .iter()
            .map(|brush| brush.geometry().face_count())
            .collect();
        assert!(
            faces.contains(&(6 + 4)),
            "one box gained the fillet's faces"
        );
        assert!(faces.contains(&6), "the other box is untouched");
    }

    #[test]
    fn ctrl_picks_multiple_preview_edges_and_apply_rounds_the_lot() {
        let (document, mut handles) = two_boxes();
        let one = handle_at(
            &handles,
            Vec3d::new(16.0, -16.0, 16.0),
            Vec3d::new(16.0, 16.0, 16.0),
        );
        let other = handle_at(
            &handles,
            Vec3d::new(96.0, -16.0, 16.0),
            Vec3d::new(96.0, 16.0, 16.0),
        );

        let settings = FilletParameters::default();
        assert_eq!(
            press(&document, &mut handles, settings, one, true),
            Outcome::Picked
        );
        assert_eq!(
            press(&document, &mut handles, settings, other, true),
            Outcome::Picked
        );
        assert_eq!(handles.selected_count(), 2);

        // A plain press keeps the multi-edge preview; Apply commits both.
        assert_eq!(
            press(&document, &mut handles, settings, one, false),
            Outcome::Picked
        );
        assert_eq!(
            apply(&document, &mut handles, settings),
            Outcome::Rounded(2)
        );
        for brush in document.borrow().selected_brushes() {
            assert_eq!(brush.geometry().face_count(), 6 + 4);
        }
    }

    #[test]
    fn a_radius_the_brush_cannot_give_up_changes_nothing() {
        let (document, mut handles) = two_boxes();
        let position = handle_at(
            &handles,
            Vec3d::new(16.0, -16.0, 16.0),
            Vec3d::new(16.0, 16.0, 16.0),
        );

        let settings = FilletParameters {
            radius: 512.0,
            inner: false,
            faces: 4,
            corners: map_model::fillet::Corners::Mitred,
            ..Default::default()
        };
        assert_eq!(
            press(&document, &mut handles, settings, position, false),
            Outcome::Picked
        );
        assert!(matches!(
            apply(&document, &mut handles, settings),
            Outcome::Refused(_)
        ));
        for brush in document.borrow().selected_brushes() {
            assert_eq!(brush.geometry().face_count(), 6);
        }
    }
}
