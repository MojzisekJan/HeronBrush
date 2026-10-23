//! Project build/run profiles. Execution starts only on an explicit user action.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub version: u32,
    pub profiles: Vec<Profile>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    #[serde(default = "project_workdir")]
    pub workdir: String,
    pub steps: Vec<Step>,
}
fn project_workdir() -> String {
    "${PROJECT_DIR}".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    ExportMap {
        path: String,
    },
    ExportTarget {
        engine: String,
        #[serde(default)]
        directory: String,
    },
    CopyFile {
        source: String,
        destination: String,
    },
    Run {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        workdir: String,
    },
}
impl Store {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match crate::files::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        if text.len() > 1024 * 1024 {
            return Err("Build profiles exceed 1 MiB".into());
        }
        let store: Self = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        store.validate()?;
        Ok(store)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!(
                "Unsupported build profile version {}",
                self.version
            ));
        }
        if self.profiles.len() > 64 {
            return Err("Too many build profiles (maximum 64)".into());
        }
        for profile in &self.profiles {
            if profile.name.trim().is_empty() || profile.steps.len() > 128 {
                return Err("Each profile needs a name and at most 128 steps".into());
            }
        }
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        // Never silently replace a corrupt/newer store after discovery failed.
        Self::load(path)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("Build profiles exceed 1 MiB".into());
        }
        std::fs::create_dir_all(path.parent().ok_or("Missing profile directory")?)
            .map_err(|e| e.to_string())?;
        crate::godot_live::atomic_write(path, &bytes).map_err(|e| e.to_string())
    }
}
pub fn path(map: &Path) -> PathBuf {
    crate::export_settings::project_path(map).with_file_name("build.json")
}
/// Simple named substitutions. No shell expansion or implicit command execution.
pub fn expand(input: &str, variables: &BTreeMap<String, String>) -> Result<String, String> {
    let mut remaining = input;
    let mut result = String::new();
    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 2..];
        let end = remaining.find('}').ok_or("Unclosed profile variable")?;
        let name = &remaining[..end];
        result.push_str(
            variables
                .get(name)
                .ok_or_else(|| format!("Unknown profile variable ${{{name}}}"))?,
        );
        remaining = &remaining[end + 1..];
    }
    result.push_str(remaining);
    if result.contains('\0') {
        return Err("Profile values cannot contain NUL".into());
    }
    Ok(result)
}
pub fn target(name: &str) -> Result<crate::export::Target, String> {
    use crate::export::Target;
    match name.to_ascii_lowercase().as_str() {
        "godot" => Ok(Target::Godot),
        "defold" => Ok(Target::Defold),
        "three" | "threejs" => Ok(Target::ThreeJs),
        "babylon" | "babylonjs" => Ok(Target::Babylon),
        "bevy" => Ok(Target::Bevy),
        "raylib" => Ok(Target::Raylib),
        "unity" => Ok(Target::Unity),
        "unreal" => Ok(Target::Unreal),
        "flax" => Ok(Target::Flax),
        "stride" => Ok(Target::Stride),
        "cocos" => Ok(Target::Cocos),
        "gdevelop" => Ok(Target::GDevelop),
        "obj" => Ok(Target::Obj),
        "gltf" => Ok(Target::Gltf),
        "glb" => Ok(Target::Glb),
        _ => Err(format!("Unknown export engine: {name}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod runner {
    use super::*;
    use crate::{
        document::{Document, MaterialSource},
        export::{Contents, Options},
        world::WorldBounds,
    };
    use std::{
        io::Read,
        process::{Command, Stdio},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        thread,
        time::Duration,
    };

    /// Plain data transferred to the worker; the editor's Document stays on its UI thread.
    pub struct Snapshot {
        source: String,
        map: PathBuf,
        definitions: Option<PathBuf>,
        materials: Vec<MaterialSource>,
        preferences: crate::preferences::Preferences,
        exports: Vec<(String, Options)>,
    }
    impl Snapshot {
        pub fn capture(document: &Document) -> Result<Self, String> {
            let map = document
                .path
                .clone()
                .ok_or("Save the map once before using build profiles")?;
            let settings = crate::export_settings::Settings::for_map(Some(&map));
            settings.validate()?;
            let exports = [
                "godot", "defold", "three", "babylon", "bevy", "raylib", "unity", "unreal", "flax",
                "stride", "cocos", "gdevelop", "obj", "gltf", "glb",
            ]
            .into_iter()
            .map(|key| (key.into(), settings.for_target(target(key).unwrap())))
            .collect();
            Ok(Self {
                source: document.to_map_text(),
                map,
                definitions: document.definitions_path().map(Path::to_path_buf),
                materials: document.material_sources().to_vec(),
                preferences: crate::preferences::current(),
                exports,
            })
        }
        fn document(&self) -> Result<Document, String> {
            let map = map_model::io::map_reader::read_map(&self.source, None)
                .map_err(|e| e.to_string())?;
            let bounds = WorldBounds::default();
            let world = geometry_math::bbox::Bbox3d::from_scalars(
                -bounds.half_extent(),
                bounds.half_extent(),
            );
            let tree = map_model::node::build_tree(&map, &world);
            let mut doc = Document::from_world(tree.world, bounds, map.format, map.game);
            doc.path = Some(self.map.clone());
            if let Some(definitions) = &self.definitions {
                doc.load_definitions(definitions)
                    .map_err(|e| e.to_string())?;
            }
            for source in &self.materials {
                match source {
                    MaterialSource::Folder(path) => {
                        doc.load_materials(path).map_err(|e| e.to_string())?;
                    }
                    MaterialSource::Package(path) => {
                        doc.load_wad(path)?;
                    }
                }
            }
            Ok(doc)
        }
        fn variables(&self) -> BTreeMap<String, String> {
            let project = path(&self.map)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            let map = if self.map.is_absolute() {
                self.map.clone()
            } else {
                std::env::current_dir().unwrap_or_default().join(&self.map)
            };
            BTreeMap::from([
                ("PROJECT_DIR".into(), project.to_string_lossy().into_owned()),
                ("MAP_PATH".into(), map.to_string_lossy().into_owned()),
                (
                    "MAP_DIR".into(),
                    map.parent().unwrap().to_string_lossy().into_owned(),
                ),
                (
                    "MAP_BASE_NAME".into(),
                    map.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                ),
                ("GAME_DIR_PATH".into(), self.preferences.game_path.clone()),
                (
                    "APP_DIR_PATH".into(),
                    std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                ),
                (
                    "CPU_COUNT".into(),
                    thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(1)
                        .to_string(),
                ),
            ])
        }
    }
    #[derive(Debug)]
    pub enum Event {
        Log(String),
        Done(Result<(), String>),
    }
    pub struct Job {
        pub events: mpsc::Receiver<Event>,
        cancelled: Arc<AtomicBool>,
    }
    impl Job {
        pub fn cancel(&self) {
            self.cancelled.store(true, Ordering::Relaxed);
        }
        pub fn start(snapshot: Snapshot, profile: Profile) -> Self {
            let (send, events) = mpsc::sync_channel(512);
            let cancelled = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&cancelled);
            thread::spawn(move || {
                crate::preferences::set_current(snapshot.preferences.clone());
                let result = execute(&snapshot, &profile, &send, &flag);
                let _ = send.send(Event::Done(result));
            });
            Self { events, cancelled }
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            self.cancel();
        }
    }
    fn check(cancel: &AtomicBool) -> Result<(), String> {
        if cancel.load(Ordering::Relaxed) {
            Err("Build stopped".into())
        } else {
            Ok(())
        }
    }
    fn resolve(
        value: &str,
        base: &Path,
        vars: &BTreeMap<String, String>,
    ) -> Result<PathBuf, String> {
        let path = PathBuf::from(expand(value, vars)?);
        Ok(if path.is_absolute() {
            path
        } else {
            base.join(path)
        })
    }
    fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::create_dir_all(path.parent().ok_or("Missing destination directory")?)
            .map_err(|e| e.to_string())?;
        crate::godot_live::atomic_write(path, bytes).map_err(|e| e.to_string())
    }
    fn output(
        mut stream: impl Read + Send + 'static,
        send: mpsc::SyncSender<Event>,
        label: &'static str,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut bytes = [0; 4096];
            loop {
                let size = match stream.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                if send
                    .send(Event::Log(format!(
                        "{label}: {}",
                        String::from_utf8_lossy(&bytes[..size])
                    )))
                    .is_err()
                {
                    break;
                }
            }
        })
    }
    fn execute(
        snapshot: &Snapshot,
        profile: &Profile,
        send: &mpsc::SyncSender<Event>,
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        if profile.steps.is_empty() {
            return Err("Add at least one build step".into());
        }
        let mut vars = snapshot.variables();
        let project = PathBuf::from(&vars["PROJECT_DIR"]);
        let workdir = resolve(&profile.workdir, &project, &vars)?;
        vars.insert(
            "WORK_DIR_PATH".into(),
            workdir.to_string_lossy().into_owned(),
        );
        let mut document = None;
        for (index, step) in profile.steps.iter().enumerate() {
            check(cancel)?;
            let _ = send.send(Event::Log(format!(
                "Step {}/{}: {:?}",
                index + 1,
                profile.steps.len(),
                step
            )));
            match step {
                Step::ExportMap { path } => {
                    let output = resolve(path, &workdir, &vars)?;
                    if output == snapshot.map
                        || output
                            .canonicalize()
                            .ok()
                            .is_some_and(|out| Some(out) == snapshot.map.canonicalize().ok())
                    {
                        return Err(
                            "Export Map must use a different path than the source map".into()
                        );
                    }
                    write(&output, snapshot.source.as_bytes())?;
                }
                Step::CopyFile {
                    source,
                    destination,
                } => {
                    let source = resolve(source, &workdir, &vars)?;
                    let destination = resolve(destination, &workdir, &vars)?;
                    std::fs::create_dir_all(destination.parent().ok_or("Missing copy directory")?)
                        .map_err(|e| e.to_string())?;
                    let pending = destination.with_extension("heron-copy-pending");
                    let mut created = false;
                    let copied = (|| -> Result<(), String> {
                        let mut input = std::fs::File::open(&source)
                            .map_err(|e| format!("{}: {e}", source.display()))?;
                        let mut output = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&pending)
                            .map_err(|e| e.to_string())?;
                        created = true;
                        let mut bytes = [0; 65536];
                        loop {
                            check(cancel)?;
                            let size = input.read(&mut bytes).map_err(|e| e.to_string())?;
                            if size == 0 {
                                break;
                            }
                            use std::io::Write;
                            output
                                .write_all(&bytes[..size])
                                .map_err(|e| e.to_string())?;
                        }
                        output.sync_all().map_err(|e| e.to_string())?;
                        std::fs::set_permissions(
                            &pending,
                            input.metadata().map_err(|e| e.to_string())?.permissions(),
                        )
                        .map_err(|e| e.to_string())?;
                        check(cancel)?;
                        std::fs::rename(&pending, &destination).map_err(|e| e.to_string())
                    })();
                    if copied.is_err() && created {
                        let _ = std::fs::remove_file(&pending);
                    }
                    copied?;
                }
                Step::ExportTarget { engine, directory } => {
                    let selected = target(engine)?;
                    let mut options = snapshot
                        .exports
                        .iter()
                        .find(|(key, _)| target(key).ok() == Some(selected))
                        .unwrap()
                        .1
                        .clone();
                    options.name = vars["MAP_BASE_NAME"].clone();
                    let destination = if directory.trim().is_empty() {
                        options.directory.clone()
                    } else {
                        directory.clone()
                    };
                    if destination.trim().is_empty() {
                        return Err(
                            "Set an export directory in the step or engine export settings".into(),
                        );
                    }
                    let destination = resolve(&destination, &workdir, &vars)?;
                    if document.is_none() {
                        document = Some(snapshot.document()?);
                    }
                    let (report, files) = crate::export::export_files(
                        document.as_ref().unwrap(),
                        &destination,
                        &options,
                    )
                    .map_err(|e| e.to_string())?;
                    for (file, contents) in files {
                        check(cancel)?;
                        let bytes = match &contents {
                            Contents::Text(text) => text.as_bytes(),
                            Contents::Bytes(bytes) => bytes.as_slice(),
                        };
                        write(&file, bytes)?;
                    }
                    for warning in report.warnings {
                        let _ = send.send(Event::Log(warning));
                    }
                }
                Step::Run {
                    program,
                    args,
                    workdir: step_dir,
                } => {
                    let program = expand(program, &vars)?;
                    if program.trim().is_empty() {
                        return Err("Choose an executable for the Run step".into());
                    }
                    let directory = if step_dir.is_empty() {
                        workdir.clone()
                    } else {
                        resolve(step_dir, &workdir, &vars)?
                    };
                    let args: Vec<_> = args
                        .iter()
                        .map(|arg| expand(arg, &vars))
                        .collect::<Result<_, _>>()?;
                    let executable = if Path::new(&program).components().count() > 1
                        && !Path::new(&program).is_absolute()
                    {
                        directory.join(&program)
                    } else {
                        PathBuf::from(&program)
                    };
                    let mut command = Command::new(&executable);
                    command
                        .args(args)
                        .current_dir(directory)
                        .stdin(Stdio::null())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped());
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        command.process_group(0);
                    }
                    let mut child = command
                        .spawn()
                        .map_err(|e| format!("Cannot start {program}: {e}"))?;
                    let stdout = output(child.stdout.take().unwrap(), send.clone(), "stdout");
                    let stderr = output(child.stderr.take().unwrap(), send.clone(), "stderr");
                    let status = loop {
                        if cancel.load(Ordering::Relaxed) {
                            #[cfg(unix)]
                            // SAFETY: `kill` reads no memory of ours. The child is
                            // still unreaped (`try_wait` has not returned its
                            // status), so its pid, and the process group
                            // `process_group(0)` gave it, cannot have been reused.
                            unsafe {
                                libc::kill(-(child.id() as i32), libc::SIGKILL);
                            }
                            #[cfg(not(unix))]
                            {
                                let _ = child.kill();
                            }
                            let _ = child.wait();
                            return Err("Build stopped".into());
                        }
                        match child.try_wait().map_err(|e| e.to_string())? {
                            Some(status) => break status,
                            None => thread::sleep(Duration::from_millis(30)),
                        }
                    };
                    // Flush ordinary tools' trailing output before reporting completion.
                    // A daemon retaining inherited pipes must not hold the editor forever.
                    let deadline = std::time::Instant::now() + Duration::from_secs(1);
                    while !(stdout.is_finished() && stderr.is_finished())
                        && std::time::Instant::now() < deadline
                    {
                        check(cancel)?;
                        thread::sleep(Duration::from_millis(10));
                    }
                    if !status.success() {
                        return Err(format!("{program} failed: {status}"));
                    }
                }
            }
        }
        check(cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_profiles_are_preserved_and_variables_are_literal() {
        let root =
            std::env::temp_dir().join(format!("heron-profile-schema-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("build.json");
        let good = Store {
            version: 1,
            profiles: vec![Profile {
                name: "Game".into(),
                workdir: "${PROJECT_DIR}".into(),
                steps: vec![Step::Run {
                    program: "game".into(),
                    args: vec!["a b".into()],
                    workdir: String::new(),
                }],
            }],
        };
        good.save(&path).unwrap();
        assert_eq!(Store::load(&path).unwrap().profiles[0].name, "Game");
        std::fs::write(&path, "{\"version\":2,\"profiles\":[]}").unwrap();
        assert!(good.save(&path).is_err());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("\"version\":2"));
        let vars = BTreeMap::from([("MAP".into(), "path with spaces/$(literal)".into())]);
        assert_eq!(
            expand("${MAP}", &vars).unwrap(),
            "path with spaces/$(literal)"
        );
        assert!(expand("${MISSING}", &vars).is_err());
        assert!(expand("${MAP", &vars).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    fn fixture(name: &str) -> (PathBuf, crate::document::Document) {
        let root =
            std::env::temp_dir().join(format!("heron-profile-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut doc = crate::document::Document::new(crate::world::WorldBounds::default());
        doc.save_as(root.join("room.map")).unwrap();
        let brush = map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(0., 32.),
            "stone",
        )
        .unwrap();
        doc.add_brushes(vec![brush]);
        (root, doc)
    }
    #[cfg(unix)]
    fn result(job: &runner::Job) -> (Result<(), String>, String) {
        let mut log = String::new();
        loop {
            match job
                .events
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap()
            {
                runner::Event::Log(text) => log.push_str(&text),
                runner::Event::Done(done) => return (done, log),
            }
        }
    }
    #[cfg(unix)]
    #[test]
    fn ordered_export_copy_and_run_keep_unsaved_map_and_literal_arguments() {
        let (root, doc) = fixture("chain");
        let source = std::fs::read_to_string(root.join("room.map")).unwrap();
        let marker = root.join("should-not-exist");
        let literal = format!("a b $(touch {})", marker.display());
        let profile = Profile {
            name: "Compile".into(),
            workdir: root.to_string_lossy().into_owned(),
            steps: vec![
                Step::ExportMap {
                    path: "build/${MAP_BASE_NAME}.map".into(),
                },
                Step::CopyFile {
                    source: "build/room.map".into(),
                    destination: "copied.map".into(),
                },
                Step::Run {
                    program: "/usr/bin/printf".into(),
                    args: vec!["%s".into(), literal.clone()],
                    workdir: String::new(),
                },
                Step::ExportTarget {
                    engine: "obj".into(),
                    directory: "meshes".into(),
                },
            ],
        };
        let job = runner::Job::start(runner::Snapshot::capture(&doc).unwrap(), profile);
        let (done, log) = result(&job);
        done.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("room.map")).unwrap(),
            source
        );
        assert_eq!(
            std::fs::read_to_string(root.join("copied.map")).unwrap(),
            doc.to_map_text()
        );
        assert!(std::fs::read_to_string(root.join("meshes/room.obj"))
            .unwrap()
            .contains("v "));
        assert!(log.contains(&literal));
        assert!(!marker.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn a_failed_tool_prevents_later_steps_and_stop_terminates_owned_job() {
        let (root, doc) = fixture("stop");
        let profile = Profile {
            name: "Failure".into(),
            workdir: root.to_string_lossy().into_owned(),
            steps: vec![
                Step::Run {
                    program: "/bin/sh".into(),
                    args: vec!["-c".into(), "exit 7".into()],
                    workdir: String::new(),
                },
                Step::ExportMap {
                    path: "must-not-run.map".into(),
                },
            ],
        };
        let job = runner::Job::start(runner::Snapshot::capture(&doc).unwrap(), profile);
        assert!(result(&job).0.unwrap_err().contains("failed"));
        assert!(!root.join("must-not-run.map").exists());
        let profile = Profile {
            name: "Stop".into(),
            workdir: root.to_string_lossy().into_owned(),
            steps: vec![Step::Run {
                program: "/bin/sleep".into(),
                args: vec!["30".into()],
                workdir: String::new(),
            }],
        };
        let job = runner::Job::start(runner::Snapshot::capture(&doc).unwrap(), profile);
        std::thread::sleep(std::time::Duration::from_millis(100));
        job.cancel();
        assert_eq!(result(&job).0.unwrap_err(), "Build stopped");
        std::fs::remove_dir_all(root).unwrap();
    }
}
