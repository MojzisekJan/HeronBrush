//! Generated metallic/roughness materials and common map lights for the baseline renderer.
use super::{escape, f, join_resource, DefoldFile, DefoldOptions};
use crate::{
    level::{Entity, Level, Texture},
    materials::AlphaMode,
};

pub(super) fn generated(entity: &Entity, options: &DefoldOptions) -> bool {
    !entity.surfaces.is_empty()
        && options.model_material == "/builtins/materials/model.material"
        && options.model_sampler == "tex0"
        && !entity
            .properties
            .keys()
            .any(|k| k == "defold_material" || k.starts_with("defold_sampler"))
        && !entity.properties.contains_key("_heron_water")
}
fn maps<'a>(texture: &'a Texture, options: &'a DefoldOptions) -> &'a [(String, String)] {
    options
        .material_maps
        .get(&texture.name)
        .map(Vec::as_slice)
        .unwrap_or(&texture.maps)
}
pub(super) fn bindings(texture: &Texture, options: &DefoldOptions) -> Vec<(String, String)> {
    let maps = maps(texture, options);
    let mut bindings = Vec::new();
    for slot in [
        "normal",
        "mr",
        "orm",
        "roughness",
        "metallic",
        "ao",
        "emissive",
    ] {
        if (slot == "orm" && maps.iter().any(|(s, _)| s == "mr"))
            || ((slot == "roughness" || slot == "metallic")
                && maps.iter().any(|(s, _)| s == "mr" || s == "orm"))
        {
            continue;
        }
        if let Some((_, path)) = maps
            .iter()
            .find(|(s, _)| s == slot || (slot == "emissive" && s == "emission"))
        {
            bindings.push((
                format!("tex_{slot}"),
                join_resource(&options.texture_prefix, path),
            ));
        }
    }
    bindings
}
pub(super) fn files(level: &Level, options: &DefoldOptions) -> Vec<DefoldFile> {
    let mut files = Vec::new();
    let dir = options.asset_dir.trim_end_matches('/');
    if level.entities.iter().any(|e| generated(e, options)) {
        files.push(DefoldFile {
            path: format!("{}/heron_model.vp", super::ident(&level.name)),
            contents: include_str!("defold_model.vp").into(),
        });
        let mut programs = std::collections::BTreeSet::new();
        for (index, texture) in level.textures.iter().enumerate() {
            let pbr = options.material_pbr.get(&texture.name);
            let surface = texture.surface.unwrap_or([1.0, 0.0, 0.0]).map(f64::from);
            let rough = pbr.map_or(surface[0], |p| p.roughness as f64);
            let metal = pbr.map_or(
                if texture.surface.is_none()
                    && maps(texture, options)
                        .iter()
                        .any(|(s, _)| s == "mr" || s == "orm" || s == "metallic")
                {
                    1.
                } else {
                    surface[1]
                },
                |p| p.metallic as f64,
            );
            let emission = pbr.map_or(
                if maps(texture, options)
                    .iter()
                    .any(|(s, _)| s == "emissive" || s == "emission")
                {
                    [1.; 3]
                } else {
                    [0.; 3]
                },
                |p| p.emissive_factor.map(f64::from),
            );
            let alpha = pbr.map_or(
                if options.material_alpha.contains(&texture.name) {
                    2
                } else {
                    1
                },
                |p| match p.alpha_mode {
                    AlphaMode::Opaque => 0,
                    AlphaMode::Mask => 1,
                    AlphaMode::Blend => 2,
                },
            );
            let double = pbr.is_some_and(|p| p.double_sided);
            let tag = if alpha == 2 {
                "heron_transparent"
            } else if double {
                "heron_double_sided"
            } else {
                "model"
            };
            let mut defines = String::new();
            let mut samplers = Vec::new();
            if texture.image.is_some() {
                defines.push_str("#define HB_ALBEDO\n");
                samplers.push("tex0".into());
            }
            for (sampler, _) in bindings(texture, options) {
                defines.push_str(&format!(
                    "#define HB_{}\n",
                    sampler.trim_start_matches("tex_").to_uppercase()
                ));
                samplers.push(sampler);
            }
            let variant = samplers.iter().fold(0u16, |mask, sampler| {
                mask | match sampler.as_str() {
                    "tex0" => 1,
                    "tex_normal" => 2,
                    "tex_mr" => 4,
                    "tex_orm" => 8,
                    "tex_roughness" => 16,
                    "tex_metallic" => 32,
                    "tex_ao" => 64,
                    "tex_emissive" => 128,
                    _ => 0,
                }
            });
            let fp = format!("heron_model_{variant:02x}.fp");
            if programs.insert(variant) {
                files.push(DefoldFile {
                    path: format!("{}/{fp}", super::ident(&level.name)),
                    contents: defines + include_str!("defold_model.fp"),
                });
            }
            let mut material=format!("name: \"heron_{index}\"\ntags: \"{tag}\"\nvertex_program: \"{dir}/heron_model.vp\"\nfragment_program: \"{dir}/{fp}\"\nvertex_space: VERTEX_SPACE_LOCAL\n");
            for (name, kind) in [
                ("mtx_world", "WORLD"),
                ("mtx_worldview", "WORLDVIEW"),
                ("mtx_proj", "PROJECTION"),
            ] {
                material.push_str(&format!(
                    "vertex_constants {{name: \"{name}\" type: CONSTANT_TYPE_{kind}}}\n"
                ));
            }
            let mut constant = |name: &str, values: [f64; 4]| {
                material.push_str(&format!("fragment_constants {{name: \"{name}\" type: CONSTANT_TYPE_USER value {{x:{} y:{} z:{} w:{}}}}}\n",f(values[0]),f(values[1]),f(values[2]),f(values[3])));
            };
            constant(
                "surface_params",
                [
                    rough,
                    metal,
                    if pbr.is_some() { 0. } else { surface[2] },
                    pbr.map_or(1., |p| p.normal_scale as f64),
                ],
            );
            constant(
                "emission_params",
                [emission[0], emission[1], emission[2], alpha as f64],
            );
            constant(
                "extra_params",
                [
                    pbr.map_or(1., |p| p.occlusion_strength as f64),
                    pbr.map_or(0.5, |p| p.alpha_cutoff as f64),
                    if double { 1. } else { 0. },
                    1.,
                ],
            );
            let sky = options.sky.as_ref();
            let sun = sky.map_or([0., 1., 0.], |s| s.to_sun());
            let color = sky.map_or([1., 1., 1.], |s| s.sun_color.map(|v| v * s.sun_intensity));
            constant(
                "sun_direction",
                [sun[0] as f64, sun[1] as f64, sun[2] as f64, 0.],
            );
            constant(
                "sun_color",
                [color[0] as f64, color[1] as f64, color[2] as f64, 1.],
            );
            let ambient = sky.map_or(0.25, |s| s.ambient_intensity as f64);
            constant("ambient_color", [ambient, ambient, ambient, 1.]);
            constant("camera_position", [0., 0., 10., 1.]);
            for name in ["light_positions", "light_colors", "light_directions"] {
                constant(name, [0.; 4]);
            }
            let wrap = if texture.clamp {
                "CLAMP_TO_EDGE"
            } else {
                "REPEAT"
            };
            for sampler in samplers {
                material.push_str(&format!("samplers {{name: \"{sampler}\" wrap_u: WRAP_MODE_{wrap} wrap_v: WRAP_MODE_{wrap} filter_min: FILTER_MODE_MIN_LINEAR_MIPMAP_LINEAR filter_mag: FILTER_MODE_MAG_LINEAR}}\n"));
            }
            files.push(DefoldFile {
                path: format!("{}/material_{index}.material", super::ident(&level.name)),
                contents: material,
            });
        }
    }
    for (id, entity) in level.entities.iter().enumerate() {
        let Some(light) = super::super::common_entities::light(entity) else {
            continue;
        };
        // Components have a private resource, avoiding property collisions with project scripts.
        let script = format!(
            "go.property(\"heron_light_metadata\", hash(\"{}\"))\n",
            escape(&entity.name)
        );
        let mut source = include_str!("defold_light.script").to_owned();
        source = source.replace(
            "vmath.vector4(1,1,1,1)",
            &format!(
                "vmath.vector4({},{},{},{})",
                f(light.color[0]),
                f(light.color[1]),
                f(light.color[2]),
                f(light.color[3])
            ),
        );
        source = source.replace(
            "vmath.vector4(1,5,-1,1)",
            &format!(
                "vmath.vector4({},{},{},{})",
                f(light.energy),
                f(light.range),
                if light.kind == "Spot" {
                    f(light.half_angle.to_radians().cos())
                } else {
                    "-1".into()
                },
                if light.kind == "Directional" {
                    "0"
                } else {
                    "1"
                }
            ),
        );
        files.push(DefoldFile {
            path: format!("{}/heron_light_{id}.script", super::ident(&level.name)),
            contents: script + &source,
        });
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Surface, Value};
    fn level() -> Level {
        Level {
            name: "test".into(),
            format: 2,
            generator: "test".into(),
            scale: 32.,
            textures: vec![Texture {
                name: "paint".into(),
                surface: Some([0.3, 0.7, 2.0]),
                image: Some("paint.png".into()),
                maps: vec![
                    ("normal".into(), "normal.png".into()),
                    ("mr".into(), "mr.png".into()),
                    ("emissive".into(), "emission.png".into()),
                ],
                size: [32.; 2],
                clamp: true,
            }],
            entities: vec![Entity {
                mesh: Some("test/mesh.glb".into()),
                surfaces: vec![Surface {
                    primitive: 0,
                    texture: "paint".into(),
                }],
                ..Default::default()
            }],
        }
    }
    #[test]
    fn defaults_bind_maps_factors_and_clamp_without_overriding_project_materials() {
        let level = level();
        let options = DefoldOptions::default();
        let files = files(&level, &options);
        let material = &files
            .iter()
            .find(|f| f.path.ends_with("material_0.material"))
            .unwrap()
            .contents;
        assert!(
            material.contains("CLAMP_TO_EDGE")
                && material.contains("tex_normal")
                && material.contains("tex_mr")
        );
        assert!(material.contains("x:0.300000") && material.contains("y:0.699999"));
        assert!(generated(&level.entities[0], &options));
        let custom = DefoldOptions {
            model_material: "/game/custom.material".into(),
            ..options
        };
        assert!(!generated(&level.entities[0], &custom));
        assert!(!super::files(&level, &custom)
            .iter()
            .any(|f| f.path.ends_with(".material")));
    }
    #[test]
    fn common_lights_become_native_runtime_components() {
        let mut level = level();
        for kind in ["light", "light_spot", "light_sun"] {
            level.entities.push(Entity {
                classname: kind.into(),
                name: kind.into(),
                properties: [("light_energy".into(), Value::Float(3.))].into(),
                ..Default::default()
            });
        }
        let files = super::super::write(&level, &[], &DefoldOptions::default());
        assert_eq!(
            files
                .iter()
                .filter(|f| f.path.contains("heron_light_"))
                .count(),
            3
        );
        assert!(files[0].contents.contains("heron_light"));
    }
    #[test]
    fn imported_alpha_and_surface_parameters_reach_generated_materials() {
        let level = level();
        let mut options = DefoldOptions::default();
        options.material_pbr.insert(
            "paint".into(),
            crate::materials::GeneratedPbr {
                alpha_mode: AlphaMode::Blend,
                alpha_cutoff: 0.25,
                double_sided: true,
                roughness: 0.25,
                metallic: 0.5,
                emissive_factor: [1., 0.5, 0.25],
                normal_scale: 0.5,
                occlusion_strength: 0.75,
                layers: [0.0; 4],
                images: vec![],
            },
        );
        let files = super::files(&level, &options);
        let material = &files
            .iter()
            .find(|f| f.path.ends_with("material_0.material"))
            .unwrap()
            .contents;
        assert!(material.contains("tags: \"heron_transparent\""));
        assert!(material.contains("x:0.25 y:0.5 z:0.0 w:0.5"));
        assert!(material.contains("x:1.0 y:0.5 z:0.25 w:2.0"));
        assert!(material.contains("x:0.75 y:0.25 z:1.0 w:1.0"));
    }
    #[test]
    fn materials_with_the_same_maps_share_one_shader_program() {
        let mut level = level();
        let mut other = level.textures[0].clone();
        other.name = "other".into();
        other.image = Some("other.png".into());
        level.textures.push(other);
        let files = super::files(&level, &DefoldOptions::default());
        assert_eq!(
            files
                .iter()
                .filter(|f| f.path.ends_with(".material"))
                .count(),
            2
        );
        assert_eq!(files.iter().filter(|f| f.path.ends_with(".fp")).count(), 1);
    }
}
