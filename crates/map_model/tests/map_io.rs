//! Tests for reading and writing `.map` files.
//!
//! The sample below is the shape an editor-written file takes, matching the example
//! in the engine's own format notes.

use geometry_math::vec::{Vec2f, Vec3d};
use map_model::io::map_reader::{read_map, MapFile};
use map_model::io::map_writer::write_map;
use map_model::map_format::MapFormat;
use map_model::uv::UvCoordSystem;

const VALVE_SAMPLE: &str = r#"// Game: heron_brush-fps
// Format: Valve
// entity 0
{
"mapversion" "220"
"classname" "worldspawn"
"_tb_textures" "textures"
// brush 0
{
( -512 -512 -32 ) ( -512 -511 -32 ) ( -512 -512 -31 ) dirt [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -512 -512 -32 ) ( -512 -512 -31 ) ( -511 -512 -32 ) dirt [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -512 -512 -32 ) ( -511 -512 -32 ) ( -512 -511 -32 ) dirt [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 512 512 32 ) ( 512 513 32 ) ( 513 512 32 ) dirt [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 512 512 32 ) ( 513 512 32 ) ( 512 512 33 ) dirt [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 512 512 32 ) ( 512 512 33 ) ( 512 513 32 ) dirt [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
// entity 1
{
"classname" "info_player_start"
"origin" "0 64 24"
}
"#;

const STANDARD_SAMPLE: &str = r#"// Game: heron_brush-fps
// Format: Standard
{
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone 0 0 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone 0 0 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone 0 0 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone 0 0 0 1 1
}
}
"#;

fn read(source: &str) -> MapFile {
    read_map(source, None).expect("the sample parses")
}

/// Clears the recorded line numbers so two readings of the same map can be compared.
///
/// A round trip does not preserve line numbers — the writer emits its own `// entity`
/// comments — and they are diagnostics rather than part of the document.
fn without_line_numbers(mut map: MapFile) -> MapFile {
    for entity in &mut map.entities {
        entity.line = 0;
        for brush in &mut entity.brushes {
            brush.line = 0;
            for face in &mut brush.faces {
                face.line = 0;
            }
        }
        for patch in &mut entity.patches {
            patch.line = 0;
        }
    }
    map
}

#[test]
fn the_header_comments_are_read() {
    let map = read(VALVE_SAMPLE);
    assert_eq!(map.game.as_deref(), Some("heron_brush-fps"));
    assert_eq!(map.format, MapFormat::Valve);
    assert!(map.warnings.is_empty(), "{:?}", map.warnings);
}

#[test]
fn entities_and_properties_are_read_in_order() {
    let map = read(VALVE_SAMPLE);
    assert_eq!(map.entities.len(), 2);

    let world = &map.entities[0];
    assert_eq!(world.classname(), Some("worldspawn"));
    assert_eq!(world.property("mapversion"), Some("220"));
    assert_eq!(world.property("_tb_textures"), Some("textures"));
    assert_eq!(world.property("nope"), None);

    let spawn = &map.entities[1];
    assert_eq!(spawn.classname(), Some("info_player_start"));
    assert_eq!(spawn.property("origin"), Some("0 64 24"));
    assert!(spawn.brushes.is_empty());
}

#[test]
fn a_valve_face_keeps_its_uv_axes() {
    let map = read(VALVE_SAMPLE);
    let face = &map.entities[0].brushes[0].faces[0];

    assert_eq!(face.material_name, "dirt");
    assert_eq!(
        face.uv_coord_system,
        UvCoordSystem::Parallel {
            u_axis: Vec3d::new(0.0, 1.0, 0.0),
            v_axis: Vec3d::new(0.0, 0.0, -1.0),
        }
    );
    assert_eq!(face.uv_attributes.offset, Vec2f::zero());
    assert_eq!(face.uv_attributes.scale, Vec2f::new(1.0, 1.0));
    assert_eq!(face.uv_attributes.rotation, 0.0);
}

#[test]
fn a_standard_face_uses_the_paraxial_system() {
    let map = read(STANDARD_SAMPLE);
    assert_eq!(map.format, MapFormat::Standard);

    let face = &map.entities[0].brushes[0].faces[0];
    assert_eq!(face.material_name, "stone");
    assert_eq!(face.uv_coord_system, UvCoordSystem::Paraxial);
}

#[test]
fn the_format_is_recognised_from_mapversion_when_the_header_is_missing() {
    let without_header = VALVE_SAMPLE
        .lines()
        .filter(|line| !line.starts_with("// Format:"))
        .collect::<Vec<_>>()
        .join("\n");

    let map = read_map(&without_header, None).expect("it still parses");
    assert_eq!(map.format, MapFormat::Valve);
}

#[test]
fn an_explicit_format_overrides_the_header() {
    let map = read_map(VALVE_SAMPLE, Some(MapFormat::Quake2Valve)).expect("it parses");
    assert_eq!(map.format, MapFormat::Quake2Valve);
}

#[test]
fn a_surface_flag_trio_with_no_header_makes_it_a_quake2_dialect() {
    // Standard-shaped faces, but each carries the `content surface value` trio.
    let source = "\
{
\"classname\" \"worldspawn\"
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) stone 0 0 0 1 1 0 8 0
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) stone 0 0 0 1 1 0 0 0
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) stone 0 0 0 1 1 0 0 0
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) stone 0 0 0 1 1 0 0 0
}
}
";
    let map = read_map(source, None).expect("it parses");
    assert_eq!(map.format, MapFormat::Quake2);

    // With bracketed UV axes it is the Valve flavour of Quake 2 instead.
    let valve = source.replace("stone 0 0 0 1 1", "stone [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1");
    let map = read_map(&valve, None).expect("it parses");
    assert_eq!(map.format, MapFormat::Quake2Valve);
}

#[test]
fn the_brush_faces_have_outward_normals() {
    let map = read(VALVE_SAMPLE);
    let brush = &map.entities[0].brushes[0];
    assert_eq!(brush.faces.len(), 6);

    // The brush spans -512..512 in x and y and -32..32 in z, so the origin is inside it
    // and must therefore be behind every face.
    for (index, face) in brush.faces.iter().enumerate() {
        assert!(
            face.plane.point_distance(Vec3d::zero()) < 0.0,
            "face {index} has its normal pointing inwards"
        );
    }
}

#[test]
fn a_brush_read_from_a_file_builds_a_polyhedron() {
    use geometry_math::bbox::Bbox3d;
    use geometry_math::scalar::Float;
    use map_model::polyhedron::Polyhedron;

    let map = read(VALVE_SAMPLE);
    let planes: Vec<_> = map.entities[0].brushes[0]
        .faces
        .iter()
        .map(|f| f.plane)
        .collect();

    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let built = Polyhedron::from_planes(&world, &planes, f64::ALMOST_ZERO)
        .expect("the planes enclose a brush");
    let polyhedron = built.polyhedron().expect("the brush is not empty");

    polyhedron.check_invariant().expect("a valid mesh");
    assert_eq!(polyhedron.face_count(), 6);
    assert_eq!(polyhedron.vertex_count(), 8);

    // Clipping interpolates along the cut edges, so the corners land within a rounding
    // error of where the planes say rather than exactly on it.
    let bounds = polyhedron.bounds();
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::new(-512.0, -512.0, -32.0), 1e-9);
    geometry_math::assert_approx_eq!(bounds.max, Vec3d::new(512.0, 512.0, 32.0), 1e-9);
}

/* ========== the optional pieces the format allows ========== */

#[test]
fn surface_flags_are_read_when_present() {
    let source = r#"// Format: Quake2 (Valve)
{
"classname" "worldspawn"
{
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) lava [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1 8 16 300
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) lava [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
"#;
    let map = read(source);
    let faces = &map.entities[0].brushes[0].faces;

    let surface = faces[0].surface_attributes.expect("the trio was written");
    assert_eq!(surface.content_flags, 8);
    assert_eq!(surface.surface_flags, 16);
    assert_eq!(surface.value, 300);

    // The trio is optional even within a format that allows it.
    assert_eq!(faces[1].surface_attributes, None);
}

#[test]
fn a_brush_primitive_is_read_for_its_geometry_with_default_uv() {
    // A 32-unit cube written as Quake 3 `brushDef`: three points, then the 2x3 texture
    // matrix, then the material and the flags.
    let source = r#"// Game: Quake 3
{
"classname" "worldspawn"
{
brushDef
{
( -16 -16 -16 ) ( -16 16 -16 ) ( -16 -16 16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
( -16 -16 -16 ) ( -16 -16 16 ) ( 16 -16 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
( -16 -16 -16 ) ( 16 -16 -16 ) ( -16 16 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
( 16 16 16 ) ( 16 -16 16 ) ( -16 16 16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
( 16 16 16 ) ( -16 16 16 ) ( 16 16 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
( 16 16 16 ) ( 16 16 -16 ) ( 16 -16 16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) stone 0 0 0
}
}
}
"#;
    let mut map = read(source);
    {
        let faces = &map.entities[0].brushes[0].faces;
        assert_eq!(faces.len(), 6, "every face was read");
        assert_eq!(faces[0].material_name, "stone");
        // Until it is resolved, the face wears the default paraxial alignment but keeps
        // the matrix it was written with.
        assert_eq!(faces[0].uv_coord_system, UvCoordSystem::Paraxial);
        assert_eq!(
            faces[0].uv_attributes,
            map_model::uv::UvAttributes::default()
        );
        assert_eq!(
            faces[0].primitive_uv,
            Some(Box::new([[0.0078125, 0.0, 0.0], [0.0, 0.0078125, 0.0]]))
        );
    }

    // One warning, whatever the brush count, and it names the format.
    assert_eq!(map.warnings.len(), 1);
    assert!(
        map.warnings[0].message.contains("brushDef"),
        "{:?}",
        map.warnings[0]
    );

    // Resolving with the material's pixel size turns the matrix into parallel UV axes.
    // The `+X` face (index 5) has normal (1,0,0); its axis base is world +Y and world -Z,
    // and a 1/128 matrix with a 128px material leaves the axes unit length.
    let face = &mut map.entities[0].brushes[0].faces[5];
    face.resolve_primitive_uv(geometry_math::vec::Vec2f::new(128.0, 128.0));
    assert!(face.primitive_uv.is_none(), "the matrix is spent");
    match face.uv_coord_system {
        UvCoordSystem::Parallel { u_axis, v_axis } => {
            geometry_math::assert_approx_eq!(u_axis, Vec3d::new(0.0, 1.0, 0.0), 1e-9);
            geometry_math::assert_approx_eq!(v_axis, Vec3d::new(0.0, 0.0, -1.0), 1e-9);
        }
        other => panic!("face 5 is {other:?}, not parallel"),
    }
    let uv = face.uv_coords(Vec3d::new(16.0, 8.0, 4.0));
    geometry_math::assert_approx_eq!(uv, geometry_math::vec::Vec2f::new(8.0, -4.0), 1e-4);

    // The geometry is right: the six planes clip to the cube they describe.
    build_cube_and_check(&map.entities[0].brushes[0].faces, -16.0, 16.0);
}

#[test]
fn a_doom3_brush_primitive_reads_its_planes() {
    // The same cube as Doom 3 `brushDef3`: each face is a plane `( nx ny nz d )` with
    // `n . p + d = 0`, then the texture matrix, then a quoted material.
    let source = r#"// Game: Doom 3
{
"classname" "worldspawn"
{
brushDef3
{
( -1 0 0 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
( 1 0 0 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
( 0 -1 0 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
( 0 1 0 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
( 0 0 -1 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
( 0 0 1 -16 ) ( ( 0.0078125 0 0 ) ( 0 0.0078125 0 ) ) "textures/base/stone" 0 0 0
}
}
}
"#;
    let map = read(source);
    let faces = &map.entities[0].brushes[0].faces;
    assert_eq!(faces.len(), 6);
    assert_eq!(faces[5].material_name, "textures/base/stone");

    // Face 5's plane is `( 0 0 1 -16 )`, so its outward normal points up and it sits at
    // z = 16 -- which is what the synthesized points have to carry.
    let up = faces[5].plane;
    geometry_math::assert_approx_eq!(up.normal, Vec3d::new(0.0, 0.0, 1.0), 1e-9);
    assert!(
        (up.distance - 16.0).abs() < 1e-9,
        "z = 16, got {}",
        up.distance
    );

    build_cube_and_check(faces, -16.0, 16.0);
}

/// Clips the world box by the given faces' planes and checks it comes out as the cube
/// from `lo` to `hi`.
fn build_cube_and_check(faces: &[map_model::brush_face::BrushFace], lo: f64, hi: f64) {
    use geometry_math::bbox::Bbox3d;
    use geometry_math::scalar::Float;
    use map_model::polyhedron::Polyhedron;

    let planes: Vec<_> = faces.iter().map(|f| f.plane).collect();
    let world = Bbox3d::from_scalars(-16384.0, 16384.0);
    let built = Polyhedron::from_planes(&world, &planes, f64::ALMOST_ZERO)
        .expect("the planes enclose a brush");
    let polyhedron = built.polyhedron().expect("the brush is not empty");
    polyhedron.check_invariant().expect("a valid mesh");
    assert_eq!(polyhedron.face_count(), 6);
    assert_eq!(polyhedron.vertex_count(), 8);
    let bounds = polyhedron.bounds();
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::new(lo, lo, lo), 1e-6);
    geometry_math::assert_approx_eq!(bounds.max, Vec3d::new(hi, hi, hi), 1e-6);
}

#[test]
fn a_bezier_patch_is_read() {
    let source = r#"// Format: Valve
{
"classname" "worldspawn"
{
patchDef2
{
curve
( 3 3 0 0 0 )
(
( ( 0 0 0 0 0 ) ( 0 4 0 0 0.5 ) ( 0 8 0 0 1 ) )
( ( 4 0 4 0.5 0 ) ( 4 4 4 0.5 0.5 ) ( 4 8 4 0.5 1 ) )
( ( 8 0 0 1 0 ) ( 8 4 0 1 0.5 ) ( 8 8 0 1 1 ) )
)
}
}
}
"#;
    let map = read(source);
    let patch = &map.entities[0].patches[0];

    assert_eq!(patch.material_name, "curve");
    assert_eq!(patch.rows, 3);
    assert_eq!(patch.columns, 3);
    assert_eq!(patch.control_points.len(), 9);
    // Stored row by row: the first row is the first point of each column.
    assert_eq!(patch.control_points[0].position, Vec3d::new(0.0, 0.0, 0.0));
    assert_eq!(patch.control_points[1].position, Vec3d::new(4.0, 0.0, 4.0));
    assert_eq!(patch.control_points[4].position, Vec3d::new(4.0, 4.0, 4.0));
}

/* ========== round trips ========== */

#[test]
fn a_valve_map_survives_a_round_trip() {
    let original = read(VALVE_SAMPLE);
    let written = write_map(&original);
    let reread = read_map(&written, None).expect("what we wrote parses");

    assert_eq!(reread.format, original.format);
    assert_eq!(reread.game, original.game);
    assert_eq!(
        without_line_numbers(reread).entities,
        without_line_numbers(original).entities
    );
}

#[test]
fn a_standard_map_survives_a_round_trip() {
    let original = read(STANDARD_SAMPLE);
    let written = write_map(&original);
    let reread = read_map(&written, None).expect("what we wrote parses");

    assert_eq!(reread.format, original.format);
    assert_eq!(
        without_line_numbers(reread).entities,
        without_line_numbers(original).entities
    );
}

#[test]
fn coordinates_are_written_in_the_editors_own_frame() {
    // The writer must not convert to any engine's axis convention: a file saved and
    // loaded again has to be unchanged.
    let original = read(VALVE_SAMPLE);
    let written = write_map(&original);
    assert!(
        written.contains("( -512 -512 -32 )"),
        "the first point was not written back unchanged:\n{written}"
    );
}

#[test]
fn whole_numbers_are_written_without_a_decimal_point() {
    let written = write_map(&read(STANDARD_SAMPLE));
    assert!(
        !written.contains("-64.0"),
        "wrote a needless decimal:\n{written}"
    );
    assert!(written.contains("( -64 -64 -16 )"), "{written}");
}

#[test]
fn quotes_in_a_property_value_are_escaped() {
    let source = "{\n\"classname\" \"worldspawn\"\n\"message\" \"say \\\"hi\\\"\"\n}\n";
    let map = read_map(source, Some(MapFormat::Standard)).expect("it parses");
    assert_eq!(map.entities[0].property("message"), Some("say \"hi\""));

    let written = write_map(&map);
    let reread = read_map(&written, Some(MapFormat::Standard)).expect("it parses again");
    assert_eq!(reread.entities[0].property("message"), Some("say \"hi\""));
}

/* ========== errors ========== */

#[test]
fn an_unterminated_entity_is_an_error() {
    let error = read_map("{\n\"classname\" \"worldspawn\"\n", None).expect_err("no close brace");
    assert!(error.to_string().contains("unterminated"), "{error}");
}

#[test]
fn a_face_with_colinear_points_is_an_error() {
    let source =
        "{\n\"classname\" \"worldspawn\"\n{\n( 0 0 0 ) ( 1 0 0 ) ( 2 0 0 ) stone 0 0 0 1 1\n}\n}\n";
    let error = read_map(source, None).expect_err("the points are colinear");
    assert!(
        error.to_string().contains("do not define a plane"),
        "{error}"
    );
}

#[test]
fn an_unterminated_string_is_an_error() {
    let error = read_map("{\n\"classname\" \"worldspawn\n", None).expect_err("no close quote");
    assert!(error.to_string().contains("unterminated string"), "{error}");
}

#[test]
fn multistop_gradients_with_surface_material_survive_roundtrip() {
    use map_model::brush_face::FaceColor;
    for kind in ["linear", "radial"] {
        for alpha in ["", " 173"] {
            let comment = format!(" // heron_brush {kind} 30 3 0 204 255 232 0.5 65 190 181 1 25 126 126{alpha} pbr 32 70 12");
            let source = VALVE_SAMPLE
                .lines()
                .map(|line| {
                    if line.starts_with('(') {
                        format!("{line}{comment}\n")
                    } else {
                        format!("{line}\n")
                    }
                })
                .collect::<String>();
            let map = read_map(&source, None).unwrap();
            let face = &map.entities[0].brushes[0].faces[0];
            assert!(
                matches!(face.paint(), Some(FaceColor::MultiGradient { radial, alpha: a, .. })
                if radial == (kind == "radial") && a == if alpha.is_empty() {255} else {173})
            );
            assert_eq!(face.surface_material.unwrap().metallic, 70);
            let reopened = read_map(&write_map(&map), None).unwrap();
            let after = &reopened.entities[0].brushes[0].faces[0];
            assert_eq!(after.color, face.color);
            assert_eq!(after.surface_material, face.surface_material);
        }
    }
}

#[test]
fn pending_primitive_matrices_survive_all_map_dialects_and_face_metadata() {
    use map_model::brush_face::{
        ColorStop, FaceColor, GradientEnds, GradientStops, Rgb, SurfaceMaterial,
    };
    let matrix = [
        [1. / 127., -0.03125, 0.1234567890123456],
        [0.0625, 1. / 61., -3.75],
    ];
    let stops = GradientStops::new(&[
        ColorStop {
            position: 0.,
            color: Rgb::new(10, 20, 30),
        },
        ColorStop {
            position: 1.,
            color: Rgb::new(50, 60, 70),
        },
    ])
    .unwrap();
    for format in [
        MapFormat::Standard,
        MapFormat::Valve,
        MapFormat::Quake2,
        MapFormat::Quake2Valve,
    ] {
        for paint in [
            None,
            Some(FaceColor::solid(Rgb::new(10, 20, 30))),
            Some(FaceColor::MultiGradient {
                stops,
                radial: false,
                angle: 37.,
                alpha: 123,
                ends: None,
            }),
            Some(FaceColor::MultiGradient {
                stops,
                radial: true,
                angle: 0.,
                alpha: 255,
                ends: Some(GradientEnds {
                    start: [0.25, 0.5],
                    end: [1.125, -0.375],
                }),
            }),
        ] {
            for pbr in [false, true] {
                for hidden in [false, true] {
                    let mut map = read(STANDARD_SAMPLE);
                    map.format = format;
                    let face = &mut map.entities[0].brushes[0].faces[0];
                    face.primitive_uv = Some(Box::new(matrix));
                    face.set_paint(paint);
                    face.surface_material = pbr.then_some(SurfaceMaterial {
                        roughness: 45,
                        metallic: 20,
                        emission: 80,
                    });
                    face.export_hidden = hidden;
                    let expected = face.clone();
                    let written = write_map(&map);
                    assert!(written.contains("// heron_brush format 3"));
                    let reopened = read(&written);
                    let actual = &reopened.entities[0].brushes[0].faces[0];
                    assert_eq!(actual, &expected, "{format:?}, {paint:?}, {pbr}, {hidden}");
                    assert_eq!(write_map(&reopened), written, "saving twice is stable");
                    let mut before = expected;
                    before.resolve_primitive_uv(Vec2f::new(128., 64.));
                    let mut after = actual.clone();
                    after.resolve_primitive_uv(Vec2f::new(128., 64.));
                    assert_eq!(after, before, "later material discovery keeps the mapping");
                }
            }
        }
    }
}

#[test]
fn malformed_or_foreign_pending_matrix_comments_do_not_override_face_uv() {
    for text in [
        "other_editor primitive_uv 1 0 0 0 1 0",
        "heron_brush primitive_uv 1 0 0",
        "heron_brush primitive_uv NaN 0 0 0 1 0",
        "heron_brush primitive_uv 1e999 0 0 0 1 0",
    ] {
        let source = STANDARD_SAMPLE.replacen(
            "stone 0 0 0 1 1\n",
            &format!("stone 0 0 0 1 1 // {text}\n"),
            1,
        );
        let parsed = read(&source);
        assert!(parsed.entities[0].brushes[0].faces[0]
            .primitive_uv
            .is_none());
        assert_eq!(
            parsed.entities[0].brushes[0].faces[0].uv_attributes,
            Default::default()
        );
    }
}

fn doom3_version_two_cube(class: &str, origin: &str) -> String {
    let mut out = format!("Version 2\n// Game: custom-doom3\n{{\n\"classname\" \"{class}\"\n\"origin\" \"{origin}\"\n{{\nbrushDef3\n{{\n");
    for plane in [
        "-1 0 0 -16",
        "1 0 0 -16",
        "0 -1 0 -16",
        "0 1 0 -16",
        "0 0 -1 -16",
        "0 0 2 -32",
    ] {
        out.push_str(&format!("( {plane} ) ( ( 0.0078125 0 0.25 ) ( 0 0.015625 -0.125 ) ) \"textures/base/stone\" 0 0 0\n"));
    }
    out.push_str("}\n}\n}\n");
    out
}

#[test]
fn doom3_version_two_reads_scaled_planes_local_origin_and_world_space_uvs() {
    use geometry_math::{bbox::Bbox3d, scalar::Float, vec};
    let source = doom3_version_two_cube("func_static", "128 64 32");
    let mut map = read(&source);
    assert_eq!(map.game.as_deref(), Some("custom-doom3"));
    let faces = &mut map.entities[0].brushes[0].faces;
    let planes: Vec<_> = faces.iter().map(|f| f.plane).collect();
    let built = map_model::polyhedron::Polyhedron::from_planes(
        &Bbox3d::from_scalars(-16384., 16384.),
        &planes,
        f64::ALMOST_ZERO,
    )
    .unwrap();
    let bounds = built.polyhedron().unwrap().bounds();
    geometry_math::assert_approx_eq!(bounds.min, Vec3d::new(112., 48., 16.), 1e-9);
    geometry_math::assert_approx_eq!(bounds.max, Vec3d::new(144., 80., 48.), 1e-9);
    for face in faces {
        let (s, t) = map_model::uv::primitive_axis_base(face.normal());
        let point = face.points[0];
        let local = point - Vec3d::new(128., 64., 32.);
        let origin = Vec3d::new(128., 64., 32.);
        // Doom 3 evaluates local geometry with an origin contribution in texVec.w.
        let expected = Vec2f::new(
            ((vec::dot(s, local) + vec::dot(s, origin)) / 128. + 0.25) as f32 * 128.,
            ((vec::dot(t, local) + vec::dot(t, origin)) / 64. - 0.125) as f32 * 64.,
        );
        face.resolve_primitive_uv(Vec2f::new(128., 64.));
        geometry_math::assert_approx_eq!(face.uv_coords(point), expected, 1e-4);
    }
    let world = read(&doom3_version_two_cube("worldspawn", "128 64 32"));
    build_cube_and_check(&world.entities[0].brushes[0].faces, -16., 16.);
    let alias = read(&source.replace("brushDef3", "brushDef2"));
    assert_eq!(alias.entities[0].brushes[0].faces[5].plane.distance, 48.);
}

#[test]
fn doom3_version_two_preserves_literal_strings_through_internal_save() {
    let source = doom3_version_two_cube("worldspawn", "0 0 0").replacen(
        "\"origin\"",
        "\"path\" \"C:\\\\assets\\\"\n\"origin\"",
        1,
    );
    let map = read(&source);
    assert_eq!(map.entities[0].property("path"), Some("C:\\\\assets\\"));
    let reopened = read(&write_map(&map));
    assert_eq!(
        reopened.entities[0].property("path"),
        map.entities[0].property("path")
    );
    assert_eq!(
        reopened.entities[0].brushes[0].faces,
        map.entities[0].brushes[0].faces
    );
}

#[test]
fn doom3_version_origin_and_plane_errors_are_explicit() {
    let source = doom3_version_two_cube("func_static", "128 64 32");
    for version in ["1", "3", "NaN"] {
        assert!(read_map(
            &source.replacen("Version 2", &format!("Version {version}"), 1),
            None
        )
        .is_err());
    }
    assert!(read_map(&source.replacen("Version 2", "Version\n2", 1), None).is_err());
    for origin in ["1 2", "1 2 3 4", "NaN 0 0", "1e999 0 0"] {
        assert!(read_map(&source.replace("128 64 32", origin), None).is_err());
    }
    for plane in ["0 0 0 -32", "0 0 1 NaN", "NaN 0 1 0"] {
        assert!(read_map(&source.replace("0 0 2 -32", plane), None).is_err());
    }
    let late = source.replacen("\"origin\" \"128 64 32\"\n", "", 1);
    let at = late.rfind('}').unwrap();
    let late = format!("{}\"origin\" \"128 64 32\"\n{}", &late[..at], &late[at..]);
    assert!(read_map(&late, None)
        .unwrap_err()
        .to_string()
        .contains("must precede"));
    let rotated = source.replacen(
        "\"origin\"",
        "\"rotation\" \"0 -1 0 1 0 0 0 0 1\"\n\"origin\"",
        1,
    );
    assert!(read(&rotated)
        .warnings
        .iter()
        .any(|w| w.message.contains("rotation")));
}

#[test]
fn doom3_native_writer_round_trips_local_planes_pending_uv_and_literal_strings() {
    let world =
        read(&doom3_version_two_cube("worldspawn", "128 64 32").replace("classname", "ClassName"));
    let world_written = map_model::io::map_writer::write_doom3_map(&world, &mut |_| None).unwrap();
    build_cube_and_check(
        &read(&world_written).entities[0].brushes[0].faces,
        -16.,
        16.,
    );
    let source = doom3_version_two_cube("func_static", "128 64 32");
    let mut map = read(&source);
    map.entities[0]
        .properties
        .push(("path".into(), "C:\\\\assets\\".into()));
    let written = map_model::io::map_writer::write_doom3_map(&map, &mut |_| {
        panic!("pending matrices already normalized")
    })
    .unwrap();
    assert!(written.starts_with("Version 2\n"));
    assert!(written.contains("\nbrushDef3\n"));
    assert!(!written.contains("primitive_uv"));
    let reopened = read(&written);
    assert_eq!(reopened.entities[0].properties, map.entities[0].properties);
    for (a, b) in map.entities[0].brushes[0]
        .faces
        .iter()
        .zip(&reopened.entities[0].brushes[0].faces)
    {
        geometry_math::assert_approx_eq!(a.plane.normal, b.plane.normal, 1e-9);
        assert!((a.plane.distance - b.plane.distance).abs() < 1e-9);
        assert_eq!(a.primitive_uv, b.primitive_uv);
    }
    map.entities[0]
        .properties
        .push(("rotation".into(), "0 -1 0 1 0 0 0 0 1".into()));
    assert!(
        map_model::io::map_writer::write_doom3_map(&map, &mut |_| None)
            .unwrap_err()
            .contains("rotation")
    );
    map.entities[0].properties.pop();
    map.entities[0]
        .properties
        .push(("bad".into(), "embedded \"quote".into()));
    assert!(
        map_model::io::map_writer::write_doom3_map(&map, &mut |_| None)
            .unwrap_err()
            .contains("strings")
    );
}
