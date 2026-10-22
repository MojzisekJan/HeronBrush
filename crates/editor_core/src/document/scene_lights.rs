//! Point and spot lights from light entities, for a renderer that draws them
//! ([`crate::render_scene::SceneLight`]).
//!
//! Reads both dialects a map can speak: the HERON_BRUSH/Godot keys the bundled definitions use
//! (`light_color`, `light_energy`, `omni_range` / `spot_range` in engine units,
//! `omni_attenuation`, `spot_angle`, `spot_angle_attenuation`, `shadow_enabled`) and
//! Quake / Half-Life ones (`light` as brightness and reach in map units, `_color`,
//! `_cone` / `_cone2`, `mangle` / `angles` / `angle` / `pitch`). Where a key is missing
//! the HERON_BRUSH definition's default applies, so a bare `light` looks as it will in the engine.
use geometry_math::vec::Vec3d;
use map_model::node::{self, EntityNode};

use crate::render_scene::{SceneLight, MAP_UNITS_PER_METRE};

/// The sRGB transfer curve undone: light colours are authored as display colours.
fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn number(entity: &EntityNode, key: &str) -> Option<f32> {
    entity
        .property(key)?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
}

fn triple(entity: &EntityNode, key: &str) -> Option<[f32; 3]> {
    let values: Vec<f32> = entity
        .property(key)?
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    match values[..] {
        [a, b, c, ..] if [a, b, c].iter().all(|v| v.is_finite()) => Some([a, b, c]),
        _ => None,
    }
}

/// The display colour, 0..1: `light_color` is 0..255; `_color` is 0..1 in Quake tools and
/// 0..255 in Half-Life ones (and in Half-Life the fourth number is the brightness).
fn color(entity: &EntityNode) -> [f32; 3] {
    if let Some(c) = triple(entity, "light_color") {
        return c.map(|v| (v / 255.0).clamp(0.0, 1.0));
    }
    if let Some(c) = triple(entity, "_color") {
        let scale = if c.iter().any(|v| *v > 1.0) {
            255.0
        } else {
            1.0
        };
        return c.map(|v| (v / scale).clamp(0.0, 1.0));
    }
    [1.0; 3]
}

/// Which way an entity faces, from Quake's angle keys: pitch down positive, yaw about
/// +Z from +X. `mangle` on a light is yaw, pitch, roll (pitch up positive), as Quake's
/// light tools read it; `angles` is pitch, yaw, roll.
fn facing(entity: &EntityNode) -> Vec3d {
    let (pitch, yaw) = if let Some([yaw, pitch, _]) = triple(entity, "mangle") {
        (-pitch, yaw)
    } else if let Some([pitch, yaw, _]) = triple(entity, "angles") {
        (pitch, yaw)
    } else {
        let pitch = number(entity, "pitch").unwrap_or(0.0);
        match number(entity, "angle") {
            // Quake's special values for straight up and straight down.
            Some(a) if (a + 1.0).abs() < 1e-3 => (-90.0, 0.0),
            Some(a) if (a + 2.0).abs() < 1e-3 => (90.0, 0.0),
            Some(a) => (pitch, a),
            None => (
                if entity.property("pitch").is_some() {
                    pitch
                } else {
                    90.0
                },
                0.0,
            ),
        }
    };
    let (p, y) = (f64::from(pitch).to_radians(), f64::from(yaw).to_radians());
    Vec3d::new(p.cos() * y.cos(), p.cos() * y.sin(), -p.sin())
}

/// The light an entity gives, if it is a point or spot light. Suns and environment
/// lights are not: the view's own sun stands for them.
pub(super) fn light(id: u64, entity: &EntityNode) -> Option<SceneLight> {
    let class = entity.classname()?.to_ascii_lowercase();
    let is_light = class == "light" || class.starts_with("light_");
    if !is_light
        || ["sun", "environment", "directional"]
            .iter()
            .any(|k| class.contains(k))
    {
        return None;
    }
    let position = entity.origin()?;
    let spot = class.contains("spot");
    let units = MAP_UNITS_PER_METRE as f32;
    // Quake's `light` is a brightness that also says how far the light reaches, in map
    // units (300 is a bright lamp lighting a few metres round it).
    let quake = number(entity, "light")
        .or_else(|| triple(entity, "_light").map(|c| c[0]))
        .filter(|v| *v > 0.0);
    let range_key = if spot { "spot_range" } else { "omni_range" };
    let godot = entity.property("light_energy").is_some() || entity.property(range_key).is_some();
    let (energy, range) = match (godot, quake) {
        (false, Some(value)) => (value / 100.0, value),
        _ => (
            number(entity, "light_energy").unwrap_or(1.0),
            number(entity, range_key).unwrap_or(5.0) * units,
        ),
    };
    let energy = energy.max(0.0);
    let range = range.max(1.0);
    let attenuation_key = if spot {
        "spot_attenuation"
    } else {
        "omni_attenuation"
    };
    let decay = number(entity, attenuation_key)
        .unwrap_or(1.0)
        .clamp(0.0, 8.0);
    let spot = spot.then(|| {
        let half_angle = number(entity, "spot_angle")
            .or_else(|| number(entity, "_cone2"))
            .or_else(|| number(entity, "_cone"))
            .unwrap_or(45.0)
            .clamp(0.5, 89.5);
        let exponent = number(entity, "spot_angle_attenuation")
            .unwrap_or(1.0)
            .clamp(0.01, 64.0);
        let d = facing(entity);
        (
            [d.x() as f32, d.y() as f32, d.z() as f32],
            half_angle.to_radians().cos(),
            exponent,
        )
    });
    // Engines multiply a light's energy by pi so that energy 1 at a metre lights a white
    // wall about as brightly as a sun of energy 1; the view's sun is calibrated the same
    // way, so the two balance as they will in Godot.
    let linear = color(entity).map(|c| to_linear(c) * energy * std::f32::consts::PI);
    let shadows = number(entity, "shadow_enabled").is_none_or(|v| v != 0.0);
    Some(SceneLight {
        id,
        position,
        color: linear,
        range,
        decay,
        spot,
        shadows,
    })
}

impl super::Document {
    /// The point and spot lights of the drawn light entities, for a renderer with dynamic
    /// lights. Hidden entities and those on hidden layers give none.
    pub fn scene_lights(&self) -> Vec<SceneLight> {
        node::point_entities_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, layer)| self.node_is_drawn(*id, *layer))
            .filter_map(|(id, entity, _)| light(id.0, entity))
            .filter(|l| l.color.iter().any(|c| *c > 0.0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(properties: &[(&str, &str)]) -> EntityNode {
        EntityNode {
            properties: properties
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            children: Vec::new(),
        }
    }

    #[test]
    fn godot_keys_give_range_in_engine_units_and_a_coloured_light() {
        let l = light(
            1,
            &entity(&[
                ("classname", "light"),
                ("origin", "1 2 3"),
                ("light_color", "255 0 0"),
                ("light_energy", "2"),
                ("omni_range", "6"),
            ]),
        )
        .unwrap();
        assert_eq!(l.range, 6.0 * MAP_UNITS_PER_METRE as f32);
        assert!(l.color[0] > 6.0 && l.color[1] == 0.0);
        assert!(l.spot.is_none() && l.shadows);
    }

    #[test]
    fn quake_light_value_is_brightness_and_reach() {
        let l = light(
            1,
            &entity(&[
                ("classname", "light_torch_small_walltorch"),
                ("origin", "0 0 0"),
                ("light", "200"),
                ("_color", "1 0.5 0.25"),
            ]),
        )
        .unwrap();
        assert_eq!(l.range, 200.0);
        assert!(l.color[0] > l.color[1] && l.color[1] > l.color[2]);
    }

    #[test]
    fn spots_point_where_their_angles_say_and_suns_are_not_lights() {
        let l = light(
            1,
            &entity(&[
                ("classname", "light_spot"),
                ("origin", "0 0 0"),
                ("angles", "90 0 0"),
                ("spot_angle", "30"),
                ("shadow_enabled", "0"),
            ]),
        )
        .unwrap();
        let (d, cutoff, _) = l.spot.unwrap();
        assert!(d[2] < -0.99, "{d:?}");
        assert!((cutoff - 30f32.to_radians().cos()).abs() < 1e-5);
        assert!(!l.shadows);
        assert!(light(
            1,
            &entity(&[("classname", "light_sun"), ("origin", "0 0 0")])
        )
        .is_none());
        assert!(light(
            1,
            &entity(&[("classname", "info_player_start"), ("origin", "0 0 0")])
        )
        .is_none());
    }
}
