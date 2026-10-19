//! Directional shells, with optional open faces. Geometry is staged before editing.
use crate::{
    brush::Brush,
    polyhedron::{FromPlanes, Polyhedron},
};
use geometry_math::bbox::Bbox3d;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Which side of the original boundary the wall occupies.
pub enum Direction {
    /// Keep the outer boundary.
    #[default]
    Inward,
    /// Keep the inner boundary.
    Outward,
    /// Split the thickness equally across the original boundary.
    Centered,
}

/// Builds a shell and opens the indexed original faces without leaving a lid.
pub fn shell(
    brush: &Brush,
    world: &Bbox3d,
    thickness: f64,
    direction: Direction,
    open: &[usize],
) -> Result<Vec<Brush>, String> {
    if !thickness.is_finite() || thickness <= 0.0 {
        return Err("Use a positive finite wall thickness.".into());
    }
    if open.iter().any(|i| *i >= brush.faces().len()) {
        return Err("An opening refers to a missing face.".into());
    }
    let (outward, inward) = match direction {
        Direction::Inward => (0.0, thickness),
        Direction::Outward => (thickness, 0.0),
        Direction::Centered => (thickness * 0.5, thickness * 0.5),
    };
    let outer = if outward == 0.0 {
        brush.clone()
    } else {
        brush
            .expand(world, outward)
            .map_err(|_| "The outer shell does not fit the world bounds.")?
    };
    let planes: Vec<_> = brush
        .faces()
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut plane = f.plane;
            plane.distance += if open.contains(&i) {
                outward + thickness.max(1.0)
            } else {
                -inward
            };
            plane
        })
        .collect();
    let FromPlanes::Built { polyhedron, .. } = Polyhedron::from_planes(world, &planes, 1e-7)
        .map_err(|_| "The inner shell cannot be built reliably.")?
    else {
        return Err("The wall thickness consumes the inside of a brush.".into());
    };
    let inside = Brush::from_polyhedron(polyhedron, brush.faces()[0].material_name.as_str())
        .with_face_attributes_from(brush.faces());
    let walls = crate::csg::subtract(&outer, &inside);
    if walls.is_empty() {
        return Err("No walls remain with these openings.".into());
    }
    Ok(walls)
}
