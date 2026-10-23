//! Which editing tool is active.
//!
//! Only one editing tool is active at a time. Pressing its button again or Escape returns to
//! selection.

use crate::i18n::translation_key;
use std::fmt;

/// The active editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tool {
    /// Click to select, drag to move, drag on nothing to sweep out a selection.
    #[default]
    Select,
    /// Drag in a flat view to draw the chosen shape.
    Brush,
    /// Cut brushes with a plane given by two or three points.
    Clip,
    /// Drag a brush's corners.
    Vertex,
    /// Drag a brush's edges.
    Edge,
    /// Drag a brush's faces.
    Face,
    /// Turn the selection about its centre.
    Rotate,
    /// Resize the selection about its centre.
    Scale,
    /// Slant the selection.
    Shear,
    /// Build a brush by clicking the points it should enclose.
    Assemble,
    /// Make geometry by sweeping the selected faces along a path.
    Sweep,
    /// Drag a patch's control points.
    ControlPoint,
    /// Round the edge that is clicked, or the edges already picked.
    ///
    /// Rounds brush edges.
    Fillet,
    /// Put the current material on the face under the pointer.
    ///
    /// Assigns materials without changing geometry.
    Paint,
    /// Raise, lower, smooth or flatten the selected terrain under a round brush.
    Sculpt,
}

impl Tool {
    /// The tools that go in the window's title bar, in the order `ALL` has them.
    ///
    /// The title bar has room for a handful of buttons, not eleven, so it carries the
    /// ones a hand switches between while shaping a brush: selecting, the clip plane, the
    /// three that drag geometry, and the two transforms. What is left is for building
    /// from scratch, which comes in stretches rather than between every edit, and keeps
    /// the row below.
    pub const ESSENTIAL: [Tool; 8] = [
        Tool::Select,
        Tool::Brush,
        Tool::Clip,
        Tool::Vertex,
        Tool::Edge,
        Tool::Face,
        Tool::Rotate,
        Tool::Scale,
    ];

    /// Every tool, in the order the toolbar shows them.
    pub const ALL: [Tool; 15] = [
        Tool::Select,
        Tool::Brush,
        Tool::Clip,
        Tool::Vertex,
        Tool::Edge,
        Tool::Face,
        Tool::Rotate,
        Tool::Scale,
        Tool::Shear,
        Tool::Assemble,
        Tool::Sweep,
        Tool::ControlPoint,
        Tool::Fillet,
        Tool::Paint,
        Tool::Sculpt,
    ];

    /// The name shown on the toolbar button.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => translation_key("Select"),
            Tool::Brush => translation_key("Brush"),
            Tool::Clip => translation_key("Clip"),
            Tool::Vertex => translation_key("Vertex"),
            Tool::Edge => translation_key("Edge"),
            Tool::Face => translation_key("Face"),
            Tool::Rotate => translation_key("Rotate"),
            Tool::Scale => translation_key("Scale"),
            Tool::Shear => translation_key("Shear"),
            Tool::Assemble => translation_key("Assemble"),
            Tool::Sweep => translation_key("Sweep"),
            Tool::ControlPoint => translation_key("Control point"),
            Tool::Fillet => translation_key("Fillet"),
            Tool::Paint => translation_key("Paint"),
            Tool::Sculpt => translation_key("Sculpt"),
        }
    }

    /// The tooltip, without the shortcut: [`Self::shortcut_id`] says which key it is now.
    pub fn tooltip(self) -> &'static str {
        match self {
            Tool::Select => translation_key("Select and move"),
            Tool::Brush => translation_key("Brush tool: drag out the chosen shape"),
            Tool::Clip => translation_key("Clip tool"),
            Tool::Vertex => translation_key("Vertex tool"),
            Tool::Edge => translation_key("Edge tool"),
            Tool::Face => translation_key("Face tool"),
            Tool::Rotate => translation_key("Rotate tool"),
            Tool::Scale => translation_key("Scale tool"),
            Tool::Shear => translation_key("Shear tool"),
            Tool::Assemble => translation_key("Assemble brush tool"),
            Tool::Sweep => translation_key("Sweep tool"),
            Tool::ControlPoint => translation_key("Control point tool (P)"),
            Tool::Fillet => {
                translation_key("Fillet tool: select preview edges, then Apply or Enter")
            }
            Tool::Paint => translation_key("Paint material"),
            Tool::Sculpt => {
                translation_key("Sculpt terrain: drag over it, Shift swaps raise and lower")
            }
        }
    }

    /// The id of the shortcut that picks this tool, in
    /// [`crate::custom_shortcuts::COMMANDS`], so a tooltip can show the key bound now
    /// rather than the default. `None` for a tool without one.
    pub fn shortcut_id(self) -> Option<&'static str> {
        Some(match self {
            Tool::Select => "tool_select",
            Tool::Brush => "tool_brush",
            Tool::Clip => "tool_clip",
            Tool::Vertex => "tool_vertex",
            Tool::Edge => "tool_edge",
            Tool::Face => "tool_face",
            Tool::Rotate => "tool_rotate",
            Tool::Scale => "tool_scale",
            Tool::Shear => "tool_shear",
            Tool::Assemble => "tool_assemble",
            Tool::Sweep => "tool_sweep",
            Tool::Fillet => "tool_fillet",
            Tool::Paint => "tool_paint",
            Tool::Sculpt => "tool_sculpt",
            Tool::ControlPoint => return None,
        })
    }

    /// The icon the toolbar button shows, as SVG.
    ///
    /// The editor's own, not the system theme's: an isometric cube with the element the
    /// tool works on picked out in colour -- a face for the face tool, an edge for the
    /// edge tool -- the way Blender and the CAD tools draw theirs. See `assets/tools`.
    pub fn icon_svg(self) -> &'static str {
        match self {
            Tool::Select => include_str!("../../heron-brush/assets/tools/select.svg"),
            Tool::Brush => include_str!("../../heron-brush/assets/tools/brush.svg"),
            Tool::Clip => include_str!("../../heron-brush/assets/tools/clip.svg"),
            Tool::Vertex => include_str!("../../heron-brush/assets/tools/vertex.svg"),
            Tool::Edge => include_str!("../../heron-brush/assets/tools/edge.svg"),
            Tool::Face => include_str!("../../heron-brush/assets/tools/face.svg"),
            Tool::Rotate => include_str!("../../heron-brush/assets/tools/rotate.svg"),
            Tool::Scale => include_str!("../../heron-brush/assets/tools/scale.svg"),
            Tool::Shear => include_str!("../../heron-brush/assets/tools/shear.svg"),
            Tool::Assemble => include_str!("../../heron-brush/assets/tools/assemble.svg"),
            Tool::Sweep => include_str!("../../heron-brush/assets/tools/sweep.svg"),
            Tool::ControlPoint => include_str!("../../heron-brush/assets/tools/controlpoint.svg"),
            Tool::Fillet => include_str!("../../heron-brush/assets/tools/fillet.svg"),
            Tool::Paint => include_str!("../../heron-brush/assets/tools/paint.svg"),
            Tool::Sculpt => include_str!("../../heron-brush/assets/tools/sculpt.svg"),
        }
    }

    /// Returns whether this tool drags the geometry's own handles rather than whole
    /// objects.
    ///
    /// The three handle tools differ only in what a drag grabs, so they share their
    /// input handling.
    #[allow(
        dead_code,
        reason = "used once the handle tools are wired into the views"
    )]
    pub fn is_handle_tool(self) -> bool {
        matches!(
            self,
            Tool::Vertex | Tool::Edge | Tool::Face | Tool::ControlPoint | Tool::Fillet
        )
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_is_the_tool_the_editor_starts_in() {
        assert_eq!(Tool::default(), Tool::Select);
    }

    #[test]
    fn every_tool_is_on_the_toolbar_exactly_once() {
        let mut seen = Tool::ALL.to_vec();
        let count = seen.len();
        seen.sort_by_key(|tool| tool.name());
        seen.dedup();
        assert_eq!(seen.len(), count);
    }

    #[test]
    fn the_title_bar_tools_are_a_subset_of_the_toolbar_in_the_same_order() {
        // A tool that is in one list and not the other, or in a different order, would
        // move under the reader between the two rows it is shown on.
        let kept: Vec<Tool> = Tool::ALL
            .into_iter()
            .filter(|tool| Tool::ESSENTIAL.contains(tool))
            .collect();
        assert_eq!(kept, Tool::ESSENTIAL.to_vec());
    }

    #[test]
    fn the_handle_tools_are_the_three_that_grab_geometry() {
        for tool in Tool::ALL {
            assert_eq!(
                tool.is_handle_tool(),
                matches!(
                    tool,
                    Tool::Vertex | Tool::Edge | Tool::Face | Tool::ControlPoint | Tool::Fillet
                ),
                "{tool}"
            );
        }
    }

    #[test]
    fn every_tool_names_itself_and_its_shortcut_command() {
        for tool in Tool::ALL {
            assert!(!tool.name().is_empty());
            assert!(tool.icon_svg().starts_with("<svg"), "{tool}: no icon");
            assert!(!tool.tooltip().is_empty());
            // The shortcut comes from the bindings, which a user can change, so a tool
            // names its command there rather than printing a key.
            if let Some(id) = tool.shortcut_id() {
                assert!(
                    crate::custom_shortcuts::COMMANDS.iter().any(|c| c.id == id),
                    "{tool}: {id}"
                );
            }
        }
    }
}
