//! Shared debounced engine live publishing. CPU work runs off the UI thread.
//!
//! The frontend calls [`LiveLink::tick`] every 100 ms and shows [`LiveLink::status`]; it
//! hears about each change through [`LiveLink::set_observer`].
use crate::{
    document::Document,
    export::{Options, Target},
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::mpsc,
};
use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MANIFEST: &str = ".heron-brush-live.json";
const ADDON: &str = "addons/heron_brush_live";
thread_local! {
    static ACTIVE_PROJECTS: RefCell<std::collections::HashSet<PathBuf>> = RefCell::new(Default::default());
}

pub struct Prepared {
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub meshes: Vec<String>,
    pub scene: String,
    pub warnings: Vec<String>,
}
struct State {
    generation: u64,
    source: String,
    root: PathBuf,
    options: Options,
    settings: scene_export::settings::Settings,
    observed: u64,
    dirty: bool,
    changed: Instant,
    first_change: Instant,
    last_text: String,
    published: String,
    pending: Option<mpsc::Receiver<Result<Prepared, String>>>,
    defold_pending: Option<mpsc::Receiver<Result<String, String>>>,
    defold_synced: String,
    defold_retry: Instant,
}

pub struct LiveLink {
    document: Weak<RefCell<Document>>,
    state: RefCell<Option<State>>,
    message: RefCell<String>,
    engine: std::cell::Cell<Target>,
    defold_hot_reload: std::cell::Cell<bool>,
    /// The frontend's indicator and status labels, told about every change.
    observer: RefCell<Option<Box<dyn Fn(&LiveLink)>>>,
    /// Whether the status changed since the observer last heard. The observer is told
    /// only once no borrow of the state is held, since it reads the state back.
    changed: std::cell::Cell<bool>,
}

/// What the frontend shows of a live link.
pub struct Status {
    /// The last thing the link said.
    pub message: String,
    /// The header indicator's label, `None` while no link runs.
    pub indicator: Option<String>,
}

pub fn project(options: &Options) -> Result<PathBuf, String> {
    if options.target == Target::Flax { return crate::flax_live::project(options); }
    if options.target == Target::Defold { return crate::defold_live::project(options); }
    if options.target == Target::Unity { return crate::unity_live::project(options); }
    if options.directory.is_empty() {
        return Err("Export this map to a Godot project first, then start the live link.".into());
    }
    let directory = Path::new(&options.directory)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let root = if options.project_dir.is_empty() {
        scene_export::materials::find_project_root(&directory)
            .ok_or("The export folder must be inside a Godot project.")?
    } else {
        PathBuf::from(&options.project_dir)
            .canonicalize()
            .map_err(|e| e.to_string())?
    };
    if !root.join("project.godot").is_file() || !directory.starts_with(&root) {
        return Err("Choose an export folder inside the Godot project.".into());
    }
    Ok(root)
}

pub fn install(root: &Path) -> Result<(), String> {
    for (name, bytes) in [
        (
            "plugin.cfg",
            include_bytes!("godot_live/plugin.cfg").as_slice(),
        ),
        (
            "plugin.gd",
            include_bytes!("godot_live/plugin.gd").as_slice(),
        ),
    ] {
        let path = root.join(ADDON).join(name);
        // Do not replace a user's locally modified addon without an explicit update flow.
        if path.exists() && std::fs::read(&path).map_err(|e| e.to_string())? != bytes {
            return Err(format!(
                "{} already exists with different contents. Move it aside before installing this version.",
                path.display()
            ));
        }
    }
    std::fs::create_dir_all(root.join(ADDON)).map_err(|e| e.to_string())?;
    for (name, bytes) in [
        (
            "plugin.cfg",
            include_bytes!("godot_live/plugin.cfg").as_slice(),
        ),
        (
            "plugin.gd",
            include_bytes!("godot_live/plugin.gd").as_slice(),
        ),
    ] {
        atomic_write(&root.join(ADDON).join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

impl LiveLink {
    pub fn new(document: &Rc<RefCell<Document>>) -> Rc<Self> {
        Rc::new(Self {
            document: Rc::downgrade(document),
            state: RefCell::new(None),
            message: RefCell::new("Live link stopped".into()),
            engine: std::cell::Cell::new(Target::Godot),
            defold_hot_reload: std::cell::Cell::new(false),
            observer: RefCell::new(None),
            changed: std::cell::Cell::new(false),
        })
    }
    /// Hands every change of [`Self::status`] to `observer`.
    pub fn set_observer(&self, observer: impl Fn(&LiveLink) + 'static) {
        *self.observer.borrow_mut() = Some(Box::new(observer));
    }
    /// What to show: the message, and the indicator's label while a link runs.
    pub fn status(&self) -> Status {
        let message = self.message.borrow().clone();
        let phase = if message.contains("updated") { "connected" }
            else if message.contains("failed") || message.contains("Could not") { "error" }
            else if message.starts_with("Building") { "syncing" } else { "waiting" };
        Status {
            indicator: self.state.borrow().is_some().then(|| format!("{}: {phase}", self.engine.get().name())),
            message,
        }
    }
    /// Whether the Defold link hot-reloads the running game.
    pub fn defold_hot_reload(&self) -> bool {
        self.defold_hot_reload.get()
    }
    /// Sets whether the Defold link hot-reloads the running game.
    pub fn set_defold_hot_reload(&self, on: bool) {
        self.defold_hot_reload.set(on);
    }
    /// Installs the engine addon into the target's project, reporting the outcome.
    pub fn install_addon(&self, target: Target) {
        let options=crate::export_settings::Settings::load().for_target(target);
        match project(&options).and_then(|root| if target == Target::Flax { crate::flax_live::install(&root) } else if target == Target::Unity { crate::unity_live::install(&root) } else { install(&root) }) {
            Ok(())=>self.say(if target == Target::Flax { "Addon installed. Let Flax compile, then choose Tools > Heron Brush: attach live map." } else if target == Target::Unity { "Addon installed. Open Window > Heron Brush > Live Link in Unity." } else { "Addon installed. Enable Heron Brush Live in Godot → Project Settings → Plugins." }),
            Err(e)=>self.say(e),
        }
        self.notify();
    }
    /// Starts the link, or says why it cannot.
    pub fn start_or_say(&self, target: Target) {
        if let Err(e) = self.start(target) {
            self.say(e);
        }
        self.notify();
    }
    fn say(&self, message: impl Into<String>) {
        let message = message.into();
        if *self.message.borrow() == message {
            return;
        }
        *self.message.borrow_mut() = message;
        self.changed.set(true);
    }
    /// Tells the observer about a change, once nothing is borrowed.
    fn notify(&self) {
        if self.changed.replace(false) {
            if let Some(observer) = self.observer.borrow().as_ref() {
                observer(self);
            }
        }
    }
    fn start(&self, target: Target) -> Result<(), String> {
        if let Some(state) = self.state.borrow().as_ref() {
            return if state.options.target == target { Ok(()) } else { Err("Stop the active live link before switching engines.".into()) };
        }
        self.engine.set(target);
        let doc = self.document.upgrade().ok_or("Map closed")?;
        let doc = doc.borrow();
        let mut options = crate::export_settings::Settings::load().for_target(target);
        let root = project(&options)?;
        if ACTIVE_PROJECTS.with(|active| active.borrow().contains(&root)) {
            return Err(format!("This {} project already has a live link from another map window. Stop that link first.", target.name()));
        }
        if target != Target::Defold && if target == Target::Flax { !crate::flax_live::installed(&root) } else if target == Target::Unity { !crate::unity_live::installed(&root) } else { !root.join(ADDON).join("plugin.cfg").is_file() } {
            return Err(format!("Install the {} addon first.", target.name()));
        }
        // Dedicated output keeps live previews away from final exports and hand-edited scenes.
        let folder = root.join(match target { Target::Unity => "Assets/HeronBrushLive", Target::Defold => crate::defold_live::FOLDER, Target::Flax => crate::flax_live::FOLDER, _ => "heron_brush_live" });
        if target == Target::Defold { crate::defold_live::claim_folder(&root)?; }
        if target == Target::Flax { crate::flax_live::claim_folder(&root)?; }
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        options.directory = folder.to_string_lossy().into();
        options.name = if matches!(target, Target::Defold | Target::Flax) { crate::defold_live::NAME.into() } else { doc.level_name() };
        options.project_dir = root.to_string_lossy().into_owned();
        let settings = options.settings(&folder);
        let now = Instant::now();
        let source = doc
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("unsaved:{}:{}", std::process::id(), doc.generation()));
        ACTIVE_PROJECTS.with(|active| {
            active.borrow_mut().insert(root.clone());
        });
        *self.state.borrow_mut() = Some(State {
            generation: doc.generation(),
            source,
            root,
            options,
            settings,
            observed: doc.revision(),
            dirty: true,
            changed: now,
            first_change: now,
            last_text: String::new(),
            published: String::new(),
            pending: None,
            defold_pending: None,
            defold_synced: String::new(),
            defold_retry: now,
        });
        self.say(format!("Building initial map for {}", target.name()));
        self.changed.set(true);
        Ok(())
    }
    pub fn stop(&self, message: &str) {
        if let Some(state) = self.state.borrow_mut().take() {
            ACTIVE_PROJECTS.with(|active| {
                active.borrow_mut().remove(&state.root);
            });
            let path = state.root.join(MANIFEST);
            if let Ok(bytes) = std::fs::read(&path) {
                if serde_json::from_slice::<serde_json::Value>(&bytes)
                    .ok()
                    .is_some_and(|v| v["revision"] == state.published)
                {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        self.say(message);
        self.changed.set(true);
        self.notify();
    }
    /// Publishes pending changes and reads the engine's acknowledgements. Call every 100 ms.
    pub fn tick(&self) {
        self.tick_inner();
        self.notify();
    }
    fn tick_inner(&self) {
        let Some(document) = self.document.upgrade() else {
            self.stop("Map closed");
            return;
        };
        let Ok(doc) = document.try_borrow() else {
            return;
        };
        let mut borrow = self.state.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };
        if doc.generation() != state.generation {
            drop(borrow);
            self.stop("Map changed — start the live link again for this map.");
            return;
        }
        let now = Instant::now();
        if doc.revision() != state.observed {
            state.observed = doc.revision();
            state.changed = now;
            if !state.dirty {
                state.first_change = now;
                state.dirty = true;
            }
        }
        if let Some(receiver) = &state.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    state.pending = None;
                    match result.and_then(|prepared| publish(prepared, &state.root, &state.source))
                    {
                        Ok((revision, warnings)) => {
                            state.published = revision;
                            self.say(if state.options.target == Target::Defold {
                                format!("Defold files published. {}", warnings.join("; "))
                            } else if warnings.is_empty() {
                                format!("Sent — waiting for the {} addon", self.engine.get().name())
                            } else {
                                format!("Sent to {}. {}", self.engine.get().name(), warnings.join("; "))
                            });
                        }
                        Err(e) => {
                            state.published.clear();
                            self.say(format!("{} live link failed: {e}. Edit the map or restart the link to retry.", self.engine.get().name()));
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    state.pending = None;
                    state.published.clear();
                    self.say(format!("{} builder stopped. Restart the link to retry.", self.engine.get().name()));
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if state.options.target == Target::Defold {
            if let Some(receiver) = &state.defold_pending {
                match receiver.try_recv() {
                    Ok(result) => {
                        state.defold_pending = None;
                        match result {
                            Ok(message) => { state.defold_synced = state.published.clone(); self.say(message); }
                            Err(error) => {
                                state.defold_retry = now + Duration::from_secs(3);
                                self.say(format!("Defold live link failed: {error} Retrying automatically."));
                            }
                        }
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        state.defold_pending = None;
                        state.defold_retry = now + Duration::from_secs(3);
                        self.say("Defold refresh worker stopped. Retrying automatically.");
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            if state.pending.is_none() && state.defold_pending.is_none()
                && !state.published.is_empty() && state.published != state.defold_synced && now >= state.defold_retry {
                let root = state.root.clone();
                let hot_reload = self.defold_hot_reload.get();
                let (send, receive) = mpsc::channel();
                state.defold_pending = Some(receive);
                std::thread::spawn(move || { let _ = send.send(crate::defold_live::refresh(&root, hot_reload)); });
            }
        }
        if state.pending.is_none() && state.defold_pending.is_none() && state.dirty && ready(now, state.changed, state.first_change) {
            state.dirty = false;
            let text = doc.to_map_text();
            if text != state.last_text {
                state.last_text = text.clone();
                let (classes, warnings) = crate::export::entity_classes(&doc);
                let settings = state.settings.clone();
                let materials = doc.materials().clone();
                let world_bounds = doc.world_bounds();
                let options = state.options.clone();
                let root = state.root.clone();
                let (send, receive) = mpsc::channel();
                state.pending = Some(receive);
                self.say(format!("Building changes for {}", self.engine.get().name()));
                std::thread::spawn(move || {
                    let result =
                        prepare_painted(&text, &world_bounds, &materials, &classes, &settings, &options, &root).map(|mut p| {
                            p.warnings.extend(warnings);
                            p
                        });
                    let _ = send.send(result);
                });
            }
        }
        if state.options.target != Target::Defold && state.pending.is_none() && !state.published.is_empty() {
            if let Ok(bytes) = std::fs::read(state.root.join(".heron-brush-live-status.json")) {
                if let Ok(ack) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if ack["source"] != state.source {
                        return;
                    }
                    if ack["revision"] == state.published && ack["state"] == "error" {
                        self.say(
                            ack["message"]
                                .as_str()
                                .unwrap_or("The engine could not update the map"),
                        );
                    } else if ack["revision"] == state.published && ack["state"] == "updated" {
                        self.say(format!("{} updated — drawing changes will sync automatically", self.engine.get().name()));
                    } else if ack["state"] == "waiting" {
                        self.say(ack["message"].as_str().unwrap_or("Waiting for the engine"));
                    }
                }
            }
        }
    }
}
impl Drop for LiveLink {
    fn drop(&mut self) {
        self.stop("Live link stopped");
    }
}

fn ready(now: Instant, changed: Instant, first_change: Instant) -> bool {
    now.duration_since(changed) >= Duration::from_millis(300)
        || now.duration_since(first_change) >= Duration::from_secs(1)
}

fn prepare_painted(text: &str, world_bounds: &geometry_math::bbox::Bbox3d,
    materials: &map_model::material::MaterialCollection,
    classes: &scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
    settings: &scene_export::settings::Settings, options: &Options, root: &Path,
) -> Result<Prepared, String> {
    let mut settings = settings.clone();
    let (text, images) = crate::export::paint_export::prepare_text(text, world_bounds, materials,
        Path::new(&options.directory), options, &mut settings).map_err(|e| e.to_string())?;
    let mut prepared = prepare(&text, classes, &settings, options, root)?;
    let mut imports = Vec::new();
    let mut files = Vec::new();
    for (path, contents) in images {
        let relative = path.strip_prefix(root).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        imports.push(if options.target == Target::Godot { format!("res://{relative}") } else { relative });
        let bytes = match contents { crate::export::Contents::Bytes(b) => b, crate::export::Contents::Text(t) => t.into_bytes() };
        files.push((path, bytes));
    }
    imports.append(&mut prepared.meshes);
    files.append(&mut prepared.files);
    prepared.meshes = imports;
    prepared.files = files;
    Ok(prepared)
}

fn prepare(
    text: &str,
    classes: &scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
    settings: &scene_export::settings::Settings,
    options: &Options,
    root: &Path,
) -> Result<Prepared, String> {
    if options.target == Target::Flax { return crate::flax_live::prepare(text, classes, settings, options, root); }
    if options.target == Target::Defold { return crate::defold_live::prepare(text, classes, settings, options, root); }
    if options.target == Target::Unity { return crate::unity_live::prepare(text, classes, settings, options, root); }
    let map = scene_export::map::parse(text, &options.name).map_err(|e| e.to_string())?;
    let classes=crate::export_bindings::classes(&map,classes,&options.entities)?;
    let mut built = scene_export::builder::build(
        &map,
        &classes,
        settings,
        &scene_export::builder::BuildOptions {
            scene_name: options.name.clone(),
            mesh_dir: options.name.clone(),
        },
    );
    crate::godot_animation::attach(&mut built.scene, settings.scale_factor, &mut built.warnings);
    let directory = Path::new(&options.directory);
    let res = |p: &Path| {
        p.strip_prefix(root)
            .map(|p| format!("res://{}", p.to_string_lossy().replace('\\', "/")))
            .map_err(|e| e.to_string())
    };
    let mut files = Vec::new();
    let mut meshes = Vec::new();
    for mesh in &built.meshes {
        let path = directory.join(&options.name).join(&mesh.file_name);
        if let Some((path, text)) = crate::export::godot_import_defaults(&path) {
            files.push((path, text.into_bytes()));
        }
        meshes.push(res(&path)?);
        files.push((path, mesh.contents.as_bytes().to_vec()));
    }
    let path = directory.join(format!("{}.tscn", options.name));
    let scene = res(&path)?;
    files.push((path, scene_export::writer::tscn::write(&built.scene).into_bytes()));
    Ok(Prepared {
        files,
        meshes,
        scene,
        warnings: built.warnings,
    })
}
fn publish(prepared: Prepared, root: &Path, source: &str) -> Result<(String, Vec<String>), String> {
    // The addon also checks this sentinel after loading, in case a newer batch
    // arrives during Godot's asynchronous resource scan.
    let busy = root.join(".heron-brush-live-busy");
    atomic_write(&busy, b"publishing").map_err(|e| e.to_string())?;
    struct Busy(std::path::PathBuf);
    impl Drop for Busy {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _busy = Busy(busy);
    for (path, bytes) in &prepared.files {
        atomic_write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let revision = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let manifest = serde_json::json!({"version":1,"revision":revision,"source":source,"scene":prepared.scene,"meshes":prepared.meshes});
    atomic_write(&root.join(MANIFEST), manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    Ok((revision, prepared.warnings))
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("heron-tmp-{}", std::process::id()));
    std::fs::write(&temp, bytes)?;
    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(temp);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    struct Temp(PathBuf);
    impl Temp {
        fn new(name: &str) -> Self {
            let p = std::env::temp_dir().join(format!("heron-live-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn live_keyframes_keep_curves_and_implicit_pivot() {
        let temp = Temp::new("keyframes");
        let options = Options { target: Target::Godot, name: "moving".into(),
            directory: temp.0.to_string_lossy().into_owned(), ..Options::default() };
        let mut document = Document::new(WorldBounds::default());
        document.add_brushes(vec![map_model::shapes::create_cuboid(&document.world_bounds(),
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)), "stone").unwrap()]);
        document.select_all();
        let id = document.create_brush_entity("func_animate").unwrap();
        document.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 32 0 0 0 0 360");
        document.remove_entity_property_on(&[id], "animation_pivot");
        // The implicit animation pivot is the brush centre, not its explicit origin.
        document.set_entity_property(id, "origin", "0 0 0");
        let (classes, _) = crate::export::entity_classes(&document);
        let prepared = prepare(&document.to_map_text(), &classes,
            &scene_export::settings::Settings::default(), &options, &temp.0).unwrap();
        let text = prepared.files.iter().find(|(p,_)| p.extension().is_some_and(|e|e == "tscn")).unwrap();
        let text = String::from_utf8_lossy(&text.1);
        assert!(text.contains("HeronAnimation"));
        assert!(text.contains("script/source ="));
        assert!(text.contains("pivot = Vector3(1.0, 1.0, 1.0)"));
        assert!(text.contains("AnimationLibrary"));
    }

    #[test]
    fn live_paint_images_precede_mesh_imports_and_colour_edits_get_new_resources() {
        use map_model::brush_face::{FaceColor, Rgb};
        for target in [Target::Godot, Target::Unity, Target::Defold, Target::Flax] {
            let temp = Temp::new(&format!("paint-{target:?}"));
            let directory = temp.0.join("Assets/maps");
            let options = Options { target, name: "painted".into(),
                directory: directory.to_string_lossy().into_owned(), ..Options::default() };
            let settings = scene_export::settings::Settings { project_root: Some(temp.0.clone()), ..Default::default() };
            let mut document = Document::new(WorldBounds::default());
            document.add_brushes(vec![map_model::shapes::create_cuboid(&document.world_bounds(),
                &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)), "stone").unwrap()]);
            document.select_all();
            let (classes, _) = crate::export::entity_classes(&document);
            let mut previous_image = String::new();
            let mut previous_mesh = String::new();
            for color in [Rgb::new(255,0,0), Rgb::new(0,0,255)] {
                document.set_face_color(Some(FaceColor::solid(color)));
                let prepared = prepare_painted(&document.to_map_text(), &document.world_bounds(), document.materials(),
                    &classes, &settings, &options, &temp.0).unwrap();
                assert!(prepared.meshes[0].ends_with(".png"), "textures must import before their users");
                assert_ne!(prepared.meshes[0], previous_image);
                previous_image = prepared.meshes[0].clone();
                if target == Target::Unity {
                    assert!(prepared.meshes[1].ends_with(".mtl"));
                    let mesh = prepared.meshes.iter().find(|p| p.ends_with(".obj")).unwrap();
                    assert_ne!(*mesh, previous_mesh, "a paint-only edit must invalidate Unity's immutable OBJ");
                    previous_mesh = mesh.clone();
                    let mtl = prepared.files.iter().find(|(p,_)| p.extension().is_some_and(|e| e == "mtl")).unwrap();
                    assert!(String::from_utf8_lossy(&mtl.1).contains("map_Kd ../painted_paint/"));
                }
                let imports = prepared.meshes.clone();
                publish(prepared, &temp.0, "paint-test").unwrap();
                for path in imports { assert!(temp.0.join(path.trim_start_matches("res://")).is_file()); }
            }
        }
    }

    /// Run against an isolated copy of examples/engine-checks/defold with Defold open.
    #[test]
    #[ignore = "requires HERON_DEFOLD_LIVE_PROJECT and a running Defold editor"]
    fn native_defold_demo_snapshot() {
        let root = PathBuf::from(std::env::var_os("HERON_DEFOLD_LIVE_PROJECT").expect("test project"))
            .canonicalize().unwrap();
        crate::defold_live::claim_folder(&root).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let demo = repository.join("demos/bevy_fps");
        let mut doc = Document::open(demo.join("maps/demo.map"), WorldBounds::default()).unwrap();
        doc.load_definitions(demo.join("gamedata/fpskit.fgd")).unwrap();
        doc.load_materials(demo.join("assets/textures")).unwrap();
        if let Ok(edit) = std::env::var("HERON_DEFOLD_EDIT") {
            let brush = doc.selectable_brushes()[0].0;
            doc.select_only(brush);
            match edit.as_str() {
                "move" => { doc.translate_selection(Vec3d::new(64.0, 0.0, 0.0)); }
                "delete" => { doc.delete_selection(); }
                "paint" => { doc.set_face_color(Some(map_model::brush_face::FaceColor::solid(map_model::brush_face::Rgb::new(20, 100, 255)))); }
                _ => panic!("unknown fixture edit"),
            }
        }
        let options = Options {
            target: Target::Defold, name: crate::defold_live::NAME.into(),
            directory: root.join(crate::defold_live::FOLDER).to_string_lossy().into_owned(),
            project_dir: root.to_string_lossy().into_owned(), texture_dir: "textures".into(),
            model_material: "/main/model.material".into(), model_sampler: "tex0".into(),
            ..Default::default()
        };
        let settings = options.settings(Path::new(&options.directory));
        let (classes, _) = crate::export::entity_classes(&doc);
        let prepared = prepare_painted(&doc.to_map_text(), &doc.world_bounds(), doc.materials(), &classes, &settings, &options, &root).unwrap();
        assert!(prepared.meshes.len() > 3);
        assert!(prepared.files.iter().any(|(path,_)|path.extension().is_some_and(|e|e=="collisionobject")));
        let count = prepared.files.len();
        let (_, warnings) = publish(prepared, &root, "native-defold-demo").unwrap();
        println!("Published {count} files; warnings: {warnings:?}");
        println!("{}", crate::defold_live::refresh(&root, std::env::var_os("HERON_DEFOLD_HOT_RELOAD").is_some()).unwrap());
    }

    #[test]
    #[ignore = "writes native Flax fixtures to HERON_FLAX_LIVE_PROJECT"]
    fn native_flax_live_fixtures() {
        let root = PathBuf::from(std::env::var_os("HERON_FLAX_LIVE_PROJECT").expect("test project")).canonicalize().unwrap();
        crate::flax_live::install(&root).unwrap();
        crate::flax_live::claim_folder(&root).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let demo = repository.join("demos/bevy_fps");
        let mut doc = Document::open(demo.join("maps/demo.map"), WorldBounds::default()).unwrap();
        doc.load_definitions(demo.join("gamedata/fpskit.fgd")).unwrap();
        doc.load_materials(demo.join("assets/textures")).unwrap();
        let options = Options { target: Target::Flax, name: "preview".into(),
            directory: root.join(crate::flax_live::FOLDER).to_string_lossy().into_owned(),
            project_dir: root.to_string_lossy().into_owned(), texture_dir: "textures".into(), ..Default::default() };
        let settings = options.settings(Path::new(&options.directory));
        let (classes, _) = crate::export::entity_classes(&doc);
        let first = doc.selectable_brushes()[0].0;
        for step in 0..4 {
            doc.select_only(first);
            match step {
                1 => { doc.translate_selection(Vec3d::new(64.,0.,0.)); }
                2 => { doc.delete_selection(); }
                3 => { doc.undo(); }
                _ => {}
            }
            let prepared = prepare_painted(&doc.to_map_text(), &doc.world_bounds(), doc.materials(), &classes, &settings, &options, &root).unwrap();
            assert!(prepared.files.iter().all(|(p,_)|p.extension().is_none_or(|e|e!="cs")));
            let snapshot = prepared.files.clone();
            publish(prepared, &root, "native-flax-demo").unwrap();
            let stage = root.join("LiveFixtures").join(step.to_string());
            for (path,bytes) in snapshot {
                let output = stage.join(path.strip_prefix(&root).unwrap());
                std::fs::create_dir_all(output.parent().unwrap()).unwrap();
                std::fs::write(output,bytes).unwrap();
            }
            std::fs::create_dir_all(&stage).unwrap();
            std::fs::copy(root.join(MANIFEST),stage.join(MANIFEST)).unwrap();
        }
    }

    #[test]
    #[ignore="prepare Flax custom-game project first; writes native live fixture"]
    fn native_flax_custom_game_live_fixture(){
        let root=std::path::PathBuf::from("/tmp/heron-flax-custom-game");
        crate::flax_live::install(&root).unwrap();
        let document=Document::open(root.join("workshop.map"),WorldBounds::default()).unwrap();
        let mut options=crate::export_settings::Settings::for_map(Some(&root.join("workshop.map"))).for_target(Target::Flax);
        options.directory=root.join("Content/HeronBrushLive").to_string_lossy().into_owned();
        options.name="workshop".into();
        let settings=options.settings(std::path::Path::new(&options.directory));
        let (classes,_)=crate::export::entity_classes(&document);
        let prepared=crate::flax_live::prepare(&document.to_map_text(),&classes,&settings,&options,&root).unwrap();
        publish(prepared,&root,"custom-puzzle").unwrap();
    }
    #[test]
    #[ignore="prepare Unity custom-game project first; writes native live fixture"]
    fn native_custom_game_live_fixture(){
        let root=std::path::PathBuf::from("/tmp/heron-unity-custom-game");
        crate::unity_live::install(&root).unwrap();
        let document=Document::open(root.join("workshop.map"),WorldBounds::default()).unwrap();
        let mut options=crate::export_settings::Settings::for_map(Some(&root.join("workshop.map"))).for_target(Target::Unity);
        options.directory=root.join("Assets/HeronBrushLive").to_string_lossy().into_owned();
        options.name="workshop".into();
        let settings=options.settings(std::path::Path::new(&options.directory));
        let (classes,_)=crate::export::entity_classes(&document);
        let prepared=crate::unity_live::prepare(&document.to_map_text(),&classes,&settings,&options,&root).unwrap();
        publish(prepared,&root,"custom-puzzle").unwrap();
    }
    #[test]
    #[ignore = "writes native Unity fixtures to HERON_UNITY_LIVE_PROJECT"]
    fn native_unity_live_fixtures() {
        let root = PathBuf::from(std::env::var_os("HERON_UNITY_LIVE_PROJECT").expect("test project")).canonicalize().unwrap();
        crate::unity_live::install(&root).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let demo = repository.join("demos/bevy_fps");
        let mut doc = Document::open(demo.join("maps/demo.map"), WorldBounds::default()).unwrap();
        doc.load_definitions(demo.join("gamedata/fpskit.fgd")).unwrap();
        doc.load_materials(demo.join("assets/textures")).unwrap();
        let options = Options { target: Target::Unity, name: "preview".into(),
            directory: root.join("Assets/HeronBrushLive").to_string_lossy().into_owned(),
            project_dir: root.to_string_lossy().into_owned(), texture_dir: "textures".into(), ..Default::default() };
        let export_dir = root.join("Assets/FinalMap");
        crate::export::export(&doc, &export_dir, &Options { directory: export_dir.to_string_lossy().into_owned(), ..options.clone() }).unwrap();
        let settings = options.settings(Path::new(&options.directory));
        let (classes, _) = crate::export::entity_classes(&doc);
        let first = doc.selectable_brushes()[0].0;
        for step in 0..4 {
            doc.select_only(first);
            match step {
                1 => { doc.translate_selection(Vec3d::new(64.,0.,0.)); }
                2 => { doc.delete_selection(); }
                3 => { doc.undo(); }
                _ => {}
            }
            let prepared = prepare_painted(&doc.to_map_text(), &doc.world_bounds(), doc.materials(), &classes, &settings, &options, &root).unwrap();
            assert!(prepared.files.iter().all(|(p,_)|p.extension().is_none_or(|e|e!="cs")));
            let snapshot = prepared.files.clone();
            publish(prepared, &root, "native-unity-demo").unwrap();
            let stage = root.join("LiveFixtures").join(step.to_string());
            for (path,bytes) in snapshot {
                let output = stage.join(path.strip_prefix(&root).unwrap());
                std::fs::create_dir_all(output.parent().unwrap()).unwrap();
                std::fs::write(output,bytes).unwrap();
            }
            std::fs::create_dir_all(&stage).unwrap();
            std::fs::copy(root.join(MANIFEST),stage.join(MANIFEST)).unwrap();
        }
    }

    #[test]
    fn debounce_waits_for_a_pause_but_does_not_starve_continuous_drawing() {
        let start = Instant::now();
        assert!(!ready(start + Duration::from_millis(200), start, start));
        assert!(ready(start + Duration::from_millis(300), start, start));
        assert!(!ready(
            start + Duration::from_millis(900),
            start + Duration::from_millis(850),
            start
        ));
        assert!(ready(
            start + Duration::from_millis(1000),
            start + Duration::from_millis(950),
            start
        ));
    }
    #[test]
    fn addon_install_is_repeatable_and_preserves_local_edits() {
        let root = Temp::new("install");
        install(&root.0).unwrap();
        install(&root.0).unwrap();
        let path = root.0.join(ADDON).join("plugin.gd");
        std::fs::write(&path, "local edits").unwrap();
        assert!(install(&root.0).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "local edits");
    }
    #[test]
    fn output_must_be_inside_the_godot_project() {
        let root = Temp::new("project");
        std::fs::write(root.0.join("project.godot"), "config_version=5").unwrap();
        let folder = root.0.join("maps");
        std::fs::create_dir(&folder).unwrap();
        let mut options = Options {
            directory: folder.to_string_lossy().into(),
            ..Options::default()
        };
        assert_eq!(project(&options).unwrap(), root.0.canonicalize().unwrap());
        options.directory = std::env::temp_dir().to_string_lossy().into();
        options.project_dir = root.0.to_string_lossy().into();
        assert!(project(&options).is_err());
    }
    #[test]
    fn unsaved_edits_deletion_and_undo_publish_complete_scenes() {
        let root = Temp::new("publish");
        std::fs::write(root.0.join("project.godot"), "config_version=5").unwrap();
        let options = Options {
            directory: root.0.join("maps").to_string_lossy().into(),
            name: "live".into(),
            collision: true,
            cull_hidden: false,
            merge_faces: false,
            ..Options::default()
        };
        let settings = scene_export::settings::Settings {
            project_root: Some(root.0.clone()),
            ..options.settings(Path::new(&options.directory))
        };
        let mut document = Document::new(WorldBounds::default());
        let brush = map_model::shapes::create_cuboid(
            &document.world_bounds(),
            &Bbox3d::from_scalars(-32.0, 32.0),
            "__TB_empty",
        )
        .unwrap();
        document.add_brushes(vec![brush]);
        for step in 0..4 {
            match step {
                1 => {
                    document.select_all();
                    document.translate_selection(Vec3d::new(128.0, 0.0, 0.0));
                }
                2 => {
                    document.select_all();
                    document.delete_selection();
                }
                3 => {
                    document.undo();
                }
                _ => {}
            }
            let prepared = prepare(
                &document.to_map_text(),
                &Default::default(),
                &settings,
                &options,
                &root.0,
            )
            .unwrap();
            assert_eq!(prepared.meshes.is_empty(), step == 2);
            let snapshot = prepared.files.clone();
            let (revision, _) = publish(prepared, &root.0, "verification-map").unwrap();
            let manifest = std::fs::read(root.0.join(MANIFEST)).unwrap();
            let json: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
            assert_eq!(json["revision"], revision);
            assert!(!root.0.join(".heron-brush-live-busy").exists());
            for (path, bytes) in &snapshot {
                assert_eq!(&std::fs::read(path).unwrap(), bytes);
            }
            // Optional fixture used by the separate headless Godot editor check.
            if let Some(fixture) = std::env::var_os("HERON_LIVE_FIXTURES") {
                let stage = PathBuf::from(fixture).join(step.to_string());
                std::fs::create_dir_all(&stage).unwrap();
                for (path, bytes) in snapshot {
                    let path = stage.join(path.strip_prefix(&root.0).unwrap());
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, bytes).unwrap();
                }
                std::fs::write(stage.join(MANIFEST), manifest).unwrap();
            }
        }
        assert!(
            document.path.is_none(),
            "live publishing must not save the user's map"
        );
    }
}
