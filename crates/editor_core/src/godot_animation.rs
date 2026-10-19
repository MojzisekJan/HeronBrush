//! Self-contained Godot playback, sharing validation with the editor.
use crate::{animation_curves::Clip, keyframe_animation::Track};
use scene_export::{
    math::{self, Vec3},
    scene::{GdValue, Node, Scene, SubResource},
};

const SCRIPT: &str = include_str!("godot_animation.gd");

pub fn attach(scene: &mut Scene, scale: f64, warnings: &mut Vec<String>) {
    let id = format!("HeronKeyframes_{}", scene.sub_resources.len());
    if visit(
        &mut scene.root,
        scale,
        &id,
        warnings,
        &mut scene.sub_resources,
    ) {
        scene.sub_resources.push(SubResource {
            id,
            ty: "GDScript".into(),
            properties: vec![("script/source".into(), GdValue::Str(SCRIPT.into()))],
        });
    }
    wire_buttons(scene);
}
fn visit(
    node: &mut Node,
    scale: f64,
    script: &str,
    warnings: &mut Vec<String>,
    resources: &mut Vec<SubResource>,
) -> bool {
    let mut used = false;
    for child in &mut node.children {
        used |= visit(child, scale, script, warnings, resources);
    }
    let Some(GdValue::Dict(props)) = node
        .meta
        .iter()
        .find(|(k, _)| k == "properties")
        .map(|(_, v)| v)
    else {
        return used;
    };
    let string = |name: &str| {
        props.iter().find(|(k, _)| k == name).and_then(|(_, v)| {
            if let GdValue::Str(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        })
    };
    let Some(keys) = string("animation_keys") else {
        return used;
    };
    let mode = string("animation_mode").unwrap_or("loop");
    let result: Result<Node, String> = (|| {
        let track = Track::from_properties(keys, mode, string("animation_curves"))?;
        let clip = track
            .curves
            .clone()
            .unwrap_or_else(|| Clip::from_legacy(&track));
        clip.validate()?;
        let pivot = if let Some(raw) = string("animation_pivot") {
            let xyz = raw
                .split_whitespace()
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "Invalid animation pivot")?;
            if xyz.len() != 3 || xyz.iter().any(|v| !v.is_finite()) {
                return Err("Invalid animation pivot".into());
            }
            math::to_godot(Vec3::new(xyz[0], xyz[1], xyz[2])) * scale
        } else {
            node.properties
                .iter()
                .find_map(|(k, v)| match (k.as_str(), v) {
                    ("transform", GdValue::Transform3D(_, p)) => Some(*p),
                    _ => None,
                })
                .unwrap_or(Vec3::ZERO)
        };
        let autoplay = !props.iter().any(|(k, v)| {
            k == "start_on"
                && match v {
                    GdValue::Int(0) | GdValue::Bool(false) => true,
                    GdValue::Float(v) => *v == 0.0,
                    GdValue::Str(s) => s.parse::<f64>().ok() == Some(0.0),
                    _ => false,
                }
        });
        let active = string("animation_name")
            .filter(|s| !s.is_empty())
            .unwrap_or("default");
        crate::animation_library::valid_name(active)?;
        let mut library =
            crate::animation_library::Library::parse(string("animation_library").unwrap_or(""))?;
        library.clips.insert(
            active.into(),
            crate::animation_library::Entry {
                clip: clip.clone(),
                mode: mode.into(),
            },
        );
        library.validate()?;
        let mut animations = Vec::new();
        for (name, entry) in &library.clips {
            let id = format!("HeronClip_{}", resources.len());
            resources.push(crate::godot_native_animation::animation(
                &id,
                name,
                &entry.clip,
                &entry.mode,
            ));
            animations.push((name.clone(), GdValue::SubResource(id)));
        }
        let library_id = format!("HeronLibrary_{}", resources.len());
        resources.push(SubResource {
            id: library_id.clone(),
            ty: "AnimationLibrary".into(),
            properties: vec![("_data".into(), GdValue::Dict(animations))],
        });
        let mut native = Node::new("AnimationPlayer", "AnimationPlayer");
        native.set(
            "libraries",
            GdValue::Dict(vec![("".into(), GdValue::SubResource(library_id))]),
        );
        native.set("callback_mode_process", GdValue::Int(0));
        let mut player = Node::new("HeronAnimation", "Node");
        player.add_child(native);
        player.set("script", GdValue::SubResource(script.into()));
        player.set("default_clip", GdValue::Str(active.into()));
        let rest = node
            .properties
            .iter()
            .find(|(k, _)| k == "transform")
            .map(|(_, v)| v.clone())
            .unwrap_or(GdValue::Transform3D(
                [1., 0., 0., 0., 1., 0., 0., 0., 1.],
                Vec3::ZERO,
            ));
        player.set("rest_transform", rest);
        player.set("autoplay", GdValue::Bool(autoplay));
        player.set("pivot", GdValue::Vec3(pivot));
        player.set("map_scale", GdValue::Float(scale));
        Ok(player)
    })();
    match result {
        Ok(mut player) => {
            let mut suffix = 1;
            while node.children.iter().any(|c| c.name == player.name) {
                suffix += 1;
                player.name = format!("HeronAnimation{suffix}");
            }
            node.add_child(player);
            true
        }
        Err(why) => {
            warnings.push(format!(
                "{}: keyframe animation was not exported: {why}",
                node.name
            ));
            used
        }
    }
}

fn wire_buttons(scene: &mut Scene) {
    fn properties(node: &Node) -> Option<&Vec<(String, GdValue)>> {
        node.meta.iter().find_map(|(k, v)| match (k.as_str(), v) {
            ("properties", GdValue::Dict(p)) => Some(p),
            _ => None,
        })
    }
    fn text<'a>(props: &'a [(String, GdValue)], name: &str) -> &'a str {
        props
            .iter()
            .find_map(|(k, v)| match (k.as_str(), v) {
                (k, GdValue::Str(s)) if k == name => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("")
    }
    fn collect(node: &Node, path: String, targets: &mut Vec<(String, String)>) {
        if let Some(props) = properties(node) {
            if let Some(player) = node
                .children
                .iter()
                .find(|c| c.properties.iter().any(|(k, _)| k == "default_clip"))
            {
                if !text(props, "targetname").is_empty() {
                    targets.push((
                        text(props, "targetname").into(),
                        format!("{path}/{}", player.name),
                    ));
                }
            }
        }
        for c in &node.children {
            collect(
                c,
                if path.is_empty() {
                    c.name.clone()
                } else {
                    format!("{path}/{}", c.name)
                },
                targets,
            );
        }
    }
    fn add(
        node: &mut Node,
        path: String,
        targets: &[(String, String)],
        script: &str,
        connections: &mut Vec<scene_export::scene::Connection>,
    ) -> bool {
        let mut used = false;
        for c in &mut node.children {
            let child_path = if path.is_empty() {
                c.name.clone()
            } else {
                format!("{path}/{}", c.name)
            };
            used |= add(c, child_path, targets, script, connections);
        }
        let Some(props) = properties(node) else {
            return used;
        };
        let class = text(props, "classname");
        if !["func_button", "trigger_multiple", "trigger_once"].contains(&class) {
            return used;
        }
        let destinations: Vec<_> = targets
            .iter()
            .filter(|(name, _)| name == text(props, "target"))
            .map(|(_, p)| p)
            .collect();
        if destinations.is_empty() {
            return used;
        }
        let wait=props.iter().find_map(|(k,v)|if k=="wait"{match v{GdValue::Float(v)=>Some(*v),GdValue::Int(v)=>Some(*v as f64),GdValue::Str(v)=>v.parse().ok(),_=>None}}else{None}).unwrap_or(if class=="trigger_multiple"{1.0}else{0.0});
        let once=props.iter().find_map(|(k,v)|if k=="animation_once"{Some(match v{GdValue::Str(v)=>v=="1",GdValue::Int(v)=>*v!=0,GdValue::Bool(v)=>*v,_=>false})}else{None}).unwrap_or(class=="trigger_once"||wait<0.0);
        let delay=if wait.is_finite(){wait.max(0.0)}else{0.0};
        let action=text(props,"animation_action").to_owned();let clip=text(props,"animation_clip").to_owned();
        let mut name = "HeronAnimationTrigger".to_string();
        let mut suffix = 1;
        while node.children.iter().any(|c| c.name == name) {
            suffix += 1;
            name = format!("HeronAnimationTrigger{suffix}");
        }
        let mut relay = Node::new(&name, "Node");
        relay.set("script", GdValue::SubResource(script.into()));
        // Go from the relay to this exported scene's root, then to each target.
        let prefix = "../".repeat(path.split('/').count() + 1);
        relay.set(
            "targets",
            GdValue::Array(
                destinations
                    .into_iter()
                    .map(|p| GdValue::NodePath(format!("{prefix}{p}")))
                    .collect(),
            ),
        );
        relay.set("once", GdValue::Bool(once));
        relay.set("delay", GdValue::Float(delay));
        relay.set("action",GdValue::Str(action));
        relay.set("clip",GdValue::Str(clip));
        connections.push(scene_export::scene::Connection {
            signal: "body_entered".into(),
            from: path.clone(),
            to: format!("{path}/{name}"),
            method: "_on_body_entered".into(),
        });
        node.add_child(relay);
        true
    }
    let mut targets = Vec::new();
    collect(&scene.root, String::new(), &mut targets);
    let script = format!("HeronAnimationTrigger_{}", scene.sub_resources.len());
    if add(
        &mut scene.root,
        String::new(),
        &targets,
        &script,
        &mut scene.connections,
    ) {
        scene.sub_resources.push(SubResource {
            id: script,
            ty: "GDScript".into(),
            properties: vec![(
                "script/source".into(),
                GdValue::Str(include_str!("godot_animation_trigger.gd").into()),
            )],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{animation_curves::Interpolation, document::Document, export, world::WorldBounds};
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};

    fn entity(keys: &str) -> Node {
        let mut node = Node::new("Moving", "AnimatableBody3D");
        node.meta.push((
            "properties".into(),
            GdValue::Dict(vec![
                ("animation_keys".into(), GdValue::Str(keys.into())),
                ("animation_pivot".into(), GdValue::Str("16 32 48".into())),
            ]),
        ));
        node
    }
    #[test]
    fn validates_before_adding_player_and_preserves_user_script() {
        let mut scene = Scene::default();
        let mut valid = entity("0 0 0 0 0 0 0; 2 32 0 0 360 720 -360");
        valid.script = Some("res://custom.gd".into());
        valid.add_child(Node::new("HeronAnimation", "Node"));
        scene.root.add_child(valid);
        scene.root.add_child(entity("broken"));
        let mut warnings = vec![];
        attach(&mut scene, 1.0 / 32.0, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert_eq!(scene.sub_resources.len(), 3);
        let valid = &scene.root.children[0];
        assert_eq!(valid.script.as_deref(), Some("res://custom.gd"));
        assert_eq!(valid.children[1].name, "HeronAnimation2");
        assert!(scene.root.children[1].children.is_empty());
        let text = scene_export::writer::tscn::write(&scene);
        assert!(text.contains("script/source ="));
        assert!(text.contains("pivot = Vector3(1.0, 1.5, 0.5)"));
    }

    #[test]
    #[ignore = "Runs an installed Godot 4 binary and imports generated meshes"]
    fn godot_animation_matches_editor_and_moves_collision() {
        let dir =
            std::env::temp_dir().join(format!("heron-godot-animation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("project.godot"),"config_version=5\n[application]\nconfig/name=\"Heron animation checks\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
        let mut doc = Document::new(WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(Vec3d::new(32., 64., 96.), Vec3d::new(96., 128., 160.)),
            "stone",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
        doc.select_all();
        let id = doc.create_brush_entity("func_animate").unwrap();
        let base = Track::parse(
            "0 0 0 0 0 0 0; 1 32 -48 64 360 180 -90; 2 -32 64 48 720 360 90",
            "once",
        )
        .unwrap();
        let mut clip = Clip::from_legacy(&base);
        clip.tracks[0][0].interpolation = Interpolation::Bezier;
        clip.tracks[0][0].outgoing = 90.;
        clip.tracks[0][1].incoming = -25.;
        clip.tracks[1][0].interpolation = Interpolation::Hold;
        clip.tracks[2].remove(1); // Independent key times.
        let pivot = Vec3d::new(16., 32., 48.);
        doc.set_entity_property(id, "animation_keys", &clip.legacy_text());
        doc.set_entity_property(id, "animation_curves", &clip.json());
        doc.set_entity_property(id, "animation_pivot", "16 32 48");
        let mut cases = vec![];
        let clip_json = clip.json();
        for (index, (mode, on, legacy)) in [
            ("once", true, false),
            ("loop", true, false),
            ("pingpong", true, false),
            ("once", false, false),
            ("loop", true, true),
        ]
        .into_iter()
        .enumerate()
        {
            doc.set_entity_property(id, "animation_mode", mode);
            doc.set_entity_property(id, "start_on", if on { "1" } else { "0" });
            doc.set_entity_property(id, "animation_curves", if legacy { "" } else { &clip_json });
            let name = format!("case{index}");
            let report = export::export(
                &doc,
                &dir,
                &export::Options {
                    target: export::Target::Godot,
                    name: name.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(
                !report.warnings.iter().any(|w| w.contains("keyframe")),
                "{:?}",
                report.warnings
            );
            let track = Track::from_properties(
                &clip.legacy_text(),
                mode,
                if legacy { None } else { Some(&clip_json) },
            )
            .unwrap();
            let samples: Vec<_> = [0., 0.125, 0.5, 0.9999, 1., 1.25, 2., 2.5, 4.5]
                .into_iter()
                .map(|time| {
                    // The mesh has its rest origin at the brush centre (64,96,128).
                    let points: Vec<_> = [
                        Vec3d::new(64., 96., 128.),
                        Vec3d::new(96., 96., 128.),
                        Vec3d::new(64., 128., 128.),
                        Vec3d::new(64., 96., 160.),
                    ]
                    .into_iter()
                    .map(|p| {
                        let v = track.transform(p, pivot, time, false);
                        vec![v.y() / 32., v.z() / 32., v.x() / 32.]
                    })
                    .collect();
                    serde_json::json!({"time":time,"points":points})
                })
                .collect();
            cases.push(serde_json::json!({"scene":format!("res://{name}.tscn"),"autoplay":on,"samples":samples}));
        }
        let mut open = Clip {
            version: 1,
            duration: 1.0,
            fps: 30,
            tracks: std::array::from_fn(|_| vec![]),
        };
        open.insert(5, 0.0, 0.0).unwrap();
        open.insert(5, 1.0, 90.0).unwrap();
        for key in &mut open.tracks[5] {
            key.interpolation = Interpolation::Bezier;
        }
        crate::animation_library::Library::door(open).write(&mut doc, id, "open");
        doc.set_entity_property(id, "start_on", "0");
        doc.set_entity_property(id, "targetname", "test_door");
        doc.add_brushes(vec![map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(Vec3d::new(192., 64., 96.), Vec3d::new(224., 96., 128.)),
            "stone",
        )
        .unwrap()]);
        let button = doc.create_brush_entity("func_button").unwrap();
        doc.set_entity_property(button, "target", "test_door");
        export::export(
            &doc,
            &dir,
            &export::Options {
                target: export::Target::Godot,
                name: "door".into(),
                ..Default::default()
            },
        )
        .unwrap();
        std::fs::write(dir.join("door.map"), doc.to_map_text()).unwrap();
        std::fs::write(
            dir.join("expected.json"),
            serde_json::to_string(&cases).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("check.gd"),
            include_str!("godot_animation_check.gd"),
        )
        .unwrap();
        let godot = std::env::var("GODOT_BIN").unwrap_or_else(|_| "godot".into());
        for args in [
            vec!["--headless", "--editor", "--import"],
            vec!["--headless", "--script", "res://check.gd"],
        ] {
            let output = std::process::Command::new(&godot)
                .args(["--path", dir.to_str().unwrap()])
                .args(args)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success()
                    && !stderr.contains("SCRIPT ERROR")
                    && !stderr.contains("ERROR:"),
                "{stdout}\n{stderr}\nFixture: {}",
                dir.display()
            );
            println!("{stdout}");
        }
        println!("Godot fixture: {}", dir.display());
    }
}
