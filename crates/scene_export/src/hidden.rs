//! Finds faces that are pressed against another brush and cannot be seen.
//!
//! Within one solid they are thrown away. Across solids they are only reported:
//! whether the cover ever moves is something only the mapper knows.

use crate::map::{Brush, Entity};
use crate::math::{self, base_winding, clip_polygon_in_place, Plane, Vec3};
use rayon::prelude::*;
use rustc_hash::FxHashMap as HashMap;

/// One face that has something flat against it.
#[derive(Debug, Clone)]
pub struct Hidden {
    pub place: Where,
    /// Which face of that brush, counted as the map file lists them.
    pub face: usize,
    pub texture: String,
    /// The brush pressed against it.
    pub covered_by: Where,
    /// The middle of the face, in map units.
    pub at: Vec3,
    /// How big the face is, in square map units, so the worst are worth doing
    /// first.
    pub area: f64,
}

/// Which brush, in the terms the mapper uses.
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

impl std::fmt::Display for Hidden {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} face {} (`{}`, {:.0} units square, around ({:.0} {:.0} {:.0})) is flat against {} and cannot be seen; texture it with skip",
            self.place,
            self.face,
            self.texture,
            self.area,
            self.at.x,
            self.at.y,
            self.at.z,
            self.covered_by
        )
    }
}

struct FaceShape {
    place: Where,
    face: usize,
    texture: String,
    plane: Plane,
    points: Vec<Vec3>,
    area: f64,
}

/// How far off a plane a face may be and still count as lying on it.
const ON_THE_PLANE: f64 = 0.05;
/// And how far outside the covering face a corner may stray, in map units.
const INSIDE: f64 = 0.05;
/// Faces smaller than this are not worth anybody's afternoon.
const WORTH_MENTIONING: f64 = 16.0;

/// Every face that another brush is flat against, in map units.
///
/// `is_tool` says which textures already mean "do not draw this". A face hidden
/// by one is not hidden, because skip leaves a hole to see through.
pub fn find(entities: &[Entity], is_tool: impl Fn(&str) -> bool) -> Vec<Hidden> {
    let mut shapes: Vec<FaceShape> = Vec::new();
    for (entity_index, entity) in entities.iter().enumerate() {
        shapes.extend(shapes_of(&entity.brushes, entity_index, entity.classname()));
    }
    pairs(shapes, is_tool, false)
}

/// The faces of one solid that another brush of it is flat against.
///
/// Safe to throw away: two brushes of one solid never move apart.
pub fn within(brushes: &[Brush], is_tool: impl Fn(&str) -> bool) -> Vec<(usize, usize)> {
    pairs(shapes_of(brushes, 0, ""), is_tool, false)
        .into_iter()
        .map(|hidden| (hidden.place.brush, hidden.face))
        .collect()
}

/// Editor omission also includes exactly matching faces. The export builder
/// normally removes those in its separate shared-face pass.
pub fn covered_faces(brushes: &[Brush], is_tool: impl Fn(&str) -> bool) -> Vec<(usize, usize)> {
    pairs(shapes_of(brushes, 0, ""), is_tool, true)
        .into_iter()
        .map(|h| (h.place.brush, h.face))
        .collect()
}

/// Brushes swallowed whole by another brush of the same solid. No amount of
/// skip saves a crate left inside a pillar, so the whole thing goes.
pub fn swallowed(brushes: &[Brush]) -> Vec<usize> {
    // One brush's corners have nothing to do with any other's.
    let corners: Vec<Vec<Vec3>> = if brushes.len() >= 64 {
        brushes
            .par_iter()
            .map(|brush| crate::overlap::corners_of(&brush.planes()))
            .collect()
    } else {
        brushes
            .iter()
            .map(|brush| crate::overlap::corners_of(&brush.planes()))
            .collect()
    };
    let boxes: Vec<(Vec3, Vec3)> = corners.iter().map(|points| bounds_of(points)).collect();

    // Swept, so brushes nowhere near each other are never compared. Without it
    // ten thousand brushes spend seconds asking every brush about every other.
    let mut order: Vec<usize> = (0..brushes.len()).collect();
    order.sort_by(|a, b| {
        boxes[*a]
            .0
            .x
            .partial_cmp(&boxes[*b].0.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut gone = vec![false; brushes.len()];
    for (place, &i) in order.iter().enumerate() {
        if corners[i].len() < 4 {
            continue;
        }
        for &j in order.iter().skip(place + 1) {
            if corners[j].len() < 4 {
                continue;
            }
            // Past the end of this one in x, and so is everything after it.
            if boxes[j].0.x > boxes[i].1.x + ON_THE_PLANE {
                break;
            }
            // Both ways round: the sweep only says the two are near each other,
            // not which of them is the bigger.
            let i_in_j = fits(boxes[i], boxes[j]) && inside(&corners[i], &brushes[j]);
            let j_in_i = fits(boxes[j], boxes[i]) && inside(&corners[j], &brushes[i]);
            if i_in_j && j_in_i {
                // The same brush drawn twice. One of them has to stay, or the
                // wall disappears; the first one keeps its place.
                gone[i.max(j)] = true;
            } else if i_in_j {
                gone[i] = true;
            } else if j_in_i {
                gone[j] = true;
            }
        }
    }
    (0..brushes.len()).filter(|i| gone[*i]).collect()
}

/// Whether one box sits inside another.
fn fits(inner: (Vec3, Vec3), outer: (Vec3, Vec3)) -> bool {
    inner.0.x >= outer.0.x - ON_THE_PLANE
        && inner.0.y >= outer.0.y - ON_THE_PLANE
        && inner.0.z >= outer.0.z - ON_THE_PLANE
        && inner.1.x <= outer.1.x + ON_THE_PLANE
        && inner.1.y <= outer.1.y + ON_THE_PLANE
        && inner.1.z <= outer.1.z + ON_THE_PLANE
}

fn bounds_of(points: &[Vec3]) -> (Vec3, Vec3) {
    let mut low = Vec3::splat(f64::INFINITY);
    let mut high = Vec3::splat(f64::NEG_INFINITY);
    for p in points {
        low = low.min(*p);
        high = high.max(*p);
    }
    (low, high)
}

/// Whether every corner sits behind every plane of a brush.
fn inside(points: &[Vec3], brush: &Brush) -> bool {
    points.iter().all(|point| {
        brush
            .faces
            .iter()
            .all(|face| face.plane.distance_to(*point) <= ON_THE_PLANE)
    })
}

fn shapes_of(brushes: &[Brush], entity: usize, classname: &str) -> Vec<FaceShape> {
    let one_brush = |(brush_index, brush): (usize, &Brush)| -> Vec<FaceShape> {
        let planes = brush.planes();
        let mut out = Vec::with_capacity(brush.faces.len());
        let mut scratch: Vec<Vec3> = Vec::new();
        for (face_index, face) in brush.faces.iter().enumerate() {
            let points = winding_of(&planes, face_index, &mut scratch);
            if points.len() < 3 {
                continue;
            }
            let area = area_of(&points);
            out.push(FaceShape {
                place: Where {
                    entity,
                    classname: classname.to_string(),
                    brush: brush_index,
                },
                face: face_index,
                texture: face.texture.clone(),
                plane: face.plane,
                points,
                area,
            });
        }
        out
    };
    // Cutting each face out of its brush is the expensive part and one brush
    // knows nothing of the next, so a big level does it on every core.
    if brushes.len() >= 64 {
        brushes
            .par_iter()
            .enumerate()
            .flat_map_iter(one_brush)
            .collect()
    } else {
        brushes.iter().enumerate().flat_map(one_brush).collect()
    }
}

fn pairs(
    shapes: Vec<FaceShape>,
    is_tool: impl Fn(&str) -> bool,
    include_equal: bool,
) -> Vec<Hidden> {
    // Only faces sharing a surface can hide each other. Two faces of one surface
    // facing opposite ways have the same key.
    let mut by_surface: HashMap<[i64; 4], Vec<usize>> = HashMap::default();
    for (i, shape) in shapes.iter().enumerate() {
        by_surface
            .entry(surface_key(&shape.plane))
            .or_default()
            .push(i);
    }

    let mut found = Vec::new();
    for group in by_surface.values_mut() {
        // Biggest first, so the search stops as soon as the candidates are too small
        // to cover anything.
        group.sort_by(|a, b| {
            shapes[*b]
                .area
                .partial_cmp(&shapes[*a].area)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Only a face looking the other way can cover one, so each face looks through
        // the other side's list alone. A big floor is one surface with thousands of
        // faces all looking the same way, and asking each of every other was seconds.
        let reference = shapes[group[0]].plane.normal;
        let (forward, backward): (Vec<usize>, Vec<usize>) = group
            .iter()
            .partition(|&&i| shapes[i].plane.normal.dot(reference) > 0.0);
        for &a in group.iter() {
            if is_tool(&shapes[a].texture) {
                continue;
            }
            let area = shapes[a].area;
            if area < WORTH_MENTIONING {
                continue;
            }
            let facing = if shapes[a].plane.normal.dot(reference) > 0.0 {
                &backward
            } else {
                &forward
            };
            for &b in facing.iter() {
                // Faces of the same size meeting exactly are thrown away by the
                // build already. Sorted biggest first, so once one is too small
                // so is the rest.
                if if include_equal {
                    shapes[b].area + 0.001 < area
                } else {
                    shapes[b].area <= area + 1.0
                } {
                    break;
                }
                if a == b
                    || shapes[a].place.brush == shapes[b].place.brush
                        && shapes[a].place.entity == shapes[b].place.entity
                {
                    continue;
                }
                // Facing each other, not the same way: two faces looking the
                // same way are two floors at the same height, not a cover.
                if shapes[a].plane.normal.dot(shapes[b].plane.normal) > -0.999 {
                    continue;
                }
                if is_tool(&shapes[b].texture) {
                    continue;
                }
                if !covers(&shapes[b], &shapes[a]) {
                    continue;
                }
                found.push(Hidden {
                    place: shapes[a].place.clone(),
                    face: shapes[a].face,
                    texture: shapes[a].texture.clone(),
                    covered_by: shapes[b].place.clone(),
                    at: middle(&shapes[a].points),
                    area,
                });
                break;
            }
        }
    }
    found.sort_by(|a, b| {
        b.area
            .partial_cmp(&a.area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    found
}

/// A key that is the same for both sides of one surface: the normal is turned
/// so its first meaningful component is positive, and the distance with it.
fn surface_key(plane: &Plane) -> [i64; 4] {
    let mut n = plane.normal;
    let mut d = plane.d;
    let first = if n.x.abs() > 1.0e-6 {
        n.x
    } else if n.y.abs() > 1.0e-6 {
        n.y
    } else {
        n.z
    };
    if first < 0.0 {
        n = -n;
        d = -d;
    }
    // Quantised, because two brushes drawn on the same grid line still differ
    // in the last bits of a float.
    [
        (n.x * 4096.0).round() as i64,
        (n.y * 4096.0).round() as i64,
        (n.z * 4096.0).round() as i64,
        (d * 16.0).round() as i64,
    ]
}

/// Whether `cover` swallows `face` whole. Both are convex and on one surface,
/// so every corner of one being inside the other settles it.
fn covers(cover: &FaceShape, face: &FaceShape) -> bool {
    // Same surface, allowing for the last bits of a float.
    for point in &face.points {
        if cover.plane.distance_to(*point).abs() > ON_THE_PLANE {
            return false;
        }
    }
    let normal = cover.plane.normal;
    // The points came out of `winding_of` already in polygon order -- clipping keeps a
    // convex polygon ordered -- and the edge test below settles which way round each
    // edge faces on its own, so there is nothing to sort (and nothing to clone: this
    // ran once per candidate pair, which the profile noticed).
    let edge_points = &cover.points;
    if edge_points.len() < 3 {
        return false;
    }
    // Which way round the corners were put is not worth relying on, so each
    // edge is pointed away from the middle of the face it belongs to.
    let inside = middle(edge_points);
    for i in 0..edge_points.len() {
        let from = edge_points[i];
        let to = edge_points[(i + 1) % edge_points.len()];
        let mut outward = (to - from).cross(normal).normalize_or_zero();
        if outward.dot(inside - from) > 0.0 {
            outward = -outward;
        }
        for point in &face.points {
            if outward.dot(*point - from) > INSIDE {
                return false;
            }
        }
    }
    true
}

/// The corners of one face of a brush, in order. `scratch` is the clip passes'
/// working buffer, kept by the caller so a loop over faces allocates nothing here.
fn winding_of(planes: &[Plane], face: usize, scratch: &mut Vec<Vec3>) -> Vec<Vec3> {
    let mut winding = base_winding(&planes[face], 1.0e5);
    for (i, other) in planes.iter().enumerate() {
        if i == face {
            continue;
        }
        clip_polygon_in_place(&mut winding, scratch, other, 1.0e-4);
        if winding.is_empty() {
            return Vec::new();
        }
    }
    math::dedup_winding_in_place(&mut winding, 1.0e-3);
    winding
}

fn middle(points: &[Vec3]) -> Vec3 {
    if points.is_empty() {
        return Vec3::ZERO;
    }
    points.iter().fold(Vec3::ZERO, |a, b| a + *b) / points.len() as f64
}

/// Area of a flat convex polygon, by fanning it into triangles.
fn area_of(points: &[Vec3]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut total = 0.0;
    for i in 1..points.len() - 1 {
        total += (points[i] - points[0])
            .cross(points[i + 1] - points[0])
            .length()
            * 0.5;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Brush, Entity, Face, UvFormat};
    use crate::math::Vec2;

    fn box_brush(min: Vec3, max: Vec3, textures: [&str; 6]) -> Brush {
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
                .zip(textures)
                .map(|((normal, d), texture)| Face {
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

    fn plain(min: Vec3, max: Vec3) -> Brush {
        box_brush(min, max, ["wall"; 6])
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

    fn tools(texture: &str) -> bool {
        matches!(texture, "skip" | "clip" | "origin")
    }

    #[test]
    fn a_crate_standing_on_a_floor_has_a_bottom_nobody_can_see() {
        let world = entity(
            "worldspawn",
            vec![
                // A wide floor, its top at z = 0.
                plain(
                    Vec3::new(-256.0, -256.0, -16.0),
                    Vec3::new(256.0, 256.0, 0.0),
                ),
                // A crate on it.
                plain(Vec3::new(0.0, 0.0, 0.0), Vec3::new(32.0, 32.0, 32.0)),
            ],
        );
        let found = find(&[world], tools);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].place.brush, 1, "the crate, not the floor");
        assert!((found[0].area - 1024.0).abs() < 0.1);
        assert!((found[0].at - Vec3::new(16.0, 16.0, 0.0)).length() < 0.01);
    }

    #[test]
    fn a_bottom_already_marked_skip_is_left_alone() {
        let mut crate_brush = plain(Vec3::new(0.0, 0.0, 0.0), Vec3::new(32.0, 32.0, 32.0));
        // The last of the six sides is the one facing down.
        crate_brush.faces[5].texture = "skip".to_string();
        let world = entity(
            "worldspawn",
            vec![
                plain(
                    Vec3::new(-256.0, -256.0, -16.0),
                    Vec3::new(256.0, 256.0, 0.0),
                ),
                crate_brush,
            ],
        );
        assert!(find(&[world], tools).is_empty());
    }

    #[test]
    fn a_crate_beside_the_floor_rather_than_on_it_keeps_its_bottom() {
        let world = entity(
            "worldspawn",
            vec![
                plain(Vec3::new(-256.0, -256.0, -16.0), Vec3::new(0.0, 256.0, 0.0)),
                // Hanging past the edge of that floor.
                plain(Vec3::new(16.0, 0.0, 0.0), Vec3::new(48.0, 32.0, 32.0)),
            ],
        );
        assert!(find(&[world], tools).is_empty());
    }

    #[test]
    fn a_crate_half_on_the_floor_is_not_called_hidden() {
        // Only part of the bottom is covered, so cutting the face up would be
        // the only way to save anything, and that is not what this is for.
        let world = entity(
            "worldspawn",
            vec![
                plain(
                    Vec3::new(-256.0, -256.0, -16.0),
                    Vec3::new(16.0, 256.0, 0.0),
                ),
                plain(Vec3::new(0.0, 0.0, 0.0), Vec3::new(32.0, 32.0, 32.0)),
            ],
        );
        assert!(find(&[world], tools).is_empty());
    }

    #[test]
    fn two_slabs_meeting_exactly_are_left_to_the_build() {
        // Their touching faces are the same size, which the build throws away by
        // itself.
        let world = entity(
            "worldspawn",
            vec![
                plain(Vec3::new(0.0, 0.0, -16.0), Vec3::new(64.0, 64.0, 0.0)),
                plain(Vec3::new(64.0, 0.0, -16.0), Vec3::new(128.0, 64.0, 0.0)),
            ],
        );
        assert!(find(&[world], tools).is_empty());
    }

    #[test]
    fn a_crate_left_inside_a_pillar_goes_altogether() {
        let brushes = vec![
            plain(Vec3::ZERO, Vec3::splat(128.0)),
            plain(Vec3::splat(32.0), Vec3::splat(64.0)),
        ];
        assert_eq!(swallowed(&brushes), vec![1]);
    }

    #[test]
    fn a_crate_poking_out_of_a_pillar_stays() {
        let brushes = vec![
            plain(Vec3::ZERO, Vec3::splat(128.0)),
            plain(Vec3::splat(96.0), Vec3::splat(160.0)),
        ];
        assert!(swallowed(&brushes).is_empty());
    }

    #[test]
    fn the_same_brush_drawn_twice_leaves_one_behind() {
        // Each is inside the other, so without care both would go and the wall
        // would vanish. The first one keeps its place.
        let brushes = vec![
            plain(Vec3::ZERO, Vec3::splat(64.0)),
            plain(Vec3::ZERO, Vec3::splat(64.0)),
        ];
        assert_eq!(swallowed(&brushes), vec![1]);
    }

    #[test]
    fn a_crate_on_a_floor_of_the_same_solid_is_handed_back_as_a_face_to_drop() {
        let brushes = vec![
            plain(
                Vec3::new(-256.0, -256.0, -16.0),
                Vec3::new(256.0, 256.0, 0.0),
            ),
            plain(Vec3::new(0.0, 0.0, 0.0), Vec3::new(32.0, 32.0, 32.0)),
        ];
        let gone = within(&brushes, tools);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].0, 1, "the crate");
        assert_eq!(gone[0].1, 5, "its bottom");
    }

    #[test]
    fn a_face_hidden_by_a_door_is_still_reported_but_says_which_door() {
        let world = entity(
            "worldspawn",
            vec![plain(
                Vec3::new(-256.0, -256.0, -16.0),
                Vec3::new(256.0, 256.0, 0.0),
            )],
        );
        let door = entity(
            "func_door",
            vec![plain(Vec3::new(0.0, 0.0, 0.0), Vec3::new(32.0, 32.0, 96.0))],
        );
        let found = find(&[world, door], tools);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].place.classname, "func_door");
        assert_eq!(found[0].covered_by.classname, "worldspawn");
    }
}
