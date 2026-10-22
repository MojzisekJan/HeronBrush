//! The editor state every part of the window shares: the document and the tools, set up
//! before the views are built.

use std::cell::RefCell;
use std::rc::Rc;

use editor_core::camera2d::ViewPlane;
use editor_core::document::Document;
use editor_core::grid::Grid;
use editor_core::tool::Tool;
use editor_core::view2d::ViewportState;
use editor_core::world::WorldBounds;
use editor_core::{
    assemble_tool, clip_tool, fillet_tool, handle_tool, rotate_tool, shape_tool, sweep_tool,
    view_filters,
};
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

/// Everything the views, bars and panels share.
#[derive(Default)]
pub struct TargetPick {
    pub source: Option<map_model::node::NodeId>,
    /// What a click in a view hit while picking: `Some(None)` is a click on nothing.
    pub picked: Option<Option<map_model::node::NodeId>>,
}

pub struct Editor {
    pub target_pick: Rc<RefCell<TargetPick>>,
    /// The open document.
    pub document: Rc<RefCell<Document>>,
    /// Where the document's materials, definitions and settings came from.
    pub sources: RefCell<editor_core::document_setup::Sources>,
    /// One grid, shared by every view and by the shortcuts that change it.
    pub grid: Rc<RefCell<Grid>>,
    /// The bounds of the editable world.
    pub bounds: WorldBounds,
    /// Which shape the next brush drag draws.
    pub shape_parameters: Rc<RefCell<shape_tool::ShapeParameters>>,
    /// The tool in hand.
    pub tool: Rc<RefCell<Tool>>,
    /// The clip tool's points.
    pub clip: Rc<RefCell<clip_tool::ClipTool>>,
    /// The handles the vertex, edge and face tools drag.
    pub handles: Rc<RefCell<handle_tool::HandleManager>>,
    /// The rotate tool's centre and angle.
    pub rotate: Rc<RefCell<rotate_tool::RotateTool>>,
    /// The assemble tool's points.
    pub assemble: Rc<RefCell<assemble_tool::AssembleBrushTool>>,
    /// What the fillet tool rounds by.
    pub fillet: Rc<RefCell<fillet_tool::FilletParameters>>,
    /// The box a brush drag would make, shared so every view shows it.
    pub pending_shape: Rc<RefCell<Option<Bbox3d>>>,
    /// What the views draw.
    pub filters: Rc<RefCell<view_filters::ViewFilters>>,
    /// The sweep tool's transform and parameters.
    pub sweep: Rc<RefCell<sweep_tool::SweepInputs>>,
    /// Where a paste lands: the point the pointer is over.
    pub paste_target: Rc<RefCell<Option<Vec3d>>>,
    /// Which flat view the pointer was last over, for the arrow keys.
    pub active_plane: Rc<RefCell<Option<ViewPlane>>>,
    /// The main 3D view's camera, which the window's shortcuts also move.
    pub camera: Rc<RefCell<editor_core::camera3d::Camera3d>>,
    /// Every flat view's state, so their cameras move together.
    pub cameras: Rc<RefCell<Vec<Rc<RefCell<ViewportState>>>>>,
    /// Asks for the window to be drawn again.
    pub redraw: Rc<dyn Fn()>,
    /// Asks the panels to catch up. The egui panels read the document every frame, so
    /// this only asks for a frame.
    pub refresh_panels: Rc<dyn Fn()>,
    /// Motion-path clicks and seeks from the views, for the animation panel.
    pub animation_link: Rc<RefCell<crate::animation_gizmo::Link>>,
}

impl Editor {
    /// Opens `path` -- its definitions and materials found beside it -- or a new map when
    /// there is none. The tests' way in; the application names definitions and materials.
    #[cfg(test)]
    pub fn open(ctx: &eframe::egui::Context, path: Option<std::path::PathBuf>) -> Self {
        Self::open_with(ctx, path, None, None)
    }

    /// [`Editor::open`] with entity definitions and materials named on the command line,
    /// which take precedence over the ones found beside the map or from the game.
    pub fn open_with(
        ctx: &eframe::egui::Context,
        path: Option<std::path::PathBuf>,
        fgd: Option<std::path::PathBuf>,
        materials: Option<std::path::PathBuf>,
    ) -> Self {
        let loaded = editor_core::document_setup::load(path, fgd, materials);
        if let Some(path) = loaded.document.path.as_deref() {
            editor_core::recent_documents::record(path);
        }
        let redraw: Rc<dyn Fn()> = Rc::new({
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        });
        Self {
            target_pick: Default::default(),
            document: Rc::new(RefCell::new(loaded.document)),
            sources: RefCell::new(loaded.sources),
            grid: Rc::new(RefCell::new(Grid::default())),
            bounds: WorldBounds::default(),
            shape_parameters: Rc::new(RefCell::new(shape_tool::ShapeParameters::default())),
            tool: Rc::new(RefCell::new(Tool::default())),
            clip: Rc::new(RefCell::new(clip_tool::ClipTool::new())),
            handles: Rc::new(RefCell::new(handle_tool::HandleManager::new())),
            rotate: Rc::new(RefCell::new(rotate_tool::RotateTool::new())),
            assemble: Rc::new(RefCell::new(assemble_tool::AssembleBrushTool::new())),
            fillet: Rc::new(RefCell::new(fillet_tool::FilletParameters::default())),
            pending_shape: Rc::new(RefCell::new(None)),
            filters: Rc::new(RefCell::new(view_filters::ViewFilters::default())),
            sweep: Rc::new(RefCell::new(sweep_tool::SweepInputs::default())),
            paste_target: Rc::new(RefCell::new(None)),
            active_plane: Rc::new(RefCell::new(None)),
            camera: Rc::new(RefCell::new(editor_core::camera3d::Camera3d::default())),
            cameras: Rc::new(RefCell::new(Vec::new())),
            refresh_panels: Rc::clone(&redraw),
            animation_link: Default::default(),
            redraw,
        }
    }
}

impl Editor {
    /// The face tool's handles narrow property/UV editing without changing its geometry selection.
    pub fn sync_face_edit_targets(&self) {
        let targets = if *self.tool.borrow() == Tool::Face {
            self.document
                .borrow()
                .faces_at(&self.handles.borrow().selected_face_centres())
        } else {
            Vec::new()
        };
        self.document.borrow_mut().set_face_edit_targets(targets);
    }
}
