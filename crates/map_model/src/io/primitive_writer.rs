//! Quake III brush primitives, with normalized texture matrices.
use super::*;
use geometry_math::vec;

/// Writes native Quake III `brushDef` brushes and patches.
/// Pending normalized matrices need no texture lookup. Other faces require actual
/// material dimensions; unavailable or invalid dimensions return an error.
/// Source: id-Software/Quake-III-Arena/q3map/map.c and brush_primit.c.
pub fn write_quake3_map(
    map: &MapFile,
    material_size: &mut dyn FnMut(&str) -> Option<Vec2f>,
) -> Result<String, String> {
    write_primitive_map(map, material_size, false)
}

/// Writes Doom 3 Version 2 maps with entity-local `brushDef3` planes.
/// Texture dimensions follow the Quake III writer's contract. Non-identity
/// brush-entity rotations and strings unrepresentable by idTech4 are rejected.
pub fn write_doom3_map(
    map: &MapFile,
    material_size: &mut dyn FnMut(&str) -> Option<Vec2f>,
) -> Result<String, String> {
    write_primitive_map(map, material_size, true)
}

fn doom3_string(value: &str) -> Result<&str, String> {
    if value.chars().any(|c| c == '"' || c.is_control()) {
        return Err("Doom 3 strings cannot contain quotes or control characters".into());
    }
    Ok(value)
}

fn doom3_origin(entity: &Entity) -> Result<Vec3d, String> {
    if entity
        .properties
        .iter()
        .rev()
        .find(|(key, _)| key.eq_ignore_ascii_case("classname"))
        .is_some_and(|(_, name)| name.eq_ignore_ascii_case("worldspawn"))
        || (entity.brushes.is_empty() && entity.patches.is_empty())
    {
        return Ok(Vec3d::zero());
    }
    for (key, value) in &entity.properties {
        let rotated = if key.eq_ignore_ascii_case("rotation") {
            let values: Vec<_> = value.split_whitespace().map(str::parse::<f64>).collect();
            values.len() != 9
                || values
                    .iter()
                    .zip([1., 0., 0., 0., 1., 0., 0., 0., 1.])
                    .any(|(v, n)| match v {
                        Ok(v) => !v.is_finite() || (v - n).abs() > 1e-9,
                        Err(_) => true,
                    })
        } else {
            (key.eq_ignore_ascii_case("angle") || key.eq_ignore_ascii_case("angles"))
                && value
                    .split_whitespace()
                    .any(|v| v.parse::<f64>().ok() != Some(0.))
        };
        if rotated {
            return Err("Doom 3 brush-entity rotation export is not implemented; bake it into geometry and reset rotation/angle properties first".into());
        }
    }
    let value = entity
        .properties
        .iter()
        .rev()
        .find(|(k, _)| k.eq_ignore_ascii_case("origin"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("0 0 0");
    let mut parts = value.split_whitespace();
    let mut xyz = [0.; 3];
    for n in &mut xyz {
        *n = parts
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .ok_or("Invalid Doom 3 brush-entity origin")?;
    }
    if parts.next().is_some() {
        return Err("Invalid Doom 3 brush-entity origin".into());
    }
    Ok(Vec3d::new(xyz[0], xyz[1], xyz[2]))
}

fn write_primitive_map(
    map: &MapFile,
    material_size: &mut dyn FnMut(&str) -> Option<Vec2f>,
    doom3: bool,
) -> Result<String, String> {
    let mut out = String::new();
    if doom3 {
        let _ = writeln!(out, "Version 2");
    }
    if let Some(game) = &map.game {
        let _ = writeln!(out, "// Game: {game}");
    }
    if !doom3 {
        let _ = writeln!(out, "// Format: Quake3");
    }
    let _ = writeln!(
        out,
        "// {METADATA_MARKER} format {}",
        map.notes_version
            .unwrap_or(0)
            .max(crate::map_format::NOTES_VERSION)
    );
    for (index, entity) in map.entities.iter().enumerate() {
        let origin = if doom3 {
            doom3_origin(entity)?
        } else {
            Vec3d::zero()
        };
        write_node_id(&mut out, entity.id);
        let _ = writeln!(out, "// entity {index}\n{{");
        for (key, value) in &entity.properties {
            // Valve's marker would falsely identify the exported map as Valve 220.
            if index == 0 && key == "mapversion" {
                continue;
            }
            let key = if doom3 {
                doom3_string(key)?.to_string()
            } else {
                escape(key)
            };
            let value = if doom3 {
                doom3_string(value)?.to_string()
            } else {
                escape(value)
            };
            let _ = writeln!(out, "\"{}\" \"{}\"", key, value);
        }
        for (brush_index, brush) in entity.brushes.iter().enumerate() {
            write_node_id(&mut out, brush.id);
            let color = uniform_color(brush);
            if let Some(color) = color {
                let _ = write!(out, "// ");
                write_color(&mut out, color);
                let _ = writeln!(out);
            }
            let kind = if doom3 { "brushDef3" } else { "brushDef" };
            let _ = writeln!(out, "// brush {brush_index}\n{{\n{kind}\n{{");
            for face in &brush.faces {
                let matrix = if let Some(matrix) = &face.primitive_uv {
                    **matrix
                } else {
                    let size = material_size(&face.material_name).ok_or_else(|| {
                        format!(
                            "Texture dimensions unavailable for '{}'",
                            face.material_name
                        )
                    })?;
                    matrix_for_face(face, size).ok_or_else(|| {
                        format!(
                            "Invalid UV mapping or texture dimensions for '{}'",
                            face.material_name
                        )
                    })?
                };
                if matrix.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(format!("Non-finite UV matrix for '{}'", face.material_name));
                }
                if doom3 {
                    let normal = face.normal();
                    let distance = -vec::dot(normal, face.points[0] - origin);
                    if !distance.is_finite() {
                        return Err("Non-finite Doom 3 plane".into());
                    }
                    let _ = write!(
                        out,
                        "( {} {} {} {} ) ",
                        Scalar(normal.x()),
                        Scalar(normal.y()),
                        Scalar(normal.z()),
                        Scalar(distance)
                    );
                } else {
                    for point in face.points {
                        let _ = write!(out, "{} ", Point(point));
                    }
                }
                let material = if doom3 {
                    doom3_string(&face.material_name)?.to_string()
                } else {
                    escape(&face.material_name)
                };
                let [[a, b, c], [d, e, f]] = matrix;
                let _ = write!(
                    out,
                    "( ( {} {} {} ) ( {} {} {} ) ) \"{}\"",
                    Scalar(a),
                    Scalar(b),
                    Scalar(c),
                    Scalar(d),
                    Scalar(e),
                    Scalar(f),
                    material
                );
                let flags = face.surface_attributes.unwrap_or_default();
                let _ = write!(
                    out,
                    " {} {} {}",
                    flags.content_flags, flags.surface_flags, flags.value
                );
                write_face_notes(&mut out, face, color.is_none(), false);
            }
            let _ = writeln!(out, "}}\n}}");
        }
        for patch in &entity.patches {
            if doom3 {
                let material = format!("\"{}\"", doom3_string(&patch.material_name)?);
                write_patch_material(&mut out, patch, &material);
            } else {
                write_patch(&mut out, patch);
            }
        }
        let _ = writeln!(out, "}}");
    }
    Ok(out)
}

fn matrix_for_face(face: &BrushFace, size: Vec2f) -> Option<[[f64; 3]; 2]> {
    let (width, height) = (f64::from(size.x()), f64::from(size.y()));
    if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
        return None;
    }
    let normal = face.normal();
    let (s, t) = crate::uv::primitive_axis_base(normal);
    let (u, v) = face.uv_coord_system.axes(normal, &face.uv_attributes);
    let u = u / f64::from(crate::uv::safe_scale(face.uv_attributes.scale.x()));
    let v = v / f64::from(crate::uv::safe_scale(face.uv_attributes.scale.y()));
    // Projecting the axes onto the plane removes their normal component. Fold
    // its constant contribution on this plane into the normalized offsets.
    let distance = vec::dot(normal, face.points[0]);
    let row = |axis: Vec3d, offset: f32, dimension: f64| {
        [
            vec::dot(axis, s) / dimension,
            vec::dot(axis, t) / dimension,
            (f64::from(offset) + vec::dot(axis, normal) * distance) / dimension,
        ]
    };
    let matrix = [
        row(u, face.uv_attributes.offset.x(), width),
        row(v, face.uv_attributes.offset.y(), height),
    ];
    matrix
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{io::map_reader::read_map, uv::UvAttributes};
    #[test]
    fn normalized_rows_match_texel_uv_on_axial_and_oblique_planes() {
        for points in [
            [
                Vec3d::new(16., 0., 0.),
                Vec3d::new(16., 4., 0.),
                Vec3d::new(16., 0., 4.),
            ],
            [
                Vec3d::new(5., 7., 11.),
                Vec3d::new(9., 8., 13.),
                Vec3d::new(4., 10., 15.),
            ],
            [
                Vec3d::new(0., 0., -32.),
                Vec3d::new(4., 0., -32.),
                Vec3d::new(0., 4., -32.),
            ],
        ] {
            for system in [
                UvCoordSystem::Paraxial,
                UvCoordSystem::Parallel {
                    u_axis: Vec3d::new(1., 0.3, 0.7),
                    v_axis: Vec3d::new(-0.2, 0.8, -1.),
                },
            ] {
                for scale in [Vec2f::new(1., 1.), Vec2f::new(-0.5, 2.5)] {
                    let face = BrushFace::from_points(
                        points,
                        "stone",
                        system,
                        UvAttributes {
                            offset: Vec2f::new(17.25, -31.5),
                            scale,
                            rotation: 37.,
                        },
                    )
                    .unwrap();
                    let size = Vec2f::new(128., 64.);
                    let matrix = matrix_for_face(&face, size).unwrap();
                    let (s, t) = crate::uv::primitive_axis_base(face.normal());
                    for point in [
                        points[0],
                        points[1],
                        points[2],
                        points[0]
                            + (points[1] - points[0]) * 13.25
                            + (points[2] - points[0]) * (-7.75),
                    ] {
                        let expected = face.uv_coords(point);
                        let st = [vec::dot(s, point), vec::dot(t, point)];
                        let u = (matrix[0][0] * st[0] + matrix[0][1] * st[1] + matrix[0][2]) * 128.;
                        let v = (matrix[1][0] * st[0] + matrix[1][1] * st[1] + matrix[1][2]) * 64.;
                        geometry_math::assert_approx_eq!(
                            Vec2f::new(u as f32, v as f32),
                            expected,
                            1e-4
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn native_map_keeps_geometry_flags_metadata_patches_and_pending_matrices() {
        let mut map = read_map(include_str!("../../tests/fixtures/format.map"), None).unwrap();
        let face = &mut map.entities[0].brushes[0].faces[0];
        face.set_paint(Some(crate::brush_face::FaceColor::solid(
            crate::brush_face::Rgb::new(3, 5, 7),
        )));
        face.surface_material = Some(crate::brush_face::SurfaceMaterial {
            roughness: 40,
            metallic: 10,
            emission: 70,
        });
        face.export_hidden = true;
        let written = write_quake3_map(&map, &mut |_| Some(Vec2f::new(128., 64.))).unwrap();
        assert!(written.contains("\nbrushDef\n"));
        assert!(!written.contains("primitive_uv"));
        assert!(!written.contains("\"mapversion\""));
        let without_comments = written
            .lines()
            .map(|line| line.split("//").next().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        let bare = read_map(&without_comments, None).unwrap();
        assert!(
            bare.entities
                .iter()
                .flat_map(|e| &e.brushes)
                .flat_map(|b| &b.faces)
                .all(|f| f.primitive_uv.is_some())
        );
        let mut reopened = read_map(&written, None).unwrap();
        assert_eq!(reopened.entities.len(), map.entities.len());
        for (original, actual) in map.entities.iter().zip(&mut reopened.entities) {
            assert_eq!(actual.id, original.id);
            assert_eq!(actual.patches, original.patches);
            for (a, b) in original.brushes.iter().zip(&mut actual.brushes) {
                assert_eq!(a.id, b.id);
                for (before, after) in a.faces.iter().zip(&mut b.faces) {
                    assert_eq!(before.points, after.points);
                    assert_eq!(before.paint(), after.paint());
                    assert_eq!(before.surface_material, after.surface_material);
                    assert_eq!(before.export_hidden, after.export_hidden);
                    assert_eq!(
                        before.surface_attributes.unwrap_or_default(),
                        after.surface_attributes.unwrap()
                    );
                    if before.primitive_uv.is_some() {
                        assert_eq!(before.primitive_uv, after.primitive_uv);
                    }
                    let mut expected = before.clone();
                    expected.resolve_primitive_uv(Vec2f::new(128., 64.));
                    after.resolve_primitive_uv(Vec2f::new(128., 64.));
                    for point in before.points {
                        geometry_math::assert_approx_eq!(
                            expected.uv_coords(point),
                            after.uv_coords(point),
                            1e-4
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn missing_or_invalid_dimensions_fail_but_pending_matrices_need_no_texture() {
        let mut map = read_map(include_str!("../../tests/fixtures/format.map"), None).unwrap();
        assert!(
            write_quake3_map(&map, &mut |_| None)
                .unwrap_err()
                .contains("dimensions unavailable")
        );
        for bad in [
            Vec2f::new(0., 64.),
            Vec2f::new(f32::NAN, 64.),
            Vec2f::new(128., -1.),
        ] {
            assert!(write_quake3_map(&map, &mut |_| Some(bad)).is_err());
        }
        for entity in &mut map.entities {
            for brush in &mut entity.brushes {
                for face in &mut brush.faces {
                    face.primitive_uv = Some(Box::new([[0.125, 0., 0.5], [0., 0.25, -0.5]]));
                }
            }
        }
        assert!(
            write_quake3_map(&map, &mut |_| panic!(
                "pending matrices do not need dimensions"
            ))
            .is_ok()
        );
    }
}

#[cfg(test)]
mod doom3_tests {
    use super::*;
    use crate::io::map_reader::read_map;
    #[test]
    fn patches_keep_world_positions_and_literal_material_names_in_translated_entities() {
        let mut map = read_map(include_str!("../../tests/fixtures/format.map"), None).unwrap();
        let mut patch = map.entities[0].patches.remove(0);
        patch.material_name = "textures/custom path\\".into();
        let entity = Entity {
            properties: vec![
                ("classname".into(), "func_static".into()),
                ("origin".into(), "128 64 32".into()),
            ],
            patches: vec![patch.clone()],
            ..Default::default()
        };
        map.entities.push(entity);
        let written = write_doom3_map(&map, &mut |_| Some(Vec2f::new(128., 64.))).unwrap();
        let reopened = read_map(&written, None).unwrap();
        assert_eq!(reopened.entities.last().unwrap().patches, vec![patch]);
        map.entities
            .last_mut()
            .unwrap()
            .properties
            .push(("rotation".into(), "0 -1 0 1 0 0 0 0 1".into()));
        assert!(
            write_doom3_map(&map, &mut |_| Some(Vec2f::new(128., 64.)))
                .unwrap_err()
                .contains("rotation")
        );
    }
}
