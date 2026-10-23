//! Portable glTF scenes from the editor tree, including native face paint and patches.
use super::{Contents, Document, Error, Options, Report, Target};
use geometry_math::vec::Vec3d;
use map_model::brush_face::FaceColor;
use map_model::node::{Node, NodeKind};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

struct Scene<'a> {
    document: &'a Document,
    options: &'a Options,
    root: Value,
    bin: Vec<u8>,
    images: Vec<Vec<u8>>,
    image_cache: HashMap<Vec<u8>, usize>,
    model_meshes: HashMap<String, (usize, usize)>,
    texture_cache: HashMap<(usize, bool), usize>,
    materials: HashMap<String, usize>,
    /// Materials given a `normalTexture`, whose primitives also get `TANGENT`.
    normal_materials: std::collections::HashSet<usize>,
    report: Report,
    settings: scene_export::settings::Settings,
    classes: scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
}

fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

impl Scene<'_> {
    fn push(&mut self, key: &str, value: Value) -> usize {
        let list = self.root[key].as_array_mut().unwrap();
        let index = list.len();
        list.push(value);
        index
    }
    fn data(&mut self, values: &[f32], width: usize, bounds: bool) -> usize {
        let start = self.bin.len();
        for v in values {
            self.bin.extend_from_slice(&v.to_le_bytes());
        }
        let view = self.push(
            "bufferViews",
            json!({"buffer":0,"byteOffset":start,"byteLength":values.len()*4,"target":34962}),
        );
        let mut accessor = json!({"bufferView":view,"componentType":5126,"count":values.len()/width,"type":format!("VEC{width}")});
        if bounds {
            let min: Vec<f32> = (0..width)
                .map(|i| {
                    values
                        .chunks_exact(width)
                        .map(|v| v[i])
                        .fold(f32::INFINITY, f32::min)
                })
                .collect();
            let max: Vec<f32> = (0..width)
                .map(|i| {
                    values
                        .chunks_exact(width)
                        .map(|v| v[i])
                        .fold(f32::NEG_INFINITY, f32::max)
                })
                .collect();
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        self.push("accessors", accessor)
    }
    fn image(&mut self, width: u32, height: u32, pixels: Vec<u8>, repeat: bool) -> usize {
        // Editor images, including textures loaded from WADs, are all 8-bit RGBA.
        let png = crate::assets::encode_png(width, height, &pixels)
            .expect("RGBA pixels of the stated size encode");
        let image = if let Some(index) = self.image_cache.get(&png) {
            *index
        } else {
            let index = self.images.len();
            self.image_cache.insert(png.clone(), index);
            self.images.push(png);
            self.push("images", json!({}));
            index
        };
        if let Some(index) = self.texture_cache.get(&(image, repeat)) {
            return *index;
        }
        let texture = self.push(
            "textures",
            json!({"source":image,"sampler":if repeat {0} else {1}}),
        );
        self.texture_cache.insert((image, repeat), texture);
        texture
    }
    fn material(
        &mut self,
        name: &str,
        solid: Option<[f32; 3]>,
        image: Option<usize>,
        alpha: bool,
        double: bool,
        surface: Option<map_model::brush_face::SurfaceMaterial>,
    ) -> usize {
        let [roughness, metallic, emission] =
            surface.map(|s| s.factors()).unwrap_or([1.0, 0.0, 0.0]);
        let mut pbr = json!({"metallicFactor":metallic,"roughnessFactor":roughness});
        if let Some(rgb) = solid {
            pbr["baseColorFactor"] = json!([linear(rgb[0]), linear(rgb[1]), linear(rgb[2]), 1.0]);
        }
        if let Some(index) = image {
            pbr["baseColorTexture"] = json!({"index":index});
        }
        let mut value = json!({"name":material_label(name),"pbrMetallicRoughness":pbr,"doubleSided":double,"alphaMode":if alpha {"BLEND"} else {"OPAQUE"}});
        if emission > 0.0 {
            let rgb = solid.map(|rgb| rgb.map(linear)).unwrap_or([1.0; 3]);
            value["emissiveFactor"] = json!(rgb);
            if let Some(index) = image {
                value["emissiveTexture"] = json!({"index":index});
            }
            value["extensions"]["KHR_materials_emissive_strength"] =
                json!({"emissiveStrength":emission});
            self.root["extensionsUsed"] = json!(["KHR_materials_emissive_strength"]);
        }
        self.push("materials", value)
    }
    fn texture_material(
        &mut self,
        name: &str,
        double: bool,
        surface: Option<map_model::brush_face::SurfaceMaterial>,
    ) -> usize {
        let key = format!("{name}:{double}:{surface:?}");
        if let Some(id) = self.materials.get(&key) {
            return *id;
        }
        let material = self.document.materials().get(name);
        let (image, alpha) = if let Some(pixels) = material
            .and_then(|m| Some((m, m.pixels()?)))
            .map(|(m, p)| (m, p[..(m.width * m.height * 4) as usize].to_vec()))
        {
            let (m, pixels) = pixels;
            let alpha = pixels.as_chunks::<4>().0.iter().any(|p| p[3] < 255);
            (Some(self.image(m.width, m.height, pixels, true)), alpha)
        } else {
            let warning = format!(
                "Texture '{name}' is not loaded in the editor; exported with a neutral material."
            );
            // One warning a texture, however many surface or sidedness variants use it.
            if name != "__TB_empty" && !self.report.warnings.contains(&warning) {
                self.report.warnings.push(warning);
            }
            (None, false)
        };
        let id = self.material(name, None, image, alpha, double, surface);
        // The companion normal map, in the glTF (OpenGL +Y) convention.
        let normal = material
            .and_then(|m| m.normal_map.as_ref())
            .filter(|_| image.is_some());
        if let Some((normal, pixels)) = normal.and_then(|n| Some((n, n.pixels()?))) {
            let mut rgba = pixels[..(normal.width * normal.height * 4) as usize].to_vec();
            for p in rgba.as_chunks_mut::<4>().0 {
                if normal.flip_y {
                    p[1] = 255 - p[1];
                }
                p[3] = 255;
            }
            let texture = self.image(normal.width, normal.height, rgba, true);
            self.root["materials"][id]["normalTexture"] = json!({"index":texture});
            self.normal_materials.insert(id);
        }
        if let Some(pbr) = material.and_then(|m| m.model_pbr.clone()) {
            self.root["materials"][id]["pbrMetallicRoughness"]["roughnessFactor"] =
                json!(pbr.roughness);
            self.root["materials"][id]["pbrMetallicRoughness"]["metallicFactor"] =
                json!(pbr.metallic);
            self.root["materials"][id]["emissiveFactor"] = json!(pbr.emissive_factor);
            self.root["materials"][id]["alphaMode"] = json!(match pbr.alpha_mode {
                map_model::material::Alpha::Opaque => "OPAQUE",
                map_model::material::Alpha::Masked => "MASK",
                map_model::material::Alpha::Blended => "BLEND",
            });
            if pbr.alpha_mode == map_model::material::Alpha::Masked {
                self.root["materials"][id]["alphaCutoff"] = json!(pbr.alpha_cutoff);
            }
            self.root["materials"][id]["doubleSided"] = json!(pbr.double_sided);
            if self.root["materials"][id]["normalTexture"].is_object() {
                self.root["materials"][id]["normalTexture"]["scale"] = json!(pbr.normal_scale);
            }
            for (slot, map) in [
                ("metallicRoughnessTexture", &pbr.metallic_roughness),
                ("occlusionTexture", &pbr.occlusion),
                ("emissiveTexture", &pbr.emissive),
            ] {
                if let Some(map) = map {
                    let texture = self.image(map.width, map.height, map.pixels.clone(), true);
                    if slot == "metallicRoughnessTexture" {
                        self.root["materials"][id]["pbrMetallicRoughness"][slot] =
                            json!({"index":texture});
                    } else {
                        self.root["materials"][id][slot] = json!({"index":texture});
                    }
                }
            }
            if self.root["materials"][id]["occlusionTexture"].is_object() {
                self.root["materials"][id]["occlusionTexture"]["strength"] =
                    json!(pbr.occlusion_strength);
            }
        }
        self.materials.insert(key, id);
        id
    }
    fn tool(&self, name: &str) -> bool {
        self.settings.is_skip(name) || self.settings.is_clip(name) || self.settings.is_origin(name)
    }
    fn face(
        &mut self,
        face: &map_model::brush_face::BrushFace,
        polygon: &[Vec3d],
    ) -> (usize, Vec<[f32; 2]>) {
        let material = self.document.materials().get(&face.material_name);
        let size = material
            .map(|m| [m.width.max(1) as f32, m.height.max(1) as f32])
            .unwrap_or([64.0, 64.0]);
        let original: Vec<[f32; 2]> = polygon
            .iter()
            .map(|p| {
                let uv = face.uv_coords(*p);
                [uv.x(), uv.y()]
            })
            .collect();
        let Some(paint) = face.paint() else {
            let id = self.texture_material(&face.material_name, false, face.surface_material);
            return (
                id,
                original
                    .iter()
                    .map(|p| [p[0] / size[0], p[1] / size[1]])
                    .collect(),
            );
        };
        if let FaceColor::Solid { color, alpha: 255 } = paint {
            let key = format!("paint:{color:?}:{:?}", face.surface_material);
            let id = if let Some(id) = self.materials.get(&key) {
                *id
            } else {
                let id = self.material(
                    &key,
                    Some(color.to_f32()),
                    None,
                    false,
                    false,
                    face.surface_material,
                );
                self.materials.insert(key, id);
                id
            };
            return (id, vec![[0.0, 0.0]; polygon.len()]);
        }
        let baked = super::paint_export::bake(face, polygon, material).expect("painted polygon");
        let uv = polygon
            .iter()
            .map(|p| {
                let uv = baked.uv(face.uv_coords(*p));
                [uv.x(), uv.y()]
            })
            .collect();
        let alpha = baked.pixels.as_chunks::<4>().0.iter().any(|p| p[3] < 255);
        let image = self.image(baked.width, baked.height, baked.pixels, false);
        let id = self.material(
            "Face paint",
            None,
            Some(image),
            alpha,
            false,
            face.surface_material,
        );
        (id, uv)
    }

    fn primitive(
        &mut self,
        points: &[Vec3d],
        normal: Vec3d,
        uv: &[[f32; 2]],
        material: usize,
    ) -> Value {
        self.primitive_with_normals(points, normal, None, uv, material)
    }

    fn primitive_with_normals(
        &mut self,
        points: &[Vec3d],
        normal: Vec3d,
        corner_normals: Option<[Vec3d; 3]>,
        uv: &[[f32; 2]],
        material: usize,
    ) -> Value {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut texcoords = Vec::new();
        for i in 1..points.len() - 1 {
            let cross = geometry_math::vec::cross(points[i] - points[0], points[i + 1] - points[0]);
            let backwards = geometry_math::vec::dot(cross, normal) < 0.0;
            let order = if backwards ^ self.options.flip_faces {
                [0, i + 1, i]
            } else {
                [0, i, i + 1]
            };
            for j in order {
                let p = points[j] / self.options.scale;
                positions.extend([p.x() as f32, p.z() as f32, -p.y() as f32]);
                let shading = corner_normals.map_or(normal, |n| n[j]);
                normals.extend([shading.x() as f32, shading.z() as f32, -shading.y() as f32]);
                texcoords.extend([
                    uv[j][0],
                    if self.options.flip_uv_v {
                        -uv[j][1]
                    } else {
                        uv[j][1]
                    },
                ]);
            }
            self.report.triangles += 1;
        }
        let p = self.data(&positions, 3, true);
        let n = self.data(&normals, 3, false);
        let uv = self.data(&texcoords, 2, false);
        let mut primitive = json!({"attributes":{"POSITION":p,"NORMAL":n,"TEXCOORD_0":uv},"material":material,"mode":4});
        if self.normal_materials.contains(&material) {
            let order: Vec<u32> = (0..positions.len() as u32 / 3).collect();
            let tangents = scene_export::writer::gltf::tangents(
                positions.as_chunks::<3>().0,
                normals.as_chunks::<3>().0,
                texcoords.as_chunks::<2>().0,
                &order,
                self.options.flip_uv_v,
            );
            let flat: Vec<f32> = tangents.into_iter().flatten().collect();
            primitive["attributes"]["TANGENT"] = json!(self.data(&flat, 4, false));
        }
        primitive
    }
    fn children(&mut self, children: &[Node]) -> Vec<usize> {
        children.iter().flat_map(|n| self.node(n)).collect()
    }
    fn node(&mut self, node: &Node) -> Vec<usize> {
        let mut model_key = None;
        let mut model_matrix = None;
        let mut cached_mesh = None;
        let before_triangles = self.report.triangles;
        let (name, children, primitives, extras, translation) = match &node.kind {
            NodeKind::Layer(l) => {
                if l.omit_from_export {
                    return vec![];
                }
                let children = self.children(&l.children);
                if !self.options.group_hierarchy {
                    return children;
                }
                (l.name.clone(), children, vec![], Value::Null, None)
            }
            NodeKind::Group(g) => {
                let children = self.children(&g.children);
                if !self.options.group_hierarchy {
                    return children;
                }
                (g.name.clone(), children, vec![], Value::Null, None)
            }
            NodeKind::Entity(e) => {
                let children = self.children(&e.children);
                let key = if self.options.name_property.is_empty() {
                    "targetname"
                } else {
                    &self.options.name_property
                };
                let name = e
                    .property(key)
                    .or_else(|| e.classname())
                    .unwrap_or("Entity")
                    .to_string();
                let properties: serde_json::Map<String, Value> = e
                    .properties
                    .iter()
                    .map(|(k, v)| (k.clone(), json!(v)))
                    .collect();
                let class = e.classname().and_then(|name| self.classes.get(name));
                let mut raw = std::collections::BTreeMap::new();
                if let Some(class) = class {
                    for (name, definition) in &class.properties {
                        if let Some(default) = &definition.default {
                            raw.insert(name.clone(), default.clone());
                        }
                    }
                }
                for (name, value) in &e.properties {
                    raw.insert(name.clone(), value.clone());
                }
                let typed: serde_json::Map<String, Value> = raw
                    .iter()
                    .map(|(name, value)| {
                        let definition = class.and_then(|class| class.properties.get(name));
                        (
                            name.clone(),
                            serde_json::to_value(scene_export::level::Value::from_fgd(
                                definition, value,
                            ))
                            .expect("entity property serializes"),
                        )
                    })
                    .collect();
                let translation = if e.children.is_empty() {
                    e.property("origin").and_then(|s| {
                        let v: Vec<f64> = s
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        (v.len() == 3).then(|| {
                            [
                                v[0] / self.options.scale,
                                v[2] / self.options.scale,
                                -v[1] / self.options.scale,
                            ]
                        })
                    })
                } else {
                    None
                };
                let build = class.map(|class| {
                    serde_json::to_value(&class.directives.entries)
                        .expect("class directives serialize")
                });
                let mut primitives = Vec::new();
                if e.property("_heron_model").is_some() {
                    if let Some(baked) = self.document.imported_model(e) {
                        let path = self.document.imported_model_path(e).unwrap();
                        let raw = e
                            .children
                            .is_empty()
                            .then(|| self.document.imported_model_source(e))
                            .flatten();
                        let model = if let Some(raw) = raw {
                            let key = path.to_string_lossy().into_owned();
                            cached_mesh = self.model_meshes.get(&key).copied();
                            model_key = Some(key);
                            let info = map_model::entity_rotation::RotationInfo {
                                kind: map_model::entity_rotation::RotationType::Euler,
                                property_key: "angles".into(),
                                usage: map_model::entity_rotation::RotationUsage::Allowed,
                            };
                            let rotation =
                                map_model::entity_rotation::rotation(&e.properties, &info);
                            let parts = e
                                .property("modelscale")
                                .unwrap_or("1")
                                .split_whitespace()
                                .filter_map(|p| p.parse::<f64>().ok())
                                .collect::<Vec<_>>();
                            let scale = match parts.as_slice() {
                                [s] => [*s; 3],
                                [x, y, z] => [*x, *y, *z],
                                _ => [1.; 3],
                            };
                            let axes = [
                                Vec3d::new(scale[0], 0., 0.),
                                Vec3d::new(0., 0., scale[2]),
                                Vec3d::new(0., -scale[1], 0.),
                            ];
                            let mut matrix = Vec::new();
                            for a in axes {
                                let p = rotation * a;
                                matrix.extend([p.x(), p.z(), -p.y(), 0.]);
                            }
                            let origin =
                                e.origin().unwrap_or_else(Vec3d::zero) / self.options.scale;
                            matrix.extend([origin.x(), origin.z(), -origin.y(), 1.]);
                            model_matrix = Some(json!(matrix));
                            raw
                        } else {
                            baked
                        };
                        if cached_mesh.is_none() {
                            let materials = model
                                .skins()
                                .enumerate()
                                .map(|(i, _)| {
                                    let name = if i == 0 {
                                        format!("#model/{}", path.to_string_lossy())
                                    } else {
                                        format!("#model/{}#skin/{i}", path.to_string_lossy())
                                    };
                                    self.texture_material(&name, false, None)
                                })
                                .collect::<Vec<_>>();
                            for triangle in &model.triangles {
                                let normal = geometry_math::vec::cross(
                                    triangle.positions[1] - triangle.positions[0],
                                    triangle.positions[2] - triangle.positions[0],
                                );
                                let length = geometry_math::vec::length(normal);
                                if length < 1e-9 {
                                    continue;
                                }
                                let skin = model.skin_at(triangle.material);
                                let uv = triangle.uv.map(|uv| {
                                    [uv.x() / skin.width as f32, uv.y() / skin.height as f32]
                                });
                                primitives.push(self.primitive_with_normals(
                                    &triangle.positions,
                                    normal / length,
                                    triangle.normals,
                                    &uv,
                                    materials[triangle.material],
                                ));
                            }
                        }
                    } else {
                        self.report.warnings.push(
                            "Imported model is unavailable; its entity has no exported mesh".into(),
                        );
                    }
                }
                (
                    name,
                    children,
                    primitives,
                    json!({"entity":properties,"heron":{"version":1,"classname":e.classname(),"properties":typed,"build":build}}),
                    translation,
                )
            }
            NodeKind::Brush(brush) => {
                let mut primitives = Vec::new();
                for (i, face) in brush.faces().iter().enumerate() {
                    if face.export_hidden {
                        continue;
                    }
                    if self.tool(&face.material_name) {
                        continue;
                    }
                    let Some(polygon) = brush.face_polygon(i) else {
                        continue;
                    };
                    if polygon.len() < 3 {
                        continue;
                    }
                    let (material, uv) = self.face(face, &polygon);
                    primitives.push(self.primitive(&polygon, face.plane.normal, &uv, material));
                }
                if primitives.is_empty() {
                    return vec![];
                }
                (
                    format!("Brush {}", node.id.0),
                    vec![],
                    primitives,
                    Value::Null,
                    None,
                )
            }
            NodeKind::Patch(patch) => {
                if self.tool(&patch.material_name) {
                    return vec![];
                }
                let material = self.texture_material(&patch.material_name, true, None);
                let primitives = map_model::patch_mesh::tessellate(
                    patch,
                    map_model::patch_mesh::DEFAULT_SUBDIVISIONS,
                )
                .iter()
                .map(|t| {
                    let uv = t.uv.map(|p| [p.x(), p.y()]);
                    self.primitive(&t.positions, t.normal, &uv, material)
                })
                .collect::<Vec<_>>();
                if primitives.is_empty() {
                    return vec![];
                }
                (
                    format!("Patch {}", node.id.0),
                    vec![],
                    primitives,
                    Value::Null,
                    None,
                )
            }
        };
        let mut value = json!({"name":name});
        if !children.is_empty() {
            value["children"] = json!(children);
        }
        if let Some((mesh, triangles)) = cached_mesh {
            value["mesh"] = json!(mesh);
            self.report.triangles += triangles;
        } else if !primitives.is_empty() {
            let mesh = self.push("meshes", json!({"name":name,"primitives":primitives}));
            value["mesh"] = json!(mesh);
            if let Some(key) = model_key {
                self.model_meshes
                    .insert(key, (mesh, self.report.triangles - before_triangles));
            }
        }
        if !extras.is_null() {
            value["extras"] = extras;
        }
        if let Some(matrix) = model_matrix {
            value["matrix"] = matrix;
        } else if let Some(t) = translation {
            value["translation"] = json!(t);
        }
        vec![self.push("nodes", value)]
    }
}

pub(super) fn build(
    document: &Document,
    directory: &Path,
    options: &Options,
) -> Result<(Report, Vec<(std::path::PathBuf, Contents)>), Error> {
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err(Error::Map(
            "Units per metre must be finite and greater than zero".into(),
        ));
    }
    let map = scene_export::map::parse(&document.to_map_text(), &options.name)
        .map_err(|error| Error::Map(error.to_string()))?;
    let (mut classes, class_warnings) = super::entity_classes(document);
    crate::export_bindings::apply(&map, &mut classes, &options.entities).map_err(Error::Map)?;
    let mut scene = Scene {
        document,
        options,
        root: json!({"asset":{"version":"2.0","generator":"Heron Brush"},"scene":0,"scenes":[],"nodes":[],"meshes":[],"materials":[],"accessors":[],"bufferViews":[],"buffers":[],"images":[],"textures":[],"samplers":[{"wrapS":10497,"wrapT":10497,"magFilter":9729,"minFilter":9729},{"wrapS":33071,"wrapT":33071,"magFilter":9729,"minFilter":9729}]}),
        bin: vec![],
        images: vec![],
        image_cache: HashMap::new(),
        model_meshes: HashMap::new(),
        texture_cache: HashMap::new(),
        materials: HashMap::new(),
        normal_materials: Default::default(),
        report: Report::default(),
        settings: options.settings(directory),
        classes,
    };
    scene.report.warnings.extend(class_warnings);
    let mut roots = Vec::new();
    for layer in &document.tree_world().layers {
        if layer.omit_from_export {
            continue;
        }
        let children = scene.children(&layer.children);
        if options.group_hierarchy && !children.is_empty() {
            roots.push(scene.push("nodes", json!({"name":layer.name,"children":children})));
        } else {
            roots.extend(children);
        }
    }
    scene.root["scenes"] = json!([{"name":options.name}]);
    if !roots.is_empty() {
        scene.root["scenes"][0]["nodes"] = json!(roots);
    }
    if options.collision || options.cull_hidden || options.merge_faces || options.merge_collision {
        scene.report.warnings.push("Portable glTF preserves individual brushes and face paint. Physics generation and mesh optimisations are not applied; skip, clip and origin faces are omitted.".into());
    }
    let mut files = Vec::new();
    for (i, image) in scene.images.clone().into_iter().enumerate() {
        if options.target == Target::Glb {
            while !scene.bin.len().is_multiple_of(4) {
                scene.bin.push(0);
            }
            let start = scene.bin.len();
            scene.bin.extend(&image);
            let view = scene.push(
                "bufferViews",
                json!({"buffer":0,"byteOffset":start,"byteLength":image.len()}),
            );
            scene.root["images"][i] = json!({"bufferView":view,"mimeType":"image/png"});
        } else {
            let path = format!("{}_textures/{i}.png", options.name);
            scene.root["images"][i] = json!({"uri":uri(&path)});
            files.push((directory.join(path), Contents::Bytes(image)));
        }
    }
    if !scene.bin.is_empty() {
        scene.root["buffers"] = json!([{"byteLength":scene.bin.len()}]);
        if options.target == Target::Gltf {
            scene.root["buffers"][0]["uri"] = json!(uri(&format!("{}.bin", options.name)));
        }
    }
    for key in [
        "nodes",
        "meshes",
        "materials",
        "accessors",
        "bufferViews",
        "buffers",
        "images",
        "textures",
    ] {
        if scene.root[key].as_array().is_some_and(|a| a.is_empty()) {
            scene.root.as_object_mut().unwrap().remove(key);
        }
    }
    if scene.images.is_empty() {
        scene.root.as_object_mut().unwrap().remove("samplers");
    }
    if options.target == Target::Glb {
        let json = serde_json::to_vec(&scene.root).unwrap();
        files.push((
            directory.join(format!("{}.glb", options.name)),
            Contents::Bytes(pack(json, scene.bin)),
        ));
    } else {
        files.push((
            directory.join(format!("{}.gltf", options.name)),
            Contents::Text(serde_json::to_string_pretty(&scene.root).unwrap()),
        ));
        if !scene.bin.is_empty() {
            files.push((
                directory.join(format!("{}.bin", options.name)),
                Contents::Bytes(scene.bin),
            ));
        }
    }
    Ok((scene.report, files))
}
/// The name a material is written under. An imported model's skins are looked up by
/// the model's source path, which would put the author's own folders in the file.
fn material_label(name: &str) -> String {
    let Some(rest) = name.strip_prefix("#model/") else {
        return name.to_string();
    };
    let (path, skin) = match rest.split_once("#skin/") {
        Some((path, skin)) => (path, Some(skin)),
        None => (rest, None),
    };
    let stem = Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_string(), |s| s.to_string_lossy().into_owned());
    match skin {
        Some(skin) => format!("{stem} skin {skin}"),
        None => stem,
    }
}

fn uri(path: &str) -> String {
    path.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn pack(mut json: Vec<u8>, mut bin: Vec<u8>) -> Vec<u8> {
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 20 + json.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json);
    if !bin.is_empty() {
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use map_model::brush_face::Rgb;

    fn cube() -> Document {
        let mut doc = Document::new(WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::from_scalars(-32.0, 32.0),
            "__TB_empty",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
        doc
    }
    fn options(target: Target) -> Options {
        Options {
            target,
            name: "Level č 1".into(),
            collision: false,
            cull_hidden: false,
            merge_faces: false,
            merge_collision: false,
            ..Options::default()
        }
    }
    fn unpack(bytes: &[u8]) -> (Value, &[u8]) {
        assert_eq!(&bytes[..4], b"glTF");
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            bytes.len()
        );
        let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json = serde_json::from_slice(&bytes[20..20 + size]).unwrap();
        let bin = if bytes.len() > 20 + size {
            &bytes[28 + size..]
        } else {
            &[]
        };
        (json, bin)
    }
    fn glb(doc: &Document) -> (Report, Value, Vec<u8>) {
        let (report, files) = build(doc, Path::new("."), &options(Target::Glb)).unwrap();
        assert_eq!(files.len(), 1);
        let Contents::Bytes(bytes) = &files[0].1 else {
            panic!()
        };
        let (json, bin) = unpack(bytes);
        (report, json, bin.to_vec())
    }
    fn read(json: &Value, bin: &[u8], index: usize, width: usize) -> Vec<Vec<f32>> {
        let a = &json["accessors"][index];
        let v = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let start = v["byteOffset"].as_u64().unwrap() as usize;
        bin[start..start + a["count"].as_u64().unwrap() as usize * width * 4]
            .chunks_exact(width * 4)
            .map(|p| {
                p.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn imported_model_materials_are_named_without_the_source_folder() {
        assert_eq!(
            material_label("#model//home/someone/props/crate.glb"),
            "crate"
        );
        assert_eq!(
            material_label("#model//home/someone/props/crate.glb#skin/1"),
            "crate skin 1"
        );
        assert_eq!(material_label("stone"), "stone");
    }

    #[test]
    fn a_missing_texture_is_reported_once() {
        // Found in the relay_yard GLB export: one missing texture used with several
        // surface materials gave the same warning once for each.
        let mut doc = Document::new(WorldBounds::default());
        for (offset, roughness) in [(0.0, 10), (128.0, 90)] {
            let brush = map_model::shapes::create_cuboid(
                &doc.world_bounds(),
                &Bbox3d::from_scalars(offset, offset + 32.0),
                "missing",
            )
            .unwrap();
            doc.clear_selection();
            doc.add_brushes(vec![brush]);
            doc.edit_faces(|face| {
                face.surface_material = Some(map_model::brush_face::SurfaceMaterial {
                    roughness,
                    metallic: 0,
                    emission: 0,
                });
            });
        }
        let (report, json, _) = glb(&doc);
        assert_eq!(json["materials"].as_array().unwrap().len(), 2);
        let missing = report
            .warnings
            .iter()
            .filter(|w| w.contains("'missing'"))
            .count();
        assert_eq!(missing, 1, "{:?}", report.warnings);
    }

    #[test]
    fn interchange_extras_keep_authored_strings_and_typed_project_defaults() {
        let mut doc = cube();
        doc.create_point_entity("beacon", Vec3d::new(32., 64., 96.));
        let mut opts = options(Target::Glb);
        opts.entities=serde_json::from_value(json!({"beacon":{"defaults":{"armed":true,"sequence":[2,3,5],"charge":9},"build":{"scene":"res://project-owned.tscn"}}})).unwrap();
        let (_, files) = build(&doc, Path::new("/tmp"), &opts).unwrap();
        let bytes = files
            .iter()
            .find_map(|(_, contents)| match contents {
                Contents::Bytes(bytes) if bytes.starts_with(b"glTF") => Some(bytes),
                _ => None,
            })
            .unwrap();
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let scene: Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
        let entity = scene["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["name"] == "beacon")
            .unwrap();
        assert_eq!(entity["extras"]["entity"]["classname"], "beacon");
        assert_eq!(
            entity["extras"]["heron"]["properties"]["armed"],
            json!({"type":"bool","value":true})
        );
        assert_eq!(
            entity["extras"]["heron"]["properties"]["sequence"]["value"][1],
            json!({"type":"int","value":3})
        );
        assert_eq!(
            entity["extras"]["heron"]["build"]["scene"],
            "res://project-owned.tscn"
        );
    }
    #[test]
    fn radial_and_multicolor_gradient_export_contains_interior_colours() {
        use map_model::brush_face::{ColorStop, GradientStops};
        for radial in [false, true] {
            let mut doc = cube();
            doc.select_all();
            doc.set_face_color(Some(FaceColor::MultiGradient {
                stops: GradientStops::new(&[
                    ColorStop {
                        position: 0.0,
                        color: Rgb::new(255, 0, 0),
                    },
                    ColorStop {
                        position: 0.5,
                        color: Rgb::new(0, 255, 0),
                    },
                    ColorStop {
                        position: 1.0,
                        color: Rgb::new(0, 0, 255),
                    },
                ])
                .unwrap(),
                radial,
                angle: 0.0,
                alpha: 123,
                ends: None,
            }));
            let (_, json, bin) = glb(&doc);
            for image in json["images"].as_array().unwrap() {
                let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
                let start = view["byteOffset"].as_u64().unwrap() as usize;
                let len = view["byteLength"].as_u64().unwrap() as usize;
                let png = image::load_from_memory(&bin[start..start + len])
                    .unwrap()
                    .to_rgba8();
                for channel in 0..3 {
                    assert!(
                        png.pixels().any(|p| p[channel] > 230),
                        "missing colour {channel}"
                    );
                }
                assert!(png.pixels().all(|p| p[3] == 123));
                if radial {
                    let middle = png.get_pixel(png.width() / 2, png.height() / 2);
                    assert!(middle[0] > 230 && middle[2] < 10);
                    assert_eq!(png.get_pixel(0, 0)[2], 255);
                }
            }
        }
    }

    #[test]
    fn untextured_crystal_paint_preserves_transparency_in_glb_and_engine_bake() {
        use map_model::brush_face::{Rgb, SurfaceMaterial};
        for paint in [
            FaceColor::solid(Rgb::new(40, 120, 220)),
            FaceColor::gradient(Rgb::new(40, 120, 220), Rgb::new(150, 220, 255), 90.0),
        ] {
            let mut doc = cube();
            doc.select_all();
            doc.set_face_color(Some(paint.with_alpha(185)));
            doc.set_surface_material(Some(SurfaceMaterial {
                roughness: 12,
                metallic: 0,
                emission: 25,
            }));
            let (_, json, bin) = glb(&doc);
            assert!(json["materials"]
                .as_array()
                .unwrap()
                .iter()
                .all(|m| m["alphaMode"] == "BLEND"));
            for image in json["images"].as_array().unwrap() {
                let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
                let at = view["byteOffset"].as_u64().unwrap() as usize;
                let len = view["byteLength"].as_u64().unwrap() as usize;
                let pixels = image::load_from_memory(&bin[at..at + len])
                    .unwrap()
                    .to_rgba8();
                assert!(pixels.pixels().all(|p| p.0[3] == 185));
            }
            let options = Options::default();
            let mut settings = options.settings(Path::new("/tmp/crystal-alpha"));
            super::super::paint_export::prepare(
                &doc,
                Path::new("/tmp/crystal-alpha"),
                &options,
                &mut settings,
            )
            .unwrap();
            assert!(settings.generated_materials.values().all(|m| m.alpha));
        }
    }
    #[test]
    fn surface_material_exports_distinct_pbr_and_emission_for_same_colour() {
        use map_model::brush_face::{Rgb, SurfaceMaterial};
        let mut doc = cube();
        doc.select_all();
        doc.set_face_color(Some(FaceColor::solid(Rgb::new(200, 100, 20))));
        let id = *doc.selection_ids().iter().next().unwrap();
        doc.set_surface_material_on(
            &[crate::document::FaceRef { node: id, face: 0 }],
            Some(SurfaceMaterial {
                roughness: 20,
                metallic: 80,
                emission: 250,
            }),
        );
        let (_, json, _) = glb(&doc);
        let materials = json["materials"].as_array().unwrap();
        assert_eq!(materials.len(), 2);
        let material = materials
            .iter()
            .find(|m| m.get("emissiveFactor").is_some())
            .unwrap();
        assert!(
            (material["pbrMetallicRoughness"]["roughnessFactor"]
                .as_f64()
                .unwrap()
                - 0.2)
                .abs()
                < 1e-6
        );
        assert!(
            (material["pbrMetallicRoughness"]["metallicFactor"]
                .as_f64()
                .unwrap()
                - 0.8)
                .abs()
                < 1e-6
        );
        assert_eq!(
            material["extensions"]["KHR_materials_emissive_strength"]["emissiveStrength"],
            2.5
        );
        assert_eq!(json["extensionsUsed"][0], "KHR_materials_emissive_strength");
    }
    #[test]
    fn cube_has_metres_outward_triangles_and_groups() {
        let mut doc = cube();
        doc.select_all();
        doc.group_selection("Car");
        let (report, json, bin) = glb(&doc);
        assert_eq!(report.triangles, 12);
        assert!(json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Car"));
        for p in json["meshes"][0]["primitives"].as_array().unwrap() {
            let pos = read(
                &json,
                &bin,
                p["attributes"]["POSITION"].as_u64().unwrap() as usize,
                3,
            );
            let normals = read(
                &json,
                &bin,
                p["attributes"]["NORMAL"].as_u64().unwrap() as usize,
                3,
            );
            for (points, n) in pos
                .as_chunks::<3>()
                .0
                .iter()
                .zip(normals.as_chunks::<3>().0.iter())
            {
                let v: Vec<Vec3d> = points
                    .iter()
                    .map(|p| Vec3d::new(p[0] as f64, p[1] as f64, p[2] as f64))
                    .collect();
                assert!(points.iter().flatten().all(|v| v.abs() <= 1.00001));
                let normal = Vec3d::new(n[0][0] as f64, n[0][1] as f64, n[0][2] as f64);
                assert!(
                    geometry_math::vec::dot(
                        geometry_math::vec::cross(v[1] - v[0], v[2] - v[0]),
                        normal
                    ) > 0.0
                );
            }
        }
    }
    #[test]
    fn painted_gradient_is_embedded_and_external_gltf_matches() {
        let mut doc = cube();
        doc.select_all();
        doc.set_face_color(Some(FaceColor::gradient(
            Rgb::new(230, 70, 30),
            Rgb::new(30, 70, 230),
            90.0,
        )));
        let (_, binary, bin) = glb(&doc);
        let images = binary["images"].as_array().unwrap();
        assert!(!images.is_empty());
        assert!(
            images.len() < 6,
            "identical gradient images should be shared"
        );
        for image in images {
            assert!(image.get("uri").is_none());
            let view = &binary["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
            let start = view["byteOffset"].as_u64().unwrap() as usize;
            let len = view["byteLength"].as_u64().unwrap() as usize;
            let png = image::load_from_memory(&bin[start..start + len])
                .unwrap()
                .to_rgba8();
            assert!(png.pixels().all(|p| p[3] == 255));
            let first = png.get_pixel(0, 0);
            let last = png.get_pixel(png.width() - 1, png.height() - 1);
            assert_ne!(first, last);
        }
        let (_, files) = build(&doc, Path::new("."), &options(Target::Gltf)).unwrap();
        let json = files
            .iter()
            .find_map(|(_, c)| {
                if let Contents::Text(s) = c {
                    Some(serde_json::from_str::<Value>(s).unwrap())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(json["nodes"], binary["nodes"]);
        assert_eq!(json["meshes"], binary["meshes"]);
        assert_eq!(json["buffers"][0]["uri"], "Level%20%C4%8D%201.bin");
        assert_eq!(files.len(), images.len() + 2);
    }
    #[test]
    fn skip_and_omitted_layers_do_not_export() {
        let mut doc = cube();
        doc.select_all();
        doc.apply_current_material();
        doc.edit_faces(|face| face.material_name = "skip".into());
        assert_eq!(glb(&doc).0.triangles, 0);
        let mut doc = cube();
        doc.set_layer_omitted(0, true);
        assert_eq!(glb(&doc).0.triangles, 0);
    }
    #[test]
    fn empty_scene_and_check_write_no_resources() {
        let doc = Document::new(WorldBounds::default());
        let (_, json, bin) = glb(&doc);
        assert!(bin.is_empty());
        assert!(json.get("buffers").is_none());
        assert!(json["scenes"][0].get("nodes").is_none());
        let opts = options(Target::Glb);
        let path = std::env::temp_dir().join(format!("heron-gltf-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        super::super::check(&doc, &path, &opts).unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn loaded_texture_and_translucent_paint_are_portable() {
        let mut doc = cube();
        let dir = std::env::temp_dir().join(format!("heron-gltf-texture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 200, 255]))
            .save(dir.join("blue.png"))
            .unwrap();
        doc.load_materials(&dir).unwrap();
        doc.edit_faces(|face| face.material_name = "blue".into());
        let (_, json, bin) = glb(&doc);
        assert_eq!(json["images"].as_array().unwrap().len(), 1);
        let decode = |json: &Value, bin: &[u8]| {
            let view =
                &json["bufferViews"][json["images"][0]["bufferView"].as_u64().unwrap() as usize];
            let start = view["byteOffset"].as_u64().unwrap() as usize;
            let len = view["byteLength"].as_u64().unwrap() as usize;
            image::load_from_memory(&bin[start..start + len])
                .unwrap()
                .to_rgba8()
        };
        assert_eq!(decode(&json, &bin).get_pixel(0, 0).0, [20, 40, 200, 255]);
        doc.select_all();
        doc.set_face_color(Some(FaceColor::Solid {
            color: Rgb::new(220, 40, 20),
            alpha: 128,
        }));
        let (_, json, bin) = glb(&doc);
        let pixel = decode(&json, &bin).get_pixel(0, 0).0;
        assert_eq!(
            pixel,
            [120, 40, 110, 255],
            "paint alpha blends over the texture, not through the surface"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn entity_position_properties_and_flattening_survive() {
        let mut doc = cube();
        doc.select_all();
        doc.group_selection("Geometry");
        doc.create_point_entity("info_player_start", Vec3d::new(32.0, 64.0, 96.0));
        let (_, json, _) = glb(&doc);
        let entity = json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["name"] == "info_player_start")
            .unwrap();
        assert_eq!(entity["translation"], json!([1.0, 3.0, -2.0]));
        assert_eq!(entity["extras"]["entity"]["classname"], "info_player_start");
        let mut opts = options(Target::Glb);
        opts.group_hierarchy = false;
        let (_, files) = build(&doc, Path::new("."), &opts).unwrap();
        let Contents::Bytes(bytes) = &files[0].1 else {
            panic!()
        };
        let (json, _) = unpack(bytes);
        assert!(!json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Geometry"));
    }
    #[test]
    fn patch_exports_its_tessellation_as_double_sided_geometry() {
        let path =
            std::env::temp_dir().join(format!("heron-gltf-patch-{}.map", std::process::id()));
        std::fs::write(
            &path,
            r#"{
"classname" "worldspawn"
{
patchDef2
{
__TB_empty
( 3 3 0 0 0 )
(
( ( -64 -64 0 0 0 ) ( -64 0 32 0 0.5 ) ( -64 64 0 0 1 ) )
( ( 0 -64 32 0.5 0 ) ( 0 0 64 0.5 0.5 ) ( 0 64 32 0.5 1 ) )
( ( 64 -64 0 1 0 ) ( 64 0 32 1 0.5 ) ( 64 64 0 1 1 ) )
)
}
}
}"#,
        )
        .unwrap();
        let doc = Document::open(&path, WorldBounds::default()).unwrap();
        let (report, json, _) = glb(&doc);
        assert_eq!(report.triangles, 72);
        assert_eq!(json["materials"][0]["doubleSided"], true);
        std::fs::remove_file(path).unwrap();
    }
}
