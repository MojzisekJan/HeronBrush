//! Agent-facing animation operations. Validate complete drafts before a single undoable edit.
use super::*;
use crate::animation_curves::{CHANNELS, Clip, Interpolation};

pub(super) fn definitions() -> Vec<Value> {
    let id = json!({"type":"string","minLength":1,"description":"Brush entity ID as a decimal string. Use make_brush_entity for loose selected brushes."});
    let key = json!({"type":"object","additionalProperties":false,"required":["time","value"],"properties":{
        "time":{"type":"number","minimum":0},"value":{"type":"number","minimum":-100000,"maximum":100000},
        "interpolation":{"type":"string","enum":["linear","hold","bezier"]},"incoming":{"type":"number"},"outgoing":{"type":"number"}}});
    let common = json!({"entity":id,"duration":{"type":"number","minimum":0.001,"maximum":100000},
        "fps":{"type":"integer","minimum":1,"maximum":120},"mode":{"type":"string","enum":["once","loop","pingpong"]},
        "pivot":vec3_schema("World-space rotation pivot; preserves existing pivot, otherwise entity bounds centre"),
        "dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}});
    let mut set = common.clone();
    set["tracks"] = json!({"type":"array","maxItems":6,"items":{"type":"object","additionalProperties":false,"required":["channel","keys"],"properties":{
        "channel":{"type":"string","enum":CHANNELS},"keys":{"type":"array","maxItems":256,"items":key}}}});
    let mut recipe = common;
    recipe["recipe"] = json!({"type":"string","enum":["bob","spin","shuttle","door"]});
    recipe["axis"] = json!({"type":"string","enum":["x","y","z"]});
    recipe["amount"] = json!({"type":"number","minimum":-100000,"maximum":100000});
    vec![
        tool("configure_animation_trigger", "Configure an animation button or trigger. source is the button/trigger ID; target_entity optionally assigns a target by ID. action: toggle, play, open, close, pause, resume, stop. Empty clip plays the selected clip. once overrides class defaults. wait is repeat cooldown seconds. All options omitted preserve settings. One Undo; dry_run and revision checks.",json!({"source":id,"target_entity":id,"action":{"type":"string","enum":crate::animation_trigger::ACTIONS},"clip":{"type":"string"},"once":{"type":"boolean"},"wait":{"type":"number","minimum":0,"maximum":100000},"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}}), &["source"]),
        tool("get_animation_trigger", "Read trigger settings and unresolved target/clip warnings.",json!({"source":id}), &["source"]),
        tool("preview_animation_trigger", "Activate a trigger in the live 2D/3D editor preview without editing the map. reset clears playback and once/cooldown state. Headless mode also evaluates the trigger state.",json!({"source":id,"reset":{"type":"boolean"}}), &["source"]),
        tool("animation_library", "List, select, duplicate (save_as), or delete named clips. All ordinary animation tools edit the selected clip. One Undo, dry_run and revision checks.",json!({"entity":id,"action":{"type":"string","enum":["list","select","save_as","delete"]},"name":{"type":"string"},"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}}), &["entity","action"]),
        tool("connect_animation_button", "Connect a func_button, trigger_multiple or trigger_once to an animated brush entity by target/targetname. Godot, Unity, Defold, Bevy and raylib exports toggle open/close clips, or play the selected clip. Does not replace scripts.", json!({"entity":id,"button":id,"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}}), &["entity","button"]),
        tool("set_animation_pivot", "Change the rest-space rotation pivot for the entire animation without rewriting keys or geometry. Rotating poses change around the new pivot; this does not preserve their world-space position. One Undo; supports dry_run and expected_revision.", json!({
            "entity":id,"pivot":vec3_schema("Rest-space rotation pivot, each coordinate between -100000 and 100000"),
            "dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}
        }), &["entity","pivot"]),
        tool("key_animation_pose", "Insert or update a pose at a frame without changing rest geometry. X/Y/Z edits record a complete position key including unchanged coordinates. channel is X/Y/Z/RX/RY/RZ, value is absolute animation offset or Euler degrees. Creates a zero key at time zero for an empty channel. Supports dry_run and expected_revision.", json!({
            "entity":id,"frame":{"type":"integer","minimum":0},"channel":{"type":"string","enum":CHANNELS},
            "value":{"type":"number","minimum":-100000,"maximum":100000},"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}
        }), &["entity","frame","channel","value"]),
        tool("retime_animation", "Scale and shift existing keys atomically, preserving normalized Bezier curves. Omit key_ids for all keys. Times are seconds. Positive scale uses pivot_seconds (default 0). Refuses crossing keys. extend_duration optionally grows the clip. Supports dry_run and expected_revision.", json!({
            "entity":id,"key_ids":{"type":"array","minItems":1,"maxItems":1536,"items":{"type":"string","minLength":1}},
            "scale":{"type":"number","minimum":0.001,"maximum":1000},
            "pivot_seconds":{"type":"number","minimum":0},"offset_seconds":{"type":"number"},
            "extend_duration":{"type":"boolean"},"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0}
        }), &["entity"]),
        tool(
            "get_animation",
            "Read an entity's independent animation tracks, stable key IDs, timing, pivot and map revision.",
            json!({"entity":id}),
            &["entity"],
        ),
        tool(
            "set_animation",
            "Replace animation tracks atomically in one Undo. Omitted channels are zero. Times are seconds, rotations degrees, slopes value/second. dry_run validates without editing. expected_revision prevents stale writes.",
            set,
            &["entity", "duration", "tracks"],
        ),
        tool(
            "apply_animation_recipe",
            "High-level agent shortcut: bob (smooth out-and-back), spin (continuous rotation), shuttle (ping-pong translation). Door creates smooth open/close rotation clips and disables autoplay. Defaults: 2 seconds, 30 FPS, Z, amount 16/360/128/90. Returns editable tracks. Supports dry_run and expected_revision.",
            recipe,
            &["entity", "recipe"],
        ),
        tool(
            "sample_animation",
            "Evaluate a precise frame without editing geometry. show_preview also requests a paused GUI preview on a live MCP connection; headless mode only returns the pose.",
            json!({"entity":id,"frame":{"type":"integer","minimum":0},"show_preview":{"type":"boolean"}}),
            &["entity", "frame"],
        ),
    ]
}
fn target(d: &Document, args: &Value) -> Result<map_model::node::NodeId, String> {
    let id = map_model::node::NodeId(extensions::node_id(&args["entity"])?);
    if !map_model::node::entities_with_ids(d.tree_world())
        .iter()
        .any(|(n, e)| *n == id && e.is_brush_entity())
    {
        return Err("entity must identify an existing brush entity".into());
    }
    Ok(id)
}
fn read(d: &Document, id: map_model::node::NodeId) -> Result<(Clip, String, Vec3d), String> {
    let (_, e) = map_model::node::entities_with_ids(d.tree_world())
        .into_iter()
        .find(|(n, _)| *n == id)
        .ok_or("entity no longer exists")?;
    let bounds = geometry_math::bbox::Bbox3d::merge_all(
        map_model::node::contents_of(d.tree_world(), id)
            .into_iter()
            .filter_map(|n| d.brush(n))
            .flat_map(|b| [b.bounds().min, b.bounds().max]),
    )
    .ok_or("entity has no brushes")?;
    let mode = e.property("animation_mode").unwrap_or("loop").to_owned();
    let clip = if let Some(curves) = e.property("animation_curves").filter(|s| !s.is_empty()) {
        Clip::parse(curves)?
    } else if let Some(keys) = e.property("animation_keys") {
        Clip::from_legacy(&crate::keyframe_animation::Track::parse(keys, &mode)?)
    } else {
        Clip {
            version: 1,
            duration: 2.0,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        }
    };
    let pivot = if let Some(text) = e.property("animation_pivot") {
        let values = text
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "invalid animation pivot")?;
        if values.len() != 3 || values.iter().any(|v| !v.is_finite()) {
            return Err("invalid animation pivot".into());
        }
        Vec3d::new(values[0], values[1], values[2])
    } else {
        bounds.center()
    };
    Ok((clip, mode, pivot))
}
pub(super) fn call(
    d: &mut Document,
    collab: &mut Collab,
    name: &str,
    args: &Value,
) -> Result<String, String> {
    if matches!(name,"configure_animation_trigger"|"get_animation_trigger"|"preview_animation_trigger") {
        let source=map_model::node::NodeId(extensions::node_id(&args["source"])?);
        let mut config=crate::animation_trigger::Config::read(d,source)?;
        if name=="preview_animation_trigger" {
            let reset=args["reset"].as_bool().unwrap_or(false);
            if reset {d.reset_trigger_preview();}else{d.preview_trigger(source)?;}
            collab.trigger_preview=Some(!reset);
            return Ok(json!({"source":source.0.to_string(),"preview_requested":!reset,"reset":reset}).to_string());
        }
        if name=="configure_animation_trigger" {
            if args["expected_revision"].as_u64().is_some_and(|r|r!=d.revision()){return Err("map revision changed".into());}
            if let Some(v)=args["action"].as_str(){config.action=v.into();}
            if let Some(v)=args["clip"].as_str(){config.clip=v.into();}
            if let Some(v)=args["once"].as_bool(){config.once=v;}
            if let Some(v)=args["wait"].as_f64(){config.wait=v;}
            let target=if args.get("target_entity").is_some(){Some(map_model::node::NodeId(extensions::node_id(&args["target_entity"])?))}else{None};
            config=crate::animation_trigger::configure(d,source,config,target,args["dry_run"].as_bool().unwrap_or(false))?;
        }
        return Ok(json!({"source":source.0.to_string(),"config":config,"warning":config.targets(d).err(),"revision":d.revision(),"dry_run":args["dry_run"].as_bool().unwrap_or(false)}).to_string());
    }
    let id = target(d, args)?;
    if matches!(name,"animation_library"|"connect_animation_button") {
        if args["expected_revision"].as_u64().is_some_and(|r|r!=d.revision()) {return Err("map revision changed".into());}
        let dry=args["dry_run"].as_bool().unwrap_or(false);
        if name=="animation_library" {
            let (mut library,current)=crate::animation_library::Library::read(d,id)?;
            let action=args["action"].as_str().ok_or("Missing action")?;
            let selected=if action=="list" {current} else {library.edit(&current,action,args["name"].as_str().ok_or("Missing animation name")?)?};
            if action!="list" && !dry {d.run_as_one_edit(|d|library.write(d,id,&selected));}
            return Ok(json!({"entity":id.0.to_string(),"selected":selected,"library":library,"revision":d.revision(),"dry_run":dry}).to_string());
        }
        let button=map_model::node::NodeId(extensions::node_id(&args["button"])?);
        let entities=map_model::node::entities_with_ids(d.tree_world());
        let (_,source)=entities.iter().find(|(n,_)|*n==button).ok_or("Button not found")?;
        if !["func_button","trigger_multiple","trigger_once"].contains(&source.classname().unwrap_or("")) {return Err("Expected func_button, trigger_multiple or trigger_once".into());}
        let (_,target)=entities.iter().find(|(n,_)|*n==id).unwrap();
        if target.property("animation_keys").is_none() {return Err("Add the target animation first".into());}
        let targetname=target.property("targetname").filter(|s|!s.is_empty()).map(str::to_owned).unwrap_or_else(||format!("animation_{}",id.0));
        if !dry {d.run_as_one_edit(|d|{d.set_entity_property(id,"targetname",&targetname);d.set_entity_property(button,"target",&targetname);});}
        return Ok(json!({"entity":id.0.to_string(),"button":button.0.to_string(),"targetname":targetname,"revision":d.revision(),"dry_run":dry}).to_string());
    }
    let (old, old_mode, old_pivot) = read(d, id)?;
    if name == "get_animation" {
        return Ok(json!({"entity":id.0.to_string(),"revision":d.revision(),"clip":old,"mode":old_mode,"pivot":old_pivot.v}).to_string());
    }
    if name == "sample_animation" {
        let frame = args["frame"].as_u64().ok_or("invalid frame")?;
        if frame as f64 > old.duration * old.fps as f64 + 1e-6 || frame > u32::MAX as u64 {
            return Err("frame outside animation duration".into());
        }
        let track =
            crate::keyframe_animation::Track::from_properties("", "once", Some(&old.json()))?;
        let (position, rotation) = track.pose(frame as f64 / old.fps as f64);
        let requested = args["show_preview"].as_bool().unwrap_or(false);
        if requested {
            collab.animation_preview = Some((frame as u32, old.fps));
        }
        return Ok(json!({"entity":id.0.to_string(),"frame":frame,"position":position.v,"rotation_degrees":rotation.v,"pivot":old_pivot.v,"preview_requested":requested}).to_string());
    }
    if args["expected_revision"]
        .as_u64()
        .is_some_and(|r| r != d.revision())
    {
        return Err("map revision changed; read the animation again before writing".into());
    }
    if name == "set_animation_pivot" {
        let pivot=vec3(args,"pivot")?;
        crate::animation_pose::Pose::validate_pivot(pivot)?;
        let dry=args["dry_run"].as_bool().unwrap_or(false);
        if !dry {d.run_as_one_edit(|d| {d.set_entity_property(id,"animation_pivot",&format!("{} {} {}",pivot.x(),pivot.y(),pivot.z()));});}
        return Ok(json!({"entity":id.0.to_string(),"revision":d.revision(),"dry_run":dry,"pivot":pivot.v,"clip":old}).to_string());
    }
    let mut clip = Clip {
        version: 1,
        duration: args["duration"].as_f64().unwrap_or(2.0),
        fps: args["fps"].as_u64().unwrap_or(30) as u32,
        tracks: std::array::from_fn(|_| Vec::new()),
    };
    let mut mode = args["mode"].as_str().unwrap_or("loop").to_owned();
    if name == "key_animation_pose" {
        let time=args["frame"].as_u64().ok_or("Invalid frame")? as f64 / old.fps as f64;
        let pose=crate::animation_pose::Pose::read(d,id,time)?;
        let channel=CHANNELS.iter().position(|c|Some(*c)==args["channel"].as_str()).ok_or("Invalid channel")?;
        clip=pose.keyed(channel,args["value"].as_f64().ok_or("Invalid value")?-old.sample(channel,time))?;
        mode=old_mode.clone();
    } else if name == "retime_animation" {
        clip = old.clone();
        mode = old_mode.clone();
        let ids = if let Some(keys) = args["key_ids"].as_array() {
            keys.iter().map(extensions::node_id).collect::<Result<std::collections::BTreeSet<_>, _>>()?
        } else { clip.tracks.iter().flatten().map(|k| k.id).collect() };
        clip.retime(&ids, args["scale"].as_f64().unwrap_or(1.0), args["pivot_seconds"].as_f64().unwrap_or(0.0), args["offset_seconds"].as_f64().unwrap_or(0.0), args["extend_duration"].as_bool().unwrap_or(false))?;
    } else if name == "apply_animation_recipe" {
        let recipe = args["recipe"].as_str().ok_or("missing recipe")?;
        let axis = match args["axis"].as_str().unwrap_or("z") {
            "x" => 0,
            "y" => 1,
            _ => 2,
        };
        let amount = args["amount"].as_f64().unwrap_or(match recipe {
            "spin" => 360.0,
            "door" => 90.0,
            "shuttle" => 128.0,
            _ => 16.0,
        });
        let channel = axis + if recipe == "spin" || recipe == "door" { 3 } else { 0 };
        clip.insert(channel, 0.0, 0.0)?;
        if recipe == "bob" {
            clip.insert(channel, clip.duration * 0.5, amount)?;
            clip.insert(channel, clip.duration, 0.0)?;
            for k in &mut clip.tracks[channel] {
                k.interpolation = Interpolation::Bezier;
            }
        } else {
            clip.insert(channel, clip.duration, amount)?;
            if recipe == "door" {
                mode="once".into();
                for k in &mut clip.tracks[channel] {k.interpolation=Interpolation::Bezier;}
            }
            if recipe == "shuttle" && args.get("mode").is_none() {
                mode = "pingpong".into();
            }
        }
    } else {
        let mut channels = std::collections::BTreeSet::new();
        for track in args["tracks"].as_array().ok_or("tracks must be an array")? {
            let channel = CHANNELS
                .iter()
                .position(|c| Some(*c) == track["channel"].as_str())
                .ok_or("unknown channel")?;
            if !channels.insert(channel) {
                return Err("duplicate channel".into());
            }
            for value in track["keys"].as_array().ok_or("keys must be an array")? {
                let time = value["time"].as_f64().ok_or("missing key time")?;
                if clip.tracks[channel]
                    .iter()
                    .any(|k| (k.time - time).abs() < 1e-6)
                {
                    return Err("duplicate key time".into());
                }
                let id = clip.insert(
                    channel,
                    time,
                    value["value"].as_f64().ok_or("missing key value")?,
                )?;
                let key = clip.key_mut(id).unwrap();
                key.interpolation = match value["interpolation"].as_str().unwrap_or("linear") {
                    "hold" => Interpolation::Hold,
                    "bezier" => Interpolation::Bezier,
                    _ => Interpolation::Linear,
                };
                key.incoming = value["incoming"].as_f64().unwrap_or(0.0);
                key.outgoing = value["outgoing"].as_f64().unwrap_or(0.0);
            }
        }
    }
    clip.validate()?;
    let pivot = if args.get("pivot").is_some() {
        vec3(args, "pivot")?
    } else {
        old_pivot
    };
    let door = if name=="apply_animation_recipe" && args["recipe"].as_str()==Some("door") {
        let (mut library,_)=crate::animation_library::Library::read(d,id)?;
        library.clips.extend(crate::animation_library::Library::door(clip.clone()).clips);
        library.validate()?;Some(library)
    } else {None};
    let dry = args["dry_run"].as_bool().unwrap_or(false);
    if !dry {
        d.run_as_one_edit(|d| {
            d.set_entity_property(id, "animation_keys", &clip.legacy_text());
            d.set_entity_property(id, "animation_curves", &clip.json());
            d.set_entity_property(id, "animation_mode", &mode);
            d.set_entity_property(
                id,
                "animation_pivot",
                &format!("{} {} {}", pivot.x(), pivot.y(), pivot.z()),
            );
            if let Some(library)=&door {
                library.write(d,id,"open");
                d.set_entity_property(id,"start_on","0");
            }
        });
    }
    Ok(json!({"entity":id.0.to_string(),"revision":d.revision(),"dry_run":dry,"clip":clip,"mode":mode,"pivot":pivot.v}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_trigger_configuration_and_preview_share_validation(){
        let (mut d,target,source,_)=crate::animation_trigger::tests::fixture();let mut c=Collab::default();let before=d.to_map_text();
        let args=json!({"source":source.0.to_string(),"target_entity":target.0.to_string(),"action":"play","clip":"route","once":false,"wait":0.0,"dry_run":true});
        let call=|d:&mut Document,c:&mut Collab,name,args:&Value|super::super::call_tool(d,c,name,args);
        call(&mut d,&mut c,"configure_animation_trigger",&args).unwrap();assert_eq!(before,d.to_map_text());
        let mut args=args;args["dry_run"]=json!(false);args["expected_revision"]=json!(0);assert!(call(&mut d,&mut c,"configure_animation_trigger",&args).is_err());args.as_object_mut().unwrap().remove("expected_revision");
        call(&mut d,&mut c,"configure_animation_trigger",&args).unwrap();let saved=d.to_map_text();
        let mut args=json!({"source":source.0.to_string()});
        let read:Value=serde_json::from_str(&call(&mut d,&mut c,"get_animation_trigger",&args).unwrap()).unwrap();assert!(read["warning"].is_null());assert_eq!(read["config"]["action"],"play");
        call(&mut d,&mut c,"preview_animation_trigger",&args).unwrap();assert_eq!(c.trigger_preview,Some(true));assert_eq!(saved,d.to_map_text());args["reset"]=json!(true);call(&mut d,&mut c,"preview_animation_trigger",&args).unwrap();assert_eq!(c.trigger_preview,Some(false));
        d.undo();assert_eq!(before,d.to_map_text());
    }
    #[test]
    fn agent_named_door_and_button_are_atomic() {
        let mut d=Document::new(WorldBounds::default());let mut c=Collab::default();
        super::super::call_tool(&mut d,&mut c,"add_shape",&json!({"kind":"cuboid","min":[0,0,0],"max":[32,32,64]})).unwrap();
        let id=d.create_brush_entity("func_animate").unwrap();
        let before=d.to_map_text();
        let args=json!({"entity":id.0.to_string(),"recipe":"door","amount":-90,"dry_run":true});
        super::super::call_tool(&mut d,&mut c,"apply_animation_recipe",&args).unwrap();assert_eq!(d.to_map_text(),before);
        let mut args=args;args["dry_run"]=json!(false);
        super::super::call_tool(&mut d,&mut c,"apply_animation_recipe",&args).unwrap();
        let (library,active)=crate::animation_library::Library::read(&d,id).unwrap();assert_eq!(active,"open");
        assert_eq!(library.clips["open"].clip.sample(5,2.),-90.);assert_eq!(library.clips["close"].clip.sample(5,2.),0.);
        d.undo();assert_eq!(d.to_map_text(),before);d.redo();
        let before=d.to_map_text();
        super::super::call_tool(&mut d,&mut c,"animation_library",&json!({"entity":id.0.to_string(),"action":"select","name":"close","dry_run":true})).unwrap();assert_eq!(d.to_map_text(),before);
        assert!(super::super::call_tool(&mut d,&mut c,"animation_library",&json!({"entity":id.0.to_string(),"action":"delete","name":"open","expected_revision":0})).is_err());
        super::super::call_tool(&mut d,&mut c,"add_shape",&json!({"kind":"cuboid","min":[64,0,0],"max":[80,16,16]})).unwrap();
        let button=d.create_brush_entity("func_button").unwrap();let before=d.to_map_text();
        let args=json!({"entity":id.0.to_string(),"button":button.0.to_string()});
        super::super::call_tool(&mut d,&mut c,"connect_animation_button",&args).unwrap();
        let entities=map_model::node::entities_with_ids(d.tree_world());
        let target=entities.iter().find(|(n,_)|*n==id).unwrap().1.property("targetname").unwrap();
        assert_eq!(entities.iter().find(|(n,_)|*n==button).unwrap().1.property("target"),Some(target));
        d.undo();assert_eq!(d.to_map_text(),before);
    }
    #[test]
    fn agent_recipes_validate_preview_and_undo_atomically() {
        let mut d = Document::new(WorldBounds::default());
        let mut c = Collab::default();
        super::super::call_tool(
            &mut d,
            &mut c,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[32,32,32]}),
        )
        .unwrap();
        let id = d.create_brush_entity("func_animate").unwrap();
        let before = d.to_map_text();
        let revision = d.revision();
        let args = json!({"entity":id.0.to_string(),"recipe":"bob","amount":24,"duration":4,"dry_run":true});
        super::super::call_tool(&mut d, &mut c, "apply_animation_recipe", &args).unwrap();
        assert_eq!(d.to_map_text(), before);
        assert_eq!(d.revision(), revision);
        let mut args = args;
        args["dry_run"] = json!(false);
        args["expected_revision"] = json!(revision);
        super::super::call_tool(&mut d, &mut c, "apply_animation_recipe", &args).unwrap();
        let saved = d.to_map_text();
        let result = super::super::call_tool(
            &mut d,
            &mut c,
            "sample_animation",
            &json!({"entity":id.0.to_string(),"frame":60,"show_preview":true}),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["position"][2], 24.0);
        assert_eq!(c.animation_preview, Some((60, 30)));
        assert_eq!(d.to_map_text(), saved);
        assert!(
            super::super::call_tool(&mut d, &mut c, "apply_animation_recipe", &args).is_err(),
            "stale revision"
        );
        assert_eq!(d.to_map_text(), saved);
        let bad = json!({"entity":id.0.to_string(),"duration":1,"tracks":[{"channel":"X","keys":[{"time":0,"value":0},{"time":2,"value":9}]}]});
        assert!(super::super::call_tool(&mut d, &mut c, "set_animation", &bad).is_err());
        assert_eq!(d.to_map_text(), saved);
        d.undo();
        assert_eq!(d.to_map_text(), before);
        d.redo();
        assert_eq!(d.to_map_text(), saved);
        let mut pivot_args=json!({"entity":id.0.to_string(),"pivot":[12,24,36],"dry_run":true});
        super::super::call_tool(&mut d,&mut c,"set_animation_pivot",&pivot_args).unwrap();
        assert_eq!(d.to_map_text(),saved);
        pivot_args["dry_run"]=json!(false);
        let old_clip=read(&d,id).unwrap().0;
        super::super::call_tool(&mut d,&mut c,"set_animation_pivot",&pivot_args).unwrap();
        let updated=read(&d,id).unwrap();
        assert_eq!(updated.0,old_clip);assert_eq!(updated.2,Vec3d::new(12.0,24.0,36.0));
        d.undo();assert_eq!(d.to_map_text(),saved);
        pivot_args["pivot"]=json!([100001,0,0]);
        assert!(super::super::call_tool(&mut d,&mut c,"set_animation_pivot",&pivot_args).is_err());
        assert_eq!(d.to_map_text(),saved);
        let keyed = json!({"entity":id.0.to_string(),"frame":30,"channel":"X","value":42.0});
        super::super::call_tool(&mut d,&mut c,"key_animation_pose",&keyed).unwrap();
        assert_eq!(read(&d,id).unwrap().0.sample(0,1.0),42.0);
        d.undo();assert_eq!(d.to_map_text(),saved);
        let mut retime = json!({"entity":id.0.to_string(),"scale":2.0,"extend_duration":true,"dry_run":true});
        super::super::call_tool(&mut d,&mut c,"retime_animation",&retime).unwrap();
        assert_eq!(d.to_map_text(),saved);
        retime["dry_run"] = json!(false);
        super::super::call_tool(&mut d,&mut c,"retime_animation",&retime).unwrap();
        let data = read(&d,id).unwrap();
        assert_eq!(data.0.duration,8.0);
        assert_eq!(data.0.sample(2,4.0),24.0);
        d.undo(); assert_eq!(d.to_map_text(),saved);
        retime["key_ids"] = json!(["999999999"]);
        assert!(super::super::call_tool(&mut d,&mut c,"retime_animation",&retime).is_err());
        assert_eq!(d.to_map_text(),saved);
        for recipe in ["spin", "shuttle"] {
            super::super::call_tool(
                &mut d,
                &mut c,
                "apply_animation_recipe",
                &json!({"entity":id.0.to_string(),"recipe":recipe}),
            )
            .unwrap();
            let data = super::super::call_tool(
                &mut d,
                &mut c,
                "get_animation",
                &json!({"entity":id.0.to_string()}),
            )
            .unwrap();
            assert!(serde_json::from_str::<Value>(&data).unwrap()["clip"]["tracks"].is_array());
        }
    }
}
