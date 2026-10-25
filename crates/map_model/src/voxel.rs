//! Voxel terrain: a grid of density samples whose zero level is a surface. Unlike brush
//! terrain it can hang over and hollow out, and it is sculpted freely, then turned into
//! triangles by surface nets for drawing and export.
//!
//! A sample is positive inside the ground and negative in the air, measured in grid
//! cells and held within [`RANGE`] of zero, so it is about the distance to the surface
//! near it and flat further away. Everything outside the grid is air, so the surface is
//! always closed, even where the ground reaches the grid's edge.
use geometry_math::bbox::Bbox3d;
use geometry_math::vec;
use geometry_math::vec::Vec3d;

/// How far from zero a sample goes, in cells: the depth over which a dab can still change
/// the surface.
pub const RANGE: f32 = 3.0;

/// The most samples one field holds, 256 × 256 × 64: about 8 MB in memory.
pub const MAX_SAMPLES: usize = 256 * 256 * 64;

/// `p` on the eighth-unit lattice brushes are written on. A brush whose corners all lie
/// on it is read back as the hull of them, however alike its faces; off it, faces of a
/// gently curved surface can be too near alike to build back from their planes. Both
/// cells round an edge round its crossing alike.
fn on_lattice(p: Vec3d) -> Vec3d {
    let round = |v: f64| (v * 8.0).round() / 8.0;
    Vec3d::new(round(p.x()), round(p.y()), round(p.z()))
}

/// How much of `brush`'s surface lies on side `side` of the box `bounds`: 0 and 1 are its
/// low and high x, 2 and 3 y, 4 and 5 z.
fn area_on_side(brush: &crate::brush::Brush, bounds: &Bbox3d, side: usize) -> f64 {
    let axis = side / 2;
    let at = if side.is_multiple_of(2) {
        bounds.min[axis]
    } else {
        bounds.max[axis]
    };
    (0..brush.faces().len())
        .filter_map(|index| brush.face_polygon(index))
        .filter(|corners| corners.iter().all(|p| (p[axis] - at).abs() < 1e-9))
        .map(|corners| {
            (1..corners.len() - 1)
                .map(|n| {
                    vec::length(vec::cross(
                        corners[n] - corners[0],
                        corners[n + 1] - corners[0],
                    )) / 2.0
                })
                .sum::<f64>()
        })
        .sum()
}

/// How near zero, in cells, a sample is taken as on the surface. A crossing near a corner
/// makes slivers whose faces a map file cannot carry back, and snapping each edge's
/// crossing on its own tilts the pieces of a cell apart, leaving thin steps and spikes in
/// the planes between cells. A sample on the surface puts every crossing beside it on its
/// corner at once, so the pieces stay whole and the cells round it agree.
const SNAP: f64 = 0.2;

/// How much ground, as a share of a cell, the hull of a cell's pieces may add for it to
/// stand in for them.
const HULL_SLACK: f64 = 0.02;

/// The start of a field's file, and the version of its layout.
const MAGIC: &[u8; 4] = b"HVOX";
const VERSION: u32 = 1;

/// A grid of density samples, `cell` units apart, from `origin` on.
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelField {
    origin: Vec3d,
    cell: f64,
    dims: [usize; 3],
    density: Vec<f32>,
}

/// What a dab does to a field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoxelTool {
    /// Moves the surface `along`, adding ground: up out of a floor, out of a wall. The
    /// ball's half behind a plane a little in front of its centre becomes ground, so held
    /// on the surface it grows a column as wide as the ball.
    Add {
        /// The way the surface moves.
        along: Vec3d,
    },
    /// Moves the surface against `along`, taking ground away: down into a floor, into a
    /// wall. The ball's half in front of a plane a little behind its centre becomes air,
    /// so held on the surface it digs a tunnel as wide as the ball.
    Remove {
        /// The way out of the ground; the surface moves the other way.
        along: Vec3d,
    },
    /// Evens out bumps and dents.
    Smooth,
    /// Moves the surface towards a plane through `at` facing `normal`: ground below it,
    /// air above.
    Flatten {
        /// A point on the plane.
        at: Vec3d,
        /// The way out of the ground.
        normal: Vec3d,
    },
}

/// A triangle mesh: positions, a normal for each, and three indices a triangle, wound
/// counter-clockwise seen from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoxelMesh {
    /// Where each vertex is.
    pub positions: Vec<Vec3d>,
    /// Each vertex's normal, pointing out of the ground.
    pub normals: Vec<Vec3d>,
    /// Three vertices a triangle.
    pub indices: Vec<u32>,
}

/// Whether a sample is ground. Zero counts: a sample on a face of the ground, as grid
/// aligned terrain has many of, then belongs to it, and so does the cell between such
/// samples.
fn solid(sample: f32) -> bool {
    sample >= 0.0
}

/// How much a dab does at `distance` from its centre, from 1 there to 0 at `radius`.
fn falloff(distance: f64, radius: f64) -> f64 {
    if radius <= 0.0 || distance >= radius {
        return 0.0;
    }
    let t = 1.0 - (distance / radius).powi(2);
    t * t
}

impl VoxelField {
    /// A field over `bounds`, `cell` units a step, whose ground is the box itself: solid
    /// inside, air outside. The grid reaches a cell past the box on every side, so the
    /// box's faces lie between samples. Fails when the cell is not positive or the grid
    /// would hold more than [`MAX_SAMPLES`].
    pub fn solid_box(bounds: &Bbox3d, cell: f64) -> Result<Self, &'static str> {
        let mut field = Self::empty(bounds, cell)?;
        for k in 0..field.dims[2] {
            for j in 0..field.dims[1] {
                for i in 0..field.dims[0] {
                    let p = field.position(i, j, k);
                    // Inside the box, the distance to its nearest face; outside, minus the
                    // distance to the box along the farthest axis it lies past.
                    let inside = (0..3)
                        .map(|a| (p[a] - bounds.min[a]).min(bounds.max[a] - p[a]))
                        .fold(f64::INFINITY, f64::min);
                    let d = (inside / cell) as f32;
                    let at = field.index(i, j, k);
                    field.density[at] = d.clamp(-RANGE, RANGE);
                }
            }
        }
        Ok(field)
    }

    /// A field over `bounds` with nothing in it.
    fn empty(bounds: &Bbox3d, cell: f64) -> Result<Self, &'static str> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err("Set a grid size first.");
        }
        let origin = bounds.min - Vec3d::new(cell, cell, cell);
        let dims = [0, 1, 2].map(|a| ((bounds.max[a] - bounds.min[a]) / cell).ceil() as usize + 3);
        let count = dims.iter().try_fold(1usize, |n, d| n.checked_mul(*d));
        match count {
            Some(n) if n <= MAX_SAMPLES => Ok(Self {
                origin,
                cell,
                dims,
                density: vec![-RANGE; n],
            }),
            _ => Err("That makes too many voxels. Use a bigger grid size."),
        }
    }

    /// A field of `dims` samples `cell` apart from `origin`, each the value `density`
    /// gives at its position, held within [`RANGE`]. Fails as [`Self::solid_box`] does.
    pub fn sampled(
        origin: Vec3d,
        cell: f64,
        dims: [usize; 3],
        density: impl Fn(Vec3d) -> f32,
    ) -> Result<Self, &'static str> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err("Set a grid size first.");
        }
        let count = dims
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul(*d))
            .filter(|n| *n <= MAX_SAMPLES && dims.iter().all(|d| *d >= 2))
            .ok_or("That makes too many voxels. Use a bigger grid size.")?;
        let mut field = Self {
            origin,
            cell,
            dims,
            density: vec![-RANGE; count],
        };
        for k in 0..dims[2] {
            for j in 0..dims[1] {
                for i in 0..dims[0] {
                    let at = field.index(i, j, k);
                    field.density[at] = density(field.position(i, j, k)).clamp(-RANGE, RANGE);
                }
            }
        }
        Ok(field)
    }

    /// The ground inside the grid as convex brushes wearing `material`. A cell that is
    /// ground all through is one box. A cell the surface passes through is cut into six
    /// tetrahedra round the same diagonal in every cell, so neighbouring cells meet face to
    /// face; within a tetrahedron the surface is flat, so the ground in it is convex. The
    /// pieces are many and small; merging neighbours afterwards makes them fewer.
    pub fn to_brushes(&self, material: &str) -> Vec<crate::brush::Brush> {
        self.to_brushes_where(material, |_| true)
    }

    /// The cells whose corners differ between this field and `before`, a field over the
    /// same grid: where a dab changed something.
    pub fn changed_cells(&self, before: &Self) -> Vec<[usize; 3]> {
        let mut cells = Vec::new();
        for k in 0..self.dims[2] - 1 {
            for j in 0..self.dims[1] - 1 {
                for i in 0..self.dims[0] - 1 {
                    let differs = (0..8).any(|n| {
                        let at = self.index(i + (n & 1), j + ((n >> 1) & 1), k + (n >> 2));
                        self.density[at] != before.density[at]
                    });
                    if differs {
                        cells.push([i, j, k]);
                    }
                }
            }
        }
        cells
    }

    /// How many of a cell's eight corners are ground: 8 where the cell is ground all
    /// through, 0 where it is air.
    pub fn ground_corners(&self, [i, j, k]: [usize; 3]) -> usize {
        (0..8)
            .filter(|n| {
                solid(self.density[self.index(i + (n & 1), j + ((n >> 1) & 1), k + (n >> 2))])
            })
            .count()
    }

    /// The box a cell of the grid fills.
    pub fn cell_bounds(&self, [i, j, k]: [usize; 3]) -> Bbox3d {
        Bbox3d::new(self.position(i, j, k), self.position(i + 1, j + 1, k + 1))
    }

    /// `cells` packed into few boxes: each grows along x as far as the cells go, then
    /// along y while every row is there, then along z while every slab is.
    pub fn boxes_of(&self, cells: &std::collections::HashSet<[usize; 3]>) -> Vec<Bbox3d> {
        let mut sorted: Vec<[usize; 3]> = cells.iter().copied().collect();
        sorted.sort_by_key(|[i, j, k]| (*k, *j, *i));
        let mut taken = std::collections::HashSet::new();
        let mut boxes = Vec::new();
        for start in sorted {
            if taken.contains(&start) {
                continue;
            }
            let free = |c: [usize; 3]| cells.contains(&c) && !taken.contains(&c);
            let [i0, j0, k0] = start;
            let mut i1 = i0;
            while free([i1 + 1, j0, k0]) {
                i1 += 1;
            }
            let mut j1 = j0;
            while (i0..=i1).all(|i| free([i, j1 + 1, k0])) {
                j1 += 1;
            }
            let mut k1 = k0;
            while (i0..=i1).all(|i| (j0..=j1).all(|j| free([i, j, k1 + 1]))) {
                k1 += 1;
            }
            for k in k0..=k1 {
                for j in j0..=j1 {
                    for i in i0..=i1 {
                        taken.insert([i, j, k]);
                    }
                }
            }
            boxes.push(Bbox3d::new(
                self.position(i0, j0, k0),
                self.position(i1 + 1, j1 + 1, k1 + 1),
            ));
        }
        boxes
    }

    /// [`Self::to_brushes`] for the cells `wanted` accepts alone.
    pub fn to_brushes_where(
        &self,
        material: &str,
        wanted: impl Fn([usize; 3]) -> bool,
    ) -> Vec<crate::brush::Brush> {
        self.to_brushes_meeting(material, wanted, |_, _| None)
    }

    /// [`Self::to_brushes_where`], with the surface crossing the edge between two samples,
    /// by their places in the grid, where `crossing` says when it says. A cell built again
    /// beside one that is kept meets it only if both put the surface on their shared
    /// edges alike; the kept cell's brushes say where, its samples may no longer.
    pub fn to_brushes_meeting(
        &self,
        material: &str,
        wanted: impl Fn([usize; 3]) -> bool,
        crossing: impl Fn([usize; 3], [usize; 3]) -> Option<Vec3d>,
    ) -> Vec<crate::brush::Brush> {
        use crate::brush::Brush;
        use crate::polyhedron::Polyhedron;
        // Corner `n` of a cell is offset by its bits: 1 along x, 2 along y, 4 along z.
        let offset = |n: usize| Vec3d::new((n & 1) as f64, ((n >> 1) & 1) as f64, (n >> 2) as f64);
        // The six tetrahedra round the diagonal from corner 0 to corner 7, one for each
        // order the three axes can be walked in.
        const TETRAHEDRA: [[usize; 4]; 6] = [
            [0, 1, 3, 7],
            [0, 1, 5, 7],
            [0, 2, 3, 7],
            [0, 2, 6, 7],
            [0, 4, 5, 7],
            [0, 4, 6, 7],
        ];
        let smallest = self.cell.powi(3) * 1e-4;
        let mut brushes = Vec::new();
        let make = |points: &[Vec3d]| -> Option<Brush> {
            // A sample of exactly zero puts a crossing on a corner already in the list,
            // and the hull refuses a point twice.
            let mut unique: Vec<Vec3d> = Vec::with_capacity(points.len());
            for p in points {
                if !unique.iter().any(|q| vec::length(*q - *p) < 1e-9) {
                    unique.push(*p);
                }
            }
            if unique.len() < 4 {
                return None;
            }
            let geometry = Polyhedron::from_points(&unique, 1e-9).ok()?;
            let brush = Brush::from_polyhedron(geometry, material);
            (brush.volume() > smallest).then_some(brush)
        };
        for k in 0..self.dims[2] - 1 {
            for j in 0..self.dims[1] - 1 {
                for i in 0..self.dims[0] - 1 {
                    if !wanted([i, j, k]) {
                        continue;
                    }
                    let d: [f32; 8] = std::array::from_fn(|n| {
                        let v =
                            self.density[self.index(i + (n & 1), j + ((n >> 1) & 1), k + (n >> 2))];
                        if f64::from(v.abs()) < SNAP {
                            0.0
                        } else {
                            v
                        }
                    });
                    let ground = d.iter().filter(|v| solid(**v)).count();
                    if ground == 0 {
                        continue;
                    }
                    let corner =
                        |n: usize| on_lattice(self.position(i, j, k) + offset(n) * self.cell);
                    let place = |n: usize| [i + (n & 1), j + ((n >> 1) & 1), k + (n >> 2)];
                    if ground == 8 {
                        brushes.extend(make(&std::array::from_fn::<_, 8, _>(corner)));
                        continue;
                    }
                    // The cell's ground as one brush where the hull of its pieces adds
                    // next to nothing, as it does wherever the surface runs flat or bulges
                    // out through the cell; else the pieces themselves. The hull must cover
                    // just what the pieces do on each of the cell's faces, or it would stand
                    // out past the neighbour there as a thin step.
                    let first = brushes.len();
                    let mut all: Vec<Vec3d> = Vec::new();
                    for tetra in TETRAHEDRA {
                        let mut points = Vec::with_capacity(8);
                        for (x, &a) in tetra.iter().enumerate() {
                            if solid(d[a]) {
                                points.push(corner(a));
                            }
                            for &b in &tetra[x + 1..] {
                                if solid(d[a]) != solid(d[b]) {
                                    if let Some(at) = crossing(place(a), place(b)) {
                                        points.push(on_lattice(at));
                                        continue;
                                    }
                                    let t = f64::from(d[a] / (d[a] - d[b]));
                                    points
                                        .push(on_lattice(corner(a) + (corner(b) - corner(a)) * t));
                                }
                            }
                        }
                        if points.len() >= 4 {
                            all.extend_from_slice(&points);
                            brushes.extend(make(&points));
                        }
                    }
                    if brushes.len() - first > 1 {
                        let pieces: f64 = brushes[first..].iter().map(Brush::volume).sum();
                        if let Some(hull) = make(&all) {
                            let bounds = self.cell_bounds([i, j, k]);
                            if hull.volume() - pieces <= self.cell.powi(3) * HULL_SLACK
                                && (0..6).all(|side| {
                                    let pieces: f64 = brushes[first..]
                                        .iter()
                                        .map(|piece| area_on_side(piece, &bounds, side))
                                        .sum();
                                    (area_on_side(&hull, &bounds, side) - pieces).abs()
                                        <= self.cell * self.cell * 1e-6
                                })
                                && hull.survives_a_file()
                            {
                                brushes.truncate(first);
                                brushes.push(hull);
                            }
                        }
                    }
                }
            }
        }
        brushes
    }

    /// Where the grid starts: its first sample.
    pub fn origin(&self) -> Vec3d {
        self.origin
    }

    /// How far apart the samples are.
    pub fn cell(&self) -> f64 {
        self.cell
    }

    /// How many samples the grid has along each axis.
    pub fn dims(&self) -> [usize; 3] {
        self.dims
    }

    /// The box the grid covers.
    pub fn bounds(&self) -> Bbox3d {
        let last = self.position(self.dims[0] - 1, self.dims[1] - 1, self.dims[2] - 1);
        Bbox3d::new(self.origin, last)
    }

    fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.dims[1] + j) * self.dims[0] + i
    }

    fn position(&self, i: usize, j: usize, k: usize) -> Vec3d {
        self.origin + Vec3d::new(i as f64, j as f64, k as f64) * self.cell
    }

    /// The sample at a grid point, which may lie outside the grid: there it is air.
    fn sample(&self, i: i64, j: i64, k: i64) -> f32 {
        let inside = |v: i64, n: usize| v >= 0 && (v as usize) < n;
        if inside(i, self.dims[0]) && inside(j, self.dims[1]) && inside(k, self.dims[2]) {
            self.density[self.index(i as usize, j as usize, k as usize)]
        } else {
            -RANGE
        }
    }

    /// The sample at grid point `[i, j, k]`.
    pub fn value(&self, [i, j, k]: [usize; 3]) -> f32 {
        self.density[self.index(i, j, k)]
    }

    /// Whether the point `p` is inside the ground, by the nearest sample.
    pub fn is_solid(&self, p: Vec3d) -> bool {
        let at = (p - self.origin) / self.cell;
        solid(self.sample(
            at.x().round() as i64,
            at.y().round() as i64,
            at.z().round() as i64,
        ))
    }

    /// One dab of `tool` at `centre`, reaching `radius` units, with `strength` from 0 to 1.
    /// Add and Remove move the surface at the centre by a tenth of the radius at full
    /// strength; Smooth and Flatten go all the way there, most at the centre. Returns the samples it changed as a box, or
    /// `None` when it changed nothing.
    pub fn sculpt(
        &mut self,
        centre: Vec3d,
        radius: f64,
        strength: f64,
        tool: VoxelTool,
    ) -> Option<Bbox3d> {
        let strength = strength.clamp(0.0, 1.0);
        let lo = (centre - Vec3d::new(radius, radius, radius) - self.origin) / self.cell;
        let hi = (centre + Vec3d::new(radius, radius, radius) - self.origin) / self.cell;
        let range = |a: usize| {
            let first = lo[a].floor().max(0.0) as usize;
            let last = (hi[a].ceil().max(0.0) as usize).min(self.dims[a] - 1);
            first..=last
        };
        let (ri, rj, rk) = (range(0), range(1), range(2));
        if ri.is_empty() || rj.is_empty() || rk.is_empty() {
            return None;
        }
        // Smooth reads the samples as they were before the dab.
        let before = matches!(tool, VoxelTool::Smooth).then(|| self.density.clone());
        let shift = radius * 0.1 * strength;
        let mut changed: Option<Bbox3d> = None;
        for k in rk.clone() {
            for j in rj.clone() {
                for i in ri.clone() {
                    let p = self.position(i, j, k);
                    let distance = vec::length(p - centre);
                    let weight = match tool {
                        VoxelTool::Add { .. } | VoxelTool::Remove { .. } => {
                            if distance < radius {
                                1.0
                            } else {
                                0.0
                            }
                        }
                        _ => falloff(distance, radius) as f32,
                    };
                    if weight <= 0.0 {
                        continue;
                    }
                    let at = self.index(i, j, k);
                    let d = self.density[at];
                    let next = match tool {
                        // Depths to the half ball, measured exactly rather than read off the
                        // samples round about, which on a coarse grid would blur the move
                        // away dab after dab.
                        VoxelTool::Add { along } => {
                            let along = vec::normalize(along);
                            let behind = vec::dot(centre + along * shift - p, along);
                            let inside = behind.min(radius - distance) / self.cell;
                            d.max(inside as f32)
                        }
                        VoxelTool::Remove { along } => {
                            let along = vec::normalize(along);
                            let ahead = vec::dot(p - (centre - along * shift), along);
                            let inside = ahead.min(radius - distance) / self.cell;
                            d.min(-inside as f32)
                        }
                        // Only the samples within a cell of the surface place it.
                        // Averaging those further in would wear away the ridge inside a
                        // thin wall, and with it the wall, dab after dab.
                        VoxelTool::Smooth if d.abs() >= 1.0 => d,
                        VoxelTool::Smooth => {
                            let old = before.as_ref().expect("kept for Smooth");
                            let read = |di: i64, dj: i64, dk: i64| {
                                let (ni, nj, nk) = (i as i64 + di, j as i64 + dj, k as i64 + dk);
                                let inside = |v: i64, n: usize| v >= 0 && (v as usize) < n;
                                if inside(ni, self.dims[0])
                                    && inside(nj, self.dims[1])
                                    && inside(nk, self.dims[2])
                                {
                                    old[self.index(ni as usize, nj as usize, nk as usize)]
                                } else {
                                    -RANGE
                                }
                            };
                            let average = (read(-1, 0, 0)
                                + read(1, 0, 0)
                                + read(0, -1, 0)
                                + read(0, 1, 0)
                                + read(0, 0, -1)
                                + read(0, 0, 1))
                                / 6.0;
                            d + (average - d) * strength as f32 * weight
                        }
                        VoxelTool::Flatten { at: level, normal } => {
                            let normal = vec::normalize(normal);
                            let target = (vec::dot(level - p, normal) / self.cell) as f32;
                            d + (target.clamp(-RANGE, RANGE) - d) * strength as f32 * weight
                        }
                    }
                    .clamp(-RANGE, RANGE);
                    if (next - d).abs() > f32::EPSILON {
                        self.density[at] = next;
                        changed = Some(match changed {
                            Some(b) => b.merged_with_point(p),
                            None => Bbox3d::new(p, p),
                        });
                    }
                }
            }
        }
        changed
    }

    /// The surface as triangles, by surface nets: a vertex in each cell the surface
    /// passes through, at the average of where it crosses the cell's edges, and a quad
    /// across each grid edge it crosses, joining the four cells round that edge.
    pub fn mesh(&self) -> VoxelMesh {
        let [nx, ny, nz] = self.dims.map(|n| n as i64);
        // Cells run from -1 so the air past the grid's edge closes the surface.
        let cells = [nx + 1, ny + 1, nz + 1];
        let cell_index =
            |c: [i64; 3]| (((c[2] + 1) * cells[1] + (c[1] + 1)) * cells[0] + (c[0] + 1)) as usize;
        let mut vertex_of = vec![u32::MAX; (cells[0] * cells[1] * cells[2]) as usize];
        let mut mesh = VoxelMesh::default();
        const CORNERS: [[i64; 3]; 8] = [
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ];
        const EDGES: [(usize, usize); 12] = [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        for ck in -1..nz {
            for cj in -1..ny {
                for ci in -1..nx {
                    let d: [f32; 8] = CORNERS.map(|[a, b, c]| self.sample(ci + a, cj + b, ck + c));
                    let ground = d.iter().filter(|v| solid(**v)).count();
                    if ground == 0 || ground == 8 {
                        continue;
                    }
                    let mut sum = Vec3d::new(0.0, 0.0, 0.0);
                    let mut crossings = 0.0;
                    for (a, b) in EDGES {
                        if solid(d[a]) == solid(d[b]) {
                            continue;
                        }
                        let t = f64::from(d[a] / (d[a] - d[b]));
                        let pa = CORNERS[a].map(|v| v as f64);
                        let pb = CORNERS[b].map(|v| v as f64);
                        sum += Vec3d::new(
                            pa[0] + (pb[0] - pa[0]) * t,
                            pa[1] + (pb[1] - pa[1]) * t,
                            pa[2] + (pb[2] - pa[2]) * t,
                        );
                        crossings += 1.0;
                    }
                    let local = sum / crossings;
                    let position = self.origin
                        + (Vec3d::new(ci as f64, cj as f64, ck as f64) + local) * self.cell;
                    // The density falls outwards, so the normal is against its gradient.
                    let gradient = |axis: usize| {
                        let mut g = 0.0;
                        for (n, corner) in CORNERS.iter().enumerate() {
                            g += if corner[axis] == 1 { d[n] } else { -d[n] };
                        }
                        f64::from(g) / 4.0
                    };
                    let normal = -Vec3d::new(gradient(0), gradient(1), gradient(2));
                    let normal = if vec::length(normal) > 1e-12 {
                        vec::normalize(normal)
                    } else {
                        Vec3d::new(0.0, 0.0, 1.0)
                    };
                    vertex_of[cell_index([ci, cj, ck])] = mesh.positions.len() as u32;
                    mesh.positions.push(position);
                    mesh.normals.push(normal);
                }
            }
        }
        // A quad for each grid edge with ground at one end and air at the other, through
        // the four cells round it, facing from the ground to the air.
        for k in -1..=nz {
            for j in -1..=ny {
                for i in -1..=nx {
                    let here = solid(self.sample(i, j, k));
                    for a in 0..3 {
                        let (b, c) = ((a + 1) % 3, (a + 2) % 3);
                        let mut next = [i, j, k];
                        next[a] += 1;
                        if next[a] > [nx, ny, nz][a] {
                            continue;
                        }
                        if here == solid(self.sample(next[0], next[1], next[2])) {
                            continue;
                        }
                        let around = |ob: i64, oc: i64| {
                            let mut cell = [i, j, k];
                            cell[b] -= 1 - ob;
                            cell[c] -= 1 - oc;
                            cell
                        };
                        let quad = [around(0, 0), around(1, 0), around(1, 1), around(0, 1)];
                        if quad
                            .iter()
                            .any(|q| (0..3).any(|x| q[x] < -1 || q[x] >= cells[x] - 1))
                        {
                            continue;
                        }
                        let v = quad.map(|q| vertex_of[cell_index(q)]);
                        if v.contains(&u32::MAX) {
                            continue;
                        }
                        // Counter-clockwise round +a when the ground is behind.
                        let [v0, v1, v2, v3] = if here { v } else { [v[0], v[3], v[2], v[1]] };
                        mesh.indices.extend([v0, v1, v2, v0, v2, v3]);
                    }
                }
            }
        }
        mesh
    }

    /// The field as bytes for its file: a short header, then the samples, each rounded to
    /// 16 bits, compressed.
    pub fn to_bytes(&self) -> Vec<u8> {
        use std::io::Write;
        let mut samples = Vec::with_capacity(self.density.len() * 2);
        for d in &self.density {
            let q = (d / RANGE * f32::from(i16::MAX)).round() as i16;
            samples.extend_from_slice(&q.to_le_bytes());
        }
        let mut packed =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        packed.write_all(&samples).expect("writing to memory");
        let packed = packed.finish().expect("writing to memory");
        let mut out = Vec::with_capacity(64 + packed.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        for v in [self.origin.x(), self.origin.y(), self.origin.z(), self.cell] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for n in self.dims {
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
        out.extend_from_slice(&crc32fast::hash(&samples).to_le_bytes());
        out.extend_from_slice(&packed);
        out
    }

    /// A field read back from [`Self::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        use std::io::Read;
        const HEADER: usize = 4 + 4 + 8 * 4 + 4 * 3 + 4;
        let broken = "The voxel terrain file is damaged.";
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err(broken);
        }
        let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
        let f64_at = |at: usize| f64::from_le_bytes(bytes[at..at + 8].try_into().expect("8 bytes"));
        if u32_at(4) != VERSION {
            return Err("The voxel terrain file is from a newer version of Heron Brush.");
        }
        let origin = Vec3d::new(f64_at(8), f64_at(16), f64_at(24));
        let cell = f64_at(32);
        let dims = [u32_at(40), u32_at(44), u32_at(48)].map(|n| n as usize);
        let count = dims
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul(*d))
            .filter(|n| *n <= MAX_SAMPLES && *n > 0)
            .ok_or(broken)?;
        if !(cell.is_finite() && cell > 0.0) {
            return Err(broken);
        }
        let mut samples = Vec::with_capacity(count * 2);
        flate2::read::ZlibDecoder::new(&bytes[HEADER..])
            .take(count as u64 * 2 + 1)
            .read_to_end(&mut samples)
            .map_err(|_| broken)?;
        if samples.len() != count * 2 || crc32fast::hash(&samples) != u32_at(52) {
            return Err(broken);
        }
        let density = samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|q| f32::from(i16::from_le_bytes(*q)) / f32::from(i16::MAX) * RANGE)
            .collect();
        Ok(Self {
            origin,
            cell,
            dims,
            density,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A field holding a ball of `radius` round `centre`.
    fn ball(centre: Vec3d, radius: f64, cell: f64) -> VoxelField {
        let r = Vec3d::new(radius, radius, radius);
        let mut field = VoxelField::empty(&Bbox3d::new(centre - r, centre + r), cell).unwrap();
        for k in 0..field.dims[2] {
            for j in 0..field.dims[1] {
                for i in 0..field.dims[0] {
                    let d = (radius - vec::length(field.position(i, j, k) - centre)) / cell;
                    let at = field.index(i, j, k);
                    field.density[at] = (d as f32).clamp(-RANGE, RANGE);
                }
            }
        }
        field
    }

    /// Whether every edge of the mesh is shared by exactly two triangles, once each way:
    /// a closed surface wound consistently.
    fn closed(mesh: &VoxelMesh) -> bool {
        let mut edges: HashMap<(u32, u32), i32> = HashMap::new();
        for t in mesh.indices.as_chunks::<3>().0 {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        edges
            .iter()
            .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
    }

    fn top_at(field: &VoxelField, x: f64, y: f64) -> f64 {
        let mesh = field.mesh();
        mesh.positions
            .iter()
            .filter(|p| (p.x() - x).abs() <= field.cell && (p.y() - y).abs() <= field.cell)
            .map(|p| p.z())
            .fold(f64::NEG_INFINITY, f64::max)
    }

    #[test]
    fn a_ball_meshes_closed_round_and_facing_out() {
        let centre = Vec3d::new(10.0, -20.0, 30.0);
        let field = ball(centre, 64.0, 8.0);
        let mesh = field.mesh();
        assert!(!mesh.indices.is_empty());
        assert!(closed(&mesh));
        for p in &mesh.positions {
            assert!((vec::length(*p - centre) - 64.0).abs() < 8.0 * 0.5, "{p:?}");
        }
        for (p, n) in mesh.positions.iter().zip(&mesh.normals) {
            assert!(vec::dot(*n, *p - centre) > 0.0);
        }
        for t in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.positions[t[i] as usize]);
            let facing = vec::cross(b - a, c - a);
            // A sample exactly on the sphere makes a triangle with no area, and no side.
            if vec::length(facing) < 1e-9 {
                continue;
            }
            assert!(
                vec::dot(facing, (a + b + c) / 3.0 - centre) > 0.0,
                "wound inwards"
            );
        }
    }

    #[test]
    fn a_box_reaching_the_grids_edge_still_closes() {
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(128.0, 64.0, 32.0));
        let mut field = VoxelField::solid_box(&bounds, 8.0).unwrap();
        // Fill the whole grid: the surface then lies past the last samples.
        field.density.iter_mut().for_each(|d| *d = RANGE);
        let mesh = field.mesh();
        assert!(closed(&mesh));
        let solid = VoxelField::solid_box(&bounds, 8.0).unwrap();
        assert!(closed(&solid.mesh()));
        assert!((top_at(&solid, 64.0, 32.0) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn add_and_remove_move_the_surface_and_flatten_and_smooth_level_it() {
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(256.0, 256.0, 64.0));
        // Room above the ground for it to grow into.
        let roomy = Bbox3d::new(bounds.min, bounds.max + Vec3d::new(0.0, 0.0, 128.0));
        let mut sky = VoxelField::empty(&roomy, 8.0).unwrap();
        for k in 0..sky.dims[2] {
            for j in 0..sky.dims[1] {
                for i in 0..sky.dims[0] {
                    let p = sky.position(i, j, k);
                    let at = sky.index(i, j, k);
                    sky.density[at] = (((32.0 - p.z()) / 8.0) as f32).clamp(-RANGE, RANGE);
                }
            }
        }
        let mut field = sky;
        let middle = Vec3d::new(128.0, 128.0, 32.0);
        let flat = top_at(&field, 128.0, 128.0);
        // Each dab where the last left the surface, as the editor aims them.
        for _ in 0..6 {
            let top = Vec3d::new(128.0, 128.0, top_at(&field, 128.0, 128.0));
            assert!(field
                .sculpt(
                    top,
                    64.0,
                    1.0,
                    VoxelTool::Add {
                        along: Vec3d::new(0.0, 0.0, 1.0)
                    }
                )
                .is_some());
        }
        let raised = top_at(&field, 128.0, 128.0);
        assert!(raised > flat + 8.0, "{flat} {raised}");
        assert!(
            (top_at(&field, 16.0, 16.0) - flat).abs() < 1e-6,
            "far away stays"
        );
        // The column's rim rounds off.
        let rim = top_at(&field, 184.0, 128.0);
        for _ in 0..6 {
            field.sculpt(Vec3d::new(184.0, 128.0, rim), 48.0, 1.0, VoxelTool::Smooth);
        }
        assert!(top_at(&field, 184.0, 128.0) < rim);
        for _ in 0..12 {
            field.sculpt(
                middle,
                96.0,
                1.0,
                VoxelTool::Flatten {
                    at: Vec3d::new(0.0, 0.0, 32.0),
                    normal: Vec3d::new(0.0, 0.0, 1.0),
                },
            );
        }
        assert!((top_at(&field, 128.0, 128.0) - flat).abs() < 2.0);
        for _ in 0..12 {
            let bottom = Vec3d::new(128.0, 128.0, top_at(&field, 128.0, 128.0));
            field.sculpt(
                bottom,
                64.0,
                1.0,
                VoxelTool::Remove {
                    along: Vec3d::new(0.0, 0.0, 1.0),
                },
            );
        }
        assert!(top_at(&field, 128.0, 128.0) < flat - 8.0);
        assert!(closed(&field.mesh()));
    }

    #[test]
    fn a_dab_can_hollow_out_a_cave() {
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(256.0, 256.0, 128.0));
        let mut field = VoxelField::solid_box(&bounds, 8.0).unwrap();
        // Dabs into the side of the box, each where the last left the wall, as a held
        // button does.
        let mut face = 0.0;
        for _ in 0..10 {
            let at = Vec3d::new(face, 128.0, 64.0);
            field.sculpt(
                at,
                48.0,
                1.0,
                VoxelTool::Remove {
                    along: Vec3d::new(-1.0, 0.0, 0.0),
                },
            );
            while face < 256.0 && !field.is_solid(Vec3d::new(face, 128.0, 64.0)) {
                face += 0.5;
            }
        }
        assert!(face > 48.0, "the cave reaches {face}");
        assert!(!field.is_solid(Vec3d::new(24.0, 128.0, 64.0)));
        assert!(
            field.is_solid(Vec3d::new(24.0, 128.0, 120.0)),
            "the roof holds"
        );
        assert!(closed(&field.mesh()));
    }

    #[test]
    fn a_field_survives_its_file() {
        let field = ball(Vec3d::new(0.0, 0.0, 0.0), 40.0, 4.0);
        let bytes = field.to_bytes();
        let back = VoxelField::from_bytes(&bytes).unwrap();
        assert_eq!(back.dims, field.dims);
        assert_eq!(back.origin, field.origin);
        assert!(back
            .density
            .iter()
            .zip(&field.density)
            .all(|(a, b)| (a - b).abs() < RANGE / 30000.0));
        let mut damaged = bytes.clone();
        let last = damaged.len() - 1;
        damaged[last] ^= 0xff;
        assert!(VoxelField::from_bytes(&damaged).is_err());
        assert!(VoxelField::from_bytes(b"HVOX").is_err());
    }

    /// The volume of the ground the field's brushes hold together.
    fn brush_volume(field: &VoxelField) -> f64 {
        field.to_brushes("rock").iter().map(|b| b.volume()).sum()
    }

    #[test]
    fn a_box_becomes_brushes_of_the_same_volume() {
        let bounds = Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(64.0, 48.0, 32.0));
        let field = VoxelField::solid_box(&bounds, 8.0).unwrap();
        let brushes = field.to_brushes("rock");
        assert!(!brushes.is_empty());
        let got = brush_volume(&field);
        assert!((got - 64.0 * 48.0 * 32.0).abs() < 1e-6, "{got}");
        // Every piece lies inside the box.
        for brush in &brushes {
            let b = brush.bounds();
            assert!(b.min.x() >= -1e-9 && b.max.z() <= 32.0 + 1e-9, "{b:?}");
        }
    }

    #[test]
    fn a_ball_becomes_convex_brushes_close_to_its_volume() {
        let field = ball(Vec3d::new(0.0, 0.0, 0.0), 64.0, 8.0);
        let exact = 4.0 / 3.0 * std::f64::consts::PI * 64f64.powi(3);
        let got = brush_volume(&field);
        assert!((got - exact).abs() / exact < 0.03, "{got} {exact}");
        // Pieces only touch: their volumes add up to the volume their union encloses,
        // which the mesh of the same field measures independently.
        let mesh = field.mesh();
        let enclosed: f64 = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
                vec::dot(a, vec::cross(b, c)) / 6.0
            })
            .sum();
        assert!((got - enclosed).abs() / exact < 0.03, "{got} {enclosed}");
    }

    #[test]
    fn a_ball_off_the_grid_becomes_brushes_a_map_file_carries_back() {
        // Off the grid's samples, so the surface crosses edges anywhere along them.
        let field = ball(Vec3d::new(3.3, -2.7, 1.9), 61.7, 16.0);
        let brushes = field.to_brushes("m");
        assert!(!brushes.is_empty());
        for brush in &brushes {
            assert!(brush.survives_a_file(), "{:?}", brush.bounds());
            for p in brush.vertex_positions() {
                for v in [p.x(), p.y(), p.z()] {
                    assert!(
                        (v * 8.0 - (v * 8.0).round()).abs() < 1e-9,
                        "{v} is off the lattice"
                    );
                }
            }
        }
    }

    #[test]
    fn a_grid_too_big_or_without_a_cell_is_refused() {
        let huge = Bbox3d::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(4096.0, 4096.0, 4096.0),
        );
        assert!(VoxelField::solid_box(&huge, 1.0).is_err());
        assert!(VoxelField::solid_box(&huge, 0.0).is_err());
    }
}
