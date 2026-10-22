//! Vector math for Heron Brush.
//!
//! Const-generic vector and matrix operations, geometric primitives, intersections and numeric
//! utilities.
//!
//! ## Numeric conventions
//!
//! Dimensions use const generics. [`scalar::Float`] supplies tolerance constants; aggregate
//! functions accept iterators or slices.

#![forbid(unsafe_code)]

pub mod abstract_line;
pub mod approx;
pub mod bbox;
pub mod bezier_surface;
pub mod convex_hull;
pub mod distance;
pub mod intersection;
pub mod line;
pub mod mat;
pub mod mat_ext;
pub mod plane;
pub mod polygon;
pub mod quat;
pub mod ray;
pub mod scalar;
pub mod segment;
pub mod util;
pub mod vec;

pub use scalar::{Float, Num};
pub use vec::{Vec2d, Vec2f, Vec2i, Vec3d, Vec3f, Vec3i, Vec4d, Vec4f, Vec4i, Vector};
