//! Which shape the draw tool makes, and the settings that shape it.
//!
//! An enum selects the brush generator and its parameters.

use crate::i18n::translation_key;
use map_model::brush::Brush;
use map_model::shapes::{self, CircleShape, ShapeError};
use geometry_math::bbox::Bbox3d;
use geometry_math::util::Axis;

/// A shape the draw tool can make.
///
/// The order is `DrawShapeToolExtensionKinds`, which is the order they appear in the
/// toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeKind {
    /// A box.
    #[default]
    Cuboid,
    /// A flight of steps.
    Stairs,
    /// A curved band of voussoirs, optionally with the corners filled in.
    Arch,
    /// A prism, hollow if asked for.
    Cylinder,
    /// A prism narrowing to a point.
    Cone,
    /// A sphere built from stacked rings.
    UvSphere,
    /// A sphere built by subdividing an icosahedron.
    IcoSphere,
    /// A closed shell assembled from convex brush cells.
    HollowSphere,
}

impl ShapeKind {
    /// Every shape, in the order the toolbar lists them.
    pub const ALL: [ShapeKind; 8] = [
        ShapeKind::Cuboid,
        ShapeKind::Stairs,
        ShapeKind::Arch,
        ShapeKind::Cylinder,
        ShapeKind::Cone,
        ShapeKind::UvSphere,
        ShapeKind::IcoSphere,
        ShapeKind::HollowSphere,
    ];

    /// Returns the name shown in the toolbar, from each extension's `name()`.
    pub fn name(self) -> &'static str {
        match self {
            ShapeKind::Cuboid => translation_key("Cuboid"),
            ShapeKind::Stairs => translation_key("Stairs"),
            ShapeKind::Arch => translation_key("Arch"),
            ShapeKind::Cylinder => translation_key("Cylinder"),
            ShapeKind::Cone => translation_key("Cone"),
            ShapeKind::UvSphere => translation_key("Spheroid (UV)"),
            ShapeKind::HollowSphere => translation_key("Hollow sphere"),
            ShapeKind::IcoSphere => translation_key("Spheroid (Icosahedron)"),
        }
    }

    /// Returns whether this shape follows a circle, and so takes the circle settings.
    pub fn is_circular(self) -> bool {
        matches!(
            self,
            ShapeKind::Arch | ShapeKind::Cylinder | ShapeKind::Cone | ShapeKind::UvSphere
        )
    }

    /// Returns whether this shape is laid out along a chosen axis.
    pub fn has_axis(self) -> bool {
        !matches!(self, ShapeKind::Cuboid | ShapeKind::IcoSphere | ShapeKind::HollowSphere)
    }
}

/// Which way a flight of stairs climbs.
///
/// `DrawShapeToolParameters::StairDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StairDirection {
    /// Climbing towards +X.
    #[default]
    PosX,
    /// Climbing towards -X.
    NegX,
    /// Climbing towards +Y.
    PosY,
    /// Climbing towards -Y.
    NegY,
}

impl StairDirection {
    /// Every direction, in the order the toolbar lists them.
    pub const ALL: [StairDirection; 4] = [
        StairDirection::PosX,
        StairDirection::NegX,
        StairDirection::PosY,
        StairDirection::NegY,
    ];

    /// Returns the label shown in the toolbar.
    pub fn name(self) -> &'static str {
        match self {
            StairDirection::PosX => "+X",
            StairDirection::NegX => "-X",
            StairDirection::PosY => "+Y",
            StairDirection::NegY => "-Y",
        }
    }

    /// `stairDirectionToAxis`: which axis the treads run along.
    fn axis(self) -> Axis {
        match self {
            StairDirection::PosY | StairDirection::NegY => Axis::Y,
            StairDirection::PosX | StairDirection::NegX => Axis::X,
        }
    }

    /// `isPositiveStairDirection`: whether the steps climb towards the axis's far end.
    fn is_positive(self) -> bool {
        matches!(self, StairDirection::PosX | StairDirection::PosY)
    }
}

/// How the vertices of a circular shape are placed, without the count they carry.
///
/// The toolbar picks the mode and the count separately, so that changing from eight sides
/// to a scalable circle and back does not lose the eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CircleMode {
    /// The edges touch the bounds.
    #[default]
    EdgeAligned,
    /// The vertices touch the bounds.
    VertexAligned,
    /// A circle that keeps its shape when scaled.
    Scalable,
}

impl CircleMode {
    /// Every mode, in the order the toolbar lists them.
    pub const ALL: [CircleMode; 3] = [
        CircleMode::EdgeAligned,
        CircleMode::VertexAligned,
        CircleMode::Scalable,
    ];

    /// Returns the label shown in the toolbar.
    pub fn name(self) -> &'static str {
        match self {
            CircleMode::EdgeAligned => translation_key("Edge aligned"),
            CircleMode::VertexAligned => translation_key("Vertex aligned"),
            CircleMode::Scalable => translation_key("Scalable"),
        }
    }
}

/// The settings every shape draws from, whether or not it uses them.
///
/// `DrawShapeToolParameters`, defaults included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeParameters {
    /// Which shape to make.
    pub kind: ShapeKind,
    /// For axis aligned shapes.
    pub axis: Axis,
    /// For circular shapes.
    pub circle_mode: CircleMode,
    /// How many sides an edge- or vertex-aligned circle has.
    pub num_sides: usize,
    /// How many times a scalable circle is refined.
    pub precision: usize,
    /// For hollow shapes.
    pub hollow: bool,
    /// How thick a hollow shape's wall is.
    pub thickness: f64,
    /// Radial thickness for the geodesic shell, separate from arch/cylinder settings.
    pub sphere_thickness: f64,
    /// Convex shell cells, in symmetric steps of four from 20 to 320.
    pub sphere_brushes: usize,
    /// For arch shapes.
    pub create_spandrel: bool,
    /// For UV spheres.
    pub num_rings: usize,
    /// For icospheres.
    pub accuracy: usize,
    /// For stair shapes.
    pub step_height: f64,
    /// Landing, turn and underside settings for stairs.
    pub stairs: shapes::StairOptions,
    /// Which way stairs climb.
    pub stair_direction: StairDirection,
}

impl Default for ShapeParameters {
    fn default() -> Self {
        Self {
            kind: ShapeKind::default(),
            axis: Axis::Z,
            circle_mode: CircleMode::EdgeAligned,
            num_sides: 8,
            precision: 0,
            hollow: false,
            thickness: 16.0,
            sphere_thickness: 8.0,
            sphere_brushes: 80,
            create_spandrel: false,
            num_rings: 8,
            accuracy: 1,
            step_height: 16.0,
            stairs: shapes::StairOptions::default(),
            stair_direction: StairDirection::default(),
        }
    }
}

impl ShapeParameters {
    /// Returns the circle the settings describe.
    pub fn circle_shape(&self) -> CircleShape {
        match self.circle_mode {
            CircleMode::EdgeAligned => CircleShape::EdgeAligned {
                num_sides: self.num_sides,
            },
            CircleMode::VertexAligned => CircleShape::VertexAligned {
                num_sides: self.num_sides,
            },
            CircleMode::Scalable => CircleShape::Scalable {
                precision: self.precision,
            },
        }
    }
}

/// Builds the brushes for the chosen shape, filling the given bounds.
///
/// Every extension's `createBrushes`, gathered into one function. The result is a list
/// because a hollow cylinder, an arch and a flight of stairs are each several brushes.
pub fn create_brushes(
    world_bounds: &Bbox3d,
    bounds: &Bbox3d,
    parameters: &ShapeParameters,
    material: &str,
) -> Result<Vec<Brush>, ShapeError> {
    let circle_shape = parameters.circle_shape();
    let axis = parameters.axis;

    match parameters.kind {
        ShapeKind::Cuboid => {
            shapes::create_cuboid(world_bounds, bounds, material).map(|brush| vec![brush])
        }

        ShapeKind::Stairs => shapes::create_stairs_options(
            world_bounds,
            bounds,
            parameters.step_height,
            parameters.stair_direction.axis(),
            parameters.stair_direction.is_positive(),
            parameters.stairs,
            material,
        ),

        ShapeKind::Arch => {
            let mut brushes = shapes::create_arch(
                world_bounds,
                bounds,
                parameters.thickness,
                circle_shape,
                axis,
                material,
            )?;
            if parameters.create_spandrel {
                brushes.extend(shapes::create_spandrel_for_arch(
                    world_bounds,
                    bounds,
                    circle_shape,
                    axis,
                    material,
                )?);
            }
            Ok(brushes)
        }

        ShapeKind::Cylinder => {
            if parameters.hollow {
                shapes::create_hollow_cylinder(
                    world_bounds,
                    bounds,
                    parameters.thickness,
                    circle_shape,
                    axis,
                    material,
                )
            } else {
                shapes::create_cylinder(world_bounds, bounds, circle_shape, axis, material)
                    .map(|brush| vec![brush])
            }
        }

        ShapeKind::Cone => {
            shapes::create_cone(world_bounds, bounds, circle_shape, axis, material)
                .map(|brush| vec![brush])
        }

        ShapeKind::UvSphere => shapes::create_uv_sphere(
            world_bounds,
            bounds,
            circle_shape,
            parameters.num_rings,
            axis,
            material,
        )
        .map(|brush| vec![brush]),

        ShapeKind::HollowSphere => shapes::create_hollow_sphere(
            world_bounds, bounds, parameters.sphere_thickness, parameters.sphere_brushes, material,
        ),
        ShapeKind::IcoSphere => {
            shapes::create_ico_sphere(world_bounds, bounds, parameters.accuracy, material)
                .map(|brush| vec![brush])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::vec::Vec3d;

    fn world() -> Bbox3d {
        Bbox3d::from_scalars(-16384.0, 16384.0)
    }

    fn bounds() -> Bbox3d {
        Bbox3d::new(
            Vec3d::new(-64.0, -64.0, 0.0),
            Vec3d::new(64.0, 64.0, 64.0),
        )
    }

    #[test]
    fn hollow_sphere_is_the_eighth_shape_and_uses_wall_settings() {
        let parameters=std::rc::Rc::new(std::cell::RefCell::new(ShapeParameters::default()));
        assert!(crate::commands::select_shape_kind(&parameters,8));
        assert_eq!(parameters.borrow().kind,ShapeKind::HollowSphere);
        assert!(!ShapeKind::HollowSphere.has_axis());
        assert!(!ShapeKind::HollowSphere.is_circular());
        let brushes=create_brushes(&world(),&bounds(),&parameters.borrow(),"stone").unwrap();
        assert_eq!(brushes.len(),80);
        let initial_depth=Bbox3d::new(Vec3d::new(-64.0,-64.0,-16.0),Vec3d::new(64.0,64.0,16.0));
        assert_eq!(create_brushes(&world(),&initial_depth,&parameters.borrow(),"stone").unwrap().len(),80);
        parameters.borrow_mut().sphere_brushes=24;
        assert_eq!(create_brushes(&world(),&bounds(),&parameters.borrow(),"stone").unwrap().len(),24);
        parameters.borrow_mut().sphere_thickness=32.0;
        assert!(create_brushes(&world(),&bounds(),&parameters.borrow(),"stone").is_err());
    }

    #[test]
    fn every_shape_can_be_built_from_its_defaults() {
        for kind in ShapeKind::ALL {
            let parameters = ShapeParameters {
                kind,
                ..Default::default()
            };
            let brushes = create_brushes(&world(), &bounds(), &parameters, "stone")
                .unwrap_or_else(|e| panic!("{}: {e}", kind.name()));
            assert!(!brushes.is_empty(), "{} made nothing", kind.name());
        }
    }

    #[test]
    fn every_shape_can_be_built_in_every_circle_mode() {
        for kind in ShapeKind::ALL.into_iter().filter(|kind| kind.is_circular()) {
            for circle_mode in CircleMode::ALL {
                let parameters = ShapeParameters {
                    kind,
                    circle_mode,
                    ..Default::default()
                };
                let brushes = create_brushes(&world(), &bounds(), &parameters, "stone")
                    .unwrap_or_else(|e| panic!("{} as {}: {e}", kind.name(), circle_mode.name()));
                assert!(!brushes.is_empty());
            }
        }
    }

    #[test]
    fn a_hollow_cylinder_is_several_brushes_where_a_solid_one_is_not() {
        let solid = ShapeParameters {
            kind: ShapeKind::Cylinder,
            ..Default::default()
        };
        let hollow = ShapeParameters {
            hollow: true,
            ..solid
        };

        assert_eq!(
            create_brushes(&world(), &bounds(), &solid, "stone").map(|b| b.len()),
            Ok(1)
        );
        assert_eq!(
            create_brushes(&world(), &bounds(), &hollow, "stone").map(|b| b.len()),
            Ok(8)
        );
    }

    #[test]
    fn a_spandrel_adds_brushes_to_an_arch() {
        let plain = ShapeParameters {
            kind: ShapeKind::Arch,
            ..Default::default()
        };
        let with_spandrel = ShapeParameters {
            create_spandrel: true,
            ..plain
        };

        let plain_count = create_brushes(&world(), &bounds(), &plain, "stone")
            .expect("an arch")
            .len();
        let spandrel_count = create_brushes(&world(), &bounds(), &with_spandrel, "stone")
            .expect("an arch")
            .len();
        assert_eq!(spandrel_count, plain_count + 2);
    }

    #[test]
    fn stairs_climb_the_way_they_are_pointed() {
        let up = ShapeParameters {
            kind: ShapeKind::Stairs,
            stair_direction: StairDirection::PosX,
            ..Default::default()
        };
        let down = ShapeParameters {
            stair_direction: StairDirection::NegX,
            ..up
        };

        let lowest_when_up = create_brushes(&world(), &bounds(), &up, "stone").expect("stairs")[0]
            .bounds()
            .min
            .x();
        let lowest_when_down =
            create_brushes(&world(), &bounds(), &down, "stone").expect("stairs")[0]
                .bounds()
                .max
                .x();

        geometry_math::assert_approx_eq!(lowest_when_up, -64.0, 1e-6);
        geometry_math::assert_approx_eq!(lowest_when_down, 64.0, 1e-6);
    }

    #[test]
    fn only_the_shapes_that_follow_a_circle_take_the_circle_settings() {
        assert!(ShapeKind::Cylinder.is_circular());
        assert!(ShapeKind::Arch.is_circular());
        assert!(!ShapeKind::Cuboid.is_circular());
        assert!(!ShapeKind::IcoSphere.is_circular());

        assert!(ShapeKind::Cylinder.has_axis());
        assert!(!ShapeKind::Cuboid.has_axis());
    }
}
