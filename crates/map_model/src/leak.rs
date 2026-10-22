//! Whether a level is sealed, and where it is not.
//!
//! Checks whether the level encloses its entities without depending on an external map compiler.
//!
//! The answer is a flood fill over a voxel grid: the space around the level is cut into
//! cells, cells that a brush reaches into are marked solid, and the rest is flooded from
//! where a point entity sits. Reaching the edge of the grid means the inside and the
//! outside are joined, which is a leak, and the path the flood took is the line to draw.
//!
//! What this can and cannot see is worth being plain about, since a check that quietly
//! misses things is worse than none:
//!
//! - A wall thinner than a cell can be stepped over, so a leak can be missed. The cell
//!   size is the caller's to choose, and the smallest wall in the level is what it should
//!   be no larger than.
//! - A cell is marked solid if a brush reaches any part of it, which is deliberately
//!   generous: it is better to call a leaking level sealed than to send someone looking
//!   for a hole that is not there.
//! - The trace runs along cell centres, so it shows which way the leak goes rather than
//!   the exact hole.

use std::collections::VecDeque;

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::brush::Brush;

/// The smallest cell the check will use, in world units.
///
/// Below this the grid grows faster than the answer improves; a wall thinner than one
/// unit is not a wall.
pub const MIN_CELL_SIZE: f64 = 1.0;

/// How many cells the check will fill before giving up.
///
/// A level a hundred thousand units across at a cell size of eight would be a hundred
/// million cells, which is not a wait anyone wants for an answer. Giving up and saying so
/// is better than appearing to hang.
pub const DEFAULT_CELL_BUDGET: usize = 8_000_000;

/// What the check found.
#[derive(Debug, Clone, PartialEq)]
pub enum Leak {
    /// Nothing outside can be reached from inside: the level is sealed.
    Sealed,
    /// The inside and the outside are joined, along roughly this path.
    Leaked(Trace),
    /// There is no solid geometry, so there is nothing to be inside of.
    NothingToSeal,
    /// There is no point entity to check from.
    ///
    /// The check answers "can the outside be reached from where the player starts", so it
    /// needs somewhere to start.
    NoStartingPoint,
    /// Every starting point is inside solid geometry, so the flood has nowhere to go.
    ///
    /// That is its own mistake -- an entity buried in a wall -- rather than a leak.
    StartsInsideSolid,
    /// The level is too large to check at the cell size asked for.
    TooLarge {
        /// How many cells the grid would have needed.
        cells: usize,
        /// How many it was allowed.
        budget: usize,
    },
}

/// The path from inside the level out to the void.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// The points the path passes through, starting at the entity it was traced from.
    ///
    /// Only the points where the path turns are kept, so a straight run down a corridor
    /// is two points rather than one per cell.
    pub points: Vec<Vec3d>,
}

/// A grid of cells covering the level and a margin around it.
struct Grid {
    origin: Vec3d,
    cell_size: f64,
    counts: [usize; 3],
}

impl Grid {
    /// The number of cells in the grid.
    fn len(&self) -> usize {
        self.counts[0] * self.counts[1] * self.counts[2]
    }

    /// The index of the cell at the given coordinates.
    fn index(&self, at: [usize; 3]) -> usize {
        (at[2] * self.counts[1] + at[1]) * self.counts[0] + at[0]
    }

    /// The coordinates of the cell with the given index.
    fn coordinates(&self, index: usize) -> [usize; 3] {
        let x = index % self.counts[0];
        let y = (index / self.counts[0]) % self.counts[1];
        let z = index / (self.counts[0] * self.counts[1]);
        [x, y, z]
    }

    /// The middle of the cell at the given coordinates.
    fn center(&self, at: [usize; 3]) -> Vec3d {
        Vec3d::new(
            self.origin.x() + (at[0] as f64 + 0.5) * self.cell_size,
            self.origin.y() + (at[1] as f64 + 0.5) * self.cell_size,
            self.origin.z() + (at[2] as f64 + 0.5) * self.cell_size,
        )
    }

    /// The box the cell at the given coordinates fills.
    fn cell_bounds(&self, at: [usize; 3]) -> Bbox3d {
        let min = Vec3d::new(
            self.origin.x() + at[0] as f64 * self.cell_size,
            self.origin.y() + at[1] as f64 * self.cell_size,
            self.origin.z() + at[2] as f64 * self.cell_size,
        );
        Bbox3d::new(
            min,
            min + Vec3d::new(self.cell_size, self.cell_size, self.cell_size),
        )
    }

    /// The cell the given world point falls in, or nothing if it is outside the grid.
    fn cell_at(&self, point: Vec3d) -> Option<[usize; 3]> {
        let mut at = [0usize; 3];
        for axis in 0..3 {
            let offset = (point[axis] - self.origin[axis]) / self.cell_size;
            if offset < 0.0 {
                return None;
            }
            let index = offset as usize;
            if index >= self.counts[axis] {
                return None;
            }
            at[axis] = index;
        }
        Some(at)
    }

    /// Whether the cell is on the outside face of the grid.
    ///
    /// The grid is made with a margin of empty cells around the level, so reaching one of
    /// these means reaching the void.
    fn is_on_the_edge(&self, at: [usize; 3]) -> bool {
        (0..3).any(|axis| at[axis] == 0 || at[axis] + 1 == self.counts[axis])
    }
}

/// Whether a brush reaches any part of the given box.
///
/// Deliberately generous: it answers yes for a box that is merely near a corner of the
/// brush, since every plane it tests is one the box has a point behind. Marking a cell
/// solid that is not quite solid can only hide a leak, never invent one, and a check that
/// invents leaks is a check nobody trusts.
fn brush_reaches(brush: &Brush, cell: &Bbox3d) -> bool {
    brush.faces().iter().all(|face| {
        // The corner of the box furthest along the plane's normal. If even that one is
        // outside the plane, the whole box is.
        let corner = Vec3d::new(
            if face.plane.normal.x() > 0.0 {
                cell.min.x()
            } else {
                cell.max.x()
            },
            if face.plane.normal.y() > 0.0 {
                cell.min.y()
            } else {
                cell.max.y()
            },
            if face.plane.normal.z() > 0.0 {
                cell.min.z()
            } else {
                cell.max.z()
            },
        );
        face.plane.point_distance(corner) <= 0.0
    })
}

/// Looks for a way from the given points out of the level.
///
/// `solids` are the brushes that seal the level, `starts` the points to flood from --
/// where the point entities are -- and `cell_size` how finely the space is cut up. The
/// cell size should be no larger than the thinnest wall, or the flood can step through
/// one.
pub fn find_leak(solids: &[Brush], starts: &[Vec3d], cell_size: f64, cell_budget: usize) -> Leak {
    if solids.is_empty() {
        return Leak::NothingToSeal;
    }
    if starts.is_empty() {
        return Leak::NoStartingPoint;
    }

    let cell_size = cell_size.max(MIN_CELL_SIZE);

    // The level, with two cells of margin all round: one so the outside exists at all,
    // and one so the outermost cells are certainly empty and can stand for the void.
    let mut bounds = solids
        .iter()
        .map(Brush::bounds)
        .reduce(|acc, next| acc.merged(&next))
        .expect("there is at least one brush");
    // An entity outside the level's own bounds is already outside, but the grid has to
    // hold it for the flood to start anywhere.
    for start in starts {
        bounds = bounds.merged(&Bbox3d::new(*start, *start));
    }
    let margin = Vec3d::new(cell_size * 2.0, cell_size * 2.0, cell_size * 2.0);
    let bounds = Bbox3d::new(bounds.min - margin, bounds.max + margin);

    let size = bounds.size();
    let counts = [
        ((size.x() / cell_size).ceil() as usize).max(3),
        ((size.y() / cell_size).ceil() as usize).max(3),
        ((size.z() / cell_size).ceil() as usize).max(3),
    ];
    let grid = Grid {
        origin: bounds.min,
        cell_size,
        counts,
    };

    let cells = grid.len();
    if cells > cell_budget {
        return Leak::TooLarge {
            cells,
            budget: cell_budget,
        };
    }

    // Which cells a brush reaches into. Each brush is only tested against the cells its
    // own bounds cover, so a level's cost is its geometry rather than its extent.
    let mut solid = vec![false; cells];
    for brush in solids {
        let brush_bounds = brush.bounds();
        let (Some(low), Some(high)) = (
            grid.cell_at(brush_bounds.min - Vec3d::new(cell_size, cell_size, cell_size)),
            grid.cell_at(brush_bounds.max + Vec3d::new(cell_size, cell_size, cell_size)),
        ) else {
            continue;
        };
        for z in low[2]..=high[2] {
            for y in low[1]..=high[1] {
                for x in low[0]..=high[0] {
                    let at = [x, y, z];
                    let index = grid.index(at);
                    if solid[index] {
                        continue;
                    }
                    if brush_reaches(brush, &grid.cell_bounds(at)) {
                        solid[index] = true;
                    }
                }
            }
        }
    }

    // Flood from every starting point at once: any of them reaching the void is a leak,
    // and starting from all of them at once finds the shortest way out.
    let mut came_from = vec![usize::MAX; cells];
    let mut queue = VecDeque::new();
    let mut started = false;
    for start in starts {
        let Some(at) = grid.cell_at(*start) else {
            continue;
        };
        let index = grid.index(at);
        if solid[index] || came_from[index] != usize::MAX {
            continue;
        }
        started = true;
        // A starting cell is its own parent, which is what ends the walk back.
        came_from[index] = index;
        queue.push_back(index);
    }
    if !started {
        return Leak::StartsInsideSolid;
    }

    while let Some(index) = queue.pop_front() {
        let at = grid.coordinates(index);
        if grid.is_on_the_edge(at) {
            return Leak::Leaked(trace_back(&grid, &came_from, index));
        }

        for (axis, step) in [(0, -1i64), (0, 1), (1, -1), (1, 1), (2, -1), (2, 1)] {
            let mut next = at;
            let moved = next[axis] as i64 + step;
            if moved < 0 || moved as usize >= grid.counts[axis] {
                continue;
            }
            next[axis] = moved as usize;

            let next_index = grid.index(next);
            if solid[next_index] || came_from[next_index] != usize::MAX {
                continue;
            }
            came_from[next_index] = index;
            queue.push_back(next_index);
        }
    }

    Leak::Sealed
}

/// Walks the flood back from where it escaped to where it started.
fn trace_back(grid: &Grid, came_from: &[usize], escaped: usize) -> Trace {
    let mut cells = vec![escaped];
    let mut index = escaped;
    while came_from[index] != index {
        index = came_from[index];
        cells.push(index);
    }
    // Back to front: the trace reads from the entity outwards, which is the direction
    // someone follows it in.
    cells.reverse();

    let points: Vec<Vec3d> = cells
        .iter()
        .map(|index| grid.center(grid.coordinates(*index)))
        .collect();

    Trace {
        points: keep_the_turns(&points),
    }
}

/// Keeps only the points where the path changes direction.
///
/// A flood through a corridor produces one point per cell, which is a hundred points
/// saying the same thing as two.
fn keep_the_turns(points: &[Vec3d]) -> Vec<Vec3d> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut kept = vec![points[0]];
    for window in points.windows(3) {
        let before = window[1] - window[0];
        let after = window[2] - window[1];
        if before != after {
            kept.push(window[1]);
        }
    }
    kept.push(points[points.len() - 1]);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn box_brush(min: Vec3d, max: Vec3d) -> Brush {
        crate::shapes::create_cuboid(&world(), &Bbox3d::new(min, max), "stone").expect("a box")
    }

    /// A hollow room with walls sixteen units thick, inside 128 units on a side.
    ///
    /// `missing` names a wall to leave out, so that a sealed room and a leaking one differ
    /// by exactly one brush.
    fn room(missing: Option<usize>) -> Vec<Brush> {
        let outer = 80.0;
        let inner = 64.0;
        let walls = [
            // -X, +X
            (
                Vec3d::new(-outer, -outer, -outer),
                Vec3d::new(-inner, outer, outer),
            ),
            (
                Vec3d::new(inner, -outer, -outer),
                Vec3d::new(outer, outer, outer),
            ),
            // -Y, +Y
            (
                Vec3d::new(-outer, -outer, -outer),
                Vec3d::new(outer, -inner, outer),
            ),
            (
                Vec3d::new(-outer, inner, -outer),
                Vec3d::new(outer, outer, outer),
            ),
            // -Z, +Z
            (
                Vec3d::new(-outer, -outer, -outer),
                Vec3d::new(outer, outer, -inner),
            ),
            (
                Vec3d::new(-outer, -outer, inner),
                Vec3d::new(outer, outer, outer),
            ),
        ];

        walls
            .into_iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != missing)
            .map(|(_, (min, max))| box_brush(min, max))
            .collect()
    }

    #[test]
    fn a_sealed_room_is_sealed() {
        let outcome = find_leak(&room(None), &[Vec3d::zero()], 8.0, DEFAULT_CELL_BUDGET);
        assert_eq!(outcome, Leak::Sealed, "a closed box should hold");
    }

    #[test]
    fn a_room_with_a_wall_missing_leaks() {
        for missing in 0..6 {
            let outcome = find_leak(
                &room(Some(missing)),
                &[Vec3d::zero()],
                8.0,
                DEFAULT_CELL_BUDGET,
            );
            assert!(
                matches!(outcome, Leak::Leaked(_)),
                "wall {missing} missing gave {outcome:?}"
            );
        }
    }

    #[test]
    fn the_trace_starts_at_the_entity_and_ends_outside() {
        let start = Vec3d::new(0.0, 0.0, 0.0);
        let Leak::Leaked(trace) = find_leak(&room(Some(0)), &[start], 8.0, DEFAULT_CELL_BUDGET)
        else {
            panic!("the room leaks");
        };

        assert!(trace.points.len() >= 2, "{:?}", trace.points);
        // The first point is the cell the entity is in, which is within a cell of it.
        let first = trace.points[0];
        assert!(
            geometry_math::vec::length(first - start) < 8.0 * 2.0,
            "the trace starts at {first:?} rather than near {start:?}"
        );
        // And the last is outside the room, which is what makes it a leak.
        let last = trace.points[trace.points.len() - 1];
        assert!(
            last.x().abs() > 80.0 || last.y().abs() > 80.0 || last.z().abs() > 80.0,
            "the trace ends at {last:?}, which is still inside"
        );
    }

    #[test]
    fn a_trace_down_a_corridor_is_not_one_point_per_cell() {
        // Only the turns are kept, so following the line is possible.
        let Leak::Leaked(trace) =
            find_leak(&room(Some(1)), &[Vec3d::zero()], 4.0, DEFAULT_CELL_BUDGET)
        else {
            panic!("the room leaks");
        };
        // A straight run out through one wall is a handful of turns, not the forty cells
        // it passes through.
        assert!(trace.points.len() < 12, "{:?}", trace.points);
    }

    #[test]
    fn a_hole_smaller_than_a_cell_is_not_stepped_through() {
        // The generous solid test is what makes this hold: a cell the wall reaches into
        // at all is solid, so a gap has to be a whole cell wide before the flood fits.
        let mut walls = room(None);
        // A gap eight units across in the -X wall, which is the width of one cell.
        walls[0] = box_brush(
            Vec3d::new(-80.0, -80.0, -80.0),
            Vec3d::new(-64.0, 80.0, -4.0),
        );
        walls.push(box_brush(
            Vec3d::new(-80.0, -80.0, 4.0),
            Vec3d::new(-64.0, 80.0, 80.0),
        ));

        // At a cell size of sixteen the gap is smaller than a cell, and the check says
        // sealed rather than guessing.
        assert_eq!(
            find_leak(&walls, &[Vec3d::zero()], 16.0, DEFAULT_CELL_BUDGET),
            Leak::Sealed
        );
        // At a cell size of two it fits, and the leak is found.
        assert!(matches!(
            find_leak(&walls, &[Vec3d::zero()], 2.0, DEFAULT_CELL_BUDGET),
            Leak::Leaked(_)
        ));
    }

    #[test]
    fn an_entity_buried_in_a_wall_is_said_so_rather_than_called_a_leak() {
        let outcome = find_leak(
            &room(None),
            &[Vec3d::new(-72.0, 0.0, 0.0)],
            8.0,
            DEFAULT_CELL_BUDGET,
        );
        assert_eq!(outcome, Leak::StartsInsideSolid);
    }

    #[test]
    fn a_map_with_no_geometry_has_nothing_to_seal() {
        assert_eq!(
            find_leak(&[], &[Vec3d::zero()], 8.0, DEFAULT_CELL_BUDGET),
            Leak::NothingToSeal
        );
    }

    #[test]
    fn a_map_with_no_point_entity_has_nowhere_to_check_from() {
        assert_eq!(
            find_leak(&room(None), &[], 8.0, DEFAULT_CELL_BUDGET),
            Leak::NoStartingPoint
        );
    }

    #[test]
    fn a_level_too_large_for_the_budget_says_so_rather_than_hanging() {
        let far = room(None);
        let outcome = find_leak(&far, &[Vec3d::zero()], 0.01, 1000);
        assert!(matches!(outcome, Leak::TooLarge { .. }), "{outcome:?}");
    }

    #[test]
    fn a_second_room_sealed_off_from_the_first_does_not_leak_through_it() {
        // Two sealed rooms with no way between them: an entity in one cannot reach the
        // void through the other.
        let mut walls = room(None);
        for wall in room(None) {
            let moved = wall
                .translate(&world(), Vec3d::new(256.0, 0.0, 0.0))
                .expect("still a box");
            walls.push(moved);
        }
        assert_eq!(
            find_leak(&walls, &[Vec3d::zero()], 8.0, DEFAULT_CELL_BUDGET),
            Leak::Sealed
        );
    }

    #[test]
    fn one_entity_outside_is_enough_to_leak() {
        // The flood starts from every entity, so one left outside the level reaches the
        // void immediately -- which is a real mistake worth being told about.
        let outcome = find_leak(
            &room(None),
            &[Vec3d::zero(), Vec3d::new(512.0, 0.0, 0.0)],
            8.0,
            DEFAULT_CELL_BUDGET,
        );
        assert!(matches!(outcome, Leak::Leaked(_)), "{outcome:?}");
    }
}
