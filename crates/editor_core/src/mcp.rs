//! A Model Context Protocol server: `heron-brush --mcp`.
//!
//! Exposes document editing over MCP, including map creation, shapes, entities, face painting, CSG,
//! grouping and export. Headless requests operate on one [`Document`].
//!
//! The transport is newline-delimited JSON-RPC 2.0, which is MCP's stdio framing. The
//! loop is single-threaded and blocking: read a line, act on it, write the reply.
//!
//! Most tools act on the *selection*, exactly as the toolbar's commands do. `add_shape`
//! leaves what it made selected; `select` changes the working set.

use std::io::{BufRead, Write};
use std::rc::Rc;

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;
use serde_json::{json, Value};

use crate::collab::{Session, SessionEvent};
use crate::document::{Document, FaceRef};
use crate::history::Edit;
use crate::world::WorldBounds;

#[path = "mcp_animation.rs"]
mod animation;
#[path = "mcp_extensions.rs"]
mod extensions;
#[path = "mcp_settings.rs"]
mod settings;

/// Where a frontend's live session stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendState {
    /// No session.
    Idle,
    /// Hosting, with this many guests connected.
    Hosting(usize),
    /// Joined someone else's session.
    Joined,
    /// A joined session's link dropped; edits are held until it is back.
    Reconnecting,
}

/// The live session an agent has joined or is hosting, plus the pending accept for a
/// host that is still waiting for the other person to connect.
#[derive(Default)]
pub struct Collab {
    pub settings_path: Option<std::path::PathBuf>,
    pub settings_changed: bool,
    pub animation_preview: Option<(u32, u32)>,
    pub trigger_preview: Option<bool>,
    session: Option<Rc<Session>>,
    pending_host: Option<crate::collab::session::SessionInbox>,
    /// Each participant's selection, from the last poll, for the agent to read back.
    peer_selection: std::collections::HashMap<u32, Vec<u64>>,
    /// The name the agent goes by in the session.
    my_name: String,
    /// Retained until a manual rejoin adopts the host's world, or explicitly left.
    outage_edits: Rc<std::cell::RefCell<Vec<Edit>>>,
    outage_active: bool,
    /// The other participants' names, as they say them.
    peer_names: std::collections::HashMap<u32, String>,
}

impl Collab {
    /// Polls a frontend-owned session without routing UI actions through JSON.
    pub fn poll_frontend(&mut self, document: &mut Document) {
        self.check_pending_host(document);
        self.absorb(document);
    }
    pub fn frontend_status(&self) -> String {
        session_status(self)
    }
    /// Where a frontend's session stands, for its window and its header indicator.
    pub fn frontend_state(&self) -> FrontendState {
        match &self.session {
            Some(session) if session.is_host() => FrontendState::Hosting(session.participants()),
            Some(_) => FrontendState::Joined,
            None if self.pending_host.is_some() => FrontendState::Hosting(0),
            None if self.outage_active => FrontendState::Reconnecting,
            None => FrontendState::Idle,
        }
    }
    pub fn frontend_active(&self) -> bool {
        self.session.is_some() || self.pending_host.is_some()
    }
    /// For a header indicator: whether this end hosts, and how many are linked. A host
    /// still waiting for its first guest counts as hosting with none.
    pub fn frontend_session(&self) -> Option<(bool, usize)> {
        match &self.session {
            Some(session) => Some((session.is_host(), session.participants())),
            None => self.pending_host.as_ref().map(|_| (true, 0)),
        }
    }
    pub fn host_frontend(
        &mut self,
        name: String,
        token: String,
        inbox: crate::collab::session::SessionInbox,
    ) -> Result<String, String> {
        if self.frontend_active() {
            return Err("Leave the current session first".into());
        }
        self.my_name = name;
        self.pending_host = Some(inbox);
        Ok(token)
    }
    pub fn join_frontend(
        &mut self,
        document: &mut Document,
        name: String,
        session: Session,
    ) -> Result<(), String> {
        if self.frontend_active() {
            return Err("Leave the current session first".into());
        }
        self.my_name = name;
        self.adopt(document, session);
        Ok(())
    }
    pub fn leave_frontend(&mut self, document: &mut Document) {
        self.session = None;
        self.pending_host = None;
        self.outage_active = false;
        self.outage_edits.borrow_mut().clear();
        self.peer_names.clear();
        self.peer_selection.clear();
        document.clear_on_edit();
        document.clear_peers();
    }
    pub fn frontend_presence(&self, document: &Document, camera: &crate::camera3d::Camera3d) {
        if let Some(session) = &self.session {
            session.send_selection(document.selection_ids());
            session.send_camera(camera.position, camera.direction());
        }
    }

    /// Wires `document`'s edit hook to a freshly made session, sends our name, stores it.
    fn adopt(&mut self, document: &mut Document, session: Session) {
        if session.is_host() {
            self.outage_edits.borrow_mut().clear();
        }
        self.outage_active = false;
        let session = Rc::new(session);
        if !self.my_name.is_empty() {
            session.send_name(&self.my_name);
        }
        document.set_on_edit(Rc::new({
            let session = Rc::clone(&session);
            move |edit: &Edit| session.send_edit(edit)
        }));
        self.session = Some(session);
    }

    /// Drains whatever the other person has sent and applies it to `document`. Returns a
    /// one-line summary, or `None` when there is no session. Drops the session on a
    /// peer-left.
    fn absorb(&mut self, document: &mut Document) -> Option<String> {
        let session = self.session.clone()?;
        let bounds = document.world_bounds();
        let mut applied = 0;
        let mut adopted = false;
        let mut left = false;
        for event in session.poll(&bounds) {
            match event {
                SessionEvent::AdoptWorld(world) => {
                    document.adopt_world(world);
                    for edit in self.outage_edits.borrow_mut().drain(..) {
                        document.apply_remote(&edit);
                        session.send_edit(&edit);
                    }
                    adopted = true;
                }
                SessionEvent::ApplyEdit(edit) => {
                    document.apply_remote(&edit);
                    applied += 1;
                }
                SessionEvent::PeerSelection { peer, ids } => {
                    self.peer_selection
                        .insert(peer, ids.iter().map(|id| id.0).collect());
                    document.set_peer_selection(peer, ids);
                }
                SessionEvent::PeerName { peer, name } => {
                    document.set_peer_name(peer, name.clone());
                    self.peer_names.insert(peer, name);
                }
                // An agent has no viewport to draw the others' cameras in; noted in the
                // document all the same so nothing else has to care who reads it.
                SessionEvent::PeerCamera {
                    peer,
                    position,
                    direction,
                } => document.set_peer_camera(peer, Some((position, direction))),
                SessionEvent::Undecodable(line) => {
                    eprintln!("collab: could not read a message: {line}");
                }
                SessionEvent::PeerLeft { peer, name } => {
                    self.peer_names.remove(&peer);
                    self.peer_selection.remove(&peer);
                    document.remove_peer(peer);
                    eprintln!(
                        "collab: {} left the session",
                        name.unwrap_or_else(|| format!("peer {peer}"))
                    );
                }
                SessionEvent::ConnectionLost => left = true,
            }
        }
        if left {
            self.outage_active = true;
            self.outage_edits
                .borrow_mut()
                .extend(session.take_unacknowledged());
            let held = self.outage_edits.clone();
            document.set_on_edit(Rc::new(move |edit| held.borrow_mut().push(edit.clone())));
            document.clear_peers();
            self.session = None;
            self.peer_selection.clear();
            self.peer_names.clear();
            return Some("the session's link dropped".to_string());
        }
        let mut parts = Vec::new();
        if adopted {
            parts.push(format!(
                "adopted the shared map ({} brushes)",
                brush_list(document).len()
            ));
        }
        if applied > 0 {
            parts.push(format!("applied {applied} edits from the other person"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    /// If a hosted session's peer has connected since last checked, finishes the hookup:
    /// sends them the current world and starts forwarding edits.
    fn check_pending_host(&mut self, document: &mut Document) -> Option<String> {
        let rx = self.pending_host.as_ref()?;
        match rx.try_recv() {
            Ok(Ok(session)) => {
                let peer = session.peer().to_string();
                if self.session.is_some() {
                    // Drain existing edits before taking the newcomer's snapshot.
                    self.absorb(document);
                    let active = self.session.as_ref().expect("host session remains active");
                    let id = active.add_peer(session);
                    active.send_world_to(id, document.tree_world());
                    active.send_name(&self.my_name);
                } else {
                    session.send_world(document.tree_world());
                    self.adopt(document, session);
                }
                Some(format!("the other person joined from {peer}"))
            }
            Ok(Err(error)) => {
                self.pending_host = None;
                Some(format!("hosting failed: {error}"))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending_host = None;
                Some("the host stopped waiting".into())
            }
        }
    }

    fn is_live(&self) -> bool {
        self.session.is_some()
    }
}

/// Sets a map the agent starts or opens up for the game the preferences name, as the
/// window does: the game's tags and face flag names, and its entity definitions (or the
/// one `.fgd` beside the map). Without this a Paintball 2 `light` was placed with the
/// keys of Heron's own `light` class, which the game reports as bad fields.
///
/// Returns what loading the definitions had to say, for the reply.
fn set_up_for_game(document: &mut Document, map: Option<&std::path::Path>) -> Vec<String> {
    let game = crate::games::current();
    crate::document_setup::apply_game(document, game.as_ref(), map.is_some());
    let Some(fgd) = crate::document_setup::definition_file(map, game.as_ref()) else {
        return Vec::new();
    };
    match document.load_definitions(&fgd) {
        Ok(warnings) => warnings,
        Err(error) => vec![format!("{}: {error}", fgd.display())],
    }
}

/// Runs the server until stdin closes, then exits.
pub fn serve() -> ! {
    // The user's game, game path and mods, as the window reads them: a wad's palette and
    // the entity models come from the game's own files.
    crate::preferences::set_current(crate::preferences::Preferences::load(
        &crate::preferences::Preferences::default_path(),
    ));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut document = Document::new(WorldBounds::default());
    let mut collab = Collab::default();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(reply) = respond(&mut document, &mut collab, &line, true) {
            send(&mut out, reply);
        }
    }
    std::process::exit(0);
}

/// Answers one JSON-RPC line, or `None` for a notification (or blank line) that wants
/// no reply. `allow_collab` is off for the live GUI endpoint, whose window already owns
/// the session -- the collab tools there would fight it for the document's edit hook.
pub fn respond(
    document: &mut Document,
    collab: &mut Collab,
    line: &str,
    allow_collab: bool,
) -> Option<Value> {
    if line.trim().is_empty() {
        return None;
    }
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(error) => {
            return Some(error_reply(
                Value::Null,
                -32700,
                &format!("parse error: {error}"),
            ));
        }
    };
    if !request.is_object()
        || request.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || request.get("method").and_then(Value::as_str).is_none()
    {
        return Some(error_reply(Value::Null, -32600, "invalid JSON-RPC request"));
    }
    let id = request.get("id").cloned();
    id.as_ref()?;
    if !id
        .as_ref()
        .is_some_and(|id| id.is_string() || id.is_i64() || id.is_u64())
    {
        return Some(error_reply(
            Value::Null,
            -32600,
            "id must be a string or integer",
        ));
    }
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    if !params.is_object() {
        return Some(error_reply(
            id.unwrap_or(Value::Null),
            -32602,
            "params must be an object",
        ));
    }
    match method {
        "initialize" => Some(result_reply(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "heron-brush", "version": env!("CARGO_PKG_VERSION") },
            }),
        )),
        "ping" => Some(result_reply(id, json!({}))),
        "tools/list" => Some(result_reply(id, json!({ "tools": tool_definitions() }))),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            if !allow_collab
                && matches!(
                    name,
                    "host_session" | "join_session" | "leave_session" | "session_status"
                )
            {
                return Some(result_reply(
                    id,
                    json!({
                        "content": [{ "type": "text", "text":
                            "the live window owns its session; host or join from its menu" }],
                        "isError": true,
                    }),
                ));
            }

            if !allow_collab
                && document.has_edit_listener()
                && matches!(name, "new_map" | "open_map")
            {
                return Some(result_reply(
                    id,
                    json!({"content":[{"type":"text","text":"replace the shared map from the GUI, or leave the collaboration session first"}],"isError":true}),
                ));
            }

            // Before the agent acts, take in whatever the other person did, so it is
            // working against the current shared map.
            let mut notes: Vec<String> = Vec::new();
            if let Some(note) = collab.check_pending_host(document) {
                notes.push(note);
            }
            if let Some(note) = collab.absorb(document) {
                notes.push(note);
            }

            let was_live = collab.is_live();
            let reply = match call_tool(document, collab, name, &arguments) {
                Ok(text) => {
                    // A map swap has to be pushed whole; ordinary edits ride the hook.
                    if was_live && matches!(name, "new_map" | "open_map") {
                        if let Some(session) = &collab.session {
                            session.send_world(document.tree_world());
                        }
                    }
                    let mut content = vec![json!({"type":"text","text":text})];
                    if !notes.is_empty() {
                        content.push(json!({"type":"text","text":notes.join("; ")}));
                    }
                    json!({ "content": content })
                }
                Err(message) => {
                    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
                }
            };
            Some(result_reply(id, reply))
        }
        // Notifications carry no id and want no reply.
        _ if method.starts_with("notifications/") => None,
        _ => id.map(|id| error_reply(id, -32601, &format!("unknown method: {method}"))),
    }
}

fn send(out: &mut impl Write, message: Value) {
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

fn result_reply(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn error_reply(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

// --------------------------------------------------------------------------------------
// Argument helpers
// --------------------------------------------------------------------------------------

fn vec3(args: &Value, key: &str) -> Result<Vec3d, String> {
    let array = args
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("`{key}` must be an array of three numbers"))?;
    if array.len() != 3 {
        return Err(format!("`{key}` must have exactly three numbers"));
    }
    let mut out = [0.0; 3];
    for (index, value) in array.iter().enumerate() {
        out[index] = value
            .as_f64()
            .ok_or_else(|| format!("`{key}[{index}]` is not a number"))?;
    }
    Ok(Vec3d::new(out[0], out[1], out[2]))
}

fn f64_or(args: &Value, key: &str, default: f64) -> f64 {
    args.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn string_or<'a>(args: &'a Value, key: &str, default: &'a str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn rgb(args: &Value, key: &str) -> Result<map_model::brush_face::Rgb, String> {
    let colour = vec3(args, key)?;
    let channel = |c: f64| c.round().clamp(0.0, 255.0) as u8;
    Ok(map_model::brush_face::Rgb::new(
        channel(colour.x()),
        channel(colour.y()),
        channel(colour.z()),
    ))
}

/// Every brush in the document, id and bounds.
fn brush_list(document: &Document) -> Vec<Value> {
    map_model::node::brushes_with_ids(document.tree_world())
        .into_iter()
        .map(|(id, brush)| {
            let bounds = brush.bounds();
            json!({
                "id": id.0.to_string(),
                "min": [bounds.min.x(), bounds.min.y(), bounds.min.z()],
                "max": [bounds.max.x(), bounds.max.y(), bounds.max.z()],
            })
        })
        .collect()
}

/// One brush face by face: the material, the paint if any, the outward normal and the
/// corners of each face that contributes surface. The face indices are the ones
/// `set_face_color` takes, so an agent can look before it paints.
fn describe_brush(document: &Document, args: &Value) -> Result<String, String> {
    let wanted = extensions::node_id(args.get("brush").ok_or("`brush` is required")?)?;
    for (id, brush) in map_model::node::brushes_with_ids(document.tree_world()) {
        if id.0 != wanted {
            continue;
        }
        let bounds = brush.bounds();
        let mut faces = Vec::new();
        brush.for_each_geometry_face(|index, face, vertices| {
            let color = face.paint().as_ref().map(|color| match color {
                map_model::brush_face::FaceColor::Solid { color, alpha } => {
                    json!({ "color": [color.r, color.g, color.b], "alpha": alpha })
                }
                map_model::brush_face::FaceColor::MultiGradient { stops, radial, angle, alpha, ends } => {
                    let mut gradient = json!({ "type": if *radial { "radial" } else { "linear" }, "angle": angle,
                        "stops": stops.as_slice().iter().map(|s| json!({"position": s.position, "color": [s.color.r, s.color.g, s.color.b]})).collect::<Vec<_>>() });
                    if let Some(ends) = ends {
                        gradient["start"] = json!(ends.start);
                        gradient["end"] = json!(ends.end);
                    }
                    json!({ "gradient": gradient, "alpha": alpha })
                }
                map_model::brush_face::FaceColor::Gradient {
                    from,
                    to,
                    angle,
                    alpha,
                } => json!({
                    "gradient": {
                        "from": [from.r, from.g, from.b],
                        "to": [to.r, to.g, to.b],
                        "angle": angle,
                    },
                    "alpha": alpha,
                }),
            });
            faces.push(json!({
                "face": index,
                // Editing tools address geometry faces; map files store source faces.
                "source_face": brush.source_face_index(index),
                "material": face.material_name.as_str(),
                "normal": [
                    face.plane.normal.x(),
                    face.plane.normal.y(),
                    face.plane.normal.z(),
                ],
                "paint": color,
                "surface_material": face.surface_material.map(|s| { let f=s.factors(); json!({"roughness":f[0],"metallic":f[1],"emission":f[2]}) }),
                "vertices": vertices
                    .iter()
                    .map(|v| [v.x(), v.y(), v.z()])
                    .collect::<Vec<_>>(),
            }));
        });
        return Ok(serde_json::to_string_pretty(&json!({
            "id": wanted.to_string(),
            "min": [bounds.min.x(), bounds.min.y(), bounds.min.z()],
            "max": [bounds.max.x(), bounds.max.y(), bounds.max.z()],
            "faces": faces,
        }))
        .unwrap_or_default());
    }
    Err(format!("no brush with id {wanted}"))
}

/// The entity classes the loaded definitions offer: summaries without a `classname`,
/// or one class in full -- its properties included -- with one.
fn list_entity_classes(document: &Document, args: &Value) -> Result<String, String> {
    let Some(definitions) = document.definitions() else {
        return Err(
            "no entity definitions loaded; call load_definitions with a .fgd or .def file".into(),
        );
    };
    let kind_name = |kind: map_model::fgd::DefinitionKind| match kind {
        map_model::fgd::DefinitionKind::Point => "point",
        map_model::fgd::DefinitionKind::Solid => "brush",
        map_model::fgd::DefinitionKind::Base => "base",
    };

    let wanted = string_or(args, "classname", "");
    if !wanted.is_empty() {
        let Some(class) = definitions.find(wanted) else {
            return Err(format!("no class named {wanted}"));
        };
        let properties: Vec<Value> = class
            .properties
            .iter()
            .map(|property| {
                json!({
                    "key": property.key,
                    "default": property.default,
                    "description": property.long_description,
                })
            })
            .collect();
        return Ok(serde_json::to_string_pretty(&json!({
            "classname": class.name,
            "kind": kind_name(class.kind),
            "description": class.description,
            "color": class.color,
            "properties": properties,
        }))
        .unwrap_or_default());
    }

    let classes: Vec<Value> = definitions
        .placeable()
        .map(|class| {
            json!({
                "classname": class.name,
                "kind": kind_name(class.kind),
                "description": class.description,
            })
        })
        .collect();
    Ok(serde_json::to_string_pretty(&classes).unwrap_or_default())
}

// --------------------------------------------------------------------------------------
// Tools
// --------------------------------------------------------------------------------------

pub(crate) fn call_tool(
    document: &mut Document,
    collab: &mut Collab,
    name: &str,
    args: &Value,
) -> Result<String, String> {
    if document.in_transaction() {
        return Err(
            "The editor is finishing an interactive drag; retry after the mouse is released."
                .into(),
        );
    }
    let definition = tool_definitions()
        .into_iter()
        .find(|tool| tool["name"] == name)
        .ok_or_else(|| format!("no such tool: {name}"))?;
    extensions::validate(args, &definition["inputSchema"], "arguments")?;
    if matches!(name, "get_editor_settings" | "set_editor_settings") {
        return settings::call(collab, name, args);
    }
    if matches!(
        name,
        "configure_animation_trigger"
            | "get_animation_trigger"
            | "preview_animation_trigger"
            | "animation_library"
            | "connect_animation_button"
            | "set_animation_pivot"
            | "key_animation_pose"
            | "retime_animation"
            | "get_animation"
            | "set_animation"
            | "apply_animation_recipe"
            | "sample_animation"
    ) {
        return animation::call(document, collab, name, args);
    }
    match name {
        "new_map" => {
            if collab.is_live() {
                return Err("leave the shared session before creating another map".into());
            }
            collab.outage_edits.borrow_mut().clear();
            if collab.outage_active {
                document.clear_on_edit();
                collab.outage_active = false;
            }
            let mut fresh = Document::new(WorldBounds::default());
            let notes = set_up_for_game(&mut fresh, None);
            document.replace_for_agent(fresh);
            extensions::reconnect(document, collab);
            Ok(extensions::with_warnings(
                "started a new map".into(),
                &notes,
            ))
        }
        "host_session" => host_session(document, collab, args),
        "join_session" => join_session(document, collab, args),
        "leave_session" => {
            let had_outage = collab.outage_active;
            collab.outage_active = false;
            collab.outage_edits.borrow_mut().clear();
            let had_session = collab.session.take().is_some();
            let was_pending = collab.pending_host.take().is_some();
            if had_session || was_pending || had_outage {
                document.clear_on_edit();
                collab.peer_selection.clear();
                collab.peer_names.clear();
                document.clear_peers();
                Ok("left the session".into())
            } else {
                Err("not in a session".into())
            }
        }
        "session_status" => Ok(session_status(collab)),
        "open_map" => {
            if collab.is_live() {
                return Err("leave the shared session before opening another map".into());
            }
            let path = string_or(args, "path", "");
            if path.is_empty() {
                return Err("`path` is required".into());
            }
            let mut opened = Document::open(path, WorldBounds::default())
                .map_err(|error| format!("could not open {path}: {error}"))?;
            let mut notes = set_up_for_game(&mut opened, Some(std::path::Path::new(path)));
            collab.outage_edits.borrow_mut().clear();
            if collab.outage_active {
                document.clear_on_edit();
                collab.outage_active = false;
            }
            document.replace_for_agent(opened);
            extensions::reconnect(document, collab);
            // The textures the map names, as the window loads them when it opens one.
            for (wad, loaded) in document.load_named_wads() {
                if let Err(error) = loaded {
                    crate::complain!("{}: {error}", wad.display());
                }
            }
            notes.extend(document.warnings());
            Ok(extensions::with_warnings(
                format!("opened {path}, {} brushes", brush_list(document).len()),
                &notes,
            ))
        }
        "add_shape" => add_shape(document, args),
        "add_point_entity" => {
            let classname = string_or(args, "classname", "");
            if classname.is_empty() {
                return Err("`classname` is required".into());
            }
            let origin = vec3(args, "origin")?;
            let id = document.create_point_entity(classname, origin);
            if let Some(properties) = args.get("properties").and_then(Value::as_object) {
                let ids = [id];
                for (key, value) in properties {
                    if let Some(value) = value.as_str() {
                        document.set_entity_property_on(&ids, key, value);
                    }
                }
            }
            Ok(format!("placed a {classname} (id {})", id.0))
        }
        "make_brush_entity" => {
            let classname = string_or(args, "classname", "");
            if classname.is_empty() {
                return Err("`classname` is required".into());
            }
            match document.create_brush_entity(classname) {
                Some(id) => Ok(format!(
                    "made a {classname} from the selection (id {})",
                    id.0
                )),
                None => Err("select some brushes first".into()),
            }
        }
        "select" => select(document, args),
        "set_face_color" => set_face_color(document, args),
        "set_face_material" => set_face_material(document, args),
        "set_face_flags" => set_face_flags(document, args),
        "omit_covered_faces" => Ok(format!(
            "Omitted {} fully covered static faces",
            document.omit_covered_faces()
        )),
        "set_face_export_hidden" => {
            let hidden = args["hidden"].as_bool().ok_or("hidden must be a boolean")?;
            match (args.get("brush"), args.get("face")) {
                (Some(brush), Some(face)) => {
                    let reference = FaceRef {
                        node: map_model::node::NodeId(extensions::node_id(brush)?),
                        face: face.as_u64().ok_or("invalid face")? as usize,
                    };
                    if document.face_at(reference).is_none() {
                        return Err("no such brush face".into());
                    }
                    document.set_export_hidden_on(&[reference], hidden);
                }
                (None, None) => document.edit_faces(|face| face.export_hidden = hidden),
                _ => return Err("provide both brush and face".into()),
            }
            Ok("Updated face export visibility; collision is preserved.".into())
        }
        "set_surface_material" => set_surface_material(document, args),
        "translate_selection" => {
            let delta = vec3(args, "delta")?;
            if document.translate_selection(delta) {
                Ok(format!("moved the selection by {delta:?}"))
            } else {
                Err("nothing selected, or the move left the world".into())
            }
        }
        "rotate_selection" => {
            let axis = vec3(args, "axis")?;
            let length = geometry_math::vec::length(axis);
            if !length.is_finite() || length <= 1e-12 {
                return Err("`axis` must be a nonzero finite vector".into());
            }
            let axis = axis / length;
            let degrees = f64_or(args, "degrees", 0.0);
            let done = match args.get("center") {
                Some(_) => document.rotate_selection_about(vec3(args, "center")?, axis, degrees),
                None => document.rotate_selection(axis, degrees),
            };
            if done {
                Ok(format!("turned the selection {degrees}° about {axis:?}"))
            } else {
                Err("nothing selected".into())
            }
        }
        "scale_selection" => scale_selection(document, args),
        "flip_selection" => {
            let axis = axis_arg(args)?;
            if document.flip_selection(axis) {
                Ok("flipped the selection".into())
            } else {
                Err("nothing selected".into())
            }
        }
        "csg_subtract" => Ok(format!(
            "subtracted, {} brushes cut",
            document.subtract_selection()
        )),
        "join_corner" => {
            if args["expected_revision"]
                .as_u64()
                .is_some_and(|r| r != document.revision())
            {
                return Err("Map or selection changed; request another preview.".into());
            }
            let read_rounding = |name: &str| -> Result<Option<(f64, usize)>, String> {
                let Some(value) = args.get(name) else {
                    return Ok(None);
                };
                let radius = value["radius"]
                    .as_f64()
                    .ok_or_else(|| format!("{name}.radius must be a number"))?;
                let segments = match value.get("segments") {
                    None => 8,
                    Some(value) => value
                        .as_u64()
                        .filter(|v| (1..=64).contains(v))
                        .ok_or_else(|| format!("{name}.segments must be from 1 to 64"))?
                        as usize,
                };
                Ok(Some((radius, segments)))
            };
            let rounding = crate::corner_join::Rounding {
                outer: read_rounding("outer")?,
                inner: read_rounding("inner")?,
                concentric: args["concentric"].as_bool().unwrap_or(false),
                wedges: args["wedges"].as_bool().unwrap_or(false),
                preserve_thickness: args["preserve_thickness"].as_bool().unwrap_or(false),
                snap: None,
            };
            let draft = document.preview_corner_join_options(rounding)?;
            let [a, b] = draft.join.before.each_ref().map(|brush| brush.bounds());
            let overlap = (0..3)
                .map(|axis| a.max[axis].min(b.max[axis]) - a.min[axis].max(b.min[axis]))
                .product::<f64>();
            let dry = args["dry_run"].as_bool().unwrap_or(true);
            if !dry {
                document.join_corner_options(draft.revision, rounding)?;
            }
            Ok(json!({"dry_run":dry,"revision":document.revision(),"brush_ids":draft.ids.map(|id|id.0.to_string()),"brush_count":2+draft.join.added.len(),"added_brushes":draft.join.added.len(),"plane":{"normal":draft.join.plane.normal.v,"distance":draft.join.plane.distance},"removed_overlap_volume":overlap,"removed_volume":draft.join.removed_volume,"added_volume":draft.join.added.iter().map(|brush|brush.volume()).sum::<f64>()}).to_string())
        }
        "csg_merge" => {
            if document.merge_selection() {
                Ok("merged the selection into one brush".into())
            } else {
                Err("select two or more brushes that overlap".into())
            }
        }
        "csg_intersect" => {
            if document.intersect_selection() {
                Ok("intersected the selection".into())
            } else {
                Err("select two or more brushes".into())
            }
        }
        "hollow" => {
            let thickness = f64_or(args, "thickness", 16.0);
            Ok(format!("hollowed {} brushes", {
                let direction = match string_or(args, "direction", "inward") {
                    "inward" => map_model::hollow::Direction::Inward,
                    "outward" => map_model::hollow::Direction::Outward,
                    "centered" => map_model::hollow::Direction::Centered,
                    _ => return Err("direction must be inward, outward or centered".into()),
                };
                let faces = if args["open_selected_faces"].as_bool().unwrap_or(false) {
                    document.selected_faces()
                } else {
                    Vec::new()
                };
                if args["open_selected_faces"].as_bool().unwrap_or(false) && faces.is_empty() {
                    return Err("Select the faces to open first.".into());
                }
                document.hollow_selection_options(
                    thickness,
                    direction,
                    &faces,
                    args["join_convex"].as_bool().unwrap_or(false),
                )?
            }))
        }
        "group_selection" => {
            let group_name = string_or(args, "name", "Group");
            match document.group_selection(group_name) {
                Some(id) => Ok(format!("grouped the selection (id {})", id.0)),
                None => Err("nothing selected".into()),
            }
        }
        "ungroup_selection" => Ok(format!(
            "took apart {} groups",
            document.ungroup_selection()
        )),
        "move_selection_to_layer" => move_selection_to_layer(document, args),
        "delete_selection" => Ok(format!("deleted {} brushes", document.delete_selection())),
        "undo" => match document.undo() {
            Some(what) => Ok(format!("undid: {what}")),
            None => Err("nothing to undo".into()),
        },
        "redo" => match document.redo() {
            Some(what) => Ok(format!("redid: {what}")),
            None => Err("nothing to redo".into()),
        },
        "list_brushes" => {
            Ok(serde_json::to_string_pretty(&brush_list(document)).unwrap_or_default())
        }
        "describe_brush" => describe_brush(document, args),
        // What the person selected in the window, for "change this".
        "get_selection" => {
            let mut ids: Vec<String> = document
                .selection_ids()
                .iter()
                .map(|id| id.0.to_string())
                .collect();
            ids.sort();
            Ok(json!({ "count": ids.len(), "ids": ids }).to_string())
        }
        "describe_selection" => Ok(serde_json::to_string_pretty(
            &crate::selection_report::describe(document),
        )
        .unwrap_or_default()),
        "entity_graph" => {
            let graph = document.entity_graph();
            let nodes: Vec<Value> = graph.nodes.iter().map(|n| json!({
                "id": n.id.0.to_string(), "classname": n.classname, "name": n.name,
                "position": n.position.map(|p| [p.x(), p.y(), p.z()]), "selected": n.selected,
            })).collect();
            let edges: Vec<Value> = graph.edges.iter().map(|e| json!({
                "from": e.from.0.to_string(), "to": e.to.0.to_string(), "key": e.key, "value": e.value,
            })).collect();
            Ok(
                serde_json::to_string_pretty(&json!({ "nodes": nodes, "edges": edges }))
                    .unwrap_or_default(),
            )
        }
        "link_entities" => {
            let from = map_model::node::NodeId(extensions::node_id(
                args.get("from").ok_or("`from` is required")?,
            )?);
            let to = map_model::node::NodeId(extensions::node_id(
                args.get("to").ok_or("`to` is required")?,
            )?);
            document
                .link_entities(from, to)
                .map(|name| format!("linked: it now aims at \"{name}\""))
        }
        "load_definitions" => {
            let path = string_or(args, "path", "");
            if path.is_empty() {
                return Err("`path` is required (a .fgd or .def file)".into());
            }
            let warnings = document
                .load_definitions(path)
                .map_err(|error| format!("could not read {path}: {error}"))?;
            let classes = document.definitions().map_or(0, |d| d.placeable().count());
            Ok(if warnings.is_empty() {
                format!("loaded {classes} entity classes")
            } else {
                format!(
                    "loaded {classes} entity classes; warnings: {}",
                    warnings.join("; ")
                )
            })
        }
        "list_entity_classes" => list_entity_classes(document, args),
        "load_materials" => {
            let path = string_or(args, "path", "");
            if path.is_empty() {
                return Err("`path` is required (a directory of images)".into());
            }
            let warnings = document
                .load_materials(path)
                .map_err(|error| format!("could not read {path}: {error}"))?;
            let count = document.materials().len();
            Ok(if warnings.is_empty() {
                format!("loaded {count} materials")
            } else {
                format!(
                    "loaded {count} materials; warnings: {}",
                    warnings.join("; ")
                )
            })
        }
        "load_wad" => {
            let path = string_or(args, "path", "");
            if path.is_empty() {
                return Err("`path` is required (a .wad file)".into());
            }
            let warnings = document
                .add_wad_to_map(std::path::Path::new(path))
                .map_err(|error| format!("could not read {path}: {error}"))?;
            let named = document
                .worldspawn()
                .property("wad")
                .unwrap_or_default()
                .to_string();
            Ok(extensions::with_warnings(
                format!(
                    "loaded {} materials; worldspawn wad is now \"{named}\"",
                    document.materials().len()
                ),
                &warnings,
            ))
        }
        "set_entity_property" => {
            let key = string_or(args, "key", "");
            if key.is_empty() {
                return Err("`key` is required".into());
            }
            let ids = match args.get("entity") {
                Some(Value::String(name)) if name == "worldspawn" => vec![Document::WORLDSPAWN],
                Some(id) => vec![map_model::node::NodeId(extensions::node_id(id)?)],
                // The selected entities, and the ones whose brushes are selected: what
                // make_brush_entity leaves selected is the door's brush.
                None => {
                    let mut ids: Vec<_> = document
                        .selection_ids()
                        .iter()
                        .filter_map(|id| document.entity_around(*id))
                        .map(|entity| entity.id)
                        .collect();
                    ids.sort();
                    ids.dedup();
                    ids
                }
            };
            if ids.is_empty() {
                return Err("name an entity, or select one".into());
            }
            let changed = match args.get("value") {
                Some(Value::String(value)) => document.set_entity_property_on(&ids, key, value),
                Some(Value::Null) | None => document.remove_entity_property_on(&ids, key),
                Some(_) => return Err("`value` must be a string, or null to remove".into()),
            };
            if changed {
                let mut notes = Vec::new();
                if key == "wad" {
                    for (wad, loaded) in document.load_named_wads() {
                        match loaded {
                            Ok(warnings) => notes.extend(warnings),
                            Err(error) => notes.push(format!("{}: {error}", wad.display())),
                        }
                    }
                }
                Ok(extensions::with_warnings(
                    format!("set {key} on {} entities", ids.len()),
                    &notes,
                ))
            } else {
                Err("no such entity".into())
            }
        }
        "list_materials" => {
            let materials = document.materials();
            if materials.is_empty() {
                return Err(
                    "no materials loaded; call load_materials with a directory of images".into(),
                );
            }
            let list: Vec<Value> = materials
                .materials()
                .iter()
                .map(|material| {
                    json!({
                        "name": material.name,
                        "width": material.width,
                        "height": material.height,
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&list).unwrap_or_default())
        }
        "get_map" => Ok(document.to_map_text()),
        "save" => {
            let path = string_or(args, "path", "");
            if path.is_empty() {
                return Err("`path` is required".into());
            }
            document
                .save_as(std::path::PathBuf::from(path))
                .map_err(|error| format!("could not save {path}: {error}"))?;
            let exports: Vec<String> =
                crate::export::export_on_save(document, &crate::export_settings::Settings::load())
                    .into_iter()
                    .map(|done| done.map_or_else(|why| why, |written| written.to_string()))
                    .collect();
            Ok(extensions::with_warnings(format!("saved {path}"), &exports))
        }
        "export_map" => export_map(document, args),
        "export" => extensions::export(document, args, false),
        "check_export" => extensions::export(document, args, true),
        "get_export_settings" => extensions::export_settings(document, args),
        "list_plugins" => extensions::list_plugins(),
        "run_plugin" => extensions::run_plugin(document, args),
        "duplicate_selection" => {
            let delta = args
                .get("delta")
                .map(|_| vec3(args, "delta"))
                .transpose()?
                .unwrap_or(Vec3d::new(0.0, 0.0, 0.0));
            let count = document.duplicate_selection_by(delta);
            if count == 0 {
                return Err("nothing selected to duplicate".into());
            }
            Ok(format!("duplicated {count} objects"))
        }
        "render_preview" => render_preview(document, args),
        other => Err(format!("no such tool: {other}")),
    }
}

// --------------------------------------------------------------------------------------
// Live collaboration
// --------------------------------------------------------------------------------------

/// Hosts a session so a person in the GUI (or another agent) can join and edit the same
/// map. `transport` is "iroh" (default, works over the open internet, no server) or
/// "tcp"; "tcp" takes a `port` (default 27960). Returns right away with the token or
/// address to hand over; the join completes on a later call.
fn host_session(
    document: &mut Document,
    collab: &mut Collab,
    args: &Value,
) -> Result<String, String> {
    if collab.is_live() {
        return Err("already in a session; leave it first".into());
    }
    collab.my_name = string_or(args, "nickname", "").to_string();
    // A build without the `iroh` feature has only the local network, so that is what an
    // unqualified request gets rather than an error a caller cannot act on.
    #[cfg(feature = "iroh")]
    let default_transport = "iroh";
    #[cfg(not(feature = "iroh"))]
    let default_transport = "tcp";
    match string_or(args, "transport", default_transport)
        .to_ascii_lowercase()
        .as_str()
    {
        "iroh" => {
            let _ = document;
            let (token, session_rx) = Session::host_iroh().map_err(|e| e.to_string())?;
            collab.pending_host = Some(session_rx);
            Ok(format!(
                "hosting over iroh. Give this token to the other person to join with:\n{token}\n\
                 They connect when they dial in; keep working and call session_status to see."
            ))
        }
        "tcp" => {
            let port = args.get("port").and_then(Value::as_u64).unwrap_or(27960) as u16;
            let (_, session_rx) =
                Session::host_on_this_network(port).map_err(|error| error.to_string())?;
            collab.pending_host = Some(session_rx);
            Ok(format!(
                "hosting on TCP port {port}. The other person joins with this machine's \
                 address and that port; call session_status to see when they are on."
            ))
        }
        other => Err(format!(
            "`transport` must be \"iroh\" or \"tcp\", not {other:?}"
        )),
    }
}

/// Joins a session someone else is hosting. `target` is an iroh token or a `host:port`
/// TCP address. Blocks briefly to connect and to take in the shared map.
fn join_session(
    document: &mut Document,
    collab: &mut Collab,
    args: &Value,
) -> Result<String, String> {
    if collab.is_live() {
        return Err("already in a session; leave it first".into());
    }
    let target = string_or(args, "target", "");
    if target.is_empty() {
        return Err("`target` is required: an iroh token or a host:port address".into());
    }
    collab.my_name = string_or(args, "nickname", "").to_string();
    // A bare host:port is TCP; anything else is treated as an iroh token.
    let looks_tcp = target
        .rsplit_once(':')
        .is_some_and(|(_, port)| port.parse::<u16>().is_ok());
    let session = if looks_tcp {
        Session::join(target).map_err(|e| format!("could not connect to {target}: {e}"))?
    } else {
        Session::join_iroh(target).map_err(|e| format!("could not dial the token: {e}"))?
    };
    let peer = session.peer().to_string();
    collab.adopt(document, session);

    // Wait a moment for the host to send the shared map.
    let deadline = web_time::Instant::now() + web_time::Duration::from_secs(3);
    let mut adopted = false;
    while web_time::Instant::now() < deadline {
        if let Some(note) = collab.absorb(document) {
            if note.contains("adopted") {
                adopted = true;
                break;
            }
        }
        std::thread::sleep(web_time::Duration::from_millis(20));
    }
    if adopted {
        Ok(format!(
            "joined the session hosted by {peer}; working on the shared map"
        ))
    } else {
        Ok(format!(
            "joined {peer} but the shared map has not arrived yet; call session_status"
        ))
    }
}

fn session_status(collab: &Collab) -> String {
    if let Some(session) = &collab.session {
        let role = if session.is_host() {
            "hosting"
        } else {
            "joined"
        };
        let who = if collab.peer_names.is_empty() {
            session.peer()
        } else {
            let mut names: Vec<String> = collab.peer_names.values().cloned().collect();
            names.sort();
            names.join(", ")
        };
        let sel = if collab.peer_selection.values().all(|ids| ids.is_empty()) {
            "nothing".to_string()
        } else {
            format!("{:?}", collab.peer_selection)
        };
        format!(
            "in a session ({role}, {} linked) with {who}, selected by them: {sel}",
            session.participants()
        )
    } else if collab.pending_host.is_some() {
        "hosting, still waiting for the other person to connect".into()
    } else {
        "not in a session".into()
    }
}

fn axis_arg(args: &Value) -> Result<geometry_math::util::Axis, String> {
    match string_or(args, "axis", "z").to_ascii_lowercase().as_str() {
        "x" => Ok(geometry_math::util::Axis::X),
        "y" => Ok(geometry_math::util::Axis::Y),
        "z" => Ok(geometry_math::util::Axis::Z),
        other => Err(format!("`axis` must be x, y or z, not {other}")),
    }
}

fn add_shape(document: &mut Document, args: &Value) -> Result<String, String> {
    use crate::shape_tool::{ShapeKind, ShapeParameters};

    let kind_name = string_or(args, "kind", "cuboid").to_ascii_lowercase();
    let min = vec3(args, "min")?;
    let max = vec3(args, "max")?;
    let bounds = Bbox3d::new(
        geometry_math::vec::min(min, max),
        geometry_math::vec::max(min, max),
    );
    let material = string_or(args, "material", map_model::shapes::DEFAULT_MATERIAL);
    let world = document.world_bounds();

    // The wedge is its own path: a single hull, not one of the toolbar's parameterised
    // shapes, so it does not go through `ShapeParameters`.
    let brushes = if kind_name == "wedge" || kind_name == "ramp" {
        let axis = if args.get("axis").is_some() {
            axis_arg(args)?
        } else {
            geometry_math::util::Axis::X
        };
        vec![
            map_model::shapes::create_wedge(&world, &bounds, axis, material)
                .map_err(|error| format!("could not build the shape: {error}"))?,
        ]
    } else {
        let kind = match kind_name.as_str() {
            "cuboid" | "box" => ShapeKind::Cuboid,
            "stairs" => ShapeKind::Stairs,
            "arch" => ShapeKind::Arch,
            "cylinder" => ShapeKind::Cylinder,
            "cone" => ShapeKind::Cone,
            "sphere" | "uvsphere" => ShapeKind::UvSphere,
            "icosphere" => ShapeKind::IcoSphere,
            "hollow_sphere" => ShapeKind::HollowSphere,
            other => return Err(format!("unknown shape kind: {other}")),
        };

        let mut parameters = ShapeParameters {
            kind,
            ..ShapeParameters::default()
        };
        parameters.axis = axis_arg(args)?;
        if kind == ShapeKind::Stairs {
            use crate::shape_tool::StairDirection;
            let axis = if args.get("axis").is_some() {
                parameters.axis
            } else {
                geometry_math::util::Axis::X
            };
            let ascending = args["ascending"].as_bool().unwrap_or(true);
            parameters.stair_direction = match (axis, ascending) {
                (geometry_math::util::Axis::X, true) => StairDirection::PosX,
                (geometry_math::util::Axis::X, false) => StairDirection::NegX,
                (geometry_math::util::Axis::Y, true) => StairDirection::PosY,
                (geometry_math::util::Axis::Y, false) => StairDirection::NegY,
                _ => return Err("Stairs run along x or y.".into()),
            };
        }
        if let Some(value) = args.get("accuracy") {
            if kind == ShapeKind::HollowSphere {
                return Err(
                    "hollow_sphere uses brushes (20 to 320, in steps of four), not accuracy".into(),
                );
            }
            parameters.accuracy = value
                .as_u64()
                .filter(|n| *n <= 4)
                .ok_or("accuracy is outside the supported range")?
                as usize;
        }
        if let Some(value) = args.get("brushes") {
            if kind != ShapeKind::HollowSphere {
                return Err("brushes is only supported for hollow_sphere".into());
            }
            parameters.sphere_brushes = value
                .as_u64()
                .filter(|n| (20..=320).contains(n) && n % 4 == 0)
                .ok_or("brushes must be a multiple of four from 20 to 320")?
                as usize;
        }
        if let Some(sides) = args.get("sides").and_then(Value::as_u64) {
            parameters.num_sides = (sides as usize).max(3);
        }
        if let Some(step) = args.get("step_height").and_then(Value::as_f64) {
            parameters.step_height = step;
        }
        parameters.stairs.layout = match string_or(args, "stair_layout", "straight") {
            "straight" => map_model::shapes::StairLayout::Straight,
            "left" => map_model::shapes::StairLayout::Left,
            "right" => map_model::shapes::StairLayout::Right,
            "return" => map_model::shapes::StairLayout::Return,
            _ => return Err("stair_layout must be straight, left, right or return".into()),
        };
        if let Some(v) = args["landing"].as_f64() {
            parameters.stairs.landing = v;
        }
        if let Some(v) = args["flight_width"].as_f64() {
            parameters.stairs.width = v;
        }
        if let Some(v) = args["slab_thickness"].as_f64() {
            parameters.stairs.slab = v;
        }
        parameters.hollow = args.get("hollow").and_then(Value::as_bool).unwrap_or(false);
        if let Some(thickness) = args.get("thickness").and_then(Value::as_f64) {
            parameters.thickness = thickness;
            parameters.sphere_thickness = thickness;
        }

        crate::shape_tool::create_brushes(&world, &bounds, &parameters, material)
            .map_err(|error| format!("could not build the shape: {error}"))?
    };
    if brushes.is_empty() {
        return Err("the bounds were too small to make a solid".into());
    }
    let brushes = document.with_default_uv_scale(brushes);

    // An optional turn about the shape's own centre, x then y then z, folded into the
    // add so the whole thing is one undo step. This is what lets an agent lay a
    // cylinder along a slope without CSG gymnastics.
    let rotate = args
        .get("rotate")
        .map(|_| vec3(args, "rotate"))
        .transpose()?;
    let count = brushes.len();
    document.run_as_one_edit(|document| {
        document.add_brushes(brushes);
        if let Some(turn) = rotate {
            for (axis, degrees) in [
                (Vec3d::new(1.0, 0.0, 0.0), turn.x()),
                (Vec3d::new(0.0, 1.0, 0.0), turn.y()),
                (Vec3d::new(0.0, 0.0, 1.0), turn.z()),
            ] {
                if degrees != 0.0 {
                    document.rotate_selection(axis, degrees);
                }
            }
        }
    });
    let ids: Vec<String> = document
        .selection_ids()
        .iter()
        .map(|id| id.0.to_string())
        .collect();
    Ok(format!("added {count} brushes, selected: {ids:?}"))
}

fn select(document: &mut Document, args: &Value) -> Result<String, String> {
    match string_or(args, "what", "").to_ascii_lowercase().as_str() {
        "all" => {
            document.select_all();
            Ok(format!(
                "selected everything ({} objects)",
                document.selection_count()
            ))
        }
        "none" => {
            document.clear_selection();
            Ok("cleared the selection".into())
        }
        _ => {
            let ids = args
                .get("ids")
                .and_then(Value::as_array)
                .ok_or("provide ids, or what: all/none")?;
            let mut chosen = std::collections::BTreeSet::new();
            for id in ids {
                let id = extensions::node_id(id)?;
                if map_model::node::find_node(document.tree_world(), map_model::node::NodeId(id))
                    .is_none()
                {
                    return Err(format!("no object with id {id}"));
                }
                chosen.insert(id);
            }
            document.clear_selection();
            let taken = chosen.len();
            for id in chosen {
                document.toggle_selected(map_model::node::NodeId(id));
            }
            Ok(format!("selected {taken} objects"))
        }
    }
}

fn set_surface_material(document: &mut Document, args: &Value) -> Result<String, String> {
    let value = if args.get("reset").and_then(Value::as_bool).unwrap_or(false) {
        None
    } else {
        let factor = |name, default, max| -> Result<f64, String> {
            let v = args
                .get(name)
                .map(|v| v.as_f64().ok_or_else(|| format!("{name} must be a number")))
                .transpose()?
                .unwrap_or(default);
            if !v.is_finite() || v < 0.0 || v > max {
                return Err(format!("{name} must be between 0 and {max}"));
            }
            Ok(v)
        };
        Some(map_model::brush_face::SurfaceMaterial {
            roughness: (factor("roughness", 0.65, 1.0)? * 100.0).round() as u8,
            metallic: (factor("metallic", 0.0, 1.0)? * 100.0).round() as u8,
            emission: (factor("emission", 0.0, 8.0)? * 100.0).round() as u16,
        })
    };
    let changed = match (args.get("brush"), args.get("face")) {
        (Some(brush), Some(face)) => {
            let reference = FaceRef {
                node: map_model::node::NodeId(extensions::node_id(brush)?),
                face: face.as_u64().ok_or("invalid face")? as usize,
            };
            if document.face_at(reference).is_none() {
                return Err("no such brush face".into());
            }
            document.set_surface_material_on(&[reference], value)
        }
        (None, None) => document.set_surface_material(value),
        _ => return Err("provide both brush and face".into()),
    };
    Ok(format!("updated {changed} faces"))
}

fn set_face_color(document: &mut Document, args: &Value) -> Result<String, String> {
    use map_model::brush_face::FaceColor;
    if args.get("color").is_some() && args.get("gradient").is_some() {
        return Err("choose color or gradient, not both".into());
    }
    let color = if let Some(gradient) = args.get("gradient") {
        let angle = f64_or(gradient, "angle", 0.0) as f32;
        if !angle.is_finite() {
            return Err("gradient angle must be finite".into());
        }
        let kind = gradient
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("linear");
        if kind != "linear" && kind != "radial" {
            return Err("gradient type must be linear or radial".into());
        }
        let point = |name: &str| -> Result<Option<[f32; 2]>, String> {
            let Some(value) = gradient.get(name) else {
                return Ok(None);
            };
            let pair = value
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or(format!("{name} must be [u, v]"))?;
            let p = [pair[0].as_f64(), pair[1].as_f64()];
            match p {
                [Some(u), Some(v)] if u.is_finite() && v.is_finite() => {
                    Ok(Some([u as f32, v as f32]))
                }
                _ => Err(format!("{name} must be two finite numbers")),
            }
        };
        let ends = match (point("start")?, point("end")?) {
            (Some(start), Some(end)) => Some(map_model::brush_face::GradientEnds { start, end }),
            (None, None) => None,
            _ => return Err("give both start and end, or neither".into()),
        };
        if let Some(values) = gradient.get("stops") {
            use map_model::brush_face::{ColorStop, GradientStops};
            let values = values.as_array().ok_or("stops must be an array")?;
            let stops = values
                .iter()
                .map(|value| {
                    Ok(ColorStop {
                        position: value
                            .get("position")
                            .and_then(Value::as_f64)
                            .ok_or("stop position is required")?
                            as f32,
                        color: rgb(value, "color")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Some(FaceColor::MultiGradient {
                stops: GradientStops::new(&stops)
                    .ok_or("provide 2 to 8 stops with positions between 0 and 1")?,
                radial: kind == "radial",
                angle,
                alpha: 255,
                ends,
            })
        } else if kind == "radial" || ends.is_some() {
            use map_model::brush_face::{ColorStop, GradientStops};
            Some(FaceColor::MultiGradient {
                stops: GradientStops::new(&[
                    ColorStop {
                        position: 0.0,
                        color: rgb(gradient, "from")?,
                    },
                    ColorStop {
                        position: 1.0,
                        color: rgb(gradient, "to")?,
                    },
                ])
                .unwrap(),
                radial: kind == "radial",
                angle,
                alpha: 255,
                ends,
            })
        } else {
            Some(FaceColor::gradient(
                rgb(gradient, "from")?,
                rgb(gradient, "to")?,
                angle,
            ))
        }
    } else if args.get("color").is_some() {
        Some(FaceColor::solid(rgb(args, "color")?))
    } else {
        None
    };
    let alpha = args
        .get("alpha")
        .or_else(|| args.get("opacity"))
        .and_then(Value::as_f64)
        .unwrap_or(255.0);
    let color = color.map(|color| color.with_alpha(alpha as u8));
    let changed = match (args.get("brush"), args.get("face")) {
        (Some(brush), Some(face)) => {
            let reference = FaceRef {
                node: map_model::node::NodeId(extensions::node_id(brush)?),
                face: face.as_u64().ok_or("invalid face")? as usize,
            };
            if document.face_at(reference).is_none() {
                return Err("no such brush face".into());
            }
            document.set_face_color_on(&[reference], color)
        }
        (None, None) => document.set_face_color(color),
        _ => return Err("provide both brush and face".into()),
    };
    if changed == 0 {
        return Err("nothing selected to paint".into());
    }
    Ok(format!("updated {changed} faces"))
}

/// Puts a material on faces of one brush: the face `face`, or every face whose normal
/// points along `facing`, so a script can give a floor its turf on top alone.
fn set_face_material(document: &mut Document, args: &Value) -> Result<String, String> {
    let material = string_or(args, "material", "");
    if material.is_empty() {
        return Err("`material` is required".into());
    }
    let faces = faces_of_brush(document, args)?;
    document.set_current_material(material);
    let changed = document.apply_current_material_on(&faces);
    Ok(format!("put {material} on {changed} faces"))
}

/// Sets the Quake 2 style content flags, surface flags and value of faces of one brush,
/// and Daikatana's light colour, picked as `set_face_material` picks them. A Daikatana
/// `.wal` gives its faces no flags, so a sky or a light panel there needs them on the
/// face. `reset` takes a face's own away again.
fn set_face_flags(document: &mut Document, args: &Value) -> Result<String, String> {
    let faces = faces_of_brush(document, args)?;
    let number = |key: &str| -> Result<i32, String> {
        match args.get(key) {
            None => Ok(0),
            Some(value) => value
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or(format!("`{key}` must be a 32-bit integer")),
        }
    };
    let surface = if args.get("reset").and_then(Value::as_bool) == Some(true) {
        None
    } else {
        let color = match args.get("color") {
            None => None,
            Some(value) => {
                let channels: Vec<u8> = value
                    .as_array()
                    .map(|array| {
                        array
                            .iter()
                            .filter_map(|n| n.as_u64().and_then(|n| u8::try_from(n).ok()))
                            .collect()
                    })
                    .unwrap_or_default();
                let [red, green, blue] = channels[..] else {
                    return Err("`color` is three integers from 0 to 255".into());
                };
                Some([red, green, blue])
            }
        };
        Some(map_model::uv::SurfaceAttributes {
            content_flags: number("content")?,
            surface_flags: number("surface")?,
            value: number("value")?,
            color,
        })
    };
    let changed = document.set_surface_attributes_on(&faces, surface);
    Ok(format!("set the flags of {changed} faces"))
}

/// The faces of `brush` a tool names: `face`, an index, or `facing`, a direction.
fn faces_of_brush(document: &Document, args: &Value) -> Result<Vec<FaceRef>, String> {
    let node = map_model::node::NodeId(extensions::node_id(
        args.get("brush").ok_or("`brush` is required")?,
    )?);
    let faces: Vec<FaceRef> = match (args.get("face"), args.get("facing")) {
        (Some(face), None) => vec![FaceRef {
            node,
            face: face.as_u64().ok_or("invalid face")? as usize,
        }],
        (None, Some(_)) => {
            let facing = vec3(args, "facing")?;
            let length = geometry_math::vec::length(facing);
            if length.is_nan() || length <= 1e-9 {
                return Err("`facing` must not be zero".into());
            }
            let facing = geometry_math::vec::normalize(facing);
            let mut found = Vec::new();
            for (id, brush) in map_model::node::brushes_with_ids(document.tree_world()) {
                if id == node {
                    brush.for_each_geometry_face(|index, face, _| {
                        if geometry_math::vec::dot(face.plane.normal, facing) > 0.999 {
                            found.push(FaceRef { node, face: index });
                        }
                    });
                }
            }
            found
        }
        _ => return Err("give `face` or `facing`, not both".into()),
    };
    if faces.is_empty() || faces.iter().any(|face| document.face_at(*face).is_none()) {
        return Err("no such brush face".into());
    }
    Ok(faces)
}

/// Moves the selection onto a layer named by name, as the layer panel's Move Selection
/// does, adding the layer first if there is none of that name.
fn move_selection_to_layer(document: &mut Document, args: &Value) -> Result<String, String> {
    let name = string_or(args, "layer", "");
    if name.is_empty() {
        return Err("`layer` is required".into());
    }
    let index = match document
        .layers()
        .iter()
        .position(|layer| layer.name == name)
    {
        Some(index) => index,
        None => document.add_layer(name),
    };
    let moved = document.move_selection_to_layer(index);
    Ok(format!("moved {moved} objects to layer {name}"))
}

/// Writes the map in a classic dialect, as `Menu/File/Export/Map` does: Heron saves in
/// Valve 220, and a compiler such as Neverball's mapc reads only Quake 3 face lines.
fn export_map(document: &mut Document, args: &Value) -> Result<String, String> {
    let path = string_or(args, "path", "");
    if path.is_empty() {
        return Err("`path` is required".into());
    }
    // Quake 3's brush primitives are not a classic dialect: each face carries a texture
    // matrix worked out from the material's pixel size, which is what the export
    // window's "Quake 3 (brush primitives)" writes.
    if args
        .get("format")
        .and_then(Value::as_str)
        .is_some_and(|name| {
            ["quake3_brushdef", "brushdef"].contains(&name.trim().to_ascii_lowercase().as_str())
        })
    {
        document
            .export_quake3_map_to(path)
            .map_err(|error| format!("could not write {path}: {error}"))?;
        return Ok(extensions::with_warnings(
            format!("exported {path} as Quake 3 brush primitives"),
            &document.quake3_map_export_problems(),
        ));
    }
    let format = match args.get("format").and_then(Value::as_str) {
        Some(name) => name
            .parse::<map_model::map_format::MapFormat>()
            .map_err(|error| error.to_string())?,
        None => crate::games::current()
            .and_then(|game| game.formats.first().map(|format| format.format))
            .unwrap_or(document.format),
    };
    document
        .export_map_to(path, format)
        .map_err(|error| format!("could not write {path}: {error}"))?;
    let lost = document.what_a_dialect_would_drop(format);
    Ok(extensions::with_warnings(
        format!("exported {path} as {format}"),
        &lost,
    ))
}

fn scale_selection(document: &mut Document, args: &Value) -> Result<String, String> {
    let factors = vec3(args, "factors")?;
    if factors.v.iter().any(|value| value.abs() < 1e-12) {
        return Err("scale factors must be nonzero".into());
    }
    let Some(bounds) = document.selection_bounds() else {
        return Err("nothing selected".into());
    };
    let center = match args.get("center") {
        Some(_) => vec3(args, "center")?,
        None => bounds.center(),
    };
    let transform = geometry_math::mat_ext::translation_matrix(center)
        * geometry_math::mat_ext::scaling_matrix(factors)
        * geometry_math::mat_ext::translation_matrix(-center);
    if document.transform_selection(&transform) {
        Ok(format!("scaled the selection by {factors:?}"))
    } else {
        Err("the scale left nothing valid".into())
    }
}

fn render_preview(document: &Document, args: &Value) -> Result<String, String> {
    let path = string_or(args, "path", "");
    if path.is_empty() {
        return Err("`path` (a .png) is required".into());
    }
    let size = args.get("size").and_then(Value::as_i64).unwrap_or(512) as i32;
    let image = crate::assets::preview(&document.to_map_text(), size)
        .ok_or_else(|| "the map has no brushes to draw".to_string())?;
    image
        .save_png(std::path::Path::new(path))
        .map_err(|error| format!("could not write {path}: {error}"))?;
    Ok(format!("wrote {path} ({size}x{size})"))
}

// --------------------------------------------------------------------------------------
// Tool schemas
// --------------------------------------------------------------------------------------

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

fn vec3_schema(what: &str) -> Value {
    json!({ "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": what })
}

fn tool_definitions() -> Vec<Value> {
    let mut tools = vec![
        tool("new_map", "Start over with an empty map.", json!({}), &[]),
        tool(
            "open_map",
            "Load a .map file, replacing the current map.",
            json!({ "path": { "type": "string" } }),
            &["path"],
        ),
        tool(
            "add_shape",
            "Add a shape's brushes and select them. kind: cuboid, stairs, arch, cylinder, cone, sphere, icosphere, hollow_sphere, wedge. hollow_sphere uses radial thickness and brushes: multiples of four from 20 to 320, default 80.",
            json!({
                "kind": { "type": "string" },
                "min": vec3_schema("one corner of the bounding box"),
                "max": vec3_schema("the opposite corner"),
                "material": { "type": "string" },
                "axis": { "type": "string", "description": "x, y or z, for circular and stair shapes; the climb direction (x or y) of a wedge" },
                "sides": { "type": "integer", "description": "sides on a circular shape" },
                "accuracy": { "type": "integer", "minimum": 0, "maximum": 4, "description": "icosphere refinement" },
                "brushes": { "type": "integer", "minimum": 20, "maximum": 320, "multipleOf": 4, "description": "hollow_sphere convex cell count; default 80" },
                "ascending":{"type":"boolean"},
                "stair_layout":{"type":"string","enum":["straight","left","right","return"]},
                "landing":{"type":"number","minimum":0},"flight_width":{"type":"number","exclusiveMinimum":0},"slab_thickness":{"type":"number","minimum":0},
                "step_height": { "type": "number" },
                "hollow": { "type": "boolean" },
                "thickness": { "type": "number", "description": "wall thickness when hollow" },
                "rotate": vec3_schema("degrees to turn about the shape's centre, x then y then z"),
            }),
            &["kind", "min", "max"],
        ),
        tool(
            "add_point_entity",
            "Place a point entity, e.g. a light.",
            json!({
                "classname": { "type": "string" },
                "origin": vec3_schema("where it goes"),
                "properties": { "type": "object", "description": "extra key/value properties" },
            }),
            &["classname", "origin"],
        ),
        tool(
            "make_brush_entity",
            "Turn the selected brushes into a brush entity, e.g. func_door.",
            json!({ "classname": { "type": "string" } }),
            &["classname"],
        ),
        tool(
            "select",
            "Set the working selection. what: \"all\" or \"none\"; or pass ids.",
            json!({
                "what": { "type": "string" },
                "ids": { "type": "array", "items": { "type": "integer" } },
            }),
            &[],
        ),
        tool("omit_covered_faces","Mark fully covered static world faces as omitted from render export. Restricts changes to selected brushes when selected; excludes brush entities and transparent paints. Undoable.",json!({}),&[]),
        tool("set_face_export_hidden","Omit selected faces or one brush face from exported render meshes, preserving collision, texture and editability. hidden=false restores rendering.",json!({"hidden":{"type":"boolean"},"brush":{"type":"integer"},"face":{"type":"integer","minimum":0}}),&["hidden"]),
        tool(
            "set_surface_material",
            "Set PBR surface on selected faces or one brush face. Roughness and metallic 0..1, emission 0..8 in the albedo colour. reset clears the override. Defaults: 0.65, 0, 0.",
            json!({
                "roughness":{"type":"number","minimum":0,"maximum":1},
                "metallic":{"type":"number","minimum":0,"maximum":1},
                "emission":{"type":"number","minimum":0,"maximum":8},
                "reset":{"type":"boolean"},
                "brush":{"type":"integer"}, "face":{"type":"integer","minimum":0}
            }), &[],
        ),
        tool(
            "set_face_material",
            "Put a material on faces of one brush: `face` is an index from describe_brush, \
             or `facing` [x,y,z] takes every face whose normal points that way, such as \
             [0,0,1] for its top.",
            json!({
                "brush": { "type": "string", "description": "an id from list_brushes" },
                "face": { "type": "integer" },
                "facing": vec3_schema("a direction"),
                "material": { "type": "string" },
            }),
            &["brush", "material"],
        ),
        tool(
            "set_face_flags",
            "Give faces of one brush their own content flags, surface flags and value, \
             which the map writes after the UV and a Quake 2 style compiler uses in place \
             of the texture's, and optionally Daikatana's light colour. Faces are picked \
             as set_face_material picks them. Missing numbers are 0; reset removes the \
             face's own so it takes its texture's again.",
            json!({
                "brush": { "type": "string", "description": "an id from list_brushes" },
                "face": { "type": "integer" },
                "facing": vec3_schema("a direction"),
                "content": { "type": "integer" },
                "surface": { "type": "integer" },
                "value": { "type": "integer" },
                "color": {
                    "type": "array", "items": { "type": "integer" }, "minItems": 3, "maxItems": 3,
                    "description": "Daikatana's surface light colour, 0 to 255 each"
                },
                "reset": { "type": "boolean" },
            }),
            &["brush"],
        ),
        tool(
            "set_face_color",
            "Paint the selected faces (or one brush face). Pass color [r,g,b], or gradient {from,to,angle} or {type: linear/radial, stops: [{position: 0..1, color: [r,g,b]}], angle}. Use 2 to 8 stops. start and end [u, v] place the gradient on the face, as fractions of the face's bounds (0,0 one corner, 1,1 the other): a linear one runs from start to end, a radial one is centred on start and reaches its last stop at end; without them it fills the face along angle. Pass neither colour nor gradient to clear. Add alpha (0 to 255, default 255) to paint a see-through colour.",
            json!({
                "color": vec3_schema("r, g, b from 0 to 255"),
                "gradient": {
                    "type": "object",
                    "properties": {
                        "type": {"type":"string","enum":["linear","radial"]},
                        "stops": {"type":"array","minItems":2,"maxItems":8,"items":{
                            "type":"object","required":["position","color"],"additionalProperties":false,
                            "properties":{"position":{"type":"number","minimum":0,"maximum":1},
                            "color":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"number","minimum":0,"maximum":255}}}
                        }},
                        "from": vec3_schema("start colour"),
                        "to": vec3_schema("end colour"),
                        "angle": { "type": "number" },
                        "start": {"type":"array","minItems":2,"maxItems":2,"items":{"type":"number"}},
                        "end": {"type":"array","minItems":2,"maxItems":2,"items":{"type":"number"}},
                    },
                },
                "alpha": { "type": "number", "description": "opacity 0 to 255, default 255" },
                "brush": { "type": "integer" },
                "face": { "type": "integer" },
            }),
            &[],
        ),
        tool(
            "translate_selection",
            "Move the selection.",
            json!({ "delta": vec3_schema("how far to move") }),
            &["delta"],
        ),
        tool(
            "rotate_selection",
            "Turn the selection about an axis.",
            json!({
                "axis": vec3_schema("the axis to turn about"),
                "degrees": { "type": "number" },
                "center": vec3_schema("the pivot; the selection's middle by default"),
            }),
            &["axis", "degrees"],
        ),
        tool(
            "scale_selection",
            "Scale the selection.",
            json!({
                "factors": vec3_schema("scale on x, y, z"),
                "center": vec3_schema("the pivot; the selection's middle by default"),
            }),
            &["factors"],
        ),
        tool(
            "flip_selection",
            "Mirror the selection across an axis (x, y or z).",
            json!({ "axis": { "type": "string" } }),
            &["axis"],
        ),
        tool("csg_subtract", "Cut the selected brushes out of the ones they touch.", json!({}), &[]),
        tool("join_corner", "Join exactly two selected axis-aligned rectangular L members with equal depth. Optional outer rounding removes volume; inner rounding adds convex brushes under their common parent. Each uses radius and segments (default 8, max 64). Concentric mode requires equal widths and both roundings; derives outer radius as inner radius plus width and uses inner segments for both arcs. Wedges mode requires both roundings and a common parent, uses inner radius/segments, and builds full-thickness convex segments between a circular inner arc and an elliptical outer arc for unequal widths (concentric circles for equal widths). preserve_thickness offsets outer facets to maintain perpendicular thickness, interpolating unequal widths. Contact points must fit strictly within both arms. dry_run defaults true; false applies one Undo. expected_revision protects the preview.", json!({"dry_run":{"type":"boolean"},"expected_revision":{"type":"integer","minimum":0},"outer":{"type":"object","required":["radius"],"properties":{"radius":{"type":"number","exclusiveMinimum":0},"segments":{"type":"integer","minimum":1,"maximum":64}}},"inner":{"type":"object","required":["radius"],"properties":{"radius":{"type":"number","exclusiveMinimum":0},"segments":{"type":"integer","minimum":1,"maximum":64}}},"concentric":{"type":"boolean"},"wedges":{"type":"boolean"},"preserve_thickness":{"type":"boolean"}}), &[]),
        tool("csg_merge", "Merge the overlapping selected brushes into one.", json!({}), &[]),
        tool("csg_intersect", "Keep only where the selected brushes overlap.", json!({}), &[]),
        tool(
            "hollow",
            "Atomically hollow selected brushes in place. direction: inward/outward/centered. Open selected faces or join a convex pair under one parent to omit their common wall.",
            json!({"thickness":{"type":"number"},"direction":{"type":"string","enum":["inward","outward","centered"]},"open_selected_faces":{"type":"boolean"},"join_convex":{"type":"boolean"}}),
            &[],
        ),
        tool(
            "group_selection",
            "Group the selection.",
            json!({ "name": { "type": "string" } }),
            &[],
        ),
        tool("ungroup_selection", "Take the selected groups apart.", json!({}), &[]),
        tool(
            "move_selection_to_layer",
            "Move the selection onto the layer of that name, adding the layer if the map \
             has none by that name.",
            json!({ "layer": { "type": "string" } }),
            &["layer"],
        ),
        tool("delete_selection", "Delete the selection.", json!({}), &[]),
        tool("undo", "Undo the last change.", json!({}), &[]),
        tool("redo", "Redo the last undone change.", json!({}), &[]),
        tool("list_brushes", "List every brush with its id and bounds.", json!({}), &[]),
        tool(
            "get_selection",
            "The ids of what is selected. Connected to the open window (Serve for an agent), \
             this is what the person selected: read it when they say \"this\" or \"the selected\".",
            json!({}),
            &[],
        ),
        tool(
            "entity_graph",
            "Which entity fires which: every entity that has a name others aim at or aims at \
             one (target, killtarget, and the FGD's target keys), and every link, with ids.",
            json!({}),
            &[],
        ),
        tool(
            "link_entities",
            "Make the entity `from` aim at the entity `to` (one undo step): `to` keeps its \
             targetname or is given one from its class, and `from` takes it under its next free \
             target key.",
            json!({ "from": { "type": "string", "description": "an entity id" }, "to": { "type": "string", "description": "an entity id" } }),
            &["from", "to"],
        ),
        tool(
            "describe_selection",
            "What is selected, in detail: each brush with its entity, layer, bounds, size and \
             materials; each entity with its class, name, origin and brush count; groups and patches.",
            json!({}),
            &[],
        ),
        tool(
            "describe_brush",
            "One brush face by face: material, paint, normal and corners of each face. \
             The face indices are the ones set_face_color takes.",
            json!({ "brush": { "type": "integer", "description": "an id from list_brushes" } }),
            &["brush"],
        ),
        tool(
            "load_definitions",
            "Load entity definitions from a .fgd or .def file.",
            json!({ "path": { "type": "string" } }),
            &["path"],
        ),
        tool(
            "list_entity_classes",
            "The entity classes the loaded definitions offer. Without arguments, every \
             placeable class in brief; pass classname for one class in full, its \
             properties included.",
            json!({ "classname": { "type": "string" } }),
            &[],
        ),
        tool(
            "load_materials",
            "Load materials from a directory of images (recursively).",
            json!({ "path": { "type": "string" } }),
            &["path"],
        ),
        tool(
            "load_wad",
            "Load the textures in a Quake .wad and name it in worldspawn's wad property, \
             which is where qbsp finds them. Relative to the map's folder once saved.",
            json!({ "path": { "type": "string" } }),
            &["path"],
        ),
        tool(
            "set_entity_property",
            "Set one property on an entity: `entity` is an id, or \"worldspawn\" for the \
             map's own keys (message, wad, worldtype); without it, every selected entity. \
             A null `value` removes the property.",
            json!({
                "entity": { "type": "string" },
                "key": { "type": "string" },
                "value": { "type": ["string", "null"] },
            }),
            &["key"],
        ),
        tool(
            "list_materials",
            "The loaded materials: name and pixel size of each.",
            json!({}),
            &[],
        ),
        tool("get_map", "Return the whole map as .map text.", json!({}), &[]),
        tool(
            "save",
            "Write the map to a .map file.",
            json!({ "path": { "type": "string" } }),
            &["path"],
        ),
        tool(
            "export_map",
            "Write a copy of the map in a classic .map dialect for a game's compiler; the \
             open map is not changed. standard, valve, quake2 (a face without flags \
             keeps its texture's), quake2_valve, quake3 (flags on every face, which \
             Neverball's mapc needs), hexen2 (Raven's qbsp) or quake3_brushdef (Quake 3 \
             brush primitives, as GtkRadiant writes them). Without a format, the first \
             one the current game lists.",
            json!({
                "path": { "type": "string" },
                "format": { "type": "string", "description": "standard, valve, quake2, quake2_valve, quake3, hexen2 or quake3_brushdef, in any case" },
            }),
            &["path"],
        ),
        tool(
            "export",
            "Export the map for an engine: godot, defold, three.js, babylon, playcanvas, bevy, raylib, unity, unreal, flax, stride, cocos, gdevelop, obj, gltf, glb.",
            json!({
                "engine": { "type": "string" },
                "path": { "type": "string", "description": "the output directory" },
                "name": { "type": "string" },
                "scale": { "type": "number", "description": "map units per engine unit" },
            }),
            &["engine", "path"],
        ),
        tool(
            "render_preview",
            "Draw a shaded preview of the map to a PNG.",
            json!({ "path": { "type": "string" }, "size": { "type": "integer" } }),
            &["path"],
        ),
        tool(
            "host_session",
            "Open the current map for live editing so a person in the GUI (or another \
             agent) can join and edit it with you. Returns a token or address to hand \
             over. Your edits and theirs sync from then on.",
            json!({
                "transport": { "type": "string", "description": "\"iroh\" (default) or \"tcp\"" },
                "port": { "type": "integer", "description": "TCP port, default 27960" },
                "nickname": { "type": "string", "description": "the name the other person sees for you" },
            }),
            &[],
        ),
        tool(
            "join_session",
            "Join a live session someone else is hosting and work on their map. \
             `target` is the iroh token they gave you or a host:port TCP address.",
            json!({
                "target": { "type": "string" },
                "nickname": { "type": "string", "description": "the name the other person sees for you" },
            }),
            &["target"],
        ),
        tool(
            "leave_session",
            "Leave the live session. Your local map stays as it is.",
            json!({}),
            &[],
        ),
        tool(
            "session_status",
            "Report whether a live session is running, who the peer is, and what they \
             have selected right now.",
            json!({}),
            &[],
        ),
    ];
    tools.extend(extensions::definitions());
    tools.extend(animation::definitions());
    tools.extend(settings::definitions());
    extensions::tighten_schemas(&mut tools);
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `call_tool` with a throwaway collab state, for the tests that do not use a session.
    fn call(document: &mut Document, name: &str, args: &Value) -> Result<String, String> {
        call_tool(document, &mut Collab::default(), name, args)
    }

    /// A new map is set up for the game in the preferences, as the window sets it up: a
    /// light placed in a Paintball 2 map is the game's `light`, not Heron's own one with
    /// its Godot keys, and the game's classes are on offer.
    #[test]
    fn a_new_map_takes_the_game_s_entity_definitions() {
        let original = crate::preferences::current();
        let mut preferences = original.clone();
        preferences.game = "Digital Paint: Paintball 2".to_string();
        crate::preferences::set_current(preferences);

        let mut document = Document::new(WorldBounds::default());
        call(&mut document, "new_map", &json!({})).expect("new map");
        call(
            &mut document,
            "add_point_entity",
            &json!({"classname":"light","origin":[0,0,64],"properties":{"light":"300"}}),
        )
        .expect("light");
        let map = call(&mut document, "get_map", &json!({})).expect("map");
        let classes = call(&mut document, "list_entity_classes", &json!({})).expect("classes");
        crate::preferences::set_current(original);

        assert!(map.contains("\"light\" \"300\""), "{map}");
        assert!(!map.contains("light_energy"), "{map}");
        assert!(!map.contains("omni_range"), "{map}");
        assert!(classes.contains("info_player_deathmatch"), "{classes}");
        assert!(classes.contains("flag"), "{classes}");
    }

    /// The person's selection reaches the agent: ids, then the brushes in detail, and the
    /// text the inspector copies names the same brushes.
    #[test]
    fn the_selection_is_read_and_described() {
        let mut document = Document::new(WorldBounds::default());
        let empty: Value =
            serde_json::from_str(&call(&mut document, "get_selection", &json!({})).unwrap())
                .unwrap();
        assert_eq!(empty["count"], 0);
        call(
            &mut document,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[64,32,128]}),
        )
        .unwrap();
        let read: Value =
            serde_json::from_str(&call(&mut document, "get_selection", &json!({})).unwrap())
                .unwrap();
        assert_eq!(read["count"], 1);
        let id = read["ids"][0].as_str().unwrap().to_string();
        let described: Value =
            serde_json::from_str(&call(&mut document, "describe_selection", &json!({})).unwrap())
                .unwrap();
        let brush = &described["brushes"][0];
        assert_eq!(brush["id"], id);
        assert_eq!(brush["size"], json!([64.0, 32.0, 128.0]));
        assert_eq!(brush["entity"]["classname"], "worldspawn");
        assert_eq!(brush["faces"], 6);
        let text = crate::selection_report::text(&document);
        assert!(
            text.contains(&format!("brush {id}")) && text.contains("64 x 32 x 128"),
            "{text}"
        );
    }

    #[test]
    fn multicolor_gradient_can_be_painted_described_and_undone() {
        let mut document = Document::new(WorldBounds::default());
        call(
            &mut document,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[64,64,64]}),
        )
        .unwrap();
        let original = document.preview_mesh(&Default::default());
        for kind in ["linear", "radial"] {
            call(&mut document, "set_face_color", &json!({"gradient":{"type":kind,"angle":25,
                "stops":[{"position":0,"color":[255,0,0]},{"position":0.5,"color":[0,255,0]},{"position":1,"color":[0,0,255]}]},"alpha":123})).unwrap();
            let mesh = document.preview_mesh(&Default::default());
            assert!(mesh
                .triangles
                .iter()
                .all(|t| t.corner_colors.is_some() && (t.alpha - 123.0 / 255.0).abs() < 1e-6));
            let id = document.selectable_brushes()[0].0;
            let result = call(
                &mut document,
                "describe_brush",
                &json!({"brush":id.0.to_string()}),
            )
            .unwrap();
            let value: Value = serde_json::from_str(&result).unwrap();
            assert_eq!(value["faces"][0]["paint"]["gradient"]["type"], kind);
            document.clear_selection();
            assert!(document
                .preview_mesh(&Default::default())
                .triangles
                .iter()
                .all(|t| !t.selected));
            document.undo();
            assert_eq!(
                document.preview_mesh(&Default::default()).triangles.len(),
                original.triangles.len()
            );
            document.select_all();
        }
    }

    #[test]
    fn the_live_endpoint_answers_and_refuses_collab() {
        let mut document = Document::new(WorldBounds::default());
        let mut collab = Collab::default();
        let reply = respond(
            &mut document,
            &mut collab,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"add_shape","arguments":{"kind":"cuboid","min":[0,0,0],"max":[64,64,64]}}}"#,
            false,
        )
        .expect("a reply");
        assert!(reply.to_string().contains("added 1 brushes"));
        assert_eq!(
            document.selection_count(),
            1,
            "the call landed in the document"
        );

        // The window owns its session; the endpoint says so instead of fighting it.
        let refused = respond(
            &mut document,
            &mut collab,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"host_session","arguments":{}}}"#,
            false,
        )
        .expect("a reply");
        assert!(refused.to_string().contains("owns its session"));

        // Blank lines and notifications want no reply at all.
        assert!(respond(&mut document, &mut collab, "  ", false).is_none());
        assert!(respond(
            &mut document,
            &mut collab,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            false
        )
        .is_none());
    }

    #[test]
    fn a_rotated_wedge_goes_in_as_one_undo_step() {
        let mut document = Document::new(WorldBounds::default());
        let added = call(
            &mut document,
            "add_shape",
            &json!({
                "kind": "wedge",
                "min": [0, 0, 0],
                "max": [128, 64, 64],
                "axis": "x",
                "rotate": [0, 0, 90],
            }),
        )
        .expect("the wedge is added");
        assert!(added.contains("added 1 brushes"));

        // A wedge is a box halved: five faces, six corners.
        let described = {
            let listed = call(&mut document, "list_brushes", &json!({})).expect("list");
            let ids: Value = serde_json::from_str(&listed).expect("json");
            let id = extensions::node_id(&ids[0]["id"]).expect("an id");
            call(
                &mut document,
                "describe_brush",
                &json!({ "brush": id.to_string() }),
            )
            .expect("describe")
        };
        let brush: Value = serde_json::from_str(&described).expect("json");
        assert_eq!(brush["faces"].as_array().expect("faces").len(), 5);
        // The quarter turn about z swaps the footprint's sides: 128 along x
        // became 128 along y.
        let (min, max) = (&brush["min"], &brush["max"]);
        let extent = |axis: usize| max[axis].as_f64().unwrap() - min[axis].as_f64().unwrap();
        assert_eq!(extent(0).round(), 64.0);
        assert_eq!(extent(1).round(), 128.0);

        // The add and the turn undo together.
        call(&mut document, "undo", &json!({})).expect("undo");
        let listed = call(&mut document, "list_brushes", &json!({})).expect("list");
        let ids: Value = serde_json::from_str(&listed).expect("json");
        assert_eq!(ids.as_array().expect("array").len(), 0);
    }

    #[test]
    fn entity_classes_come_back_out_of_a_loaded_fgd() {
        let mut document = Document::new(WorldBounds::default());

        // The common catalogue is available before loading a project FGD.
        let initial = call(&mut document, "list_entity_classes", &json!({})).unwrap();
        assert!(initial.contains("trigger_multiple"));

        let fgd = "@PointClass size(-8 -8 -8, 8 8 8) color(255 255 40) = light : \"A light\" [\n\
                   \tlight(integer) : \"Brightness\" : \"300\"\n\
                   ]\n\
                   @SolidClass = func_door : \"A sliding door\" []\n";
        let path = std::env::temp_dir().join("heron-mcp-classes-test.fgd");
        std::fs::write(&path, fgd).expect("the test fgd is written");
        let loaded = call(
            &mut document,
            "load_definitions",
            &json!({ "path": path.to_string_lossy() }),
        )
        .expect("the fgd loads");
        assert!(loaded.contains("entity classes"));

        let listed = call(&mut document, "list_entity_classes", &json!({})).expect("classes list");
        let classes: Value = serde_json::from_str(&listed).expect("the list is JSON");
        let classes = classes.as_array().expect("an array");
        assert!(classes.len() > 2);
        assert_eq!(
            classes.iter().find(|c| c["classname"] == "light").unwrap()["kind"],
            "point"
        );
        assert_eq!(
            classes
                .iter()
                .find(|c| c["classname"] == "func_door")
                .unwrap()["kind"],
            "brush"
        );
        assert!(classes.iter().any(|c| c["classname"] == "trigger_multiple"));

        let one = call(
            &mut document,
            "list_entity_classes",
            &json!({ "classname": "light" }),
        )
        .expect("one class in full");
        let class: Value = serde_json::from_str(&one).expect("the class is JSON");
        assert_eq!(class["properties"][0]["key"], "light");
        assert_eq!(class["properties"][0]["default"], "300");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_brush_describes_its_faces_with_material_paint_and_corners() {
        let mut document = Document::new(WorldBounds::default());
        call(
            &mut document,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [0, 0, 0], "max": [64, 64, 64], "material": "stone" }),
        )
        .expect("the cuboid is added");
        call(
            &mut document,
            "set_face_color",
            &json!({ "color": [10, 20, 30] }),
        )
        .expect("the faces are painted");

        let listed = call(&mut document, "list_brushes", &json!({})).expect("brushes list");
        let ids: Vec<Value> = serde_json::from_str(&listed).expect("the list is JSON");
        let id = extensions::node_id(&ids[0]["id"]).expect("an id");

        let described = call(
            &mut document,
            "describe_brush",
            &json!({ "brush": id.to_string() }),
        )
        .expect("the brush describes itself");
        let brush: Value = serde_json::from_str(&described).expect("the description is JSON");
        let faces = brush["faces"].as_array().expect("faces");
        assert_eq!(faces.len(), 6);
        assert_eq!(faces[0]["material"], "stone");
        assert_eq!(faces[0]["paint"]["color"], json!([10, 20, 30]));
        assert_eq!(faces[0]["vertices"].as_array().expect("corners").len(), 4);

        let missing = call(&mut document, "describe_brush", &json!({ "brush": 424242 }));
        assert!(missing.is_err());
    }

    #[test]
    fn a_shape_a_colour_and_a_group_go_in_through_the_tools() {
        let mut document = Document::new(WorldBounds::default());

        let added = call(
            &mut document,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [-32, -32, 0], "max": [32, 32, 32] }),
        )
        .expect("the cuboid is added");
        assert!(added.contains("added 1 brushes"));
        assert_eq!(document.selection_count(), 1);

        let painted = call(
            &mut document,
            "set_face_color",
            &json!({ "color": [200, 40, 40] }),
        )
        .expect("the faces are painted");
        assert!(painted.contains("updated 6 faces"));

        call(
            &mut document,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [0, 0, 32], "max": [16, 16, 64] }),
        )
        .expect("a second cuboid");
        call(&mut document, "select", &json!({ "what": "all" })).expect("select all");
        let grouped = call(
            &mut document,
            "group_selection",
            &json!({ "name": "thing" }),
        )
        .expect("the group is made");
        assert!(grouped.contains("grouped the selection"));
        let moved = call(
            &mut document,
            "move_selection_to_layer",
            &json!({ "layer": "Upper" }),
        )
        .expect("the grouped brushes move");
        assert_eq!(moved, "moved 2 objects to layer Upper");
        assert_eq!(document.layers()[1].name, "Upper");
        assert_eq!(document.layers()[1].objects, 2);

        let map = call(&mut document, "get_map", &json!({})).expect("the map comes back");
        assert!(
            map.contains("// heron_brush color 200 40 40"),
            "the colour rode along"
        );
    }

    #[test]
    fn a_tool_that_needs_an_argument_says_so_rather_than_panicking() {
        let mut document = Document::new(WorldBounds::default());
        let error = call(&mut document, "add_shape", &json!({ "kind": "cuboid" }))
            .expect_err("min and max are missing");
        assert!(error.contains("min"));
    }

    #[test]
    #[ignore = "spawns OS threads and binds a socket; run with --ignored"]
    fn two_agents_edit_one_map_through_a_session() {
        use web_time::{Duration, Instant};

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let mut host_doc = Document::new(WorldBounds::default());
        let mut host = Collab::default();
        call_tool(
            &mut host_doc,
            &mut host,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [-16, -16, -16], "max": [16, 16, 16] }),
        )
        .unwrap();
        call_tool(
            &mut host_doc,
            &mut host,
            "host_session",
            &json!({ "transport": "tcp", "port": port }),
        )
        .unwrap();

        let mut guest_doc = Document::new(WorldBounds::default());
        let mut guest = Collab::default();
        // The host's listener binds on a thread, so the first join may beat it.
        let target = json!({ "target": format!("127.0.0.1:{port}") });
        let joined = Instant::now() + Duration::from_secs(3);
        loop {
            match call_tool(&mut guest_doc, &mut guest, "join_session", &target) {
                Ok(_) => break,
                Err(_) if Instant::now() < joined => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("guest could not join: {e}"),
            }
        }

        // `serve` runs this pump around every request; the test drives it directly.
        let pump = |doc: &mut Document, c: &mut Collab| {
            c.check_pending_host(doc);
            c.absorb(doc);
        };

        // The host finishes the handshake and pushes its world.
        let deadline = Instant::now() + Duration::from_secs(3);
        while brush_list(&guest_doc).is_empty() && Instant::now() < deadline {
            pump(&mut host_doc, &mut host);
            pump(&mut guest_doc, &mut guest);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            brush_list(&guest_doc).len(),
            1,
            "the guest adopted the host's box"
        );

        // The guest adds a box; the host takes it in.
        call_tool(
            &mut guest_doc,
            &mut guest,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [64, 64, 64], "max": [96, 96, 96] }),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while brush_list(&host_doc).len() < 2 && Instant::now() < deadline {
            pump(&mut host_doc, &mut host);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            brush_list(&host_doc).len(),
            2,
            "the host saw the guest's box"
        );
    }

    /// A Daikatana sky: the `.wal` has no flags, so the lid's underside gets sky and
    /// light with Daikatana's light colour, and the export writes them on its line.
    #[test]
    fn a_face_gets_its_own_flags_and_daikatana_colour() {
        let mut document = Document::new(WorldBounds::default());
        let added = call(
            &mut document,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,192],"max":[128,128,208],"material":"e1m1/sky2a"}),
        )
        .expect("a lid");
        let brush: Vec<String> =
            serde_json::from_str(added.split("selected: ").nth(1).expect("ids")).expect("ids");
        let done = call(
            &mut document,
            "set_face_flags",
            &json!({"brush": brush[0], "facing": [0, 0, -1], "surface": 5, "value": 600,
                    "color": [255, 255, 204]}),
        )
        .expect("the underside");
        assert!(done.contains("of 1 faces"), "{done}");
        for wrong in [
            json!({"brush": brush[0], "face": 0, "color": [1, 2]}),
            json!({"brush": brush[0], "face": 0, "color": [1, 2, 256]}),
            json!({"brush": brush[0], "face": 0, "value": 1.5}),
            json!({"brush": brush[0], "face": 99}),
        ] {
            assert!(
                call(&mut document, "set_face_flags", &wrong).is_err(),
                "{wrong}"
            );
        }
        let surfaces = |document: &Document| -> Vec<_> {
            map_model::node::brushes_with_ids(document.tree_world())
                .into_iter()
                .flat_map(|(_, brush)| brush.faces().to_vec())
                .filter_map(|face| face.surface_attributes)
                .collect()
        };
        assert_eq!(
            surfaces(&document),
            [map_model::uv::SurfaceAttributes {
                content_flags: 0,
                surface_flags: 5,
                value: 600,
                color: Some([255, 255, 204]),
            }]
        );

        let directory =
            std::env::temp_dir().join(format!("heron-mcp-daikatana-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a directory");
        let path = directory.join("sky.map");
        call(
            &mut document,
            "export_map",
            &json!({"path": path.to_string_lossy(), "format": "quake2"}),
        )
        .expect("exported");
        let text = std::fs::read_to_string(&path).expect("the export");
        let _ = std::fs::remove_dir_all(&directory);
        let flagged: Vec<&str> = text.lines().filter(|l| l.contains(" 0 5 600 ")).collect();
        assert_eq!(flagged.len(), 1, "{text}");
        assert!(flagged[0].ends_with(" 0 5 600 255 255 204"), "{text}");

        call(
            &mut document,
            "set_face_flags",
            &json!({"brush": brush[0], "facing": [0, 0, -1], "reset": true}),
        )
        .expect("reset");
        assert!(surfaces(&document).is_empty());
    }

    /// A Neverball floor: turf on top, grey on the sides, exported in the face lines
    /// mapc reads.
    #[test]
    fn a_floor_gets_its_top_material_and_exports_for_mapc() {
        let mut document = Document::new(WorldBounds::default());
        let added = call(
            &mut document,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,-16],"max":[128,128,0],"material":"mtrl/turf-grey"}),
        )
        .expect("a slab");
        let brush: Vec<String> =
            serde_json::from_str(added.split("selected: ").nth(1).expect("ids")).expect("ids");
        let done = call(
            &mut document,
            "set_face_material",
            &json!({"brush": brush[0], "facing": [0, 0, 2], "material": "mtrl/turf-green"}),
        )
        .expect("the top");
        assert!(done.contains("on 1 faces"), "{done}");
        call(
            &mut document,
            "set_face_material",
            &json!({"brush": brush[0], "face": 0, "material": "mtrl/goal"}),
        )
        .expect("by index");
        for wrong in [
            json!({"brush": brush[0], "facing": [0, 0, 0], "material": "x"}),
            json!({"brush": brush[0], "face": 99, "material": "x"}),
            json!({"brush": brush[0], "face": 0, "facing": [0, 0, 1], "material": "x"}),
            json!({"brush": brush[0], "face": 0}),
        ] {
            assert!(
                call(&mut document, "set_face_material", &wrong).is_err(),
                "{wrong}"
            );
        }

        let directory =
            std::env::temp_dir().join(format!("heron-mcp-neverball-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a directory");
        let path = directory.join("level.map");
        let done = call(
            &mut document,
            "export_map",
            &json!({"path": path, "format": "quake3"}),
        )
        .expect("exported");
        assert!(done.ends_with("as Quake3"), "{done}");
        let text = std::fs::read_to_string(&path).expect("written");
        let faces: Vec<&str> = text.lines().filter(|l| l.starts_with('(')).collect();
        assert_eq!(faces.len(), 6, "{text}");
        assert_eq!(
            faces
                .iter()
                .filter(|l| l.contains(" mtrl/turf-green 0 0 0 1 1 0 0 0"))
                .count(),
            1,
            "{text}"
        );
        assert!(faces.iter().any(|l| l.contains(" mtrl/goal ")), "{text}");
        assert!(!text.contains("mapversion"), "{text}");
        assert!(call(
            &mut document,
            "export_map",
            &json!({"path": path, "format": "hexen3"})
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A copy in Hexen II's dialect, for Raven's qbsp, leaves the open map as it was.
    #[test]
    fn a_map_is_exported_in_another_dialect() {
        let mut document = Document::new(WorldBounds::default());
        let directory =
            std::env::temp_dir().join(format!("heron-mcp-dialect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory is made");
        call(
            &mut document,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [0, 0, 0], "max": [64, 64, 64], "material": "rtex225" }),
        )
        .expect("a box");
        let path = directory.join("raven.map");
        let format = document.format;
        let reply = call(
            &mut document,
            "export_map",
            &json!({ "path": path, "format": "Hexen2" }),
        )
        .expect("export");
        assert!(reply.contains("as Hexen2"), "{reply}");
        let text = std::fs::read_to_string(&path).expect("it was written");
        assert!(text.contains("// Format: Hexen2"), "{text}");
        assert!(!text.contains("mapversion"), "{text}");
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with('(') && line.contains(" rtex225 0 0 0 1 1 0"))
                .count(),
            6,
            "{text}"
        );
        assert_eq!(document.format, format, "the open map keeps its dialect");
        assert!(call(
            &mut document,
            "export_map",
            &json!({ "path": path, "format": "Hexen3" })
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// Quake 3's brush primitives, which need each material's pixel size.
    #[test]
    fn a_map_is_exported_as_quake3_brush_primitives() {
        let mut document = Document::new(WorldBounds::default());
        let directory =
            std::env::temp_dir().join(format!("heron-mcp-brushdef-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("base")).expect("the directory is made");
        image::RgbaImage::from_pixel(64, 32, image::Rgba([90, 90, 90, 255]))
            .save(directory.join("base/wall.png"))
            .expect("the texture");
        call(
            &mut document,
            "add_shape",
            &json!({ "kind": "cuboid", "min": [0, 0, 0], "max": [64, 64, 64], "material": "base/wall" }),
        )
        .expect("a box");
        let path = directory.join("q3.map");
        // Without the texture there is no pixel size to build the matrix from.
        assert!(call(
            &mut document,
            "export_map",
            &json!({ "path": path, "format": "quake3_brushdef" })
        )
        .is_err());
        call(
            &mut document,
            "load_materials",
            &json!({ "path": directory }),
        )
        .expect("the textures");
        let reply = call(
            &mut document,
            "export_map",
            &json!({ "path": path, "format": "Quake3_brushDef" }),
        )
        .expect("export");
        assert!(reply.contains("brush primitives"), "{reply}");
        let text = std::fs::read_to_string(&path).expect("it was written");
        assert_eq!(text.matches("brushDef").count(), 1, "{text}");
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with('(') && line.contains(" ) ) \"base/wall\" 0 0 0"))
                .count(),
            6,
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A Quake map's keys: worldspawn's, a brush entity's, and the wad qbsp reads.
    #[test]
    fn entity_properties_and_wads_are_set_for_a_quake_map() {
        let mut document = Document::new(WorldBounds::default());
        let directory =
            std::env::temp_dir().join(format!("heron-mcp-quake-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let map = directory.join("level.map");
        crate::document::quake_tests::write_wad(&directory.join("tex.wad"), "door1");
        call(&mut document, "save", &json!({ "path": map })).expect("save");
        let loaded = call(
            &mut document,
            "load_wad",
            &json!({ "path": directory.join("tex.wad") }),
        )
        .expect("load the wad");
        assert!(loaded.contains("\"tex.wad\""), "{loaded}");
        assert!(call(
            &mut document,
            "load_wad",
            &json!({ "path": "/nowhere.wad" })
        )
        .is_err());

        call(
            &mut document,
            "set_entity_property",
            &json!({ "entity": "worldspawn", "key": "message", "value": "Hi" }),
        )
        .expect("worldspawn");
        assert!(
            call(
                &mut document,
                "set_entity_property",
                &json!({ "key": "a", "value": "b" })
            )
            .is_err(),
            "nothing selected and no entity named"
        );
        call(
            &mut document,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[16,64,128],"material":"door1"}),
        )
        .expect("a brush");
        let made = call(
            &mut document,
            "make_brush_entity",
            &json!({"classname":"func_door"}),
        )
        .expect("a door");
        let id = made
            .rsplit("id ")
            .next()
            .expect("an id")
            .trim_end_matches(')');
        call(
            &mut document,
            "set_entity_property",
            &json!({ "entity": id, "key": "targetname", "value": "door1" }),
        )
        .expect("by id");
        call(
            &mut document,
            "set_entity_property",
            &json!({ "key": "angle", "value": "-1" }),
        )
        .expect("the selected door");
        let text = call(&mut document, "get_map", &json!({})).expect("the map");
        for line in [
            "\"wad\" \"tex.wad\"",
            "\"message\" \"Hi\"",
            "\"targetname\" \"door1\"",
            "\"angle\" \"-1\"",
        ] {
            assert!(text.contains(line), "{line} in {text}");
        }
        call(
            &mut document,
            "set_entity_property",
            &json!({ "entity": id, "key": "angle", "value": null }),
        )
        .expect("removed");
        let text = call(&mut document, "get_map", &json!({})).expect("the map");
        assert!(!text.contains("\"angle\""));
    }

    #[test]
    fn the_tool_list_is_well_formed() {
        for tool in tool_definitions() {
            assert!(tool.get("name").and_then(Value::as_str).is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }
}

#[cfg(test)]
mod corner_join_tests {
    use super::*;
    #[test]
    fn corner_mcp_defaults_to_preview_and_checks_revision() {
        let mut d = Document::new(Default::default());
        let mut c = Collab::default();
        for max in [[128, 16, 8], [24, 96, 8]] {
            call_tool(
                &mut d,
                &mut c,
                "add_shape",
                &json!({"kind":"cuboid","min":[0,0,0],"max":max}),
            )
            .unwrap();
        }
        d.select_all();
        let before = d.to_map_text();
        let rev = d.revision();
        let preview = call_tool(&mut d, &mut c, "join_corner", &json!({})).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&preview).unwrap()["dry_run"],
            true
        );
        assert_eq!(d.to_map_text(), before);
        call_tool(
            &mut d,
            &mut c,
            "join_corner",
            &json!({"dry_run":false,"expected_revision":rev}),
        )
        .unwrap();
        let after = d.to_map_text();
        assert_ne!(before, after);
        assert!(call_tool(
            &mut d,
            &mut c,
            "join_corner",
            &json!({"dry_run":false,"expected_revision":rev})
        )
        .is_err());
        assert_eq!(d.to_map_text(), after);
        d.undo();
        assert_eq!(d.to_map_text(), before);
    }
    #[test]
    fn corner_mcp_inner_concentric_and_wedge_rounding_are_atomic() {
        for (wedges, preserve_thickness) in [(false, false), (true, false), (true, true)] {
            let mut d = Document::new(Default::default());
            let mut c = Collab::default();
            let dimensions = if wedges {
                [[128, 8, 8], [24, 96, 8]]
            } else {
                [[128, 16, 8], [16, 96, 8]]
            };
            for max in dimensions {
                call_tool(
                    &mut d,
                    &mut c,
                    "add_shape",
                    &json!({"kind":"cuboid","min":[0,0,0],"max":max}),
                )
                .unwrap();
            }
            d.select_all();
            let before = d.to_map_text();
            let revision = d.revision();
            let options = json!({"outer":{"radius":8},"inner":{"radius":48,"segments":8},"concentric":!wedges,"wedges":wedges,"preserve_thickness":preserve_thickness});
            let response = call_tool(&mut d, &mut c, "join_corner", &options).unwrap();
            let response: Value = serde_json::from_str(&response).unwrap();
            assert_eq!(response["brush_count"], 10);
            assert_eq!(response["added_brushes"], 8);
            assert!(
                (response["removed_overlap_volume"].as_f64().unwrap()
                    - if wedges { 1536.0 } else { 2048.0 })
                .abs()
                    < 1e-5
            );
            assert!(response["removed_volume"].as_f64().unwrap() > 2048.0);
            assert!(response["added_volume"].as_f64().unwrap() > 0.0);
            assert_eq!(before, d.to_map_text());
            let mut apply = options;
            apply["dry_run"] = json!(false);
            apply["expected_revision"] = json!(revision);
            call_tool(&mut d, &mut c, "join_corner", &apply).unwrap();
            assert_eq!(brush_list(&d).len(), 10);
            d.undo();
            assert_eq!(before, d.to_map_text());
            assert!(call_tool(
                &mut d,
                &mut c,
                "join_corner",
                &json!({"inner":{"radius":8,"segments":0}})
            )
            .is_err());
            assert_eq!(before, d.to_map_text());
        }
    }
}

#[cfg(test)]
mod hollow_sphere_tests {
    use super::*;
    #[test]
    fn hollow_sphere_insert_save_and_undo_are_atomic() {
        let mut document = Document::new(Default::default());
        let mut collab = Collab::default();
        let before = document.to_map_text();
        let args = json!({"kind":"hollow_sphere","min":[-64,-64,-64],"max":[64,64,64],"thickness":8,"brushes":84});
        call_tool(&mut document, &mut collab, "add_shape", &args).unwrap();
        assert_eq!(map_model::node::brushes(document.tree_world()).len(), 84);
        assert_eq!(document.selection_ids().len(), 84);
        let after = document.to_map_text();
        let map = map_model::io::map_reader::read_map(&after, None).unwrap();
        let format = map.format;
        let tree = map_model::node::build_tree_owned(map, &document.world_bounds());
        let reopened = Document::from_world(tree.world, Default::default(), format, None);
        assert_eq!(map_model::node::brushes(reopened.tree_world()).len(), 84);
        assert!(map_model::node::brushes(reopened.tree_world())
            .iter()
            .all(|b| !b.contains_point(Vec3d::zero())));
        let volume = |d: &Document| {
            map_model::node::brushes(d.tree_world())
                .iter()
                .map(|b| b.volume())
                .sum::<f64>()
        };
        assert!((volume(&document) - volume(&reopened)).abs() < 1e-5);
        document.undo();
        assert_eq!(document.to_map_text(), before);
        document.redo();
        assert_eq!(document.to_map_text(), after);
        for invalid in [
            json!({"thickness":64}),
            json!({"brushes":21}),
            json!({"brushes":324}),
            json!({"brushes":-1}),
            json!({"brushes":20.5}),
            json!({"accuracy":1}),
        ] {
            let mut args = args.clone();
            for (key, value) in invalid.as_object().unwrap() {
                args[key] = value.clone();
            }
            assert!(call_tool(&mut document, &mut collab, "add_shape", &args).is_err());
            assert_eq!(document.to_map_text(), after);
        }
    }
}

#[cfg(test)]
mod geometry_upgrade_tests {
    use super::*;
    #[test]
    fn advanced_stairs_and_directional_hollow_use_their_mcp_options() {
        let mut document = Document::new(Default::default());
        let mut collab = Collab::default();
        call_tool(&mut document,&mut collab,"add_shape",&json!({"kind":"stairs","min":[0,0,0],"max":[128,160,64],"step_height":8,"axis":"y","ascending":false,"stair_layout":"left","flight_width":16,"slab_thickness":4})).unwrap();
        let brushes = map_model::node::brushes(document.tree_world());
        assert_eq!(brushes.len(), 9);
        assert!((brushes[0].bounds().max[1] - 160.0).abs() < 1e-6);
        assert!((brushes[0].bounds().max[0] - 16.0).abs() < 1e-6);
        document.undo();
        assert!(map_model::node::brushes(document.tree_world()).is_empty());
        call_tool(
            &mut document,
            &mut collab,
            "add_shape",
            &json!({"kind":"cuboid","min":[0,0,0],"max":[64,64,64]}),
        )
        .unwrap();
        let before = document.to_map_text();
        assert!(call_tool(
            &mut document,
            &mut collab,
            "hollow",
            &json!({"thickness":8,"open_selected_faces":true})
        )
        .is_err());
        assert_eq!(document.to_map_text(), before);
        call_tool(
            &mut document,
            &mut collab,
            "hollow",
            &json!({"thickness":8,"direction":"centered"}),
        )
        .unwrap();
        let brushes = map_model::node::brushes(document.tree_world());
        assert_eq!(brushes.len(), 6);
        assert!(
            (brushes
                .iter()
                .map(|b| b.bounds().max[0])
                .fold(f64::NEG_INFINITY, f64::max)
                - 68.0)
                .abs()
                < 1e-6
        );
        document.undo();
        assert_eq!(document.to_map_text(), before);
    }
}
