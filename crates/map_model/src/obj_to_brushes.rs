//! Turning a triangle mesh into brushes.
//!
//! A brush is a convex solid and a modelling tool's mesh is neither convex nor solid, so
//! there is no way to turn one into the other that keeps the shape and the count. What
//! this does is give every triangle a thickness: the triangle's corners are offset
//! inwards and the two become a wedge, which is convex whatever the mesh around it does.
//! The result is a shell of the model, hollow, with a wall as thick as it was asked for.
//!
//! What holds, and what does not. Every pair of triangles sharing an **edge** gives two
//! brushes that share a face and do not overlap -- `two_triangles_sharing_an_edge_...`
//! holds that at every fold angle. Pairs that share only a **corner** can still meet in
//! a sliver near it: the monkey has 1642 such pairs out of 962 brushes, none of them
//! edge neighbours, where the obvious construction had 4734 of both kinds. Separating
//! the ones round a corner needs a plane per pair and there is no single one that does
//! it, so this stops there.
//!
//! Three things get it that far, none of them obvious:
//!
//! A corner goes back along the **corner's** normal -- the average of the triangles that
//! meet there -- not the triangle's, so two neighbours agree where their shared back edge
//! is. That is what a modelling tool's own solidify does, and for the same reason.
//!
//! The side of a wedge along an edge shared with another triangle is the plane that
//! bisects the two. Both wedges are bounded by that same plane, from opposite sides, and
//! two convex solids either side of one plane cannot overlap -- so no pair of neighbours
//! can, whatever the surface does between them. Taking the plane through three of the
//! four corners of that side instead, which is the obvious thing again, leaves the fourth
//! off it and the brush reaching past into its neighbour.
//!
//! And the wall is thinner where the model has no room for it: a corner asks how far it
//! is to the far side of the model and takes less than half, and asks how square-on the
//! triangles meeting there are and takes less again where the surface folds inwards.
//! Without the first, the ears and the brow -- thinner than two walls -- have the shell
//! meeting itself coming back.
//!
//! That is what a level wants anyway. Brushes are volumes and a level is built out of
//! walls; a solid-filled model would be a solid block of geometry no one can walk into.
//! It is also the only decomposition that cannot fail: a convex decomposition of an
//! arbitrary mesh is a hard problem with cases that defeat it, and a per-triangle wedge
//! has no cases at all.
//!
//! The cost is one brush per triangle, which is a lot -- Blender's monkey is nine hundred
//! and sixty-eight. Neighbouring triangles that share a plane are merged first, which
//! takes the flat parts of a model down to one brush each and leaves the curved parts
//! alone.

use geometry_math::bbox::Bbox3d;
use geometry_math::plane::Plane3d;
use geometry_math::vec::{self, Vec3d};

use crate::brush::Brush;
use crate::brush_face::BrushFace;

/// What the conversion was asked for.
#[derive(Debug, Clone)]
pub struct Options {
    /// How thick the shell is, in map units.
    pub thickness: f64,
    /// The size the model's longest side is scaled to, in map units. `None` keeps the
    /// mesh's own units, which for a modelling tool is usually about two.
    pub size: Option<f64>,
    /// The material every face wears.
    pub material: String,
    /// The material for a face that is inside the model: the back of the shell and the
    /// sides where one brush meets the next. A compiler and every engine exporter here
    /// throw a face wearing this away, so what is written is only the surface anyone can
    /// see. Empty leaves every face wearing `material`.
    pub inside_material: String,
    /// Join brushes into one where the join adds no space and the result runs into
    /// nothing else. What that buys is fewer brushes for the same shape.
    pub join: bool,
    /// Stand the model up: a modelling tool writes an `.obj` with Y up and a map has Z
    /// up, so a model imported as written lies on its back.
    pub y_up: bool,
    /// Build the model as horizontal layers instead of shelling its surface.
    ///
    /// The way a person builds an organic shape in a brush editor: slice the solid into
    /// slabs, and cut each slab's cross-section into convex pieces. Neighbouring slabs
    /// share the plane between them and pieces within a slab share their edges, so every
    /// brush joins the next by a face and none can overlap. It needs `snap` to say how
    /// tall a slab is.
    pub layered: bool,
    /// Fill the model with grid-aligned boxes instead of shelling its surface.
    ///
    /// A shell cannot avoid overlapping itself where the model is thinner than two walls
    /// -- the monkey's ears and brow are, and no thickness small enough to clear them
    /// leaves brushes worth having. Boxes on a grid cannot overlap at all, by
    /// construction, and a level built of brushes is built of boxes anyway. It needs
    /// `snap` to say how big a box is.
    pub solid: bool,
    /// Round every corner onto a grid of this size before anything else, in map units
    /// after scaling. `None` keeps the mesh's own corners.
    ///
    /// This is the simplification: corners that land on the same grid point become one,
    /// the triangles between them collapse and are dropped, and what is left is coarser
    /// and has fewer brushes. It also puts the model on the grid, which is where a
    /// brush-built level wants its geometry anyway.
    pub snap: Option<f64>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            thickness: 4.0,
            size: Some(256.0),
            material: "stone".to_string(),
            inside_material: "skip".to_string(),
            join: true,
            y_up: true,
            layered: false,
            solid: false,
            snap: None,
        }
    }
}

/// What the conversion produced, and what it could not.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many triangles the mesh had.
    pub triangles: usize,
    /// How many brushes came out.
    pub brushes: usize,
    /// Triangles that made no brush: ones with no area, or so thin that thickening them
    /// produced a solid the polyhedron builder threw away.
    pub dropped: usize,
    /// Triangles that collapsed onto the grid, two of their corners having become one.
    pub collapsed: usize,
    /// Brushes that went away by being joined into a neighbour.
    pub joined: usize,
}

/// Reads a Wavefront `.obj` and builds the brushes for what it holds.
///
/// The positions and the faces, and nothing else: a material, a texture coordinate and a
/// normal all belong to a mesh rather than to a brush, and a brush's material is the one
/// the options name. A polygon of more than three corners is fanned into triangles from
/// its first, which is what the rest of this workspace does with an `.obj` face.
pub fn brushes_from_obj(
    source: &str,
    world_bounds: &Bbox3d,
    options: &Options,
) -> (Vec<Brush>, Report) {
    let mut positions: Vec<Vec3d> = Vec::new();
    let mut triangles: Vec<[Vec3d; 3]> = Vec::new();

    for line in source.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        match words.next() {
            Some("v") => {
                let mut number = || words.next().and_then(|w| w.parse::<f64>().ok());
                if let (Some(x), Some(y), Some(z)) = (number(), number(), number()) {
                    positions.push(Vec3d::new(x, y, z));
                }
            }
            Some("f") => {
                // `v`, `v/vt`, `v//vn` and `v/vt/vn` all begin with the position, and a
                // negative index counts back from the end.
                let corners: Vec<usize> = words
                    .filter_map(|word| {
                        let index: i64 = word.split('/').next()?.parse().ok()?;
                        if index > 0 {
                            Some(index as usize - 1)
                        } else if index < 0 {
                            positions.len().checked_sub((-index) as usize)
                        } else {
                            None
                        }
                    })
                    .filter(|at| *at < positions.len())
                    .collect();
                for at in 1..corners.len().saturating_sub(1) {
                    triangles.push([
                        positions[corners[0]],
                        positions[corners[at]],
                        positions[corners[at + 1]],
                    ]);
                }
            }
            _ => {}
        }
    }

    brushes_from_triangles(&triangles, world_bounds, options)
}

/// Builds the brushes for a mesh of triangles, each three positions in order.
///
/// `world_bounds` is what a brush is clipped against, as everywhere else.
pub fn brushes_from_triangles(
    triangles: &[[Vec3d; 3]],
    world_bounds: &Bbox3d,
    options: &Options,
) -> (Vec<Brush>, Report) {
    let mut report = Report {
        triangles: triangles.len(),
        ..Report::default()
    };

    let stood_up = if options.y_up {
        // The same fold the `.obj` export undoes on the way out: what the file calls up
        // becomes the map's up, and what it calls forward becomes the map's -y.
        triangles
            .iter()
            .map(|t| t.map(|p| Vec3d::new(p.x(), -p.z(), p.y())))
            .collect()
    } else {
        triangles.to_vec()
    };
    let scaled = scale_to_fit(&stood_up, options.size);

    if options.layered {
        let Some(grid) = options.snap.filter(|g| *g > 0.0) else {
            return (Vec::new(), report);
        };
        let mut brushes = Vec::new();
        for (bounds, rings) in slabs(&scaled, grid) {
            for piece in crate::text::convex_pieces(&rings) {
                match slab_brush(&piece, bounds, world_bounds, &options.material) {
                    Some(brush) => brushes.push(brush),
                    None => report.dropped += 1,
                }
            }
        }
        report.brushes = brushes.len();
        return (brushes, report);
    }

    if options.solid {
        let Some(grid) = options.snap.filter(|g| *g > 0.0) else {
            return (Vec::new(), report);
        };
        let boxes = fill_with_boxes(&scaled, grid);
        let mut brushes = Vec::new();
        for bounds in boxes {
            match crate::shapes::create_cuboid(world_bounds, &bounds, &options.material) {
                Ok(brush) => brushes.push(brush),
                Err(_) => report.dropped += 1,
            }
        }
        report.brushes = brushes.len();
        return (brushes, report);
    }

    let snapped = snap_to_grid(&scaled, options.snap);
    report.collapsed = scaled.len() - snapped.len();

    // Which way each corner faces, before any merging: two triangles that meet at a
    // corner have to agree about where its back is, or the wedges built from them cross.
    let corners = corner_normals(&snapped);

    let edges = edge_bisectors(&snapped);
    let thickness = corner_thickness(&snapped, &corners, options.thickness.abs().max(1e-3));

    let mut brushes: Vec<Brush> = Vec::new();
    for triangle in &snapped {
        match wedge(
            triangle,
            &thickness,
            &corners,
            &edges,
            world_bounds,
            &options.material,
        ) {
            Some(brush) => brushes.push(brush),
            None => report.dropped += 1,
        }
    }
    if options.join {
        let before = brushes.len();
        brushes = join_convex(brushes, world_bounds);
        report.joined = before - brushes.len();
    }
    if !options.inside_material.is_empty() {
        let surface: Vec<Plane3d> = snapped.iter().filter_map(|t| plane_of(t)).collect();
        brushes = brushes
            .iter()
            .map(|brush| {
                dress(
                    brush,
                    &surface,
                    world_bounds,
                    &options.material,
                    &options.inside_material,
                )
                .unwrap_or_else(|| brush.clone())
            })
            .collect();
    }

    report.brushes = brushes.len();
    (brushes, report)
}

/// Puts the inside material on every face that is not part of the model's surface.
///
/// A face is outside when it lies in the plane of one of the mesh's own triangles and
/// faces the same way. Everything else is the back of the shell or a wall against the
/// next brush, and an export throws a face wearing the inside material away -- which is
/// most of the faces in the file.
///
/// By plane rather than by which face was built first, so it still holds after two
/// brushes have been joined into one and the faces are the hull's rather than the
/// wedge's.
fn dress(
    brush: &Brush,
    surface: &[Plane3d],
    world_bounds: &Bbox3d,
    outside: &str,
    inside: &str,
) -> Option<Brush> {
    let on_surface = |plane: &Plane3d| {
        surface.iter().any(|face| {
            vec::dot(face.normal, plane.normal) > 0.9995
                && (face.distance - plane.distance).abs() < 1e-4
        })
    };
    let faces: Vec<BrushFace> = brush
        .faces()
        .iter()
        .map(|face| {
            let mut face = face.clone();
            let name = if on_surface(&face.plane) { outside } else { inside };
            face.material_name = name.into();
            face
        })
        .collect();
    Brush::from_faces(world_bounds, faces).ok()
}

/// Joins brushes into one wherever the join changes nothing.
///
/// Two brushes may be replaced by the convex hull of the two only when that hull is the
/// two and nothing else -- when their union was already convex. The test is the volume:
/// the hull has the volume of both exactly then, and more whenever it has swallowed space
/// between them. And a join is only kept when the brush it makes runs into no other
/// brush, since a hull that reaches past its neighbours would trade one problem for
/// another.
///
/// Greedy and repeated: every accepted join makes new neighbours worth trying, so the
/// sweep runs until a pass changes nothing.
fn join_convex(mut brushes: Vec<Brush>, world_bounds: &Bbox3d) -> Vec<Brush> {
    loop {
        let mut joined = false;
        let mut at = 0;
        while at < brushes.len() {
            let mut with = at + 1;
            let mut merged_into = None;
            while with < brushes.len() {
                if let Some(one) = join_two(&brushes[at], &brushes[with], world_bounds) {
                    // Not into anything else, either.
                    let clear = brushes.iter().enumerate().all(|(other, brush)| {
                        other == at || other == with || !solids_meet(&one, brush)
                    });
                    if clear {
                        merged_into = Some((with, one));
                        break;
                    }
                }
                with += 1;
            }
            match merged_into {
                Some((with, one)) => {
                    brushes[at] = one;
                    brushes.remove(with);
                    joined = true;
                }
                None => at += 1,
            }
        }
        if !joined {
            return brushes;
        }
    }
}

/// The two as one brush, if the join adds nothing.
fn join_two(a: &Brush, b: &Brush, world_bounds: &Bbox3d) -> Option<Brush> {
    // Far apart is most pairs, and the hull is the expensive part.
    if !a.bounds().intersects(&b.bounds()) {
        return None;
    }
    let mut points = a.vertex_positions();
    points.extend(b.vertex_positions());
    let geometry =
        crate::polyhedron::Polyhedron::from_points(&points, geometry_math::scalar::Float::ALMOST_ZERO).ok()?;
    let one = Brush::from_polyhedron(geometry, a.faces().first()?.material_name.as_str());
    if !world_bounds.contains(&one.bounds()) {
        return None;
    }

    // The join is free exactly when it swallowed no space between the two.
    let together = a.volume() + b.volume();
    let slack = together.max(1.0) * 1e-6;
    (one.volume() - together < slack).then_some(one)
}

/// Whether two convex solids have volume in common, rather than merely touching.
fn solids_meet(a: &Brush, b: &Brush) -> bool {
    if !a.bounds().intersects(&b.bounds()) {
        return false;
    }
    !apart(a, b) && !apart(b, a)
}

/// Whether a face of `a` has all of `b` on or beyond it.
fn apart(a: &Brush, b: &Brush) -> bool {
    let corners: Vec<Vec3d> = b.geometry().vertices().iter().map(|v| v.position).collect();
    a.faces().iter().any(|face| {
        corners
            .iter()
            .all(|corner| face.plane.point_distance(*corner) > -1e-6)
    })
}

/// Scales the mesh so its longest side is `size`, about its own middle.
///
/// A modelling tool's model is a couple of units across and a map's grid is measured in
/// tens, so an unscaled import lands as a speck. `None` keeps what the file said.
fn scale_to_fit(triangles: &[[Vec3d; 3]], size: Option<f64>) -> Vec<[Vec3d; 3]> {
    let Some(size) = size else {
        return triangles.to_vec();
    };
    let mut bounds: Option<Bbox3d> = None;
    for triangle in triangles {
        for corner in triangle {
            bounds = Some(match bounds {
                Some(box3) => box3.merged_with_point(*corner),
                None => Bbox3d::new(*corner, *corner),
            });
        }
    }
    let Some(bounds) = bounds else {
        return Vec::new();
    };
    let extent = bounds.size();
    let longest = extent.x().max(extent.y()).max(extent.z());
    if longest <= 0.0 {
        return triangles.to_vec();
    }
    let factor = size / longest;
    let centre = bounds.center();
    triangles
        .iter()
        .map(|triangle| triangle.map(|corner| (corner - centre) * factor))
        .collect()
}

/// The model sliced into horizontal slabs, each with the outlines of its cross-section.
///
/// The cut is taken at the middle of the slab, so a slab is as wide as the model is
/// half-way up it. Each triangle crossing that height contributes a segment, and the
/// segments are chained into closed rings.
fn slabs(triangles: &[[Vec3d; 3]], grid: f64) -> Vec<((f64, f64), Vec<crate::text::Outline>)> {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for triangle in triangles {
        for corner in triangle {
            low = low.min(corner.z());
            high = high.max(corner.z());
        }
    }
    if !low.is_finite() || high <= low {
        return Vec::new();
    }

    let first = (low / grid).floor() as i64;
    let last = (high / grid).ceil() as i64;
    let mut out = Vec::new();
    for step in first..last {
        let bottom = step as f64 * grid;
        let top = bottom + grid;
        let rings = rings_at(triangles, (bottom + top) / 2.0);
        if !rings.is_empty() {
            out.push(((bottom, top), rings));
        }
    }
    out
}

/// The closed outlines where the mesh crosses the plane `z = height`.
fn rings_at(triangles: &[[Vec3d; 3]], height: f64) -> Vec<crate::text::Outline> {
    let mut segments: Vec<(geometry_math::vec::Vec2d, geometry_math::vec::Vec2d)> = Vec::new();
    for triangle in triangles {
        let mut crossings = Vec::new();
        for at in 0..3 {
            let (a, b) = (triangle[at], triangle[(at + 1) % 3]);
            let (za, zb) = (a.z() - height, b.z() - height);
            // One corner either side: the edge crosses. A corner exactly on the plane is
            // taken as being above it, so an edge lying in the plane contributes nothing
            // and a corner touching it is not counted twice.
            if (za < 0.0) == (zb < 0.0) {
                continue;
            }
            let t = za / (za - zb);
            let point = a + (b - a) * t;
            crossings.push(geometry_math::vec::Vec2d::new(point.x(), point.y()));
        }
        if crossings.len() == 2 {
            segments.push((crossings[0], crossings[1]));
        }
    }
    chain(segments)
}

/// Joins loose segments into closed rings, end to end.
fn chain(mut segments: Vec<(geometry_math::vec::Vec2d, geometry_math::vec::Vec2d)>) -> Vec<crate::text::Outline> {
    // A cut through a mesh gives the same point from two triangles at slightly different
    // precision, so ends are matched by nearness rather than by equality.
    const NEAR: f64 = 1e-6;
    let close = |a: geometry_math::vec::Vec2d, b: geometry_math::vec::Vec2d| {
        (a.x() - b.x()).abs() < NEAR && (a.y() - b.y()).abs() < NEAR
    };

    let mut rings = Vec::new();
    while let Some((start, mut end)) = segments.pop() {
        let mut ring = vec![start, end];
        loop {
            let next = segments.iter().position(|(a, b)| close(*a, end) || close(*b, end));
            let Some(at) = next else {
                break;
            };
            let (a, b) = segments.swap_remove(at);
            end = if close(a, end) { b } else { a };
            if close(end, start) {
                break;
            }
            ring.push(end);
        }
        if ring.len() >= 3 {
            rings.push(ring);
        }
    }
    rings
}

/// One convex piece of a slab, extruded from its bottom to its top.
///
/// From its corners rather than from planes: the piece is convex, so every corner is on
/// the solid and its hull is the prism. That is how `text::prism` builds a letter's
/// stroke, and it saves having to get a winding right for each of the walls.
fn slab_brush(
    polygon: &[geometry_math::vec::Vec2d],
    (bottom, top): (f64, f64),
    world_bounds: &Bbox3d,
    material: &str,
) -> Option<Brush> {
    if polygon.len() < 3 || top <= bottom {
        return None;
    }
    let mut points = Vec::with_capacity(polygon.len() * 2);
    for corner in polygon {
        points.push(Vec3d::new(corner.x(), corner.y(), bottom));
        points.push(Vec3d::new(corner.x(), corner.y(), top));
    }
    let geometry =
        crate::polyhedron::Polyhedron::from_points(&points, geometry_math::scalar::Float::ALMOST_ZERO).ok()?;
    let brush = Brush::from_polyhedron(geometry, material);
    world_bounds.contains(&brush.bounds()).then_some(brush)
}

/// Fills the inside of a closed mesh with grid-aligned boxes.
///
/// Every cell of a grid over the model is tested for being inside it, and the cells that
/// are get joined into as few boxes as they will make. Two boxes on a grid never share
/// space, so nothing here can overlap; and a box is what a brush-built level is made of,
/// so the result is geometry an editor can go on working with rather than a mesh in
/// disguise.
///
/// Inside is decided by counting how many triangles a ray from the cell's middle crosses
/// on its way out: an odd number means it started inside. The ray goes along +x, and a
/// crossing exactly at a triangle's edge is counted by a consistent rule, so a cell on a
/// seam is not counted twice.
fn fill_with_boxes(triangles: &[[Vec3d; 3]], grid: f64) -> Vec<Bbox3d> {
    let mut bounds: Option<Bbox3d> = None;
    for triangle in triangles {
        for corner in triangle {
            bounds = Some(match bounds {
                Some(box3) => box3.merged_with_point(*corner),
                None => Bbox3d::new(*corner, *corner),
            });
        }
    }
    let Some(bounds) = bounds else {
        return Vec::new();
    };

    let cell = |v: f64| (v / grid).floor() as i64;
    let (lo, hi) = (bounds.min, bounds.max);
    let (x0, y0, z0) = (cell(lo.x()), cell(lo.y()), cell(lo.z()));
    let (x1, y1, z1) = (cell(hi.x()) + 1, cell(hi.y()) + 1, cell(hi.z()) + 1);
    let (nx, ny, nz) = (
        (x1 - x0).max(0) as usize,
        (y1 - y0).max(0) as usize,
        (z1 - z0).max(0) as usize,
    );
    if nx == 0 || ny == 0 || nz == 0 || nx * ny * nz > 4_000_000 {
        return Vec::new();
    }

    let at = |x: usize, y: usize, z: usize| (z * ny + y) * nx + x;
    let mut inside = vec![false; nx * ny * nz];
    for z in 0..nz {
        for y in 0..ny {
            // One ray per row along +x, so a row is decided in one pass over the
            // triangles rather than one pass per cell.
            let oy = (y0 + y as i64) as f64 * grid + grid / 2.0;
            let oz = (z0 + z as i64) as f64 * grid + grid / 2.0;
            let crossings = crossings_along_x(triangles, oy, oz);
            if crossings.is_empty() {
                continue;
            }
            for x in 0..nx {
                let ox = (x0 + x as i64) as f64 * grid + grid / 2.0;
                let ahead = crossings.iter().filter(|c| **c > ox).count();
                if ahead % 2 == 1 {
                    inside[at(x, y, z)] = true;
                }
            }
        }
    }

    greedy_boxes(&inside, (nx, ny, nz), (x0, y0, z0), grid)
}

/// Where a ray along +x through `(y, z)` crosses the mesh.
fn crossings_along_x(triangles: &[[Vec3d; 3]], y: f64, z: f64) -> Vec<f64> {
    let mut out = Vec::new();
    for triangle in triangles {
        // Barycentric in the y/z plane, which is what the ray's direction leaves.
        let (a, b, c) = (triangle[0], triangle[1], triangle[2]);
        let (aby, abz) = (b.y() - a.y(), b.z() - a.z());
        let (acy, acz) = (c.y() - a.y(), c.z() - a.z());
        let denominator = aby * acz - abz * acy;
        if denominator.abs() < 1e-12 {
            continue;
        }
        let (py, pz) = (y - a.y(), z - a.z());
        let u = (py * acz - pz * acy) / denominator;
        let v = (pz * aby - py * abz) / denominator;
        // Closed, with a little slack: a ray landing exactly on the diagonal two
        // triangles share must not be turned away by both, which is what a half-open
        // test does when the two do not agree about which of them owns it. Both claim
        // it here and the pair of crossings is folded into one below.
        const SLACK: f64 = 1e-9;
        if u < -SLACK || v < -SLACK || u + v > 1.0 + SLACK {
            continue;
        }
        out.push(a.x() + u * (b.x() - a.x()) + v * (c.x() - a.x()));
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    out
}

/// Joins a field of filled cells into as few boxes as they will make.
///
/// The usual greedy sweep: take the first cell not yet in a box, grow along x while the
/// row is filled, then along y while the whole row is, then along z while the whole
/// slab is. It does not find the fewest boxes there are -- that is a hard problem -- but
/// it turns a wall into one brush and a flat region into a handful.
fn greedy_boxes(
    inside: &[bool],
    (nx, ny, nz): (usize, usize, usize),
    (x0, y0, z0): (i64, i64, i64),
    grid: f64,
) -> Vec<Bbox3d> {
    let at = |x: usize, y: usize, z: usize| (z * ny + y) * nx + x;
    let mut taken = vec![false; inside.len()];
    let mut out = Vec::new();

    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                if !inside[at(x, y, z)] || taken[at(x, y, z)] {
                    continue;
                }
                let mut w = 1;
                while x + w < nx && inside[at(x + w, y, z)] && !taken[at(x + w, y, z)] {
                    w += 1;
                }
                let mut h = 1;
                'rows: while y + h < ny {
                    for i in 0..w {
                        if !inside[at(x + i, y + h, z)] || taken[at(x + i, y + h, z)] {
                            break 'rows;
                        }
                    }
                    h += 1;
                }
                let mut d = 1;
                'slabs: while z + d < nz {
                    for j in 0..h {
                        for i in 0..w {
                            if !inside[at(x + i, y + j, z + d)] || taken[at(x + i, y + j, z + d)] {
                                break 'slabs;
                            }
                        }
                    }
                    d += 1;
                }

                for k in 0..d {
                    for j in 0..h {
                        for i in 0..w {
                            taken[at(x + i, y + j, z + k)] = true;
                        }
                    }
                }
                let min = Vec3d::new(
                    (x0 + x as i64) as f64 * grid,
                    (y0 + y as i64) as f64 * grid,
                    (z0 + z as i64) as f64 * grid,
                );
                let max = min + Vec3d::new(w as f64 * grid, h as f64 * grid, d as f64 * grid);
                out.push(Bbox3d::new(min, max));
            }
        }
    }
    out
}

/// Rounds every corner onto a grid, and drops the triangles that collapse doing it.
///
/// Vertex clustering, the simplest decimation there is: corners near each other become
/// the same corner, and a triangle whose corners are no longer three distinct points has
/// no area and goes. It is a blunt instrument -- it does not choose what to keep -- but
/// it cannot fail on any mesh, and the coarseness it leaves is the coarseness a
/// brush-built level is made of anyway.
fn snap_to_grid(triangles: &[[Vec3d; 3]], grid: Option<f64>) -> Vec<[Vec3d; 3]> {
    let Some(grid) = grid.filter(|g| *g > 0.0) else {
        return triangles.to_vec();
    };
    let round = |v: Vec3d| {
        Vec3d::new(
            (v.x() / grid).round() * grid,
            (v.y() / grid).round() * grid,
            (v.z() / grid).round() * grid,
        )
    };
    triangles
        .iter()
        .map(|t| t.map(round))
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
        .collect()
}

/// The way each corner of the mesh faces: the average of the normals of the triangles
/// that meet there.
///
/// Keyed by the rounded position, because a modelling tool writes the same corner into
/// several triangles and they have to be recognised as one. Where a corner is not in the
/// table -- it belongs to no triangle with area -- the caller falls back to the face's
/// own normal.
fn corner_normals(triangles: &[[Vec3d; 3]]) -> std::collections::HashMap<(i64, i64, i64), Vec3d> {
    let mut sums: std::collections::HashMap<(i64, i64, i64), Vec3d> = Default::default();
    for triangle in triangles {
        let Some(plane) = plane_of(triangle) else {
            continue;
        };
        for corner in triangle {
            let at = sums.entry(corner_key(*corner)).or_insert_with(Vec3d::zero);
            *at += plane.normal;
        }
    }
    sums.into_iter()
        .filter_map(|(key, sum)| {
            let length = vec::length(sum);
            (length > 1e-9).then(|| (key, sum / length))
        })
        .collect()
}

/// How thick the wall may be at each corner, which is never more than the model has room
/// for there.
///
/// A shell of a fixed thickness meets itself wherever the model is thinner than two
/// walls -- the monkey's ears and brow are -- and the two wedges then genuinely occupy
/// the same space. So each corner asks how far it is to the far side: a ray straight in
/// along the corner's own normal, and the nearest triangle it reaches. The wall there is
/// the smaller of what was asked for and a little under half that distance, so the two
/// sides stop short of each other with a gap between them.
///
/// Only the inner surface moves. From outside the model is exactly the mesh it was.
fn corner_thickness(
    triangles: &[[Vec3d; 3]],
    normals: &std::collections::HashMap<(i64, i64, i64), Vec3d>,
    asked: f64,
) -> std::collections::HashMap<(i64, i64, i64), f64> {
    // How square-on each triangle meeting at a corner is to the corner's own normal. At
    // a corner where the surface folds inwards, a triangle can face nearly sideways to
    // it, and a corner pushed the full depth along the normal comes out on the far side
    // of that triangle -- which is a wedge reaching into a neighbour it shares no edge
    // with, and the bisecting planes say nothing about those. Shortening the wall by how
    // square-on the worst of them is keeps it on the inside of all of them.
    let mut squareness: std::collections::HashMap<(i64, i64, i64), f64> = Default::default();
    for triangle in triangles {
        let Some(plane) = plane_of(triangle) else {
            continue;
        };
        for corner in triangle {
            let key = corner_key(*corner);
            let Some(normal) = normals.get(&key) else {
                continue;
            };
            let facing = vec::dot(plane.normal, *normal);
            let at = squareness.entry(key).or_insert(1.0);
            *at = at.min(facing);
        }
    }

    normals
        .iter()
        .map(|(key, normal)| {
            let from = Vec3d::new(
                key.0 as f64 / 1024.0,
                key.1 as f64 / 1024.0,
                key.2 as f64 / 1024.0,
            );
            let room = nearest_hit(triangles, from, -*normal).unwrap_or(f64::INFINITY);
            let facing = squareness.get(key).copied().unwrap_or(1.0).clamp(0.05, 1.0);
            (*key, (asked * facing).min(room * 0.45).max(1e-3))
        })
        .collect()
}

/// How far along `direction` from `from` the nearest triangle is, ignoring the ones the
/// ray starts on.
fn nearest_hit(triangles: &[[Vec3d; 3]], from: Vec3d, direction: Vec3d) -> Option<f64> {
    // Far enough off the surface not to hit the triangles meeting at this very corner,
    // and near enough not to step over a thin wall.
    const SKIN: f64 = 1e-3;
    let mut nearest: Option<f64> = None;
    for triangle in triangles {
        let (a, b, c) = (triangle[0], triangle[1], triangle[2]);
        let (e1, e2) = (b - a, c - a);
        let h = vec::cross(direction, e2);
        let det = vec::dot(e1, h);
        if det.abs() < 1e-12 {
            continue;
        }
        let inverse = 1.0 / det;
        let s = from - a;
        let u = vec::dot(s, h) * inverse;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = vec::cross(s, e1);
        let v = vec::dot(direction, q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let distance = vec::dot(e2, q) * inverse;
        if distance > SKIN && nearest.is_none_or(|best| distance < best) {
            nearest = Some(distance);
        }
    }
    nearest
}

/// A corner's key in the normal table: its position, rounded fine enough that two corners
/// a modelling tool wrote at slightly different precision meet.
fn corner_key(corner: Vec3d) -> (i64, i64, i64) {
    (
        (corner.x() * 1024.0).round() as i64,
        (corner.y() * 1024.0).round() as i64,
        (corner.z() * 1024.0).round() as i64,
    )
}

/// The plane a triangle lies in, or `None` when it has no area to have one.
fn plane_of(polygon: &[Vec3d]) -> Option<Plane3d> {
    let (a, b, c) = (*polygon.first()?, *polygon.get(1)?, *polygon.get(2)?);
    let normal = vec::cross(b - a, c - a);
    let length = vec::length(normal);
    if length <= 1e-9 {
        return None;
    }
    let normal = normal / length;
    Some(Plane3d::from_anchor_and_normal(a, normal))
}

/// A convex solid: the triangle in front, the same triangle offset behind, and one side
/// joining them along each edge.
///
/// The side along an edge shared with another triangle is that edge's bisecting plane,
/// which both wedges use, so neither can reach into the other. An edge with only one
/// triangle on it -- the rim of an open mesh -- has nothing to bisect, and takes the
/// plane through the edge and its own offset instead.
fn wedge(
    triangle: &[Vec3d; 3],
    thickness: &std::collections::HashMap<(i64, i64, i64), f64>,
    corners: &std::collections::HashMap<(i64, i64, i64), Vec3d>,
    edges: &std::collections::HashMap<EdgeKey, Vec3d>,
    world_bounds: &Bbox3d,
    material: &str,
) -> Option<Brush> {
    let plane = plane_of(triangle)?;
    // Each corner goes back along its own normal, by its own wall thickness, so a corner
    // shared with the next triangle goes back to the same place for both and the two
    // wedges meet rather than cross.
    let back_of = |corner: Vec3d| {
        let key = corner_key(corner);
        let normal = corners.get(&key).copied().unwrap_or(plane.normal);
        let deep = thickness.get(&key).copied().unwrap_or(1.0);
        corner - normal * deep
    };

    // Wound so each face's normal points out of the solid, which is the convention a
    // `.map` face line is written in.
    let mut faces = vec![
        [triangle[0], triangle[2], triangle[1]],
        [back_of(triangle[0]), back_of(triangle[1]), back_of(triangle[2])],
    ];
    for at in 0..3 {
        let a = triangle[at];
        let b = triangle[(at + 1) % 3];
        // The plane containing the edge and the surface's own direction there, which is
        // the plane the triangle on the other side of that edge uses too. Wound so its
        // normal points away from the triangle's own third corner: a face names a
        // half-space by its winding, and the wrong way round cuts the wedge to nothing.
        let third = triangle[(at + 2) % 3];
        let along = match edges.get(&edge_key(a, b)) {
            Some(surface) => *surface,
            None => back_of(a) - a,
        };
        // `plane::from_points(p1, p2, p3)` takes its normal as `cross(p3 - p1, p2 - p1)`.
        faces.push(if vec::dot(vec::cross(along, b - a), third - a) > 0.0 {
            [b, a, b + along]
        } else {
            [a, b, a + along]
        });
    }

    let faces: Vec<BrushFace> = faces
        .into_iter()
        .map(|points| {
            BrushFace::from_points(
                points,
                material,
                crate::uv::UvCoordSystem::Paraxial,
                crate::uv::UvAttributes::default(),
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Brush::from_faces(world_bounds, faces).ok()
}

/// An edge, named by its two corners in a fixed order so both triangles on it agree.
type EdgeKey = ((i64, i64, i64), (i64, i64, i64));

fn edge_key(a: Vec3d, b: Vec3d) -> EdgeKey {
    let (ka, kb) = (corner_key(a), corner_key(b));
    if ka <= kb {
        (ka, kb)
    } else {
        (kb, ka)
    }
}

/// The surface normal along each edge: the average of the triangles that share it.
///
/// The plane containing an edge and this direction is what separates the two wedges built
/// on that edge. Only edges with two triangles are in the table; an edge with one is the
/// rim of an open mesh and has nothing to separate.
fn edge_bisectors(triangles: &[[Vec3d; 3]]) -> std::collections::HashMap<EdgeKey, Vec3d> {
    let mut sums: std::collections::HashMap<EdgeKey, (Vec3d, usize)> = Default::default();
    for triangle in triangles {
        let Some(plane) = plane_of(triangle) else {
            continue;
        };
        for at in 0..3 {
            let key = edge_key(triangle[at], triangle[(at + 1) % 3]);
            let entry = sums.entry(key).or_insert((Vec3d::zero(), 0));
            entry.0 += plane.normal;
            entry.1 += 1;
        }
    }
    sums.into_iter()
        .filter(|(_, (_, count))| *count == 2)
        .filter_map(|(key, (sum, _))| {
            let length = vec::length(sum);
            (length > 1e-9).then(|| (key, sum / length))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    /// Two triangles making one unit square in the z = 0 plane.
    fn flat_square() -> Vec<[Vec3d; 3]> {
        vec![
            [
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(1.0, 1.0, 0.0),
            ],
            [
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 1.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ],
        ]
    }

    #[test]
    fn a_triangle_becomes_a_solid_wedge() {
        let options = Options {
            size: None,
            thickness: 2.0,
            // What a wedge is: not what the join does with two of them, and in the
            // triangles' own frame rather than stood up.
            join: false,
            y_up: false,
            ..Options::default()
        };
        let (brushes, report) = brushes_from_triangles(&flat_square(), &world(), &options);

        assert_eq!(report.triangles, 2);
        assert_eq!(brushes.len(), 2, "one wedge each");
        assert_eq!(report.dropped, 0);
        // A wedge of a triangle is a prism: five faces, and two units thick.
        assert_eq!(brushes[0].faces().len(), 5);
        assert!((brushes[0].bounds().size().z() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn the_model_is_scaled_to_the_size_it_was_asked_for() {
        let options = Options {
            size: Some(64.0),
            thickness: 2.0,
            ..Options::default()
        };
        let (brushes, _) = brushes_from_triangles(&flat_square(), &world(), &options);
        let bounds = brushes[0].bounds();
        // The square's longest side was 1; it is 64 now, and centred on the origin.
        assert!((bounds.size().x() - 64.0).abs() < 1e-6, "{bounds:?}");
        assert!(bounds.center().x().abs() < 1e-6, "{bounds:?}");
    }

    #[test]
    fn a_triangle_with_no_area_is_dropped_rather_than_built() {
        let flat = vec![[
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(2.0, 0.0, 0.0),
        ]];
        let (brushes, report) = brushes_from_triangles(&flat, &world(), &Options::default());
        assert!(brushes.is_empty());
        assert_eq!(report.triangles, 1);
        assert_eq!(report.dropped, 1);
    }

    #[test]
    fn an_obj_is_read_down_to_its_positions_and_faces() {
        // A quad fanned into two triangles, with the texture coordinates and normals an
        // `.obj` carries and a brush has no use for.
        let source = "\
# a note
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vt 0 0
vn 0 0 1
f 1/1/1 2/1/1 3/1/1 4/1/1
";
        let options = Options {
            size: None,
            thickness: 2.0,
            join: false,
            ..Options::default()
        };
        let (brushes, report) = brushes_from_obj(source, &world(), &options);
        assert_eq!(report.triangles, 2, "the quad fanned into two");
        assert_eq!(brushes.len(), 2);
    }

    #[test]
    fn a_negative_index_counts_back_from_the_end() {
        let source = "v 0 0 0\nv 1 0 0\nv 1 1 0\nf -3 -2 -1\n";
        let options = Options {
            size: None,
            thickness: 2.0,
            ..Options::default()
        };
        let (brushes, report) = brushes_from_obj(source, &world(), &options);
        assert_eq!(report.triangles, 1);
        assert_eq!(brushes.len(), 1);
    }

    #[test]
    fn filling_a_box_with_boxes_covers_it_and_overlaps_nothing() {
        // A closed cube two grid cells on a side: the fill should be the cube, and the
        // greedy sweep should make it one brush rather than eight.
        let cube = |lo: f64, hi: f64| {
            let c = [
                Vec3d::new(lo, lo, lo),
                Vec3d::new(hi, lo, lo),
                Vec3d::new(hi, hi, lo),
                Vec3d::new(lo, hi, lo),
                Vec3d::new(lo, lo, hi),
                Vec3d::new(hi, lo, hi),
                Vec3d::new(hi, hi, hi),
                Vec3d::new(lo, hi, hi),
            ];
            let quad = |a: usize, b: usize, d: usize, e: usize| {
                [[c[a], c[b], c[d]], [c[a], c[d], c[e]]]
            };
            let mut out = Vec::new();
            for face in [
                quad(0, 3, 2, 1),
                quad(4, 5, 6, 7),
                quad(0, 1, 5, 4),
                quad(1, 2, 6, 5),
                quad(2, 3, 7, 6),
                quad(3, 0, 4, 7),
            ] {
                out.extend_from_slice(&face);
            }
            out
        };

        let boxes = fill_with_boxes(&cube(0.0, 16.0), 8.0);
        assert_eq!(boxes.len(), 1, "eight cells joined into one box: {boxes:?}");
        assert!((boxes[0].size().x() - 16.0).abs() < 1e-9, "{boxes:?}");
        assert!((boxes[0].size().z() - 16.0).abs() < 1e-9, "{boxes:?}");
    }

    #[test]
    fn a_solid_fill_needs_a_grid_to_say_how_big_a_box_is() {
        let options = Options {
            solid: true,
            snap: None,
            ..Options::default()
        };
        let (brushes, _) = brushes_from_triangles(&flat_square(), &world(), &options);
        assert!(brushes.is_empty(), "nothing to build without one");
    }

    #[test]
    fn an_open_surface_fills_nothing() {
        // Two triangles are not a solid, so a ray through them crosses an even number of
        // times everywhere and no cell is inside.
        let options = Options {
            solid: true,
            snap: Some(0.25),
            size: None,
            ..Options::default()
        };
        let (brushes, _) = brushes_from_triangles(&flat_square(), &world(), &options);
        assert!(brushes.is_empty(), "{} brushes from a flat sheet", brushes.len());
    }

    /// A closed cube of triangles, wound so the normals face out.
    fn closed_cube(lo: f64, hi: f64) -> Vec<[Vec3d; 3]> {
        let c = [
            Vec3d::new(lo, lo, lo),
            Vec3d::new(hi, lo, lo),
            Vec3d::new(hi, hi, lo),
            Vec3d::new(lo, hi, lo),
            Vec3d::new(lo, lo, hi),
            Vec3d::new(hi, lo, hi),
            Vec3d::new(hi, hi, hi),
            Vec3d::new(lo, hi, hi),
        ];
        let quad = |a: usize, b: usize, d: usize, e: usize| {
            [[c[a], c[b], c[d]], [c[a], c[d], c[e]]]
        };
        let mut out = Vec::new();
        for face in [
            quad(0, 3, 2, 1),
            quad(4, 5, 6, 7),
            quad(0, 1, 5, 4),
            quad(1, 2, 6, 5),
            quad(2, 3, 7, 6),
            quad(3, 0, 4, 7),
        ] {
            out.extend_from_slice(&face);
        }
        out
    }

    #[test]
    fn the_shell_is_on_the_inside_of_the_model() {
        // The whole construction depends on knowing which way is in. A shell built
        // outwards looks the same from a distance and is wrong about everything else:
        // the model grows, the wall thickness cannot be clamped against the far side,
        // and every measurement of overlap is measuring the wrong thing.
        let options = Options {
            size: None,
            thickness: 4.0,
            ..Options::default()
        };
        let (brushes, report) =
            brushes_from_triangles(&closed_cube(0.0, 64.0), &world(), &options);
        assert!(report.brushes > 0, "{report:?}");

        let whole = brushes.iter().fold(brushes[0].bounds(), |all, b| all.merged(&b.bounds()));
        // Every brush is within the cube, give or take the rounding of a corner.
        assert!(whole.min.x() > -0.01, "it grew outwards: {whole:?}");
        assert!(whole.max.x() < 64.01, "it grew outwards: {whole:?}");
    }

    #[test]
    fn a_wall_is_never_thicker_than_the_model_has_room_for() {
        // A cube 8 across, asked for a wall of 10: the two sides would pass through each
        // other. Each corner asks how far it is to the far side and takes less than half.
        let options = Options {
            size: None,
            thickness: 10.0,
            ..Options::default()
        };
        let (brushes, _) = brushes_from_triangles(&closed_cube(0.0, 8.0), &world(), &options);
        let whole = brushes.iter().fold(brushes[0].bounds(), |all, b| all.merged(&b.bounds()));
        assert!(whole.min.x() > -0.01 && whole.max.x() < 8.01, "{whole:?}");
        // A wedge of a whole face of the cube is as wide as the face, so two of its three
        // extents are the face's own. What the clamp governs is the third: how deep the
        // wall goes, which must leave room for the wall coming the other way.
        for brush in &brushes {
            let size = brush.bounds().size();
            let deepest = size.x().min(size.y()).min(size.z());
            // Half the cube: as far as a wall may go before it is where the wall from
            // the other side would be.
            assert!(deepest <= 4.0 + 1e-6, "a wall past the middle: {:?}", brush.bounds());
        }
    }

    /// Two triangles sharing an edge, folded at an angle: the smallest case of the thing
    /// the whole construction promises.
    fn folded_pair(fold: f64) -> Vec<[Vec3d; 3]> {
        let a = Vec3d::new(0.0, -32.0, 0.0);
        let b = Vec3d::new(0.0, 32.0, 0.0);
        let left = Vec3d::new(-32.0, 0.0, 0.0);
        let right = Vec3d::new(32.0 * fold.cos(), 0.0, 32.0 * fold.sin());
        vec![[a, left, b], [a, b, right]]
    }

    #[test]
    fn two_triangles_sharing_an_edge_share_a_face_and_do_not_overlap() {
        for fold in [0.2, 0.6, 1.0, 1.6] {
            let options = Options {
                size: None,
                thickness: 4.0,
                ..Options::default()
            };
            let (brushes, _) = brushes_from_triangles(&folded_pair(fold), &world(), &options);
            assert_eq!(brushes.len(), 2, "fold {fold}");

            // Sharing a face: each has a face whose plane is the other's, the other way
            // round. That is what "the brushes join" means -- the two are back to back
            // along one plane rather than merely near each other.
            let shared = brushes[0].faces().iter().any(|a| {
                brushes[1].faces().iter().any(|b| {
                    let opposite = geometry_math::vec::dot(a.plane.normal, b.plane.normal) < -0.999;
                    opposite && (a.plane.distance + b.plane.distance).abs() < 1e-6
                })
            });
            assert!(shared, "fold {fold}: the two do not share a face");

            // And not overlapping: every corner of one is on or outside that plane.
            let corners: Vec<Vec3d> = brushes[1]
                .geometry()
                .vertices()
                .iter()
                .map(|v| v.position)
                .collect();
            let apart = brushes[0].faces().iter().any(|face| {
                corners
                    .iter()
                    .all(|corner| face.plane.point_distance(*corner) > -1e-6)
            });
            assert!(apart, "fold {fold}: the two share space");
        }
    }

    #[test]
    fn two_wedges_of_one_flat_face_become_one_brush() {
        // Two triangles of a square: their wedges are two halves of one slab, and the
        // hull of the two is that slab and nothing else.
        let options = Options {
            size: None,
            thickness: 2.0,
            join: true,
            ..Options::default()
        };
        let (brushes, report) = brushes_from_triangles(&flat_square(), &world(), &options);
        assert_eq!(brushes.len(), 1, "the square is one brush");
        assert_eq!(report.joined, 1);
    }

    #[test]
    fn wedges_that_fold_are_left_alone() {
        // Folded, the two make an L. Its hull is the corner filled in, which is space
        // neither brush had, so the join is refused.
        let options = Options {
            size: None,
            thickness: 2.0,
            join: true,
            ..Options::default()
        };
        let (brushes, report) = brushes_from_triangles(&folded_pair(0.9), &world(), &options);
        assert_eq!(brushes.len(), 2, "still two");
        assert_eq!(report.joined, 0);
    }

    #[test]
    fn the_faces_inside_the_model_are_named_apart_from_the_surface() {
        let options = Options {
            size: None,
            thickness: 2.0,
            material: "rock".to_string(),
            inside_material: "skip".to_string(),
            ..Options::default()
        };
        let (brushes, _) = brushes_from_triangles(&folded_pair(0.9), &world(), &options);
        for brush in &brushes {
            let outside = brush
                .faces()
                .iter()
                .filter(|face| face.material_name.as_str() == "rock")
                .count();
            assert_eq!(outside, 1, "one face of each is the model's own surface");
            assert!(
                brush.faces().iter().any(|f| f.material_name.as_str() == "skip"),
                "the rest are inside it"
            );
        }
    }
}
