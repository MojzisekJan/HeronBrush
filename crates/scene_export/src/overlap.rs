//! Finds brushes that share space.
//!
//! Meeting face to face is how a level is built. Insides overlapping is not:
//! the buried faces are never seen, and where two land on the same plane the
//! renderer cannot choose between them and the surface flickers.

use crate::map::{Brush, Entity};
use crate::math::{self, Aabb, Plane, Vec3};
use rayon::prelude::*;

/// One pair of brushes that share space, named the way the editor names them.
#[derive(Debug, Clone)]
pub struct Overlap {
    pub first: Where,
    pub second: Where,
    /// Roughly where they meet, in map units, so it can be typed into
    /// the editor's camera position.
    pub at: Vec3,
    /// How far one would have to move to come apart, in map units. Small
    /// numbers are usually a brush a unit or two out of place.
    pub depth: f64,
}

/// Which brush, in the terms the mapper uses: the entity it belongs to and
/// which brush of that entity it is.
#[derive(Debug, Clone)]
pub struct Where {
    pub entity: usize,
    pub classname: String,
    pub brush: usize,
}

impl std::fmt::Display for Where {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (entity {}) brush {}",
            self.classname, self.entity, self.brush
        )
    }
}

impl std::fmt::Display for Overlap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} and {} share space around ({:.0} {:.0} {:.0}), {:.1} units deep",
            self.first, self.second, self.at.x, self.at.y, self.at.z, self.depth
        )
    }
}

/// A brush reduced to what the test needs.
struct Solid {
    place: Where,
    points: Vec<Vec3>,
    planes: Vec<Plane>,
    bounds: Aabb,
}

/// How far two brushes have to be into each other before it counts. Faces that
/// touch land a hair either side of their shared plane.
const TOUCHING: f64 = 0.05;

/// Every pair of brushes that share more than a touch, in map units.
///
/// `skip` leaves out volumes meant to sit inside the level, such as triggers.
pub fn find(entities: &[Entity], skip: impl Fn(&Entity, &Brush) -> bool) -> Vec<Overlap> {
    // Cutting the corners out is the expensive part and each brush stands alone,
    // so a big level does it on every core.
    let wanted: Vec<(usize, usize)> = entities
        .iter()
        .enumerate()
        .flat_map(|(ei, entity)| {
            entity
                .brushes
                .iter()
                .enumerate()
                .filter(|(_, brush)| !skip(entity, brush))
                .map(move |(bi, _)| (ei, bi))
        })
        .collect();
    let make = |(entity_index, brush_index): &(usize, usize)| -> Option<Solid> {
        let entity = &entities[*entity_index];
        let planes = entity.brushes[*brush_index].planes();
        let points = corners_of(&planes);
        let bounds = Aabb::from_points(points.iter().copied())?;
        Some(Solid {
            place: Where {
                entity: *entity_index,
                classname: entity.classname().to_string(),
                brush: *brush_index,
            },
            points,
            planes,
            bounds,
        })
    };
    let solids: Vec<Solid> = if wanted.len() >= 64 {
        wanted.par_iter().filter_map(make).collect()
    } else {
        wanted.iter().filter_map(make).collect()
    };

    // Sorted by where each brush starts, so the sweep can stop early.
    let mut order: Vec<usize> = (0..solids.len()).collect();
    order.sort_by(|a, b| {
        solids[*a]
            .bounds
            .min
            .x
            .partial_cmp(&solids[*b].bounds.min.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut found = Vec::new();
    for (i, &a) in order.iter().enumerate() {
        for &b in order.iter().skip(i + 1) {
            // Past the end of this one in x, and so is everything after it.
            if solids[b].bounds.min.x > solids[a].bounds.max.x - TOUCHING {
                break;
            }
            if !boxes_meet(&solids[a].bounds, &solids[b].bounds) {
                continue;
            }
            if let Some((depth, at)) = overlap_of(&solids[a], &solids[b]) {
                found.push(Overlap {
                    first: solids[a].place.clone(),
                    second: solids[b].place.clone(),
                    at,
                    depth,
                });
            }
        }
    }
    found
}

fn boxes_meet(a: &Aabb, b: &Aabb) -> bool {
    a.min.x < b.max.x - TOUCHING
        && b.min.x < a.max.x - TOUCHING
        && a.min.y < b.max.y - TOUCHING
        && b.min.y < a.max.y - TOUCHING
        && a.min.z < b.max.z - TOUCHING
        && b.min.z < a.max.z - TOUCHING
}

/// The corners of a convex brush, from its planes. The same clipping the mesh
/// builder does, kept separate because this runs before anything is built.
pub fn corners_of(planes: &[Plane]) -> Vec<Vec3> {
    corners_within(planes, Vec3::splat(-1.0e5), Vec3::splat(1.0e5))
}

/// [`corners_of`] when the brush is already known to sit inside `min`..`max`: the base
/// windings are sized to that box instead of to 100000 units, so a large merged plane
/// set does not spend its time clipping enormous polygons down.
pub fn corners_within(planes: &[Plane], min: Vec3, max: Vec3) -> Vec<Vec3> {
    let around = (min + max) * 0.5;
    // Half the box diagonal, with room to spare, so the square always covers the face.
    let size = ((max - min).length() * 0.75).max(1.0);
    let mut points: Vec<Vec3> = Vec::new();
    let mut winding: Vec<Vec3> = Vec::new();
    let mut scratch: Vec<Vec3> = Vec::new();
    for (i, plane) in planes.iter().enumerate() {
        math::base_winding_around_into(&mut winding, plane, around, size);
        for (j, other) in planes.iter().enumerate() {
            if i == j {
                continue;
            }
            math::clip_polygon_in_place(&mut winding, &mut scratch, other, 1.0e-4);
            if winding.is_empty() {
                break;
            }
        }
        math::dedup_winding_in_place(&mut winding, 1.0e-3);
        points.extend_from_slice(&winding);
    }
    points
}

/// Whether two convex brushes share space, and by how much.
///
/// The separating axis theorem. Face normals alone are not enough for solids
/// at an angle, where the separating plane runs along an edge of each, so the
/// cross products of the edge directions go in as well.
fn overlap_of(a: &Solid, b: &Solid) -> Option<(f64, Vec3)> {
    let mut axes: Vec<Vec3> = Vec::with_capacity(64);
    axes.extend(a.planes.iter().map(|p| p.normal));
    axes.extend(b.planes.iter().map(|p| p.normal));
    for one in a.planes.iter() {
        for two in b.planes.iter() {
            let cross = one.normal.cross(two.normal);
            if cross.length_squared() > 1.0e-8 {
                axes.push(cross.normalize());
            }
        }
    }

    let mut least = f64::INFINITY;
    for axis in axes {
        let (a_min, a_max) = span(&a.points, axis);
        let (b_min, b_max) = span(&b.points, axis);
        let overlap = (a_max.min(b_max)) - (a_min.max(b_min));
        if overlap <= TOUCHING {
            return None;
        }
        least = least.min(overlap);
    }
    // Where to send the mapper. The middle of the shared box is close enough to
    // find it in the editor.
    let shared = Aabb {
        min: a.bounds.min.max(b.bounds.min),
        max: a.bounds.max.min(b.bounds.max),
    };
    Some((least, shared.center()))
}

fn span(points: &[Vec3], axis: Vec3) -> (f64, f64) {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for p in points {
        let d = p.dot(axis);
        low = low.min(d);
        high = high.max(d);
    }
    (low, high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Brush, Entity, Face, UvFormat};
    use crate::math::Vec2;

    fn box_brush(min: Vec3, max: Vec3, texture: &str) -> Brush {
        let sides = [
            (Vec3::X, max.x),
            (-Vec3::X, -min.x),
            (Vec3::Y, max.y),
            (-Vec3::Y, -min.y),
            (Vec3::Z, max.z),
            (-Vec3::Z, -min.z),
        ];
        Brush {
            faces: sides
                .into_iter()
                .map(|(normal, d)| Face {
                    plane: Plane { normal, d },
                    points: [Vec3::ZERO; 3],
                    texture: texture.to_string(),
                    format: UvFormat::Valve220,
                    uv_axes: [Vec3::X, Vec3::Y],
                    offset: Vec2::ZERO,
                    rotation: 0.0,
                    scale: Vec2::new(1.0, 1.0),
                    surface: None,
                })
                .collect(),
        }
    }

    fn entity(classname: &str, brushes: Vec<Brush>) -> Entity {
        let mut properties = indexmap::IndexMap::new();
        properties.insert("classname".to_string(), classname.to_string());
        Entity {
            properties,
            brushes,
            source_index: 0,
            ..Default::default()
        }
    }

    fn find_all(entities: &[Entity]) -> Vec<Overlap> {
        find(entities, |_, _| false)
    }

    #[test]
    fn brushes_that_only_touch_are_not_worth_a_word() {
        // Two boxes sharing a face, which is how every wall meets every floor.
        let world = entity(
            "worldspawn",
            vec![
                box_brush(
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(16.0, 16.0, 16.0),
                    "wall",
                ),
                box_brush(
                    Vec3::new(16.0, 0.0, 0.0),
                    Vec3::new(32.0, 16.0, 16.0),
                    "wall",
                ),
            ],
        );
        assert!(find_all(&[world]).is_empty());
    }

    #[test]
    fn brushes_a_long_way_apart_are_not_worth_a_word_either() {
        let world = entity(
            "worldspawn",
            vec![
                box_brush(Vec3::ZERO, Vec3::splat(16.0), "wall"),
                box_brush(Vec3::splat(500.0), Vec3::splat(516.0), "wall"),
            ],
        );
        assert!(find_all(&[world]).is_empty());
    }

    #[test]
    fn one_brush_pushed_into_another_is_reported_with_where_and_how_deep() {
        let world = entity(
            "worldspawn",
            vec![
                box_brush(Vec3::ZERO, Vec3::splat(16.0), "wall"),
                box_brush(Vec3::splat(12.0), Vec3::splat(28.0), "wall"),
            ],
        );
        let found = find_all(&[world]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].first.brush, 0);
        assert_eq!(found[0].second.brush, 1);
        assert!((found[0].depth - 4.0).abs() < 0.001, "{:?}", found[0].depth);
        // The middle of the shared corner.
        assert!((found[0].at - Vec3::splat(14.0)).length() < 0.001);
    }

    #[test]
    fn a_brush_swallowed_whole_by_another_counts() {
        let world = entity(
            "worldspawn",
            vec![
                box_brush(Vec3::ZERO, Vec3::splat(64.0), "wall"),
                box_brush(Vec3::splat(16.0), Vec3::splat(32.0), "wall"),
            ],
        );
        assert_eq!(find_all(&[world]).len(), 1);
    }

    #[test]
    fn brushes_at_an_angle_that_miss_each_other_are_left_alone() {
        // The case face normals alone get wrong: two boxes crossing like an X,
        // separated by an edge pair and by no face of either.
        let flat = box_brush(
            Vec3::new(-64.0, -4.0, -4.0),
            Vec3::new(64.0, 4.0, 4.0),
            "wall",
        );
        let mut turned = box_brush(
            Vec3::new(-64.0, -4.0, -4.0),
            Vec3::new(64.0, 4.0, 4.0),
            "wall",
        );
        // Turned 45 degrees about z and lifted clear of the first one.
        let angle: f64 = std::f64::consts::FRAC_PI_4;
        for face in turned.faces.iter_mut() {
            let n = face.plane.normal;
            let rotated = Vec3::new(
                n.x * angle.cos() - n.y * angle.sin(),
                n.x * angle.sin() + n.y * angle.cos(),
                n.z,
            );
            // Lifting it 12 units up moves the plane by the height component.
            face.plane = Plane {
                normal: rotated,
                d: face.plane.d + rotated.z * 12.0,
            };
        }
        let world = entity("worldspawn", vec![flat, turned]);
        assert!(
            find_all(&[world]).is_empty(),
            "two boxes twelve units apart in z are not touching"
        );
    }

    #[test]
    fn what_the_caller_skips_is_not_looked_at() {
        let world = entity(
            "worldspawn",
            vec![
                box_brush(Vec3::ZERO, Vec3::splat(16.0), "wall"),
                box_brush(Vec3::splat(4.0), Vec3::splat(20.0), "clip"),
            ],
        );
        let all = find(std::slice::from_ref(&world), |_, _| false);
        assert_eq!(all.len(), 1, "without a filter both brushes count");
        let filtered = find(&[world], |_, brush| {
            brush.faces.iter().all(|f| f.texture == "clip")
        });
        assert!(filtered.is_empty(), "a clip volume is meant to sit inside");
    }
}
