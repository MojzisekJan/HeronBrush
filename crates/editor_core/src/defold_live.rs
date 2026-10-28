//! Defold project refresh and optional runtime hot reload through its local editor API.
use crate::{export::Options, godot_live::Prepared};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

pub const FOLDER: &str = "heron_brush_live";
// A fixed preview path lets game projects reference the collection once.
pub const NAME: &str = "preview";

pub fn project(options: &Options) -> Result<PathBuf, String> {
    let directory = Path::new(&options.directory)
        .canonicalize()
        .map_err(|_| "Export into a Defold project before starting the live link.")?;
    let root = if options.project_dir.is_empty() {
        directory
            .ancestors()
            .find(|p| p.join("game.project").is_file())
            .ok_or("The export folder must be inside a Defold project.")?
            .to_path_buf()
    } else {
        Path::new(&options.project_dir)
            .canonicalize()
            .map_err(|e| e.to_string())?
    };
    if !root.join("game.project").is_file() || !directory.starts_with(&root) {
        return Err("Choose an export folder inside the Defold project.".into());
    }
    Ok(root)
}

/// Reserve a dedicated directory; never adopt an existing user-owned scene folder.
pub fn claim_folder(root: &Path) -> Result<(), String> {
    let folder = root.join(FOLDER);
    let marker = folder.join(".heron-brush-live");
    if folder.exists() {
        if folder.canonicalize().map_err(|e| e.to_string())? != folder || !marker.is_file() {
            return Err("The heron_brush_live folder already exists and is not owned by this live link. Move it aside first.".into());
        }
    } else {
        std::fs::create_dir(&folder).map_err(|e| e.to_string())?;
        crate::godot_live::atomic_write(&marker, b"Heron Brush live preview v1\n")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn prepare(
    text: &str,
    classes: &scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
    settings: &scene_export::settings::Settings,
    options: &Options,
    root: &Path,
) -> Result<Prepared, String> {
    let map = scene_export::map::parse(text, &options.name).map_err(|e| e.to_string())?;
    let classes = crate::export_bindings::classes(&map, classes, &options.entities)?;
    let built = scene_export::builder::build_level(
        &map,
        &classes,
        settings,
        &scene_export::builder::LevelOptions {
            name: options.name.clone(),
            mesh_dir: options.name.clone(),
            collision: options.collision,
            inline_collision: true,
            mesh_collision: false,
            convex_collision: true,
            mesh_format: scene_export::builder::MeshFormat::Glb,
        },
    );
    scene_export::writer::defold::validate(&built.level)?;
    let directory = Path::new(&options.directory);
    let relative = directory
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    let mut defold = scene_export::writer::defold::DefoldOptions {
        asset_dir: format!("/{relative}/{}", options.name),
        ..Default::default()
    };
    if !options.model_material.is_empty() {
        defold.model_material = options.model_material.clone();
    }
    if !options.model_sampler.is_empty() {
        defold.model_sampler = options.model_sampler.clone();
    }
    let mut files = crate::export::configure_defold_materials(
        &built.level,
        settings,
        directory,
        &options.name,
        root,
        &mut defold,
    )
    .map_err(|e| e.to_string())?;
    scene_export::writer::defold::validate_material_bindings(&built.level, &defold)?;
    let mut meshes = Vec::new();
    for mesh in built.meshes {
        let path = directory.join(&options.name).join(mesh.file_name);
        meshes.push(
            path.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/"),
        );
        files.push((path, mesh.bytes));
    }
    for file in scene_export::writer::defold::write(&built.level, &built.collision, &defold) {
        files.push((directory.join(file.path), file.contents.into_bytes()));
    }
    Ok(Prepared {
        files,
        meshes,
        scene: format!("{relative}/{}.collection", options.name),
        warnings: built.warnings,
    })
}

/// Runs on a worker. No executable, shell, remote host or arbitrary Lua from the map is used.
pub fn refresh(root: &Path, hot_reload: bool) -> Result<String, String> {
    let editor = Editor::discover(root)?;
    let (_, schema) = editor.request("GET", "/openapi.json", "")?;
    let schema: serde_json::Value = serde_json::from_slice(&schema).map_err(|e| e.to_string())?;
    if schema.pointer("/paths/~1eval/post").is_none() {
        return Err(
            "This Defold version does not expose the editor scripting API. Update Defold.".into(),
        );
    }
    // Creating the existing directory triggers Defold's resource synchronization. It
    // does not save open documents or alter their text. Verify the collection is known
    // to the editor after synchronization, rather than equating HTTP 202 with success.
    let (status, reply) = editor.request("POST", "/eval",
        "editor.create_directory(\"/heron_brush_live\")\nreturn editor.can_get(\"/heron_brush_live/preview.collection\", \"path\")")?;
    if status != 200
        || !String::from_utf8_lossy(&reply)
            .lines()
            .any(|l| l.trim() == "=> true")
    {
        return Err("Defold has not confirmed loading the live collection.".into());
    }
    if !hot_reload {
        return Ok(
            "Defold updated — open heron_brush_live/preview.collection to see the map.".into(),
        );
    }
    if !supports_command(&schema, "hot-reload") {
        return Err(
            "Defold project updated, but this editor version does not expose hot reload.".into(),
        );
    }
    let (status, reply) = editor.request("POST", "/command/hot-reload", "")
        .map_err(|e| format!("Project updated, but hot reload failed: {e} Start the game from Defold, or turn off hot reload."))?;
    reload_result(status, &reply)
}

fn supports_command(schema: &serde_json::Value, command: &str) -> bool {
    schema["paths"][format!("/command/{command}")]["post"].is_object()
        || schema["paths"]["/command/{command}"]["post"]["parameters"]
            .as_array()
            .is_some_and(|parameters| {
                parameters.iter().any(|p| {
                    p["name"] == "command"
                        && p["schema"]["enum"]
                            .as_array()
                            .is_some_and(|values| values.iter().any(|v| v == command))
                })
            })
}
fn reload_result(status: u16, body: &[u8]) -> Result<String, String> {
    let json = serde_json::from_slice::<serde_json::Value>(body).unwrap_or_default();
    if json["success"] == false {
        return Err(format!(
            "Defold hot reload failed: {}",
            String::from_utf8_lossy(body)
        ));
    }
    match status {
        200 if json["success"] == true => Ok("Defold updated — hot reload completed. Structural and collision changes may require restarting the level.".into()),
        200 | 202 => Ok("Defold project updated; hot reload requested. This Defold version does not confirm runtime completion.".into()),
        _ => Err(format!("Defold hot reload failed (HTTP {status}). Start the game from Defold first.")),
    }
}

struct Editor {
    port: u16,
    token: String,
}
impl Editor {
    fn discover(root: &Path) -> Result<Self, String> {
        let internal = root.join(".internal");
        let port = std::fs::read_to_string(internal.join("editor.port"))
            .map_err(|_| "Open this project in Defold first.")?
            .trim()
            .parse::<u16>()
            .map_err(|_| "Defold's editor port is invalid.")?;
        let token = std::fs::read_to_string(internal.join("editor.token"))
            .map_err(|_| {
                "Defold's editor API token is missing. Open the project in a recent Defold version."
            })?
            .trim()
            .to_owned();
        if port == 0 || token.is_empty() || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("Defold's editor connection details are invalid.".into());
        }
        Ok(Self { port, token })
    }
    fn request(&self, method: &str, path: &str, body: &str) -> Result<(u16, Vec<u8>), String> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .map_err(|_| "Could not connect to Defold. Open the target project and retry.")?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        // HTTP/1.0 asks for a close-delimited response; Defold's local server honors it.
        write!(stream, "{method} {path} HTTP/1.0\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", self.port, self.token, body.len()).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        stream
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Defold request did not complete: {e}"))?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Defold response exceeded 2 MiB.".into());
        }
        let result = parse_response(&bytes)?;
        if result.0 >= 400 {
            return Err(format!(
                "Defold request failed (HTTP {}): {}",
                result.0,
                String::from_utf8_lossy(&result.1)
            ));
        }
        Ok(result)
    }
}
fn parse_response(bytes: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let end = bytes
        .windows(4)
        .position(|s| s == b"\r\n\r\n")
        .ok_or("Invalid Defold HTTP response.")?;
    let header = std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())?;
    let status = header
        .lines()
        .next()
        .and_then(|s| s.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or("Invalid Defold HTTP status.")?;
    let body = &bytes[end + 4..];
    for line in header.lines().skip(1) {
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                let length = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "Invalid Defold response length.")?;
                if length != body.len() {
                    return Err("Incomplete Defold response.".into());
                }
            }
            if key.eq_ignore_ascii_case("transfer-encoding")
                && !value.trim().eq_ignore_ascii_case("identity")
            {
                return Err("Unsupported Defold HTTP response encoding.".into());
            }
        }
    }
    Ok((status, body.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::Document, export::Target, world::WorldBounds};
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    struct Temp(PathBuf);
    impl Temp {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("heron-defold-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn project_boundaries_and_user_owned_folders_are_preserved() {
        let root = Temp::new("paths");
        std::fs::write(root.0.join("game.project"), "[project]\n").unwrap();
        let mut options = Options {
            target: Target::Defold,
            directory: root.0.to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert_eq!(project(&options).unwrap(), root.0);
        options.project_dir = root.0.to_string_lossy().into_owned();
        options.directory = std::env::temp_dir().to_string_lossy().into_owned();
        assert!(project(&options).is_err());
        std::fs::create_dir(root.0.join(FOLDER)).unwrap();
        let user = root.0.join(FOLDER).join("preview.collection");
        std::fs::write(&user, "user scene").unwrap();
        assert!(claim_folder(&root.0).is_err());
        assert_eq!(std::fs::read_to_string(user).unwrap(), "user scene");
        let owned = Temp::new("owned");
        claim_folder(&owned.0).unwrap();
        claim_folder(&owned.0).unwrap();
    }
    #[test]
    fn edits_deletion_and_undo_replace_the_collection_and_keep_resource_paths_local() {
        let root = Temp::new("snapshots");
        claim_folder(&root.0).unwrap();
        let options = Options {
            target: Target::Defold,
            name: NAME.into(),
            directory: root.0.join(FOLDER).to_string_lossy().into_owned(),
            // A saved final-export override must never leak into the preview.
            asset_dir: "/main/final".into(),
            model_material: "/main/model.material".into(),
            model_sampler: "tex0".into(),
            collision: true,
            ..Default::default()
        };
        let settings = scene_export::settings::Settings {
            project_root: Some(root.0.clone()),
            ..Default::default()
        };
        let mut doc = Document::new(WorldBounds::default());
        doc.add_brushes(vec![
            map_model::shapes::create_cuboid(
                &doc.world_bounds(),
                &Bbox3d::from_scalars(-32.0, 32.0),
                "stone",
            )
            .unwrap(),
        ]);
        let mut snapshots = Vec::new();
        for step in 0..4 {
            doc.select_all();
            match step {
                1 => {
                    doc.translate_selection(Vec3d::new(64.0, 0.0, 0.0));
                }
                2 => {
                    doc.delete_selection();
                }
                3 => {
                    doc.undo();
                }
                _ => {}
            }
            let p = prepare(
                &doc.to_map_text(),
                &Default::default(),
                &settings,
                &options,
                &root.0,
            )
            .unwrap();
            assert!(
                p.files
                    .iter()
                    .all(|(path, _)| path.starts_with(root.0.join(FOLDER)))
            );
            assert_eq!(p.meshes.is_empty(), step == 2);
            let scene = p
                .files
                .iter()
                .find(|(path, _)| path.ends_with("preview.collection"))
                .unwrap()
                .1
                .clone();
            if step != 2 {
                assert!(String::from_utf8_lossy(&scene).contains("/heron_brush_live/preview/"));
                assert!(
                    p.files
                        .iter()
                        .any(|(path, _)| path.extension().is_some_and(|e| e == "collisionobject"))
                );
            }
            let model = p
                .files
                .iter()
                .find(|(path, _)| path.extension().is_some_and(|e| e == "model"));
            if let Some((_, model)) = model {
                assert!(String::from_utf8_lossy(model).contains("/main/model.material"));
            }
            snapshots.push(p.files);
        }
        assert_ne!(snapshots[0], snapshots[1]);
        assert_eq!(snapshots[1], snapshots[3]);
        assert!(doc.path.is_none());
    }
    #[test]
    fn asynchronous_requests_are_not_reported_as_completed() {
        assert!(
            reload_result(202, b"")
                .unwrap()
                .contains("does not confirm")
        );
        assert!(
            reload_result(200, br#"{"success":true}"#)
                .unwrap()
                .contains("completed")
        );
        assert!(reload_result(200, br#"{"success":false,"issues":[]}"#).is_err());
        assert!(reload_result(403, b"").is_err());
        assert!(supports_command(
            &serde_json::json!({"paths":{"/command/hot-reload":{"post":{}}}}),
            "hot-reload"
        ));
        assert!(supports_command(
            &serde_json::json!({"paths":{"/command/{command}":{"post":{"parameters":[{"name":"command","schema":{"enum":["hot-reload"]}}]}}}}),
            "hot-reload"
        ));
        assert!(!supports_command(&serde_json::json!({}), "hot-reload"));
    }
    #[test]
    fn truncated_http_responses_are_rejected() {
        assert_eq!(
            parse_response(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK").unwrap(),
            (200, b"OK".to_vec())
        );
        assert!(parse_response(b"HTTP/1.0 200 OK\r\nContent-Length: 6\r\n\r\nOK").is_err());
        assert!(parse_response(b"bad").is_err());
    }
    #[test]
    #[ignore = "requires an open Defold project in HERON_DEFOLD_LIVE_PROJECT"]
    fn native_editor_refresh() {
        let root = PathBuf::from(
            std::env::var_os("HERON_DEFOLD_LIVE_PROJECT").expect("test project path"),
        );
        println!(
            "{}",
            refresh(&root, std::env::var_os("HERON_DEFOLD_HOT_RELOAD").is_some()).unwrap()
        );
    }
}
