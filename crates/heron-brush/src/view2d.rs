//! A flat view in egui: the core's [`FlatView`] drawn through [`EguiCanvas`], with egui's
//! pointer and keys forwarded to its tools.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::camera2d::{Camera2d, ViewPlane};
use editor_core::view2d::{DragState, FlatView, Modifiers, SelectAndMove, ViewportState};
use eframe::egui;

use crate::editor::Editor;
use crate::egui_canvas::EguiCanvas;

/// The modifier keys egui reports, as the core reads them. Command stands in for Control
/// on macOS, where it is what the shortcuts use.
pub fn modifiers(ui: &egui::Ui) -> Modifiers {
    ui.input(|i| Modifiers {
        shift: i.modifiers.shift,
        control: i.modifiers.command || i.modifiers.ctrl,
        alt: i.modifiers.alt,
    })
}

/// Counts a primary press as the second of a double click, by the time and distance from
/// the press before it. egui reports a double click only on the frame it is released,
/// which a press handler never sees, so the views count presses themselves.
#[derive(Default)]
pub(crate) struct DoublePress {
    last: Option<(f64, egui::Pos2)>,
}

impl DoublePress {
    /// How long and how far apart two presses may be and still make a double click.
    const DELAY: f64 = 0.4;
    const REACH: f32 = 6.0;

    /// A press at `at` at `time`: whether it is the second of a double click. A third
    /// press starts counting again.
    pub(crate) fn press(&mut self, time: f64, at: egui::Pos2) -> bool {
        let double = self.last.is_some_and(|(then, there)| {
            time - then <= Self::DELAY && there.distance(at) <= Self::REACH
        });
        self.last = if double { None } else { Some((time, at)) };
        double
    }
}

/// One flat pane.
pub struct Pane2d {
    target_pick: Rc<RefCell<crate::editor::TargetPick>>,
    pub animation: crate::animation_gizmo::Gizmo,
    /// The view's drawing and handlers.
    pub flat: Rc<FlatView>,
    /// The primary-button drag and every tool's drag.
    drag: SelectAndMove,
    /// Where the primary press began, while the button is held.
    pressed_at: Option<egui::Pos2>,
    /// Whether the press held now is the second of a double click.
    double: bool,
    presses: DoublePress,
    paint_stroke: Option<(editor_core::document::MaterialStroke, egui::Pos2)>,
    /// Whether the pointer was over the view last frame.
    hovered: bool,
    /// The modifiers last seen over the view.
    modifiers: Modifiers,
    /// Where a secondary press began, for the right-click menu.
    secondary: Option<egui::Pos2>,
    /// Whether the secondary button has been dragged far enough to pan.
    secondary_panned: bool,
    /// A right-click that stayed put: where, and what it was over.
    pub context_click: Option<(egui::Pos2, editor_core::view2d::Probe)>,
    pub material_paste: Option<editor_core::document::FaceRef>,
    /// The picture of the view's bottom layer, kept between frames.
    pub base: BaseCache,
}

/// A flat view's background, grid and unselected brush edges, tessellated once and
/// painted again every frame until something they show changes
/// ([`editor_core::view2d::base_key`], the pane's place and the pixel density): the 3D
/// camera turning, the pointer moving or the selection being dragged leave them alone,
/// and redrawing tens of thousands of edges each frame was most of a big map's frame.
#[derive(Default)]
pub struct BaseCache {
    key: Option<u64>,
    meshes: Vec<std::sync::Arc<egui::Mesh>>,
    /// How many times the picture was drawn afresh, for the tests.
    pub rebuilds: usize,
}

#[cfg(test)]
impl BaseCache {
    pub(crate) fn invalidate(&mut self) {
        self.key = None;
    }
}

/// [`BaseCache`] seen by the core as the keeper of the bottom layer, for one frame.
struct KeepBase<'a> {
    cache: &'a mut BaseCache,
    painter: &'a egui::Painter,
    rect: egui::Rect,
}

impl editor_core::view2d::BaseKeeper for KeepBase<'_> {
    fn base(&mut self, key: u64, draw: &dyn Fn(&mut dyn editor_core::canvas::Canvas)) {
        use std::hash::{Hash, Hasher};
        let ppp = self.painter.ctx().pixels_per_point();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut h);
        for v in [
            self.rect.min.x,
            self.rect.min.y,
            self.rect.max.x,
            self.rect.max.y,
            ppp,
        ] {
            v.to_bits().hash(&mut h);
        }
        let key = h.finish();
        if self.cache.key != Some(key) {
            let mut capture = EguiCanvas::capturing(self.painter, self.rect);
            draw(&mut capture);
            self.cache.meshes = capture.into_capture().into_meshes(
                self.painter.ctx(),
                self.painter.clip_rect(),
                ppp,
            );
            self.cache.key = Some(key);
            self.cache.rebuilds += 1;
        }
        self.painter
            .extend(self.cache.meshes.iter().cloned().map(egui::Shape::Mesh));
    }
}

impl Pane2d {
    /// A pane showing `plane` of the editor's document.
    pub fn new(editor: &Editor, plane: ViewPlane) -> Self {
        let bounds = editor.bounds;
        let state = Rc::new(RefCell::new(ViewportState {
            camera: Camera2d::new(plane, bounds.half_extent()),
            cursor: None,
            bounds,
            framed_generation: None,
            creating: None,
            creating_elsewhere: None,
            lasso: None,
            scaling: None,
            rotating: false,
            moving: false,
            dragging_handle: None,
            entity_drop: None,
        }));
        editor.cameras.borrow_mut().push(Rc::clone(&state));
        let flat = Rc::new(FlatView {
            state: Rc::clone(&state),
            extrude_preview: Rc::new(RefCell::new(Vec::new())),
            grid: Rc::clone(&editor.grid),
            document: Rc::clone(&editor.document),
            tool: Rc::clone(&editor.tool),
            clip: Rc::clone(&editor.clip),
            handles: Rc::clone(&editor.handles),
            rotate: Rc::clone(&editor.rotate),
            filters: Rc::clone(&editor.filters),
            sweep: Rc::clone(&editor.sweep),
            fillet: Rc::clone(&editor.fillet),
            pending_shape: Rc::clone(&editor.pending_shape),
            cameras: Rc::clone(&editor.cameras),
            paste_target: Rc::clone(&editor.paste_target),
            active_plane: Rc::clone(&editor.active_plane),
            redraw: Rc::clone(&editor.redraw),
            redraw_everywhere: Rc::clone(&editor.redraw),
            refresh_panels: Rc::clone(&editor.refresh_panels),
        });
        let drag = SelectAndMove {
            state,
            grid: Rc::clone(&editor.grid),
            document: Rc::clone(&editor.document),
            shape_parameters: Rc::clone(&editor.shape_parameters),
            tool: Rc::clone(&editor.tool),
            clip: Rc::clone(&editor.clip),
            handles: Rc::clone(&editor.handles),
            rotate: Rc::clone(&editor.rotate),
            fillet: Rc::clone(&editor.fillet),
            pending_shape: Rc::clone(&editor.pending_shape),
            redraw_everywhere: Rc::clone(&editor.redraw),
            sweep: Rc::clone(&editor.sweep),
            refresh_panels: Rc::clone(&editor.refresh_panels),
            redraw: Rc::clone(&editor.redraw),
            focus: Rc::new(|| {}),
            drag: Rc::new(RefCell::new(DragState::default())),
            marquee: Rc::new(RefCell::new(None)),
        };
        Self {
            target_pick: Rc::clone(&editor.target_pick),
            animation: crate::animation_gizmo::Gizmo::linked(&editor.animation_link),
            flat,
            drag,
            pressed_at: None,
            double: false,
            presses: DoublePress::default(),
            paint_stroke: None,
            hovered: false,
            modifiers: Modifiers::default(),
            secondary: None,
            secondary_panned: false,
            context_click: None,
            material_paste: None,
            base: BaseCache::default(),
        }
    }

    /// Lays the pane out in `rect`: input first, then the drawing.
    pub fn show(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let plane = self.flat.state.borrow().camera.plane;
        let response = ui.interact(
            rect,
            ui.scope_id().with(("view2d", plane.name())),
            egui::Sense::click_and_drag(),
        );
        let viewport = (f64::from(rect.width()), f64::from(rect.height()));
        let local = |p: egui::Pos2| (f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y));
        let modifiers = modifiers(ui);

        // The primary button: the press begins the drag at once, so a click that never
        // moves still selects.
        let (pressed, released, pointer, press_origin, moved, time) = ui.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.interact_pos(),
                i.pointer.press_origin(),
                pointer_moved(i),
                i.time,
            )
        });
        let picking = self.target_pick.borrow().source.is_some();
        if picking && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.target_pick.borrow_mut().source = None;
        }
        if picking && response.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
            if let Some(at) = pointer {
                let (x, y) = local(at);
                self.target_pick.borrow_mut().picked = Some(self.flat.pick_node(x, y, viewport));
            }
        }
        if self.pressed_at.is_some()
            && ui.input(|i| {
                !i.focused || (!i.pointer.primary_down() && !i.pointer.primary_released())
            })
        {
            // `lose` closes the move's transaction, keeping what the drag did; left
            // open, it swallowed every later edit from the history.
            self.drag.lose();
            self.pressed_at = None;
            self.paint_stroke = None;
        }
        if pressed
            && !picking
            && response.hovered()
            && self.flat.filters.borrow().animation_frame.is_none()
        {
            // Where the button went down, not where the pointer is by the time the frame
            // runs: a slow frame can carry the first motion along with the press.
            if let Some(at) = press_origin.or(pointer) {
                let (x, y) = local(at);
                let double = self.presses.press(time, at);
                self.double = double;
                // The press is a click about a face first, then the start of a drag.
                self.paint_stroke = None;
                if *self.flat.tool.borrow() == editor_core::tool::Tool::Paint && !modifiers.shift {
                    let mut stroke = self.flat.document.borrow().begin_material_stroke();
                    self.flat.paint_material_stroke_at(
                        if double { 2 } else { 1 },
                        x,
                        y,
                        viewport,
                        &mut stroke,
                    );
                    self.paint_stroke = Some((stroke, at));
                } else {
                    self.flat
                        .face_click(if double { 2 } else { 1 }, x, y, modifiers, viewport);
                }
                // A double click with the Clip tool puts the clip plane on the face
                // (`ClipToolController`), so it does not also add a point.
                let clip_face = double && *self.flat.tool.borrow() == editor_core::tool::Tool::Clip;
                if !clip_face {
                    self.drag.begin(x, y, modifiers, viewport);
                }
                self.pressed_at = Some(at);
            }
        }
        if let (Some(origin), Some(at)) = (self.pressed_at, pointer) {
            let delta = at - origin;
            if moved {
                if let Some((stroke, last)) = self.paint_stroke.as_mut() {
                    let steps = ((last.distance(at) / 3.0).ceil() as usize).clamp(1, 128);
                    for n in 1..=steps {
                        let point = last.lerp(at, n as f32 / steps as f32);
                        let (x, y) = local(point);
                        self.flat
                            .paint_material_stroke_at(1, x, y, viewport, stroke);
                    }
                    *last = at;
                }
                self.drag
                    .update(f64::from(delta.x), f64::from(delta.y), modifiers, viewport);
            }
            if released {
                self.drag
                    .end(f64::from(delta.x), f64::from(delta.y), modifiers, viewport);
                // The second click of a double click, let go where it was pressed:
                // `SelectionTool::mouseDoubleClick`.
                if self.double && delta.length() < editor_core::view2d::DRAG_THRESHOLD as f32 {
                    let (x, y) = local(origin);
                    self.flat.double_click(x, y, modifiers, viewport);
                }
                self.double = false;
                self.pressed_at = None;
                self.paint_stroke = None;
            }
        } else if released {
            self.pressed_at = None;
            self.paint_stroke = None;
        }
        if self.pressed_at.is_some()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
            && self.drag.escape()
        {
            self.pressed_at = None;
        }

        // A right-click that stays put opens the menu, `MapViewBase::showPopupMenu`.
        if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary))
            && response.hovered()
        {
            self.secondary = pointer;
            self.secondary_panned = false;
        }
        // Dragged further than a shaking click, the right button pans (`CameraTool2D`).
        if let (Some(from), Some(at)) = (self.secondary, pointer) {
            self.secondary_panned |= from.distance(at) > 4.0;
            if self.secondary_panned && response.dragged_by(egui::PointerButton::Secondary) {
                let delta = response.drag_delta();
                self.flat.pan(f64::from(delta.x), f64::from(delta.y));
            }
        }
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
            let panned = std::mem::take(&mut self.secondary_panned);
            if let (Some(from), Some(at)) = (self.secondary.take(), pointer) {
                if from.distance(at) <= 4.0 && !panned {
                    let (x, y) = local(at);
                    self.context_click = Some((at, self.flat.probe(x, y, viewport)));
                }
            }
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
                self.flat.entity_drag_over(Some(&name), x, y, viewport);
            }
            _ => self.flat.entity_drag_leave(),
        }
        if let (Some(dropped), Some(at)) = (
            response.dnd_release_payload::<crate::widgets::Dragged>(),
            pointer,
        ) {
            let (x, y) = local(at);
            match &*dropped {
                crate::widgets::Dragged::Class(name) => {
                    self.flat.drop_entity(name, x, y, viewport);
                }
                crate::widgets::Dragged::Prefab(path) => {
                    self.flat.drop_prefab(path, x, y, viewport);
                }
            }
        }

        // The middle button pans.
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.flat.pan(f64::from(delta.x), f64::from(delta.y));
        }

        if response.clicked_by(egui::PointerButton::Middle)
            && !picking
            && !modifiers.shift
            && !modifiers.control
            && !modifiers.alt
            && self.flat.filters.borrow().animation_frame.is_none()
        {
            if let Some(at) = pointer {
                let (x, y) = local(at);
                self.material_paste = self.flat.face_under_pointer(x, y, viewport);
            }
        }

        // Two fingers pan and pinch (crate::touch).
        if let Some(motion) = crate::touch::take_motion(ui.ctx(), plane_view(plane)) {
            self.flat
                .pan(f64::from(motion.pan.x), f64::from(motion.pan.y));
            let (x, y) = local(motion.centre);
            self.flat.zoom_about(f64::from(motion.zoom), x, y, viewport);
        }

        // The wheel zooms about the pointer, one step a notch. The tools count a step
        // up as negative, egui as positive.
        if response.hovered() {
            let steps: Vec<f32> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y),
                        _ => None,
                    })
                    .collect()
            });
            let at = pointer.map(local);
            for step in steps {
                let dy = -f64::from(step.signum());
                let taken =
                    at.is_some_and(|(x, y)| self.flat.scroll_with(dy, modifiers, x, y, viewport));
                if !taken {
                    self.flat.scroll(dy, viewport);
                }
            }
        }

        // The pointer over the view, and the modifiers changing under a still pointer.
        if response.hovered() {
            if let Some(at) = ui.input(|i| i.pointer.hover_pos()) {
                if moved || !self.hovered {
                    let (x, y) = local(at);
                    self.flat.motion(x, y, modifiers, viewport);
                } else if modifiers != self.modifiers {
                    self.flat.modifiers_changed(modifiers, viewport);
                }
            }
            self.hovered = true;
        } else if self.hovered {
            self.hovered = false;
            self.flat.leave();
        }
        self.modifiers = modifiers;

        let painter = ui.painter_at(rect);
        let mut canvas = EguiCanvas::new(&painter, rect);
        let mut keeper = KeepBase {
            cache: &mut self.base,
            painter: &painter,
            rect,
        };
        self.flat
            .draw_keeping_base(&mut canvas, Some(&mut keeper), viewport);
        let camera = self.flat.state.borrow().camera;
        let tool = *self.flat.tool.borrow();
        self.animation.show_2d(
            ui,
            rect,
            &self.flat.document,
            &self.flat.filters,
            &self.flat.grid,
            tool,
            camera,
        );
    }
}

/// The view number the layout and the touch gestures know a flat view by.
fn plane_view(plane: ViewPlane) -> usize {
    match plane {
        ViewPlane::Top => 1,
        ViewPlane::Front => 2,
        ViewPlane::Side => 3,
    }
}

/// Whether the pointer moved since the last frame. egui's `is_moving` needs three samples
/// within a tenth of a second, so below about 20 frames a second it stays false through
/// a whole drag and the drag never moves anything; the frame's own delta does not.
pub fn pointer_moved(input: &egui::InputState) -> bool {
    input.pointer.delta() != egui::Vec2::ZERO
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        }
    }

    fn picture(pane: &Pane2d) -> Vec<(Vec<egui::epaint::Vertex>, Vec<u32>)> {
        pane.base
            .meshes
            .iter()
            .map(|m| (m.vertices.clone(), m.indices.clone()))
            .collect()
    }

    /// The kept picture of a flat view's bottom layer is the one a fresh pane draws, after
    /// every kind of change: a drag of the selection (which keeps it), selecting, an edit
    /// of an unselected brush, undo and redo, hiding a layer and a colour preference.
    #[test]
    fn the_kept_flat_view_picture_is_never_stale() {
        editor_core::preferences::set_current(Default::default());
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        *editor.document.borrow_mut() = editor_core::document::Document::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map"),
            Default::default(),
        )
        .unwrap();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut pane = Pane2d::new(&editor, ViewPlane::Top);
        let frame = |pane: &mut Pane2d| {
            let mut output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
            output.textures_delta.clear();
        };
        let check = |pane: &mut Pane2d, what: &str| {
            frame(pane);
            let mut fresh = Pane2d::new(&editor, ViewPlane::Top);
            fresh.flat.state.borrow_mut().camera = pane.flat.state.borrow().camera;
            fresh.flat.state.borrow_mut().framed_generation =
                pane.flat.state.borrow().framed_generation;
            frame(&mut fresh);
            assert!(
                picture(pane) == picture(&fresh),
                "the kept picture is stale after {what}"
            );
        };
        frame(&mut pane);
        frame(&mut pane);
        assert_eq!(
            pane.base.rebuilds, 1,
            "an unchanged view draws its picture once"
        );
        let ids: Vec<_> = map_model::node::brushes_with_ids(editor.document.borrow().tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert!(ids.len() >= 2, "the sample has brushes to move");

        editor.document.borrow_mut().select_only(ids[0]);
        check(&mut pane, "selecting");
        let rebuilds = pane.base.rebuilds;
        editor
            .document
            .borrow_mut()
            .translate_selection(geometry_math::vec::Vec3d::new(16.0, 0.0, 0.0));
        check(&mut pane, "moving the selection");
        assert_eq!(
            pane.base.rebuilds, rebuilds,
            "moving the selection keeps the picture"
        );

        editor.document.borrow_mut().select_only(ids[1]);
        editor
            .document
            .borrow_mut()
            .translate_selection(geometry_math::vec::Vec3d::new(0.0, 32.0, 0.0));
        editor.document.borrow_mut().clear_selection();
        check(&mut pane, "moving another brush");
        editor.document.borrow_mut().undo();
        check(&mut pane, "undo");
        editor.document.borrow_mut().redo();
        check(&mut pane, "redo");
        editor.document.borrow_mut().set_layer_hidden(0, true);
        check(&mut pane, "hiding a layer");
        editor.document.borrow_mut().set_layer_hidden(0, false);
        editor_core::preferences::edit(|p| {
            p.edge_color = editor_core::preferences::Color::rgb(1.0, 0.0, 0.0)
        });
        check(&mut pane, "changing the edge colour");
        editor.grid.borrow_mut().increase();
        check(&mut pane, "changing the grid");
    }
}
