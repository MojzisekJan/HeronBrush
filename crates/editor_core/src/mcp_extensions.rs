//! Validation and headless access to the editor's export and plugin operations.
use super::*;
use crate::export::{Options, Target};
use crate::plugin_manifest::{Setting, SettingKind, SettingValue};

pub(super) fn node_id(value: &Value) -> Result<u64, String> {
    match value {
        Value::String(text) => text
            .parse::<u64>()
            .map_err(|_| "invalid decimal object ID".into()),
        Value::Number(number) => number
            .as_u64()
            .filter(|id| *id <= 9_007_199_254_740_991)
            .ok_or_else(|| "object IDs above 2^53-1 must be decimal strings".into()),
        _ => Err("object ID must be a decimal string".into()),
    }
}

/// Validate the subset of JSON Schema used by our tool definitions before mutations.
pub(super) fn validate(value: &Value, schema: &Value, path: &str) -> Result<(), String> {
    if let Some(variants) = schema.get("anyOf").and_then(Value::as_array) {
        if variants.iter().any(|s| validate(value, s, path).is_ok()) {
            return Ok(());
        }
        return Err(format!("{path}: value does not match any allowed type"));
    }
    if let Some(kind) = schema.get("type").and_then(Value::as_str) {
        let valid = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "integer" => value.is_u64() || value.is_i64(),
            "number" => value.as_f64().is_some_and(f64::is_finite),
            _ => false,
        };
        if !valid {
            return Err(format!("{path}: expected {kind}"));
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.contains(value) {
            return Err(format!("{path}: unsupported value {value}"));
        }
    }
    if let Some(number) = value.as_f64() {
        for (key, bad) in [("minimum", false), ("maximum", true)] {
            if let Some(bound) = schema.get(key).and_then(Value::as_f64) {
                if if bad { number > bound } else { number < bound } {
                    return Err(format!("{path}: outside allowed range"));
                }
            }
        }
        if schema
            .get("exclusiveMinimum")
            .and_then(Value::as_f64)
            .is_some_and(|min| number <= min)
        {
            return Err(format!("{path}: must be positive"));
        }
    }
    if let Some(text) = value.as_str() {
        if schema
            .get("minLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| text.len() < n as usize)
        {
            return Err(format!("{path}: must not be empty"));
        }
    }
    if let Some(items) = value.as_array() {
        if schema
            .get("minItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| items.len() < n as usize)
            || schema
                .get("maxItems")
                .and_then(Value::as_u64)
                .is_some_and(|n| items.len() > n as usize)
        {
            return Err(format!("{path}: wrong number of items"));
        }
        if let Some(item_schema) = schema.get("items") {
            for (i, item) in items.iter().enumerate() {
                validate(item, item_schema, &format!("{path}[{i}]"))?;
            }
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    return Err(format!("{path}: missing {key}"));
                }
            }
        }
        for (key, item) in object {
            if let Some(property) = schema.get("properties").and_then(|p| p.get(key)) {
                validate(item, property, &format!("{path}.{key}"))?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(format!("{path}: unknown argument {key}"));
            } else if let Some(additional) =
                schema.get("additionalProperties").filter(|v| v.is_object())
            {
                validate(item, additional, &format!("{path}.{key}"))?;
            }
        }
    }
    Ok(())
}

pub(super) fn with_warnings(mut text: String, warnings: &[String]) -> String {
    if !warnings.is_empty() {
        text.push_str("\nWarnings / follow-up results:\n");
        text.push_str(&warnings.join("\n"));
    }
    text
}

pub(super) fn reconnect(document: &mut Document, collab: &Collab) {
    if let Some(session) = &collab.session {
        let session = Rc::clone(session);
        document.set_on_edit(Rc::new(move |edit| session.send_edit(edit)));
    }
}

fn target(args: &Value) -> Result<Target, String> {
    match string_or(args, "engine", "").to_ascii_lowercase().as_str() {
        "godot" => Ok(Target::Godot),
        "defold" => Ok(Target::Defold),
        "three.js" | "threejs" | "three" => Ok(Target::ThreeJs),
        "bevy" => Ok(Target::Bevy),
        "raylib" => Ok(Target::Raylib),
        "unity" => Ok(Target::Unity),
        "unreal" | "ue" | "ue5" => Ok(Target::Unreal),
        "flax" => Ok(Target::Flax),
        "stride" | "stride3d" | "stride 3d" => Ok(Target::Stride),
        "cocos" | "cocos creator" | "cocoscreator" => Ok(Target::Cocos),
        "gdevelop" | "gdevelop5" | "gd" => Ok(Target::GDevelop),
        "obj" | "wavefront" => Ok(Target::Obj),
        "gltf" => Ok(Target::Gltf),
        "glb" => Ok(Target::Glb),
        other => Err(format!("unknown engine: {other}")),
    }
}

fn options(document: &Document, args: &Value) -> Result<Options, String> {
    let settings = crate::export_settings::Settings::for_map(document.path.as_deref());
    settings.validate()?;
    let mut options = settings.for_target(target(args)?);
    options.name = document.level_name();
    let mut value = serde_json::to_value(options).map_err(|e| e.to_string())?;
    if let Some(overrides) = args.get("options").and_then(Value::as_object) {
        for (key, v) in overrides {
            value[key] = v.clone();
        }
    }
    for key in ["name", "scale"] {
        if let Some(v) = args.get(key) {
            value[key] = v.clone();
        }
    }
    let options: Options = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err("scale must be positive".into());
    }
    if options.name.trim().is_empty()
        || options.name.contains(['/', '\\'])
        || matches!(options.name.as_str(), "." | "..")
    {
        return Err("name must be a nonempty file name, without directory separators".into());
    }
    Ok(options)
}

pub(super) fn export_settings(document: &Document, args: &Value) -> Result<String, String> {
    let mut value = serde_json::to_value(options(document, args)?).map_err(|e| e.to_string())?;
    value.as_object_mut().unwrap().remove("target");
    Ok(value.to_string())
}

pub(super) fn export(document: &Document, args: &Value, checking: bool) -> Result<String, String> {
    let options = options(document, args)?;
    let path = std::path::Path::new(string_or(args, "path", ""));
    let report = if checking {
        crate::export::check(document, path, &options)
    } else {
        crate::export::export(document, path, &options)
    }
    .map_err(|e| e.to_string())?;
    Ok(
        json!({"files": report.files, "warnings": report.warnings, "triangles": report.triangles,
        "saved_faces": report.saved_faces, "checked": checking})
        .to_string(),
    )
}

fn setting_schema(setting: &Setting) -> Value {
    let mut schema = match setting.kind {
        SettingKind::Number => json!({"type":"number", "default":setting.default_number()}),
        SettingKind::Check => json!({"type":"boolean", "default":setting.default_check()}),
        SettingKind::Text | SettingKind::Choice => {
            json!({"type":"string", "default":setting.default})
        }
    };
    schema["description"] = json!(setting.label);
    if setting.kind == SettingKind::Number {
        if let Some(min) = setting.min {
            schema["minimum"] = json!(min);
        }
        if let Some(max) = setting.max {
            schema["maximum"] = json!(max);
        }
    }
    if setting.kind == SettingKind::Choice && !setting.choices.is_empty() {
        schema["enum"] = json!(setting.choices);
    }
    schema
}

pub(super) fn list_plugins() -> Result<String, String> {
    Ok(Value::Array(crate::plugins::discover().iter().map(|p| {
        let settings: serde_json::Map<String, Value> = p.settings.iter().map(|s| (s.key.clone(), setting_schema(s))).collect();
        json!({"id":p.id,"name":p.name,"enabled":p.enabled,"description":p.summary,"version":p.version,"unsupported_api":p.wrong_api,"settings":settings,"panel_action":p.panel_action,"actions":p.actions.iter().map(|a|json!({"id":a.id,"label":a.label,"description":a.description})).collect::<Vec<_>>()})
    }).collect()).to_string())
}

pub(super) fn run_plugin(document: &mut Document, args: &Value) -> Result<String, String> {
    let (source, settings, wrong_api, actions) = match (args.get("name"), args.get("path")) {
        (Some(name), None) => {
            let mut matches: Vec<_> = crate::plugins::discover()
                .into_iter()
                .filter(|p| p.name == name.as_str().unwrap_or(""))
                .collect();
            if matches.len() != 1 {
                return Err(
                    "plugin name is missing or ambiguous; use an explicit script path".into(),
                );
            }
            let plugin = matches.remove(0);
            if !plugin.enabled {
                return Err("Plugin is disabled".into());
            }
            (
                plugin.source,
                plugin.settings,
                plugin.wrong_api,
                plugin.actions,
            )
        }
        (None, Some(path)) => {
            let mut path = std::path::PathBuf::from(path.as_str().ok_or("invalid path")?);
            if path.is_dir() {
                path.push("main.rhai");
            }
            crate::plugins::check_path_enabled(&path)?;
            let source =
                std::fs::read_to_string(&path).map_err(|e| format!("cannot read plugin: {e}"))?;
            let manifest = crate::plugin_manifest::Manifest::beside(&path).unwrap_or_default();
            let wrong = if manifest.api_matches() {
                None
            } else {
                manifest.api
            };
            (source, manifest.settings, wrong, manifest.actions)
        }
        _ => return Err("provide exactly one of name or path".into()),
    };
    if let Some(api) = wrong_api {
        return Err(format!("plugin requires unsupported API {api}"));
    }
    let overrides = args.get("settings").cloned().unwrap_or_else(|| json!({}));
    let properties: serde_json::Map<String, Value> = settings
        .iter()
        .map(|s| (s.key.clone(), setting_schema(s)))
        .collect();
    validate(
        &overrides,
        &json!({"type":"object","properties":properties,"additionalProperties":false}),
        "settings",
    )?;
    let mut values = Vec::new();
    for setting in &settings {
        let schema = setting_schema(setting);
        let value = overrides.get(&setting.key).unwrap_or(&schema["default"]);
        validate(value, &schema, &setting.key)?;
        let value = match setting.kind {
            SettingKind::Number => SettingValue::Number(value.as_f64().unwrap()),
            SettingKind::Check => SettingValue::Check(value.as_bool().unwrap()),
            SettingKind::Text | SettingKind::Choice => {
                SettingValue::Text(value.as_str().unwrap().to_string())
            }
        };
        values.push((setting.key.clone(), value));
    }
    let action = args.get("action").and_then(Value::as_str).unwrap_or("");
    if !action.is_empty() && !actions.iter().any(|a| a.id == action) {
        return Err("Unknown plugin action".into());
    }
    // Same undo semantics as the Plugins window, including partial failed runs.
    let result =
        document.run_as_one_edit(|doc| crate::plugin::run_action(doc, &source, &values, action));
    result
        .map(|output| json!({"messages":output.lines,"panel":output.panel}).to_string())
        .map_err(|e| {
            format!("plugin failed: {e}; any completed edits are grouped into one undo step")
        })
}

fn export_properties() -> Value {
    let mut properties = json!({
        "entities":{"type":"object","description":"Project class bindings: build directives and typed default properties keyed by classname."},
        "scale":{"type":"number","exclusiveMinimum":0},
        "lightmap_texel_size":{"type":"number","exclusiveMinimum":0},
        "trigger_layer":{"type":"integer","minimum":0,"maximum":32},
    });
    for key in [
        "collision",
        "sky",
        "cull_hidden",
        "merge_faces",
        "static_cells",
        "merge_collision",
        "lightmap_uv2",
        "group_hierarchy",
        "flip_faces",
        "flip_uv_v",
        "export_on_save",
    ] {
        properties[key] = json!({"type":"boolean"});
    }
    for key in [
        "name",
        "texture_dir",
        "material_dir",
        "clip_texture",
        "skip_texture",
        "origin_texture",
        "asset_dir",
        "content_dir",
        "model_material",
        "model_sampler",
        "directory",
        "project_dir",
        "name_property",
    ] {
        properties[key] = json!({"type":"string"});
    }
    properties
}

pub(super) fn definitions() -> Vec<Value> {
    vec![
        tool("get_export_settings", "Read effective export settings. Per-call overrides are not persisted.", json!({"engine":{"type":"string"}}), &["engine"]),
        tool("check_export", "Validate exported geometry without writing files. Accepts the same options as export.", json!({"engine":{"type":"string"},"path":{"type":"string","minLength":1},"name":{"type":"string"},"scale":{"type":"number","exclusiveMinimum":0}}), &["engine","path"]),
        tool("list_plugins", "List bundled and installed plugins, activation state, action IDs and setting schemas.", json!({}), &[]),
        tool("run_plugin", "Run a plugin on the current selection, as one undo step. Provide name from list_plugins or script/folder path, and optional settings and action ID. Disabled plugins cannot run.", json!({"name":{"type":"string"},"path":{"type":"string"},"settings":{"type":"object"},"action":{"type":"string"}}), &[]),
        tool("duplicate_selection", "Duplicate selected objects, optionally moving the copies by delta.", json!({"delta":vec3_schema("offset of the copies")}), &[]),
    ]
}

pub(super) fn tighten_schemas(tools: &mut [Value]) {
    let id = json!({"anyOf":[{"type":"string","minLength":1},{"type":"integer","minimum":0,"maximum":9_007_199_254_740_991u64}],"description":"decimal object ID string"});
    for tool in tools {
        let name = tool["name"].as_str().unwrap().to_owned();
        let props = &mut tool["inputSchema"]["properties"];
        match name.as_str() {
            "select" => {
                props["ids"]["items"] = id.clone();
            }
            "describe_brush"
            | "set_face_color"
            | "set_surface_material"
            | "set_face_export_hidden" => {
                props["brush"] = id.clone();
            }
            _ => {}
        }
        if name == "set_face_color" {
            props["face"] = json!({"type":"integer","minimum":0});
            for key in ["alpha", "opacity"] {
                props[key] = json!({"type":"number","minimum":0,"maximum":255});
            }
            props["color"]["items"] = json!({"type":"number","minimum":0,"maximum":255});
            props["gradient"]["anyOf"] =
                json!([{"required":["from", "to"]}, {"required":["stops"]}]);
            props["gradient"]["additionalProperties"] = json!(false);
            for key in ["from", "to"] {
                props["gradient"]["properties"][key]["items"] =
                    json!({"type":"number","minimum":0,"maximum":255});
            }
        }
        if name == "add_point_entity" {
            props["properties"]["additionalProperties"] = json!({"type":"string"});
        }
        if name == "add_shape" {
            props["sides"] = json!({"type":"integer","minimum":3,"maximum":128});
            props["step_height"] = json!({"type":"number","exclusiveMinimum":0});
            props["thickness"] = json!({"type":"number","exclusiveMinimum":0});
        }
        if name == "hollow" {
            props["thickness"] = json!({"type":"number","exclusiveMinimum":0});
        }
        if name == "render_preview" {
            props["size"] = json!({"type":"integer","minimum":16,"maximum":2048});
        }
        if name == "host_session" {
            props["port"] = json!({"type":"integer","minimum":0,"maximum":65535});
        }
        if name == "export" || name == "check_export" {
            props["scale"] = json!({"type":"number","exclusiveMinimum":0});
            props["options"] = json!({"type":"object","properties":export_properties(),"additionalProperties":false});
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        let mut document = Document::new(WorldBounds::default());
        call_tool(
            &mut document,
            &mut Collab::default(),
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[64,32,16]}),
        )
        .unwrap();
        document
    }
    fn call(document: &mut Document, name: &str, args: Value) -> Result<String, String> {
        call_tool(document, &mut Collab::default(), name, &args)
    }
    fn temp(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "heron-mcp-{label}-{}-{}",
            std::process::id(),
            web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn string_ids_round_trip_and_bad_selection_is_atomic() {
        let mut document = doc();
        let id = brush_list(&document)[0]["id"].clone();
        assert!(id.is_string());
        let described: Value = serde_json::from_str(
            &call(&mut document, "describe_brush", json!({"brush":id})).unwrap(),
        )
        .unwrap();
        assert_eq!(described["id"], id);
        assert!(node_id(&json!(u64::MAX)).is_err());
        assert_eq!(node_id(&json!(u64::MAX.to_string())).unwrap(), u64::MAX);
        call(&mut document, "select", json!({"ids":[id.clone(),id]})).unwrap();
        assert_eq!(document.selection_count(), 1);
        assert!(call(&mut document, "select", json!({"ids":["0"]})).is_err());
        assert_eq!(document.selection_count(), 1);
    }

    #[test]
    fn nonunit_rotation_preserves_dimensions_and_bad_arguments_do_not_edit() {
        let mut document = doc();
        for args in [
            json!({"axis":[0,0,1],"degrees":"oops"}),
            json!({"axis":[0,0,1]}),
            json!({"axis":[0,0,0],"degrees":90}),
        ] {
            let before = document.to_map_text();
            assert!(call(&mut document, "rotate_selection", args).is_err());
            assert_eq!(before, document.to_map_text());
        }
        call(
            &mut document,
            "rotate_selection",
            json!({"axis":[0,0,2],"degrees":90}),
        )
        .unwrap();
        let brush = &brush_list(&document)[0];
        for (i, wanted) in [32., 64., 16.].iter().enumerate() {
            let actual = brush["max"][i].as_f64().unwrap() - brush["min"][i].as_f64().unwrap();
            assert!((actual - wanted).abs() < 1e-6);
        }
        assert!(call(&mut document, "scale_selection", json!({"factors":[1,0,1]})).is_err());
        assert!(call(
            &mut document,
            "render_preview",
            json!({"path":"unused.png","size":-1})
        )
        .is_err());
    }

    #[test]
    fn targeted_clear_only_changes_that_face() {
        let mut document = doc();
        call(
            &mut document,
            "set_face_color",
            json!({"color":[200,30,40]}),
        )
        .unwrap();
        let id = brush_list(&document)[0]["id"].clone();
        call(
            &mut document,
            "set_face_color",
            json!({"brush":id,"face":0}),
        )
        .unwrap();
        let value: Value = serde_json::from_str(
            &call(&mut document, "describe_brush", json!({"brush":id})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            value["faces"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["paint"].is_null())
                .count(),
            1
        );
    }

    #[test]
    fn notifications_never_execute_mutating_tools() {
        let mut document = doc();
        let before = document.to_map_text();
        assert!(respond(
            &mut document,
            &mut Collab::default(),
            r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"new_map","arguments":{}}}"#,
            true
        )
        .is_none());
        assert_eq!(before, document.to_map_text());
        let invalid = respond(&mut document, &mut Collab::default(), "[]", true).unwrap();
        assert_eq!(invalid["error"]["code"], -32600);
    }

    #[test]
    fn map_replacement_keeps_listener_and_changes_revision() {
        let mut document = doc();
        let events = Rc::new(std::cell::Cell::new(0));
        let shared = Rc::clone(&events);
        document.set_on_edit(Rc::new(move |_| shared.set(shared.get() + 1)));
        let redraws = Rc::new(std::cell::Cell::new(0));
        let changed = Rc::clone(&redraws);
        document.set_on_changed(Rc::new(move || changed.set(changed.get() + 1)));
        let revision = document.revision();
        call(&mut document, "new_map", json!({})).unwrap();
        assert_eq!(redraws.get(), 1);
        assert_ne!(revision, document.revision());
        call(
            &mut document,
            "add_shape",
            json!({"kind":"cuboid","min":[0,0,0],"max":[1,1,1]}),
        )
        .unwrap();
        assert_eq!(events.get(), 1);
        let refused=respond(&mut document,&mut Collab::default(),r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"new_map","arguments":{}}}"#,false).unwrap();
        assert_eq!(refused["result"]["isError"], true);
    }

    #[test]
    fn save_sets_document_state_and_obj_creates_directory() {
        let directory = temp("save");
        let mut document = doc();
        let map = directory.join("saved.map");
        assert!(document.is_modified());
        call(&mut document, "save", json!({"path":map})).unwrap();
        assert!(!document.is_modified());
        assert_eq!(document.level_name(), "saved");
        let out = directory.join("new-output");
        let result = call(
            &mut document,
            "export",
            json!({"engine":"obj","path":out,"name":"probe","options":{"scale":32.0}}),
        )
        .unwrap();
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["files"].as_array().unwrap().len(), 2);
        assert!(out.join("probe.obj").is_file());
        let check_dir = directory.join("must-not-exist");
        let checked: Value = serde_json::from_str(
            &call(
                &mut document,
                "check_export",
                json!({"engine":"obj","path":check_dir}),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(checked["warnings"]
            .as_array()
            .is_some_and(|w| !w.is_empty()));
        assert!(!check_dir.exists());
        assert!(call(
            &mut document,
            "export",
            json!({"engine":"obj","path":out,"options":{"collision":"yes"}})
        )
        .is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn bundled_plugin_and_settings_use_one_undo_step() {
        let mut document = doc();
        let before = document.to_map_text();
        let list: Value = serde_json::from_str(&list_plugins().unwrap()).unwrap();
        assert!(list
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "checker"));
        call(&mut document, "run_plugin", json!({"name":"checker"})).unwrap();
        assert_ne!(before, document.to_map_text());
        document.undo().unwrap();
        assert_eq!(before, document.to_map_text());
        assert!(call(
            &mut document,
            "run_plugin",
            json!({"name":"checker","settings":{"typo":1}})
        )
        .is_err());
        assert_eq!(before, document.to_map_text());
        let directory = temp("plugin");
        std::fs::write(
            directory.join("plugin.toml"),
            "[[setting]]\nkey = \"copies\"\nkind = \"number\"\ndefault = 2\nmin = 1\nmax = 3\n",
        )
        .unwrap();
        std::fs::write(
            directory.join("main.rhai"),
            "say(\"copies=\" + setting(\"copies\"));",
        )
        .unwrap();
        assert!(call(
            &mut document,
            "run_plugin",
            json!({"path":directory,"settings":{"copies":4}})
        )
        .is_err());
        assert!(call(
            &mut document,
            "run_plugin",
            json!({"path":directory,"settings":{"copies":3}})
        )
        .unwrap()
        .contains("copies=3"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod export_schema_tests {
    #[test]
    fn engine_material_and_content_options_are_exposed() {
        let properties = super::export_properties();
        for key in ["content_dir", "model_material", "model_sampler"] {
            assert_eq!(properties[key]["type"], "string");
        }
    }
}
