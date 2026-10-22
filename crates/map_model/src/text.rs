//! Text built out of brushes.
//!
//! Converts text into brush geometry for signs, labels and numbers.
//!
//! The font is a sixteen-segment display -- the kind on an instrument panel or a
//! scoreboard, which is about as technical as lettering gets. Every glyph is a subset of
//! the same sixteen bars, and the bars are laid out with a gap between neighbours, so no
//! two brushes of a glyph ever share a volume.
//!
//! Glyph space runs x rightward and y upward, the baseline at 0 and the cap height at 1.
//! [`text_brushes`] lays the glyphs out along x, scales the box to `height`, extrudes it
//! `depth` along the world's y, and drops the result at `origin`.

use geometry_math::bbox::Bbox3d;
use geometry_math::mat_ext;
use geometry_math::vec::Vec3d;
use geometry_math::Float;

use crate::brush::Brush;

/// A segment's shape in glyph units.
#[derive(Debug, Clone, Copy)]
enum Seg {
    /// An axis-aligned bar between two corners.
    Rect { x0: f64, y0: f64, x1: f64, y1: f64 },
    /// A slanted bar of the given thickness between two points -- the four diagonals.
    Bar {
        a: (f64, f64),
        b: (f64, f64),
        thick: f64,
    },
}

/// The glyph box width.
const W: f64 = 0.60;
/// The segment thickness.
const T: f64 = 0.12;
/// The gap left between neighbouring segments so their brushes never touch.
const G: f64 = 0.05;
/// The space after every glyph (the font is monospaced).
const TRACKING: f64 = 0.18;

/// The centre of the glyph box.
const CX: f64 = W / 2.0;
/// The top of the horizontal segments.
const TOP: f64 = 1.0 - T;
/// The bottom edge of the middle segments.
const MID_LO: f64 = 0.5 - T / 2.0;
/// The top edge of the middle segments.
const MID_HI: f64 = 0.5 + T / 2.0;

/// The sixteen segments, in the order the bit masks below number them:
/// A1 A2 B C D1 D2 E F G1 G2 H I J K L M.
fn segments() -> [Seg; 16] {
    let diag = T * 0.72;
    [
        // A1, A2: the top, split at the centre.
        Seg::Rect {
            x0: 0.0,
            y0: TOP,
            x1: CX - G,
            y1: 1.0,
        },
        Seg::Rect {
            x0: CX + G,
            y0: TOP,
            x1: W,
            y1: 1.0,
        },
        // B, C: the right side, upper and lower.
        Seg::Rect {
            x0: W - T,
            y0: MID_HI + G,
            x1: W,
            y1: TOP - G,
        },
        Seg::Rect {
            x0: W - T,
            y0: T + G,
            x1: W,
            y1: MID_LO - G,
        },
        // D1, D2: the bottom, split at the centre.
        Seg::Rect {
            x0: 0.0,
            y0: 0.0,
            x1: CX - G,
            y1: T,
        },
        Seg::Rect {
            x0: CX + G,
            y0: 0.0,
            x1: W,
            y1: T,
        },
        // E, F: the left side, lower and upper.
        Seg::Rect {
            x0: 0.0,
            y0: T + G,
            x1: T,
            y1: MID_LO - G,
        },
        Seg::Rect {
            x0: 0.0,
            y0: MID_HI + G,
            x1: T,
            y1: TOP - G,
        },
        // G1, G2: the middle, split at the centre.
        Seg::Rect {
            x0: T + G,
            y0: MID_LO,
            x1: CX - T / 2.0 - G,
            y1: MID_HI,
        },
        Seg::Rect {
            x0: CX + T / 2.0 + G,
            y0: MID_LO,
            x1: W - T - G,
            y1: MID_HI,
        },
        // H, J: the upper diagonals, from a corner in towards the centre.
        Seg::Bar {
            a: (T + G + diag * 0.4, TOP - G),
            b: (CX - T / 2.0 - G, MID_HI + G),
            thick: diag,
        },
        // I: the upper centre vertical.
        Seg::Rect {
            x0: CX - T / 2.0,
            y0: MID_HI + G,
            x1: CX + T / 2.0,
            y1: TOP - G,
        },
        Seg::Bar {
            a: (W - T - G - diag * 0.4, TOP - G),
            b: (CX + T / 2.0 + G, MID_HI + G),
            thick: diag,
        },
        // K: the lower-left diagonal.
        Seg::Bar {
            a: (T + G + diag * 0.4, T + G),
            b: (CX - T / 2.0 - G, MID_LO - G),
            thick: diag,
        },
        // L: the lower centre vertical.
        Seg::Rect {
            x0: CX - T / 2.0,
            y0: T + G,
            x1: CX + T / 2.0,
            y1: MID_LO - G,
        },
        // M: the lower-right diagonal.
        Seg::Bar {
            a: (W - T - G - diag * 0.4, T + G),
            b: (CX + T / 2.0 + G, MID_LO - G),
            thick: diag,
        },
    ]
}

// Segment bits, in the order `segments()` returns them.
const A1: u16 = 1 << 0;
const A2: u16 = 1 << 1;
const B: u16 = 1 << 2;
const C: u16 = 1 << 3;
const D1: u16 = 1 << 4;
const D2: u16 = 1 << 5;
const E: u16 = 1 << 6;
const F: u16 = 1 << 7;
const G1: u16 = 1 << 8;
const G2: u16 = 1 << 9;
const H: u16 = 1 << 10;
const I: u16 = 1 << 11;
const J: u16 = 1 << 12;
const K: u16 = 1 << 13;
const L: u16 = 1 << 14;
const M: u16 = 1 << 15;

/// Handy combinations.
const TOP_BAR: u16 = A1 | A2;
const BOTTOM_BAR: u16 = D1 | D2;
const MID_BAR: u16 = G1 | G2;
const LEFT_SIDE: u16 = E | F;
const RIGHT_SIDE: u16 = B | C;
const CENTRE_VERT: u16 = I | L;
const OUTLINE: u16 = TOP_BAR | BOTTOM_BAR | LEFT_SIDE | RIGHT_SIDE;

/// Which segments a lowercase letter lights up, where it is drawn differently from its
/// capital. A sixteen-segment display cannot do a true x-height, so these are the shapes
/// that read as lowercase inside the full cell -- a bowl in the lower half, a stem beside
/// it. Letters not here (c k o s v w x z and the rest) look the same in both cases.
fn lowercase_pattern(c: char) -> Option<u16> {
    Some(match c {
        'a' => G1 | G2 | E | C | D1 | D2 | B,
        'b' => LEFT_SIDE | G1 | G2 | C | D1 | D2,
        'd' => RIGHT_SIDE | G1 | G2 | E | D1 | D2,
        'e' => G1 | G2 | E | C | D1 | D2,
        'f' => TOP_BAR | G1 | G2 | I | L,
        'g' => TOP_BAR | F | B | G1 | G2 | C | D1 | D2,
        'h' => LEFT_SIDE | G1 | G2 | C,
        'i' => L,
        'j' => C | D1 | D2 | E,
        'm' => G1 | G2 | E | C | L,
        'n' => G1 | G2 | E | C,
        'r' => G1 | G2 | E,
        't' => CENTRE_VERT | G1 | G2 | D1 | D2,
        'u' => E | C | D1 | D2,
        'y' => F | B | C | G1 | G2 | D1 | D2,
        _ => return None,
    })
}

/// Which segments a character lights up. `None` for a character it does not know; a bare
/// `0` for the space.
fn pattern(c: char) -> Option<u16> {
    if let Some(lower) = c
        .is_ascii_lowercase()
        .then(|| lowercase_pattern(c))
        .flatten()
    {
        return Some(lower);
    }
    let p = match c.to_ascii_uppercase() {
        ' ' => 0,
        'A' => TOP_BAR | RIGHT_SIDE | LEFT_SIDE | MID_BAR,
        'B' => TOP_BAR | RIGHT_SIDE | BOTTOM_BAR | G2 | I | L,
        'C' => TOP_BAR | LEFT_SIDE | BOTTOM_BAR,
        'D' => TOP_BAR | RIGHT_SIDE | BOTTOM_BAR | I | L,
        'E' => TOP_BAR | LEFT_SIDE | BOTTOM_BAR | MID_BAR,
        'F' => TOP_BAR | LEFT_SIDE | MID_BAR,
        'G' => TOP_BAR | LEFT_SIDE | BOTTOM_BAR | C | G2,
        'H' => LEFT_SIDE | RIGHT_SIDE | MID_BAR,
        'I' => TOP_BAR | BOTTOM_BAR | CENTRE_VERT,
        'J' => RIGHT_SIDE | D1 | D2 | E,
        'K' => LEFT_SIDE | G1 | J | M,
        'L' => LEFT_SIDE | BOTTOM_BAR,
        'M' => LEFT_SIDE | RIGHT_SIDE | H | J,
        'N' => LEFT_SIDE | RIGHT_SIDE | H | M,
        'O' => OUTLINE,
        'P' => TOP_BAR | LEFT_SIDE | B | MID_BAR,
        'Q' => OUTLINE | M,
        'R' => TOP_BAR | LEFT_SIDE | B | MID_BAR | M,
        'S' => TOP_BAR | F | MID_BAR | C | BOTTOM_BAR,
        'T' => TOP_BAR | CENTRE_VERT,
        'U' => LEFT_SIDE | RIGHT_SIDE | BOTTOM_BAR,
        'V' => LEFT_SIDE | K | M,
        'W' => LEFT_SIDE | RIGHT_SIDE | K | M,
        'X' => H | J | K | M,
        'Y' => H | J | L,
        'Z' => TOP_BAR | J | K | BOTTOM_BAR,
        '0' => OUTLINE | J | K,
        '1' => B | C | J,
        '2' => TOP_BAR | B | MID_BAR | E | BOTTOM_BAR,
        '3' => TOP_BAR | RIGHT_SIDE | MID_BAR | BOTTOM_BAR,
        '4' => F | B | C | MID_BAR,
        '5' => TOP_BAR | F | G1 | M | D1 | D2,
        '6' => TOP_BAR | LEFT_SIDE | C | MID_BAR | BOTTOM_BAR,
        '7' => TOP_BAR | B | C,
        '8' => OUTLINE | MID_BAR,
        '9' => TOP_BAR | F | B | C | MID_BAR | BOTTOM_BAR,
        '-' => MID_BAR,
        '_' => BOTTOM_BAR,
        '=' => MID_BAR | BOTTOM_BAR,
        '+' => MID_BAR | CENTRE_VERT,
        '/' => J | K,
        '\\' => H | M,
        '.' => return Some(PERIOD),
        ',' => return Some(COMMA),
        ':' => return Some(COLON),
        '!' => return Some(BANG),
        '?' => TOP_BAR | B | G2 | L,
        '\'' => I,
        '"' => I | B,
        '(' => A2 | E | F | D2,
        ')' => A1 | B | C | D1,
        '<' => J | M,
        '>' => H | K,
        _ => return None,
    };
    Some(p)
}

/// Sentinels for the few marks that are not one of the sixteen segments.
const PERIOD: u16 = 0xF001;
const COMMA: u16 = 0xF002;
const COLON: u16 = 0xF003;
const BANG: u16 = 0xF004;

/// A small square block centred on x, its bottom at `y`.
fn dot(y: f64) -> Seg {
    Seg::Rect {
        x0: CX - T / 2.0,
        y0: y,
        x1: CX + T / 2.0,
        y1: y + T,
    }
}

/// The extra shapes those sentinels stand for.
fn special(mark: u16) -> Vec<Seg> {
    match mark {
        PERIOD => vec![dot(0.0)],
        COMMA => vec![Seg::Rect {
            x0: CX - T / 2.0,
            y0: -0.18,
            x1: CX + T / 2.0,
            y1: T,
        }],
        COLON => vec![dot(0.16), dot(0.62)],
        BANG => vec![
            Seg::Rect {
                x0: CX - T / 2.0,
                y0: T * 2.0,
                x1: CX + T / 2.0,
                y1: 1.0,
            },
            dot(0.0),
        ],
        _ => vec![],
    }
}

/// Whether the font has a glyph for a character.
pub fn has_glyph(c: char) -> bool {
    pattern(c).is_some()
}

/// The strokes that make up one character.
fn glyph_segments(c: char) -> Option<Vec<Seg>> {
    let bits = pattern(c)?;
    if (bits & 0xF000) == 0xF000 {
        return Some(special(bits));
    }
    let all = segments();
    let mut segs: Vec<Seg> = (0..16)
        .filter(|i| bits & (1 << i) != 0)
        .map(|i| all[i])
        .collect();
    // The question mark carries a dot below, the same as the exclamation.
    if c == '?' {
        segs.push(dot(0.0));
    }
    Some(segs)
}

/// Turns `text` into brushes: a row of sixteen-segment glyphs, `height` tall, extruded
/// `depth` along the world's y axis, with the bottom-left of the first glyph's baseline
/// at `origin`. Characters the font does not have are skipped. Newlines start a new line
/// below the last.
pub fn text_brushes(
    text: &str,
    height: f64,
    depth: f64,
    material: &str,
    origin: Vec3d,
    world_bounds: &Bbox3d,
) -> Vec<Brush> {
    let advance = W + TRACKING;
    let mut brushes = Vec::new();
    let mut pen_x = 0.0;
    let mut line_y = 0.0;

    for c in text.chars() {
        if c == '\n' {
            pen_x = 0.0;
            line_y -= 1.4;
            continue;
        }
        let Some(strokes) = glyph_segments(c) else {
            continue;
        };
        for seg in strokes {
            if let Some(brush) = seg_brush(
                seg,
                pen_x,
                line_y,
                height,
                depth,
                material,
                origin,
                world_bounds,
            ) {
                brushes.push(brush);
            }
        }
        pen_x += advance;
    }
    brushes
}

/// The width `text` would come out, in world units.
pub fn text_width(text: &str, height: f64) -> f64 {
    let advance = W + TRACKING;
    text.lines()
        .map(|line| line.chars().filter(|c| has_glyph(*c)).count() as f64 * advance)
        .fold(0.0_f64, f64::max)
        * height
}

#[allow(clippy::too_many_arguments)]
fn seg_brush(
    seg: Seg,
    pen_x: f64,
    line_y: f64,
    height: f64,
    depth: f64,
    material: &str,
    origin: Vec3d,
    world_bounds: &Bbox3d,
) -> Option<Brush> {
    // Glyph units to world: x and z scale by `height`, y is the extrusion.
    let wx = |gx: f64| origin.x() + (pen_x + gx) * height;
    let wz = |gz: f64| origin.z() + (line_y + gz) * height;

    match seg {
        Seg::Rect { x0, y0, x1, y1 } => box_brush(
            world_bounds,
            Vec3d::new(wx(x0), origin.y(), wz(y0)),
            Vec3d::new(wx(x1), origin.y() + depth, wz(y1)),
            material,
        ),
        Seg::Bar { a, b, thick } => {
            let (ax, az) = (wx(a.0), wz(a.1));
            let (bx, bz) = (wx(b.0), wz(b.1));
            let (dx, dz) = (bx - ax, bz - az);
            let len = (dx * dx + dz * dz).sqrt();
            if len < 1e-6 {
                return None;
            }
            let half_t = thick * height / 2.0;
            let bar = box_brush(
                world_bounds,
                Vec3d::new(-len / 2.0, 0.0, -half_t),
                Vec3d::new(len / 2.0, depth, half_t),
                material,
            )?;
            let mid = Vec3d::new((ax + bx) / 2.0, origin.y(), (az + bz) / 2.0);
            let transform = mat_ext::translation_matrix(mid)
                * mat_ext::rotation_matrix_about_axis(Vec3d::new(0.0, 1.0, 0.0), -dz.atan2(dx));
            bar.transform(world_bounds, &transform).ok()
        }
    }
}

/// An axis-aligned box brush between two corners, or nothing if it is degenerate.
fn box_brush(world_bounds: &Bbox3d, a: Vec3d, b: Vec3d, material: &str) -> Option<Brush> {
    let bounds = Bbox3d::new_repaired(a, b);
    let size = bounds.size();
    if size.x() < 1e-4 || size.y() < 1e-4 || size.z() < 1e-4 {
        return None;
    }
    crate::shapes::create_cuboid(world_bounds, &bounds, material).ok()
}

/* ========== outlines ========== */

/// A closed outline, in glyph space: x rightward, y upward, the baseline at 0.
///
/// What a real font gives, once its curves have been flattened into line segments. The
/// sixteen-segment font above is what this editor draws without one; a font on the machine
/// gives letters with the shapes people expect, and [`brushes_from_outlines`] turns those
/// into geometry.
pub type Outline = Vec<geometry_math::vec::Vec2d>;

/// Turns a set of closed outlines into brushes: each is filled, holes are left out, and
/// the result is extruded `depth` along the world's y axis from `origin`.
///
/// The outlines are in glyph space and scaled by `height`, the same way [`text_brushes`]
/// scales its own font, so the two can stand in for each other.
///
/// A letter is not convex and a brush is, so each filled region is cut into convex pieces:
/// triangulated by ear clipping, then neighbouring triangles merged back together while
/// the result stays convex. That is Hertel and Mehlhorn's method, and it leaves a letter
/// as a handful of brushes rather than as forty triangles.
pub fn brushes_from_outlines(
    outlines: &[Outline],
    height: f64,
    depth: f64,
    material: &str,
    origin: Vec3d,
    world_bounds: &Bbox3d,
) -> Vec<Brush> {
    let mut brushes = Vec::new();
    for piece in convex_pieces(outlines) {
        if let Some(brush) = prism(&piece, height, depth, material, origin, world_bounds) {
            brushes.push(brush);
        }
    }
    brushes
}

/// Extrudes one convex polygon in glyph space into a brush.
fn prism(
    polygon: &[geometry_math::vec::Vec2d],
    height: f64,
    depth: f64,
    material: &str,
    origin: Vec3d,
    world_bounds: &Bbox3d,
) -> Option<Brush> {
    if polygon.len() < 3 || depth <= 0.0 {
        return None;
    }

    let mut points = Vec::with_capacity(polygon.len() * 2);
    for corner in polygon {
        let x = origin.x() + corner.x() * height;
        let z = origin.z() + corner.y() * height;
        points.push(Vec3d::new(x, origin.y(), z));
        points.push(Vec3d::new(x, origin.y() + depth, z));
    }

    // The hull is the prism: every point is on it, since the polygon is convex.
    let geometry = crate::polyhedron::Polyhedron::from_points(&points, f64::ALMOST_ZERO).ok()?;
    let brush = Brush::from_polyhedron(geometry, material);
    // A glyph placed outside the world is no glyph at all.
    world_bounds.contains(&brush.bounds()).then_some(brush)
}

/// Cuts the filled regions the outlines describe into convex polygons.
pub(crate) fn convex_pieces(outlines: &[Outline]) -> Vec<Vec<geometry_math::vec::Vec2d>> {
    let rings: Vec<Outline> = outlines
        .iter()
        .filter(|ring| ring.len() >= 3)
        .cloned()
        .collect();

    // Which rings are holes: a ring inside an odd number of others, which is the even-odd
    // rule a font's outlines are drawn by.
    let mut pieces = Vec::new();
    for (index, ring) in rings.iter().enumerate() {
        let depth = rings
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .filter(|(_, candidate)| contains_ring(candidate, ring))
            .count();
        if depth % 2 == 1 {
            continue;
        }

        // The holes directly inside this one.
        let holes: Vec<Outline> = rings
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .filter(|(_, candidate)| contains_ring(ring, candidate))
            .filter(|(other, candidate)| {
                // Only the holes of *this* ring: one nested deeper belongs to whatever
                // island sits inside the hole.
                rings
                    .iter()
                    .enumerate()
                    .filter(|(third, _)| *third != index && *third != *other)
                    .all(|(_, third)| !contains_ring(third, candidate))
            })
            .map(|(_, candidate)| candidate.clone())
            .collect();

        // Scan symmetric counters in horizontal bands. Ear clipping followed by
        // greedy merging depends on the first vertex and breaks mirrored brush seams.
        if !holes.is_empty() && reflection_symmetric(ring, &holes) {
            pieces.extend(horizontal_bands(ring, &holes));
            continue;
        }

        // The ring counter-clockwise and its holes clockwise, which is what lets a bridge
        // be walked round both without crossing itself.
        let mut ring = ring.clone();
        if signed_area(&ring) < 0.0 {
            ring.reverse();
        }
        let holes: Vec<Outline> = holes
            .into_iter()
            .map(|mut hole| {
                if signed_area(&hole) > 0.0 {
                    hole.reverse();
                }
                hole
            })
            .collect();

        let with_holes = bridge_holes(&ring, &holes);
        let triangles = ear_clip(&with_holes);
        pieces.extend(merge_into_convex(triangles));
    }
    pieces
}

/// Detect reflection of complete boundary edges, independent of winding/start vertex.
fn reflection_symmetric(outer: &Outline, holes: &[Outline]) -> bool {
    (0..2).any(|axis| {
        let min = outer.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
        let max = outer
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        let reflect = |mut p: geometry_math::vec::Vec2d| {
            p[axis] = min + max - p[axis];
            p
        };
        let close = |a: geometry_math::vec::Vec2d, b: geometry_math::vec::Vec2d| {
            geometry_math::vec::length(a - b) < 1e-9
        };
        std::iter::once(outer).chain(holes.iter()).all(|ring| {
            ring.iter()
                .zip(ring.iter().cycle().skip(1))
                .all(|(&a, &b)| {
                    ring.iter()
                        .zip(ring.iter().cycle().skip(1))
                        .any(|(&c, &d)| {
                            (close(reflect(a), c) && close(reflect(b), d))
                                || (close(reflect(a), d) && close(reflect(b), c))
                        })
                })
        })
    })
}

/// Between consecutive vertex heights, even-odd filled spans are convex trapezoids.
/// Evaluating both ends on the original edges keeps adjacent bands coincident.
fn horizontal_bands(outer: &Outline, holes: &[Outline]) -> Vec<Outline> {
    use geometry_math::vec::Vec2d;
    let rings = std::iter::once(outer)
        .chain(holes.iter())
        .collect::<Vec<_>>();
    let mut levels = rings
        .iter()
        .flat_map(|r| r.iter().map(|p| p.y()))
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
    let mut result = Vec::new();
    for band in levels.windows(2) {
        let (bottom, top) = (band[0], band[1]);
        let middle = (bottom + top) * 0.5;
        let mut crossings = Vec::new();
        for ring in &rings {
            for (&a, &b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                if middle <= a.y().min(b.y()) || middle >= a.y().max(b.y()) {
                    continue;
                }
                let at = |y: f64| a.x() + (y - a.y()) * (b.x() - a.x()) / (b.y() - a.y());
                crossings.push((at(middle), at(bottom), at(top)));
            }
        }
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in crossings.as_chunks::<2>().0 {
            let (left, right) = (pair[0], pair[1]);
            let mut polygon = vec![
                Vec2d::new(left.1, bottom),
                Vec2d::new(right.1, bottom),
                Vec2d::new(right.2, top),
                Vec2d::new(left.2, top),
            ];
            polygon.dedup_by(|a, b| geometry_math::vec::length(*a - *b) < 1e-10);
            if polygon.len() > 2
                && geometry_math::vec::length(polygon[0] - *polygon.last().unwrap()) < 1e-10
            {
                polygon.pop();
            }
            if polygon.len() >= 3 {
                result.push(polygon);
            }
        }
    }
    result
}

/// Whether every corner of `inner` lies inside `outer`.
fn contains_ring(outer: &[geometry_math::vec::Vec2d], inner: &[geometry_math::vec::Vec2d]) -> bool {
    inner.iter().all(|point| point_in_ring(outer, *point))
}

/// Whether a point is inside a ring, by the crossing rule.
fn point_in_ring(ring: &[geometry_math::vec::Vec2d], point: geometry_math::vec::Vec2d) -> bool {
    let mut inside = false;
    let mut previous = ring[ring.len() - 1];
    for corner in ring {
        let crosses = (corner.y() > point.y()) != (previous.y() > point.y());
        if crosses {
            let at = (previous.x() - corner.x()) * (point.y() - corner.y())
                / (previous.y() - corner.y())
                + corner.x();
            if point.x() < at {
                inside = !inside;
            }
        }
        previous = *corner;
    }
    inside
}

/// Cuts each hole into the ring around it, leaving one ring that has no holes.
///
/// The usual bridge: the hole's rightmost corner is joined to a corner of the outer ring,
/// and the hole is walked into the outer ring's own list at that point. The seam is a
/// zero-width slit, which the triangulation then treats as an edge like any other.
fn bridge_holes(
    ring: &[geometry_math::vec::Vec2d],
    holes: &[Outline],
) -> Vec<geometry_math::vec::Vec2d> {
    let mut out = ring.to_vec();
    // The rightmost hole first, so a bridge never has to cross a hole that is still to
    // come.
    let mut holes: Vec<&Outline> = holes.iter().collect();
    holes.sort_by(|a, b| rightmost(b).x().total_cmp(&rightmost(a).x()));

    for hole in holes {
        let start = hole
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.x().total_cmp(&b.x()))
            .map(|(index, _)| index)
            .unwrap_or(0);
        let from = hole[start];

        // Cast rightward to the nearest boundary. The nearest vertex can lie
        // behind a concavity or create crossing seams between several counters.
        let hit = out
            .iter()
            .enumerate()
            .filter_map(|(index, a)| {
                let b = out[(index + 1) % out.len()];
                if (a.y() > from.y()) == (b.y() > from.y()) {
                    return None;
                }
                let x = a.x() + (from.y() - a.y()) * (b.x() - a.x()) / (b.y() - a.y());
                (x > from.x() + 1e-9)
                    .then_some((index, geometry_math::vec::Vec2d::new(x, from.y())))
            })
            .min_by(|(_, a), (_, b)| a.x().total_cmp(&b.x()));
        let Some((edge, point)) = hit else {
            continue;
        };
        let next = (edge + 1) % out.len();
        let join = if same_point(out[edge], point) {
            edge
        } else if same_point(out[next], point) {
            next
        } else {
            out.insert(edge + 1, point);
            edge + 1
        };

        // After the outer corner: into the hole, once round it, back to where the hole
        // was entered, and back out to the outer corner. The seam is walked twice, which
        // is what makes it a slit rather than an edge.
        let mut inserted = Vec::with_capacity(hole.len() + 2);
        for step in 0..hole.len() {
            inserted.push(hole[(start + step) % hole.len()]);
        }
        inserted.push(from);
        inserted.push(out[join]);
        out.splice(join + 1..join + 1, inserted);
    }
    out
}

/// The rightmost corner of a ring.
fn rightmost(ring: &[geometry_math::vec::Vec2d]) -> geometry_math::vec::Vec2d {
    ring.iter()
        .copied()
        .max_by(|a, b| a.x().total_cmp(&b.x()))
        .unwrap_or_default()
}

/// Twice the signed area of a ring: positive when it winds counter-clockwise.
fn signed_area(ring: &[geometry_math::vec::Vec2d]) -> f64 {
    let mut sum = 0.0;
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + 1) % ring.len()];
        sum += a.x() * b.y() - b.x() * a.y();
    }
    sum
}

/// Cuts a ring with no holes into triangles, by clipping ears.
fn ear_clip(ring: &[geometry_math::vec::Vec2d]) -> Vec<Vec<geometry_math::vec::Vec2d>> {
    let mut corners: Vec<geometry_math::vec::Vec2d> = ring.to_vec();
    if signed_area(&corners) < 0.0 {
        corners.reverse();
    }

    let mut triangles = Vec::new();
    let mut guard = corners.len() * corners.len() + 16;
    while corners.len() > 3 && guard > 0 {
        guard -= 1;
        let count = corners.len();
        let mut clipped = false;
        for index in 0..count {
            let previous = corners[(index + count - 1) % count];
            let corner = corners[index];
            let next = corners[(index + 1) % count];

            // A convex corner whose triangle holds no other corner is an ear.
            if cross(previous, corner, next) <= 0.0 {
                continue;
            }
            let holds_another = corners.iter().enumerate().any(|(other, point)| {
                other != index
                    && other != (index + count - 1) % count
                    && other != (index + 1) % count
                    && blocks_ear(previous, corner, next, *point)
            });
            if holds_another {
                continue;
            }

            triangles.push(vec![previous, corner, next]);
            corners.remove(index);
            clipped = true;
            break;
        }
        if !clipped {
            // A ring that will not clip is one whose seams overlap; what is left is
            // dropped rather than looped over for ever.
            break;
        }
    }
    if corners.len() == 3 {
        triangles.push(corners);
    }
    triangles
}

/// Twice the signed area of the triangle `a b c`.
fn cross(
    a: geometry_math::vec::Vec2d,
    b: geometry_math::vec::Vec2d,
    c: geometry_math::vec::Vec2d,
) -> f64 {
    (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x())
}

/// A distinct corner inside or on an ear blocks clipping. Removing an ear through
/// another boundary vertex can cut across a concavity and silently lose geometry.
/// Duplicate seam endpoints used to bridge holes are excluded explicitly.
fn blocks_ear(
    a: geometry_math::vec::Vec2d,
    b: geometry_math::vec::Vec2d,
    c: geometry_math::vec::Vec2d,
    point: geometry_math::vec::Vec2d,
) -> bool {
    if same_point(a, point) || same_point(b, point) || same_point(c, point) {
        return false;
    }
    const EDGE: f64 = 1e-9;
    cross(a, b, point) >= -EDGE && cross(b, c, point) >= -EDGE && cross(c, a, point) >= -EDGE
}

/// Joins neighbouring triangles back together while what they make stays convex.
///
/// Hertel and Mehlhorn: every diagonal that is not needed is taken out, which leaves at
/// most four times as many pieces as the fewest possible -- and, for a letter, a handful.
fn merge_into_convex(
    triangles: Vec<Vec<geometry_math::vec::Vec2d>>,
) -> Vec<Vec<geometry_math::vec::Vec2d>> {
    let mut pieces = triangles;
    let mut merged_any = true;
    while merged_any {
        merged_any = false;
        'outer: for one in 0..pieces.len() {
            for other in (one + 1)..pieces.len() {
                let Some(joined) = join_if_convex(&pieces[one], &pieces[other]) else {
                    continue;
                };
                pieces[one] = joined;
                pieces.remove(other);
                merged_any = true;
                break 'outer;
            }
        }
    }
    pieces
}

/// Joins two polygons along a shared edge, if the result is convex.
fn join_if_convex(
    one: &[geometry_math::vec::Vec2d],
    other: &[geometry_math::vec::Vec2d],
) -> Option<Vec<geometry_math::vec::Vec2d>> {
    // The shared edge, as a pair of positions in each polygon.
    let mut shared = None;
    for a in 0..one.len() {
        let a_next = (a + 1) % one.len();
        for b in 0..other.len() {
            let b_next = (b + 1) % other.len();
            if same_point(one[a], other[b_next]) && same_point(one[a_next], other[b]) {
                shared = Some((a, b));
            }
        }
    }
    let (a, b) = shared?;

    // Walk one polygon from the end of the shared edge, then the other from the end of
    // its own copy of it.
    let mut joined = Vec::with_capacity(one.len() + other.len() - 2);
    for step in 1..one.len() {
        joined.push(one[(a + step) % one.len()]);
    }
    for step in 1..other.len() {
        joined.push(other[(b + step) % other.len()]);
    }

    is_convex(&joined).then_some(joined)
}

/// Whether two positions are the same corner.
fn same_point(a: geometry_math::vec::Vec2d, b: geometry_math::vec::Vec2d) -> bool {
    geometry_math::vec::squared_distance(a, b) < 1e-12
}

/// Whether a ring turns the same way at every corner.
fn is_convex(ring: &[geometry_math::vec::Vec2d]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut sign = 0.0;
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + 1) % ring.len()];
        let c = ring[(index + 2) % ring.len()];
        let turn = cross(a, b, c);
        if turn.abs() < 1e-9 {
            continue;
        }
        if sign == 0.0 {
            sign = turn;
        } else if (turn > 0.0) != (sign > 0.0) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod outline_tests {
    use super::*;
    use geometry_math::vec::Vec2d;

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Outline {
        vec![
            Vec2d::new(x0, y0),
            Vec2d::new(x1, y0),
            Vec2d::new(x1, y1),
            Vec2d::new(x0, y1),
        ]
    }

    #[test]
    fn a_square_comes_out_as_one_brush() {
        let world = Bbox3d::from_scalars(-1024.0, 1024.0);
        let brushes = brushes_from_outlines(
            &[square(0.0, 0.0, 1.0, 1.0)],
            32.0,
            8.0,
            "stone",
            Vec3d::zero(),
            &world,
        );
        assert_eq!(brushes.len(), 1, "a convex outline is one brush");
        let bounds = brushes[0].bounds();
        assert_eq!(bounds.size(), Vec3d::new(32.0, 8.0, 32.0));
    }

    #[test]
    fn a_letter_with_a_hole_leaves_the_hole_empty() {
        let world = Bbox3d::from_scalars(-1024.0, 1024.0);
        // An O: a square with a square hole, the hole wound the other way as a font
        // writes it.
        let mut hole = square(0.25, 0.25, 0.75, 0.75);
        hole.reverse();
        let brushes = brushes_from_outlines(
            &[square(0.0, 0.0, 1.0, 1.0), hole],
            32.0,
            8.0,
            "stone",
            Vec3d::zero(),
            &world,
        );
        assert!(brushes.len() >= 4, "a ring needs at least four pieces");

        // Nothing fills the hole: the middle of the ring is outside every brush.
        let middle = Vec3d::new(16.0, 4.0, 16.0);
        assert!(
            !brushes.iter().any(|brush| brush.contains_point(middle)),
            "the hole was filled in"
        );
        // And the ring itself is solid.
        let on_the_ring = Vec3d::new(4.0, 4.0, 16.0);
        assert!(brushes
            .iter()
            .any(|brush| brush.contains_point(on_the_ring)));
    }

    #[test]
    fn a_shape_that_is_not_convex_is_cut_into_pieces_that_are() {
        let world = Bbox3d::from_scalars(-1024.0, 1024.0);
        // An L.
        let outline = vec![
            Vec2d::new(0.0, 0.0),
            Vec2d::new(1.0, 0.0),
            Vec2d::new(1.0, 0.3),
            Vec2d::new(0.3, 0.3),
            Vec2d::new(0.3, 1.0),
            Vec2d::new(0.0, 1.0),
        ];
        let brushes = brushes_from_outlines(&[outline], 32.0, 8.0, "stone", Vec3d::zero(), &world);
        assert_eq!(brushes.len(), 2, "an L is two convex pieces");
    }
}
