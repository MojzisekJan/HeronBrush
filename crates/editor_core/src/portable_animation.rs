//! Validated animation payload shared by Defold, Bevy, Three.js, Flax and Unity runtimes.
use crate::{
    animation_curves::Clip,
    animation_library::{Entry, Library},
    keyframe_animation::Track,
};
use scene_export::level::{Level, Value};
pub fn prepare(level: &mut Level, warnings: &mut Vec<String>) {
    let scale = level.scale;
    for e in &mut level.entities {
        let get = |name: &str| match e.properties.get(name) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        };
        if get("animation_keys").is_none() && get("animation_curves").is_none() {
            continue;
        }
        let data = (|| -> Result<String, String> {
            let mode = get("animation_mode").unwrap_or("loop");
            let track = Track::from_properties(
                get("animation_keys").unwrap_or(""),
                mode,
                get("animation_curves"),
            )?;
            let clip = track
                .curves
                .clone()
                .unwrap_or_else(|| Clip::from_legacy(&track));
            let selected = get("animation_name")
                .filter(|s| !s.is_empty())
                .unwrap_or("default");
            let mut library = Library::parse(get("animation_library").unwrap_or(""))?;
            library.clips.insert(
                selected.into(),
                Entry {
                    clip,
                    mode: mode.into(),
                },
            );
            library.validate()?;
            let pivot = match e.properties.get("animation_pivot") {
                Some(Value::Vec3(v)) => *v,
                Some(Value::String(s)) => {
                    let v = s
                        .split_whitespace()
                        .map(str::parse::<f64>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| "Invalid pivot")?;
                    if v.len() != 3 {
                        return Err("Invalid pivot".into());
                    }
                    [v[0], v[1], v[2]]
                }
                None => [
                    e.transform.translation[2] * scale,
                    e.transform.translation[0] * scale,
                    e.transform.translation[1] * scale,
                ],
                _ => return Err("Invalid pivot".into()),
            };
            if pivot.iter().any(|v| !v.is_finite()) || !scale.is_finite() || scale <= 0.0 {
                return Err("Invalid pivot or scale".into());
            }
            let autoplay = match e.properties.get("start_on") {
                Some(Value::Bool(v)) => *v,
                Some(Value::Int(v)) => *v != 0,
                Some(Value::Float(v)) => *v != 0.0,
                Some(Value::String(v)) => v.parse::<f64>().unwrap_or(1.0) != 0.0,
                _ => true,
            };
            let clips:Vec<_>=library.clips.iter().map(|(name,entry)|serde_json::json!({"name":name,"mode":entry.mode,"duration":entry.clip.duration,"channels":entry.clip.tracks.iter().map(|keys|serde_json::json!({"keys":keys})).collect::<Vec<_>>()})).collect();
            Ok(serde_json::json!({"version":1,"scale":scale,"pivot":pivot,"selected":selected,"autoplay":autoplay,"clips":clips}).to_string())
        })();
        match data {
            Ok(json) => {
                e.properties
                    .insert("_heron_animation".into(), Value::String(json));
            }
            Err(why) => warnings.push(format!("Animation {}: {why}; left static", e.name)),
        }
    }
}

pub fn prepare_triggers(level: &mut Level) {
    let animated: std::collections::HashSet<_> = level
        .entities
        .iter()
        .filter(|e| {
            e.properties.contains_key("animation_keys")
                || e.properties.contains_key("animation_curves")
        })
        .map(|e| e.name.clone())
        .collect();
    for e in &mut level.entities {
        if !matches!(
            e.classname.as_str(),
            "func_button" | "trigger_once" | "trigger_multiple"
        ) {
            continue;
        }
        let targets: Vec<_> = e
            .targets
            .iter()
            .filter(|name| animated.contains(*name))
            .cloned()
            .collect();
        if targets.is_empty() {
            continue;
        }
        let default_wait = if e.classname == "trigger_multiple" {
            1.0
        } else {
            0.0
        };
        let wait = match e.properties.get("wait") {
            Some(Value::Float(v)) => *v,
            Some(Value::Int(v)) => *v as f64,
            Some(Value::String(v)) => v.parse().unwrap_or(default_wait),
            _ => default_wait,
        };
        let volumes = e
            .properties
            .get("_heron_trigger_volumes")
            .and_then(|v| {
                if let Value::String(s) = v {
                    serde_json::from_str::<serde_json::Value>(s).ok()
                } else {
                    None
                }
            })
            .unwrap_or(serde_json::json!([]));
        let text = |key: &str| match e.properties.get(key) {
            Some(Value::String(v)) => v.clone(),
            Some(Value::Int(v)) => v.to_string(),
            Some(Value::Bool(v)) => if *v { "1" } else { "0" }.into(),
            _ => String::new(),
        };
        let once = if e.properties.contains_key("animation_once") {
            text("animation_once") == "1"
        } else {
            e.classname == "trigger_once" || wait < 0.0
        };
        let action = text("animation_action");
        let action = if action.is_empty() { "toggle" } else { &action };
        let clip = text("animation_clip");
        let data = serde_json::json!({"action":action,"clip":clip,"name":e.name,"targets":targets,"once":once,"wait":if wait.is_finite(){wait.max(0.0)}else{1.0},"button":e.classname=="func_button","origin":e.transform.translation,"volumes":volumes});
        e.properties
            .insert("_heron_trigger".into(), Value::String(data.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{animation_curves::Interpolation, document::Document, export};
    use geometry_math::vec::Vec3d;
    #[test]
    fn exports_defold_bevy_unity_players_with_validated_named_clips() {
        let dir = std::env::temp_dir().join("heron-portable-animation-check");
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::new(
                Vec3d::new(32., 64., 96.),
                Vec3d::new(96., 128., 160.),
            ),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "targetname", "animated");
        let mut clip = Clip {
            version: 1,
            duration: 2.,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        };
        for axis in 0..6 {
            clip.insert(axis, 0., 0.).unwrap();
            clip.insert(axis, 0.7, 20. + axis as f64 * 10.).unwrap();
            clip.insert(axis, 2., 80. + axis as f64 * 30.).unwrap();
        }
        clip.tracks[0][0].interpolation = Interpolation::Bezier;
        clip.tracks[0][0].outgoing = 100.;
        clip.tracks[0][1].incoming = -30.;
        clip.tracks[1][0].interpolation = Interpolation::Hold;
        let mut library = Library::parse("").unwrap();
        for mode in ["once", "loop", "pingpong"] {
            library.clips.insert(
                mode.into(),
                Entry {
                    mode: mode.into(),
                    clip: clip.clone(),
                },
            );
        }
        let mut open = Clip {
            version: 1,
            duration: 2.,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        };
        open.insert_position(0., [0.; 3]).unwrap();
        open.insert_position(2., [100., 0., 0.]).unwrap();
        let mut close = open.clone();
        for key in &mut close.tracks[0] {
            key.value = 100. - key.value;
        }
        library.clips.insert(
            "open".into(),
            Entry {
                mode: "once".into(),
                clip: open,
            },
        );
        library.clips.insert(
            "close".into(),
            Entry {
                mode: "once".into(),
                clip: close,
            },
        );
        library.write(&mut d, id, "loop");
        d.set_entity_property(id, "start_on", "0");
        d.set_entity_property(id, "animation_pivot", "16 32 48");
        d.clear_selection();
        let button = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::new(Vec3d::new(256., 0., 0.), Vec3d::new(320., 64., 64.)),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![button]);
        let button = d.create_brush_entity("trigger_once").unwrap();
        d.set_entity_property(button, "target", "animated");
        for action in ["play", "open", "close", "pause", "resume", "stop"] {
            d.clear_selection();
            let brush = map_model::shapes::create_cuboid(
                &d.world_bounds(),
                &geometry_math::bbox::Bbox3d::new(
                    Vec3d::new(512., 0., 0.),
                    Vec3d::new(544., 32., 32.),
                ),
                "stone",
            )
            .unwrap();
            d.add_brushes(vec![brush]);
            let trigger = d.create_brush_entity("func_button").unwrap();
            d.set_entity_property(trigger, "target", "animated");
            d.set_entity_property(trigger, "animation_action", action);
            d.set_entity_property(trigger, "animation_clip", "pingpong");
            d.set_entity_property(trigger, "animation_once", "0");
            d.set_entity_property(trigger, "wait", "0");
        }
        let mut samples = Vec::new();
        for mode in ["once", "loop", "pingpong"] {
            let track =
                Track::from_properties(&clip.legacy_text(), mode, Some(&clip.json())).unwrap();
            for time in [0., 0.125, 0.69999, 0.7, 1.2, 2., 2.5, 3.5, 4.5] {
                let pos = track.transform(
                    Vec3d::new(64., 96., 128.),
                    Vec3d::new(16., 32., 48.),
                    time,
                    false,
                );
                let axes: Vec<_> = [
                    Vec3d::new(0., 1., 0.),
                    Vec3d::new(0., 0., 1.),
                    Vec3d::new(1., 0., 0.),
                ]
                .into_iter()
                .map(|v| {
                    let r = track.transform(v, Vec3d::zero(), time, true);
                    [r.y(), r.z(), r.x()]
                })
                .collect();
                samples.push(serde_json::json!({"mode":mode,"time":time,"position":[pos.y()/32.,pos.z()/32.,pos.x()/32.],"axes":axes}));
            }
        }
        std::fs::write(
            dir.join("expected.json"),
            serde_json::to_string(&samples).unwrap(),
        )
        .unwrap();
        for target in [
            export::Target::Defold,
            export::Target::Bevy,
            export::Target::Unity,
            export::Target::ThreeJs,
            export::Target::Flax,
        ] {
            let sub = match target {
                export::Target::Defold => "defold/main",
                export::Target::Bevy => "bevy",
                export::Target::ThreeJs => "three",
                export::Target::Flax => "flax",
                _ => "unity/Assets/Level",
            };
            let path = dir.join(sub);
            std::fs::create_dir_all(&path).unwrap();
            let report = export::export(
                &d,
                &path,
                &export::Options {
                    target,
                    name: "check".into(),
                    asset_dir: "/main/check".into(),
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(
                !report
                    .warnings
                    .iter()
                    .any(|w| w.contains("keyframe playback") || w.starts_with("Animation ")),
                "{:?}",
                report.warnings
            );
            let level: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(path.join(if target == export::Target::Flax {
                    "check.level.json"
                } else {
                    "check.json"
                }))
                .unwrap(),
            )
            .unwrap();
            assert!(
                level["entities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["properties"]["_heron_trigger"]["value"]
                        .as_str()
                        .is_some()),
                "trigger links must be exported"
            );
            for action in ["play", "open", "close", "pause", "resume", "stop"] {
                let data = level["entities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|e| e["properties"]["_heron_trigger"]["value"].as_str())
                    .filter_map(|v| serde_json::from_str::<serde_json::Value>(v).ok())
                    .find(|v| v["action"] == action)
                    .expect("export each configured action");
                assert_eq!(data["clip"], "pingpong");
                assert_eq!(data["once"], false);
                assert_eq!(data["wait"], 0.0);
            }
            let data = level["entities"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|e| e["properties"]["_heron_animation"]["value"].as_str())
                .unwrap();
            let parsed: serde_json::Value = serde_json::from_str(data).unwrap();
            assert_eq!(parsed["clips"].as_array().unwrap().len(), 5);
            assert_eq!(parsed["autoplay"], false);
            std::fs::write(dir.join("data.json"), data).unwrap();
            match target {
                export::Target::Defold => {
                    let script =
                        std::fs::read_to_string(path.join("check/func_animate.script")).unwrap();
                    assert!(script.contains("function update(self,dt)"));
                    assert!(script.contains("local hb_data = json.decode"));
                    assert!(std::fs::read_to_string(
                        path.join("check/func_animate.collisionobject")
                    )
                    .unwrap()
                    .contains("COLLISION_OBJECT_TYPE_KINEMATIC"));
                }
                export::Target::Bevy => assert!(path.join("heron_animation.rs").exists()),
                export::Target::ThreeJs => assert!(path.join("heron_animation.js").exists()),
                export::Target::Flax => assert!(path.join("HeronBrushAnimation.cs").exists()),
                _ => {
                    assert!(path.join("HeronBrushAnimation.cs").exists());
                    assert!(std::fs::read_to_string(
                        path.join("Editor/HeronBrushLevelImporter.cs")
                    )
                    .unwrap()
                    .contains("AddComponent<HeronBrushAnimation>"));
                }
            }
        }
        // Bad data produces a warning and no runtime payload, preserving static export.
        d.set_entity_property(id, "animation_curves", "not json");
        let path = dir.join("invalid");
        std::fs::create_dir_all(&path).unwrap();
        let report = export::export(
            &d,
            &path,
            &export::Options {
                target: export::Target::Bevy,
                name: "invalid".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(report.warnings.iter().any(|w| w.starts_with("Animation ")));
        eprintln!("Portable animation fixtures: {}", dir.display());
    }
}
