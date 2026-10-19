//! Raylib export: water volumes compiled into a C99 header, queries and swimming in
//! plain C and, with raylib, the shader that draws the surface and the fog under it.
use crate::raylib_animation::quoted;
use scene_export::level::{Level, Value};

/// The header, or `None` when the level has no water.
pub fn write(level: &Level) -> Option<String> {
    let mut data = String::new();
    let mut waters = Vec::new();
    for (index, entity) in level.entities.iter().enumerate() {
        let Some(Value::String(json)) = entity.properties.get("_heron_water") else { continue };
        let Ok(water) = serde_json::from_str::<serde_json::Value>(json) else { continue };
        let n = waters.len();
        let mut volumes = Vec::new();
        for (v, volume) in water["volumes"].as_array().into_iter().flatten().enumerate() {
            let planes: Vec<String> = volume["planes"].as_array().into_iter().flatten()
                .map(|p| format!("{{{:.17e},{:.17e},{:.17e},{:.17e}}}", p["x"].as_f64().unwrap_or(0.), p["y"].as_f64().unwrap_or(0.), p["z"].as_f64().unwrap_or(0.), p["d"].as_f64().unwrap_or(0.)))
                .collect();
            if planes.is_empty() {
                continue;
            }
            data.push_str(&format!("static const double hb_water_planes_{n}_{v}[][4]={{{}}};\n", planes.join(",")));
            volumes.push(format!("{{hb_water_planes_{n}_{v},{},{:.17e}}}", planes.len(), volume["surface"].as_f64().unwrap_or(0.)));
        }
        if volumes.is_empty() {
            continue;
        }
        data.push_str(&format!("static const HbWaterVolume hb_water_volumes_{n}[]={{{}}};\n", volumes.join(",")));
        // The planes are in the entity's frame; its rest pose takes a world point there.
        let t = &entity.transform;
        let rows: Vec<String> = scene_export::water::world_to_local(t).iter().map(|v| format!("{v:.17e}")).collect();
        let number = |key: &str| water[key].as_f64().unwrap_or(0.);
        let color: Vec<String> = (0..3).map(|i| format!("{:.17e}", water["color"][i].as_f64().unwrap_or(0.))).collect();
        waters.push(format!(
            "{{{},{index},hb_water_volumes_{n},{},{{{}}},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{},{{{}}},{:.17e},{:.17e},{:.17e},{:.17e},{}}}",
            quoted(&entity.name), volumes.len(), rows.join(","), t.scale[1].abs(),
            number("swim_speed"), number("rise_speed"), number("sink_speed"), number("drag"),
            water["priority"].as_i64().unwrap_or(0), color.join(","),
            number("opacity"), number("fog_density"), number("wave_height"), number("wave_speed"),
            i32::from(water["surface_material"].as_bool().unwrap_or(true)),
        ));
    }
    if waters.is_empty() {
        return None;
    }
    let mut out = String::from("#ifndef HERON_EXPORTED_WATER_H\n#define HERON_EXPORTED_WATER_H\n#include <math.h>\n");
    out.push_str("typedef struct {const double (*planes)[4];int count;double surface;} HbWaterVolume;\n");
    out.push_str("/* `entity` is the entity's index in the .level/.json. `to_local` is the 3x4 world-to-entity matrix of its rest pose, by rows. */\n");
    out.push_str("typedef struct {const char *name;int entity;const HbWaterVolume *volumes;int count;double to_local[12],scale_y;\n    double swim_speed,rise_speed,sink_speed,drag;int priority;double color[3],opacity,fog_density,wave_height,wave_speed;int surface_material;} HbWater;\n");
    out.push_str(&data);
    out.push_str(&format!("#define HB_WATER_COUNT {}\nstatic const HbWater hb_waters[]={{{}}};\n", waters.len(), waters.join(",\n")));
    out.push_str(include_str!("raylib_water.h"));
    out.push_str("#endif\n");
    Some(out)
}
