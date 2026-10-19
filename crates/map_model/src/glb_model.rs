//! Static glTF 2 binary models. Unsupported deformation/compression is an error.
use crate::entity_model::{EntityModel, ModelTriangle, Skin};
use geometry_math::{
    bbox::Bbox3d,
    vec::{Vec2f, Vec3d},
};
use serde_json::Value;
type Matrix = [f64; 16];
fn identity() -> Matrix {
    [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ]
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| (0..4).map(|k| a[k * 4 + i % 4] * b[i / 4 * 4 + k]).sum())
}
fn point(m: Matrix, p: [f64; 3]) -> Vec3d {
    let q: [f64; 3] =
        std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r]);
    Vec3d::new(q[0], -q[2], q[1])
}
fn transform_normal(m: Matrix, n: &[f64]) -> Result<Vec3d, String> {
    use geometry_math::vec;
    let a = Vec3d::new(m[0], m[1], m[2]);
    let b = Vec3d::new(m[4], m[5], m[6]);
    let c = Vec3d::new(m[8], m[9], m[10]);
    let determinant = vec::dot(a, vec::cross(b, c));
    if !determinant.is_finite() || determinant == 0.0 {
        return Err("GLB normal transform is singular".into());
    }
    let q =
        (vec::cross(b, c) * n[0] + vec::cross(c, a) * n[1] + vec::cross(a, b) * n[2]) / determinant;
    let length = vec::length(q);
    if !length.is_finite() || length < 1e-12 {
        return Err("Invalid GLB normal".into());
    }
    Ok(Vec3d::new(q.x(), -q.z(), q.y()) / length)
}
fn index(v: &Value) -> Result<usize, String> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| "Invalid GLB index".into())
}
fn at<'a>(root: &'a Value, name: &str, n: usize) -> Result<&'a Value, String> {
    root[name]
        .as_array()
        .and_then(|a| a.get(n))
        .ok_or_else(|| format!("Missing GLB {name}[{n}]"))
}
fn u32at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or("Truncated GLB")?
            .try_into()
            .unwrap(),
    ))
}
fn array<const N: usize>(v: &Value, default: [f64; N]) -> Result<[f64; N], String> {
    if v.is_null() {
        return Ok(default);
    }
    let a = v
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or("Invalid GLB transform")?;
    let mut out = default;
    for (i, v) in a.iter().enumerate() {
        out[i] = v
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or("Invalid GLB number")?
    }
    Ok(out)
}
fn transform(node: &Value) -> Result<Matrix, String> {
    if !node["matrix"].is_null() {
        return array(&node["matrix"], identity());
    }
    let [x, y, z, w] = array(&node["rotation"], [0., 0., 0., 1.])?;
    let [sx, sy, sz] = array(&node["scale"], [1.; 3])?;
    let [tx, ty, tz] = array(&node["translation"], [0.; 3])?;
    Ok([
        (1. - 2. * (y * y + z * z)) * sx,
        2. * (x * y + z * w) * sx,
        2. * (x * z - y * w) * sx,
        0.,
        2. * (x * y - z * w) * sy,
        (1. - 2. * (x * x + z * z)) * sy,
        2. * (y * z + x * w) * sy,
        0.,
        2. * (x * z + y * w) * sz,
        2. * (y * z - x * w) * sz,
        (1. - 2. * (x * x + y * y)) * sz,
        0.,
        tx,
        ty,
        tz,
        1.,
    ])
}
fn accessor(
    root: &Value,
    bin: &[u8],
    n: usize,
    components: usize,
) -> Result<Vec<Vec<f64>>, String> {
    let a = at(root, "accessors", n)?;
    if !a["sparse"].is_null() {
        return Err("Sparse GLB accessors are not supported".into());
    }
    let kind = match components {
        1 => "SCALAR",
        2 => "VEC2",
        3 => "VEC3",
        _ => return Err("Invalid component count".into()),
    };
    if a["type"].as_str() != Some(kind) {
        return Err("Invalid GLB accessor type".into());
    }
    let view = at(root, "bufferViews", index(&a["bufferView"])?)?;
    if view["buffer"].as_u64() != Some(0) {
        return Err("External GLB buffers are not supported".into());
    }
    let count = index(&a["count"])?;
    if count > 3_000_000 {
        return Err("GLB accessor exceeds 3 million elements".into());
    }
    let component = a["componentType"]
        .as_u64()
        .ok_or("Missing component type")?;
    let size = match component {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        _ => return Err("Unsupported GLB component type".into()),
    };
    let begin = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let len = index(&view["byteLength"])?;
    let viewbytes = bin
        .get(begin..begin.checked_add(len).ok_or("GLB offset overflow")?)
        .ok_or("GLB buffer view is out of bounds")?;
    let offset = a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"]
        .as_u64()
        .unwrap_or((size * components) as u64) as usize;
    if stride < size * components {
        return Err("Invalid GLB stride".into());
    }
    let normalized = a["normalized"].as_bool().unwrap_or(false);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = i
            .checked_mul(stride)
            .and_then(|n| n.checked_add(offset))
            .ok_or("GLB offset overflow")?;
        let row = viewbytes
            .get(
                start
                    ..start
                        .checked_add(size * components)
                        .ok_or("GLB offset overflow")?,
            )
            .ok_or("GLB accessor is out of bounds")?;
        let mut values = Vec::with_capacity(components);
        for j in 0..components {
            let b = &row[j * size..(j + 1) * size];
            let value = match component {
                5120 => {
                    let v = b[0] as i8 as f64;
                    if normalized { (v / 127.).max(-1.) } else { v }
                }
                5121 => {
                    let v = b[0] as f64;
                    if normalized { v / 255. } else { v }
                }
                5122 => {
                    let v = i16::from_le_bytes(b.try_into().unwrap()) as f64;
                    if normalized { (v / 32767.).max(-1.) } else { v }
                }
                5123 => {
                    let v = u16::from_le_bytes(b.try_into().unwrap()) as f64;
                    if normalized { v / 65535. } else { v }
                }
                5125 => u32::from_le_bytes(b.try_into().unwrap()) as f64,
                5126 => f32::from_le_bytes(b.try_into().unwrap()) as f64,
                _ => unreachable!(),
            };
            if !value.is_finite() {
                return Err("Non-finite GLB vertex".into());
            }
            values.push(value)
        }
        out.push(values);
    }
    Ok(out)
}
fn texture(root: &Value, bin: &[u8], info: &Value) -> Result<Option<Skin>, String> {
    if info.is_null() {
        return Ok(None);
    }
    if info["texCoord"].as_u64().unwrap_or(0) != 0 || !info["extensions"].is_null() {
        return Err("GLB texture transforms and alternate UV sets are not supported".into());
    }
    let image = at(
        root,
        "images",
        index(&at(root, "textures", index(&info["index"])?)?["source"])?,
    )?;
    if !image["uri"].is_null() {
        return Err("GLB textures must be embedded".into());
    }
    let view = at(root, "bufferViews", index(&image["bufferView"])?)?;
    if view["buffer"].as_u64() != Some(0) {
        return Err("External GLB image buffers are not supported".into());
    }
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let len = index(&view["byteLength"])?;
    let data = bin
        .get(start..start.checked_add(len).ok_or("GLB offset overflow")?)
        .ok_or("GLB image is out of bounds")?;
    let decoded = image::load_from_memory(data)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    Ok(Some(Skin {
        width: decoded.width(),
        height: decoded.height(),
        pixels: decoded.into_raw(),
    }))
}
fn factor(value: &Value, default: f32, bounded: bool) -> Result<f32, String> {
    if value.is_null() {
        return Ok(default);
    }
    let f = value
        .as_f64()
        .filter(|f| f.is_finite())
        .ok_or("Invalid GLB material factor")? as f32;
    if !f.is_finite() || (bounded && !(0.0..=1.0).contains(&f)) {
        return Err("Invalid GLB material factor".into());
    }
    Ok(f)
}
fn tint_srgb(byte: u8, factor: f64) -> u8 {
    let s = byte as f64 / 255.;
    let linear = if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    } * factor;
    let s = if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1. / 2.4) - 0.055
    };
    (s.clamp(0., 1.) * 255.).round() as u8
}

/// Reads the active static scene, applying node transforms and embedded base-colour texture.
pub fn read(bytes: &[u8]) -> Result<EntityModel, String> {
    if bytes.len() > 256 * 1024 * 1024 {
        return Err("GLB exceeds 256 MiB".into());
    }
    if bytes.get(..4) != Some(b"glTF")
        || u32at(bytes, 4)? != 2
        || u32at(bytes, 8)? as usize != bytes.len()
    {
        return Err("Not a complete glTF 2 binary file".into());
    }
    let mut pos = 12usize;
    let mut json = None;
    let mut bin = None;
    while pos < bytes.len() {
        let size = u32at(bytes, pos)? as usize;
        let kind = u32at(bytes, pos + 4)?;
        pos = pos.checked_add(8).ok_or("GLB offset overflow")?;
        let end = pos.checked_add(size).ok_or("GLB offset overflow")?;
        let chunk = bytes.get(pos..end).ok_or("Truncated GLB chunk")?;
        match kind {
            0x4e4f534a => {
                if json.replace(chunk).is_some() {
                    return Err("Duplicate GLB JSON".into());
                }
            }
            0x004e4942 => {
                if bin.replace(chunk).is_some() {
                    return Err("Duplicate GLB binary".into());
                }
            }
            _ => {}
        }
        pos = end;
    }
    let root: Value =
        serde_json::from_slice(json.ok_or("Missing GLB JSON")?).map_err(|e| e.to_string())?;
    let bin = bin.ok_or("Missing GLB binary")?;
    if root["asset"]["version"].as_str() != Some("2.0") {
        return Err("Unsupported glTF version".into());
    }
    if root["extensionsRequired"]
        .as_array()
        .is_some_and(|a| !a.is_empty())
    {
        return Err("Required GLB extensions are not supported".into());
    }
    let scene = at(
        &root,
        "scenes",
        root["scene"].as_u64().unwrap_or(0) as usize,
    )?;
    let nodes = scene["nodes"].as_array().ok_or("Missing GLB scene nodes")?;
    let mut triangles = vec![];
    let mut material = Vec::new();
    fn visit(
        root: &Value,
        bin: &[u8],
        n: usize,
        parent: Matrix,
        path: &mut Vec<usize>,
        triangles: &mut Vec<ModelTriangle>,
        material: &mut Vec<Option<usize>>,
    ) -> Result<(), String> {
        if path.len() >= 128 || path.contains(&n) {
            return Err("Cyclic or too deeply nested GLB nodes".into());
        }
        path.push(n);
        let node = at(root, "nodes", n)?;
        if !node["skin"].is_null() {
            return Err("Skinned GLB models are not supported".into());
        }
        let matrix = multiply(parent, transform(node)?);
        if !node["mesh"].is_null() {
            let mesh = at(root, "meshes", index(&node["mesh"])?)?;
            for primitive in mesh["primitives"]
                .as_array()
                .ok_or("Missing GLB primitives")?
            {
                if primitive["mode"].as_u64().unwrap_or(4) != 4
                    || primitive["targets"]
                        .as_array()
                        .is_some_and(|a| !a.is_empty())
                {
                    return Err("GLB import requires static triangle primitives".into());
                }
                let mat = primitive["material"].as_u64().map(|n| n as usize);
                let material_index = material.iter().position(|m| *m == mat).unwrap_or_else(|| {
                    material.push(mat);
                    material.len() - 1
                });
                let positions =
                    accessor(root, bin, index(&primitive["attributes"]["POSITION"])?, 3)?;
                let uv = if primitive["attributes"]["TEXCOORD_0"].is_null() {
                    vec![vec![0.; 2]; positions.len()]
                } else {
                    accessor(root, bin, index(&primitive["attributes"]["TEXCOORD_0"])?, 2)?
                };
                if uv.len() != positions.len() {
                    return Err("GLB vertex and UV counts differ".into());
                }
                let normals = if primitive["attributes"]["NORMAL"].is_null() {
                    None
                } else {
                    let normals =
                        accessor(root, bin, index(&primitive["attributes"]["NORMAL"])?, 3)?;
                    if normals.len() != positions.len() {
                        return Err("GLB vertex and normal counts differ".into());
                    }
                    Some(normals)
                };
                let indices = if primitive["indices"].is_null() {
                    (0..positions.len()).collect::<Vec<_>>()
                } else {
                    let a = at(root, "accessors", index(&primitive["indices"])?)?;
                    if !matches!(a["componentType"].as_u64(), Some(5121 | 5123 | 5125))
                        || a["normalized"].as_bool() == Some(true)
                    {
                        return Err("Invalid GLB index component type".into());
                    }
                    accessor(root, bin, index(&primitive["indices"])?, 1)?
                        .iter()
                        .map(|row| row[0] as usize)
                        .collect()
                };
                if indices.len() % 3 != 0 || triangles.len() + indices.len() / 3 > 1_000_000 {
                    return Err("Invalid or excessive GLB triangles".into());
                }
                let determinant = matrix[0] * (matrix[5] * matrix[10] - matrix[9] * matrix[6])
                    - matrix[4] * (matrix[1] * matrix[10] - matrix[9] * matrix[2])
                    + matrix[8] * (matrix[1] * matrix[6] - matrix[5] * matrix[2]);
                for source_ids in indices.chunks_exact(3) {
                    let ids = if determinant < 0.0 {
                        [source_ids[0], source_ids[2], source_ids[1]]
                    } else {
                        [source_ids[0], source_ids[1], source_ids[2]]
                    };
                    if ids.iter().any(|i| *i >= positions.len()) {
                        return Err("GLB index is out of range".into());
                    }
                    triangles.push(ModelTriangle {
                        material: material_index,
                        normals: normals
                            .as_ref()
                            .map(|normals| {
                                ids.map(|i| transform_normal(matrix, &normals[i]))
                                    .into_iter()
                                    .collect::<Result<Vec<_>, _>>()
                                    .map(|n| [n[0], n[1], n[2]])
                            })
                            .transpose()?,
                        positions: std::array::from_fn(|i| {
                            let p = &positions[ids[i]];
                            point(matrix, [p[0], p[1], p[2]])
                        }),
                        uv: std::array::from_fn(|i| {
                            Vec2f::new(uv[ids[i]][0] as f32, uv[ids[i]][1] as f32)
                        }),
                    });
                }
            }
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                visit(root, bin, index(child)?, matrix, path, triangles, material)?
            }
        }
        path.pop();
        Ok(())
    }
    for node in nodes {
        visit(
            &root,
            bin,
            index(node)?,
            identity(),
            &mut vec![],
            &mut triangles,
            &mut material,
        )?
    }
    if triangles
        .iter()
        .flat_map(|t| t.positions)
        .any(|p| ![p.x(), p.y(), p.z()].iter().all(|n| n.is_finite()))
    {
        return Err("GLB transform produces non-finite positions".into());
    }
    if triangles.is_empty() {
        return Err("GLB scene has no triangles".into());
    }
    let mut skins = Vec::new();
    let mut pbr_materials = Vec::new();
    for material in material {
        let mut skin = Skin {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        let mut imported = crate::model_pbr::ModelPbr::default();
        if let Some(n) = material {
            let mat = at(&root, "materials", n)?;
            imported.alpha_mode = match mat.get("alphaMode") {
                None => crate::material::Alpha::Opaque,
                Some(Value::String(mode)) if mode == "OPAQUE" => crate::material::Alpha::Opaque,
                Some(Value::String(mode)) if mode == "MASK" => crate::material::Alpha::Masked,
                Some(Value::String(mode)) if mode == "BLEND" => crate::material::Alpha::Blended,
                _ => return Err("Invalid GLB alpha mode".into()),
            };
            imported.alpha_cutoff = factor(&mat["alphaCutoff"], 0.5, false)?;
            if imported.alpha_cutoff < 0.0 { return Err("Invalid GLB alpha cutoff".into()); }
            imported.double_sided = match mat.get("doubleSided") {
                None => false,
                Some(Value::Bool(value)) => *value,
                _ => return Err("Invalid GLB double-sided flag".into()),
            };
            let pbr = &mat["pbrMetallicRoughness"];
            let color = array(&pbr["baseColorFactor"], [1.; 4])?;
            skin = texture(&root, bin, &pbr["baseColorTexture"])?.unwrap_or(Skin {
                width: 1,
                height: 1,
                pixels: vec![255; 4],
            });
            imported.roughness = factor(&pbr["roughnessFactor"], 1., true)?;
            imported.metallic = factor(&pbr["metallicFactor"], 1., true)?;
            imported.normal_scale = factor(&mat["normalTexture"]["scale"], 1., false)?;
            imported.occlusion_strength = factor(&mat["occlusionTexture"]["strength"], 1., true)?;
            imported.emissive_factor = array(&mat["emissiveFactor"], [0.; 3])?.map(|n| n as f32);
            if imported
                .emissive_factor
                .iter()
                .any(|f| !(0.0..=1.0).contains(f))
            {
                return Err("Invalid GLB emissive factor".into());
            }
            imported.normal = texture(&root, bin, &mat["normalTexture"])?.map(std::sync::Arc::new);
            imported.metallic_roughness =
                texture(&root, bin, &pbr["metallicRoughnessTexture"])?.map(std::sync::Arc::new);
            imported.occlusion =
                texture(&root, bin, &mat["occlusionTexture"])?.map(std::sync::Arc::new);
            imported.emissive =
                texture(&root, bin, &mat["emissiveTexture"])?.map(std::sync::Arc::new);
            for pixel in skin.pixels.chunks_exact_mut(4) {
                for i in 0..4 {
                    pixel[i] = if i < 3 {
                        tint_srgb(pixel[i], color[i].clamp(0., 1.))
                    } else {
                        (pixel[i] as f64 * color[i].clamp(0., 1.)).round() as u8
                    }
                }
            }
        }
        skins.push(skin);
        pbr_materials.push(imported);
    }
    let mut bounds = Bbox3d::new(triangles[0].positions[0], triangles[0].positions[0]);
    for triangle in &mut triangles {
        for p in triangle.positions {
            bounds = bounds.merged_with_point(p)
        }
        let skin = &skins[triangle.material];
        for uv in &mut triangle.uv {
            *uv = Vec2f::new(uv.x() * skin.width as f32, uv.y() * skin.height as f32)
        }
    }
    let skin = skins.remove(0);
    Ok(EntityModel {
        triangles,
        skin,
        extra_skins: skins,
        pbr: pbr_materials,
        bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(index: u16) -> Vec<u8> {
        let mut bin = vec![];
        for n in [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            bin.extend(n.to_le_bytes())
        }
        for n in [0_u16, 1, index] {
            bin.extend(n.to_le_bytes())
        }
        while bin.len() % 4 != 0 {
            bin.push(0)
        }
        let value = serde_json::json!({"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"translation":[2,3,4],"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}],"buffers":[{"byteLength":bin.len()}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}]});
        encode(value, bin)
    }
    fn encode(value: Value, bin: Vec<u8>) -> Vec<u8> {
        let mut json = serde_json::to_vec(&value).unwrap();
        while json.len() % 4 != 0 {
            json.push(b' ')
        }
        let size = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = b"glTF".to_vec();
        out.extend(2_u32.to_le_bytes());
        out.extend((size as u32).to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend(0x4e4f534a_u32.to_le_bytes());
        out.extend(json);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(0x004e4942_u32.to_le_bytes());
        out.extend(bin);
        out
    }
    #[test]
    fn invalid_material_rules_are_rejected_without_restricting_valid_cutoffs() {
        let bytes = fixture(2);
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let root: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        let binary = bytes[28 + json_len..].to_vec();
        for material in [serde_json::json!({"alphaMode":"UNKNOWN"}),
            serde_json::json!({"alphaMode":2}), serde_json::json!({"alphaCutoff":-0.1}),
            serde_json::json!({"doubleSided":"true"})] {
            let mut root = root.clone();
            root["materials"] = serde_json::json!([material]);
            root["meshes"][0]["primitives"][0]["material"] = serde_json::json!(0);
            assert!(read(&encode(root, binary.clone())).is_err());
        }
        let mut root = root;
        root["materials"] = serde_json::json!([{"alphaMode":"MASK","alphaCutoff":2.0}]);
        root["meshes"][0]["primitives"][0]["material"] = serde_json::json!(0);
        assert_eq!(read(&encode(root, binary)).unwrap().pbr[0].alpha_cutoff, 2.0);
    }

    #[test]
    fn source_material_modes_cutoff_and_sidedness_survive_import() {
        use crate::material::Alpha;
        let model = read(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../examples/editor-checks/fixtures/material_modes.glb"))).unwrap();
        assert_eq!(model.pbr.len(), 6);
        for (index, material) in model.pbr.iter().enumerate() {
            assert_eq!(material.alpha_mode, [Alpha::Opaque, Alpha::Masked, Alpha::Blended][index / 2]);
            assert_eq!(material.double_sided, index % 2 == 1);
            assert_eq!(material.alpha_cutoff, if index / 2 == 1 { 0.42 } else { 0.5 });
        }
        // Identical texture pixels must not merge materials with different rules.
        let hashes: std::collections::HashSet<_> = model.pbr.iter().map(|material| {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            material.hash(&mut hash); hash.finish()
        }).collect();
        assert_eq!(hashes.len(), 6);
    }

    #[test]
    fn pbr_factors_channels_and_map_dimensions_are_retained() {
        let model = read(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/editor-checks/fixtures/pbr.glb"
        )))
        .unwrap();
        assert_eq!(model.pbr.len(), 2);
        let p = &model.pbr[0];
        assert_eq!(
            (
                p.roughness,
                p.metallic,
                p.normal_scale,
                p.occlusion_strength
            ),
            (0.6, 0.4, 0.35, 0.25)
        );
        assert_eq!(p.emissive_factor, [0.2, 0.4, 0.6]);
        assert_eq!(
            (
                p.normal.as_ref().unwrap().width,
                p.normal.as_ref().unwrap().height
            ),
            (2, 2)
        );
        assert_eq!(
            (
                p.metallic_roughness.as_ref().unwrap().width,
                p.metallic_roughness.as_ref().unwrap().height
            ),
            (2, 1)
        );
        assert_eq!(
            (
                p.occlusion.as_ref().unwrap().width,
                p.occlusion.as_ref().unwrap().height
            ),
            (1, 2)
        );
        assert_eq!(&p.preview_orm().pixels[..4], &[64, 128, 192, 255]);
        assert!(
            model.skin.pixels[0] > 120 && model.skin.pixels[0] < 140,
            "base factor must multiply linear light"
        );
        assert_eq!(model.pbr[1].roughness, 0.9);
    }

    #[test]
    fn distinct_materials_and_inverse_transpose_normals_survive_mirroring() {
        let data = fixture(2);
        let json_len = u32at(&data, 12).unwrap() as usize;
        let mut root: Value = serde_json::from_slice(&data[20..20 + json_len]).unwrap();
        let mut bin = data[28 + json_len..].to_vec();
        let offset = bin.len();
        for _ in 0..3 {
            for v in [1_f32, 1., 1.] {
                bin.extend(v.to_le_bytes());
            }
        }
        root["buffers"][0]["byteLength"] = bin.len().into();
        root["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"buffer":0,"byteOffset":offset,"byteLength":36}));
        root["accessors"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"bufferView":2,"componentType":5126,"count":3,"type":"VEC3"}));
        root["nodes"][0]["scale"] = serde_json::json!([-2, 3, 4]);
        root["meshes"][0]["primitives"][0]["attributes"]["NORMAL"] = 2.into();
        root["meshes"][0]["primitives"][0]["material"] = 0.into();
        let mut second = root["meshes"][0]["primitives"][0].clone();
        second["material"] = 1.into();
        root["meshes"][0]["primitives"]
            .as_array_mut()
            .unwrap()
            .push(second);
        root["materials"] = serde_json::json!([
            {"pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1]}},
            {"pbrMetallicRoughness":{"baseColorFactor":[0,1,0,1]}}
        ]);
        let model = read(&encode(root, bin)).unwrap();
        assert_eq!(model.triangles.len(), 2);
        assert_eq!(model.skin.pixels, [255, 0, 0, 255]);
        assert_eq!(model.extra_skins[0].pixels, [0, 255, 0, 255]);
        assert_eq!(model.triangles[1].material, 1);
        let expected = geometry_math::vec::normalize(Vec3d::new(-0.5, -0.25, 1.0 / 3.0));
        let actual = model.triangles[0].normals.unwrap()[0];
        assert!(geometry_math::vec::length(expected - actual) < 1e-9);
        let mut singular = identity();
        singular[0] = 0.0;
        assert!(transform_normal(singular, &[1.0, 0.0, 0.0]).is_err());
    }

    #[test]
    fn active_scene_transform_and_axis_conversion_are_applied() {
        let model = read(&fixture(2)).unwrap();
        assert_eq!(model.triangles.len(), 1);
        assert_eq!(model.triangles[0].positions[0], Vec3d::new(2., -4., 3.));
        assert_eq!(model.bounds.max, Vec3d::new(3., -4., 4.));
    }
    #[test]
    fn malformed_indices_lengths_and_accessors_are_rejected() {
        assert!(read(&fixture(50)).is_err());
        let mut data = fixture(2);
        data.pop();
        assert!(read(&data).is_err());
        assert!(read(b"not a model").is_err());
    }
}
