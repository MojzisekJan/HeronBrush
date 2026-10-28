//! Shared interpretation of the HERON_BRUSH light properties used by engine importers.
use crate::level::{Entity, Value};

pub(super) struct Light {
    pub kind: &'static str,
    pub color: [f64; 4],
    pub energy: f64,
    pub range: f64,
    pub half_angle: f64,
    pub shadows: bool,
}

pub(super) fn light(entity: &Entity) -> Option<Light> {
    let kind = match entity.classname.as_str() {
        "light" => "Point",
        "light_spot" => "Spot",
        "light_sun" => "Directional",
        _ => return None,
    };
    let number = |key: &str, default: f64| {
        let n = match entity.properties.get(key) {
            Some(Value::Float(v)) => *v,
            Some(Value::Int(v)) => *v as f64,
            Some(Value::Bool(v)) => {
                if *v {
                    1.
                } else {
                    0.
                }
            }
            Some(Value::String(v)) => v.parse().unwrap_or(default),
            _ => default,
        };
        if n.is_finite() { n } else { default }
    };
    let color = match entity.properties.get("light_color") {
        Some(Value::Color(v)) => *v,
        Some(Value::String(v)) => match crate::level::parse_color(v, 255.) {
            Value::Color(v) => v,
            _ => [1.; 4],
        },
        _ => [1.; 4],
    }
    .map(|v| if v.is_finite() { v.clamp(0., 1.) } else { 1. });
    Some(Light {
        kind,
        color,
        energy: number("light_energy", 1.).max(0.),
        range: number(
            if kind == "Spot" {
                "spot_range"
            } else {
                "omni_range"
            },
            5.,
        )
        .max(0.001),
        half_angle: number("spot_angle", 45.).clamp(0.5, 89.5),
        shadows: number("shadow_enabled", 1.) != 0.,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Level, Transform};

    fn fixture() -> Level {
        Level {
            format: 2,
            generator: "test".into(),
            name: "common".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![
                Entity {
                    classname: "light_spot".into(),
                    name: "Key \"light\"\nabove".into(),
                    properties: [
                        ("light_color".into(), Value::Color([1., 0.5, 0.25, 1.])),
                        ("light_energy".into(), Value::Float(2.)),
                        ("spot_range".into(), Value::Float(7.)),
                        ("spot_angle".into(), Value::Float(30.)),
                        ("shadow_enabled".into(), Value::Bool(false)),
                    ]
                    .into(),
                    transform: Transform {
                        translation: [1., 2., 3.],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Entity {
                    classname: "func_door".into(),
                    name: "door".into(),
                    solid: true,
                    properties: [
                        ("speed".into(), Value::Float(100.)),
                        ("travel".into(), Value::Float(128.)),
                    ]
                    .into(),
                    targets: vec!["first".into(), "second".into()],
                    ..Default::default()
                },
                Entity {
                    classname: "info_player_start".into(),
                    name: "spawn".into(),
                    ..Default::default()
                },
            ],
        }
    }

    #[test]
    fn native_three_and_flax_lights_share_normalized_metadata() {
        let level = fixture();
        let json: serde_json::Value =
            serde_json::from_str(&crate::writer::three::write(&level, &[])).unwrap();
        assert_eq!(json["entities"][0]["light"]["kind"], "Spot");
        assert_eq!(json["entities"][0]["light"]["half_angle"], 30.);
        assert_eq!(json["entities"][0]["light"]["range"], 7.);
        assert_eq!(json["entities"][0]["light"]["shadows"], false);
        assert!(json["entities"][1].get("light").is_none());
        let live: serde_json::Value =
            serde_json::from_str(&crate::writer::flax::live_snapshot(&level, &[])).unwrap();
        assert_eq!(
            live["placements"][0]["metadata"]["light"],
            json["entities"][0]["light"]
        );
        let files = crate::writer::flax::write(
            &level,
            &crate::writer::flax::FlaxOptions::for_level("common"),
        );
        assert!(files[0].contents.contains("AddLight(actor, data)"));
        assert!(
            files[0]
                .contents
                .contains("Radius = (float)spec[\"range\"] * 100f")
        );
        let header = crate::writer::raylib::lighting(&level, None);
        assert!(header.contains("#define HB_RENDER_LIGHT_COUNT 1"));
        assert!(header.contains("0.866025404f")); // cos(30 degree half angle)
    }

    #[test]
    fn lights_use_typed_properties_and_safe_defaults() {
        let l = light(&fixture().entities[0]).unwrap();
        assert_eq!(l.color, [1., 0.5, 0.25, 1.]);
        assert_eq!(l.range, 7.);
        assert!(!l.shadows);
        let mut e = Entity {
            classname: "light_spot".into(),
            ..Default::default()
        };
        e.properties
            .insert("spot_range".into(), Value::String("NaN".into()));
        e.properties
            .insert("spot_angle".into(), Value::Float(1000.));
        assert_eq!(light(&e).unwrap().range, 5.);
        assert_eq!(light(&e).unwrap().half_angle, 89.5);
        assert!(light(&fixture().entities[1]).is_none());
    }

    #[test]
    fn native_lights_convert_cones_and_units_and_keep_metadata() {
        let level = fixture();
        let unity = crate::writer::unity::write(&level, &[], &Default::default());
        let cs = &unity[0].contents;
        assert!(cs.contains("LightType.Spot"));
        assert!(cs.contains("range = 7.000000f"));
        assert!(cs.contains("spotAngle = 60.000000f"));
        assert!(cs.contains("LightShadows.None"));
        assert!(cs.contains("nodes[1].AddComponent<HeronBrushEntity>()"));
        assert!(cs.contains("Directory.GetParent(scriptPath).Parent.FullName"));
        assert!(unity.iter().any(|f| f.path == "HeronBrushEntity.cs"));
        assert_eq!(unity[1].contents, level.to_json());
        let py = crate::writer::unreal::write(
            &level,
            &crate::writer::unreal::UnrealOptions {
                content_dir: "/Game/Custom".into(),
            },
        );
        assert!(py.contains("unreal.PlayerStart"));
        assert!(py.contains("attenuation_radius\", 700.000000"));
        assert!(py.contains("outer_cone_angle\", 30.000000"));
        assert!(py.contains("heron_brush_entity_index:1"));
        assert!(py.contains("ENTITY_DATA = json.loads("));
        assert!(py.contains("/Game/Custom"));
        assert!(py.contains("Key \\\"light\\\"\\nabove"));
    }

    #[test]
    fn defold_keeps_brush_properties_all_targets_and_empty_spawn_markers() {
        let level = fixture();
        let files = crate::writer::defold::write(&level, &[], &Default::default());
        let door = &files
            .iter()
            .find(|f| f.path.ends_with("/door.script"))
            .unwrap()
            .contents;
        assert!(door.contains("go.property(\"speed\", 100.0)"));
        assert!(door.contains("go.property(\"travel\", 128.0)"));
        let spawn = &files
            .iter()
            .find(|f| f.path.ends_with("/spawn.script"))
            .unwrap()
            .contents;
        assert!(spawn.contains("hash(\"info_player_start\")"));
        let manifest = &files
            .iter()
            .find(|f| f.path == "common.json")
            .unwrap()
            .contents;
        assert_eq!(manifest, &level.to_json());
    }
}
