//! Stairs with landings, L/U layouts and an optional continuous sloped underside.
use super::*;
/// The footprint of two flights around a landing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StairLayout {
    /// A straight flight, optionally interrupted by a landing.
    #[default]
    Straight,
    /// A left turn as seen while ascending.
    Left,
    /// A right turn as seen while ascending.
    Right,
    /// Two parallel flights with a return landing.
    Return,
}
/// Dimensions shared by the advanced stair layouts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StairOptions {
    /// The landing arrangement.
    pub layout: StairLayout,
    /// Width of each flight in a turned layout.
    pub width: f64,
    /// Middle landing length for a straight layout; zero omits it.
    pub landing: f64,
    /// Vertical underside thickness; zero fills each tread down to the base.
    pub slab: f64,
}
impl Default for StairOptions {
    fn default() -> Self {
        Self {
            layout: StairLayout::Straight,
            width: 32.0,
            landing: 0.0,
            slab: 0.0,
        }
    }
}

/// Produces touching convex treads and a landing, bounded before allocating cells.
pub fn create_stairs_options(
    world: &Bbox3d,
    bounds: &Bbox3d,
    step_height: f64,
    axis: Axis,
    ascending: bool,
    options: StairOptions,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    let invalid = || {
        ShapeError::InvalidParameter("The stair dimensions must be finite, positive and leave room for both flights and the landing.")
    };
    let size = bounds.size();
    if axis == Axis::Z
        || size.v.iter().any(|n| !n.is_finite() || *n <= 0.0)
        || !step_height.is_finite()
        || step_height <= 0.0
        || !options.width.is_finite()
        || options.width <= 0.0
        || !options.landing.is_finite()
        || options.landing < 0.0
        || !options.slab.is_finite()
        || options.slab < 0.0
    {
        return Err(invalid());
    }
    let count = (size.z() / step_height).ceil().max(1.0);
    if count > 512.0 {
        return Err(ShapeError::InvalidParameter("Use at most 512 steps."));
    }
    if options.layout == StairLayout::Straight && options.landing == 0.0 && options.slab == 0.0 {
        return create_stairs(world, bounds, step_height, axis, ascending, material);
    }
    let count = (count as usize).max(2);
    let first = count / 2;
    let second = count - first;
    let rise = size.z() / count as f64;
    let u = axis.index();
    let v = 1 - u;
    let run = size[u];
    let side = size[v];
    let sign = if ascending { 1.0 } else { -1.0 };
    let side_sign = (if u == 0 { 1.0 } else { -1.0 })
        * sign
        * if options.layout == StairLayout::Right {
            -1.0
        } else {
            1.0
        };
    let point = |r: f64, s: f64, z: f64| {
        let mut p = bounds.min;
        p[u] = if sign > 0.0 {
            bounds.min[u] + r
        } else {
            bounds.max[u] - r
        };
        p[v] = if side_sign > 0.0 {
            bounds.min[v] + s
        } else {
            bounds.max[v] - s
        };
        p[2] += z;
        p
    };
    let mut result = Vec::with_capacity(count + 1);
    let mut flight = |origin: (f64, f64),
                      direction: (f64, f64),
                      across: (f64, f64),
                      length: f64,
                      width: f64,
                      n: usize,
                      height: f64|
     -> Result<(), ShapeError> {
        for i in 0..n {
            let start = length * i as f64 / n as f64;
            let end = length * (i + 1) as f64 / n as f64;
            let top = height + rise * (i + 1) as f64;
            let lower = [
                if options.slab > 0.0 {
                    height + rise * i as f64 - options.slab
                } else {
                    0.0
                },
                if options.slab > 0.0 {
                    top - options.slab
                } else {
                    0.0
                },
            ];
            let mut points = Vec::with_capacity(8);
            for (t, bottom) in [(start, lower[0]), (end, lower[1])] {
                for w in [0.0, width] {
                    for z in [bottom, top] {
                        points.push(point(
                            origin.0 + direction.0 * t + across.0 * w,
                            origin.1 + direction.1 * t + across.1 * w,
                            z,
                        ));
                    }
                }
            }
            let raw = create_brush(world, &points, material)?;
            let plane = geometry_math::plane::Plane3d::from_anchor_and_normal(
                bounds.min,
                Vec3d::new(0.0, 0.0, -1.0),
            );
            let brush = match raw.clip(&plane) {
                crate::brush::ClipOutcome::Clipped(b) => b,
                crate::brush::ClipOutcome::Unchanged => raw,
                _ => return Err(invalid()),
            };
            result.push(brush);
        }
        Ok(())
    };
    let (landing_origin, landing_size) = if options.layout == StairLayout::Straight {
        if options.landing >= run {
            return Err(invalid());
        }
        let before = (run - options.landing) * first as f64 / count as f64;
        let after = run - options.landing - before;
        flight((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), before, side, first, 0.0)?;
        flight(
            (before + options.landing, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            after,
            side,
            second,
            first as f64 * rise,
        )?;
        ((before, 0.0), (options.landing, side))
    } else {
        let width = options.width;
        if width >= run
            || width >= side
            || (options.layout == StairLayout::Return && 2.0 * width > side)
        {
            return Err(invalid());
        }
        flight(
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            run - width,
            width,
            first,
            0.0,
        )?;
        if options.layout == StairLayout::Return {
            flight(
                (run - width, side - width),
                (-1.0, 0.0),
                (0.0, 1.0),
                run - width,
                width,
                second,
                first as f64 * rise,
            )?;
            ((run - width, 0.0), (width, side))
        } else {
            flight(
                (run - width, width),
                (0.0, 1.0),
                (1.0, 0.0),
                side - width,
                width,
                second,
                first as f64 * rise,
            )?;
            ((run - width, 0.0), (width, width))
        }
    };
    if landing_size.0 > 0.0 {
        let top = first as f64 * rise;
        let bottom = if options.slab > 0.0 {
            (top - options.slab).max(0.0)
        } else {
            0.0
        };
        let points: Vec<_> = [0.0, landing_size.0]
            .into_iter()
            .flat_map(|r| {
                [0.0, landing_size.1].into_iter().flat_map(move |s| {
                    [bottom, top].map(|z| point(landing_origin.0 + r, landing_origin.1 + s, z))
                })
            })
            .collect();
        result.push(create_brush(world, &points, material)?);
    }
    if result
        .iter()
        .flat_map(|b| b.vertex_positions())
        .any(|p| !world.contains_point(p))
    {
        return Err(ShapeError::InvalidParameter(
            "The stairs do not fit the world bounds.",
        ));
    }
    Ok(result)
}
