//! The GL calls that differ between a native GL and WebGL 2.
//!
//! A view on GL makes a glow context of its own over the GL eframe opened, draws into a
//! framebuffer of its own and blits it into egui's target in a paint callback. Natively the
//! context comes from eframe's loader and egui's context binds the framebuffer by its
//! name. In the browser the context is made over the canvas's WebGL 2 context, and since
//! two glow contexts there share no names, the callback blits with the view's own
//! context, found through [`publish`] (docs/web.md).

use crate::view3d::ProcAddress;

/// A glow context of the views' own on the GL eframe opened.
///
/// # Safety
/// `proc_address` must be eframe's loader for a GL context current on this thread.
pub unsafe fn context(proc_address: &ProcAddress) -> glow::Context {
    #[cfg(not(target_arch = "wasm32"))]
    return unsafe { glow::Context::from_loader_function_cstr(|name| proc_address(name)) };
    #[cfg(target_arch = "wasm32")]
    {
        use eframe::wasm_bindgen::JsCast as _;
        let _ = proc_address;
        let context = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(crate::web::CANVAS_ID))
            .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .and_then(|canvas| canvas.get_context("webgl2").ok().flatten())
            .and_then(|context| context.dyn_into::<web_sys::WebGl2RenderingContext>().ok())
            .expect("eframe draws the page's canvas with WebGL 2");
        glow::Context::from_webgl2_context(context)
    }
}

/// The same in eframe's glow, whose context egui paints with.
#[cfg(not(target_arch = "wasm32"))]
pub fn eframe_framebuffer(binding: i32) -> Option<eframe::glow::Framebuffer> {
    std::num::NonZeroU32::new(binding as u32).map(eframe::glow::NativeFramebuffer)
}

/// A framebuffer's name, which egui's context can bind it by.
#[cfg(not(target_arch = "wasm32"))]
pub fn name(framebuffer: glow::Framebuffer) -> i32 {
    framebuffer.0.get() as i32
}

#[cfg(target_arch = "wasm32")]
pub use web::{blit, publish};

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

    use glow::HasContext as _;

    thread_local! {
        /// Each view's context and the framebuffer its picture is in, by the view's key.
        static PICTURES: RefCell<HashMap<u64, (Weak<glow::Context>, glow::Framebuffer)>> = RefCell::new(HashMap::new());
    }

    /// Where the view `key` keeps its picture, for [`blit`] in the paint callback, which
    /// may not hold the context itself.
    pub fn publish(key: u64, gl: &Rc<glow::Context>, framebuffer: glow::Framebuffer) {
        PICTURES.with(|pictures| {
            pictures
                .borrow_mut()
                .insert(key, (Rc::downgrade(gl), framebuffer))
        });
    }

    /// Copies the view `key`'s picture, `width` by `height`, into egui's target at
    /// `viewport`. Both glow contexts drive the same WebGL context, so the target egui
    /// bound is the one drawn to.
    pub fn blit(
        key: u64,
        width: i32,
        height: i32,
        viewport: eframe::egui::epaint::ViewportInPixels,
    ) {
        let Some((gl, framebuffer)) = PICTURES.with(|pictures| {
            pictures
                .borrow()
                .get(&key)
                .and_then(|(gl, framebuffer)| Some((gl.upgrade()?, *framebuffer)))
        }) else {
            return;
        };
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(framebuffer));
            gl.blit_framebuffer(
                0,
                0,
                width,
                height,
                viewport.left_px,
                viewport.from_bottom_px,
                viewport.left_px + viewport.width_px,
                viewport.from_bottom_px + viewport.height_px,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        }
    }
}
