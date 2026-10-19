//! Undo and redo.
//!
//! Each edit records the brushes it touched, before and after. Undoing restores the
//! "before" state and redoing restores the "after", so the two directions are the same
//! operation with the two halves swapped.
//!
//! This records what changed rather than snapshotting the whole document, so the memory
//! an edit costs is proportional to what it touched. Snapshotting would be simpler, but a
//! map with a few thousand brushes would make every nudge of one of them expensive.

use map_model::node::{Node, NodeId, NodeKind};

/// How many edits can be undone.
///
/// Deep enough to cover any run of work a user would want back, and shallow enough that
/// a long session cannot grow without bound.
pub const CAPACITY: usize = 256;

/// How much of the undo stack is kept, in bytes.
///
/// The count alone is not a bound on memory. An edit holds the nodes it touched, before
/// and after, so one that moved a single brush is a few kilobytes and one that moved a
/// selection of four thousand is a couple of hundred megabytes -- and 256 of *those*
/// have no ceiling worth the name. Whichever limit bites first wins.
///
/// A quarter of a gigabyte is generous for the ordinary case (thousands of small edits
/// fit) and firm for the pathological one.
pub const BUDGET: usize = 256 * 1024 * 1024;

/// A rough count of the bytes a slice of nodes holds, for [`History`]'s budget.
///
/// An estimate, deliberately: what an allocator actually holds cannot be asked for, and
/// the number is only used to decide when the oldest step goes. It counts the things
/// that scale with the map -- a brush's faces and the winged-edge mesh under them, which
/// is the bulk of a brush -- and takes the rest at the size of the struct.
fn weigh(nodes: &[Node]) -> usize {
    nodes.iter().map(weigh_node).sum()
}

fn weigh_node(node: &Node) -> usize {
    let inside = match &node.kind {
        NodeKind::Brush(brush) => {
            let geometry = brush.geometry();
            std::mem::size_of_val(brush.faces())
                + std::mem::size_of_val(geometry.vertices())
                + std::mem::size_of_val(geometry.half_edges())
                + std::mem::size_of_val(geometry.edges())
                + std::mem::size_of_val(geometry.faces())
        }
        NodeKind::Group(group) => weigh(&group.children),
        NodeKind::Entity(entity) => weigh(&entity.children),
        NodeKind::Layer(layer) => weigh(&layer.children),
        NodeKind::Patch(patch) => {
            std::mem::size_of_val(patch.control_points.as_slice())
        }
    };
    std::mem::size_of::<Node>() + inside
}

/// One reversible edit.
///
/// Edits work on whole nodes rather than only on brushes, so moving a brush, renaming an
/// entity and deleting a group all undo through the same machinery.
#[derive(Debug, Clone)]
pub enum Edit {
    /// Nodes were replaced with new versions of themselves.
    Changed {
        /// What they were.
        before: Vec<Node>,
        /// What they became.
        after: Vec<Node>,
    },
    /// Nodes were added to a layer.
    Added {
        /// Which layer they went into, by its position in the world's layer list.
        layer: usize,
        /// The nodes, so redo can put them back.
        nodes: Vec<Node>,
    },
    /// Nodes were removed from a layer.
    Removed {
        /// Which layer they came out of.
        layer: usize,
        /// The nodes, so undo can put them back.
        nodes: Vec<Node>,
    },
    /// Root order within a layer, without copying brush geometry.
    Reordered { layer: usize, before: Vec<NodeId>, after: Vec<NodeId> },
    /// Several edits that are one thing to the user.
    ///
    /// A CSG operation takes brushes out and puts others in, and someone who asked for
    /// one thing expects one press of undo to take it back. The parts are applied in
    /// order and undone in reverse, which is what makes putting back what was removed
    /// happen after taking away what was added.
    Batch(Vec<Edit>),
}

impl Edit {
    /// Roughly what this edit holds, in bytes. See [`weigh`].
    pub fn weight(&self) -> usize {
        match self {
            Edit::Changed { before, after } => weigh(before) + weigh(after),
            Edit::Added { nodes, .. } | Edit::Removed { nodes, .. } => weigh(nodes),
            Edit::Reordered { before, after, .. } => (before.len() + after.len()) * std::mem::size_of::<NodeId>(),
            Edit::Batch(parts) => parts.iter().map(Edit::weight).sum(),
        }
    }

    /// Returns this edit with its two directions swapped.
    ///
    /// Applying an edit and then its reverse leaves the document as it was, which is what
    /// makes undo and redo the same machinery.
    pub fn reversed(self) -> Self {
        match self {
            Edit::Changed { before, after } => Edit::Changed {
                before: after,
                after: before,
            },
            Edit::Added { layer, nodes } => Edit::Removed { layer, nodes },
            Edit::Removed { layer, nodes } => Edit::Added { layer, nodes },
            Edit::Reordered { layer, before, after } => Edit::Reordered { layer, before: after, after: before },
            Edit::Batch(parts) => {
                Edit::Batch(parts.into_iter().rev().map(Edit::reversed).collect())
            }
        }
    }

    /// Returns a short description, for the status line.
    pub fn description(&self) -> String {
        match self {
            Edit::Changed { after, .. } => {
                format!("change {}", count(after.len(), "object", "objects"))
            }
            Edit::Added { nodes, .. } => {
                format!("add {}", count(nodes.len(), "object", "objects"))
            }
            Edit::Removed { nodes, .. } => {
                format!("delete {}", count(nodes.len(), "object", "objects"))
            }
            Edit::Reordered { .. } => "reorder objects".into(),
            // The last part is what the operation left behind, which is the half worth
            // naming: "add six objects" says more than "a batch of two".
            Edit::Batch(parts) => parts
                .last()
                .map_or_else(|| "do nothing".to_string(), Edit::description),
        }
    }
}

fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", text_utils::string_format::str_plural(n, singular, plural))
}

/// The undo and redo stacks.
#[derive(Debug)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    /// What the undo stack weighs, by [`Edit::weight`], kept as steps go on and come off
    /// rather than recomputed -- the estimate walks every node it holds.
    weight: usize,
    /// The most the undo stack may weigh. [`BUDGET`] unless a caller says otherwise,
    /// which only the tests do -- they would otherwise have to build a quarter of a
    /// gigabyte of geometry to reach it.
    budget: usize,
}

impl Default for History {
    fn default() -> Self {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
            weight: 0,
            budget: BUDGET,
        }
    }
}

impl History {
    /// Creates an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an edit that has just been applied.
    ///
    /// This clears the redo stack: once the document has moved in a new direction, the
    /// edits that were undone can no longer be replayed onto it.
    pub fn record(&mut self, edit: Edit) {
        self.weight += edit.weight();
        self.undo.push(edit);
        self.forget_the_oldest();
        self.redo.clear();
    }

    /// Drops the oldest steps until the stack is within both limits.
    ///
    /// The newest is never dropped, however heavy: an edit that on its own exceeds the
    /// budget is exactly the one a person most wants back, and refusing to hold it would
    /// make undo quietly do nothing.
    fn forget_the_oldest(&mut self) {
        while self.undo.len() > 1 && (self.undo.len() > CAPACITY || self.weight > self.budget) {
            let dropped = self.undo.remove(0);
            self.weight = self.weight.saturating_sub(dropped.weight());
        }
    }

    /// How many undo steps have been recorded, for [`Self::coalesce_since`].
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Folds every step recorded since `mark` into one, so a run of separate edits undoes
    /// in a single press. A no-op when nothing was recorded since.
    ///
    /// The weight does not change: the same nodes are held, in one step instead of many.
    pub fn coalesce_since(&mut self, mark: usize) {
        let mark = mark.min(self.undo.len());
        let steps: Vec<Edit> = self.undo.drain(mark..).collect();
        match steps.len() {
            0 => {}
            1 => self.undo.push(steps.into_iter().next().expect("length checked")),
            _ => self.undo.push(Edit::Batch(steps)),
        }
    }

    /// Replaces the latest insertion while a shape is still being configured.
    /// A strict ID check prevents amending unrelated history; only the latest
    /// geometry is retained, rather than a batch containing every intermediate mesh.
    pub(crate) fn amend_added(&mut self, layer: usize, ids: &[map_model::node::NodeId], nodes: Vec<Node>) -> bool {
        let Some(Edit::Added { layer: old_layer, nodes: old }) = self.undo.last_mut() else {return false;};
        if *old_layer!=layer || old.len()!=ids.len() || !old.iter().zip(ids).all(|(n,id)|n.id==*id) {return false;}
        let before=weigh(old);
        *old=nodes;
        self.weight=self.weight.saturating_sub(before)+weigh(old);
        self.redo.clear();
        self.forget_the_oldest();
        true
    }

    /// Takes the next edit to undo, and remembers it for redo.
    ///
    /// The caller applies the returned edit; it is already reversed.
    pub fn undo(&mut self) -> Option<Edit> {
        let edit = self.undo.pop()?;
        self.weight = self.weight.saturating_sub(edit.weight());
        self.redo.push(edit.clone());
        Some(edit.reversed())
    }

    /// Takes the next edit to redo, and remembers it for undo.
    pub fn redo(&mut self) -> Option<Edit> {
        let edit = self.redo.pop()?;
        self.weight += edit.weight();
        self.undo.push(edit.clone());
        // A redone step can be the one that puts the stack over, the same as a new one.
        self.forget_the_oldest();
        Some(edit)
    }

    /// Returns what undoing would do, or `None` if there is nothing to undo. Kept for the
    /// tests and for a caller that wants to name the next step; the views no longer paint
    /// it.
    #[allow(dead_code)]
    pub fn next_undo(&self) -> Option<String> {
        self.undo.last().map(Edit::description)
    }

    /// Returns what redoing would do, or `None` if there is nothing to redo.
    #[allow(dead_code)]
    pub fn next_redo(&self) -> Option<String> {
        self.redo.last().map(Edit::description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(n: usize) -> Edit {
        Edit::Added {
            layer: n,
            nodes: Vec::new(),
        }
    }

    #[test]
    fn an_empty_history_has_nothing_to_do() {
        let mut history = History::new();
        assert_eq!(history.next_undo(), None);
        assert_eq!(history.next_redo(), None);
        assert!(history.undo().is_none());
        assert!(history.redo().is_none());
    }

    #[test]
    fn undoing_moves_an_edit_onto_the_redo_stack() {
        let mut history = History::new();
        history.record(edit(1));

        assert!(history.next_undo().is_some());
        assert!(history.next_redo().is_none());

        let undone = history.undo().expect("there is something to undo");
        // What comes back is reversed, ready to apply.
        assert!(matches!(undone, Edit::Removed { layer: 1, .. }));
        assert!(history.next_undo().is_none());
        assert!(history.next_redo().is_some());

        let redone = history.redo().expect("there is something to redo");
        assert!(matches!(redone, Edit::Added { layer: 1, .. }));
        assert!(history.next_undo().is_some());
        assert!(history.next_redo().is_none());
    }

    #[test]
    fn a_new_edit_discards_what_was_undone() {
        let mut history = History::new();
        history.record(edit(1));
        history.undo();
        assert!(history.next_redo().is_some());

        history.record(edit(2));
        // The document has moved in a new direction, so the old branch is gone.
        assert!(history.next_redo().is_none());
    }

    #[test]
    fn edits_undo_in_the_order_they_were_made_reversed() {
        let mut history = History::new();
        history.record(edit(1));
        history.record(edit(2));
        history.record(edit(3));

        for expected in [3, 2, 1] {
            let undone = history.undo().expect("there is something to undo");
            let Edit::Removed { layer, .. } = undone else {
                panic!("expected a reversed add");
            };
            assert_eq!(layer, expected);
        }
        assert!(history.next_undo().is_none());
    }

    #[test]
    fn the_history_does_not_grow_without_bound() {
        let mut history = History::new();
        for i in 0..CAPACITY + 50 {
            history.record(edit(i));
        }
        assert_eq!(history.undo.len(), CAPACITY);
        // The oldest edits fell off the bottom, so the first one still there is not 0.
        let Edit::Added { layer, .. } = history.undo[0] else {
            panic!("expected an add");
        };
        assert_eq!(layer, 50);
    }

    /// A cube's worth of node, so a test edit weighs something real.
    fn a_brush_node() -> Node {
        let world = geometry_math::bbox::Bbox3d::from_scalars(-16384.0, 16384.0);
        let brush = map_model::shapes::create_cuboid(
            &world,
            &geometry_math::bbox::Bbox3d::from_scalars(-8.0, 8.0),
            "stone",
        )
        .expect("a cube");
        Node::new(NodeKind::Brush(brush))
    }

    #[test]
    fn a_brush_weighs_more_than_the_node_around_it() {
        let weight = weigh(&[a_brush_node()]);
        println!("a cube node weighs {weight} bytes");
        // The faces alone are six of them; the winged-edge mesh under them is the bulk.
        assert!(
            weight > 6 * std::mem::size_of::<map_model::brush_face::BrushFace>(),
            "a cube weighed {weight}, which is less than its faces"
        );
    }

    #[test]
    fn heavy_edits_are_held_to_the_budget_long_before_the_count() {
        let one = weigh(&[a_brush_node()]);
        // A budget of three brushes, so the rule can be watched rather than inferred.
        let mut history = History { budget: one * 3, ..History::default() };
        for layer in 0..10 {
            history.record(Edit::Added { layer, nodes: vec![a_brush_node()] });
        }
        assert!(history.weight <= history.budget, "the stack weighs {}", history.weight);
        assert!(
            history.undo.len() < CAPACITY,
            "ten steps fit the count; it is the budget that must have stopped this at {}",
            history.undo.len()
        );
        // What is left is the newest, in order.
        let Edit::Added { layer, .. } = history.undo[history.undo.len() - 1] else {
            panic!("expected an add");
        };
        assert_eq!(layer, 9);
    }

    #[test]
    fn one_edit_larger_than_the_whole_budget_is_still_kept() {
        let mut history = History { budget: 1, ..History::default() };
        history.record(Edit::Added { layer: 0, nodes: vec![a_brush_node()] });
        history.record(Edit::Added { layer: 1, nodes: vec![a_brush_node()] });
        assert_eq!(history.undo.len(), 1, "the newest step is never the one dropped");
        assert!(history.weight > history.budget, "and it is over the budget on its own");
        let Edit::Added { layer, .. } = history.undo[0] else {
            panic!("expected an add");
        };
        assert_eq!(layer, 1);
    }

    #[test]
    fn undoing_and_redoing_keep_the_running_weight_honest() {
        let mut history = History::new();
        for layer in 0..4 {
            history.record(Edit::Added { layer, nodes: vec![a_brush_node()] });
        }
        let full = history.weight;
        history.undo();
        history.undo();
        assert_eq!(history.weight, history.undo.iter().map(Edit::weight).sum::<usize>());
        history.redo();
        history.redo();
        assert_eq!(history.weight, full, "back where it started");
    }

    #[test]
    fn reversing_twice_gives_back_the_original() {
        let original = Edit::Changed {
            before: Vec::new(),
            after: Vec::new(),
        };
        assert!(matches!(
            original.clone().reversed().reversed(),
            Edit::Changed { .. }
        ));

        assert!(matches!(edit(1).reversed().reversed(), Edit::Added { .. }));
    }

    #[test]
    fn descriptions_read_as_actions() {
        assert_eq!(
            Edit::Added {
                layer: 0,
                nodes: Vec::new()
            }
            .description(),
            "add 0 objects"
        );
    }
}
