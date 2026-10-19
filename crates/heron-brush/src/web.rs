//! The web build: the editor on a `<canvas>` in the browser, entered from
//! `crates/heron-brush-web`.
//!
//! The window and the 3D view run on wgpu, over WebGPU where the browser has it. The views,
//! the panels and the tools are the desktop ones.
//!
//! Files: the editor's own files (maps, preferences, the recent list) live in memory
//! behind [`editor_core::files`] and are kept in the browser's IndexedDB between visits.
//! A map the user opens or saves gets a path under [`FILES_DIR`]; writing it writes the
//! file they picked where the browser allows that (File System Access, in Chromium) and
//! downloads it elsewhere. The JavaScript half is `web_files.js`. See docs/web.md.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use eframe::egui;
use eframe::wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::wasm_bindgen;

/// The id of the canvas `index.html` gives the editor.
pub const CANVAS_ID: &str = "heron-brush";

/// Where the data the package ships is, as `games/` and `assets/` beside a desktop
/// executable (`scripts/stage-web-bundle.sh`).
const INSTALL_DIR: &str = "/heron-brush/install";

/// Legacy picked files and copied folder trees.
pub const FILES_DIR: &str = "/heron-brush/files";
/// Each picked file has an isolated directory, retaining its original basename.
const PICKED_DIR: &str = "/heron-brush/picked";

#[wasm_bindgen(module = "/src/web_files.js")]
extern "C" {
    async fn use_webgpu() -> wasm_bindgen::JsValue;
    async fn clipboard_text() -> wasm_bindgen::JsValue;
    async fn load_files() -> wasm_bindgen::JsValue;
    async fn load_bundle() -> wasm_bindgen::JsValue;
    fn store_file(path: &str, bytes: &[u8]);
    fn delete_file(path: &str);
    fn set_unsaved_changes(unsaved: bool);
    async fn pick_open(extensions: Vec<String>) -> wasm_bindgen::JsValue;
    async fn pick_save(suggested: &str, extensions: Vec<String>) -> wasm_bindgen::JsValue;
    async fn pick_folder() -> wasm_bindgen::JsValue;
    fn adopt_handle(path: &str);
    async fn picked_file_path(root: &str, name: &str) -> wasm_bindgen::JsValue;
    fn save_user_file(path: &str, name: &str, bytes: &[u8]) -> js_sys::Promise;
    /// Hands the browser a file to keep in its downloads.
    pub fn download(name: &str, bytes: &[u8]);
    #[wasm_bindgen(js_name = open_playtest)]
    fn js_open_playtest() -> bool;
    fn playtest_active() -> bool;
    fn playtest_export_epoch() -> u32;
    fn playtest_export_busy() -> bool;
    fn take_playtest_export() -> wasm_bindgen::JsValue;
    fn cancel_playtest_export(close: bool);
    fn submit_playtest_export(snapshot: &str, directory: &str, options: &str, files: js_sys::Array, map_key: &str, prepare_ms: f64) -> bool;
    fn publish_playtest_export(answer: &wasm_bindgen::JsValue) -> bool;

}

thread_local! {
    /// The editor's context, so an answer arriving between frames can ask for one.
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    /// Clipboard text read for a paste from the menu, until the next frame takes it.
    static CLIPBOARD: RefCell<Option<String>> = const { RefCell::new(None) };
    /// What the page was last told about unsaved changes.
    static UNSAVED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the open map has unsaved changes, so closing or reloading the tab asks first.
/// Called every frame; the page hears only of a change.
pub fn set_unsaved(unsaved: bool) {
    if UNSAVED.with(|cell| cell.replace(unsaved)) != unsaved {
        set_unsaved_changes(unsaved);
    }
}

/// Starts the editor on the page's canvas. Returns once it runs; eframe drives it from the
/// browser's animation frames after that.
pub async fn run() -> Result<(), String> {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    // Paths that name no disk: without them the profile would be looked for under the
    // temporary directory, and asking for that panics in the browser.
    editor_core::preferences::set_platform_directories("/heron-brush/config".into(), INSTALL_DIR.into());
    editor_core::files::preload(pairs(load_bundle().await).into_iter().map(|(path, contents)| (Path::new(INSTALL_DIR).join(path), contents)));
    editor_core::files::preload(pairs(load_files().await));
    editor_core::files::set_fallback(|path| editor_core::samples::bundled(path).map(str::as_bytes));
    editor_core::files::set_write_hook(written);
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(CANVAS_ID))
        .ok_or_else(|| format!("the page has no element #{CANVAS_ID}"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| format!("#{CANVAS_ID} is not a canvas"))?;
    #[cfg_attr(not(feature = "wgpu"), allow(unused_mut))]
    let mut options = eframe::WebOptions::default();
    // WebGPU draws with render_wgpu. Without it (Firefox, Safari before 26, Chromium on
    // Linux) the window runs on WebGL 2 and the 3D view on the GL renderer, rather than
    // wgpu over WebGL, which render_wgpu cannot run on.
    #[cfg(feature = "wgpu")]
    {
        options.renderer = if use_webgpu().await.as_bool() == Some(true) { eframe::Renderer::Wgpu } else { eframe::Renderer::Glow };
        if options.renderer == eframe::Renderer::Glow {
            // The 3D view blits its picture into the canvas, which WebGL refuses for a
            // multisampled canvas, and `antialias` is on unless asked otherwise. The context
            // made here first is the one eframe is handed back; egui smooths its own edges.
            let attributes = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&attributes, &"antialias".into(), &false.into());
            let _ = canvas.get_context_with_context_options("webgl2", &attributes);
        }
        crate::request_device(&mut options.wgpu_options);
    }
    eframe::WebRunner::new()
        .start(
            canvas,
            options,
            Box::new(|cc| {
                CONTEXT.with(|context| *context.borrow_mut() = Some(cc.egui_ctx.clone()));
                crate::create_app(cc, crate::Startup::default())
            }),
        )
        .await
        .map_err(|error| format!("{error:?}"))
}

/// Reads the clipboard for a paste from the menu (Ctrl+V needs none: it arrives as a
/// paste event). The browser may ask the user first; the text waits for the next frame.
pub fn read_clipboard() {
    wasm_bindgen_futures::spawn_local(async {
        if let Some(text) = clipboard_text().await.as_string() {
            CLIPBOARD.with(|slot| *slot.borrow_mut() = Some(text));
            CONTEXT.with(|context| {
                if let Some(context) = &*context.borrow() {
                    context.request_repaint();
                }
            });
        }
    });
}

/// The clipboard text [`read_clipboard`] brought, once.
pub fn take_clipboard() -> Option<String> {
    CLIPBOARD.with(|slot| slot.borrow_mut().take())
}

/// Names the browser tab, the way the window title names a desktop window.
pub fn set_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        document.set_title(title);
    }
}

/// `[path, bytes]` pairs from `web_files.js`: the bundled data, or what IndexedDB kept.
fn pairs(value: wasm_bindgen::JsValue) -> Vec<(PathBuf, Vec<u8>)> {
    let Ok(entries) = value.dyn_into::<js_sys::Array>() else { return Vec::new() };
    entries
        .iter()
        .filter_map(|entry| {
            let entry = entry.dyn_into::<js_sys::Array>().ok()?;
            let path = entry.get(0).as_string()?;
            Some((PathBuf::from(path), js_sys::Uint8Array::new(&entry.get(1)).to_vec()))
        })
        .collect()
}

/// Every write of [`editor_core::files`]: kept in IndexedDB, and a user's map also
/// written to their file. The autosaver's backups beside it stay in the browser. A
/// deleted file leaves IndexedDB too.
fn written(path: &Path, contents: Option<&[u8]>) {
    let key = path.to_string_lossy();
    let Some(contents) = contents else {
        delete_file(&key);
        return;
    };
    store_file(&key, contents);
    if path.parent() == Some(Path::new(FILES_DIR)) || path.parent().and_then(Path::parent) == Some(Path::new(PICKED_DIR)) {
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let _ = save_user_file(&key, &name, contents);
    }
}

/// Opens the browser's chooser for a file to open or a place to save, and hands the path
/// to [`crate::file_dialog`] under `key` once the user has answered. A picked file is in
/// [`editor_core::files`] by then.
pub fn choose_file(key: String, save: bool, extensions: Vec<String>, suggested: Option<String>) {
    wasm_bindgen_futures::spawn_local(async move {
        let path = if save { save_target(&extensions, suggested).await } else { open_file(extensions).await };
        crate::file_dialog::deliver(key, path);
        CONTEXT.with(|context| {
            if let Some(context) = &*context.borrow() {
                context.request_repaint();
            }
        });
    });
}

/// Opens the browser's folder chooser and hands the folder's path to
/// [`crate::file_dialog`] under `key` once the user has answered. Its files are in
/// [`editor_core::files`] and kept in IndexedDB by then, under [`FILES_DIR`].
pub fn choose_folder(key: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let folder = async {
            let picked = pick_folder().await.dyn_into::<js_sys::Array>().ok()?;
            let name = picked.get(0).as_string()?;
            let root = PathBuf::from(picked_file_path(FILES_DIR, &name).await.as_string()?);
            for (relative, contents) in pairs(picked.get(1)) {
                let path = root.join(relative);
                store_file(&path.to_string_lossy(), &contents);
                editor_core::files::store(path, contents);
            }
            Some(root)
        }.await;
        crate::file_dialog::deliver(key, folder);
        CONTEXT.with(|context| {
            if let Some(context) = &*context.borrow() {
                context.request_repaint();
            }
        });
    });
}

async fn open_file(extensions: Vec<String>) -> Option<PathBuf> {
    let picked = pick_open(extensions).await.dyn_into::<js_sys::Array>().ok()?;
    let name = picked.get(0).as_string()?;
    let contents = js_sys::Uint8Array::new(&picked.get(1)).to_vec();
    let path = PathBuf::from(picked_file_path(PICKED_DIR, &name).await.as_string()?);
    adopt_handle(&path.to_string_lossy());
    store_file(&path.to_string_lossy(), &contents);
    editor_core::files::store(path.clone(), contents);
    Some(path)
}

async fn save_target(extensions: &[String], suggested: Option<String>) -> Option<PathBuf> {
    let fallback = format!("untitled.{}", extensions.first().map_or("map", String::as_str));
    let mut name = pick_save(&suggested.unwrap_or(fallback), extensions.to_vec()).await.as_string()?;
    // The path the editor will write to, extension and all, is the one the picked file
    // goes with.
    let has_extension = Path::new(&name).extension().is_some_and(|ext| extensions.iter().any(|wanted| ext.eq_ignore_ascii_case(wanted)));
    if !has_extension {
        if let Some(extension) = extensions.first() {
            name = format!("{name}.{extension}");
        }
    }
    let path = PathBuf::from(picked_file_path(PICKED_DIR, &name).await.as_string()?);
    adopt_handle(&path.to_string_lossy());
    Some(path)
}

/// Debounce scene exports until editing settles. No browser/disk writes are involved.
struct Playtest {
    revision: Option<(u64, u64, u64)>,
    changed: web_time::Instant,
    sent: Option<(u64, u64, u64)>,
    in_flight: Option<(u64, u64, u64)>,
    file_revision: u64,
    worker_epoch: u32,
}
thread_local! {
    static PLAYTEST: RefCell<Playtest> = RefCell::new(Playtest { revision: None, changed: web_time::Instant::now(), sent: None, in_flight: None, file_revision: 0, worker_epoch: 0 });
}
/// Open the viewer from a user click so popup permission applies.
pub fn open_playtest() {
    if js_open_playtest() {
        PLAYTEST.with(|slot| slot.borrow_mut().sent = None);
        CONTEXT.with(|slot| { if let Some(ctx) = slot.borrow().as_ref() { ctx.request_repaint(); } });
    } else { editor_core::complain!("The browser blocked the playtest tab. Allow popups for this editor."); }
}
/// Send an authoring snapshot to the export worker after changes stop for 300 ms.
pub fn update_playtest(ctx: &egui::Context, document: &editor_core::document::Document) {
    if !playtest_active() {
        cancel_playtest_export(true);
        PLAYTEST.with(|slot| { let mut state = slot.borrow_mut(); state.in_flight = None; state.sent = None; });
        return;
    }
    PLAYTEST.with(|slot| {
        let mut state = slot.borrow_mut();
        let revision = (document.generation(), document.content_revision(), document.materials_revision());
        if state.revision != Some(revision) {
            cancel_playtest_export(false);
            state.in_flight = None;
            state.revision = Some(revision); state.changed = web_time::Instant::now();
        }
        let answer = take_playtest_export();
        if !answer.is_undefined() && !answer.is_null() {
            if state.in_flight == Some(revision) {
                if let Some(error) = js_sys::Reflect::get(&answer, &"error".into()).ok().and_then(|v|v.as_string()) {
                    editor_core::complain!("Playtest: {error}");
                } else {
                    if let Ok(warnings) = js_sys::Reflect::get(&answer, &"warnings".into()) {
                        for warning in js_sys::Array::from(&warnings).iter().filter_map(|v|v.as_string()) { editor_core::say!("Playtest: {warning}"); }
                    }
                    if !publish_playtest_export(&answer) { state.sent = None; }
                }
            }
            state.in_flight = None;
        }
        if playtest_export_busy() { ctx.request_repaint_after(web_time::Duration::from_millis(40)); }
        if state.sent == Some(revision) { return; }
        let delay = web_time::Duration::from_millis(300);
        if state.changed.elapsed() < delay {
            ctx.request_repaint_after(delay - state.changed.elapsed()); return;
        }
        let settings = editor_core::export_settings::Settings::for_map(document.path.as_deref());
        if let Err(error) = settings.validate() { state.sent = Some(revision); editor_core::complain!("Playtest: {error}"); return; }
        let mut options = settings.for_target(editor_core::export::Target::ThreeJs);
        options.name = document.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "untitled".into());
        let directory = document.path.as_ref().and_then(|p| p.parent()).unwrap_or(Path::new(FILES_DIR));
        let prepare = web_time::Instant::now();
        let snapshot = match serde_json::to_string(&document.export_snapshot(&directory.join(format!("{}.map",options.name)))) {
            Ok(value) => value, Err(error) => { state.sent = Some(revision); editor_core::complain!("Playtest snapshot: {error}"); return; }
        };
        let epoch = playtest_export_epoch();
        if epoch != state.worker_epoch { state.worker_epoch = epoch; state.file_revision = 0; }
        let mut preferences_changed = false;
        if state.file_revision > 0 {
            let preferences = editor_core::preferences::config_dir().join("preferences");
            editor_core::files::visit_changes_since(state.file_revision, |path, _| {
                preferences_changed |= path == preferences;
            });
        }
        if preferences_changed {
            // A worker may have cached process-wide preferences. Rebootstrap it
            // after configuration changes so game mounts and models stay current.
            cancel_playtest_export(true);
            state.worker_epoch = playtest_export_epoch();
            state.file_revision = 0;
        }
        let files = js_sys::Array::new();
        let file_revision = editor_core::files::visit_changes_since(state.file_revision, |path, bytes| {
            let pair = js_sys::Array::new(); pair.push(&path.to_string_lossy().as_ref().into());
            match bytes {
                Some(bytes) => { let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32); copy.copy_from(bytes); pair.push(&copy); },
                None => { pair.push(&wasm_bindgen::JsValue::NULL); },
            }
            files.push(&pair);
        });
        let options = serde_json::to_string(&options).expect("export options serialize");
        let key = format!("{}:{}", document.generation(), document.path.as_ref().map(|p|p.to_string_lossy().into_owned()).unwrap_or_else(||"untitled".into()));
        if submit_playtest_export(&snapshot, &directory.to_string_lossy(), &options, files, &key, prepare.elapsed().as_secs_f64()*1000.0) {
            state.file_revision = file_revision;
            state.sent = Some(revision); state.in_flight = Some(revision);
            ctx.request_repaint_after(web_time::Duration::from_millis(40));
        }
    });
}
