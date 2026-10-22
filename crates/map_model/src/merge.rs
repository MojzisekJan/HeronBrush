//! A structural three-way merge of `.map` documents.
//!
//! Three-way map reconciliation for version-control merges and live-session resynchronisation.
//!
//! The merge works on objects, not lines. A `.map` brush spans many lines and its entity
//! and layer structure is carried in `_tb_` properties and id references, so a line merge
//! readily produces a file that is syntactically broken or has orphaned nodes. This one
//! matches entities across the three versions (by `_tb_id`, with `worldspawn` as a
//! singleton and everything else positional within its classname), then merges each
//! entity's properties key by key and its brushes and patches as sets.
//!
//! A brush carries no id, so an edited brush reads as one removed and one added. A base
//! brush both sides dropped is paired with a similar brush each side added -- similar
//! meaning they share most of their face planes, which survives a face being dragged or
//! retextured -- so an edit of one brush on both sides comes out as a single [`Conflict`]
//! with *ours* kept, not two overlapping brushes.
//!
//! ## What v1 does not do
//!
//! - It does not merge *inside* a brush: two people editing different faces of one brush
//!   still conflict, rather than both edits landing.
//! - An entity with a repeated property key falls back to a whole-property-list
//!   three-way; keys are otherwise merged one at a time.
//! - Group and layer structure rides along as ordinary `_tb_` properties, so a move
//!   between layers on both sides is a property conflict, not a structural one.

use std::cell::RefCell;
use std::collections::HashMap;

use geometry_math::plane::Plane3d;
use geometry_math::vec;

use crate::entity_properties::keys;
use crate::io::map_reader::{read_map, Brush, Entity, MapFile, ReadError};
use crate::io::map_writer::write_map;

/// How an entity is identified across the three versions being merged. Everything but
/// `Worldspawn` and `TbId` carries `nth`, the count of earlier entities in the same map
/// with the same identity, so two lights stacked at one spot still get distinct keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityKey {
    /// The worldspawn, matched as the single entity with that classname.
    Worldspawn,
    /// Matched by the id the editor writes before the entity, which says outright which
    /// entity this is. Everything below it is a way of recognising an entity that has
    /// none -- one from another editor, or from a file written before ids existed.
    NodeId(u64),
    /// Matched by its `_tb_id` property, which a `.map` file carries on a group or layer.
    TbId(String),
    /// Matched by classname and `targetname` -- the name a trigger, door or logic entity
    /// is wired to others by, which survives the entity being moved.
    Named {
        /// The `classname`.
        classname: String,
        /// The `targetname`.
        targetname: String,
        /// How many earlier entities in the map share this identity.
        nth: usize,
    },
    /// Matched by classname and `origin` rounded to whole units -- an entity that has not
    /// moved more than half a unit.
    Spot {
        /// The `classname`.
        classname: String,
        /// The `origin`, each component rounded to an integer.
        origin: String,
        /// How many earlier entities in the map share this identity.
        nth: usize,
    },
    /// No name and no origin: matched by classname and position among such entities.
    Positional {
        /// The `classname`, or `""` if it has none.
        classname: String,
        /// The 0-based position among same-classname entities matched this way.
        nth: usize,
    },
}

impl std::fmt::Display for EntityKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = |c: &str| {
            if c.is_empty() {
                "(no classname)".to_string()
            } else {
                c.to_string()
            }
        };
        match self {
            EntityKey::Worldspawn => f.write_str("worldspawn"),
            EntityKey::NodeId(id) => write!(f, "entity {id:x}"),
            EntityKey::TbId(id) => write!(f, "entity _tb_id {id}"),
            EntityKey::Named {
                classname,
                targetname,
                ..
            } => {
                write!(f, "{} \"{targetname}\"", name(classname))
            }
            EntityKey::Spot {
                classname, origin, ..
            } => {
                write!(f, "{} at {origin}", name(classname))
            }
            EntityKey::Positional { classname, nth } => write!(f, "{} #{nth}", name(classname)),
        }
    }
}

/// Something the merge could not decide on its own. The merge still produces a complete
/// map; a conflict says which part of it was resolved by preferring *ours*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// Where the conflict is, e.g. `worldspawn` or `light #2`.
    pub scope: String,
    /// What could not be reconciled.
    pub detail: String,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.scope, self.detail)
    }
}

/// The result of [`merge`].
#[derive(Debug, Clone)]
pub struct MergeOutput {
    /// The merged map.
    pub map: MapFile,
    /// Every part the merge resolved by preferring *ours*. Empty means a clean merge.
    pub conflicts: Vec<Conflict>,
}

/// The standard three-way rule for a single value: a side that did not change from the
/// base yields to the side that did; if both changed to the same thing that is the
/// result; otherwise it is a conflict.
fn three_way<T: PartialEq + Clone>(base: &T, ours: &T, theirs: &T) -> Result<T, ()> {
    if ours == base {
        Ok(theirs.clone())
    } else if theirs == base || ours == theirs {
        Ok(ours.clone())
    } else {
        Err(())
    }
}

fn tb_id(entity: &Entity) -> Option<&str> {
    entity.property(keys::TB_ID).filter(|s| !s.is_empty())
}

/// `"0.0 0 64.00"` -> `"0 0 64"`; `None` if it is not three numbers.
fn normalize_origin(origin: &str) -> Option<String> {
    let parts: Vec<f64> = origin
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let [x, y, z] = parts[..] else { return None };
    Some(format!(
        "{} {} {}",
        x.round() as i64,
        y.round() as i64,
        z.round() as i64
    ))
}

/// The part of an entity's identity that does not depend on how many like it came before.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Candidate {
    Named(String, String),
    Spot(String, String),
    Positional(String),
}

/// Keys every entity in `map`, in file order, and returns `key -> index`.
fn key_entities(map: &MapFile) -> Vec<(EntityKey, usize)> {
    let mut result = Vec::with_capacity(map.entities.len());
    let mut counts: HashMap<Candidate, usize> = HashMap::new();
    for (index, entity) in map.entities.iter().enumerate() {
        let key = if entity.classname() == Some("worldspawn") {
            EntityKey::Worldspawn
        } else if let Some(id) = entity.id {
            EntityKey::NodeId(id)
        } else if let Some(id) = tb_id(entity) {
            EntityKey::TbId(id.to_string())
        } else {
            let classname = entity.classname().unwrap_or("").to_string();
            let candidate = if let Some(name) =
                entity.property(keys::TARGETNAME).filter(|s| !s.is_empty())
            {
                Candidate::Named(classname.clone(), name.to_string())
            } else if let Some(origin) = entity.property(keys::ORIGIN).and_then(normalize_origin) {
                Candidate::Spot(classname.clone(), origin)
            } else {
                Candidate::Positional(classname.clone())
            };
            let nth = *counts
                .entry(candidate.clone())
                .and_modify(|c| *c += 1)
                .or_insert(0);
            match candidate {
                Candidate::Named(classname, targetname) => EntityKey::Named {
                    classname,
                    targetname,
                    nth,
                },
                Candidate::Spot(classname, origin) => EntityKey::Spot {
                    classname,
                    origin,
                    nth,
                },
                Candidate::Positional(classname) => EntityKey::Positional { classname, nth },
            }
        };
        result.push((key, index));
    }
    result
}

fn lookup(keyed: &[(EntityKey, usize)], key: &EntityKey) -> Option<usize> {
    keyed.iter().find(|(k, _)| k == key).map(|(_, i)| *i)
}

/// Merges `ours` and `theirs`, which both descend from `base`.
pub fn merge(base: &MapFile, ours: &MapFile, theirs: &MapFile) -> MergeOutput {
    let mut conflicts = Vec::new();

    let format = three_way(&base.format, &ours.format, &theirs.format).unwrap_or_else(|()| {
        conflicts.push(Conflict {
            scope: "map".into(),
            detail: format!(
                "format changed on both sides ({} vs {}); kept ours",
                ours.format, theirs.format
            ),
        });
        ours.format
    });
    let game = three_way(&base.game, &ours.game, &theirs.game).unwrap_or_else(|()| {
        conflicts.push(Conflict {
            scope: "map".into(),
            detail: "game changed on both sides; kept ours".into(),
        });
        ours.game.clone()
    });

    let base_keys = key_entities(base);
    let ours_keys = key_entities(ours);
    let theirs_keys = key_entities(theirs);

    // The key order: base's entities in base order, then keys new in ours, then new in
    // theirs.
    let mut order: Vec<EntityKey> = base_keys.iter().map(|(k, _)| k.clone()).collect();
    for (k, _) in &ours_keys {
        if !order.contains(k) {
            order.push(k.clone());
        }
    }
    for (k, _) in &theirs_keys {
        if !order.contains(k) {
            order.push(k.clone());
        }
    }

    let mut entities = Vec::new();
    for key in &order {
        let in_base = lookup(&base_keys, key).map(|i| &base.entities[i]);
        let in_ours = lookup(&ours_keys, key).map(|i| &ours.entities[i]);
        let in_theirs = lookup(&theirs_keys, key).map(|i| &theirs.entities[i]);

        match (in_base, in_ours, in_theirs) {
            (Some(b), Some(o), Some(t)) => {
                entities.push(merge_entity(key, b, o, t, &mut conflicts));
            }
            (Some(b), Some(o), None) => {
                if o == b {
                    // theirs removed it, ours left it alone: remove it.
                } else {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "edited on our side, removed on theirs; kept ours".into(),
                    });
                    entities.push(o.clone());
                }
            }
            (Some(b), None, Some(t)) => {
                if t == b {
                    // ours removed it, theirs left it alone: remove it.
                } else {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "removed on our side, edited on theirs; dropped".into(),
                    });
                }
            }
            (Some(_), None, None) => {
                // both removed it.
            }
            (None, Some(o), Some(t)) => {
                if o == t {
                    entities.push(o.clone());
                } else {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "added on both sides with different content; kept ours".into(),
                    });
                    entities.push(o.clone());
                }
            }
            (None, Some(o), None) => entities.push(o.clone()),
            (None, None, Some(t)) => entities.push(t.clone()),
            (None, None, None) => unreachable!("a key came from one of the three maps"),
        }
    }

    MergeOutput {
        map: MapFile {
            format,
            game,
            entities,
            // What comes out is written by this editor, whatever the three inputs said.
            notes_version: Some(crate::map_format::NOTES_VERSION),
            warnings: Vec::new(),
        },
        conflicts,
    }
}

fn merge_entity(
    key: &EntityKey,
    base: &Entity,
    ours: &Entity,
    theirs: &Entity,
    conflicts: &mut Vec<Conflict>,
) -> Entity {
    let properties = merge_properties(key, base, ours, theirs, conflicts);
    let brushes = merge_brushes(
        key,
        &base.brushes,
        &ours.brushes,
        &theirs.brushes,
        conflicts,
    );
    let patches = merge_set(&base.patches, &ours.patches, &theirs.patches);

    // Ours' identity, since a merged entity is the one that was there: the id is what it
    // is, not something the merge decides.
    Entity {
        properties,
        brushes,
        patches,
        id: ours.id.or(theirs.id),
        line: ours.line,
    }
}

fn has_duplicate_keys(entity: &Entity) -> bool {
    let mut seen = std::collections::HashSet::new();
    !entity
        .properties
        .iter()
        .all(|(k, _)| seen.insert(k.as_str()))
}

fn merge_properties(
    key: &EntityKey,
    base: &Entity,
    ours: &Entity,
    theirs: &Entity,
    conflicts: &mut Vec<Conflict>,
) -> Vec<(String, String)> {
    if has_duplicate_keys(base) || has_duplicate_keys(ours) || has_duplicate_keys(theirs) {
        return match three_way(&base.properties, &ours.properties, &theirs.properties) {
            Ok(merged) => merged,
            Err(()) => {
                conflicts.push(Conflict {
                    scope: key.to_string(),
                    detail: "properties changed on both sides (repeated key); kept ours".into(),
                });
                ours.properties.clone()
            }
        };
    }

    let get = |e: &Entity, k: &str| e.property(k).map(str::to_string);

    // key order: base order, then keys new in ours, then new in theirs.
    let mut prop_order: Vec<String> = base.properties.iter().map(|(k, _)| k.clone()).collect();
    for (k, _) in ours.properties.iter().chain(theirs.properties.iter()) {
        if !prop_order.contains(k) {
            prop_order.push(k.clone());
        }
    }

    let mut result = Vec::new();
    for k in &prop_order {
        let b = get(base, k);
        let o = get(ours, k);
        let t = get(theirs, k);
        match three_way(&b, &o, &t) {
            Ok(Some(value)) => result.push((k.clone(), value)),
            Ok(None) => {}
            Err(()) => {
                conflicts.push(Conflict {
                    scope: key.to_string(),
                    detail: format!("property \"{k}\" changed on both sides; kept ours"),
                });
                if let Some(value) = o {
                    result.push((k.clone(), value));
                }
            }
        }
    }
    result
}

/// Removes the first item of `haystack` equal to `needle`, returning whether it did.
fn take_one<T: PartialEq>(haystack: &mut Vec<T>, needle: &T) -> bool {
    if let Some(pos) = haystack.iter().position(|x| x == needle) {
        haystack.remove(pos);
        true
    } else {
        false
    }
}

/// Multiset difference `a \ b`.
fn difference<T: PartialEq + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut remaining: Vec<T> = b.to_vec();
    let mut out = Vec::new();
    for item in a {
        if !take_one(&mut remaining, item) {
            out.push(item.clone());
        }
    }
    out
}

/// A set (really multiset) three-way merge: keep what all three still have, then add what
/// each side added, without adding the same new item twice.
fn merge_set<T: PartialEq + Clone>(base: &[T], ours: &[T], theirs: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut ours_pool: Vec<T> = ours.to_vec();
    let mut theirs_pool: Vec<T> = theirs.to_vec();
    for item in base {
        // kept only if it survives in both sides
        if take_one(&mut ours_pool, item) && take_one(&mut theirs_pool, item) {
            result.push(item.clone());
        }
    }

    let added_ours = difference(ours, base);
    let mut added_theirs = difference(theirs, base);
    for item in &added_ours {
        result.push(item.clone());
        take_one(&mut added_theirs, item);
    }
    result.extend(added_theirs);
    result
}

fn plane_close(a: &Plane3d, b: &Plane3d) -> bool {
    vec::dot(a.normal, b.normal) > 1.0 - 1e-4 && (a.distance - b.distance).abs() < 1e-3
}

/// Where a plane falls on a coarse lattice, so brushes that could share it are found
/// without comparing every pair.
type PlaneCell = (i64, i64, i64, i64);

/// The lattice [`PlaneIndex`] buckets by. Far coarser than [`plane_close`]'s tolerance,
/// so two planes it calls equal almost always land in one cell -- and where they
/// straddle a boundary, [`plane_cells`] returns both.
const NORMAL_CELL: f64 = 0.25;
const DISTANCE_CELL: f64 = 4.0;

/// How far apart [`plane_close`] still calls two planes equal, per coordinate. A dot
/// product over `1 - 1e-4` puts the angle under about `0.015` radians, so no component
/// of the normal moves further than that.
const NORMAL_TOLERANCE: f64 = 0.02;
const DISTANCE_TOLERANCE: f64 = 1e-3;

/// Every cell a plane within tolerance of this one could be in: its own, and the
/// neighbour along any coordinate it sits near the edge of. One cell in the ordinary
/// case, up to sixteen right on a corner -- so they go in an array rather than a `Vec`,
/// since this is called for every face of every brush being matched.
fn plane_cells(plane: &Plane3d) -> ([PlaneCell; 16], usize) {
    let coordinates = [
        (plane.normal[0], NORMAL_CELL, NORMAL_TOLERANCE),
        (plane.normal[1], NORMAL_CELL, NORMAL_TOLERANCE),
        (plane.normal[2], NORMAL_CELL, NORMAL_TOLERANCE),
        (plane.distance, DISTANCE_CELL, DISTANCE_TOLERANCE),
    ];
    let mut cells = [[0i64; 4]; 16];
    let mut count = 1;
    for (axis, (value, cell, tolerance)) in coordinates.iter().enumerate() {
        let scaled = value / cell;
        let here = scaled.floor() as i64;
        let within = scaled - scaled.floor();
        let neighbour = if within < tolerance / cell {
            Some(here - 1)
        } else if within > 1.0 - tolerance / cell {
            Some(here + 1)
        } else {
            None
        };
        for cell in cells.iter_mut().take(count) {
            cell[axis] = here;
        }
        if let Some(neighbour) = neighbour {
            for at in 0..count {
                let mut other = cells[at];
                other[axis] = neighbour;
                cells[count + at] = other;
            }
            count *= 2;
        }
    }
    let mut out = [(0i64, 0, 0, 0); 16];
    for at in 0..count {
        out[at] = (cells[at][0], cells[at][1], cells[at][2], cells[at][3]);
    }
    (out, count)
}

/// Which brushes of a pool hold which face planes.
///
/// [`brush_similarity`] is the share of face planes two brushes have in common, and
/// [`BRUSH_MATCH`] is above zero -- so a brush sharing no plane with the one being
/// matched can never be the answer, and need never be compared against it. Without this
/// the pairing step compares every removed brush with every added one, which on a map
/// where both sides moved everything is the whole document squared: 3,960 brushes took
/// 9.7 seconds, and the merge runs as a git merge driver where that reads as a hang.
struct PlaneIndex {
    holders: HashMap<PlaneCell, Vec<usize>>,
    /// Scratch for [`PlaneIndex::candidates`], kept across calls. A merge asks this
    /// question once per brush and the answer can name hundreds of pool positions, so
    /// allocating and sorting a fresh list each time costs more than the scan it
    /// replaces on a small map.
    scratch: RefCell<Scratch>,
}

/// The working space one [`PlaneIndex::candidates`] call needs, reused by the next.
#[derive(Default)]
struct Scratch {
    /// How many of the asking brush's faces matched each pool position.
    counts: Vec<u32>,
    /// Which entries of `counts` were touched, so only those are cleared again.
    touched: Vec<usize>,
    /// The face each pool position was last counted for, so one face counts once
    /// however many of its cells the position sits in.
    counted_for: Vec<u32>,
    /// Bumped per face, so `counted_for` needs no clearing.
    face: u32,
}

impl PlaneIndex {
    fn of(pool: &[&Brush]) -> PlaneIndex {
        let mut index = PlaneIndex {
            holders: HashMap::new(),
            scratch: RefCell::new(Scratch {
                counts: vec![0; pool.len()],
                touched: Vec::new(),
                counted_for: vec![0; pool.len()],
                face: 0,
            }),
        };
        for (at, brush) in pool.iter().enumerate() {
            index.add(at, brush);
        }
        index
    }

    /// Records that the brush at `at` holds the planes it holds.
    fn add(&mut self, at: usize, brush: &Brush) {
        let mut scratch = self.scratch.borrow_mut();
        if scratch.counts.len() <= at {
            scratch.counts.resize(at + 1, 0);
            scratch.counted_for.resize(at + 1, 0);
        }
        drop(scratch);
        for face in &brush.faces {
            let (cells, count) = plane_cells(&face.plane);
            for cell in &cells[..count] {
                let bucket = self.holders.entry(*cell).or_default();
                // A brush with two faces in one cell is listed once.
                if bucket.last() != Some(&at) {
                    bucket.push(at);
                }
            }
        }
    }

    /// Every brush sharing a plane cell with `brush`, as `(pool position, how many of
    /// `brush`'s faces it could match)`, in pool order.
    ///
    /// Pool order is the order a scan over the whole pool would have visited them in, so
    /// the pairing picks the same brush it always did when two are equally alike.
    ///
    /// The count is what makes this worth having on real geometry. Axis-aligned boxes
    /// share their side planes with every box in the same row, so "shares a plane" alone
    /// still names hundreds of candidates; "shares enough planes to clear the threshold"
    /// names the handful that could actually match. It counts cells rather than planes,
    /// and cells are coarser than [`plane_close`], so it can only ever over-count -- and
    /// an over-count keeps a candidate that would then be measured properly, which is
    /// the safe direction.
    fn candidates(&self, brush: &Brush) -> Vec<(usize, usize)> {
        let mut scratch = self.scratch.borrow_mut();
        let scratch = &mut *scratch;
        for at in scratch.touched.drain(..) {
            scratch.counts[at] = 0;
        }

        for face in &brush.faces {
            scratch.face += 1;
            let stamp = scratch.face;
            let (cells, count) = plane_cells(&face.plane);
            for cell in &cells[..count] {
                let Some(bucket) = self.holders.get(cell) else {
                    continue;
                };
                for at in bucket {
                    // One face counts once for a pool brush, however many of its cells
                    // that brush sits in.
                    if scratch.counted_for[*at] == stamp {
                        continue;
                    }
                    scratch.counted_for[*at] = stamp;
                    if scratch.counts[*at] == 0 {
                        scratch.touched.push(*at);
                    }
                    scratch.counts[*at] += 1;
                }
            }
        }

        // Pool order, which is the order a scan over the whole pool would have visited
        // them in -- so the pairing picks the same brush it always did when two are
        // equally alike.
        scratch.touched.sort_unstable();
        scratch
            .touched
            .iter()
            .map(|at| (*at, scratch.counts[*at] as usize))
            .collect()
    }
}

/// How alike two brushes are, `0.0` to `1.0`: the share of face planes they have in
/// common. Two versions of one brush after a face was dragged or retextured still share
/// most of their planes.
fn brush_similarity(a: &Brush, b: &Brush) -> f64 {
    if a.faces.is_empty() && b.faces.is_empty() {
        return 1.0;
    }
    if a.faces.is_empty() || b.faces.is_empty() {
        return 0.0;
    }
    let shared = a
        .faces
        .iter()
        .filter(|fa| b.faces.iter().any(|fb| plane_close(&fa.plane, &fb.plane)))
        .count();
    2.0 * shared as f64 / (a.faces.len() + b.faces.len()) as f64
}

/// The most similar of `candidates` to `target`, if any clears [`BRUSH_MATCH`].
///
/// `candidates` are pool positions in ascending order and `taken` says which of the pool
/// is already spoken for, so this walks the same brushes in the same order a scan over
/// the whole pool would have -- just without the ones that cannot possibly match.
fn best_match(
    pool: &[&Brush],
    taken: &[bool],
    candidates: &[(usize, usize)],
    target: &Brush,
) -> Option<usize> {
    candidates
        .iter()
        .copied()
        .filter(|(at, _)| !taken[*at])
        // `2 * shared / (a + b) >= BRUSH_MATCH` cannot hold unless the two share at
        // least `(a + b) * BRUSH_MATCH / 2` planes, and `could` is an over-count of the
        // shared ones -- so a candidate failing this could not have cleared the
        // threshold, and is dropped without the full comparison.
        .filter(|(at, could)| {
            let both = (pool[*at].faces.len() + target.faces.len()) as f64;
            2.0 * *could as f64 >= BRUSH_MATCH * both
        })
        .map(|(at, _)| (at, brush_similarity(pool[at], target)))
        .filter(|(_, s)| *s >= BRUSH_MATCH)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(at, _)| at)
}

const BRUSH_MATCH: f64 = 0.5;

/// The brush set merge, with the extra step of pairing a base brush that both sides
/// dropped with a similar brush each side added -- an edit of the same brush on both
/// sides, which the plain set merge would otherwise keep twice.
///
/// Assembled as references and cloned once at the end, so the "is this already in the
/// result" test can index what is there without borrowing a growing `Vec` of owned
/// brushes.
fn merge_brushes<'a>(
    key: &EntityKey,
    base: &'a [Brush],
    ours: &'a [Brush],
    theirs: &'a [Brush],
    conflicts: &mut Vec<Conflict>,
) -> Vec<Brush> {
    let mut result: Vec<&'a Brush> = Vec::new();
    let ours_pool: Vec<&'a Brush> = ours.iter().collect();
    let theirs_pool: Vec<&'a Brush> = theirs.iter().collect();
    // Taken out of the pool by being matched, marked rather than removed so the index
    // keeps pointing at the right brushes.
    let mut ours_taken = vec![false; ours_pool.len()];
    let mut theirs_taken = vec![false; theirs_pool.len()];

    let ours_index = PlaneIndex::of(&ours_pool);
    let theirs_index = PlaneIndex::of(&theirs_pool);

    // The id pass. A brush written by this editor carries one, so the three versions can
    // say outright which brush is which instead of the geometry having to be recognised.
    // What that buys is the case the similarity pass cannot get right: a brush edited on
    // one side and left alone on the other is an edit, not a removal and an addition, so
    // the edit lands rather than being weighed against a look-alike.
    let mut base_handled = vec![false; base.len()];
    let where_by_id = |pool: &[&'a Brush]| -> HashMap<u64, usize> {
        pool.iter()
            .enumerate()
            .filter_map(|(at, brush)| brush.id.map(|id| (id, at)))
            .collect()
    };
    let ours_by_id = where_by_id(&ours_pool);
    let theirs_by_id = where_by_id(&theirs_pool);

    for (at, b) in base.iter().enumerate() {
        let Some(id) = b.id else {
            continue;
        };
        let ours_at = ours_by_id.get(&id).copied().filter(|at| !ours_taken[*at]);
        let theirs_at = theirs_by_id
            .get(&id)
            .copied()
            .filter(|at| !theirs_taken[*at]);
        base_handled[at] = true;
        match (ours_at, theirs_at) {
            (Some(oi), Some(ti)) => {
                ours_taken[oi] = true;
                theirs_taken[ti] = true;
                let (ob, tb) = (ours_pool[oi], theirs_pool[ti]);
                // Whichever side changed it, that is what the brush became. Both sides
                // changing it the same way is not a disagreement either.
                if ob == tb || tb == b {
                    result.push(ob);
                } else if ob == b {
                    result.push(tb);
                } else {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "a brush was edited on both sides; kept ours".into(),
                    });
                    result.push(ob);
                }
            }
            (Some(oi), None) => {
                ours_taken[oi] = true;
                if ours_pool[oi] == b {
                    // Removed on theirs and untouched on ours: it goes.
                } else {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "a brush was edited on our side and removed on theirs; kept ours"
                            .into(),
                    });
                    result.push(ours_pool[oi]);
                }
            }
            (None, Some(ti)) => {
                theirs_taken[ti] = true;
                if theirs_pool[ti] != b {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "a brush was removed on our side and edited on theirs; dropped"
                            .into(),
                    });
                }
            }
            // Both sides removed it, and agree.
            (None, None) => {}
        }
    }

    /// Takes the first brush of the pool equal to `needle`, if it is still there. An
    /// equal brush shares every plane, so the index has it among the candidates.
    fn take_equal(pool: &[&Brush], taken: &mut [bool], index: &PlaneIndex, needle: &Brush) -> bool {
        for (at, _) in index.candidates(needle) {
            if !taken[at] && pool[at] == needle {
                taken[at] = true;
                return true;
            }
        }
        false
    }

    let mut removed_by_both: Vec<&'a Brush> = Vec::new();
    for (at, b) in base.iter().enumerate() {
        if base_handled[at] {
            continue;
        }
        let in_ours = take_equal(&ours_pool, &mut ours_taken, &ours_index, b);
        let in_theirs = take_equal(&theirs_pool, &mut theirs_taken, &theirs_index, b);
        match (in_ours, in_theirs) {
            (true, true) => result.push(b),
            (false, false) => removed_by_both.push(b),
            // dropped by exactly one side: gone. If that side replaced it, the new brush
            // is still in its pool and falls through to the union below.
            _ => {}
        }
    }

    // What is left untaken in each pool is a brush that side added.
    for removed in &removed_by_both {
        let ours_near = ours_index.candidates(removed);
        let theirs_near = theirs_index.candidates(removed);
        let ours_best = best_match(&ours_pool, &ours_taken, &ours_near, removed);
        let theirs_best = best_match(&theirs_pool, &theirs_taken, &theirs_near, removed);
        match (ours_best, theirs_best) {
            (Some(oi), Some(ti)) => {
                ours_taken[oi] = true;
                theirs_taken[ti] = true;
                let (ob, tb) = (ours_pool[oi], theirs_pool[ti]);
                if ob != tb {
                    conflicts.push(Conflict {
                        scope: key.to_string(),
                        detail: "a brush was edited on both sides; kept ours".into(),
                    });
                }
                result.push(ob);
            }
            (Some(_), None) => {
                conflicts.push(Conflict {
                    scope: key.to_string(),
                    detail: "a brush was edited on our side and removed on theirs; kept ours"
                        .into(),
                });
                // ours' edited brush stays untaken and joins the union.
            }
            (None, Some(ti)) => {
                conflicts.push(Conflict {
                    scope: key.to_string(),
                    detail: "a brush was removed on our side and edited on theirs; dropped".into(),
                });
                theirs_taken[ti] = true;
            }
            (None, None) => {}
        }
    }

    // Ours' additions join in pool order, then theirs' that are not there already. That
    // last test goes through an index of the result for the same reason the pairing
    // does: scanning the result per brush is the document squared again.
    for (b, taken) in ours_pool.iter().zip(&ours_taken) {
        if !taken {
            result.push(b);
        }
    }
    let mut result_index = PlaneIndex::of(&result);
    for (b, taken) in theirs_pool.iter().zip(&theirs_taken) {
        if *taken {
            continue;
        }
        let seen = result_index
            .candidates(b)
            .into_iter()
            .any(|(at, _)| result[at] == *b);
        if !seen {
            result_index.add(result.len(), b);
            result.push(b);
        }
    }
    result.into_iter().cloned().collect()
}

/// Reads three `.map` texts, merges them and writes the result.
pub fn merge_text(
    base: &str,
    ours: &str,
    theirs: &str,
) -> Result<(String, Vec<Conflict>), ReadError> {
    let base = read_map(base, None)?;
    let ours = read_map(ours, None)?;
    let theirs = read_map(theirs, None)?;
    let output = merge(&base, &ours, &theirs);
    Ok((write_map(&output.map), output.conflicts))
}
