//! Finder document delivery, including events received before the first frame.
use eframe::egui;
use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSArray, NSURL};
use std::{cell::RefCell, path::PathBuf};

thread_local! {
    static OPEN: RefCell<(Option<PathBuf>, Option<egui::Context>)> = const { RefCell::new((None, None)) };
}

// winit 0.30.13 requires its own delegate instance for lifecycle handling.
// Extend its class with the missing document callback without replacing that delegate.
#[link(name = "objc")]
extern "C" {
    fn class_addMethod(
        class: *const objc2::runtime::AnyClass,
        selector: objc2::runtime::Sel,
        implementation: unsafe extern "C" fn(),
        types: *const std::ffi::c_char,
    ) -> objc2::runtime::Bool;
}

extern "C" fn open_urls(
    _delegate: &objc2::runtime::AnyObject,
    _selector: objc2::runtime::Sel,
    _app: &NSApplication,
    urls: &NSArray<NSURL>,
) {
    for url in urls {
        // SAFETY: NSURL getters are called on valid URLs supplied by AppKit.
        let path = unsafe {
            if !url.isFileURL() {
                continue;
            }
            url.path()
        };
        let Some(path) = path else {
            continue;
        };
        let path = PathBuf::from(path.to_string());
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("map"))
        {
            continue;
        }
        OPEN.with(|state| {
            let mut state = state.borrow_mut();
            state.0 = Some(path);
            if let Some(ctx) = &state.1 {
                ctx.request_repaint();
            }
        });
    }
}

pub fn take_open(ctx: &egui::Context) -> Option<PathBuf> {
    OPEN.with(|state| {
        let mut state = state.borrow_mut();
        state.1 = Some(ctx.clone());
        state.0.take()
    })
}

pub fn run(options: eframe::NativeOptions, creator: eframe::AppCreator<'_>) -> eframe::Result {
    let event_loop =
        winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build()?;
    let mtm = MainThreadMarker::new().expect("macOS event loop must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: winit has installed its delegate during EventLoop::build. The
    // callback has Objective-C's void(id, SEL, id, id) ABI, encoded as v@:@@.
    // Its class lives for the process lifetime; registration precedes event delivery.
    unsafe {
        let delegate = app.delegate().expect("winit application delegate");
        let class: &objc2::runtime::AnyClass = objc2::msg_send![&*delegate, class];
        let implementation = std::mem::transmute::<
            extern "C" fn(
                &objc2::runtime::AnyObject,
                objc2::runtime::Sel,
                &NSApplication,
                &NSArray<NSURL>,
            ),
            unsafe extern "C" fn(),
        >(open_urls);
        assert!(
            class_addMethod(
                class,
                objc2::sel!(application:openURLs:),
                implementation,
                b"v@:@@\0".as_ptr().cast(),
            )
            .as_bool(),
            "could not register Finder document callback"
        );
    }
    let mut native = eframe::create_native("heron-brush", options, creator, &event_loop);
    let result = event_loop.run_app(&mut native);
    result?;
    Ok(())
}
