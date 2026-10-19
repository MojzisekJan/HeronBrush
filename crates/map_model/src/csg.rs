//! Constructive solid geometry: combining brushes by cutting them against each other.
//!
//! A brush must be convex, so subtracting one from another generally cannot be one brush:
//! carving a doorway out of a wall leaves a shape with a notch in it. The result is a set
//! of convex pieces instead, which is how a level is built out of brushes in the first
//! place.

use geometry_math::plane::Plane3d;

use crate::brush::{Brush, ClipOutcome};

/// Subtracts `tool` from `target`, returning the pieces of `target` that lie outside it.
///
/// Returns the target unchanged as a single piece if the two do not overlap, and nothing
/// at all if the tool swallows the target whole.
///
/// The pieces do not overlap one another: each is the part of what is left over that lies
/// in front of one of the tool's faces, so every point of the target outside the tool
/// belongs to exactly one piece.
pub fn subtract(target: &Brush, tool: &Brush) -> Vec<Brush> {
    // Brushes that are nowhere near each other cannot cut each other.
    if !target.bounds().intersects(&tool.bounds()) {
        return vec![target.clone()];
    }

    let mut pieces = Vec::new();
    // What is left after taking off the parts already known to be outside the tool.
    let mut remainder = target.clone();

    for plane in tool.face_planes() {
        // The part of the remainder in front of this face is outside the tool for good.
        if let ClipOutcome::Clipped(piece) = remainder.clip(&flip(&plane)) {
            pieces.push(piece);
        }

        // Carry on with the part behind this face, which may still be inside the tool.
        match remainder.clip(&plane) {
            ClipOutcome::Clipped(behind) => remainder = behind,
            // Everything is already behind this face; nothing to take off.
            ClipOutcome::Unchanged => {}
            // Everything is in front of it, so the whole remainder is outside the tool.
            ClipOutcome::Empty => {
                pieces.push(remainder);
                return pieces;
            }
        }
    }

    // Whatever is still in the remainder is inside the tool, and is what was subtracted.
    pieces
}

/// Returns the part the two brushes have in common, or `None` if they do not overlap.
pub fn intersect(a: &Brush, b: &Brush) -> Option<Brush> {
    if !a.bounds().intersects(&b.bounds()) {
        return None;
    }

    let mut result = a.clone();
    for plane in b.face_planes() {
        match result.clip(&plane) {
            ClipOutcome::Clipped(clipped) => result = clipped,
            ClipOutcome::Unchanged => {}
            ClipOutcome::Empty => return None,
        }
    }
    Some(result)
}

/// Returns the pieces left after subtracting every tool from the target in turn.
///
/// Each piece of one subtraction is fed into the next, so the result is the target with
/// all of the tools taken out of it.
pub fn subtract_all(target: &Brush, tools: &[Brush]) -> Vec<Brush> {
    let mut pieces = vec![target.clone()];
    for tool in tools {
        pieces = pieces
            .iter()
            .flat_map(|piece| subtract(piece, tool))
            .collect();
        if pieces.is_empty() {
            break;
        }
    }
    pieces
}

/// Turns a face plane around, so that clipping by it keeps what was in front of it.
fn flip(plane: &Plane3d) -> Plane3d {
    plane.flip()
}
