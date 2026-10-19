//! GDevelop 5 export: a scene of 3D model objects in a project's `game.json`.
//!
//! GDevelop has no import step to hand a level to: a project is one JSON file, and a
//! scene ("layout") lists its objects, the instances placed in it, its layers with
//! their lights, and its events. So the exporter writes the scene itself -- one Model3D
//! object per mesh, with Physics3D behaviours, typed instance variables, the editor's
//! sky and sun, and a JavaScript event that plays the level's animation and triggers --
//! and either starts a project around it or replaces the scene of the same name in the
//! project already there, leaving every other scene, object and resource alone.
//!
//! GDevelop's frame is X right, Y down the screen, Z up, in pixels. A Model3D object
//! rotated 90° about X takes a glTF's Y-up frame into it as x = X, y = Z, elevation = Y,
//! and fits the model into its width, height and depth, measured over the model's
//! bounds and its origin. Sizing the object to exactly those bounds keeps the mesh in
//! place, so the brush GLBs need nothing rewritten.
use super::sky::Sky;
use crate::level::{CollisionMesh, Entity, Level, Value};
use serde_json::{json, Value as Json};

/// GDevelop's own scale for 3D physics, and so for everything: 100 pixels a metre.
pub const PIXELS_PER_METRE: f64 = 100.;
/// The sky's cube faces, in pixels.
const SKY_SIZE: u32 = 256;

pub struct GDevelopOptions {
    pub sky: Option<Sky>,
}

/// A mesh the exporter wrote, by its file name in `<export>/<level>/` and its bytes.
pub struct GDevelopMesh<'a> {
    pub file_name: &'a str,
    pub glb: &'a [u8],
}

pub struct GDevelopFile {
    /// Relative to the export directory, which holds `game.json`.
    pub path: String,
    pub contents: Vec<u8>,
}

fn glb_json(glb: &[u8]) -> Option<Json> {
    let length = u32::from_le_bytes(glb.get(12..16)?.try_into().ok()?) as usize;
    serde_json::from_slice(glb.get(20..20 + length)?).ok()
}

/// The visual mesh's bounds, grown to take in the origin as GDevelop's fit does.
fn bounds_with_origin(glb: &[u8]) -> Option<([f64; 3], [f64; 3])> {
    let doc = glb_json(glb)?;
    let mut lo = [0f64; 3];
    let mut hi = [0f64; 3];
    for primitive in doc["meshes"][0]["primitives"].as_array()? {
        let accessor = &doc["accessors"][primitive["attributes"]["POSITION"].as_u64()? as usize];
        for i in 0..3 {
            lo[i] = lo[i].min(accessor["min"][i].as_f64()?);
            hi[i] = hi[i].max(accessor["max"][i].as_f64()?);
        }
    }
    Some((lo, hi))
}

/// A GLB of bare triangles, for a collision shape GDevelop builds from a model.
fn collision_glb(triangles: &[[f32; 3]]) -> Vec<u8> {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    let mut bin = Vec::with_capacity(triangles.len() * 12);
    for p in triangles {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
            bin.extend(p[i].to_le_bytes());
        }
    }
    let doc = json!({
        "asset": {"version": "2.0", "generator": "heron_brush"},
        "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
        "accessors": [{"bufferView": 0, "componentType": 5126, "count": triangles.len(), "type": "VEC3", "min": lo, "max": hi}],
        "bufferViews": [{"buffer": 0, "byteLength": bin.len()}],
        "buffers": [{"byteLength": bin.len()}],
    });
    let mut text = serde_json::to_vec(&doc).unwrap();
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend(((12 + 8 + text.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(text);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    out
}

/// The level's rotation (YXZ Euler, radians) as a quaternion, x y z w.
fn quaternion(entity: &Entity) -> [f64; 4] {
    let [x, y, z] = entity.transform.rotation_yxz;
    glam::DQuat::from_euler(glam::EulerRot::YXZ, y, x, z).to_array()
}

/// A rotation as GDevelop's angle, rotationY and rotationX in degrees: the same
/// rotation seen with Y and Z swapped, taken apart in Z, Y, X order.
pub fn gdevelop_rotation(q: [f64; 4]) -> (f64, f64, f64) {
    let m = glam::DMat3::from_quat(glam::DQuat::from_array(q));
    let p = [0, 2, 1];
    let g = |i: usize, j: usize| m.col(p[j])[p[i]];
    let s = g(2, 0).clamp(-1., 1.);
    let (angle, x) = if s.abs() < 0.9999999 { (g(1, 0).atan2(g(0, 0)), g(2, 1).atan2(g(2, 2))) } else { ((-g(0, 1)).atan2(g(1, 1)), 0.) };
    (angle.to_degrees(), (-s.asin()).to_degrees(), x.to_degrees())
}

/// A name GDevelop accepts for an object: letters, digits and underscores.
fn identifier(name: &str) -> String {
    let mut out: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn variable(name: &str, value: &Value) -> Json {
    let mut v = value_json(value);
    v["name"] = json!(name);
    v
}
fn value_json(value: &Value) -> Json {
    let numbers = |v: &[f64]| json!({"type": "array", "children": v.iter().map(|x| json!({"type": "number", "value": x})).collect::<Vec<_>>()});
    match value {
        Value::Bool(b) => json!({"type": "boolean", "value": b}),
        Value::Int(i) => json!({"type": "number", "value": i}),
        Value::Float(f) => json!({"type": "number", "value": f}),
        Value::String(s) => json!({"type": "string", "value": s}),
        Value::Vec2(v) => numbers(v),
        Value::Vec3(v) => numbers(v),
        Value::Color(v) => numbers(v),
        Value::Array(items) => json!({"type": "array", "children": items.iter().map(value_json).collect::<Vec<_>>()}),
    }
}
fn strings(name: &str, items: &[String]) -> Json {
    json!({"name": name, "type": "array", "children": items.iter().map(|s| json!({"type": "string", "value": s})).collect::<Vec<_>>()})
}

fn uuid(seed: &str) -> String {
    let id = super::stride::asset_id(seed).to_ascii_lowercase();
    format!("{}4{}a{}", &id[..14], &id[15..19], &id[20..])
}

fn physics(body: &str, shape: &str, size: [f64; 3], offset: [f64; 3], mesh: &str) -> Json {
    json!({
        "name": "Physics3D", "type": "Physics3D::Physics3DBehavior", "object3D": "Object3D",
        "bodyType": body, "bullet": false, "fixedRotation": false, "shape": shape,
        "meshShapeResourceName": mesh, "shapeOrientation": "Z",
        "shapeDimensionA": size[0], "shapeDimensionB": size[1], "shapeDimensionC": size[2],
        "shapeOffsetX": offset[0], "shapeOffsetY": offset[1], "shapeOffsetZ": offset[2],
        "massCenterOffsetX": 0, "massCenterOffsetY": 0, "massCenterOffsetZ": 0,
        "density": 1, "massOverride": 0, "friction": 0.3, "restitution": 0.1,
        "linearDamping": 0.1, "angularDamping": 0.1, "gravityScale": 1,
        "layers": if body == "Static" { 1 } else { 16 }, "masks": 17,
    })
}

fn model_object(name: &str, resource: &str, size: [f64; 3], behaviors: Vec<Json>, variables: Vec<Json>) -> Json {
    json!({
        "assetStoreId": "", "name": name, "type": "Scene3D::Model3DObject",
        "variables": variables, "effects": [], "behaviors": behaviors,
        "content": {
            "modelResourceName": resource, "width": size[0], "height": size[1], "depth": size[2],
            "rotationX": 90, "rotationY": 0, "rotationZ": 0, "keepAspectRatio": false,
            "materialType": "KeepOriginal", "originLocation": "ModelOrigin", "centerLocation": "ModelOrigin",
            "crossfadeDuration": 0, "isCastingShadow": true, "isReceivingShadow": true, "animations": [],
        },
    })
}

/// An invisible box to hold a point entity's place and variables.
fn point_object(name: &str) -> Json {
    let mut content = json!({"width": 16, "height": 16, "depth": 16, "enableTextureTransparency": false,
        "facesOrientation": "Y", "backFaceUpThroughWhichAxisRotation": "X", "materialType": "Basic", "tint": "255;255;255",
        "isCastingShadow": false, "isReceivingShadow": false});
    for face in ["front", "back", "left", "right", "top", "bottom"] {
        content[format!("{face}FaceResourceName")] = json!("");
        content[format!("{face}FaceVisible")] = json!(false);
        content[format!("{face}FaceResourceRepeat")] = json!(false);
    }
    json!({"assetStoreId": "", "name": name, "type": "Scene3D::Cube3DObject", "variables": [], "effects": [], "behaviors": [], "content": content})
}

fn resource(name: &str) -> Json {
    let kind = if name.ends_with(".png") { "image" } else { "model3D" };
    let mut r = json!({"file": name, "kind": kind, "metadata": "", "name": name, "userAdded": false});
    if kind == "image" {
        r["smoothed"] = json!(true);
        r["alwaysLoaded"] = json!(false);
    }
    r
}

fn effect(kind: &str, name: &str, doubles: Json, strings: Json, booleans: Json) -> Json {
    json!({"effectType": kind, "name": name, "doubleParameters": doubles, "stringParameters": strings, "booleanParameters": booleans})
}
fn rgb(c: [f32; 3]) -> String {
    c.map(|v| (v.clamp(0., 1.) * 255.).round() as u8).map(|v| v.to_string()).join(";")
}

/// A fresh project holding nothing but what a scene needs to run.
fn new_project(level: &str) -> Json {
    let loading = json!({"backgroundColor": 0, "backgroundFadeInDuration": 0.2, "backgroundImageResourceName": "",
                "gdevelopLogoStyle": "light", "logoAndProgressFadeInDuration": 0.2, "logoAndProgressLogoFadeInDelay": 0.2,
                "minDuration": 0, "progressBarColor": 16777215, "progressBarHeight": 20, "progressBarMaxWidth": 200,
                "progressBarMinWidth": 40, "progressBarWidthPercent": 30, "showGDevelopSplash": true, "showProgressBar": true});
    let properties = json!({
            "adaptGameResolutionAtRuntime": true, "antialiasingMode": "MSAA", "antialisingEnabledOnMobile": false,
            "folderProject": false, "orientation": "landscape", "packageName": format!("com.heronbrush.{}", identifier(level).to_ascii_lowercase()),
            "pixelsRounding": false, "projectUuid": uuid(&format!("project/{level}")), "scaleMode": "linear",
            "sizeOnStartupMode": "adaptWidth", "templateSlug": "", "version": "1.0.0", "name": level,
            "description": "", "author": "", "windowWidth": 1280, "windowHeight": 720, "latestCompilationDirectory": "",
            "maxFPS": 60, "minFPS": 20, "verticalSync": false,
            "loadingScreen": loading,
            "watermark": {"placement": "bottom-left", "showWatermark": true},
            "authorIds": [], "authorUsernames": [], "categories": [], "playableDevices": [], "extensionProperties": [],
            "platforms": [{"name": "GDevelop JS platform"}], "currentPlatform": "GDevelop JS platform",
    });
    json!({
        "firstLayout": level,
        "gdVersion": {"build": 284, "major": 5, "minor": 6, "revision": 0},
        "properties": properties,
        "resources": {"resources": []},
        "objects": [], "objectsFolderStructure": {"folderName": "__ROOT"}, "objectsGroups": [], "variables": [],
        "layouts": [], "externalEvents": [], "eventsFunctionsExtensions": [], "externalLayouts": [],
    })
}

/// Writes the level's scene into `existing` (a project's `game.json`), or into a new
/// project when there is none. Returns the files to write beside the meshes.
pub fn write(level: &Level, collision: &[CollisionMesh], options: &GDevelopOptions, meshes: &[GDevelopMesh], existing: Option<&str>) -> Result<Vec<GDevelopFile>, String> {
    let s = PIXELS_PER_METRE;
    let mut project = match existing {
        Some(text) => serde_json::from_str::<Json>(text).map_err(|why| format!("game.json is not a GDevelop project: {why}"))?,
        None => new_project(&level.name),
    };
    if !project["layouts"].is_array() || !project["resources"]["resources"].is_array() {
        return Err("game.json is not a GDevelop project: it has no layouts or resources".into());
    }
    let prefix = format!("{}/", level.name);
    let mut files = Vec::new();
    let mut resources: Vec<Json> = Vec::new();
    let mut objects: Vec<Json> = Vec::new();
    let mut instances: Vec<Json> = Vec::new();
    let mut runtime_entities = Vec::new();
    let mut hidden = Vec::new();
    let mut used = std::collections::HashSet::new();
    let mut unique = |name: String| {
        let mut candidate = name.clone();
        let mut n = 1;
        while !used.insert(candidate.clone()) {
            n += 1;
            candidate = format!("{name}_{n}");
        }
        candidate
    };
    let point = unique("HeronPoint".into());
    let mut points = false;

    for (index, entity) in level.entities.iter().enumerate() {
        let t = entity.transform.translation;
        let q = quaternion(entity);
        let (angle, rotation_y, rotation_x) = gdevelop_rotation(q);
        let scale = entity.transform.scale;
        let mut variables = vec![
            json!({"name": "heron_name", "type": "string", "value": entity.name}),
            json!({"name": "heron_classname", "type": "string", "value": entity.classname}),
            strings("heron_targets", &entity.targets),
            strings("heron_tags", &entity.tags),
        ];
        variables.extend(entity.properties.iter().map(|(k, v)| variable(k, v)));
        let instance = |object: &str, layer_z: i64| {
            let mut i = json!({
                "angle": angle, "customSize": false, "height": 0, "width": 0, "layer": "", "locked": false,
                "name": object, "persistentUuid": uuid(&format!("{}/{}/{object}", level.name, entity.name)),
                "x": t[0] * s, "y": t[2] * s, "z": t[1] * s, "zOrder": layer_z,
                "rotationX": rotation_x, "rotationY": rotation_y,
                "numberProperties": [], "stringProperties": [], "initialVariables": [],
            });
            i["initialVariables"] = json!(variables);
            i
        };
        let mut entity_objects = Vec::new();
        let mesh = entity.mesh.as_deref().and_then(|m| meshes.iter().find(|x| m.ends_with(&format!("/{}", x.file_name)) || m == x.file_name));
        let shape = collision.get(index);
        let sensor = entity.collision.as_ref().is_some_and(|c| c.sensor);
        let mover = entity.collision.as_ref().is_some_and(|c| c.kind == crate::level::CollisionShape::Convex);
        if let Some(mesh) = mesh.filter(|_| !entity.surfaces.is_empty()) {
            let (lo, hi) = bounds_with_origin(mesh.glb).ok_or_else(|| format!("{}: unreadable mesh", entity.name))?;
            let size = [(hi[0] - lo[0]) * s * scale[0].abs(), (hi[2] - lo[2]) * s * scale[2].abs(), (hi[1] - lo[1]) * s * scale[1].abs()];
            let name = unique(identifier(&entity.name));
            let resource_name = format!("{}{}", prefix, mesh.file_name);
            let mut behaviors = Vec::new();
            if mover && !sensor {
                if let Some(boxes) = shape.filter(|c| !c.boxes.is_empty()).map(|c| &c.boxes) {
                    let (mut a, mut b) = ([f32::MAX; 3], [f32::MIN; 3]);
                    for [l, h] in boxes {
                        for i in 0..3 {
                            a[i] = a[i].min(l[i]);
                            b[i] = b[i].max(h[i]);
                        }
                    }
                    // A box body sits at the object's centre; offset it to the hull's.
                    let centre = |i: usize| (lo[i] + hi[i]) / 2.;
                    let gd = |v: [f64; 3]| [v[0], v[2], v[1]];
                    let extent = gd([0, 1, 2].map(|i| f64::from(b[i] - a[i]) * s * scale[i].abs()));
                    let offset = gd([0, 1, 2].map(|i| ((f64::from(a[i] + b[i])) / 2. - centre(i)) * s * scale[i]));
                    behaviors.push(physics("Kinematic", "Box", extent, offset, ""));
                }
            }
            let mut object = model_object(&name, &resource_name, size, behaviors, Vec::new());
            // A see-through water surface would shade the floor under it.
            if entity.properties.contains_key("_heron_water") {
                object["content"]["isCastingShadow"] = json!(false);
            }
            objects.push(object);
            resources.push(resource(&resource_name));
            instances.push(instance(&name, index as i64));
            entity_objects.push(name);
        }
        // Static geometry collides through a triangle mesh of its own, so collision the
        // eye does not see (clip brushes) still blocks, and drawn faces marked as not
        // solid do not.
        if let Some(triangles) = shape.map(|c| &c.triangles).filter(|t| !t.is_empty() && !sensor && !mover) {
            let name = unique(format!("{}_collision", identifier(&entity.name)));
            let file = format!("{prefix}{}_collision.glb", identifier(&entity.name));
            // The model's fit mirrors GDevelop's Y; Jolt's triangles are one-sided, so
            // they are wound the other way round to keep facing out of the solid.
            let wound: Vec<[f32; 3]> = triangles.chunks_exact(3).flat_map(|t| [t[0], t[2], t[1]]).collect();
            let glb = collision_glb(&wound);
            let (lo, hi) = bounds_with_origin(&glb).unwrap();
            let size = [(hi[0] - lo[0]) * s * scale[0].abs(), (hi[2] - lo[2]) * s * scale[2].abs(), (hi[1] - lo[1]) * s * scale[1].abs()];
            files.push(GDevelopFile { path: file.clone(), contents: glb });
            objects.push(model_object(&name, &file, size, vec![physics("Static", "Mesh", [0.; 3], [0.; 3], "")], Vec::new()));
            resources.push(resource(&file));
            instances.push(instance(&name, index as i64));
            hidden.push(name.clone());
            entity_objects.push(name);
        }
        if entity_objects.is_empty() {
            points = true;
            instances.push(instance(&point, index as i64));
        }
        let property = |key: &str| match entity.properties.get(key) {
            Some(Value::String(text)) => Some(text.clone()),
            _ => None,
        };
        let animation = property("_heron_animation");
        let trigger = property("_heron_trigger");
        let water = property("_heron_water");
        if animation.is_some() || trigger.is_some() || water.is_some() {
            runtime_entities.push(json!({
                "name": entity.name, "objects": entity_objects, "position": t, "rotation": q, "scale": scale,
                "animation": animation, "trigger": trigger, "water": water,
            }));
        }
    }
    if points {
        objects.insert(0, point_object(&point));
    }

    // Lights and sky are layer effects. The sun is a directional light; the sky lights
    // standard materials through the skybox as GDevelop's environment.
    let mut effects = Vec::new();
    if let Some(sky) = &options.sky {
        let to_sun = sky.to_sun();
        // In GDevelop's axes the sun is (x, z, y) of the level's; its rotation is the
        // compass angle in the ground plane, its elevation the angle above it.
        let (x, y, z) = (f64::from(to_sun[0]), f64::from(to_sun[2]), f64::from(to_sun[1]));
        effects.push(effect("Scene3D::DirectionalLight", "Heron sun",
            json!({"intensity": sky.sun_intensity, "elevation": z.asin().to_degrees(), "rotation": y.atan2(x).to_degrees(),
                   "shadowDistance": 3000, "distanceFromCamera": 1500, "frustumSize": 4000, "minimumShadowBias": 0.0008}),
            json!({"color": rgb(sky.sun_color), "top": "Z+", "shadowQuality": "high", "shadowRenderingMode": "fittedToCameraView"}),
            json!({"isCastingShadow": true})));
        // GDevelop turns the cube from three.js's Y up to its Z up, which lands the
        // level's +Z on its -Y: the faces are written mirrored in Z to meet it.
        let faces = sky.png_faces(SKY_SIZE, |face, u, v| {
            let mut d = super::sky::cube_direction(face, u, v);
            d[2] = -d[2];
            d
        });
        let names = ["right", "left", "top", "bottom", "front", "back"];
        let mut parameters = json!({"top": "Z+"});
        for (face, png) in names.iter().zip(faces) {
            let file = format!("{prefix}sky_{face}.png");
            files.push(GDevelopFile { path: file.clone(), contents: png });
            resources.push(resource(&file));
            parameters[format!("{face}FaceResourceName")] = json!(file);
        }
        effects.push(effect("Scene3D::Skybox", "Heron sky", json!({"environmentIntensity": sky.ambient_intensity, "backgroundIntensity": 1}), parameters, json!({})));
        if let Some((color, density)) = sky.fog {
            effects.push(effect("Scene3D::ExponentialFog", "Heron fog", json!({"density": f64::from(density) / s}), json!({"color": rgb(color)}), json!({})));
        }
        // The 3D view tone maps with AgX after scaling by 1.8 (render_wgpu post.wgsl).
        effects.push(effect("Scene3D::ToneMapping", "Heron tone mapping", json!({"exposure": f64::from(sky.exposure) * 1.8}), json!({"mode": "AgX"}), json!({})));
    } else {
        effects.push(effect("Scene3D::HemisphereLight", "Heron ambient", json!({"elevation": 90, "intensity": 1, "rotation": 0}),
            json!({"skyColor": "255;255;255", "groundColor": "127;127;127", "top": "Z+"}), json!({})));
    }

    // Start the camera where the player starts, looking the way the start faces.
    let camera = level.entities.iter().find(|e| e.classname == "info_player_start").map(|e| {
        let t = e.transform.translation;
        let forward = glam::DQuat::from_array(quaternion(e)) * glam::DVec3::NEG_Z;
        json!({"x": t[0] * s, "y": t[2] * s, "z": (t[1] + 1.6) * s, "rotationX": 90, "angle": forward.z.atan2(forward.x).to_degrees() + 90.})
    });
    let data = json!({"level": level.name, "pixelsPerMetre": s, "entities": runtime_entities, "hidden": hidden, "camera": camera});
    let script = include_str!("gdevelop_runtime.js").replace("__DATA__", &serde_json::to_string(&data).unwrap());
    let events = json!([{
        "type": "BuiltinCommonInstructions::JsCode",
        "inlineCode": script.lines().collect::<Vec<_>>(),
        "parameterObjects": "", "useStrict": true, "eventsSheetExpanded": false,
    }]);

    let ui = json!({"grid": false, "gridType": "rectangular", "gridWidth": 32, "gridHeight": 32, "gridDepth": 32,
            "gridOffsetX": 0, "gridOffsetY": 0, "gridOffsetZ": 0, "gridColor": 10401023, "gridAlpha": 0.8, "snap": false,
            "zoomFactor": 1, "windowMask": false, "selectedLayer": ""});
    let layer = json!({
            "ambientLightColorB": 200, "ambientLightColorG": 200, "ambientLightColorR": 200,
            "camera2DPlaneMaxDrawingDistance": 5000, "camera3DFarPlaneDistance": 100000, "camera3DFieldOfView": 60,
            "camera3DNearPlaneDistance": 3, "cameraType": "perspective", "followBaseLayerCamera": false,
            "isLightingLayer": false, "isLocked": false, "name": "", "renderingType": "3d", "visibility": true,
            "cameras": [{"defaultSize": true, "defaultViewport": true, "height": 0, "viewportBottom": 1, "viewportLeft": 0, "viewportRight": 1, "viewportTop": 0, "width": 0}],
            "effects": effects,
        });
    let layout = json!({
        "b": 230, "r": 160, "v": 200, "disableInputWhenNotFocused": true, "mangledName": identifier(&level.name),
        "name": level.name, "standardSortMethod": true, "stopSoundsOnStartup": true, "title": "", "renderer3DWorldScale": 100,
        "uiSettings": ui,
        "objectsGroups": [], "variables": [], "instances": instances,
        "objectsFolderStructure": {"folderName": "__ROOT", "children": objects.iter().map(|o| json!({"objectName": o["name"]})).collect::<Vec<_>>()},
        "objects": objects, "events": events,
        "layers": [layer],
        "behaviorsSharedData": [{"name": "Physics3D", "type": "Physics3D::Physics3DBehavior", "gravityX": 0, "gravityY": 0, "gravityZ": -9.8, "worldScale": s}],
    });

    // Replace this level's scene and resources; keep everything else in the project.
    let layouts = project["layouts"].as_array_mut().unwrap();
    match layouts.iter().position(|l| l["name"] == json!(level.name)) {
        Some(at) => layouts[at] = layout,
        None => layouts.push(layout),
    }
    let list = project["resources"]["resources"].as_array_mut().unwrap();
    list.retain(|r| !r["name"].as_str().is_some_and(|n| n.starts_with(&prefix)));
    list.extend(resources);
    files.push(GDevelopFile { path: "game.json".into(), contents: serde_json::to_vec_pretty(&project).unwrap() });
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Collision, CollisionShape, Transform};

    fn glb(min: [f32; 3], max: [f32; 3]) -> Vec<u8> {
        collision_glb(&[min, max, [min[0], max[1], min[2]]])
    }

    fn level() -> Level {
        Level { format: 2, generator: "test".into(), name: "arena".into(), scale: 32., textures: vec![], entities: vec![] }
    }

    fn project(files: &[GDevelopFile]) -> Json {
        serde_json::from_slice(&files.iter().find(|f| f.path == "game.json").unwrap().contents).unwrap()
    }

    #[test]
    fn a_mesh_is_sized_to_its_bounds_with_the_origin_so_it_stays_in_place() {
        let mut level = level();
        level.entities.push(Entity { name: "worldspawn".into(), classname: "worldspawn".into(), mesh: Some("arena/worldspawn.glb".into()),
            surfaces: vec![crate::level::Surface { primitive: 0, texture: "stone".into() }], ..Default::default() });
        let bytes = glb([2., 0., -5.], [4., 1., -3.]);
        let files = write(&level, &[], &GDevelopOptions { sky: None }, &[GDevelopMesh { file_name: "worldspawn.glb", glb: &bytes }], None).unwrap();
        let p = project(&files);
        assert_eq!(p["firstLayout"], "arena");
        let layout = &p["layouts"][0];
        let object = &layout["objects"][0];
        assert_eq!(object["type"], "Scene3D::Model3DObject");
        // x 0..4, z -5..0 (GDevelop y), y 0..1 (elevation), in pixels.
        assert_eq!([&object["content"]["width"], &object["content"]["height"], &object["content"]["depth"]], [&json!(400.), &json!(500.), &json!(100.)]);
        assert_eq!(object["content"]["rotationX"], 90);
        assert_eq!(layout["instances"][0]["x"], 0.);
        assert_eq!(p["resources"]["resources"][0]["file"], "arena/worldspawn.glb");
    }

    #[test]
    fn positions_and_rotations_take_gdevelops_axes() {
        // A quarter turn about the level's Y (up) is a turn about GDevelop's Z, its angle.
        let (angle, y, x) = gdevelop_rotation(glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2).to_array());
        assert!((angle.abs() - 90.).abs() < 1e-9 && y.abs() < 1e-9 && x.abs() < 1e-9, "{angle} {y} {x}");
        // A turn about the level's X stays a turn about X, mirrored by the axis swap.
        let (angle, y, x) = gdevelop_rotation(glam::DQuat::from_rotation_x(0.5).to_array());
        assert!(angle.abs() < 1e-9 && y.abs() < 1e-9 && (x + 0.5f64.to_degrees()).abs() < 1e-9, "{angle} {y} {x}");
        let mut level = level();
        let mut e = Entity { name: "lamp".into(), classname: "light".into(), ..Default::default() };
        e.transform = Transform { translation: [1., 2., 3.], ..Default::default() };
        e.properties.insert("charge".into(), Value::Int(7));
        e.properties.insert("armed".into(), Value::Bool(true));
        level.entities.push(e);
        let p = project(&write(&level, &[], &GDevelopOptions { sky: None }, &[], None).unwrap());
        let instance = &p["layouts"][0]["instances"][0];
        assert_eq!((&instance["x"], &instance["y"], &instance["z"]), (&json!(100.), &json!(300.), &json!(200.)));
        assert_eq!(instance["name"], "HeronPoint");
        let vars = instance["initialVariables"].as_array().unwrap();
        assert!(vars.contains(&json!({"name": "charge", "type": "number", "value": 7})));
        assert!(vars.contains(&json!({"name": "armed", "type": "boolean", "value": true})));
        assert!(vars.contains(&json!({"name": "heron_classname", "type": "string", "value": "light"})));
    }

    #[test]
    fn static_collision_gets_a_hidden_mesh_body_and_movers_a_kinematic_box() {
        let mut level = level();
        let surfaces = vec![crate::level::Surface { primitive: 0, texture: "stone".into() }];
        level.entities.push(Entity { name: "worldspawn".into(), classname: "worldspawn".into(), mesh: Some("arena/worldspawn.glb".into()), surfaces: surfaces.clone(),
            collision: Some(Collision { kind: CollisionShape::Trimesh, mesh: 1, parts: 1, sensor: false }), ..Default::default() });
        level.entities.push(Entity { name: "lift".into(), classname: "func_animate".into(), mesh: Some("arena/lift.glb".into()), surfaces,
            collision: Some(Collision { kind: CollisionShape::Convex, mesh: 1, parts: 1, sensor: false }), ..Default::default() });
        let world = glb([-1., -1., -1.], [1., 0., 1.]);
        let lift = glb([-1., -0.5, -1.], [1., 0.5, 1.]);
        let collision = vec![
            CollisionMesh { triangles: vec![[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.]], boxes: vec![], hulls: vec![] },
            CollisionMesh { triangles: vec![], boxes: vec![[[-1., -0.5, -1.], [1., 0.5, 1.]]], hulls: vec![] },
        ];
        let files = write(&level, &collision, &GDevelopOptions { sky: None }, &[GDevelopMesh { file_name: "worldspawn.glb", glb: &world }, GDevelopMesh { file_name: "lift.glb", glb: &lift }], None).unwrap();
        let p = project(&files);
        let objects = p["layouts"][0]["objects"].as_array().unwrap();
        let named = |n: &str| objects.iter().find(|o| o["name"] == n).unwrap();
        assert_eq!(named("worldspawn_collision")["behaviors"][0]["shape"], "Mesh");
        assert_eq!(named("worldspawn_collision")["behaviors"][0]["bodyType"], "Static");
        assert_eq!(named("lift")["behaviors"][0]["bodyType"], "Kinematic");
        assert_eq!(named("lift")["behaviors"][0]["shapeDimensionA"], 200.);
        assert!(files.iter().any(|f| f.path == "arena/worldspawn_collision.glb"));
        let script = p["layouts"][0]["events"][0]["inlineCode"].as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect::<Vec<_>>().join("\n");
        assert!(script.contains("\"hidden\":[\"worldspawn_collision\"]"), "{script}");
    }

    #[test]
    fn re_export_replaces_only_the_levels_scene_and_resources() {
        let mut level = level();
        level.entities.push(Entity { name: "start".into(), classname: "info_player_start".into(), ..Default::default() });
        let mut existing = new_project("menu");
        existing["layouts"] = json!([{"name": "menu", "objects": []}, {"name": "arena", "objects": [{"name": "stale"}]}]);
        existing["resources"]["resources"] = json!([{"name": "ui/logo.png", "file": "ui/logo.png", "kind": "image"}, {"name": "arena/old.glb", "file": "arena/old.glb", "kind": "model3D"}]);
        let sky = Sky { sky: [0.1, 0.4, 0.85], horizon: [0.7, 0.8, 0.9], ground: [0.2, 0.18, 0.15], sun_color: [1., 0.9, 0.8], sun_intensity: 3., ambient_intensity: 0.5, sun_elevation: 45., sun_azimuth: 0., exposure: 1., fog: None };
        let files = write(&level, &[], &GDevelopOptions { sky: Some(sky) }, &[], Some(&existing.to_string())).unwrap();
        let p = project(&files);
        assert_eq!(p["firstLayout"], "menu");
        assert_eq!(p["layouts"].as_array().unwrap().len(), 2);
        assert_eq!(p["layouts"][0]["name"], "menu");
        assert!(p["layouts"][1]["objects"].as_array().unwrap().iter().all(|o| o["name"] != "stale"));
        let names: Vec<_> = p["resources"]["resources"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap().to_string()).collect();
        assert!(names.contains(&"ui/logo.png".into()) && !names.contains(&"arena/old.glb".into()) && names.contains(&"arena/sky_top.png".into()), "{names:?}");
        let effects = p["layouts"][1]["layers"][0]["effects"].as_array().unwrap();
        assert!(effects.iter().any(|e| e["effectType"] == "Scene3D::Skybox") && effects.iter().any(|e| e["effectType"] == "Scene3D::DirectionalLight"));
        assert!(write(&level, &[], &GDevelopOptions { sky: None }, &[], Some("{\"not\": 1}")).is_err());
    }
}
