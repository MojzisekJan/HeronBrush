//! The bounds of the editable world.
//!
//! Every editor needs a finite world to draw and to clamp coordinates against, but how
//! large it is belongs to the target game rather than to the editor. This keeps the limit
//! in one place so a per-game configuration can set it later, instead of scattering a
//! magic number through the views.

/// The extent of the editable world in each direction from the origin, in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBounds {
    half_extent: f64,
}

/// The default half-extent, chosen because 16384 is what most Quake-lineage engines and
/// their tooling assume. It is only a default; a game with a different coordinate scale
/// should set its own.
pub const DEFAULT_HALF_EXTENT: f64 = 16384.0;

impl Default for WorldBounds {
    fn default() -> Self {
        Self::new(DEFAULT_HALF_EXTENT)
    }
}

impl WorldBounds {
    /// Creates world bounds extending `half_extent` units in each direction from the
    /// origin.
    ///
    /// # Panics
    ///
    /// Panics if `half_extent` is not positive.
    pub fn new(half_extent: f64) -> Self {
        assert!(half_extent > 0.0, "the world must have a positive extent");
        Self { half_extent }
    }

    /// Returns the extent in each direction from the origin.
    pub fn half_extent(&self) -> f64 {
        self.half_extent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_extent_is_what_was_given() {
        assert_eq!(WorldBounds::new(100.0).half_extent(), 100.0);
        assert_eq!(WorldBounds::default().half_extent(), DEFAULT_HALF_EXTENT);
    }

    #[test]
    #[should_panic(expected = "positive extent")]
    fn a_world_must_have_a_positive_extent() {
        WorldBounds::new(0.0);
    }
}
