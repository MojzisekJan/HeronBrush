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
            export::Target::Babylon,
            export::Target::PlayCanvas,
            export::Target::Flax,
        ] {
            let sub = match target {
                export::Target::Defold => "defold/main",
                export::Target::Bevy => "bevy",
                export::Target::ThreeJs => "three",
                export::Target::Babylon => "babylon",
                export::Target::PlayCanvas => "playcanvas",
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
                export::Target::Babylon => {
                    assert!(std::fs::read_to_string(path.join("heron_babylon.js"))
                        .unwrap()
                        .contains("export class HeronAnimation "))
                }
                export::Target::PlayCanvas => {
                    assert!(std::fs::read_to_string(path.join("heron_playcanvas.js"))
                        .unwrap()
                        .contains("export class HeronAnimation "))
                }
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

    #[test]
    #[ignore = "writes the feature lab's editor reference to HERON_FEATURE_REFERENCE"]
    fn feature_lab_editor_reference() {
        // Engine checks compare their import with this: four map points of the mover
        // and the door placed by the editor's own preview transform, and the colour
        // the editor gives points inside each face of the material stations.
        let output = std::env::var_os("HERON_FEATURE_REFERENCE").expect("output JSON path");
        let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demos/feature_lab");
        let text = std::fs::read_to_string(lab.join("feature_lab.map")).expect("feature lab map");
        let map = map_model::io::map_reader::read_map(&text, None).expect("feature lab parses");
        let station = |name: &str| {
            map.entities
                .iter()
                .find(|e| e.property("targetname") == Some(name))
                .expect("station in the map")
        };
        let flat = |points: &mut dyn Iterator<Item = Vec3d>| {
            points
                .flat_map(|p| [p.x(), p.y(), p.z()])
                .collect::<Vec<_>>()
        };
        let mut cases = Vec::new();
        for (name, clip, times) in [
            (
                "fl_mover",
                "",
                &[0., 0.5, 1., 1.5, 2., 2.5, 3., 3.5, 3.9, 5.][..],
            ),
            ("fl_door", "open", &[0., 0.25, 0.5, 0.75, 1., 2.][..]),
        ] {
            let entity = station(name);
            let track = Track::from_properties(
                entity.property("animation_keys").unwrap_or(""),
                entity.property("animation_mode").unwrap_or("loop"),
                entity.property("animation_curves"),
            )
            .expect("valid animation");
            let pivot: Vec<f64> = entity
                .property("animation_pivot")
                .unwrap_or("0 0 0")
                .split_whitespace()
                .map(|v| v.parse().expect("pivot number"))
                .collect();
            let pivot = Vec3d::new(pivot[0], pivot[1], pivot[2]);
            let rest = [
                pivot,
                pivot + Vec3d::new(64., 0., 0.),
                pivot + Vec3d::new(0., 64., 0.),
                pivot + Vec3d::new(0., 0., 64.),
            ];
            for &time in times {
                let posed = flat(&mut rest.iter().map(|p| track.transform(*p, pivot, time, false)));
                cases.push(
                    serde_json::json!({"entity": name, "clip": clip, "time": time,
                    "rest": flat(&mut rest.iter().copied()), "posed": posed}),
                );
            }
        }
        let bounds = geometry_math::bbox::Bbox3d::new(Vec3d::fill(-65536.), Vec3d::fill(65536.));
        let mut faces = Vec::new();
        for name in [
            "fl_textures",
            "fl_colours",
            "fl_gradients",
            "fl_surface",
            "fl_normal",
        ] {
            for brush in &station(name).brushes {
                let brush = map_model::brush::Brush::from_faces(&bounds, brush.faces.clone())
                    .expect("station brush");
                for (index, face) in brush.faces().iter().enumerate() {
                    if face.export_hidden {
                        continue;
                    }
                    let polygon = brush.face_polygon(index).expect("face polygon");
                    let image =
                        image::open(lab.join(format!("textures/{}.png", face.material_name)))
                            .expect("station texture")
                            .to_rgba8();
                    let paint = face.paint();
                    let sampler = paint.map(|paint| {
                        map_model::brush_face::PaintOnFace::new(face, &polygon, paint)
                    });
                    let alpha = paint.map_or(0., |p| f32::from(p.alpha()) / 255.);
                    let centre = polygon.iter().fold(Vec3d::zero(), |a, p| a + *p)
                        * (1. / polygon.len() as f64);
                    let (mut points, mut colours) = (Vec::new(), Vec::new());
                    for (k, s, t) in (0..polygon.len())
                        .flat_map(|k| [0.13, 0.31, 0.47, 0.66, 0.83].map(|s| (k, s)))
                        .flat_map(|(k, s)| [0., 0.37, 0.5].map(|t| (k, s, t)))
                    {
                        let edge = polygon[k] * (1. - t) + polygon[(k + 1) % polygon.len()] * t;
                        let point = centre + (edge - centre) * s;
                        let uv = face.uv_coords(point);
                        // Keep clear of texel edges, where nearest sampling is a coin toss.
                        let inside = |c: f32| (0.2..0.8).contains(&(c - c.floor()));
                        if alpha < 1. && !(inside(uv.x()) && inside(uv.y())) {
                            continue;
                        }
                        let x = (uv.x().floor() as i64).rem_euclid(i64::from(image.width()));
                        let y = (uv.y().floor() as i64).rem_euclid(i64::from(image.height()));
                        let texel = image.get_pixel(x as u32, y as u32).0;
                        let paint = sampler.as_ref().map_or([0.; 3], |s| s.at_uv(uv));
                        colours.extend((0..3).map(|i| {
                            (paint[i] * 255. * alpha + f32::from(texel[i]) * (1. - alpha)).round()
                        }));
                        points.extend([point.x(), point.y(), point.z()]);
                    }
                    let n = face.normal();
                    faces.push(serde_json::json!({"station": name,
                        "texture": face.material_name.as_str(), "normal": [n.x(), n.y(), n.z()],
                        "points": points, "colours": colours}));
                }
            }
        }
        let json = serde_json::json!({ "cases": cases, "faces": faces }).to_string();
        std::fs::write(output, json).expect("write the reference");
    }

    /// Writes the editor's poses of the feature lab's `fl_mover` for engine checks to
    /// compare against: every 1/60 s over two loops, the same a nanosecond earlier,
    /// and a few off-grid times.
    /// Positions and axes are in a Y-up frame in metres, as Defold and Flax use.
    #[test]
    fn writes_feature_lab_mover_poses_for_engine_checks() {
        let map = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/feature_lab/feature_lab.map");
        let d = Document::open(map, Default::default()).expect("the feature lab opens");
        let entities = map_model::node::entities_with_ids(d.tree_world());
        let (_, mover) = entities
            .iter()
            .find(|(_, e)| e.property("targetname") == Some("fl_mover"))
            .expect("the feature lab has fl_mover");
        let property = |key: &str| mover.property(key).expect("fl_mover is animated");
        let track = Track::from_properties(
            property("animation_keys"),
            property("animation_mode"),
            Some(property("animation_curves")),
        )
        .expect("fl_mover's animation parses");
        let pivot: Vec<f64> = property("animation_pivot")
            .split_whitespace()
            .map(|v| v.parse().expect("the pivot is numeric"))
            .collect();
        let pivot = Vec3d::new(pivot[0], pivot[1], pivot[2]);
        let pose = |time: f64| {
            let position = track.transform(pivot, pivot, time, false);
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
            serde_json::json!({
                "time": time,
                "position": [position.y() / 32., position.z() / 32., position.x() / 32.],
                "axes": axes,
            })
        };
        let frames: Vec<_> = (0..=480).map(|frame| pose(frame as f64 / 60.)).collect();
        // A runtime that sums 1/60 per frame reads 2.9999999999999942 at frame 180,
        // just short of the held key at 3 s, so each frame also has the pose a
        // nanosecond earlier.
        let frames_before: Vec<_> = (0..=480)
            .map(|frame| pose((frame as f64 / 60. - 1e-9).max(0.)))
            .collect();
        let samples: Vec<_> = [0.25, 0.5, 0.999, 1.0, 1.37, 2.0, 2.9, 3.0, 3.5, 3.99, 5.0]
            .into_iter()
            .map(pose)
            .collect();
        // Spot checks against the fixture's description: map X peaks at 128 units
        // at 2 s and map Z is held at 32 units from 1 s to 3 s.
        assert!((samples[5]["position"][2].as_f64().expect("z") + 26.5 - 4.).abs() < 1e-9);
        assert!((samples[3]["position"][1].as_f64().expect("y") - 0.5 - 1.).abs() < 1e-9);
        assert!((samples[2]["position"][1].as_f64().expect("y") - 0.5).abs() < 1e-9);
        let path = std::env::var_os("HERON_FEATURE_LAB_POSES")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("heron-feature-lab-poses.json"));
        let poses = serde_json::json!({
            "frames": frames,
            "frames_before": frames_before,
            "samples": samples,
        });
        std::fs::write(&path, poses.to_string()).expect("the poses are written");
    }
}
