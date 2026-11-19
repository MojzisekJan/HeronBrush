//! Native iPad entry point. The app bundle is read-only; user data lives in its sandbox.
use std::path::PathBuf;

/// Starts the editor with Metal and the shared touch interface.
pub fn run() -> eframe::Result {
    let home = PathBuf::from(std::env::var_os("HOME").expect("iOS application home"));
    let documents = home.join("Documents");
    let config = home.join("Library/Application Support/Heron Brush");
    let bundle = std::env::current_exe()
        .expect("application executable")
        .parent()
        .expect("application bundle")
        .to_path_buf();
    std::fs::create_dir_all(&documents).expect("create document directory");
    std::fs::create_dir_all(&config).expect("create configuration directory");
    editor_core::preferences::set_platform_directories(config, bundle);
    crate::file_dialog::set_places(vec![
        ("Documents".into(), documents),
        ("Examples".into(), editor_core::samples::directory()),
    ]);
    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default().with_title("Heron Brush"),
        ..Default::default()
    };
    crate::request_device(&mut options.wgpu_options);
    eframe::run_native(
        "heron-brush",
        options,
        Box::new(|cc| crate::create_app(cc, crate::Startup::default())),
    )
}

/// Retains the UIKit view for main-thread geometry queries throughout the app lifetime.
pub(super) struct View {
    view: objc2::rc::Retained<objc2::runtime::AnyObject>,
    system_insets: std::cell::Cell<eframe::egui::SafeAreaInsets>,
}

impl View {
    pub(super) fn from_context(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let RawWindowHandle::UiKit(handle) = cc.window_handle().ok()?.as_raw() else {
            return None;
        };
        // SAFETY: eframe's creation callback runs on UIKit's main thread with a live
        // UIView. Retaining it keeps the object valid until the app is dropped there.
        unsafe {
            objc2::rc::Retained::retain(handle.ui_view.as_ptr().cast()).map(|view| Self {
                view,
                system_insets: Default::default(),
            })
        }
    }

    pub(super) fn keyboard_insets(
        &self,
        ctx: &eframe::egui::Context,
        raw: &mut eframe::egui::RawInput,
    ) {
        use objc2::msg_send;
        use objc2_foundation::CGRect;
        // SAFETY: raw_input_hook runs on the same UIKit main thread as creation.
        // All receivers are retained/live UIKit objects; these geometry APIs are
        // available since iOS 15, below our iOS 16 deployment target.
        let (bounds, keyboard): (CGRect, CGRect) = unsafe {
            let guide: *mut objc2::runtime::AnyObject = msg_send![&*self.view, keyboardLayoutGuide];
            let _: () = msg_send![&*self.view, layoutIfNeeded];
            (
                msg_send![&*self.view, bounds],
                msg_send![guide, layoutFrame],
            )
        };
        let rect = |r: CGRect| {
            eframe::egui::Rect::from_min_size(
                eframe::egui::pos2(r.origin.x as f32, r.origin.y as f32),
                eframe::egui::vec2(r.size.width as f32, r.size.height as f32),
            )
        };
        let bottom =
            crate::touch_keyboard::docked_inset(rect(bounds), rect(keyboard), ctx.zoom_factor());
        if let Some(insets) = raw.safe_area_insets {
            self.system_insets.set(insets);
        }
        let mut insets = self.system_insets.get();
        insets.0.bottom = insets.0.bottom.max(bottom);
        raw.safe_area_insets = Some(insets);
        // Keyboard layout can animate without winit emitting a resize. Poll only
        // during text input / while covered, leaving an idle editor asleep.
        if ctx.egui_wants_keyboard_input() || bottom > 32.0 / ctx.zoom_factor() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
