//! Placing a point entity: where a new one goes, whether from a double-click in the
//! entity browser, a drag onto the 3D view along the pointer's ray, or a drag onto a
//! flat view onto a plane through the selection.

use std::cell::RefCell;
use std::rc::Rc;

use geometry_math::vec::Vec3d;

use crate::document::Document;
use crate::grid::Grid;

/// The class placed when the definitions say nothing.
///
/// Every Quake-lineage game has it, and it does nothing, so an entity placed without
/// definitions is a marker rather than a mistake.
pub const FALLBACK_CLASS: &str = "info_null";

/// Places one entity of the given class, on the grid, where the selection is.
///
/// Where a new thing goes is the same question the shape tools answer: at the middle of
/// what is selected, or at the origin when nothing is.
pub fn place_entity(document: &Rc<RefCell<Document>>, classname: &str, grid_size: f64) {
    let mut document = document.borrow_mut();
    let origin = match document.selection_bounds() {
        Some(bounds) => bounds.center(),
        None => Vec3d::zero(),
    };
    let snapped = geometry_math::vec::round(origin / grid_size) * grid_size;

    document.create_point_entity(classname, snapped);
    crate::say!("placed a {classname}");
}

/// How far down the ray an entity lands when the ray hits nothing.
///
/// `gl::Camera::DefaultPointDistance`, the same distance a drag from empty space in the
/// 3D view puts the first corner of a new brush at.
pub const DEFAULT_POINT_DISTANCE: f64 = 256.0;

/// The box the editor draws a class with, in the class's own frame.
///
/// The FGD's `size(...)` where the class gives one, and otherwise the default cube, which
/// is what [`Document::entity_boxes`] draws an entity with.
pub fn class_bounds(document: &Document, classname: &str) -> geometry_math::bbox::Bbox3d {
    document
        .definitions()
        .and_then(|definitions| definitions.find(classname))
        .and_then(|definition| definition.bounds)
        .unwrap_or_else(|| {
            geometry_math::bbox::Bbox3d::from_scalars(
                -crate::document::DEFAULT_ENTITY_RADIUS,
                crate::document::DEFAULT_ENTITY_RADIUS,
            )
        })
}

/// Places one entity of the given class where a ray through the 3D view lands.
///
/// `CreateEntityTool::updateEntityPosition3D`: the class's box is set down on the face
/// under the pointer, resting on it rather than cutting through it, and where the ray
/// meets nothing it goes a set distance down the ray with its centre on the grid. The
/// entity lands selected, in one undo step, as a placement from the button does.
pub fn place_entity_on_ray(
    document: &Rc<RefCell<Document>>,
    grid: &Grid,
    classname: &str,
    ray: &geometry_math::ray::Ray3d,
) -> bool {
    let mut document = document.borrow_mut();
    let origin = drop_origin_on_ray(&document, grid, classname, ray);
    document.create_point_entity(classname, origin);
    crate::say!("placed a {classname}");
    true
}

/// Where a drop into the 3D view would put the entity's origin.
///
/// Split out from [`place_entity_on_ray`] so that the view can draw the box a drop would
/// leave while the drag is still over it.
pub fn drop_origin_on_ray(
    document: &Document,
    grid: &Grid,
    classname: &str,
    ray: &geometry_math::ray::Ray3d,
) -> Vec3d {
    let bounds = class_bounds(document, classname);
    match document
        .pick(ray)
        .and_then(|hit| Some((document.brush(hit.node)?, hit.face?)))
        .map(|(brush, face)| {
            brush
                .geometry()
                .face_plane(map_model::polyhedron::FaceId(face as u32))
        }) {
        Some(plane) => grid.move_delta_for_bounds(&plane, &bounds, ray),
        None => {
            let landing = ray.origin + ray.direction * DEFAULT_POINT_DISTANCE;
            let centre = bounds.center();
            grid.move_delta_for_point(centre, landing - centre)
        }
    }
}

/// Places one entity of the given class where a ray through a flat view lands.
///
/// `CreateEntityTool::updateEntityPosition2D`. A flat view looks down an axis and its ray
/// meets nothing, so the entity is set down on a plane facing the viewer through the far
/// end of whatever is selected -- or of a small box at the origin when nothing is, which
/// is `Map::referenceBounds`. That keeps a drop into a flat view at the depth the person
/// is already working at rather than at the far edge of the world.
pub fn place_entity_on_reference_plane(
    document: &Rc<RefCell<Document>>,
    grid: &Grid,
    classname: &str,
    ray: &geometry_math::ray::Ray3d,
) -> bool {
    let mut document = document.borrow_mut();
    let origin = drop_origin_on_reference_plane(&document, grid, classname, ray);
    document.create_point_entity(classname, origin);
    crate::say!("placed a {classname}");
    true
}

/// Where a drop into a flat view would put the entity's origin.
pub fn drop_origin_on_reference_plane(
    document: &Document,
    grid: &Grid,
    classname: &str,
    ray: &geometry_math::ray::Ray3d,
) -> Vec3d {
    let bounds = class_bounds(document, classname);
    let reference = document
        .selection_bounds()
        .unwrap_or_else(|| geometry_math::bbox::Bbox3d::from_scalars(-16.0, 16.0));

    // Whichever end of the reference box is furthest along the ray is the depth to drop
    // at, so the box comes to rest on the viewer's side of the selection.
    let anchor = if geometry_math::vec::dot(reference.min - ray.origin, ray.direction)
        > geometry_math::vec::dot(reference.max - ray.origin, ray.direction)
    {
        reference.min
    } else {
        reference.max
    };
    let plane = geometry_math::plane::Plane3d::from_anchor_and_normal(anchor, -ray.direction);

    grid.move_delta_for_bounds(&plane, &bounds, ray)
}

/// The box a drop would leave, in the world, for the view to outline while the drag is
/// over it.
///
/// `flat` says which of the two placements the view uses: a flat view drops onto the
/// reference plane, the 3D view onto what the ray hits.
pub fn drop_preview_bounds(
    document: &Document,
    grid: &Grid,
    classname: &str,
    ray: &geometry_math::ray::Ray3d,
    flat: bool,
) -> geometry_math::bbox::Bbox3d {
    let origin = if flat {
        drop_origin_on_reference_plane(document, grid, classname, ray)
    } else {
        drop_origin_on_ray(document, grid, classname, ray)
    };
    class_bounds(document, classname).translate(origin)
}

/// The class a keyboard placement uses: whatever the browser has chosen, or the first
/// point class the definitions offer.
pub fn default_class(document: &Document) -> String {
    document
        .definitions()
        .and_then(|definitions| {
            definitions
                .placeable()
                .find(|definition| definition.kind == map_model::fgd::DefinitionKind::Point)
                .map(|definition| definition.name.clone())
        })
        .unwrap_or_else(|| FALLBACK_CLASS.to_string())
}
