//! Writing the map out as a Wavefront `.obj`.
//!
//! Writes brush faces as OBJ polygons with `usemtl` references and a companion MTL file pointing to
//! material images.
//!
//! Writes patches using [`map_model::patch_mesh::tessellate`], sharing the geometry used by the viewport.
//!
//! The window that asks for this is [`crate::export_dialog`], which offers it as one
//! target among the engines.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use geometry_math::vec::Vec3d;

use crate::document::Document;

/// What to put in the file.
#[derive(Debug, Clone, Copy)]
pub struct ObjOptions {
    /// Write only what is selected rather than the whole map.
    pub selection_only: bool,
    /// Turn Quake's Z-up coordinates into the Y-up a modelling tool expects.
    pub y_up: bool,
}

/// What an export produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjReport {
    /// How many brushes were written.
    pub brushes: usize,
    /// How many patches were written.
    pub patches: usize,
    /// How many triangles they came to.
    pub triangles: usize,
    /// How many distinct materials were referenced.
    pub materials: usize,
    /// Generated images, relative to the OBJ directory.
    pub images: Vec<String>,
}

/// Writes `<name>.obj` and `<name>.mtl` beside it, returning what went in.
pub fn write(document: &Document, obj_path: &Path, options: ObjOptions) -> io::Result<ObjReport> {
    let (report, files) = files(document, obj_path, options);
    for (path, contents) in files {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    Ok(report)
}

/// What [`write`] writes, as paths and contents: the images, then the `.obj` and the
/// `.mtl`.
pub fn files(
    document: &Document,
    obj_path: &Path,
    options: ObjOptions,
) -> (ObjReport, Vec<(PathBuf, Vec<u8>)>) {
    files_scaled(document, obj_path, options, 1.)
}

/// Engine export units; the standalone mesh writer retains map units by default.
pub fn files_scaled(
    document: &Document,
    obj_path: &Path,
    options: ObjOptions,
    scale_factor: f64,
) -> (ObjReport, Vec<(PathBuf, Vec<u8>)>) {
    let stem = obj_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "map".to_string());
    let mtl_name = format!("{stem}.mtl");

    let (obj, mtl, report, images) =
        generate_painted_scaled(document, &mtl_name, options, scale_factor);
    let directory = obj_path.parent().unwrap_or(Path::new("."));
    let mut files: Vec<(PathBuf, Vec<u8>)> = images
        .into_iter()
        .map(|(relative, png)| (directory.join(relative), png))
        .collect();
    files.push((obj_path.to_path_buf(), obj.into_bytes()));
    files.push((obj_path.with_file_name(&mtl_name), mtl.into_bytes()));
    (report, files)
}

/// Builds the `.obj` and `.mtl` text without touching the disk, so it can be checked.
#[cfg(test)]
fn generate(
    document: &Document,
    mtl_name: &str,
    options: ObjOptions,
) -> (String, String, ObjReport) {
    let (obj, mtl, report, _) = generate_painted(document, mtl_name, options);
    (obj, mtl, report)
}

#[cfg(test)]
fn generate_painted(
    document: &Document,
    mtl_name: &str,
    options: ObjOptions,
) -> (String, String, ObjReport, Vec<(String, Vec<u8>)>) {
    generate_painted_scaled(document, mtl_name, options, 1.)
}

fn generate_painted_scaled(
    document: &Document,
    mtl_name: &str,
    options: ObjOptions,
    scale_factor: f64,
) -> (String, String, ObjReport, Vec<(String, Vec<u8>)>) {
    let mut surfaces = std::collections::BTreeMap::new();
    let mut paint_images = std::collections::BTreeMap::<String, Vec<u8>>::new();
    let paint_dir = format!("{}_paint", mtl_name.trim_end_matches(".mtl"));
    let fold = |p: Vec3d| {
        if options.y_up {
            // Quake is Z-up right-handed; the usual modelling frame is Y-up. This is the
            // same fold the `.map` importer would undo.
            Vec3d::new(p.x(), p.z(), -p.y())
        } else {
            p
        }
    };

    let mut obj = String::new();
    let _ = writeln!(obj, "# exported by Heron Brush");
    let _ = writeln!(obj, "mtllib {mtl_name}");

    let mut positions = String::new();
    let mut normals = String::new();
    let mut uvs = String::new();
    let mut faces = String::new();

    let mut vertex_count = 0usize;
    let mut normal_count = 0usize;
    let mut brush_count = 0usize;
    let mut triangle_count = 0usize;
    let mut used_materials: Vec<String> = Vec::new();
    let mut current_material: Option<String> = None;

    for (id, brush) in map_model::node::brushes_with_ids(document.tree_world()) {
        if options.selection_only && !document.is_selected(id) {
            continue;
        }
        brush_count += 1;
        let _ = writeln!(faces, "o brush_{}", brush_count);

        for (index, face) in brush.faces().iter().enumerate() {
            if face.export_hidden {
                continue;
            }
            let Some(polygon) = brush.face_polygon(index) else {
                continue;
            };
            if polygon.len() < 3 {
                continue;
            }

            let paint = if scene_export::settings::Settings::default()
                .is_tool_texture(&face.material_name)
            {
                None
            } else {
                crate::export::paint_export::bake(
                    face,
                    &polygon,
                    document.materials().get(&face.material_name),
                )
            };
            let surface_suffix = face
                .surface_material
                .map(|s| format!("_pbr_{}_{}_{}", s.roughness, s.metallic, s.emission))
                .unwrap_or_default();
            let material_name = if let Some(image) = &paint {
                let png = image.png().expect("built-in PNG encoder");
                let name = format!(
                    "{}{}",
                    crate::export::paint_export::image_name(&png),
                    surface_suffix
                );
                paint_images.entry(name.clone()).or_insert(png);
                name
            } else {
                format!("{}{}", face.material_name, surface_suffix)
            };
            if let Some(surface) = face.surface_material {
                surfaces.insert(
                    material_name.clone(),
                    (face.material_name.to_string(), surface),
                );
            }
            if current_material.as_deref() != Some(material_name.as_str()) {
                let _ = writeln!(faces, "usemtl {}", material_name);
                current_material = Some(material_name.to_string());
                if !used_materials.contains(&material_name) {
                    used_materials.push(material_name.to_string());
                }
            }

            let size = document
                .materials()
                .get(&face.material_name)
                .map(|m| (m.width.max(1) as f64, m.height.max(1) as f64))
                .unwrap_or((64.0, 64.0));

            let normal = fold(face.plane.normal);
            normal_count += 1;
            let _ = writeln!(
                normals,
                "vn {:.6} {:.6} {:.6}",
                normal.x(),
                normal.y(),
                normal.z()
            );
            let normal_index = normal_count;

            let first_vertex = vertex_count + 1;
            for corner in &polygon {
                let p = fold(*corner);
                vertex_count += 1;
                let _ = writeln!(
                    positions,
                    "v {:.6} {:.6} {:.6}",
                    p.x() * scale_factor,
                    p.y() * scale_factor,
                    p.z() * scale_factor
                );

                let original_uv = face.uv_coords(*corner);
                let uv = paint
                    .as_ref()
                    .map(|p| p.uv(original_uv))
                    .unwrap_or_else(|| {
                        geometry_math::vec::Vec2f::new(
                            original_uv.x() / size.0 as f32,
                            original_uv.y() / size.1 as f32,
                        )
                    });
                // OBJ's V runs up, the material's runs down, so it is flipped.
                let _ = writeln!(uvs, "vt {:.6} {:.6}", f64::from(uv.x()), -f64::from(uv.y()));
            }

            // A convex polygon as a triangle fan from its first vertex. Each vertex got a
            // `vt` in step with its `v`, so the two indices are the same.
            let n = normal_index;
            for offset in 1..polygon.len() - 1 {
                let a = first_vertex;
                let b = first_vertex + offset;
                let c = first_vertex + offset + 1;
                let _ = writeln!(faces, "f {a}/{a}/{n} {b}/{b}/{n} {c}/{c}/{n}");
                triangle_count += 1;
            }
        }
    }

    // Each patch becomes a separate triangulated OBJ object. Patch UVs already use texture-relative
    // coordinates; brush-face UVs are converted from texels using the material dimensions.
    let mut patch_count = 0usize;
    for (id, patch, _) in map_model::node::patches_by_layer(document.tree_world()) {
        if options.selection_only && !document.is_selected(id) {
            continue;
        }
        let mesh =
            map_model::patch_mesh::tessellate(patch, map_model::patch_mesh::DEFAULT_SUBDIVISIONS);
        if mesh.is_empty() {
            continue;
        }

        patch_count += 1;
        let _ = writeln!(faces, "o patch_{patch_count}");
        if current_material.as_deref() != Some(patch.material_name.as_str()) {
            let _ = writeln!(faces, "usemtl {}", patch.material_name);
            current_material = Some(patch.material_name.clone());
            if !used_materials.contains(&patch.material_name) {
                used_materials.push(patch.material_name.clone());
            }
        }

        for triangle in mesh {
            let normal = fold(triangle.normal);
            normal_count += 1;
            let _ = writeln!(
                normals,
                "vn {:.6} {:.6} {:.6}",
                normal.x(),
                normal.y(),
                normal.z()
            );
            let n = normal_count;

            let first_vertex = vertex_count + 1;
            for corner in 0..3 {
                let p = fold(triangle.positions[corner]);
                vertex_count += 1;
                let _ = writeln!(
                    positions,
                    "v {:.6} {:.6} {:.6}",
                    p.x() * scale_factor,
                    p.y() * scale_factor,
                    p.z() * scale_factor
                );

                let uv = triangle.uv[corner];
                let _ = writeln!(uvs, "vt {:.6} {:.6}", uv.x(), -uv.y());
            }

            let (a, b, c) = (first_vertex, first_vertex + 1, first_vertex + 2);
            let _ = writeln!(faces, "f {a}/{a}/{n} {b}/{b}/{n} {c}/{c}/{n}");
            triangle_count += 1;
        }
    }

    for (id, entity, layer) in map_model::node::point_entities_by_layer(document.tree_world()) {
        if (options.selection_only && !document.is_selected(id))
            || document.tree_world().layers[layer].omit_from_export
        {
            continue;
        }
        let Some(model) = document.imported_model(entity) else {
            continue;
        };
        let mut names = Vec::new();
        for skin in model.skins() {
            use std::hash::{Hash, Hasher};
            let mut hash = crate::export_hash::ExportHasher::default();
            skin.pixels.hash(&mut hash);
            skin.width.hash(&mut hash);
            skin.height.hash(&mut hash);
            let name = format!("heron_import_{:016x}", hash.finish());
            if !paint_images.contains_key(&name) {
                if let Ok(png) = crate::assets::encode_png(skin.width, skin.height, &skin.pixels) {
                    paint_images.insert(name.clone(), png);
                }
            }
            if !used_materials.contains(&name) {
                used_materials.push(name.clone());
            }
            names.push(name);
        }
        let _ = writeln!(faces, "o imported_{vertex_count}");
        let origin = entity.origin().unwrap_or_else(Vec3d::zero);
        for triangle in &model.triangles {
            let skin = model.skin_at(triangle.material);
            let _ = writeln!(faces, "usemtl {}", names[triangle.material]);
            let normal = geometry_math::vec::cross(
                triangle.positions[1] - triangle.positions[0],
                triangle.positions[2] - triangle.positions[0],
            );
            let length = geometry_math::vec::length(normal);
            if length < 1e-9 {
                continue;
            }
            let normals_at = triangle.normals.unwrap_or([normal / length; 3]);
            let first = vertex_count + 1;
            let first_normal = normal_count + 1;
            for i in 0..3 {
                let normal = fold(normals_at[i]);
                normal_count += 1;
                let _ = writeln!(
                    normals,
                    "vn {:.6} {:.6} {:.6}",
                    normal.x(),
                    normal.y(),
                    normal.z()
                );
                let p = fold(triangle.positions[i] + origin) * scale_factor;
                vertex_count += 1;
                let _ = writeln!(positions, "v {:.6} {:.6} {:.6}", p.x(), p.y(), p.z());
                let uv = triangle.uv[i];
                let _ = writeln!(
                    uvs,
                    "vt {:.6} {:.6}",
                    uv.x() / skin.width as f32,
                    1.0 - uv.y() / skin.height as f32
                );
            }
            let (a, b, c) = (first, first + 1, first + 2);
            let (n, m, k) = (first_normal, first_normal + 1, first_normal + 2);
            let _ = writeln!(faces, "f {a}/{a}/{n} {b}/{b}/{m} {c}/{c}/{k}");
            triangle_count += 1;
        }
    }

    obj.push_str(&positions);
    obj.push_str(&uvs);
    obj.push_str(&normals);
    obj.push_str(&faces);

    for name in &used_materials {
        if paint_images.contains_key(name) {
            continue;
        }
        let Some(material) = document.materials().get(name).filter(|m| m.is_game_asset()) else {
            continue;
        };
        if let Some(pixels) = material.pixels() {
            if let Ok(png) = crate::assets::encode_png(material.width, material.height, &pixels) {
                paint_images.insert(name.clone(), png);
            }
        }
    }

    let mut mtl = String::from("# exported by Heron Brush\n");
    for name in &used_materials {
        let _ = writeln!(mtl, "\nnewmtl {name}");
        if let Some((_, surface)) = surfaces.get(name) {
            let [roughness, metallic, emission] = surface.factors();
            let specular = 0.04 * (1.0 - metallic) + metallic;
            let _ = writeln!(
                mtl,
                "Pr {roughness}\nPm {metallic}\nNs {}\nKs {specular} {specular} {specular}",
                (2.0 / roughness.max(0.05).powi(4) - 2.0).min(1000.0)
            );
            if emission > 0.0 {
                let _ = writeln!(mtl, "Ke {emission} {emission} {emission}");
                if paint_images.contains_key(name) {
                    let _ = writeln!(mtl, "map_Ke {paint_dir}/{name}.png");
                }
            }
        }
        if paint_images.contains_key(name) {
            let _ = writeln!(mtl, "Kd 1 1 1\nmap_Kd {paint_dir}/{name}.png");
            continue;
        }
        let _ = writeln!(mtl, "Kd 0.8 0.8 0.8");
        if let Some(material) = document.materials().get(
            surfaces
                .get(name)
                .map(|(name, _)| name.as_str())
                .unwrap_or(name),
        ) {
            let _ = writeln!(mtl, "map_Kd {}", material.path.display());
        }
    }

    (
        obj,
        mtl,
        ObjReport {
            brushes: brush_count,
            patches: patch_count,
            triangles: triangle_count,
            materials: used_materials.len(),
            images: paint_images
                .keys()
                .map(|name| format!("{paint_dir}/{name}.png"))
                .collect(),
        },
        paint_images
            .into_iter()
            .map(|(name, png)| (format!("{paint_dir}/{name}.png"), png))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;

    fn one_cube() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush =
            map_model::shapes::create_cuboid(&world, &Bbox3d::from_scalars(-32.0, 32.0), "stone")
                .expect("a cube");
        document.add_brushes(vec![brush]);
        document
    }

    /// A map holding one 3x3 patch, the same one the document's own patch tests use.
    fn one_patch(name: &str) -> Document {
        let source = r#"
{
"classname" "worldspawn"
{
patchDef2
{
common/caulk
( 3 3 0 0 0 )
(
( ( -64 -64 0 0 0 ) ( -64 0 32 0 0.5 ) ( -64 64 0 0 1 ) )
( ( 0 -64 32 0.5 0 ) ( 0 0 64 0.5 0.5 ) ( 0 64 32 0.5 1 ) )
( ( 64 -64 0 1 0 ) ( 64 0 32 1 0.5 ) ( 64 64 0 1 1 ) )
)
}
}
}
"#;
        let path = std::env::temp_dir().join(format!("heron-obj-patch-{name}.map"));
        fs::write(&path, source).expect("a written map");
        Document::open(&path, WorldBounds::default()).expect("it opens")
    }

    fn opts() -> ObjOptions {
        ObjOptions {
            selection_only: false,
            y_up: false,
        }
    }

    #[test]
    fn a_cube_writes_one_object_of_twelve_triangles() {
        let (obj, _, report) = generate(&one_cube(), "m.mtl", opts());
        assert_eq!(report.brushes, 1);
        assert_eq!(report.triangles, 12, "six quads, two triangles each");
        assert_eq!(
            obj.matches("\nf ").count() + obj.starts_with("f ") as usize,
            12
        );
        assert!(obj.contains("mtllib m.mtl"));
        assert!(obj.lines().filter(|l| l.starts_with("v ")).count() == 24);
    }

    #[test]
    fn the_material_becomes_a_usemtl_and_an_mtl_entry() {
        let (obj, mtl, report) = generate(&one_cube(), "m.mtl", opts());
        assert_eq!(report.materials, 1);
        assert!(obj.contains("usemtl stone"));
        assert!(mtl.contains("newmtl stone"));
    }

    #[test]
    fn selection_only_writes_nothing_when_nothing_is_selected() {
        let mut document = one_cube();
        document.clear_selection();
        let (_, _, report) = generate(
            &document,
            "m.mtl",
            ObjOptions {
                selection_only: true,
                y_up: false,
            },
        );
        assert_eq!(report.brushes, 0);
    }

    #[test]
    fn write_puts_an_obj_and_an_mtl_side_by_side() {
        let dir = std::env::temp_dir().join(format!("tbr_obj_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let obj = dir.join("room.obj");

        let report = write(&one_cube(), &obj, opts()).expect("the write succeeds");
        assert_eq!(report.brushes, 1);
        assert!(obj.is_file());
        assert!(dir.join("room.mtl").is_file());
        assert!(fs::read_to_string(&obj)
            .unwrap()
            .contains("mtllib room.mtl"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn y_up_folds_the_coordinates() {
        let flat = generate(&one_cube(), "m.mtl", opts()).0;
        let folded = generate(
            &one_cube(),
            "m.mtl",
            ObjOptions {
                selection_only: false,
                y_up: true,
            },
        )
        .0;
        // The cube spans -32..32 on every axis, so its own corners look the same either
        // way; what changes is which coordinate carries the height. A point at z = 32
        // becomes y = 32, and one at y = 32 becomes z = -32.
        assert_ne!(flat, folded);
        assert!(folded.contains("v 32.000000 32.000000 32.000000"));
    }

    #[test]
    fn a_patch_is_written_as_its_tessellation() {
        let (obj, mtl, report) = generate(&one_patch("written"), "m.mtl", opts());
        assert_eq!(report.brushes, 0);
        assert_eq!(report.patches, 1);
        // One 3x3 piece cut into the default steps both ways, two triangles a cell.
        let steps = map_model::patch_mesh::DEFAULT_SUBDIVISIONS;
        assert_eq!(report.triangles, steps * steps * 2);
        assert!(obj.contains("o patch_1"));
        assert!(obj.contains("usemtl common/caulk"));
        assert!(mtl.contains("newmtl common/caulk"));
    }

    #[test]
    fn a_patch_carries_the_map_s_own_uv_coordinates() {
        let (obj, _, _) = generate(&one_patch("uv"), "m.mtl", opts());
        // The corner control point is at (0, 0) on the material and the far one at
        // (1, 1); OBJ's V runs the other way, so the far corner comes out as (1, -1).
        // A face's texel coordinates would be divided by the material's size instead.
        assert!(obj.contains("vt 0.000000 -0.000000") || obj.contains("vt 0.000000 0.000000"));
        assert!(obj.contains("vt 1.000000 -1.000000"));
    }

    #[test]
    fn selection_only_leaves_out_an_unselected_patch() {
        let mut document = one_patch("unselected");
        document.clear_selection();
        let (_, _, report) = generate(
            &document,
            "m.mtl",
            ObjOptions {
                selection_only: true,
                y_up: false,
            },
        );
        assert_eq!(report.patches, 0);
        assert_eq!(report.triangles, 0);
    }
}
