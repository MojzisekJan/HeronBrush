//! Android: the entry the NativeActivity library calls, and what the platform gives an
//! app instead of a home directory, an executable folder and system file dialogs.
//!
//! - The profile (preferences, recent maps, examples, autosaves) lives in the activity's
//!   internal files directory; maps the user makes go to `files/maps` by default.
//! - The data a desktop package ships beside the executable (`assets/`, `games/`) is
//!   packed into the APK's assets by `scripts/build-android.sh` and copied out on the first
//!   run after each install or update.
//! - Files are chosen with the in-app browser ([`crate::file_dialog`]) over app storage,
//!   the shared Documents and Download folders, and the app's folder on shared storage.
//! - The rest of the application is the desktop one: the window, the views, the touch
//!   gestures ([`crate::touch`]) and the compact layout follow the screen, not the platform.
//!
//! See docs/android.md.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use eframe::egui;
pub use winit::platform::android::activity::AndroidApp;

/// The activity, for the frames' questions about the screen and the keyboard.
static APP: OnceLock<AndroidApp> = OnceLock::new();

/// The application, from `android_main`. Returns when the activity is destroyed; the
/// process then exits, since winit's event loop cannot be created a second time in it.
pub fn run(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("HeronBrush"));
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));

    let files = app.internal_data_path().unwrap_or_else(|| PathBuf::from("/data/local/tmp/heron-brush"));
    let install = files.join("bundle");
    let temporary = files.join("tmp");
    let _ = std::fs::create_dir_all(&temporary);
    // `std::env::temp_dir` is /data/local/tmp otherwise, which an app cannot write. Set
    // before any other thread starts.
    std::env::set_var("TMPDIR", &temporary);
    unpack_bundle(&app, &install);
    editor_core::preferences::set_platform_directories(files.join("config"), install);
    let maps = files.join("maps");
    let _ = std::fs::create_dir_all(&maps);
    crate::file_dialog::set_places(places(&app, &maps));
    first_run_preferences();
    let _ = APP.set(app.clone());

    let viewport = egui::ViewportBuilder::default().with_title("Heron Brush");
    let mut options = eframe::NativeOptions { android_app: Some(app), viewport, ..Default::default() };
    #[cfg(feature = "wgpu")]
    {
        options.renderer = eframe::Renderer::Wgpu;
        crate::request_device(&mut options.wgpu_options);
    }
    #[cfg(not(feature = "wgpu"))]
    {
        options.renderer = eframe::Renderer::Glow;
    }
    let result = eframe::run_native("heron-brush", options, Box::new(|cc| crate::create_app(cc, crate::Startup::default())));
    if let Err(error) = result {
        log::error!("the editor stopped: {error}");
    }
    std::process::exit(0);
}

/// The places the file browser lists: the app's own maps, the examples, the shared
/// Documents and Download folders, and the app's folder on shared storage, which a
/// computer sees over USB. Shared folders an app may not read are left out.
fn places(app: &AndroidApp, maps: &Path) -> Vec<crate::file_dialog::Place> {
    let mut places = vec![("Heron Brush".to_string(), maps.to_path_buf()), (editor_core::i18n::tr("Examples").to_string(), editor_core::samples::directory())];
    let shared = Path::new("/storage/emulated/0");
    let documents = shared.join("Documents/Heron Brush");
    if std::fs::create_dir_all(&documents).is_ok() {
        places.push(("Documents".to_string(), documents));
    }
    let download = shared.join("Download");
    if std::fs::read_dir(&download).is_ok() {
        places.push(("Download".to_string(), download));
    }
    if let Some(external) = app.external_data_path() {
        if std::fs::create_dir_all(&external).is_ok() {
            places.push(("Android/data".to_string(), external));
        }
    }
    places
}

/// The first run on a phone starts with one view at a time (the 3D view; the label in
/// its corner steps through the others) rather than four panes on a small screen.
fn first_run_preferences() {
    let path = editor_core::preferences::Preferences::default_path();
    if path.exists() {
        return;
    }
    let mut preferences = editor_core::preferences::Preferences::default();
    preferences.map_view_layout = editor_core::preferences::Layout::One;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = preferences.save(&path) {
        log::warn!("could not write the first preferences: {error}");
    }
}

/// Copies the data packed into the APK (`assets/heron-bundle/…`, listed in its
/// `index.txt`) to `install`, unless the copy there came from the same package. The index
/// starts with a stamp line the build script makes unique per build.
fn unpack_bundle(app: &AndroidApp, install: &Path) {
    let assets = app.asset_manager();
    let read = |name: &str| -> Option<Vec<u8>> {
        let name = std::ffi::CString::new(format!("heron-bundle/{name}")).ok()?;
        let mut asset = assets.open(&name)?;
        let mut bytes = Vec::new();
        asset.read_to_end(&mut bytes).ok()?;
        Some(bytes)
    };
    let Some(index) = read("index.txt") else {
        log::warn!("the APK carries no heron-bundle/index.txt: no assets or games");
        return;
    };
    let stamp = install.join("index.txt");
    if std::fs::read(&stamp).ok().as_deref() == Some(&index[..]) {
        return;
    }
    let text = String::from_utf8_lossy(&index);
    let mut copied = 0;
    for name in text.lines().skip(1).filter(|line| !line.is_empty() && !line.contains("..")) {
        let Some(bytes) = read(name) else {
            log::warn!("{name} is listed but not in the APK");
            continue;
        };
        let target = install.join(name);
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&target, bytes) {
            Ok(()) => copied += 1,
            Err(error) => log::warn!("{}: {error}", target.display()),
        }
    }
    let _ = std::fs::write(&stamp, &index);
    log::info!("unpacked {copied} bundled files to {}", install.display());
}

/// What the system covers of the window (the keyboard, a cut-out), from the activity's
/// content rectangle, as egui's safe area: the app lays itself out inside
/// `Context::content_rect`. Usually nothing, since the activity is laid out between the
/// system bars and resized for the keyboard (docs/android.md).
pub fn covered(ctx: &egui::Context, raw: &egui::RawInput) -> egui::SafeAreaInsets {
    let Some(app) = APP.get() else { return egui::SafeAreaInsets::default() };
    let rect = app.content_rect();
    let ppp = ctx.pixels_per_point();
    let Some(screen) = raw.screen_rect.or_else(|| Some(ctx.viewport_rect())) else { return egui::SafeAreaInsets::default() };
    let (width, height) = (screen.width() * ppp, screen.height() * ppp);
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return egui::SafeAreaInsets::default();
    }
    let side = |pixels: f32| pixels.max(0.0) / ppp;
    egui::SafeAreaInsets(egui::epaint::MarginF32 {
        left: side(rect.left as f32),
        top: side(rect.top as f32),
        right: side(width - rect.right as f32),
        bottom: side(height - rect.bottom as f32),
    })
}

/// Shows the soft keyboard while a text field has the keyboard focus and hides it when
/// none has. winit asks for it "implicitly", which some keyboards ignore when no Android
/// text view is focused, so this asks explicitly. Call once a frame.
pub fn soft_keyboard(ctx: &egui::Context) {
    static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let Some(app) = APP.get() else { return };
    let wanted = ctx.egui_wants_keyboard_input();
    if SHOWN.swap(wanted, std::sync::atomic::Ordering::Relaxed) != wanted {
        if wanted {
            app.show_soft_input(false);
        } else {
            app.hide_soft_input(false);
        }
    }
}
