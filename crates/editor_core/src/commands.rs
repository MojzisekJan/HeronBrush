//! The editor's keyboard commands: the table of shortcuts every map view shares, and the
//! editing actions behind them. The frontend hands each key press to
//! [`Commands::key_pressed`] and does what [`Window`] asks of the window itself.

use std::{cell::RefCell, rc::Rc};

use geometry_math::{util::Axis, vec::Vec3d};

use crate::document_setup::Sources;
use crate::i18n::{counted, fill, tr};
use crate::input::{Key, Modifiers};
use crate::{
    assemble_tool, camera2d, camera3d, clip_tool, fillet_tool, grid, handle_tool, preferences,
    render_camera, rotate_tool, shape_tool, tool, view2d,
};
use crate::{camera2d::ViewPlane, document::Document, grid::Grid, world::WorldBounds};

/// The size a material fit assumes when the material itself is not loaded.
///
/// 64 by 64 is the usual size in a Quake-lineage material set, so fitting against it puts
/// the mapping in the right range for a real material to be swapped in later.
pub const DEFAULT_MATERIAL_SIZE: geometry_math::vec::Vec2f =
    geometry_math::vec::Vec2f::new(64.0, 64.0);

/// Says what an undo, or with `redo` a redo, did.
pub fn report(result: Option<String>, redo: bool) {
    let line = match (result, redo) {
        (Some(description), false) => fill("Undone: {}", &[&description]),
        (Some(description), true) => fill("Redone: {}", &[&description]),
        (None, false) => tr("Nothing to undo").to_string(),
        (None, true) => tr("Nothing to redo").to_string(),
    };
    crate::say!("{line}");
}

/// What the commands ask of the window around the views.
pub trait Window {
    /// Redraws every view.
    fn redraw(&self);
    /// Brings the inspector up to date.
    fn refresh_inspector(&self);
    /// Brings the toolbar up to date.
    fn refresh_toolbar(&self);
    /// How large the `index`th flat view is, in the order of the shared cameras.
    fn flat_view_size(&self, index: usize) -> Option<(f64, f64)>;
    /// Last visible 3D viewport size; hidden views use a square fallback.
    fn perspective_view_size(&self) -> Option<(f64, f64)> {
        None
    }
    /// Which view has the keyboard: 0 is the 3D view.
    fn focused_view(&self) -> Option<usize>;
    /// `Cycle map view`, Space: a pane that shows one view at a time moves to the next.
    fn cycle_layout(&self);
    /// `maximizeCurrentView`, Ctrl+Space.
    fn toggle_maximised(&self);
    /// Brings up an inspector tab: 0 Map, 1 Entity, 2 Face.
    fn show_inspector_page(&self, page: usize);
    /// `Toggle Info Panel`.
    fn toggle_console(&self);
    /// `Toggle Inspector`.
    fn toggle_inspector(&self);
    /// `Toggle Toolbar`.
    fn toggle_toolbar(&self);
    /// Puts text on the clipboard.
    fn set_clipboard(&self, text: &str);
    /// Reads the clipboard and hands its text on, when there is some.
    fn with_clipboard(&self, then: Box<dyn FnOnce(String)>);
    /// Opens the window that moves the selection by a typed number.
    fn go_to(&self);
    /// Saves the map, asking where when it has never been saved.
    fn save(&self);
}

/// The shared document, tool and view state the keyboard commands act on.
#[derive(Clone)]
pub struct Commands {
    pub grid: Rc<RefCell<Grid>>,
    pub document: Rc<RefCell<Document>>,
    pub tool: Rc<RefCell<tool::Tool>>,
    pub clip: Rc<RefCell<clip_tool::ClipTool>>,
    pub assemble: Rc<RefCell<assemble_tool::AssembleBrushTool>>,
    pub handles: Rc<RefCell<handle_tool::HandleManager>>,
    pub rotate: Rc<RefCell<rotate_tool::RotateTool>>,
    pub shape_parameters: Rc<RefCell<shape_tool::ShapeParameters>>,
    /// Every flat view's camera, in the order [`Window::flat_view_size`] counts them.
    pub cameras: Rc<RefCell<Vec<Rc<RefCell<view2d::ViewportState>>>>>,
    pub bounds: WorldBounds,
    pub paste_target: Rc<RefCell<Option<Vec3d>>>,
    pub active_plane: Rc<RefCell<Option<ViewPlane>>>,
    /// What the fillet rounds by, for the Return that applies it.
    pub fillet: Rc<RefCell<fillet_tool::FilletParameters>>,
    /// The 3D camera, for focusing it on the selection.
    pub camera_3d: Rc<RefCell<camera3d::Camera3d>>,
    /// What to load again when the definitions or the materials are asked for afresh.
    pub sources: Rc<RefCell<Sources>>,
    /// The window around the views.
    pub window: Rc<dyn Window>,
}

impl Commands {
    /// Does what a key press means, and says whether it meant anything.
    /// `hardware_digit` is the digit printed on the physical key, if it is one.
    pub fn key_pressed(&self, key: Key, hardware_digit: Option<u32>, modifiers: Modifiers) -> bool {
        #[allow(unused_variables)]
        let (key, modifiers) =
            crate::keymap::translate(preferences::pref(|p| p.keymap), key, modifiers);
        self.key_pressed_canonical(key, hardware_digit, modifiers)
    }

    /// Dispatches an explicitly configured command without translating the preset twice.
    pub fn key_pressed_canonical(
        &self,
        key: Key,
        hardware_digit: Option<u32>,
        modifiers: Modifiers,
    ) -> bool {
        #[allow(unused_variables)]
        let Self {
            grid,
            document,
            tool,
            clip,
            assemble,
            handles,
            rotate,
            shape_parameters,
            cameras,
            bounds,
            paste_target,
            active_plane,
            fillet,
            camera_3d,
            sources,
            window,
        } = self;

        // Which digit the number row was pressed at, whatever the layout puts there.
        let digit = crate::input::digit(key, hardware_digit);
        let (ctrl, shift, alt) = (modifiers.control, modifiers.shift, modifiers.alt);
        if ctrl
            && alt
            && !shift
            && (digit == Some(0) || matches!(key, Key::Keypad(0) | Key::Insert))
        {
            render_camera::align_to_view(&mut document.borrow_mut(), camera_3d);
            window.refresh_inspector();
            window.refresh_toolbar();
            window.redraw();
            return true;
        }

        {
            // A copy rather than a borrow held across the match: an arm that refreshes
            // the toolbar would otherwise find the grid already borrowed and panic.
            // Written back below,
            // before anything is told to catch up.
            let mut grid_now = *grid.borrow();
            match key {
                Key::Plus => grid_now.increase(),
                Key::Minus => grid_now.decrease(),
                // Both 0 and # toggle grid visibility.
                _ if digit == Some(0) && !alt && !ctrl => {
                    let visible = grid_now.visible();
                    grid_now.set_visible(!visible);
                }
                Key::NumberSign => {
                    let visible = grid_now.visible();
                    grid_now.set_visible(!visible);
                }
                // `ActionManager`: snapping is Alt+0, and Ctrl+G groups.
                _ if digit == Some(0) && alt && !ctrl => {
                    let snap = grid_now.snap();
                    grid_now.set_snap(!snap);
                }
                // Ctrl+Shift+U resets 2D views to the origin and default zoom. U toggles UV lock
                // during geometry edits.
                Key::Letter('u') if !ctrl && !shift && !alt => {
                    let locked = !preferences::pref(|p| p.uv_lock);
                    preferences::edit(|preferences| preferences.uv_lock = locked);
                    crate::say!(
                        "{}",
                        tr(if locked {
                            "UV lock is on"
                        } else {
                            "UV lock is off"
                        })
                    );
                }
                // `Reset Camera Zoom`, Ctrl+Alt+Z.
                Key::Letter('z') if ctrl && alt && !shift => {
                    for state in cameras.borrow().iter() {
                        state.borrow_mut().camera.zoom = 1.0;
                    }
                    window.redraw();
                    crate::say!("{}", tr("Reset the zoom"));
                }
                Key::Letter('u') if ctrl && shift => {
                    for state in cameras.borrow().iter() {
                        let mut state = state.borrow_mut();
                        let plane = state.camera.plane;
                        state.camera = camera2d::Camera2d::new(plane, bounds.half_extent());
                    }
                    window.redraw();
                }
                // `focusCameraOnSelection`, Ctrl+U: put every camera where the
                // selection fills the view. The 3D camera keeps the direction it was
                // pointing and backs off far enough for the box to fit; a flat view
                // centres on it and zooms so it sits inside with a little air.
                Key::Letter('u') if ctrl && !shift && !alt => {
                    let Some(selection) = document.borrow().selection_bounds() else {
                        crate::say!("{}", tr("Select something to focus on"));
                        return false;
                    };
                    let (width, height) = window.perspective_view_size().unwrap_or((1.0, 1.0));
                    camera_3d
                        .borrow_mut()
                        .frame_in_viewport(&selection, width, height);
                    for (index, state) in cameras.borrow().iter().enumerate() {
                        let mut state = state.borrow_mut();
                        let plane = state.camera.plane;
                        state.camera.set_center(plane.project(selection.center()));
                        if let Some((view_width, view_height)) = window.flat_view_size(index) {
                            let seen = plane.project(selection.size());
                            let across = (view_width / seen.x().abs().max(1.0))
                                .min(view_height / seen.y().abs().max(1.0));
                            // A little air around it, and never so far in that a small
                            // brush fills the window.
                            state.camera.zoom = (across * 0.8).clamp(0.02, 4.0);
                        }
                    }
                    window.redraw();
                }
                // `maximizeCurrentView`, Ctrl+Space: the view the keyboard is in fills
                // the window, and pressing it again puts the other three back.
                // `Cycle map view`, Space: a pane that shows one view at a time moves
                // to the next. The four-pane layout has nothing to cycle.
                Key::Space if !ctrl && !shift && !alt => window.cycle_layout(),
                Key::Space if ctrl && !shift && !alt => {
                    window.toggle_maximised();
                }
                // `Switch to Map/Entity/Face Inspector`, Ctrl+1 to Ctrl+3, and
                // `Toggle Inspector` and `Toggle Toolbar`, which put the window's
                // furniture away when the map wants the room.
                _ if digit == Some(1) && ctrl => window.show_inspector_page(0),
                _ if digit == Some(2) && ctrl => window.show_inspector_page(1),
                _ if digit == Some(3) && ctrl => window.show_inspector_page(2),
                // `Toggle Info Panel`, Ctrl+4: what the editor has said.
                _ if digit == Some(4) && ctrl => {
                    window.toggle_console();
                }
                _ if digit == Some(5) && ctrl => {
                    window.toggle_inspector();
                }
                Key::Letter('t') if ctrl && alt => {
                    window.toggle_toolbar();
                }

                // `Reload Material Collections` and `Reload Entity Definitions`: what was
                // loaded, loaded again, for a texture folder or an `.fgd` edited while
                // the editor is open.
                Key::Function(5) => {
                    let materials = sources.borrow().materials.clone();
                    match materials {
                        Some(path) => {
                            match document.borrow_mut().load_materials(&path) {
                                Ok(_) => {
                                    crate::say!(
                                        "{}",
                                        fill(
                                            "Reloaded the materials in {}",
                                            &[&path.display().to_string()]
                                        )
                                    )
                                }
                                Err(error) => crate::complain!(
                                    "{}",
                                    fill(
                                        "Could not reload {}: {}",
                                        &[&path.display().to_string(), &error.to_string()]
                                    )
                                ),
                            }
                            window.redraw();
                            window.refresh_inspector();
                        }
                        None => crate::say!("{}", tr("No material folder has been loaded")),
                    }
                }
                Key::Function(6) => {
                    let definitions = sources.borrow().definitions.clone();
                    match definitions {
                        Some(path) => {
                            match document.borrow_mut().load_definitions(&path) {
                                Ok(warnings) => {
                                    crate::say!(
                                        "{}",
                                        fill(
                                            "Reloaded the entity definitions from {}",
                                            &[&path.display().to_string()]
                                        )
                                    );
                                    for warning in warnings {
                                        crate::complain!("warning: {warning}");
                                    }
                                }
                                Err(error) => crate::complain!(
                                    "{}",
                                    fill(
                                        "Could not reload {}: {}",
                                        &[&path.display().to_string(), &error.to_string()]
                                    )
                                ),
                            }
                            window.redraw();
                            window.refresh_inspector();
                        }
                        None => crate::say!("{}", tr("No entity definitions have been loaded")),
                    }
                }
                Key::Letter('s') if !ctrl => {
                    let size = grid_now.actual_size();
                    let report = document.borrow_mut().snap_vertices_to_grid(size);
                    crate::say!(
                        "{}",
                        fill(
                            "Snapped the vertices to the {} grid: {}",
                            &[&size.to_string(), &report.to_string()]
                        )
                    );
                }
                // `ActionManager`'s Edit/Select menu: Ctrl+Alt+A inverts, Ctrl+Shift+A
                // deselects, Ctrl+T selects everything the drawn brushes touch, Ctrl+E
                // everything they enclose -- the last two consuming the brushes.
                Key::Letter('a') if ctrl && alt => document.borrow_mut().invert_selection(),
                Key::Letter('a') if ctrl && shift => document.borrow_mut().clear_selection(),
                Key::Letter('b') if ctrl => document.borrow_mut().select_siblings(),
                // `ActionManager` View menu: Ctrl+I isolates, Ctrl+Alt+I hides,
                // Ctrl+Shift+I shows everything again.
                Key::Letter('i') if ctrl && alt => {
                    let hidden = document.borrow_mut().hide_selection();
                    crate::say!("{}", fill("Hid {}", &[&counted(hidden, "object")]));
                }
                Key::Letter('i') if ctrl && shift => {
                    let shown = document.borrow_mut().show_all();
                    crate::say!("{}", fill("Showed {}", &[&counted(shown, "object")]));
                }
                Key::Letter('i') if ctrl => {
                    let hidden = document.borrow_mut().isolate_selection();
                    if hidden > 0 {
                        crate::say!(
                            "{}",
                            fill(
                                "Isolated the selection and hid {}",
                                &[&counted(hidden, "object")]
                            )
                        );
                    } else {
                        crate::say!("{}", tr("Select something to isolate first"));
                    }
                }
                Key::Letter('t') if ctrl => {
                    let did = document.borrow_mut().select_touching();
                    crate::say!(
                        "{}",
                        tr(if did {
                            "Selected everything the brushes touch"
                        } else {
                            "Select only brushes first"
                        })
                    );
                }
                Key::Letter('e') if ctrl && shift => {
                    let did = match *active_plane.borrow() {
                        Some(plane) => document.borrow_mut().select_tall(plane.depth_axis()),
                        None => false,
                    };
                    crate::say!(
                        "{}",
                        tr(if did {
                            "Selected everything above and below the brushes"
                        } else {
                            "Point at a flat view with only brushes selected"
                        })
                    );
                }
                Key::Letter('e') if ctrl => {
                    let did = document.borrow_mut().select_inside();
                    crate::say!(
                        "{}",
                        tr(if did {
                            "Selected everything inside the brushes"
                        } else {
                            "Select only brushes first"
                        })
                    );
                }
                // With a handle tool active, select-all means its handles rather than the
                // brushes, which are already selected for the tool to be showing any.
                Key::Letter('a') if ctrl => {
                    if handle_tool::HandleKind::of(*tool.borrow()).is_some() {
                        handles.borrow_mut().select_all();
                    } else {
                        document.borrow_mut().select_all();
                    }
                }
                // Through the menu action rather than saving here: Save has to ask
                // where to go when the map has never been saved, and it writes the
                // engine levels again afterwards. A second copy of that here is a
                // second copy to keep right, and it had already drifted -- the key
                // saved and the menu item did more.
                Key::Letter('s') if ctrl && !shift => window.save(),
                // A drag in progress is abandoned before anything else, putting the
                // brushes back where they were: `RotateTool::cancelRotation`.
                // `Deactivate Current Tool`, Shift+Esc. Plain Escape does it too, along
                // with everything else it cancels.
                Key::Escape if document.borrow().has_shape_preview() => {
                    document.borrow_mut().cancel_shape_preview();
                    window.refresh_inspector();
                }
                Key::Enter if document.borrow().has_shape_preview() => {
                    document.borrow_mut().finish_shape_preview();
                    window.refresh_inspector();
                }
                Key::Escape if shift => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Select);
                }
                Key::Escape if document.borrow().in_transaction() => {
                    document.borrow_mut().cancel_transaction();
                }
                Key::Escape => {
                    // `ClipToolControllerBase::cancel`: Escape takes back the last clip
                    // point before it does anything else, so a misplaced point costs one
                    // key rather than the whole plane, and once the last one is gone the
                    // tool starts over.
                    // The assemble tool takes back a point before it gives up the tool.
                    let cancelled_assemble = *tool.borrow() == tool::Tool::Assemble
                        && assemble.borrow_mut().remove_last_point();
                    let cancelled_clip =
                        !cancelled_assemble && *tool.borrow() == tool::Tool::Clip && {
                            let mut clip = clip.borrow_mut();
                            if clip.is_dragging() {
                                clip.cancel_drag_point();
                                true
                            } else if clip.remove_last_point() {
                                if !clip.has_points() {
                                    clip.reset();
                                }
                                true
                            } else {
                                false
                            }
                        };
                    // `NodeHandleToolSelectPartBase::cancel`: with a handle tool in hand
                    // the first Escape drops what is picked, and only the next one leaves
                    // the tool.
                    let cleared_handles = !cancelled_assemble
                        && !cancelled_clip
                        && handle_tool::HandleKind::of(*tool.borrow()).is_some()
                        && handles.borrow().selected_count() > 0
                        && {
                            handles.borrow_mut().clear_selection();
                            true
                        };
                    if cancelled_assemble || cancelled_clip || cleared_handles {
                        // Nothing more to do; the point or the picking is gone.
                    } else if *tool.borrow() != tool::Tool::Select {
                        // Escape exits the active tool before clearing the selection.
                        clip.borrow_mut().reset();
                        *tool.borrow_mut() = tool::Tool::Select;
                    } else if document.borrow().has_selection()
                        || document.borrow().has_face_selection()
                    {
                        document.borrow_mut().clear_selection();
                    } else {
                        // `MapViewBase::cancel`: with nothing left to cancel or deselect,
                        // Escape closes the open group.
                        document.borrow_mut().close_group();
                    }
                }
                // `ActionManager`: Ctrl+G groups the selection, Ctrl+Shift+G takes the
                // groups it covers apart.
                Key::Letter('g') if ctrl && shift => {
                    let dissolved = document.borrow_mut().ungroup_selection();
                    crate::say!("{}", fill("Ungrouped {}", &[&counted(dissolved, "group")]));
                }
                Key::Letter('g') if ctrl => {
                    let count = document.borrow().groups().len();
                    if document
                        .borrow_mut()
                        .group_selection(format!("Group {}", count + 1))
                        .is_some()
                    {
                        crate::say!("{}", tr("Grouped the selection"));
                    }
                }
                // Return commits the pinned fillet preview, or edges/vertices picked
                // with their respective tools.
                Key::Enter
                    if !matches!(*tool.borrow(), tool::Tool::Assemble | tool::Tool::Clip) =>
                {
                    let outcome = crate::fillet_tool::apply(
                        document,
                        &mut handles.borrow_mut(),
                        *fillet.borrow(),
                    );
                    crate::fillet_tool::report(&outcome);
                    window.refresh_inspector();
                }
                // The assemble tool's keys: Return makes the brush the points enclose,
                // Backspace takes back the last one.
                Key::Enter if *tool.borrow() == tool::Tool::Assemble => {
                    let mut assemble = assemble.borrow_mut();
                    let mut document = document.borrow_mut();
                    let world = document.world_bounds();
                    let material = document.current_material().to_string();
                    match assemble.brush(&world, &material) {
                        Some(brush) => {
                            document.add_brushes(vec![brush]);
                            assemble.reset();
                            crate::say!("{}", tr("Assembled a brush"));
                        }
                        None => crate::say!("{}", tr("The points do not enclose a volume yet")),
                    }
                }
                Key::Delete | Key::Backspace if *tool.borrow() == tool::Tool::Assemble => {
                    assemble.borrow_mut().remove_last_point();
                }
                // The clip tool's own keys, from `ActionManager`: Return cuts, Ctrl+Return
                // moves on to the next side to keep.
                Key::Enter if *tool.borrow() == tool::Tool::Clip => {
                    let mut clip = clip.borrow_mut();
                    if ctrl {
                        if clip.toggle_side() {
                            crate::say!("{}", tr(clip.clip_side().kept()));
                        }
                    } else {
                        let cut = document.borrow_mut().clip_selection(&clip);
                        if cut > 0 {
                            clip.reset();
                            crate::say!("{}", fill("Clipped {}", &[&counted(cut, "brush")]));
                        }
                    }
                }
                Key::Letter('n') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Assemble)
                }
                // Tool shortcuts: B draws brushes, E selects edges, and Y activates sweep.
                Key::Letter('y') if !ctrl && !shift && !alt => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Sweep)
                }
                Key::Letter('b') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Brush)
                }
                Key::Letter('e') if !ctrl && !shift && !alt => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Edge)
                }
                Key::Letter('c') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Clip)
                }
                Key::Letter('v') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Vertex)
                }
                // Tab is Blender's mode key: with a brush in hand it steps around the
                // ring whole brush -> vertices -> edges -> faces -> whole brush, so one
                // key reaches every way of editing what is selected.
                Key::Tab if !ctrl && !shift && !alt => {
                    if !document.borrow().selected_brushes().is_empty() {
                        let next = match *tool.borrow() {
                            tool::Tool::Select => tool::Tool::Vertex,
                            tool::Tool::Vertex => tool::Tool::Edge,
                            tool::Tool::Edge => tool::Tool::Face,
                            _ => tool::Tool::Select,
                        };
                        set_tool(tool, clip, handles, rotate, assemble, next);
                    }
                }
                // `Move objects`, Ctrl+Alt+M: the window that moves the selection by a
                // typed delta, which is the same window as Go to.
                Key::Letter('m') if ctrl && alt => window.go_to(),
                // `Convert Selection to Patches`, Ctrl+P.
                Key::Letter('p') if ctrl && !shift && !alt => {
                    let made = document
                        .borrow_mut()
                        .convert_selection_to_patches(map_model::face_to_patch::DEFAULT_POINTS);
                    if made > 0 {
                        crate::say!(
                            "{}",
                            fill("Converted the faces to {}", &[&counted(made, "patch")])
                        );
                    } else {
                        crate::say!("{}", tr("Select some brush faces first"));
                    }
                    window.refresh_inspector();
                }
                Key::Letter('p') if !ctrl => set_tool(
                    tool,
                    clip,
                    handles,
                    rotate,
                    assemble,
                    tool::Tool::ControlPoint,
                ),
                Key::Letter('f') if !ctrl && !shift => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Face)
                }
                Key::Letter('r') if !ctrl && !shift => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Rotate);
                    // `RotateTool::doActivate` resets the centre, so the ring is around
                    // the selection from the first frame rather than only once a flat
                    // view has drawn and set it.
                    if *tool.borrow() == tool::Tool::Rotate {
                        rotate
                            .borrow_mut()
                            .reset_center(&document.borrow(), &grid_now);
                    }
                }
                Key::Letter('t') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Scale)
                }
                Key::Letter('g') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Shear)
                }
                Key::Letter('k') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Paint)
                }
                Key::Letter('j') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Sculpt)
                }
                Key::Letter('o') if !ctrl => {
                    set_tool(tool, clip, handles, rotate, assemble, tool::Tool::Fillet)
                }
                Key::Letter('z') if ctrl && shift => report(document.borrow_mut().redo(), true),
                Key::Letter('z') if ctrl => report(document.borrow_mut().undo(), false),
                Key::Letter('y') if ctrl => report(document.borrow_mut().redo(), true),
                // Join brushes, beside Ctrl+J's merge into one: as few convex brushes as
                // keep the shape.
                Key::Letter('j') if ctrl && shift => {
                    let before = document.borrow().selection_count();
                    let result = document.borrow_mut().join_brushes();
                    match result {
                        Ok(removed) => crate::say!(
                            "{}: {} → {}",
                            tr("Join brushes"),
                            before,
                            counted(before - removed, "brush")
                        ),
                        Err(why) => crate::say!("{}", tr(why)),
                    }
                }
                // `ActionManager`'s CSG menu: Ctrl+J merge, Ctrl+K subtract,
                // Ctrl+Shift+K hollow, Ctrl+L intersect.
                Key::Letter('j') if ctrl && !shift => {
                    let merged = document.borrow_mut().merge_selection();
                    crate::say!(
                        "{}",
                        tr(if merged {
                            "Merged the brushes into one convex brush"
                        } else {
                            "Nothing to merge"
                        })
                    );
                }
                Key::Letter('k') if ctrl && !shift => {
                    let changed = document.borrow_mut().subtract_selection();
                    crate::say!(
                        "{}",
                        fill(
                            "Subtracted the selection from {}",
                            &[&counted(changed, "brush")]
                        )
                    );
                }
                Key::Letter('k') if ctrl && shift => {
                    let thickness = grid_now.actual_size();
                    let hollowed = document.borrow_mut().hollow_selection(thickness);
                    crate::say!(
                        "{}",
                        fill(
                            "Hollowed {} with walls {} units thick",
                            &[&counted(hollowed, "brush"), &thickness.to_string()]
                        )
                    );
                }
                Key::Letter('l') if ctrl => {
                    let ok = document.borrow_mut().intersect_selection();
                    crate::say!(
                        "{}",
                        tr(if ok {
                            "Intersected the brushes"
                        } else {
                            "The brushes have nothing in common"
                        })
                    );
                }
                // Material alignment on every face of the selection, until faces can be
                // selected on their own.
                Key::Letter('f') if ctrl && shift => {
                    document
                        .borrow_mut()
                        .fit_materials_to_selection(DEFAULT_MATERIAL_SIZE);
                    crate::say!("{}", tr("Fitted the material to each face"));
                }
                // Alt+Shift+R resets UV alignment and replaces Valve axes with world-aligned
                // paraxial axes.
                Key::Letter('r') if shift && alt && !ctrl => {
                    document
                        .borrow_mut()
                        .edit_faces(|face| face.reset_uv_to_paraxial());
                    crate::say!("{}", tr("Reset the material alignment to the world axes"));
                }
                // Shift+R resets texture alignment; Control+R repeats edits and Control+Shift+R
                // clears repeat history.
                Key::Letter('r') if shift && !ctrl => {
                    document.borrow_mut().edit_faces(|face| face.reset_uv());
                    crate::say!("{}", tr("Reset the material alignment"));
                }
                Key::Letter('r') if ctrl && shift => {
                    document.borrow_mut().clear_repeatable();
                    crate::say!("{}", tr("Cleared what Repeat would do"));
                }
                Key::Letter('r') if ctrl => repeat_last_commands(document, &**window),
                // Directional keys edit UVs for face selections and transform objects otherwise.
                // See [`arrow_key`].
                Key::Left | Key::Right | Key::Up | Key::Down | Key::PageUp | Key::PageDown
                    if arrow_key(
                        document,
                        &grid_now,
                        // The 3D view goes by its camera; a flat view by the plane it
                        // shows, which is the last one the pointer was over.
                        match window.focused_view() {
                            Some(0) => Some(Pressed::Perspective(*camera_3d.borrow())),
                            _ => active_plane.borrow().map(Pressed::Flat),
                        },
                        key,
                        Modifiers {
                            control: ctrl,
                            shift,
                            alt,
                        },
                        *tool.borrow(),
                        handles,
                        rotate,
                    ) => {}
                Key::BracketLeft => rotate_uv(document, -15.0),
                Key::BracketRight => rotate_uv(document, 15.0),
                // `Move to Previous Point` and `Move to Next Point`, which walk the
                // camera along a leak's trace. With faces selected they scale the
                // material instead, the way the arrows are the material's while a face
                // is: a trace and a face are never both what is being worked on.
                Key::Comma if document.borrow().face_selection_count() > 0 => {
                    scale_uv(document, 0.5)
                }
                Key::Period if document.borrow().face_selection_count() > 0 => {
                    scale_uv(document, 2.0)
                }
                Key::Comma | Key::Period => {
                    let step = document.borrow_mut().step_along_trace(key == Key::Period);
                    match step {
                        Some((at, looking)) => {
                            let mut camera = camera_3d.borrow_mut();
                            camera.position = at;
                            camera.look_along(looking);
                            drop(camera);
                            // The flat views follow, as they do for `Move Camera to...`:
                            // a point named in one is a point in all of them.
                            for state in cameras.borrow().iter() {
                                let mut state = state.borrow_mut();
                                let plane = state.camera.plane;
                                state.camera.set_center(plane.project(at));
                            }
                            window.redraw();
                        }
                        None => crate::say!("{}", tr("The trace ends here")),
                    }
                }
                Key::Letter('d') if ctrl && shift => {
                    let size = grid_now.actual_size();
                    match document.borrow_mut().create_linked_duplicate(size) {
                        Some(_) => crate::say!("{}", tr("Made a linked duplicate")),
                        None => {
                            crate::say!("{}", tr("Select a whole group to make a linked duplicate"))
                        }
                    }
                    window.refresh_inspector();
                }
                Key::Letter('d') if ctrl => {
                    let size = grid_now.actual_size();
                    let copies = document.borrow_mut().duplicate_selection(size);
                    crate::say!("{}", fill("Duplicated {}", &[&counted(copies, "object")]));
                }
                // The clipboard contains `.map` text for object selections, or material and UV
                // attributes for selected faces.
                Key::Letter('c') if ctrl && !shift => {
                    let document = document.borrow();
                    let text = if !document.selected_edit_faces().is_empty() {
                        document.copy_face_attributes_text()
                    } else {
                        document.copy_selection()
                    };
                    if let Some(text) = text {
                        window.set_clipboard(&text);
                    }
                }
                Key::Letter('x') if ctrl && !shift => {
                    if let Some(text) = document.borrow_mut().cut_selection() {
                        window.set_clipboard(&text);
                    }
                }
                // `ActionManager`: Ctrl+Shift+V snaps the selected brushes' vertices to
                // integer coordinates, Ctrl+Alt+Shift+V to the current grid.
                Key::Letter('v') if ctrl && shift && !alt => {
                    let report = document.borrow_mut().snap_vertices_to_grid(1.0);
                    crate::say!(
                        "{}",
                        fill(
                            "Snapped the vertices to whole units: {}",
                            &[&report.to_string()]
                        )
                    );
                }
                Key::Letter('v') if ctrl && shift && alt => {
                    let size = grid_now.actual_size();
                    let report = document.borrow_mut().snap_vertices_to_grid(size);
                    crate::say!(
                        "{}",
                        fill(
                            "Snapped the vertices to the {} grid: {}",
                            &[&size.to_string(), &report.to_string()]
                        )
                    );
                }
                Key::Letter('v') if ctrl => {
                    // Ctrl+V pastes at the snapped pointer position; Ctrl+Alt+V preserves the
                    // copied coordinates.
                    let target = if alt {
                        None
                    } else {
                        paste_target.borrow().map(|at| grid_now.snap_point(at))
                    };
                    window.with_clipboard(Box::new({
                        let document = Rc::clone(document);
                        let window = Rc::clone(window);
                        move |text: String| {
                            let pasted = document.borrow_mut().paste_at(&text, target);
                            crate::say!("{}", fill("Pasted {}", &[&counted(pasted, "object")]));
                            window.redraw();
                            window.refresh_inspector();
                        }
                    }));
                }
                // Ctrl+F and Ctrl+Alt+F flip objects along the current view axes, or flip the UV
                // mapping when faces are selected.
                Key::Letter('f') if ctrl && !shift => {
                    if document.borrow().face_selection_count() > 0 {
                        let horizontally = !alt;
                        document
                            .borrow_mut()
                            .edit_faces(|face| face.flip_uv(horizontally));
                        crate::say!("{}", tr("Flipped the material"));
                    } else {
                        let pressed = match window.focused_view() {
                            Some(0) => Some(Pressed::Perspective(*camera_3d.borrow())),
                            _ => active_plane.borrow().map(Pressed::Flat),
                        };
                        let Some(pressed) = pressed else {
                            return false;
                        };
                        // Across the screen, or up and down it.
                        let along = move_direction(
                            pressed,
                            if alt {
                                Direction::Forward
                            } else {
                                Direction::Right
                            },
                        );
                        flip(document, axis_of(along));
                    }
                }
                // Rotate a quarter turn about each axis, and mirror along it.
                Key::Letter('x') if ctrl && shift => flip(document, Axis::X),
                Key::Letter('y') if ctrl && shift => flip(document, Axis::Y),
                Key::Letter('z') if ctrl && shift && modifiers.alt => flip(document, Axis::Z),
                // Insert places a point entity at the selection centre, or at the origin when
                // nothing is selected.
                Key::Insert => {
                    let size = grid_now.actual_size();
                    place_entity(document, size);
                }

                // Alt-modified shortcuts rotate by a quarter turn without conflicting with tool
                // shortcuts.
                Key::Letter('x') if alt && !ctrl => {
                    rotate_by_quarter_turn(document, Axis::X, shift)
                }
                Key::Letter('y') if alt && !ctrl => {
                    rotate_by_quarter_turn(document, Axis::Y, shift)
                }
                Key::Letter('z') if alt && !ctrl => {
                    rotate_by_quarter_turn(document, Axis::Z, shift)
                }
                // `MapWindow`: with the clip tool active, delete takes back the last
                // clip point rather than deleting the selection.
                Key::Delete | Key::Backspace if *tool.borrow() == tool::Tool::Clip => {
                    let mut clip = clip.borrow_mut();
                    if clip.remove_last_point() && !clip.has_points() {
                        clip.reset();
                    }
                }
                // `MapWindow::deleteSelection` asks the tools first: with a handle tool
                // in hand Delete takes the picked corners off the brush rather than
                // deleting the brush itself.
                Key::Delete | Key::Backspace
                    if handle_tool::HandleKind::of(*tool.borrow()).is_some()
                        && handles.borrow().selected_count() > 0 =>
                {
                    let picked = handles.borrow().selected_vertices();
                    let changed = document.borrow_mut().remove_handles(&picked);
                    handles.borrow_mut().clear_selection();
                    if changed > 0 {
                        crate::say!(
                            "{}",
                            fill("Took the corners off {}", &[&counted(changed, "brush")])
                        );
                    } else {
                        crate::say!("{}", tr("Without those corners nothing would be left"));
                    }
                }
                Key::Delete | Key::Backspace => {
                    let deleted = document.borrow_mut().delete_selection();
                    crate::say!("{}", fill("Deleted {}", &[&counted(deleted, "object")]));
                }
                // Number keys select grid sizes, with 1 selecting one unit and 9 selecting 256.
                // Alt-number shortcuts select drawing shapes.
                _ if matches!(digit, Some(1..=9)) && !ctrl && !alt => {
                    if !set_grid_size(&mut grid_now, digit.unwrap_or(0)) {
                        return false;
                    }
                }
                _ if matches!(digit, Some(1..=8)) && alt && !ctrl => {
                    if !select_shape_kind(shape_parameters, digit.unwrap_or(0)) {
                        return false;
                    }
                }
                _ => return false,
            }
            *grid.borrow_mut() = grid_now;
        }
        // Every view, the 3D one included: a tool change has to reach it too, or its
        // handles lag a frame behind the flat views until the pointer moves over it.
        window.redraw();
        // Anything that changes the selection or the document changes what the inspector
        // and the toolbar should be showing.
        window.refresh_inspector();
        window.refresh_toolbar();
        true
    }
}

/// Shifts the material on the selected faces by whole texels.
pub fn shift_uv(document: &Rc<RefCell<Document>>, x: f32, y: f32) {
    // A whole texel is the smallest step that stays on the material's own grid.
    const STEP: f32 = 1.0;
    let delta = geometry_math::vec::Vec2f::new(x * STEP, y * STEP);
    document.borrow_mut().edit_faces(|face| face.move_uv(delta));
}

/// Turns the material on the selected faces.
pub fn rotate_uv(document: &Rc<RefCell<Document>>, degrees: f32) {
    document
        .borrow_mut()
        .edit_faces(|face| face.rotate_uv(degrees));
}

/// Which world axis a direction lies along.
///
/// `MapViewBase::flipAxis`, which takes the same view-relative direction a move would and
/// asks which axis it is: a flip is about an axis rather than along a direction.
pub fn axis_of(direction: Vec3d) -> Axis {
    let axis = (0..3)
        .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
        .unwrap_or(0);
    match axis {
        0 => Axis::X,
        1 => Axis::Y,
        _ => Axis::Z,
    }
}

/// Does the last run of edits again, to whatever is selected now.
///
/// `Menu/Edit/Repeat`, Ctrl+R. The work is the document's; this says what happened.
pub fn repeat_last_commands(document: &Rc<RefCell<Document>>, window: &dyn Window) {
    if document.borrow_mut().repeat() {
        crate::say!("{}", tr("Repeated the last commands"));
        window.redraw();
    } else {
        crate::say!("{}", tr("Nothing to repeat"));
    }
}

/// Which way a keyboard move goes, in the view the keyboard is in.
///
/// `geometry_math::direction`, whose names are the 3D view's: "forward" is up the screen of a flat
/// view and "up" is out of it, towards the viewer, which is what
/// `MapView2D::moveDirection` maps them to. The mapping reads oddly until you remember
/// that the up arrow means forward in the 3D view and up the screen in a flat one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Up the screen.
    Forward,
    /// Down the screen.
    Backward,
    /// Left across the screen.
    Left,
    /// Right across the screen.
    Right,
    /// Out of the screen, towards the viewer.
    Up,
    /// Into the screen.
    Down,
}

/// Which view a key was pressed in, which is what says where "left" is.
#[derive(Debug, Clone, Copy)]
pub enum Pressed {
    /// One of the flat views, by the plane it shows.
    Flat(camera2d::ViewPlane),
    /// The 3D view, from where its camera is looking.
    Perspective(camera3d::Camera3d),
}

/// The axis a vector points most nearly along, as a unit vector.
///
/// `geometry_math::get_abs_max_component_axis`: a keyboard move goes along a world axis, so a
/// camera that is looking somewhere between two of them has to pick one.
pub fn nearest_axis(vector: Vec3d) -> Vec3d {
    let axis = (0..3)
        .max_by(|&a, &b| vector[a].abs().total_cmp(&vector[b].abs()))
        .unwrap_or(0);
    let mut out = Vec3d::zero();
    out[axis] = if vector[axis] < 0.0 { -1.0 } else { 1.0 };
    out
}

/// Which way that is in the world, for the view the keyboard is in.
///
/// `MapView2D::moveDirection` and `MapView3D::moveDirection`, both snapped to an axis: a
/// flat view's own axes are world axes, and in the 3D view a move that went off at the
/// camera's own angle would leave the grid.
pub fn move_direction(pressed: Pressed, direction: Direction) -> Vec3d {
    let plane = match pressed {
        Pressed::Flat(plane) => plane,
        Pressed::Perspective(camera) => return camera_direction(&camera, direction),
    };
    let [horizontal, vertical] = plane.shown_axes();
    let mut delta = Vec3d::zero();
    match direction {
        Direction::Forward => delta[vertical] = 1.0,
        Direction::Backward => delta[vertical] = -1.0,
        Direction::Right => delta[horizontal] = 1.0,
        Direction::Left => delta[horizontal] = -1.0,
        Direction::Up => return -plane.view_direction(),
        Direction::Down => return plane.view_direction(),
    }
    delta
}

/// Which way that is for the 3D view. `MapView3D::moveDirection`.
///
/// Forward is where the camera is looking, flattened onto the ground and snapped to an
/// axis; up is the world's up, whatever the camera is doing, because that is what a person
/// looking at a level means by up.
pub fn camera_direction(camera: &camera3d::Camera3d, direction: Direction) -> Vec3d {
    let up = Vec3d::new(0.0, 0.0, 1.0);
    let forward = {
        let looking = camera.direction();
        let flattened = looking - up * geometry_math::vec::dot(looking, up);
        if geometry_math::vec::length(flattened) < 1e-6 {
            // Looking straight down or up, where the screen's own up is the only thing
            // left to go by.
            nearest_axis(camera.up()) * if looking.z() < 0.0 { 1.0 } else { -1.0 }
        } else {
            nearest_axis(flattened)
        }
    };
    let right = {
        let sideways = nearest_axis(camera.right());
        // Looking almost straight down, the camera's right can land on the same axis as
        // forward; turning it about the world's up is what separates them again.
        if sideways == forward {
            geometry_math::vec::cross(sideways, up)
        } else {
            sideways
        }
    };

    match direction {
        Direction::Forward => forward,
        Direction::Backward => -forward,
        Direction::Right => right,
        Direction::Left => -right,
        Direction::Up => up,
        Direction::Down => -up,
    }
}

/// How far a keyboard step on a material goes.
///
/// `UvActionMode`: plain is the grid, Shift is twice it -- a step that is felt -- and
/// Ctrl is the finest step the material has, which is one texel or one degree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvStep {
    /// The grid.
    Normal,
    /// Twice the grid, or a quarter turn.
    Coarse,
    /// One texel, or one degree.
    Fine,
}

impl UvStep {
    /// Which step the modifiers ask for.
    fn of(modifiers: Modifiers) -> Self {
        match modifiers {
            Modifiers { control: true, .. } => UvStep::Fine,
            Modifiers { shift: true, .. } => UvStep::Coarse,
            _ => UvStep::Normal,
        }
    }

    /// How many texels a move covers. `MapViewBase::moveUvDistance`.
    fn distance(self, grid: &grid::Grid) -> f32 {
        match self {
            UvStep::Fine => 1.0,
            UvStep::Coarse => 2.0 * grid.actual_size() as f32,
            UvStep::Normal => grid.actual_size() as f32,
        }
    }

    /// How far a turn goes. `MapViewBase::rotateUvAngle`.
    fn angle(self, grid: &grid::Grid) -> f32 {
        match self {
            UvStep::Fine => 1.0,
            UvStep::Coarse => 90.0,
            UvStep::Normal => grid.angle() as f32,
        }
    }
}

/// Does what an arrow or page key means where it was pressed, and says whether it meant
/// anything at all.
///
/// With faces selected, the directional keys move or rotate UVs. Otherwise they move objects,
/// duplicate with Control, or rotate with Alt. Directions depend on the active view.
#[allow(
    clippy::too_many_arguments,
    reason = "the keys act on whichever of these the tool in hand says"
)]
pub fn arrow_key(
    document: &Rc<RefCell<Document>>,
    grid: &grid::Grid,
    pressed: Option<Pressed>,
    key: Key,
    modifiers: Modifiers,
    tool: tool::Tool,
    handles: &Rc<RefCell<handle_tool::HandleManager>>,
    rotate: &Rc<RefCell<rotate_tool::RotateTool>>,
) -> bool {
    let direction = match key {
        Key::Left => Direction::Left,
        Key::Right => Direction::Right,
        Key::Up => Direction::Forward,
        Key::Down => Direction::Backward,
        Key::PageUp => Direction::Up,
        Key::PageDown => Direction::Down,
        _ => return false,
    };

    // A face selection takes the keys, which is what makes them the material's: a face is
    // only ever selected on purpose, and the material is what is being worked on while it
    // is.
    if document.borrow().face_selection_count() > 0 && !modifiers.alt {
        let step = UvStep::of(modifiers);
        match direction {
            // The page keys turn the material rather than moving it, since a material has
            // no third direction to be moved in.
            Direction::Up => rotate_uv(document, step.angle(grid)),
            Direction::Down => rotate_uv(document, -step.angle(grid)),
            _ => {
                let distance = step.distance(grid);
                let (x, y) = match direction {
                    Direction::Left => (-distance, 0.0),
                    Direction::Right => (distance, 0.0),
                    Direction::Forward => (0.0, -distance),
                    _ => (0.0, distance),
                };
                shift_uv(document, x, y);
            }
        }
        return true;
    }

    let Some(pressed) = pressed else {
        return false;
    };
    let delta = move_direction(pressed, direction) * grid.actual_size();

    // `MapViewBase::move` asks the tools first: with a handle tool in hand the keys move
    // the handles that are picked, and with the rotate tool they move the centre it turns
    // about. Only with neither do they move what is selected.
    if !modifiers.alt && !modifiers.control {
        if handle_tool::HandleKind::of(tool).is_some() {
            let picked = handles.borrow().selected_vertices();
            if picked.is_empty() {
                return false;
            }
            let moved = if tool == tool::Tool::ControlPoint {
                document.borrow_mut().move_control_points(&picked, delta)
            } else {
                document.borrow_mut().move_handles(&picked, delta)
            };
            if moved {
                // The handles have moved with what they moved, or the next press would
                // pick up nothing.
                handles.borrow_mut().move_selection(delta);
            }
            return moved;
        }
        if tool == tool::Tool::Rotate {
            let mut rotate = rotate.borrow_mut();
            if let Some(center) = rotate.center() {
                // `moveRotationCenter`: the ring's centre goes where the keys point,
                // which is how it is put on something that is not the middle.
                rotate.set_center(center + delta);
                return true;
            }
        }
    }

    if modifiers.alt {
        // `Roll`, `Yaw` and `Pitch`, a quarter turn about the view's own axes: Alt with
        // the arrows rolls and yaws, Alt with the page keys pitches.
        let (axis, clockwise) = match direction {
            Direction::Forward => (-move_direction(pressed, Direction::Forward), true),
            Direction::Backward => (-move_direction(pressed, Direction::Forward), false),
            Direction::Left => (move_direction(pressed, Direction::Up), true),
            Direction::Right => (move_direction(pressed, Direction::Up), false),
            Direction::Up => (move_direction(pressed, Direction::Right), true),
            Direction::Down => (move_direction(pressed, Direction::Right), false),
        };
        let axis = if clockwise { -axis } else { axis };
        // `MapViewBase::rotate` turns by a quarter with no rotate tool to say otherwise.
        return document.borrow_mut().rotate_selection(axis, 90.0);
    }

    if modifiers.control {
        // `duplicateAndMoveObjects`: the copies are made where they are going, so undo
        // takes back the copy and the move together.
        let copies = document.borrow_mut().duplicate_selection_by(delta);
        if copies == 0 {
            return false;
        }
        crate::say!(
            "{}",
            fill(
                "Duplicated {} one step along",
                &[&counted(copies, "object")]
            )
        );
        return true;
    }

    if modifiers.shift {
        // Shift is the material's, and there is no face selected to use it on.
        return false;
    }

    let moved = document.borrow_mut().translate_selection(delta);
    if moved {
        crate::say!(
            "{}",
            fill(
                "Moved by {} units",
                &[&(grid.actual_size() as i64).to_string()]
            )
        );
    }
    moved
}

/// Scales the material on the selected faces.
pub fn scale_uv(document: &Rc<RefCell<Document>>, factor: f32) {
    let factors = geometry_math::vec::Vec2f::new(factor, factor);
    document
        .borrow_mut()
        .edit_faces(|face| face.scale_uv(factors));
}

/// Switches to a tool, or back to selecting if it is already active.
///
/// Pressing a tool's key twice returns to selecting.
pub fn set_tool(
    current: &Rc<RefCell<tool::Tool>>,
    clip: &Rc<RefCell<clip_tool::ClipTool>>,
    handles: &Rc<RefCell<handle_tool::HandleManager>>,
    rotate: &Rc<RefCell<rotate_tool::RotateTool>>,
    assemble: &Rc<RefCell<assemble_tool::AssembleBrushTool>>,
    wanted: tool::Tool,
) {
    let mut current = current.borrow_mut();
    let next = if *current == wanted {
        tool::Tool::Select
    } else {
        wanted
    };
    // Leaving a tool forgets what it was working on, as each `doDeactivate` does.
    if *current == tool::Tool::Clip && next != tool::Tool::Clip {
        clip.borrow_mut().reset();
    }
    // Handles are dropped when the next tool wants a different kind of them. Going to a
    // tool with no handles at all -- selecting, most of the time -- keeps what was picked,
    // so that edges taken with the edge tool are still there for the fillet panel to round
    // once the pointer is back to selecting.
    if let Some(kind) = handle_tool::HandleKind::of(next) {
        if handle_tool::HandleKind::of(*current) != Some(kind) {
            handles.borrow_mut().clear();
        }
    }
    if *current == tool::Tool::Rotate && next != tool::Tool::Rotate {
        rotate.borrow_mut().clear();
    }
    if *current == tool::Tool::Assemble && next != tool::Tool::Assemble {
        assemble.borrow_mut().reset();
    }
    *current = next;
}

pub fn place_entity(document: &Rc<RefCell<Document>>, grid_size: f64) {
    let classname = crate::entity_placement::default_class(&document.borrow());
    crate::entity_placement::place_entity(document, &classname, grid_size);
}

/// Rotates the selection a quarter turn about the given axis.
///
/// Shift turns it the other way, which is quicker than pressing the key three times.
pub fn rotate_by_quarter_turn(document: &Rc<RefCell<Document>>, axis: Axis, backwards: bool) {
    let direction = if backwards { -90.0 } else { 90.0 };
    let mut axis_vector = Vec3d::zero();
    axis_vector[axis.index()] = 1.0;

    if !document
        .borrow_mut()
        .rotate_selection(axis_vector, direction)
    {
        crate::say!("{}", tr("Nothing to rotate"));
    }
}

/// Mirrors the selection along the given axis.
pub fn flip(document: &Rc<RefCell<Document>>, axis: Axis) {
    if !document.borrow_mut().flip_selection(axis) {
        crate::say!("{}", tr("Nothing to flip"));
    }
}

/// Sets the grid size from a number key: 1 is a grid of 1, 5 is 16, 9 is 256.
///
/// Sets the grid size directly; unsupported sizes are ignored.
pub fn set_grid_size(grid: &mut Grid, digit: u32) -> bool {
    // Keys 1 through 9 select grid sizes from 1 through 256.
    let Some(exponent) = digit.checked_sub(1).map(|step| step as i32) else {
        return false;
    };
    if digit == 0 {
        return false;
    }
    if !(grid::MIN_SIZE..=grid::MAX_SIZE).contains(&exponent) {
        return false;
    }

    *grid = grid.with_size(exponent);
    crate::say!(
        "{}",
        fill("Grid size {}", &[&grid.actual_size().to_string()])
    );
    true
}

/// Picks which shape the draw tool makes, from the number keys.
///
/// The keys follow the order the toolbar lists the shapes in, so 1 is a cuboid. Choosing
/// a shape does not place anything: the shape is what the next drag in
/// a 2D view will draw.
pub fn select_shape_kind(
    parameters: &Rc<RefCell<shape_tool::ShapeParameters>>,
    digit: u32,
) -> bool {
    if digit == 0 {
        return false;
    }
    let index = digit as usize - 1;

    let Some(kind) = shape_tool::ShapeKind::ALL.get(index).copied() else {
        return false;
    };
    parameters.borrow_mut().kind = kind;
    crate::say!("{}", fill("Shape: {}", &[tr(kind.name())]));
    true
}
