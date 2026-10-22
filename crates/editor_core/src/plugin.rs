//! Running a plugin script against the document.
//!
//! Rhai plugins can call only registered editor functions. Operation and call-depth limits stop
//! runaway scripts. The API shares document operations with MCP; see `docs/plugins.md`.
//!
//! This is the headless slice: `heron-brush --run-script <script> <in.map> [out.map]`
//! opens the map, runs the script with the whole map selected, and writes the result. The
//! language binding and every function are proved here before any of this reaches the GUI.

use std::cell::RefCell;
use std::rc::Rc;

use rhai::{Array, Dynamic, Engine, EvalAltResult, Position};

use crate::document::{Document, FaceRef};
use crate::plugin_manifest::SettingValue;
use crate::world::WorldBounds;
use map_model::brush_face::{FaceColor, Rgb};
use map_model::node::NodeId;

/// How many operations a script may run before it is stopped. Generous for a batch pass
/// over a large map -- a script touching every face of a thousand brushes is well under
/// this -- and far short of a hang.
const MAX_OPERATIONS: u64 = 10_000_000;

/// How deep a script's calls may nest.
const MAX_CALL_LEVELS: usize = 64;

/// Maps the small integer handles a script sees to the ids the document uses.
///
/// A `NodeId` carries a random high-bit prefix, so it does not fit an `i64` and would
/// mean nothing to read. A handle is its position in this table: the first brush a script
/// asks about is `0`, the next new one `1`, and so on, stable for the run.
#[derive(Default)]
struct Handles {
    brushes: Vec<NodeId>,
}

impl Handles {
    /// The handle for a node, adding it to the table if it is new.
    fn handle_of(&mut self, node: NodeId) -> i64 {
        match self.brushes.iter().position(|known| *known == node) {
            Some(index) => index as i64,
            None => {
                self.brushes.push(node);
                (self.brushes.len() - 1) as i64
            }
        }
    }

    /// The node a handle stands for.
    fn node_of(&self, handle: i64) -> Result<NodeId, Box<EvalAltResult>> {
        usize::try_from(handle)
            .ok()
            .and_then(|index| self.brushes.get(index).copied())
            .ok_or_else(|| runtime(format!("there is no brush {handle}")))
    }
}

/// Declarative output, independent of a GUI toolkit. Applied after a successful run.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum UiItem {
    Text(String),
    Value { label: String, value: String },
    Progress { label: String, fraction: f64 },
}
#[derive(Debug, Default)]
pub struct RunOutput {
    pub lines: Vec<String>,
    pub panel: Vec<UiItem>,
}
fn push_ui(items: &Rc<RefCell<Vec<UiItem>>>, item: UiItem) -> Result<(), Box<EvalAltResult>> {
    let text_len = match &item {
        UiItem::Text(s) => s.len(),
        UiItem::Value { label, value } => label.len() + value.len(),
        UiItem::Progress { label, .. } => label.len(),
    };
    if text_len > 4096 || items.borrow().len() >= 64 {
        return Err(runtime(
            "Plugin panel output limit exceeded (64 items, 4096 bytes per item)".into(),
        ));
    }
    items.borrow_mut().push(item);
    Ok(())
}

/// Runs `script` against `document`. What the script sees as `selection()` is whatever is
/// selected now; the caller selects the whole map first if that is what it wants (the
/// headless runner does). `settings` are the answers to what the manifest asked for,
/// which the script reads back by key through `setting("…")`. Returns the lines the
/// script printed through `say`, or the message of the error that stopped it.
pub fn run_with(
    document: &mut Document,
    script: &str,
    settings: &[(String, SettingValue)],
) -> Result<Vec<String>, String> {
    let output = run_action(document, script, settings, "")?;
    let mut lines = output.lines;
    for item in output.panel {
        lines.push(match item {
            UiItem::Text(s) => s,
            UiItem::Value { label, value } => format!("{label}: {value}"),
            UiItem::Progress { label, fraction } => format!("{label}: {:.0}%", fraction * 100.0),
        });
    }
    Ok(lines)
}

/// A loaded plugin: compile once, retain bounded project-independent state until reload.
pub struct Session {
    ast: rhai::AST,
    state: Rc<RefCell<std::collections::BTreeMap<String, Dynamic>>>,
}
impl Session {
    /// Compile source without running it or touching a document.
    pub fn new(source: &str) -> Result<Self, String> {
        Ok(Self {
            ast: Engine::new().compile(source).map_err(|e| e.to_string())?,
            state: Rc::new(RefCell::new(std::collections::BTreeMap::new())),
        })
    }
    /// Execute an action using the loaded AST and retained scalar state.
    pub fn run(
        &mut self,
        document: &mut Document,
        settings: &[(String, SettingValue)],
        action: &str,
    ) -> Result<RunOutput, String> {
        execute(document, settings, action, &self.ast, self.state.clone())
    }
    /// Inspect an isolated snapshot. Even a misbehaving panel cannot edit the map.
    pub fn inspect(
        &mut self,
        document: &Document,
        settings: &[(String, SettingValue)],
        action: &str,
    ) -> Result<RunOutput, String> {
        let mut snapshot = Document::from_world(
            document.tree_world().clone(),
            WorldBounds::new(document.world_bounds().max.x()),
            document.format,
            document.game.clone(),
        );
        snapshot.restore_selection(&document.selection_snapshot());
        self.run(&mut snapshot, settings, action)
    }
}

/// Run an explicitly chosen action; scripts are never executed to discover UI.
pub fn run_action(
    document: &mut Document,
    script: &str,
    settings: &[(String, SettingValue)],
    action: &str,
) -> Result<RunOutput, String> {
    Session::new(script)?.run(document, settings, action)
}

fn execute(
    document: &mut Document,
    settings: &[(String, SettingValue)],
    action: &str,
    ast: &rhai::AST,
    state: Rc<RefCell<std::collections::BTreeMap<String, Dynamic>>>,
) -> Result<RunOutput, String> {
    // The document is shared with the registered functions for the length of the run. An
    // empty document stands in its place meanwhile; the real one is put back at the end.
    let shared = Rc::new(RefCell::new(std::mem::replace(
        document,
        Document::new(WorldBounds::default()),
    )));
    let said: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let handles: Rc<RefCell<Handles>> = Rc::new(RefCell::new(Handles::default()));

    let panel = Rc::new(RefCell::new(Vec::new()));
    let mut engine = build_engine(
        Rc::clone(&shared),
        Rc::clone(&said),
        Rc::clone(&handles),
        settings.to_vec(),
    );
    let action = action.to_string();
    engine.register_fn("action", move || action.clone());
    {
        let panel = panel.clone();
        engine.register_fn("ui_text", move |text: &str| {
            push_ui(&panel, UiItem::Text(text.to_owned()))
        });
    }
    {
        let panel = panel.clone();
        engine.register_fn("ui_value", move |label: &str, value: Dynamic| {
            push_ui(
                &panel,
                UiItem::Value {
                    label: label.to_owned(),
                    value: value.to_string(),
                },
            )
        });
    }
    {
        let panel = panel.clone();
        engine.register_fn("ui_progress", move |label: &str, fraction: f64| {
            if !fraction.is_finite() {
                return Err(runtime("Progress must be finite".into()));
            }
            push_ui(
                &panel,
                UiItem::Progress {
                    label: label.to_owned(),
                    fraction: fraction.clamp(0.0, 1.0),
                },
            )
        });
    }
    let read_state = state.clone();
    engine.register_fn("state", move |key: &str| {
        read_state
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn(
        "set_state",
        move |key: &str, value: Dynamic| -> Result<(), Box<EvalAltResult>> {
            if !(value.is::<rhai::INT>()
                || value.is::<f64>()
                || value.is::<bool>()
                || value.is::<rhai::ImmutableString>()
                || value.is_unit())
                || key.len() > 128
                || value.to_string().len() > 4096
                || (!state.borrow().contains_key(key) && state.borrow().len() >= 64)
            {
                return Err(runtime("Plugin state limit exceeded".into()));
            }
            state.borrow_mut().insert(key.to_owned(), value);
            Ok(())
        },
    );
    let result = engine.run_ast(ast);
    // The engine's registered closures each hold a handle on the shared state; dropping
    // it is what lets the document be taken back out.
    drop(engine);

    let recovered = Rc::try_unwrap(shared)
        .unwrap_or_else(|_| panic!("the engine kept a handle on the document"))
        .into_inner();
    *document = recovered;
    let said = Rc::try_unwrap(said)
        .unwrap_or_else(|_| panic!("the engine kept a handle"))
        .into_inner();

    match result {
        Ok(()) => Ok(RunOutput {
            lines: said,
            panel: Rc::try_unwrap(panel).expect("released panel").into_inner(),
        }),
        Err(error) => Err(script_error(&error)),
    }
}

/// The message from a script error, without rhai's position noise for a one-file script.
fn script_error(error: &EvalAltResult) -> String {
    match error {
        EvalAltResult::ErrorRuntime(value, _) => value.to_string(),
        other => other.to_string(),
    }
}

/// Builds the engine with every plugin function registered against the shared state.
fn build_engine(
    document: Rc<RefCell<Document>>,
    said: Rc<RefCell<Vec<String>>>,
    handles: Rc<RefCell<Handles>>,
    settings: Vec<(String, SettingValue)>,
) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    // A plugin has no business reaching the file system or spawning anything.
    engine.set_max_modules(0);

    // setting(key): what the person answered for the manifest's `[[setting]]` of that
    // key -- a float, a string or a boolean. A key the manifest never declared comes back
    // as `()`, so a script can offer a setting and still run under an older manifest:
    //
    //     let copies = setting("copies") ?? 5;
    {
        engine.register_fn("setting", move |key: &str| -> Dynamic {
            settings
                .iter()
                .find(|(declared, _)| declared == key)
                .map(|(_, value)| match value {
                    SettingValue::Number(number) => Dynamic::from_float(*number),
                    SettingValue::Text(text) => Dynamic::from(text.clone()),
                    SettingValue::Check(yes) => Dynamic::from_bool(*yes),
                })
                .unwrap_or(Dynamic::UNIT)
        });
    }

    // say(text): a line for the console (stdout in the headless runner).
    {
        let said = Rc::clone(&said);
        engine.register_fn("say", move |text: &str| {
            said.borrow_mut().push(text.to_string());
        });
    }

    // selection(): the handles of the selected brushes.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn("selection", move || -> Array {
            let document = document.borrow();
            let mut handles = handles.borrow_mut();
            document
                .selectable_brushes()
                .into_iter()
                .filter(|(id, _)| document.is_selected(*id))
                .map(|(id, _)| Dynamic::from_int(handles.handle_of(id)))
                .collect()
        });
    }

    // brushes(): the handles of every brush in the map.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn("brushes", move || -> Array {
            let document = document.borrow();
            let mut handles = handles.borrow_mut();
            document
                .selectable_brushes()
                .into_iter()
                .map(|(id, _)| Dynamic::from_int(handles.handle_of(id)))
                .collect()
        });
    }

    // face_count(brush): how many drawn faces the brush has.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "face_count",
            move |brush: i64| -> Result<i64, Box<EvalAltResult>> {
                let node = handles.borrow().node_of(brush)?;
                let document = document.borrow();
                let brush = document
                    .brush(node)
                    .ok_or_else(|| runtime(format!("brush {brush} is gone")))?;
                // `geometry_faces` builds each face's point list, which is a heap
                // allocation apiece thrown away unread when all that is wanted is how
                // many there are.
                Ok(brush.geometry().face_count() as i64)
            },
        );
    }

    // brush_center(brush): [x, y, z] of the middle of the brush's bounds.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "brush_center",
            move |brush: i64| -> Result<Array, Box<EvalAltResult>> {
                let node = handles.borrow().node_of(brush)?;
                let document = document.borrow();
                let brush = document
                    .brush(node)
                    .ok_or_else(|| runtime(format!("brush {brush} is gone")))?;
                let c = brush.bounds().center();
                Ok(vec![
                    Dynamic::from_float(c.x()),
                    Dynamic::from_float(c.y()),
                    Dynamic::from_float(c.z()),
                ])
            },
        );
    }

    // brush_bounds(brush): [[minx, miny, minz], [maxx, maxy, maxz]].
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "brush_bounds",
            move |brush: i64| -> Result<Array, Box<EvalAltResult>> {
                let node = handles.borrow().node_of(brush)?;
                let document = document.borrow();
                let brush = document
                    .brush(node)
                    .ok_or_else(|| runtime(format!("brush {brush} is gone")))?;
                let bounds = brush.bounds();
                let point = |p: geometry_math::vec::Vec3d| {
                    Dynamic::from_array(vec![
                        Dynamic::from_float(p.x()),
                        Dynamic::from_float(p.y()),
                        Dynamic::from_float(p.z()),
                    ])
                };
                Ok(vec![point(bounds.min), point(bounds.max)])
            },
        );
    }

    // face_color(brush, face): () when the face wears none, [r, g, b, a] for a solid
    // colour, or ["grad", [r,g,b,a], [r,g,b,a], angle] for a gradient.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "face_color",
            move |brush: i64, face: i64| -> Result<Dynamic, Box<EvalAltResult>> {
                let reference = face_ref(&handles.borrow(), brush, face)?;
                let document = document.borrow();
                let Some(face) = document.face_at(reference) else {
                    return Err(runtime(format!("brush {brush} has no face {face}")));
                };
                Ok(match face.paint() {
                    None => Dynamic::UNIT,
                    Some(FaceColor::Solid { color, alpha }) => {
                        Dynamic::from_array(rgba_array(color, alpha))
                    }
                    Some(FaceColor::MultiGradient {
                        stops,
                        radial,
                        angle,
                        alpha,
                        ends,
                    }) => {
                        let mut gradient = vec![
                            Dynamic::from(if radial { "radial" } else { "linear" }),
                            Dynamic::from_array(
                                stops
                                    .as_slice()
                                    .iter()
                                    .map(|s| {
                                        Dynamic::from_array(vec![
                                            Dynamic::from_float(f64::from(s.position)),
                                            Dynamic::from_array(rgba_array(s.color, alpha)),
                                        ])
                                    })
                                    .collect(),
                            ),
                            Dynamic::from_float(f64::from(angle)),
                        ];
                        if let Some(ends) = ends {
                            gradient.push(Dynamic::from_array(
                                [ends.start[0], ends.start[1], ends.end[0], ends.end[1]]
                                    .map(|v| Dynamic::from_float(f64::from(v)))
                                    .to_vec(),
                            ));
                        }
                        Dynamic::from_array(gradient)
                    }
                    Some(FaceColor::Gradient {
                        from,
                        to,
                        angle,
                        alpha,
                    }) => Dynamic::from_array(vec![
                        Dynamic::from("grad"),
                        Dynamic::from_array(rgba_array(from, alpha)),
                        Dynamic::from_array(rgba_array(to, alpha)),
                        Dynamic::from_float(f64::from(angle)),
                    ]),
                })
            },
        );
    }

    // set_face_color(brush, face, value): value is [r,g,b] or [r,g,b,a] for a solid
    // colour, ["grad", [..], [..], angle] for a gradient, or () to take the colour off.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "set_face_color",
            move |brush: i64, face: i64, value: Dynamic| -> Result<(), Box<EvalAltResult>> {
                let reference = face_ref(&handles.borrow(), brush, face)?;
                let colour = parse_color(value)?;
                let mut document = document.borrow_mut();
                if document.face_at(reference).is_none() {
                    return Err(runtime(format!("brush {brush} has no face {face}")));
                }
                document.set_face_color_on(&[reference], colour);
                Ok(())
            },
        );
    }

    // translate(brush, [dx, dy, dz]): move one brush.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "translate",
            move |brush: i64, delta: Array| -> Result<(), Box<EvalAltResult>> {
                let node = handles.borrow().node_of(brush)?;
                let delta = vec3(&delta)?;
                let mut document = document.borrow_mut();
                if document.brush(node).is_none() {
                    return Err(runtime(format!("brush {brush} is gone")));
                }
                document.select_only(node);
                document.translate_selection(delta);
                Ok(())
            },
        );
    }

    // add_cuboid([x, y, z], [x, y, z], material): a box between the two corners, wearing
    // the given material or the current one when it is `()`. Returns the new brush handle.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "add_cuboid",
            move |min: Array, max: Array, material: Dynamic| -> Result<i64, Box<EvalAltResult>> {
                let (min, max) = (vec3(&min)?, vec3(&max)?);
                let bounds = geometry_math::bbox::Bbox3d::new(
                    geometry_math::vec::min(min, max),
                    geometry_math::vec::max(min, max),
                );
                if (bounds.max - bounds.min).v.iter().any(|e| *e <= 0.0) {
                    return Err(runtime(
                        "a cuboid needs a positive size on every axis".into(),
                    ));
                }
                let mut document = document.borrow_mut();
                let name = material
                    .into_string()
                    .unwrap_or_else(|_| document.current_material().to_string());
                let world = document.world_bounds();
                let brush = map_model::shapes::create_cuboid(&world, &bounds, &name)
                    .map_err(|error| runtime(format!("could not build the cuboid: {error}")))?;
                document.add_brushes(vec![brush]);
                // `add_brushes` leaves the new brush the whole selection.
                let id = document
                    .selectable_brushes()
                    .into_iter()
                    .map(|(id, _)| id)
                    .find(|id| document.is_selected(*id))
                    .ok_or_else(|| runtime("the cuboid was not added".into()))?;
                Ok(handles.borrow_mut().handle_of(id))
            },
        );
    }

    // delete(brush): remove one brush from the map.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "delete",
            move |brush: i64| -> Result<(), Box<EvalAltResult>> {
                let node = handles.borrow().node_of(brush)?;
                let mut document = document.borrow_mut();
                if document.brush(node).is_none() {
                    return Err(runtime(format!("brush {brush} is gone")));
                }
                document.select_only(node);
                document.delete_selection();
                Ok(())
            },
        );
    }

    // select(handles): make those brushes the selection, for a plugin that leaves the
    // person with something picked. An array, or one handle.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "select",
            move |chosen: Dynamic| -> Result<(), Box<EvalAltResult>> {
                let wanted = handle_list(chosen)?;
                set_selection(&mut document.borrow_mut(), &handles.borrow(), &wanted)
            },
        );
    }

    // csg_subtract(cutters): take those brushes away from every other brush, as the CSG
    // Subtract command does with the selection. Returns how many pieces the map gained.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "csg_subtract",
            move |cutters: Dynamic| -> Result<i64, Box<EvalAltResult>> {
                let wanted = handle_list(cutters)?;
                let mut document = document.borrow_mut();
                set_selection(&mut document, &handles.borrow(), &wanted)?;
                Ok(document.subtract_selection() as i64)
            },
        );
    }

    // csg_merge(brushes): fuse those brushes into the smallest convex brush that holds
    // them all. Returns the new brush's handle, or `()` when there was nothing to merge.
    {
        let document = Rc::clone(&document);
        let handles = Rc::clone(&handles);
        engine.register_fn(
            "csg_merge",
            move |chosen: Dynamic| -> Result<Dynamic, Box<EvalAltResult>> {
                let wanted = handle_list(chosen)?;
                let mut document = document.borrow_mut();
                set_selection(&mut document, &handles.borrow(), &wanted)?;
                if !document.merge_selection() {
                    return Ok(Dynamic::UNIT);
                }
                // The merge leaves the one new brush selected.
                let id = document
                    .selectable_brushes()
                    .into_iter()
                    .map(|(id, _)| id)
                    .find(|id| document.is_selected(*id))
                    .ok_or_else(|| runtime("the merge left nothing selected".into()))?;
                Ok(Dynamic::from_int(handles.borrow_mut().handle_of(id)))
            },
        );
    }

    // Entity handles share the invocation-local table, but every property API checks type.
    {
        let document = document.clone();
        let handles = handles.clone();
        engine.register_fn("entities", move || -> Array {
            let doc = document.borrow();
            let mut handles = handles.borrow_mut();
            map_model::node::entities_with_ids(doc.tree_world())
                .into_iter()
                .map(|(id, _)| Dynamic::from_int(handles.handle_of(id)))
                .collect()
        });
    }
    {
        let document = document.clone();
        let handles = handles.clone();
        engine.register_fn("selected_entities", move || -> Array {
            let doc = document.borrow();
            let mut handles = handles.borrow_mut();
            let mut selected = std::collections::HashSet::new();
            for id in doc.selection_ids() {
                if let Some(entity) = doc.entity_around(*id) {
                    selected.insert(entity.id);
                }
            }
            map_model::node::entities_with_ids(doc.tree_world())
                .into_iter()
                .filter(|(id, _)| selected.contains(id))
                .map(|(id, _)| Dynamic::from_int(handles.handle_of(id)))
                .collect()
        });
    }
    {
        let document = document.clone();
        let handles = handles.clone();
        engine.register_fn(
            "property",
            move |handle: i64, key: &str| -> Result<Dynamic, Box<EvalAltResult>> {
                let id = handles.borrow().node_of(handle)?;
                let doc = document.borrow();
                let entity = map_model::node::entities_with_ids(doc.tree_world())
                    .into_iter()
                    .find(|(eid, _)| *eid == id)
                    .ok_or_else(|| runtime("Handle is not an entity".into()))?
                    .1;
                Ok(entity
                    .property(key)
                    .map(|value| Dynamic::from(value.to_owned()))
                    .unwrap_or(Dynamic::UNIT))
            },
        );
    }
    {
        let document = document.clone();
        let handles = handles.clone();
        engine.register_fn(
            "set_property",
            move |handle: i64, key: &str, value: &str| -> Result<bool, Box<EvalAltResult>> {
                if key.is_empty()
                    || key.len() > 128
                    || value.len() > 4096
                    || key.chars().any(char::is_control)
                {
                    return Err(runtime("Invalid entity property".into()));
                }
                let id = handles.borrow().node_of(handle)?;
                let mut doc = document.borrow_mut();
                if !map_model::node::entities_with_ids(doc.tree_world())
                    .iter()
                    .any(|(eid, _)| *eid == id)
                {
                    return Err(runtime("Handle is not an entity".into()));
                }
                Ok(doc.set_entity_property(id, key, value))
            },
        );
    }
    engine
}

/// Makes the brushes for `wanted` the document's selection, skipping a handle whose
/// brush is gone and erroring on an out-of-range one.
fn set_selection(
    document: &mut Document,
    handles: &Handles,
    wanted: &[i64],
) -> Result<(), Box<EvalAltResult>> {
    document.clear_selection();
    for handle in wanted {
        let node = handles.node_of(*handle)?;
        if document.brush(node).is_some() {
            document.toggle_selected(node);
        }
    }
    Ok(())
}

/// A script's handle argument as a list: an array of them, or a single one.
fn handle_list(chosen: Dynamic) -> Result<Vec<i64>, Box<EvalAltResult>> {
    if chosen.is_array() {
        chosen
            .into_array()
            .expect("just checked it is an array")
            .into_iter()
            .map(|v| {
                v.as_int()
                    .map_err(|_| runtime("a handle is a whole number".into()))
            })
            .collect()
    } else {
        Ok(vec![chosen.as_int().map_err(|_| {
            runtime("expected a handle or an array of handles".into())
        })?])
    }
}

/// A rhai runtime error carrying just a message.
fn runtime(message: String) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(message.into(), Position::NONE))
}

/// A `FaceRef` from a brush handle and a geometry face index, both as a script passes
/// them.
fn face_ref(handles: &Handles, brush: i64, face: i64) -> Result<FaceRef, Box<EvalAltResult>> {
    let node = handles.node_of(brush)?;
    let face = usize::try_from(face)
        .map_err(|_| runtime(format!("a face index cannot be negative: {face}")))?;
    Ok(FaceRef { node, face })
}

/// `[r, g, b, a]` as rhai integers.
fn rgba_array(rgb: Rgb, alpha: u8) -> Array {
    vec![
        Dynamic::from_int(i64::from(rgb.r)),
        Dynamic::from_int(i64::from(rgb.g)),
        Dynamic::from_int(i64::from(rgb.b)),
        Dynamic::from_int(i64::from(alpha)),
    ]
}

/// One channel of a colour array, clamped to 0..=255.
fn channel(value: &Dynamic, what: &str) -> Result<u8, Box<EvalAltResult>> {
    let n = value
        .as_int()
        .map_err(|_| runtime(format!("{what} is not a whole number")))?;
    Ok(n.clamp(0, 255) as u8)
}

/// A number from a rhai value that may be an integer or a float.
fn number(value: &Dynamic) -> Option<f64> {
    value
        .as_float()
        .ok()
        .or_else(|| value.as_int().ok().map(|i| i as f64))
}

/// An `[x, y, z]` array as a vector.
fn vec3(array: &Array) -> Result<geometry_math::vec::Vec3d, Box<EvalAltResult>> {
    if array.len() != 3 {
        return Err(runtime("a point is three numbers".into()));
    }
    let at = |i: usize| number(&array[i]).ok_or_else(|| runtime("a point is three numbers".into()));
    Ok(geometry_math::vec::Vec3d::new(at(0)?, at(1)?, at(2)?))
}

/// Reads a colour a script passed: `()`, `[r,g,b(,a)]`, or `["grad", from, to, angle]`.
fn parse_color(value: Dynamic) -> Result<Option<FaceColor>, Box<EvalAltResult>> {
    if value.is_unit() {
        return Ok(None);
    }
    let array = value
        .into_array()
        .map_err(|_| runtime("a colour is an array or ()".into()))?;

    let kind = array.first().and_then(|v| v.clone().into_string().ok());
    if matches!(kind.as_deref(), Some("linear" | "radial")) {
        use map_model::brush_face::{ColorStop, GradientStops};
        if !(3..=4).contains(&array.len()) {
            return Err(runtime("a gradient is [kind, [[position, rgba], ...], angle] or with [su, sv, eu, ev] after the angle".into()));
        }
        let values = array[1]
            .clone()
            .into_array()
            .map_err(|_| runtime("stops must be an array".into()))?;
        let mut stops = Vec::new();
        let mut alpha = 255;
        for value in values {
            let pair = value
                .into_array()
                .map_err(|_| runtime("stop must be [position, rgba]".into()))?;
            if pair.len() != 2 {
                return Err(runtime("stop must be [position, rgba]".into()));
            }
            let position =
                number(&pair[0]).ok_or_else(|| runtime("invalid stop position".into()))? as f32;
            let (color, opacity) = parse_rgba(&pair[1])?;
            if stops.is_empty() {
                alpha = opacity;
            }
            stops.push(ColorStop { position, color });
        }
        let angle = number(&array[2]).ok_or_else(|| runtime("invalid angle".into()))? as f32;
        if !angle.is_finite() {
            return Err(runtime("invalid angle".into()));
        }
        let ends = match array.get(3) {
            None => None,
            Some(value) => {
                let values = value
                    .clone()
                    .into_array()
                    .map_err(|_| runtime("the ends are [su, sv, eu, ev]".into()))?;
                let numbers: Vec<f32> = values
                    .iter()
                    .filter_map(number)
                    .map(|v| v as f32)
                    .filter(|v| v.is_finite())
                    .collect();
                let [su, sv, eu, ev] = numbers[..] else {
                    return Err(runtime("the ends are [su, sv, eu, ev]".into()));
                };
                Some(map_model::brush_face::GradientEnds {
                    start: [su, sv],
                    end: [eu, ev],
                })
            }
        };
        return Ok(Some(FaceColor::MultiGradient {
            stops: GradientStops::new(&stops)
                .ok_or_else(|| runtime("provide 2 to 8 stops with positions from 0 to 1".into()))?,
            radial: kind.as_deref() == Some("radial"),
            angle,
            alpha,
            ends,
        }));
    }

    // A gradient: the first entry is the word "grad".
    if array
        .first()
        .and_then(|v| v.clone().into_string().ok())
        .as_deref()
        == Some("grad")
    {
        if array.len() != 4 {
            return Err(runtime("a gradient is [\"grad\", from, to, angle]".into()));
        }
        let (from, from_a) = parse_rgba(&array[1])?;
        let (to, _to_a) = parse_rgba(&array[2])?;
        let angle = number(&array[3])
            .ok_or_else(|| runtime("the gradient angle is not a number".into()))?;
        return Ok(Some(
            FaceColor::gradient(from, to, angle as f32).with_alpha(from_a),
        ));
    }

    let (rgb, alpha) = parse_rgba(&Dynamic::from_array(array))?;
    Ok(Some(FaceColor::solid(rgb).with_alpha(alpha)))
}

/// `[r, g, b]` or `[r, g, b, a]` as a colour and its alpha (255 when the fourth is
/// missing).
fn parse_rgba(value: &Dynamic) -> Result<(Rgb, u8), Box<EvalAltResult>> {
    let array = value
        .clone()
        .into_array()
        .map_err(|_| runtime("a colour is [r, g, b] or [r, g, b, a]".into()))?;
    if array.len() != 3 && array.len() != 4 {
        return Err(runtime("a colour is [r, g, b] or [r, g, b, a]".into()));
    }
    let rgb = Rgb::new(
        channel(&array[0], "red")?,
        channel(&array[1], "green")?,
        channel(&array[2], "blue")?,
    );
    let alpha = match array.get(3) {
        Some(a) => channel(a, "alpha")?,
        None => 255,
    };
    Ok((rgb, alpha))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manifest::SettingValue;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;

    fn cube_with_colours() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .expect("the cube is built");
        document.add_brushes(vec![brush]);
        document.select_all();
        // A bright red on every face, and a black-to-white gradient on the first.
        document.set_face_color(Some(FaceColor::solid(Rgb::new(220, 20, 20))));
        let id = document.selectable_brushes()[0].0;
        document.set_face_color_on(
            &[FaceRef { node: id, face: 0 }],
            Some(FaceColor::gradient(
                Rgb::new(0, 0, 0),
                Rgb::new(255, 255, 255),
                30.0,
            )),
        );
        document
    }

    #[test]
    fn script_multicolor_gradient_round_trips() {
        for kind in ["linear", "radial"] {
            let value = Dynamic::from_array(vec![
                Dynamic::from(kind),
                Dynamic::from_array(vec![
                    Dynamic::from_array(vec![
                        Dynamic::from_float(0.0),
                        Dynamic::from_array(rgba_array(Rgb::new(255, 0, 0), 123)),
                    ]),
                    Dynamic::from_array(vec![
                        Dynamic::from_float(0.5),
                        Dynamic::from_array(rgba_array(Rgb::new(0, 255, 0), 123)),
                    ]),
                    Dynamic::from_array(vec![
                        Dynamic::from_float(1.0),
                        Dynamic::from_array(rgba_array(Rgb::new(0, 0, 255), 123)),
                    ]),
                ]),
                Dynamic::from_float(45.0),
            ]);
            let Some(FaceColor::MultiGradient {
                stops,
                radial,
                angle,
                alpha,
                ..
            }) = parse_color(value).unwrap()
            else {
                panic!("not a gradient")
            };
            assert_eq!(stops.as_slice().len(), 3);
            assert_eq!(radial, kind == "radial");
            assert_eq!(angle, 45.0);
            assert_eq!(alpha, 123);
        }
    }

    #[test]
    fn a_script_can_read_and_repaint_every_face() {
        let mut document = cube_with_colours();
        let said = run_with(
            &mut document,
            r#"
            fn grey(rgb) {
                let l = (0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]).to_int();
                [l, l, l, rgb[3]]
            }

            let painted = 0;
            for b in selection() {
                for f in range(0, face_count(b)) {
                    let c = face_color(b, f);
                    if c == () { continue; }
                    if c[0] == "grad" {
                        set_face_color(b, f, ["grad", grey(c[1]), grey(c[2]), c[3]]);
                    } else {
                        set_face_color(b, f, grey(c));
                    }
                    painted += 1;
                }
            }
            say("painted " + painted);
            "#,
            &[],
        )
        .expect("the script runs");

        assert_eq!(said, vec!["painted 6".to_string()]);

        let id = document.selectable_brushes()[0].0;
        // The plain red faces are now a flat grey: 0.299*220 + 0.587*20 + 0.114*20 = 79.
        match document
            .face_at(FaceRef { node: id, face: 1 })
            .unwrap()
            .paint()
        {
            Some(FaceColor::Solid { color, .. }) => assert_eq!(color, Rgb::new(79, 79, 79)),
            other => panic!("face 1 is {other:?}, not a solid grey"),
        }
        // The gradient's ends went to grey too (already black and white here), angle kept.
        match document
            .face_at(FaceRef { node: id, face: 0 })
            .unwrap()
            .paint()
        {
            Some(FaceColor::Gradient {
                from, to, angle, ..
            }) => {
                assert_eq!(from, Rgb::new(0, 0, 0));
                assert_eq!(to, Rgb::new(255, 255, 255));
                assert_eq!(angle, 30.0);
            }
            other => panic!("face 0 is {other:?}, not a gradient"),
        }
    }

    #[test]
    fn a_plugin_run_wrapped_in_run_as_one_edit_undoes_in_one_press() {
        let mut document = cube_with_colours();
        let id = document.selectable_brushes()[0].0;
        let before = document
            .face_at(FaceRef { node: id, face: 1 })
            .unwrap()
            .paint();

        document.run_as_one_edit(|document| {
            run_with(
                document,
                r#"for b in selection() {
                    for f in range(0, face_count(b)) { set_face_color(b, f, [10, 20, 30]); }
                }"#,
                &[],
            )
            .expect("the script runs")
        });

        // Six faces were repainted, but one undo takes the whole run off.
        assert!(matches!(
            document
                .face_at(FaceRef { node: id, face: 3 })
                .unwrap()
                .paint(),
            Some(FaceColor::Solid {
                color: Rgb { r: 10, .. },
                ..
            })
        ));
        document.undo();
        assert_eq!(
            document
                .face_at(FaceRef { node: id, face: 1 })
                .unwrap()
                .paint(),
            before,
            "one undo restores the face the plugin changed"
        );
        assert_eq!(
            document
                .face_at(FaceRef { node: id, face: 3 })
                .unwrap()
                .paint(),
            Some(FaceColor::solid(Rgb::new(220, 20, 20))),
        );
    }

    #[test]
    fn a_script_can_add_delete_and_select_brushes() {
        let mut document = cube_with_colours();
        document.clear_selection();

        let said = run_with(
            &mut document,
            r#"
            // Two boxes, then throw the first away and leave the second picked.
            let a = add_cuboid([0, 0, 0], [16, 16, 16], "stone");
            let b = add_cuboid([32, 0, 0], [48, 16, 16], ());
            delete(a);
            select(b);
            say("built 2, kept 1");
            "#,
            &[],
        )
        .expect("the script runs");
        assert_eq!(said, vec!["built 2, kept 1".to_string()]);

        // The starting cube plus the one box that was kept.
        assert_eq!(document.selectable_brushes().len(), 2);
        assert!(document.has_selection(), "the kept box is selected");
        let min = document.selection_bounds().unwrap().min;
        assert!(
            (min - Vec3d::new(32.0, 0.0, 0.0))
                .v
                .iter()
                .all(|d| d.abs() < 1e-6),
            "the kept box is the second one, at {min:?}"
        );

        // The whole run undoes in one press when it is wrapped.
        let mut fresh = cube_with_colours();
        fresh.clear_selection();
        fresh.run_as_one_edit(|document| {
            run_with(
                document,
                "add_cuboid([0,0,0],[8,8,8],()); add_cuboid([0,0,16],[8,8,24],());",
                &[],
            )
            .expect("runs")
        });
        assert_eq!(fresh.selectable_brushes().len(), 3);
        fresh.undo();
        assert_eq!(
            fresh.selectable_brushes().len(),
            1,
            "one undo removes both boxes"
        );
    }

    #[test]
    fn a_script_can_subtract_and_merge() {
        // A box with a smaller box cut out of the middle: the cutter is gone and the box
        // is a shell of several pieces.
        let mut document = Document::new(WorldBounds::default());
        let said = run_with(
            &mut document,
            r#"
            add_cuboid([-64, -64, -64], [64, 64, 64], ());
            let hole = add_cuboid([-16, -16, -16], [16, 16, 16], ());
            let cut = csg_subtract(hole);
            say("cut " + cut);
            "#,
            &[],
        )
        .expect("the script runs");
        assert_eq!(said, vec!["cut 1".to_string()], "one box was carved");
        assert!(
            document.selectable_brushes().len() > 1,
            "the box is a shell now, not one brush"
        );

        // csg_merge on two touching boxes gives one brush.
        let mut two = Document::new(WorldBounds::default());
        run_with(
            &mut two,
            r#"
            let a = add_cuboid([0, 0, 0], [16, 16, 16], ());
            let b = add_cuboid([16, 0, 0], [32, 16, 16], ());
            csg_merge([a, b]);
            "#,
            &[],
        )
        .expect("runs");
        assert_eq!(
            two.selectable_brushes().len(),
            1,
            "the two boxes became one"
        );
    }

    #[test]
    fn a_script_reads_the_settings_it_was_answered_with() {
        let mut document = Document::new(WorldBounds::default());
        let answers = [
            ("copies".to_string(), SettingValue::Number(3.0)),
            (
                "direction".to_string(),
                SettingValue::Text("down".to_string()),
            ),
            ("align".to_string(), SettingValue::Check(true)),
        ];
        let said = run_with(
            &mut document,
            r#"
            say("copies " + setting("copies"));
            say("direction " + setting("direction"));
            say("align " + setting("align"));
            // A key nobody answered comes back as (), so a script can have a fallback.
            say("missing " + (setting("nope") ?? "default"));
            "#,
            &answers,
        )
        .expect("the script runs");
        assert_eq!(
            said,
            vec![
                "copies 3.0".to_string(),
                "direction down".to_string(),
                "align true".to_string(),
                "missing default".to_string(),
            ]
        );
    }

    #[test]
    fn a_runaway_script_is_stopped() {
        let mut document = cube_with_colours();
        let error = run_with(&mut document, "let n = 0; loop { n += 1; }", &[])
            .expect_err("the loop never ends");
        assert!(
            error.to_lowercase().contains("operation"),
            "the error should name the operation limit: {error}"
        );
    }

    #[test]
    fn a_bad_handle_comes_back_as_a_message() {
        let mut document = cube_with_colours();
        let error = run_with(&mut document, "set_face_color(999, 0, [1, 2, 3]);", &[])
            .expect_err("there is no brush 999");
        assert!(error.contains("999"), "{error}");
    }
}

#[cfg(test)]
mod panel_api_tests {
    use super::*;
    #[test]
    fn actions_return_typed_output_and_old_runner_prints_it() {
        let mut doc = Document::new(WorldBounds::default());
        let result = run_action(
            &mut doc,
            "ui_text(action()); ui_value(\"Count\", 3); ui_progress(\"Done\", 2.0);",
            &[],
            "inspect",
        )
        .unwrap();
        assert_eq!(
            result.panel,
            vec![
                UiItem::Text("inspect".into()),
                UiItem::Value {
                    label: "Count".into(),
                    value: "3".into()
                },
                UiItem::Progress {
                    label: "Done".into(),
                    fraction: 1.0
                }
            ]
        );
        assert!(!doc.is_modified());
        assert_eq!(
            run_with(&mut doc, "ui_value(\"Count\", 3);", &[]).unwrap(),
            vec!["Count: 3"]
        );
    }
    #[test]
    fn failed_scripts_do_not_publish_partial_panel_and_output_is_bounded() {
        let mut doc = Document::new(WorldBounds::default());
        assert!(run_action(
            &mut doc,
            "ui_text(\"partial\"); throw \"failed\";",
            &[],
            "inspect"
        )
        .is_err());
        let error = run_action(
            &mut doc,
            "for n in 0..65 { ui_text(\"row\"); }",
            &[],
            "inspect",
        )
        .unwrap_err();
        assert!(error.contains("output limit"));
        let huge = format!("ui_text(\"{}\");", "x".repeat(4097));
        assert!(run_action(&mut doc, &huge, &[], "").is_err());
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn loaded_actions_retain_bounded_state_and_inspection_cannot_edit_document() {
        let mut document = Document::new(WorldBounds::default());
        let mut session = Session::new(
            r#"
            let count = state("count") ?? 0;
            set_state("count", count + 1);
            ui_value("Runs", state("count"));
            if action() == "panel" { add_cuboid([0.0,0.0,0.0], [16.0,16.0,16.0], "stone"); }
        "#,
        )
        .unwrap();
        let before = document.to_map_text();
        session.inspect(&document, &[], "panel").unwrap();
        assert_eq!(document.to_map_text(), before);
        let output = session.run(&mut document, &[], "inspect").unwrap();
        assert!(matches!(&output.panel[0], UiItem::Value { value, .. } if value == "2"));
        assert!(Session::new("set_state(\"bad\", [1,2,3]);")
            .unwrap()
            .run(&mut document, &[], "")
            .is_err());
    }
}

#[cfg(test)]
mod entity_plugin_tests {
    use super::*;
    #[test]
    fn bulk_property_example_changes_only_selection_and_undoes_once() {
        let mut doc = Document::new(WorldBounds::default());
        let a = doc.create_point_entity("relic", geometry_math::vec::Vec3d::zero());
        let b = doc.create_point_entity("relic", geometry_math::vec::Vec3d::fill(32.0));
        let c = doc.create_point_entity("other", geometry_math::vec::Vec3d::fill(64.0));
        doc.select_only(a);
        doc.toggle_selected(b);
        let source = include_str!("../../../assets/plugins/bulk-properties/main.rhai");
        let mut session = Session::new(source).unwrap();
        let values = vec![
            ("key".into(), SettingValue::Text("team".into())),
            ("value".into(), SettingValue::Text("blue".into())),
        ];
        doc.run_as_one_edit(|doc| session.run(doc, &values, "apply"))
            .unwrap();
        let get = |doc: &Document, id| {
            map_model::node::entities_with_ids(doc.tree_world())
                .into_iter()
                .find(|(eid, _)| *eid == id)
                .unwrap()
                .1
                .property("team")
                .map(str::to_owned)
        };
        assert_eq!(get(&doc, a).as_deref(), Some("blue"));
        assert_eq!(get(&doc, b).as_deref(), Some("blue"));
        assert_eq!(get(&doc, c), None);
        doc.undo().unwrap();
        assert_eq!(get(&doc, a), None);
        assert_eq!(get(&doc, b), None);
        let before = doc.to_map_text();
        Session::new(include_str!(
            "../../../assets/plugins/project-rules/main.rhai"
        ))
        .unwrap()
        .inspect(
            &doc,
            &[
                ("classname".into(), SettingValue::Text("relic".into())),
                ("required".into(), SettingValue::Text("charge".into())),
            ],
            "check",
        )
        .unwrap();
        assert_eq!(doc.to_map_text(), before);
    }
}
