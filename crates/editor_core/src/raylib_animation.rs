//! Raylib export: exact scalar tracks compiled into a dependency-free C99 player.
use crate::{
    animation_curves::{Clip, Interpolation},
    animation_library::{Entry, Library},
    keyframe_animation::Track,
};
use scene_export::level::{Level, Value};
fn string<'a>(e: &'a scene_export::level::Entity, k: &str) -> Option<&'a str> {
    match e.properties.get(k) {
        Some(Value::String(s)) => Some(s),
        _ => None,
    }
}
pub(crate) fn quoted(s: &str) -> String {
    // C UTF-8 string, with fixed-width octal escapes for untrusted bytes.
    let mut out = String::from("\"");
    for b in s.bytes() {
        if (32..127).contains(&b) && b != b'"' && b != b'\\' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\{b:03o}"));
        }
    }
    out.push('"');
    out
}
pub fn write(level: &Level, warnings: &mut Vec<String>) -> String {
    let mut out =
        String::from("#ifndef HERON_EXPORTED_ANIMATION_H\n#define HERON_EXPORTED_ANIMATION_H\n");
    out.push_str(include_str!("raylib_animation_runtime.h"));
    let mut clips = Vec::new();
    let mut animations = Vec::new();
    let mut animation_names = Vec::new();
    for (index, e) in level.entities.iter().enumerate() {
        if string(e, "animation_keys").is_none() && string(e, "animation_curves").is_none() {
            continue;
        }
        let result = (|| -> Result<_, String> {
            let name = string(e, "animation_name")
                .filter(|s| !s.is_empty())
                .unwrap_or("default");
            let mode = string(e, "animation_mode").unwrap_or("loop");
            let track = Track::from_properties(
                string(e, "animation_keys").unwrap_or(""),
                mode,
                string(e, "animation_curves"),
            )?;
            let clip = track
                .curves
                .clone()
                .unwrap_or_else(|| Clip::from_legacy(&track));
            let mut library = Library::parse(string(e, "animation_library").unwrap_or(""))?;
            library.clips.insert(
                name.into(),
                Entry {
                    clip,
                    mode: mode.into(),
                },
            );
            library.validate()?;
            let pivot = match e.properties.get("animation_pivot") {
                Some(Value::Vec3(p)) => *p,
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
                    e.transform.translation[2] * level.scale,
                    e.transform.translation[0] * level.scale,
                    e.transform.translation[1] * level.scale,
                ],
                _ => return Err("Invalid pivot".into()),
            };
            if pivot.iter().any(|v| !v.is_finite())
                || !level.scale.is_finite()
                || level.scale <= 0.0
            {
                return Err("Invalid pivot or scale".into());
            }
            let autoplay = match e.properties.get("start_on") {
                Some(Value::Bool(v)) => *v,
                Some(Value::Int(v)) => *v != 0,
                Some(Value::Float(v)) => *v != 0.0,
                Some(Value::String(v)) => v.parse::<f64>().unwrap_or(1.0) != 0.0,
                _ => true,
            };
            Ok((library, name.to_owned(), pivot, autoplay))
        })();
        let (library, name, pivot, autoplay) = match result {
            Ok(v) => v,
            Err(why) => {
                warnings.push(format!("Raylib animation {}: {why}; left static", e.name));
                continue;
            }
        };
        let first = clips.len();
        let mut selected = first;
        for (clip_name, entry) in &library.clips {
            let n = clips.len();
            if *clip_name == name {
                selected = n;
            }
            let mut channels = Vec::new();
            for (axis, keys) in entry.clip.tracks.iter().enumerate() {
                if keys.is_empty() {
                    channels.push("{0,0}".into());
                    continue;
                }
                let symbol = format!("hb_keys_{n}_{axis}");
                out.push_str(&format!("static const HbKey {symbol}[]={{\n"));
                for k in keys {
                    let interp = match k.interpolation {
                        Interpolation::Linear => 0,
                        Interpolation::Hold => 1,
                        Interpolation::Bezier => 2,
                    };
                    out.push_str(&format!(
                        "{{{:.17e},{:.17e},{:.17e},{:.17e},{interp}}},\n",
                        k.time, k.value, k.incoming, k.outgoing
                    ));
                }
                out.push_str("};\n");
                channels.push(format!("{{{symbol},{}}}", keys.len()));
            }
            let mode = match entry.mode.as_str() {
                "once" => 0,
                "pingpong" => 2,
                _ => 1,
            };
            clips.push(format!(
                "{{{},{:.17e},{mode},{{{}}}}}",
                quoted(clip_name),
                entry.clip.duration,
                channels.join(",")
            ));
        }
        animation_names.push(e.name.clone());
        animations.push(format!(
            "{{{},{index},{first},{},{selected},{},{{{:.17e},{:.17e},{:.17e}}},{:.17e}}}",
            quoted(&e.name),
            library.clips.len(),
            i32::from(autoplay),
            pivot[0],
            pivot[1],
            pivot[2],
            level.scale
        ));
    }
    out.push_str(&format!(
        "#define HB_ANIMATION_COUNT {}\n",
        animations.len()
    ));
    out.push_str(&format!(
        "static const HbClip hb_clips[]={{\n{}\n}};\n",
        if clips.is_empty() {
            "{0}".into()
        } else {
            clips.join(",\n")
        }
    ));
    out.push_str(&format!(
        "static const HbAnimation hb_animations[]={{\n{}\n}};\n",
        if animations.is_empty() {
            "{0}".into()
        } else {
            animations.join(",\n")
        }
    ));
    out.push_str(include_str!("raylib_animation_functions.h"));
    out.push_str(&trigger_data(level, &animation_names));
    out.push_str(include_str!("raylib_animation_triggers.h"));
    out.push_str("\n#endif\n");
    out
}

fn trigger_data(level: &Level, names: &[String]) -> String {
    let mut out=String::from("typedef struct {const double (*planes)[4];int count;} HbVolume;\ntypedef struct {const char *name;const HbVolume *volumes;int count;const int *targets;int target_count,once;double wait;const char *action,*clip;} HbTrigger;\n");
    let mut triggers = Vec::new();
    for e in &level.entities {
        let Some(data) = string(e, "_heron_trigger")
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        else {
            continue;
        };
        let targets: Vec<_> = data["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| {
                names
                    .iter()
                    .position(|name| Some(name.as_str()) == n.as_str())
            })
            .collect();
        if targets.is_empty() {
            continue;
        }
        let n = triggers.len();
        let mut volumes = Vec::new();
        for (v, planes) in data["volumes"].as_array().into_iter().flatten().enumerate() {
            let planes: Vec<[f64; 4]> = serde_json::from_value(planes.clone()).unwrap_or_default();
            if planes.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "static const double hb_planes_{n}_{v}[][4]={{{}}};\n",
                planes
                    .iter()
                    .map(|p| format!(
                        "{{{:.17e},{:.17e},{:.17e},{:.17e}}}",
                        p[0], p[1], p[2], p[3]
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            volumes.push(format!("{{hb_planes_{n}_{v},{}}}", planes.len()));
        }
        let count = volumes.len();
        out.push_str(&format!(
            "static const HbVolume hb_volumes_{n}[]={{{}}};\n",
            if count == 0 {
                "{0}".into()
            } else {
                volumes.join(",")
            }
        ));
        out.push_str(&format!(
            "static const int hb_targets_{n}[]={{{}}};\n",
            targets
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        triggers.push(format!(
            "{{{},hb_volumes_{n},{count},hb_targets_{n},{},{},{:.17e},{},{}}}",
            quoted(&e.name),
            targets.len(),
            i32::from(data["once"].as_bool().unwrap_or(false)),
            data["wait"].as_f64().unwrap_or(1.0),
            quoted(data["action"].as_str().unwrap_or("toggle")),
            quoted(data["clip"].as_str().unwrap_or(""))
        ));
    }
    out.push_str(&format!(
        "#define HB_TRIGGER_COUNT {}\nstatic const HbTrigger hb_triggers[]={{{}}};\n",
        triggers.len(),
        if triggers.is_empty() {
            "{0}".into()
        } else {
            triggers.join(",")
        }
    ));
    out
}

/// `sky` is the sky cube's image path, when the export wrote one; `water` the water
/// header's, when the level has water.
pub fn preview(level: &Level, header: &str, sky: Option<&str>, water: Option<&str>) -> String {
    let mut out = format!("#include \"raylib.h\"\n#include {}\n", quoted(header));
    if let Some(sky) = sky {
        out.push_str(&format!("#define HB_SKY {}\n", quoted(sky)));
    }
    if let Some(water) = water {
        out.push_str(&format!("#define HB_WATER\n#include {}\n", quoted(water)));
    }
    out.push_str("typedef struct {const char *mesh; double pos[3],rot[3],scale[3];} PreviewEntity;\nstatic const PreviewEntity entities[]={\n");
    for e in &level.entities {
        let array = |v: [f64; 3]| format!("{{{:.17e},{:.17e},{:.17e}}}", v[0], v[1], v[2]);
        out.push_str(&format!(
            "{{{},{},{},{}}},\n",
            quoted(e.mesh.as_deref().unwrap_or("")),
            array(e.transform.translation),
            array(e.transform.rotation_yxz),
            array(e.transform.scale)
        ));
    }
    if level.entities.is_empty() {
        out.push_str("{0},\n");
    }
    out.push_str(&format!(
        "}};\n#define ENTITY_COUNT {}\n",
        level.entities.len()
    ));
    out.push_str(include_str!("raylib_animation_preview.c"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raylib_exported_c_matches_editor_curves_pivots_modes_and_controls() {
        use crate::{document::Document, export};
        use geometry_math::vec::Vec3d;
        let dir =
            std::env::temp_dir().join(format!("heron-raylib-animation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = Document::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
            Default::default(),
        )
        .unwrap();
        d.select_all();
        d.group_selection("Animated group");
        let id = d.create_brush_entity("func_animate").unwrap();
        let mut clip = Clip {
            version: 1,
            duration: 2.0,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        };
        for axis in 0..6 {
            clip.insert(axis, 0.0, axis as f64 * 3.0).unwrap();
            clip.insert(axis, 0.7, axis as f64 * 17.0 + 16.0).unwrap();
            clip.insert(axis, 2.0, axis as f64 * 31.0 - 80.0).unwrap();
        }
        clip.tracks[0][0].interpolation = Interpolation::Bezier;
        clip.tracks[0][0].outgoing = 140.0;
        clip.tracks[0][1].incoming = -30.0;
        clip.tracks[1][0].interpolation = Interpolation::Hold;
        clip.tracks[5][2].value = 720.0;
        let mut library = Library::parse("").unwrap();
        for mode in ["once", "loop", "pingpong"] {
            library.clips.insert(
                mode.into(),
                Entry {
                    clip: clip.clone(),
                    mode: mode.into(),
                },
            );
        }
        library.write(&mut d, id, "loop");
        d.set_entity_property(id, "animation_pivot", "16 -32 48");
        d.set_entity_property(id, "start_on", "0");
        d.set_entity_property(id, "targetname", "animated");
        d.clear_selection();
        let trigger = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::new(Vec3d::new(256., 0., 0.), Vec3d::new(320., 64., 64.)),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![trigger]);
        let trigger = d.create_brush_entity("trigger_once").unwrap();
        d.set_entity_property(trigger, "target", "animated");
        let report = export::export(
            &d,
            &dir,
            &export::Options {
                target: export::Target::Raylib,
                name: "check".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| w.contains("keyframe playback") || w.contains("Raylib animation")),
            "{:?}",
            report.warnings
        );
        assert!(dir.join("check_preview.c").exists());
        let mut c=String::from("#include <assert.h>\n#include <stdio.h>\n#include \"check_animation.h\"\nint main(void){assert(HB_ANIMATION_COUNT==1);HbPlayer p;HbInit(&p,0);assert(!p.playing&&!p.posed);double in[3]={1,2,3},out[3],m[16];HbPoint(0,&p,in,out,0);assert(out[0]==1&&out[1]==2&&out[2]==3);\n");
        let pivot = Vec3d::new(16.0, -32.0, 48.0);
        for mode in ["once", "loop", "pingpong"] {
            let track =
                Track::from_properties(&clip.legacy_text(), mode, Some(&clip.json())).unwrap();
            c.push_str(&format!("assert(HbPlay(&p,0,{}));\n", quoted(mode)));
            for time in [0.0, 0.125, 0.69999, 0.7, 1.2, 2.0, 2.5, 3.5, 4.5, 20.25] {
                for normal in [false, true] {
                    let point = Vec3d::new(120.0, 64.0, -40.0);
                    let result = track.transform(point, pivot, time, normal);
                    let v = point.v;
                    let r = result.v;
                    c.push_str(&format!("HbSeek(&p,{time});{{double in[3]={{{},{},{}}};HbPoint(0,&p,in,out,{});assert(fabs(out[0]-({:.17e}))<1e-8);assert(fabs(out[1]-({:.17e}))<1e-8);assert(fabs(out[2]-({:.17e}))<1e-8);}}\n",v[1]/32.0,v[2]/32.0,v[0]/32.0,i32::from(normal),r[1]/32.0,r[2]/32.0,r[0]/32.0));
                }
            }
        }
        c.push_str("assert(HbPlay(&p,0,\"pingpong\"));HbUpdate(&p,2.5);HbUpdate(&p,.25);assert(fabs(p.time-2.75)<1e-9);HbPause(&p);HbUpdate(&p,1);assert(fabs(p.time-2.75)<1e-9);HbResume(&p);HbUpdate(&p,.25);assert(fabs(p.time-3)<1e-9);HbPlayer old=p;assert(!HbPlay(&p,0,\"missing\"));assert(p.time==old.time);HbMatrix(0,&p,m);HbPoint(0,&p,in,out,0);for(int j=0;j<3;j++)assert(fabs(out[j]-(m[j]*in[0]+m[4+j]*in[1]+m[8+j]*in[2]+m[12+j]))<1e-8);HbPlay(&p,0,\"once\");HbUpdate(&p,10);assert(!p.playing&&p.posed&&p.time==2);HbStop(&p);HbPoint(0,&p,in,out,0);assert(out[0]==1&&out[1]==2&&out[2]==3);assert(HB_TRIGGER_COUNT==1);HbTriggerState states[HB_TRIGGER_COUNT]={{0}};double inside[3]={1,1,9},outside[3]={0,0,0};HbCheckTriggers(states,&p,outside,0);assert(!p.playing);HbCheckTriggers(states,&p,inside,1);assert(p.playing);HbStop(&p);HbCheckTriggers(states,&p,outside,2);HbCheckTriggers(states,&p,inside,3);assert(!p.playing);assert(!HbActivateTrigger(states,&p,0,4));assert(HbAnimationAction(&p,0,\"play\",\"pingpong\"));assert(!strcmp(hb_clips[p.clip].name,\"pingpong\"));assert(HbAnimationAction(&p,0,\"pause\",0));assert(!p.playing);assert(HbAnimationAction(&p,0,\"resume\",0));assert(p.playing);assert(HbAnimationAction(&p,0,\"stop\",0));assert(!p.playing&&!p.posed);assert(!HbAnimationAction(&p,0,\"play\",\"missing\"));{HbTriggerState fresh[HB_TRIGGER_COUNT]={{0}};double feet[3]={1,-1.5,9};HbCheckTriggersSpan(fresh,&p,feet,1,10);assert(!p.playing);HbCheckTriggersSpan(fresh,&p,feet,2,11);assert(p.playing);}puts(\"raylib animation checks passed\");return 0;}\n");
        std::fs::write(dir.join("check.c"), c).unwrap();
        let output = std::process::Command::new("cc")
            .current_dir(&dir)
            .args([
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "check.c",
                "-lm",
                "-o",
                "check-runtime",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = std::process::Command::new(dir.join("check-runtime"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if let Ok(root) = std::env::var("HERON_TEST_RAYLIB") {
            let include = format!("-I{root}/include");
            let lib = format!("{root}/lib/libraylib.a");
            let output = std::process::Command::new("cc")
                .current_dir(&dir)
                .args([
                    "-std=c99",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    &include,
                    "check_preview.c",
                    &lib,
                    "-lGL",
                    "-lm",
                    "-lpthread",
                    "-ldl",
                    "-lrt",
                    "-lX11",
                    "-o",
                    "preview",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        eprintln!("Raylib C fixture: {}", dir.display());
    }
}
