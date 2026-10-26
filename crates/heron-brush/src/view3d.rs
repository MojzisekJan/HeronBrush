//! A 3D view in egui: the core's [`Scene3d`] drawing through the shared renderer into a
//! framebuffer of its own, shown in egui by a GPU-side blit, with egui's pointer and keys
//! forwarded to its tools.
//!
//! The renderer needs `glow` 0.18 and eframe 0.36 is built on 0.17, so each view makes its
//! own 0.18 context from eframe's `get_proc_address`. Both wrap the same native GL
//! context, which eframe shares between all of its windows, so a framebuffer made here is
//! valid in the paint callback of any window.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use editor_core::camera3d::Camera3d;
use editor_core::view3d::{Scene3d, Scene3dParts};
use eframe::egui;
use eframe::egui_glow;
use glow::HasContext;

use crate::editor::Editor;
use crate::egui_canvas::EguiCanvas;
use crate::view2d::modifiers;

/// The loader eframe hands out, kept so a view opened later can build its own context.
pub type ProcAddress = Arc<dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void + Send + Sync>;

/// The GPU eframe started with: GL (the shared `render` crate draws), or with the `wgpu`
/// feature, wgpu (`render_wgpu` draws).
#[derive(Clone)]
pub enum Gpu {
    /// eframe's glow renderer; views make their own glow 0.18 contexts from this.
    Gl(ProcAddress),
    /// eframe's wgpu renderer.
    #[cfg(feature = "wgpu")]
    Wgpu(eframe::egui_wgpu::RenderState),
}

impl Gpu {
    /// What eframe made at start-up.
    pub fn from_creation_context(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        #[cfg(feature = "wgpu")]
        if let Some(state) = &cc.wgpu_render_state {
            return Ok(Gpu::Wgpu(state.clone()));
        }
        // The browser has no loader: a view's context is made over the canvas instead
        // (`gl_glue::context`).
        #[cfg(target_arch = "wasm32")]
        if cc.gl.is_some() {
            return Ok(Gpu::Gl(Arc::new(|_| std::ptr::null())));
        }
        cc.get_proc_address
            .clone()
            .map(Gpu::Gl)
            .ok_or_else(|| "eframe started with no usable renderer".to_string())
    }
}

/// The framebuffer the renderer draws into.
struct Target {
    framebuffer: glow::Framebuffer,
    color: glow::Renderbuffer,
    depth: glow::Renderbuffer,
    size: (i32, i32),
}

/// The buttons a view is being dragged with, and where each press began.
#[derive(Default)]
struct Presses {
    primary: Option<egui::Pos2>,
    /// Whether the primary press held now is the second of a double click.
    double: bool,
    clicks: crate::view2d::DoublePress,
    secondary: Option<egui::Pos2>,
    /// What an Alt+right drag orbits about, while it lasts.
    orbit: Option<geometry_math::vec::Vec3d>,
    middle: Option<egui::Pos2>,
    middle_dragged: bool,
}

/// The GL side of a view: the shared renderer drawing into a framebuffer of its own,
/// made with the renderer's context (`gl`), since in WebGL a glow context cannot use
/// objects another one made.
struct GlView {
    gl: Rc<glow::Context>,
    renderer: render::Renderer,
    target: Option<Target>,
    /// The picture the framebuffer holds ([`editor_core::view3d::picture_key`]), so a
    /// frame that would draw the same picture shows it again instead: egui repaints the
    /// whole window for a tooltip or the autosave timer, and the 3D view is most of the
    /// cost of a frame on a software renderer.
    drawn: std::cell::Cell<Option<u64>>,
    /// `GL_TIME_ELAPSED` round the drawing, only while profiling.
    gpu_timer: Option<crate::profile::GpuTimer>,
    /// The GPU time the timer last measured, in milliseconds.
    gpu_ms: Option<f64>,
}

/// Which renderer draws the view.
enum Backend {
    Gl(Box<GlView>),
    #[cfg(feature = "wgpu")]
    Wgpu(Box<crate::view3d_wgpu::WgpuView>),
    /// No renderer this GPU can run (wgpu on GLES, a GL that would not compile the
    /// shaders): the view says so, and the rest of the editor works.
    Unavailable(String),
}

/// A 3D view with its own camera and GPU resources.
pub struct View3d {
    target_pick: Rc<RefCell<crate::editor::TargetPick>>,
    pub animation: crate::animation_gizmo::Gizmo,
    /// The paint handles on the selected faces.
    paint: crate::paint_gizmo::PaintGizmo,
    /// The texture handles on the face being edited.
    texture: crate::texture_gizmo::TextureGizmo,
    backend: Backend,
    /// What the view draws and what its input does.
    pub scene: Rc<Scene3d>,
    presses: Presses,
    hovered: bool,
    modifiers: editor_core::input::Modifiers,
    /// The document generation the camera was last framed for.
    framed: Option<u64>,
    /// A right-click that stayed put: where, and what it was over. A right-drag turns
    /// the camera, so only a click opens the menu.
    pub context_click: Option<(egui::Pos2, editor_core::view2d::Probe)>,
    pub material_paste: Option<editor_core::document::FaceRef>,
    /// How far ahead the touch gestures orbit about, while one goes on.
    touch_pivot: Option<f64>,
}

/// An egui key, by what it means to the core.
pub fn key(key: egui::Key) -> editor_core::input::Key {
    use editor_core::input::Key as K;
    use egui::Key;
    let name = key.name();
    if name.len() == 1 {
        let c = name.chars().next().unwrap_or(' ');
        if c.is_ascii_alphabetic() {
            return K::Letter(c.to_ascii_lowercase());
        }
        if c.is_ascii_digit() {
            return K::Digit(c as u8 - b'0');
        }
    }
    match key {
        Key::Escape => K::Escape,
        Key::Enter => K::Enter,
        Key::Tab => K::Tab,
        Key::Space => K::Space,
        Key::Backspace => K::Backspace,
        Key::Delete => K::Delete,
        Key::Insert => K::Insert,
        Key::Home => K::Home,
        Key::End => K::End,
        Key::PageUp => K::PageUp,
        Key::PageDown => K::PageDown,
        Key::ArrowLeft => K::Left,
        Key::ArrowRight => K::Right,
        Key::ArrowUp => K::Up,
        Key::ArrowDown => K::Down,
        Key::Plus | Key::Equals => K::Plus,
        Key::Minus => K::Minus,
        Key::OpenBracket => K::BracketLeft,
        Key::CloseBracket => K::BracketRight,
        Key::Comma => K::Comma,
        Key::Period => K::Period,
        Key::Slash => K::Slash,
        _ => name
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .map_or(K::Other, K::Function),
    }
}

impl View3d {
    /// Sets the renderer up in the GL context eframe made current, with `camera` for this
    /// view and everything else shared with the editor.
    pub fn new(gpu: &Gpu, editor: &Editor, camera: Rc<RefCell<Camera3d>>) -> Result<Self, String> {
        let scene = Rc::new(Scene3d::new(Scene3dParts {
            document: Rc::clone(&editor.document),
            tool: Rc::clone(&editor.tool),
            handles: Rc::clone(&editor.handles),
            rotate: Rc::clone(&editor.rotate),
            camera,
            fillet: Rc::clone(&editor.fillet),
            pending_shape: Rc::clone(&editor.pending_shape),
            shape_parameters: Rc::clone(&editor.shape_parameters),
            clip: Rc::clone(&editor.clip),
            assemble: Rc::clone(&editor.assemble),
            grid: Rc::clone(&editor.grid),
            filters: Rc::clone(&editor.filters),
            sweep: Rc::clone(&editor.sweep),
            paste_target: Rc::clone(&editor.paste_target),
            refresh_panels: Rc::clone(&editor.refresh_panels),
            redraw: Rc::clone(&editor.redraw),
            render: Rc::clone(&editor.redraw),
            focus: Rc::new(|| {}),
        }));
        let backend = match gpu {
            Gpu::Gl(proc_address) => {
                // SAFETY: Views are made in eframe's creation callback or in a frame's
                // update, both of which run with eframe's GL context current on this
                // thread, and `proc_address` is eframe's loader for it
                // (`Gpu::from_creation_context`).
                let make = || unsafe { crate::gl_glue::context(proc_address) };
                match render::Renderer::new(
                    make(),
                    editor_core::render_settings::from_preferences(),
                ) {
                    Ok(mut renderer) => {
                        scene.renderer_created(&mut renderer);
                        Backend::Gl(Box::new(GlView {
                            gl: Rc::clone(renderer.gl()),
                            renderer,
                            target: None,
                            drawn: std::cell::Cell::new(None),
                            gpu_timer: crate::profile::GpuTimer::from_env(),
                            gpu_ms: None,
                        }))
                    }
                    Err(why) => {
                        let why = editor_core::i18n::fill(
                            "The 3D view cannot run on this GPU: {}. The 2D views work.",
                            &[&why.to_string()],
                        );
                        editor_core::complain!("{why}");
                        Backend::Unavailable(why)
                    }
                }
            }
            #[cfg(feature = "wgpu")]
            Gpu::Wgpu(state) => match crate::view3d_wgpu::unsupported(state) {
                Some(why) => {
                    editor_core::complain!("{why}");
                    Backend::Unavailable(why)
                }
                None => Backend::Wgpu(Box::new(crate::view3d_wgpu::WgpuView::new(state))),
            },
        };
        Ok(Self {
            target_pick: Rc::clone(&editor.target_pick),
            animation: crate::animation_gizmo::Gizmo::linked(&editor.animation_link),
            paint: Default::default(),
            texture: Default::default(),
            backend,
            scene,
            presses: Presses::default(),
            hovered: false,
            modifiers: Default::default(),
            framed: None,
            context_click: None,
            material_paste: None,
            touch_pivot: None,
        })
    }

    /// Lays the view out in `rect`: input, then a frame, then the HUD.
    pub fn show(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Point the camera at a newly opened map.
        let generation = self.scene.document.borrow().generation();
        if self.framed != Some(generation) {
            if let Some(bounds) = self.scene.document.borrow().bounds() {
                self.scene.camera.borrow_mut().frame_in_viewport(
                    &bounds,
                    f64::from(rect.width()),
                    f64::from(rect.height()),
                );
            }
            self.framed = Some(generation);
        }

        let response = ui.interact(
            rect,
            ui.scope_id()
                .with(("view3d", Rc::as_ptr(&self.scene) as usize)),
            egui::Sense::click_and_drag(),
        );
        let size = (f64::from(rect.width()), f64::from(rect.height()));
        let profiling = crate::profile::enabled();
        let started = web_time::Instant::now();
        let revision = self.scene.document.borrow().revision();
        // A press on a paint handle is the handle's, not a selection or a drag.
        let tool = *self.scene.tool.borrow();
        let camera = *self.scene.camera.borrow();
        let animating = self.scene.filters.borrow().animation_frame.is_some();
        let painting = !animating
            && response.hovered()
            && self
                .paint
                .press(ui, rect, &self.scene.document, tool, &camera);
        let texturing = !animating
            && !painting
            && !self.paint.busy()
            && self.texture.update(
                ui,
                rect,
                &self.scene.document,
                tool,
                &camera,
                response.hovered(),
            );
        self.handle_input(
            ui,
            &response,
            rect,
            size,
            painting || self.paint.busy() || texturing,
        );
        if let Some(motion) = crate::touch::take_motion(ui.ctx(), 0) {
            self.touch_motion(motion, size);
        }

        let ppp = ui.ctx().pixels_per_point();
        let scale = ppp.round().max(1.0) as i32;
        let drew = match &mut self.backend {
            Backend::Gl(view) => view.show(ui, rect, &self.scene, size, scale, profiling),
            #[cfg(feature = "wgpu")]
            Backend::Wgpu(view) => view.show(ui, rect, &self.scene, size, scale, profiling),
            Backend::Unavailable(why) => {
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, crate::theme::base());
                let galley = painter.layout(
                    why.clone(),
                    crate::theme::body(),
                    crate::theme::dim(),
                    (rect.width() - 32.0).max(80.0),
                );
                painter.galley(
                    egui::pos2(
                        rect.center().x - galley.size().x / 2.0,
                        crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
                    ),
                    galley,
                    crate::theme::dim(),
                );
                false
            }
        };
        if profiling {
            let changed = self.scene.document.borrow().revision() != revision;
            crate::profile::view3d_frame(started.elapsed(), changed, drew);
        }

        let camera = *self.scene.camera.borrow();
        let hidden =
            self.presses.primary.is_some() || self.scene.filters.borrow().animation_frame.is_some();
        self.texture.show(
            ui,
            rect,
            &self.scene.document,
            tool,
            &camera,
            hidden || self.paint.busy(),
        );
        self.paint.show(
            ui,
            rect,
            &self.scene.document,
            tool,
            &camera,
            hidden || self.texture.busy(),
        );
        self.animation.show(
            ui,
            rect,
            &self.scene.document,
            &self.scene.filters,
            &self.scene.grid,
            camera,
        );
        let hud = self.scene.hud.borrow();
        if !hud.hidden {
            let painter = ui.painter_at(rect);
            hud.draw(&mut EguiCanvas::new(&painter, rect), size);
        }
    }

    fn handle_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        size: (f64, f64),
        paint_handle: bool,
    ) {
        let scene = &self.scene;
        let local = |p: egui::Pos2| (f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y));
        let mods = modifiers(ui);
        let (pointer, moving, press_origin) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                crate::view2d::pointer_moved(i),
                i.pointer.press_origin(),
            )
        });
        let pressed = |button| ui.input(|i| i.pointer.button_pressed(button));
        let released = |button| ui.input(|i| i.pointer.button_released(button));

        // The primary button: the drag gesture, the click and the face-attribute stroke,
        // in that order for each press.
        let picking = self.target_pick.borrow().source.is_some();
        if picking && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.target_pick.borrow_mut().source = None;
        }
        if picking && response.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
            if let Some(at) = pointer {
                let (x, y) = local(at);
                self.target_pick.borrow_mut().picked = Some(scene.pick_node(x, y, size));
            }
        }
        if self.presses.primary.is_some()
            && ui.input(|i| {
                !i.focused || (!i.pointer.primary_down() && !i.pointer.primary_released())
            })
        {
            scene.drag_cancel();
            scene.stroke_cancel();
            self.presses.primary = None;
        }
        if pressed(egui::PointerButton::Primary)
            && !picking
            && !paint_handle
            && response.hovered()
            && scene.filters.borrow().animation_frame.is_none()
        {
            if let Some(at) = press_origin.or(pointer) {
                if !scene.edit_press() {
                    let (x, y) = local(at);
                    let double = self.presses.clicks.press(ui.input(|i| i.time), at);
                    self.presses.double = double;
                    scene.drag_begin(x, y, mods, size);
                    scene.stroke_begin(x, y, mods, size);
                    scene.click(if double { 2 } else { 1 }, x, y, mods, size);
                    self.presses.primary = Some(at);
                }
            }
        }
        if let (Some(origin), Some(at)) = (self.presses.primary, pointer) {
            let delta = at - origin;
            let (dx, dy) = (f64::from(delta.x), f64::from(delta.y));
            if moving {
                scene.drag_update(dx, dy, mods, size);
                scene.stroke_update(local(origin), dx, dy, size);
            }
            let (x, y) = local(at);
            let seconds = f64::from(ui.input(|i| i.stable_dt));
            if scene.stroke_hold(x, y, seconds, size) {
                ui.ctx().request_repaint();
            }
            if released(egui::PointerButton::Primary) {
                scene.drag_end(dx, dy, mods, size);
                scene.stroke_end();
                // The second click of a double click, let go where it was pressed:
                // `SelectionTool::mouseDoubleClick`.
                if self.presses.double
                    && delta.length() < editor_core::view3d::DRAG_THRESHOLD as f32
                {
                    let (x, y) = local(origin);
                    scene.double_click(x, y, mods, size);
                }
                self.presses.double = false;
                self.presses.primary = None;
            }
        }
        if self.presses.primary.is_some()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
            && scene.escape()
        {
            scene.drag_cancel();
            self.presses.primary = None;
        }

        // A class or a prefab dragged from a browser: the landing box follows the pointer,
        // and letting go puts it there.
        let hovering = response.dnd_hover_payload::<crate::widgets::Dragged>();
        let classname = hovering.as_deref().and_then(|dragged| match dragged {
            crate::widgets::Dragged::Class(name) => Some(name.clone()),
            crate::widgets::Dragged::Prefab(_) => None,
        });
        match (pointer, classname) {
            (Some(at), Some(name)) => {
                let (x, y) = local(at);
                scene.entity_drag_over(Some(&name), x, y, size);
            }
            _ => scene.entity_drag_leave(),
        }
        if let (Some(dropped), Some(at)) = (
            response.dnd_release_payload::<crate::widgets::Dragged>(),
            pointer,
        ) {
            let (x, y) = local(at);
            match &*dropped {
                crate::widgets::Dragged::Class(name) => {
                    scene.drop_entity(name, x, y, size);
                }
                crate::widgets::Dragged::Prefab(path) => {
                    scene.drop_prefab(path, x, y, size);
                }
            }
        }

        // The right button looks around, the middle one pans.
        if pressed(egui::PointerButton::Secondary) && response.hovered() {
            self.presses.secondary = press_origin.or(pointer);
            self.presses.orbit = None;
            match self.presses.secondary {
                Some(at) if mods.alt => {
                    let (x, y) = local(at);
                    self.presses.orbit = Some(scene.orbit_begin(x, y, size));
                }
                _ => scene.look_begin(),
            }
        }
        if let (Some(origin), Some(at)) = (self.presses.secondary, pointer) {
            let delta = at - origin;
            match self.presses.orbit {
                Some(center) => scene.orbit_update(center, f64::from(delta.x), f64::from(delta.y)),
                None => scene.look_update(f64::from(delta.x), f64::from(delta.y)),
            }
        }
        if released(egui::PointerButton::Secondary) {
            if let (Some(from), Some(at)) = (self.presses.secondary.take(), pointer) {
                if from.distance(at) <= 4.0 {
                    let (x, y) = local(at);
                    self.context_click = Some((at, scene.probe(x, y, size)));
                }
            }
        }
        if pressed(egui::PointerButton::Middle) && response.hovered() {
            scene.pan_begin();
            self.presses.middle = press_origin.or(pointer);
            self.presses.middle_dragged = false;
        }
        if let (Some(origin), Some(at)) = (self.presses.middle, pointer) {
            let delta = at - origin;
            self.presses.middle_dragged |= delta.length() > 4.0;
            if self.presses.middle_dragged {
                scene.pan_update(f64::from(delta.x), f64::from(delta.y), mods);
            }
        }
        if released(egui::PointerButton::Middle) {
            if self.presses.middle.is_some()
                && !self.presses.middle_dragged
                && response.hovered()
                && !picking
                && !mods.shift
                && !mods.control
                && !mods.alt
                && scene.filters.borrow().animation_frame.is_none()
            {
                if let Some(at) = pointer {
                    let (x, y) = local(at);
                    self.material_paste = editor_core::view3d::face_under(
                        &scene.camera.borrow(),
                        size,
                        &scene.document.borrow(),
                        x,
                        y,
                    );
                }
            }
            self.presses.middle = None;
        }
        if self.presses.middle.is_some()
            && ui.input(|i| {
                !i.focused
                    || (!i.pointer.button_down(egui::PointerButton::Middle)
                        && !i.pointer.button_released(egui::PointerButton::Middle))
            })
        {
            self.presses.middle = None;
        }

        if response.hovered() {
            // The wheel, one step a notch; the tools count a step towards the user as positive.
            // macOS turns Shift and the wheel into a sideways scroll, which is taken as the
            // wheel when Shift is held.
            let steps: Vec<f32> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y),
                        egui::Event::MouseWheel {
                            delta, modifiers, ..
                        } if modifiers.shift && delta.x != 0.0 => Some(delta.x),
                        _ => None,
                    })
                    .collect()
            });
            for step in steps {
                let dy = -f64::from(step.signum());
                if !scene.scroll_with(dy, mods, size) {
                    scene.scroll(dy, size);
                }
            }
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                if moving || !self.hovered {
                    let (x, y) = local(at);
                    scene.pointer_over();
                    scene.motion(x, y, mods, size);
                } else if mods != self.modifiers {
                    scene.modifiers_changed(mods, size);
                }
            }
            self.hovered = true;
        } else if self.hovered {
            self.hovered = false;
            scene.pointer_away();
            scene.leave();
        }
        self.modifiers = mods;
    }

    /// Orbits, pans and zooms the camera for the touch gestures. They turn about a point
    /// straight ahead: what the view's centre shows, taken when the gesture starts, or the
    /// map's middle when the centre shows nothing.
    fn touch_motion(&mut self, motion: crate::touch::Motion, size: (f64, f64)) {
        let scene = &self.scene;
        if motion.started || self.touch_pivot.is_none() {
            let ray = scene.ray(size.0 / 2.0, size.1 / 2.0, size);
            let hit = scene.document.borrow().pick(&ray).map(|hit| hit.distance);
            let camera = scene.camera.borrow();
            let middle = scene
                .document
                .borrow()
                .bounds()
                .map(|bounds| geometry_math::vec::length(bounds.center() - camera.position));
            self.touch_pivot = Some(hit.or(middle).unwrap_or(256.0).max(16.0));
        }
        let mut distance = self.touch_pivot.unwrap_or(256.0);
        {
            let mut camera = scene.camera.borrow_mut();
            let pivot = camera.position + camera.direction() * distance;
            if motion.orbit != egui::Vec2::ZERO {
                // The look speed preference, slowed: a finger crosses a phone in a moment,
                // and half the screen should turn the view a third of the way round, not more.
                let (yaw, pitch) = editor_core::view3d::turn_per_pixel();
                camera.turn(
                    f64::from(motion.orbit.x) * yaw * 0.6,
                    f64::from(motion.orbit.y) * pitch * 0.6,
                );
                camera.position = pivot - camera.direction() * distance;
            }
            if motion.pan != egui::Vec2::ZERO {
                // The fingers keep hold of the point they pan.
                let per_pixel = camera.world_units_per_pixel(pivot, size.1.max(1.0));
                camera.pan(
                    -f64::from(motion.pan.x) * per_pixel,
                    f64::from(motion.pan.y) * per_pixel,
                );
            }
            if motion.zoom.is_finite()
                && motion.zoom > 0.0
                && (motion.zoom - 1.0).abs() > f32::EPSILON
            {
                let pivot = camera.position + camera.direction() * distance;
                distance = (distance / f64::from(motion.zoom)).clamp(1.0, 1.0e6);
                camera.position = pivot - camera.direction() * distance;
            }
        }
        self.touch_pivot = Some(distance);
        scene.pointer_away();
        (scene.render)();
    }

    /// The GPU time of the last measured draw, in milliseconds, while profiling on GL.
    pub fn gpu_ms(&self) -> Option<f64> {
        match &self.backend {
            Backend::Gl(view) => view.gpu_ms,
            #[cfg(feature = "wgpu")]
            Backend::Wgpu(_) => None,
            Backend::Unavailable(_) => None,
        }
    }

    /// Frees everything on the GPU. With GL, the GL context must be current.
    pub fn destroy(self) {
        match self.backend {
            Backend::Gl(mut view) => {
                if let Some(target) = view.target.take() {
                    view.delete_target(target);
                }
                if let Some(timer) = view.gpu_timer.take() {
                    timer.destroy(&view.gl);
                }
                view.renderer.destroy();
            }
            #[cfg(feature = "wgpu")]
            Backend::Wgpu(_) => {}
            Backend::Unavailable(_) => {}
        }
    }
}

impl GlView {
    /// Draws the scene into the framebuffer and shows it in `rect`. Returns whether it
    /// drew a new picture.
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        scene: &Scene3d,
        size: (f64, f64),
        scale: i32,
        profiling: bool,
    ) -> bool {
        let width = (f64::from(rect.width()) * f64::from(scale))
            .round()
            .max(1.0) as i32;
        let height = (f64::from(rect.height()) * f64::from(scale))
            .round()
            .max(1.0) as i32;
        if self.target.as_ref().map(|t| t.size) != Some((width, height)) {
            self.resize(width, height);
        }
        let Some(target) = &self.target else {
            crate::theme::text(
                ui.painter(),
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "no framebuffer",
                egui::FontId::default(),
                crate::theme::error(),
            );
            return false;
        };
        let before = self.drawn.get();

        self.renderer
            .set_settings(editor_core::render_settings::from_preferences());
        // SAFETY: `show` runs inside egui's update, with eframe's GL context current on this
        // thread, and `self.gl` wraps that same native context. `target.framebuffer` is the live
        // framebuffer `resize` made with it, and the previous binding is put back.
        unsafe {
            // In WebGL the bound framebuffer may be egui's, which this context cannot name;
            // egui draws to the canvas there, so that is what goes back.
            #[cfg(not(target_arch = "wasm32"))]
            let previous = self
                .gl
                .get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            #[cfg(target_arch = "wasm32")]
            let previous = None;
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            if let Some(timer) = &mut self.gpu_timer {
                timer.begin(&self.gl);
            }
            scene.render_kept(&mut self.renderer, size.0, size.1, scale, Some(&self.drawn));
            if let Some(timer) = &mut self.gpu_timer {
                self.gpu_ms = timer.end(&self.gl);
            }
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, previous);
        }
        if scene.wants_frame(&self.renderer) {
            ui.ctx().request_repaint();
        }

        // The picture never leaves the GPU: the paint callback blits it into egui's target.
        #[cfg(target_arch = "wasm32")]
        let callback = {
            let key = Rc::as_ptr(&self.gl) as usize as u64;
            crate::gl_glue::publish(key, &self.gl, target.framebuffer);
            egui_glow::CallbackFn::new(move |info, _| {
                crate::gl_glue::blit(key, width, height, info.viewport_in_pixels())
            })
        };
        #[cfg(not(target_arch = "wasm32"))]
        let source = crate::gl_glue::name(target.framebuffer);
        #[cfg(not(target_arch = "wasm32"))]
        let callback = egui_glow::CallbackFn::new(move |info, painter| {
            use eframe::glow::HasContext as _;
            let gl = painter.gl();
            let viewport = info.viewport_in_pixels();
            // SAFETY: egui_glow runs paint callbacks with its context current, and
            // eframe's windows share one native context (see the module docs), so
            // `source` names the view's framebuffer here too. A name deleted since only
            // raises a GL error. The read binding found on entry is restored.
            unsafe {
                let previous = gl.get_parameter_i32(eframe::glow::READ_FRAMEBUFFER_BINDING);
                let source = crate::gl_glue::eframe_framebuffer(source)
                    .expect("framebuffer names are non-zero");
                gl.bind_framebuffer(eframe::glow::READ_FRAMEBUFFER, Some(source));
                gl.blit_framebuffer(
                    0,
                    0,
                    width,
                    height,
                    viewport.left_px,
                    viewport.from_bottom_px,
                    viewport.left_px + viewport.width_px,
                    viewport.from_bottom_px + viewport.height_px,
                    eframe::glow::COLOR_BUFFER_BIT,
                    eframe::glow::LINEAR,
                );
                gl.bind_framebuffer(
                    eframe::glow::READ_FRAMEBUFFER,
                    crate::gl_glue::eframe_framebuffer(previous),
                );
            }
        });
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: Arc::new(callback),
        });

        if profiling {
            // SAFETY: Still inside egui's update, with the context current as above.
            unsafe { self.gl.finish() };
        }
        self.drawn.get() != before
    }

    fn resize(&mut self, width: i32, height: i32) {
        // A new framebuffer holds nothing yet.
        self.drawn.set(None);
        if let Some(old) = self.target.take() {
            self.delete_target(old);
        }
        let gl = &self.gl;
        // SAFETY: Only `show` calls this, inside egui's update with the context current. The
        // objects are made here, and the previous draw binding is restored before the end.
        unsafe {
            let (Ok(framebuffer), Ok(color), Ok(depth)) = (
                gl.create_framebuffer(),
                gl.create_renderbuffer(),
                gl.create_renderbuffer(),
            ) else {
                eprintln!("3D view: could not create a framebuffer");
                return;
            };
            #[cfg(not(target_arch = "wasm32"))]
            let previous = gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            #[cfg(target_arch = "wasm32")]
            let previous = None;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(color));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(color),
            );
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            let complete =
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, previous);
            let target = Target {
                framebuffer,
                color,
                depth,
                size: (width, height),
            };
            if complete {
                self.target = Some(target);
            } else {
                eprintln!("3D view: framebuffer {width}x{height} is incomplete");
                self.delete_target(target);
            }
        }
    }

    fn delete_target(&self, target: Target) {
        // SAFETY: `resize` and `destroy` call this, both with the GL context current (see their
        // docs). The names were made by `self.gl`, and `target` is moved in, so nothing uses
        // them afterwards.
        unsafe {
            self.gl.delete_framebuffer(target.framebuffer);
            self.gl.delete_renderbuffer(target.color);
            self.gl.delete_renderbuffer(target.depth);
        }
    }
}
