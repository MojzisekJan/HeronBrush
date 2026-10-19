//! Small shared enumerations.
//!
//! [`Axis`] identifies a coordinate axis; [`Axis::index`] provides its numeric index.

/// Which side of a surface something is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// In front of the surface.
    Front,
    /// Behind the surface.
    Back,
    /// On both sides of the surface.
    Both,
}

/// A direction relative to a viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Away from the viewer.
    Forward,
    /// Towards the viewer.
    Backward,
    /// To the viewer's left.
    Left,
    /// To the viewer's right.
    Right,
    /// Above the viewer.
    Up,
    /// Below the viewer.
    Down,
}

/// One of the three axes a rotation can be about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RotationAxis {
    /// Rotation about the forward axis.
    Roll,
    /// Rotation about the sideways axis.
    Pitch,
    /// Rotation about the up axis.
    Yaw,
}

/// Where a point lies relative to a plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaneStatus {
    /// In front of the plane.
    Above,
    /// Behind the plane.
    Below,
    /// On the plane, within the epsilon used to classify it.
    Inside,
}

/// One of the three coordinate axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
    /// The Z axis.
    Z,
}

impl Axis {
    /// Returns the component index this axis corresponds to.
    #[inline]
    pub fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    /// Returns the axis for the given component index, or `None` if it is out of range.
    #[inline]
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Axis::X),
            1 => Some(Axis::Y),
            2 => Some(Axis::Z),
            _ => None,
        }
    }
}
