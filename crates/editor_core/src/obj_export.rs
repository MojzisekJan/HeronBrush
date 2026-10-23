//! Writing the map out as a Wavefront `.obj`.
//!
//! Writes brush faces as OBJ polygons with `usemtl` references and a companion MTL file pointing to
//! material images.
//!
//! Writes patches using [`map_model::patch_mesh::tessellate`], sharing the geometry used by the viewport.
//!
//! Like the glTF targets, it writes only what is drawn: layers omitted from export and
//! faces wearing a clip, skip or origin tool material are left out.
//!
//! The window that asks for this is [`crate::export_dialog`], which offers it as one
//! target among the engines.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use geometry_math::vec::Vec3d;

use map_model::node::Node;
use map_model::node::NodeId;
use map_model::node::NodeKind;
use scene_export::settings::Settings;

use crate::document::Document;
use crate::export::PortableVisuals;

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
    let (classes, _) = crate::export::entity_classes(document);
    files_scaled(
        document,
        obj_path,
        options,
        1.,
        &Settings::default(),
        &classes,
    )
}

/// Engine export units; the standalone mesh writer retains map units by default.
/// `tools` names the clip, skip and origin materials, whose faces are not written.
/// `classes` says which entities draw nothing (`visuals=false`, such as triggers).
pub fn files_scaled(
    document: &Document,
    obj_path: &Path,
    options: ObjOptions,
    scale_factor: f64,
    tools: &Settings,
    classes: &Classes,
) -> (ObjReport, Vec<(PathBuf, Vec<u8>)>) {
    let stem = obj_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "map".to_string());
    let mtl_name = format!("{stem}.mtl");

    let (obj, mtl, report, images) =
        generate_painted_scaled(document, obj_path, options, scale_factor, tools, classes);
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
    let obj_path = Path::new(mtl_name).with_extension("obj");
    let (classes, _) = crate::export::entity_classes(document);
    generate_painted_scaled(
        document,
        &obj_path,
        options,
        1.,
        &Settings::default(),
        &classes,
    )
}

/// Whether the polygon's first corners turn clockwise seen from where `normal` points.
fn clockwise(polygon: &[Vec3d], normal: Vec3d) -> bool {
    let turn = geometry_math::vec::cross(polygon[1] - polygon[0], polygon[2] - polygon[0]);
    geometry_math::vec::dot(turn, normal) < 0.0
}

type Classes = scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>;

/// The entities that draw less than all of themselves, with the brushes and patches
/// inside them; everything not in the map draws in full.
fn limited_visuals(
    document: &Document,
    classes: &Classes,
) -> HashMap<map_model::node::NodeId, PortableVisuals> {
    fn mark(nodes: &[Node], visuals: PortableVisuals, out: &mut HashMap<NodeId, PortableVisuals>) {
        for node in nodes {
            out.insert(node.id, visuals);
            if let NodeKind::Group(group) = &node.kind {
                mark(&group.children, visuals, out);
            }
        }
    }
    fn walk(nodes: &[Node], classes: &Classes, out: &mut HashMap<NodeId, PortableVisuals>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Entity(entity) => {
                    let visuals = crate::export::portable_visuals(entity, classes);
                    if visuals != PortableVisuals::All {
                        out.insert(node.id, visuals);
                        mark(&entity.children, visuals, out);
                    }
                }
                NodeKind::Group(group) => walk(&group.children, classes, out),
                NodeKind::Layer(layer) => walk(&layer.children, classes, out),
                NodeKind::Brush(_) | NodeKind::Patch(_) => {}
            }
        }
    }
    let mut out = HashMap::new();
    for layer in &document.tree_world().layers {
        walk(&layer.children, classes, &mut out);
    }
    out
}

fn generate_painted_scaled(
    document: &Document,
    obj_path: &Path,
    options: ObjOptions,
    scale_factor: f64,
    tools: &Settings,
    classes: &Classes,
) -> (String, String, ObjReport, Vec<(String, Vec<u8>)>) {
    let limited = limited_visuals(document, classes);
    let stem = obj_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "map".to_string());
    let mtl_name = &format!("{stem}.mtl");
    let omitted = |layer: usize| document.tree_world().layers[layer].omit_from_export;
    let mut surfaces = std::collections::BTreeMap::new();
    let mut paint_images = std::collections::BTreeMap::<String, Vec<u8>>::new();
    let paint_dir = format!("{}_paint", mtl_name.trim_end_matches(".mtl"));
    // Texture files go beside the export, so it moves as one folder and opens on another
    // machine; each is named by its own file name, numbered when two share one.
    let texture_dir = format!("{stem}_textures");
    let mut textures = std::collections::BTreeMap::<String, Vec<u8>>::new();
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

    for (id, brush, layer) in map_model::node::brushes_by_layer(document.tree_world()) {
        if (options.selection_only && !document.is_selected(id)) || omitted(layer) {
            continue;
        }
        // Counted even when not drawn, as a clip brush is, so `brush_N` stays the
        // map's Nth brush.
        brush_count += 1;
        let visuals = limited.get(&id).copied().unwrap_or(PortableVisuals::All);
        if visuals == PortableVisuals::Nothing {
            continue;
        }
        let mut named = false;

        for (index, face) in brush.faces().iter().enumerate() {
            if face.export_hidden
                || tools.is_tool_texture(&face.material_name)
                || (visuals == PortableVisuals::WaterSurface && face.plane.normal.z() <= 0.5)
            {
                continue;
            }
            let Some(polygon) = brush.face_polygon(index) else {
                continue;
            };
            if polygon.len() < 3 {
                continue;
            }
            if !named {
                let _ = writeln!(faces, "o brush_{brush_count}");
                named = true;
            }

            let paint = crate::export::paint_export::bake(
                face,
                &polygon,
                document.materials().get(&face.material_name),
            );
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
            // `vt` in step with its `v`, so the two indices are the same. OBJ's front
            // faces wind counter-clockwise around their normal; a brush polygon runs the
            // other way, so the fan is turned when it does.
            let n = normal_index;
            let backwards = clockwise(&polygon, face.plane.normal);
            for offset in 1..polygon.len() - 1 {
                let a = first_vertex;
                let (mut b, mut c) = (first_vertex + offset, first_vertex + offset + 1);
                if backwards {
                    std::mem::swap(&mut b, &mut c);
                }
                let _ = writeln!(faces, "f {a}/{a}/{n} {b}/{b}/{n} {c}/{c}/{n}");
                triangle_count += 1;
            }
        }
    }

    // Each patch becomes a separate triangulated OBJ object. Patch UVs already use texture-relative
    // coordinates; brush-face UVs are converted from texels using the material dimensions.
    let mut patch_count = 0usize;
    for (id, patch, layer) in map_model::node::patches_by_layer(document.tree_world()) {
        if (options.selection_only && !document.is_selected(id))
            || omitted(layer)
            || tools.is_tool_texture(&patch.material_name)
            || limited.contains_key(&id)
        {
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

            let (a, mut b, mut c) = (first_vertex, first_vertex + 1, first_vertex + 2);
            if clockwise(&triangle.positions, triangle.normal) {
                std::mem::swap(&mut b, &mut c);
            }
            let _ = writeln!(faces, "f {a}/{a}/{n} {b}/{b}/{n} {c}/{c}/{n}");
            triangle_count += 1;
        }
    }

    for (id, entity, layer) in map_model::node::point_entities_by_layer(document.tree_world()) {
        if (options.selection_only && !document.is_selected(id))
            || document.tree_world().layers[layer].omit_from_export
            || limited.get(&id) == Some(&PortableVisuals::Nothing)
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
            for (i, &normal) in normals_at.iter().enumerate() {
                let normal = fold(normal);
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
            // A copy beside the export when the file can be read; otherwise a path relative
            // to the `.mtl`, so the export can still move with the textures it uses.
            // A relative path would be read against the process's own directory, so only an
            // absolute one is copied.
            let readable = material.path.is_absolute();
            let copied = readable
                .then(|| fs::read(&material.path).ok())
                .flatten()
                .and_then(|bytes| {
                    let file = material.path.file_name()?.to_string_lossy().into_owned();
                    let mut name = format!("{texture_dir}/{file}");
                    let mut n = 1;
                    while textures.get(&name).is_some_and(|other| *other != bytes) {
                        n += 1;
                        name = format!("{texture_dir}/{n}-{file}");
                    }
                    textures.insert(name.clone(), bytes);
                    Some(name)
                });
            let path = match copied {
                Some(name) => name,
                None if material.path.is_absolute() => {
                    crate::document::relative_material_path(&material.path, obj_path)
                }
                None => material.path.display().to_string(),
            };
            let _ = writeln!(mtl, "map_Kd {path}");
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
            .chain(textures)
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

    /// Opens `source` as a map saved under the temporary folder as `<name>.map`.
    fn open_map(name: &str, source: &str) -> Document {
        let path = std::env::temp_dir().join(format!("heron-obj-{name}.map"));
        fs::write(&path, source).expect("a written map");
        Document::open(&path, WorldBounds::default()).expect("it opens")
    }

    /// A cube from `min` to `max`, every face wearing `material`.
    fn cube_text(min: i32, max: i32, material: &str) -> String {
        let (a, b, c) = (min, max, min + 1);
        let d = max + 1;
        format!(
            "{{\n( {a} {a} {a} ) ( {a} {c} {a} ) ( {a} {a} {c} ) {material} 0 0 0 1 1\n\
             ( {b} {b} {b} ) ( {b} {b} {d} ) ( {b} {d} {b} ) {material} 0 0 0 1 1\n\
             ( {a} {a} {a} ) ( {a} {a} {c} ) ( {c} {a} {a} ) {material} 0 0 0 1 1\n\
             ( {b} {b} {b} ) ( {d} {b} {b} ) ( {b} {b} {d} ) {material} 0 0 0 1 1\n\
             ( {b} {b} {b} ) ( {b} {d} {b} ) ( {d} {b} {b} ) {material} 0 0 0 1 1\n\
             ( {a} {a} {a} ) ( {c} {a} {a} ) ( {a} {c} {a} ) {material} 0 0 0 1 1\n}}\n"
        )
    }

    #[test]
    fn tool_faces_and_omitted_layers_are_left_out() {
        // Found exporting a probe map to every target: the plain .obj carried clip and
        // skip brushes and a layer omitted from export, which the glTF targets and the
        // engine meshes all leave out.
        let source = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}{}{}}}\n\
             {{\n\"classname\" \"func_group\"\n\"_tb_type\" \"_tb_layer\"\n\
             \"_tb_name\" \"Omitted\"\n\"_tb_id\" \"7\"\n\
             \"_tb_layer_omit_from_export\" \"1\"\n{}}}\n",
            cube_text(0, 32, "stone"),
            cube_text(64, 96, "clip"),
            cube_text(128, 160, "tools/skip"),
            cube_text(192, 224, "stone"),
        );
        let (obj, mtl, report) = generate(&open_map("tools", &source), "m.mtl", opts());
        assert_eq!(
            report.triangles, 12,
            "only the stone cube on a written layer"
        );
        assert_eq!(obj.matches("o brush_").count(), 1);
        assert!(!mtl.contains("clip") && !mtl.contains("skip"));
    }

    #[test]
    fn faces_wind_counter_clockwise_around_their_normals() {
        // Found reading the feature lab's .obj: every brush face was wound clockwise, so
        // a viewer that culls back faces showed the inside of each box.
        let (obj, _, _) = generate(&one_cube(), "m.mtl", opts());
        let parse = |prefix: &str| -> Vec<[f64; 3]> {
            obj.lines()
                .filter_map(|l| l.strip_prefix(prefix))
                .map(|l| {
                    let v: Vec<f64> = l.split(' ').map(|x| x.parse().expect("a number")).collect();
                    [v[0], v[1], v[2]]
                })
                .collect()
        };
        let (positions, normals) = (parse("v "), parse("vn "));
        let mut faces = 0;
        for line in obj.lines().filter_map(|l| l.strip_prefix("f ")) {
            let corners: Vec<Vec<usize>> = line
                .split(' ')
                .map(|c| c.split('/').map(|i| i.parse().expect("an index")).collect())
                .collect();
            let p = |i: usize| {
                let [x, y, z] = positions[corners[i][0] - 1];
                Vec3d::new(x, y, z)
            };
            let [x, y, z] = normals[corners[0][2] - 1];
            assert!(
                !clockwise(&[p(0), p(1), p(2)], Vec3d::new(x, y, z)),
                "{line}"
            );
            faces += 1;
        }
        assert_eq!(faces, 12);
    }

    #[test]
    fn a_trigger_is_left_out_and_water_draws_only_its_top() {
        // Found checking the feature lab's .obj: the trigger drew as a solid box and the
        // water as a closed cube, although both classes draw nothing in the engines.
        let source = format!(
            "{{\n\"classname\" \"worldspawn\"\n{}}}\n\
             {{\n\"classname\" \"trigger_multiple\"\n{}}}\n\
             {{\n\"classname\" \"water_volume\"\n{}}}\n",
            cube_text(0, 32, "stone"),
            cube_text(64, 96, "hazard"),
            cube_text(128, 160, "water"),
        );
        let (obj, mtl, report) = generate(&open_map("volumes", &source), "m.mtl", opts());
        assert_eq!(
            report.triangles,
            12 + 2,
            "the stone cube and the water's top"
        );
        assert!(obj.contains("o brush_1\n") && obj.contains("o brush_3\n"));
        assert!(!obj.contains("o brush_2\n"));
        assert!(!obj.contains("usemtl hazard") && !mtl.contains("hazard"));
        let water = obj
            .split("usemtl water")
            .nth(1)
            .expect("the water is drawn");
        assert_eq!(water.matches("\nf ").count(), 2);
    }

    #[test]
    fn a_texture_file_is_copied_beside_the_mtl() {
        // Found loading the .obj target's output in Blender: map_Kd held the absolute
        // path of the texture on the machine that exported it, and the export did not
        // carry the texture along.
        let root = std::env::temp_dir().join(format!("heron-obj-relative-{}", std::process::id()));
        let textures = root.join("textures");
        fs::create_dir_all(&textures).unwrap();
        let png = crate::assets::encode_png(2, 2, &[200; 16]).unwrap();
        fs::write(textures.join("stone.png"), png).unwrap();
        let mut document = one_cube();
        document.load_materials(&textures).unwrap();
        let obj = root.join("export").join("room.obj");
        let (_, files) = files(&document, &obj, opts());
        let mtl = files
            .iter()
            .find(|(path, _)| path.extension().is_some_and(|e| e == "mtl"))
            .map(|(_, contents)| String::from_utf8_lossy(contents).into_owned())
            .unwrap();
        assert!(
            mtl.contains("map_Kd room_textures/stone.png"),
            "the MTL was:\n{mtl}"
        );
        let copy = root.join("export").join("room_textures").join("stone.png");
        assert!(
            files
                .iter()
                .any(|(path, bytes)| *path == copy && bytes.starts_with(b"\x89PNG")),
            "the texture travels with the export"
        );
        let _ = fs::remove_dir_all(&root);
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
