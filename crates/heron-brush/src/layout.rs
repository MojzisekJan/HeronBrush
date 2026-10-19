//! How the four map views are arranged: one cycling pane, the 3D view beside a cycling
//! one, the 3D view beside the top view over a cycling side view, or the classic four. Any of them can show one view on
//! its own (Ctrl+Space), and Space moves a cycling pane to its next view.
//!
//! Views are numbered as `Panes` numbers them: 0 the 3D view, 1 top (XY), 2 front (XZ),
//! 3 side (YZ).

use std::collections::HashMap;

use eframe::egui;
use editor_core::preferences::Layout;

use crate::theme;

/// A node of the arrangement.
enum Node {
    /// One view.
    View(usize),
    /// Several views one at a time; which one shows is kept per pane.
    Cycling(&'static str, Vec<usize>),
    /// Two nodes side by side (`horizontal`) or one above the other, with a splitter.
    Split { id: &'static str, horizontal: bool, first: Box<Node>, second: Box<Node> },
}

/// The arrangement's state, kept across frames.
#[derive(Default)]
pub struct Panes {
    /// Which view each cycling pane shows, by index into its list.
    cycles: HashMap<&'static str, usize>,
    /// Where each splitter sits, as a fraction.
    splits: HashMap<&'static str, f32>,
    /// Which view is shown on its own, if any.
    pub maximised: Option<usize>,
    /// Which view has the keyboard: the one a press last landed in.
    pub focused: Option<usize>,
    /// Whether the 3D view is in a window of its own.
    pub detached: bool,
    /// The arrangement last laid out, so changing it can drop a maximised view.
    built: Option<(Layout, bool)>,
}

impl Panes {
    fn tree(&self, layout: Layout) -> Node {
        use Node::*;
        let split = |id, horizontal, a, b| Split { id, horizontal, first: Box::new(a), second: Box::new(b) };
        if self.detached {
            return match layout {
                Layout::One | Layout::Two => Cycling("flat", vec![1, 2, 3]),
                Layout::Three => split("h", true, View(1), Cycling("sides", vec![2, 3])),
                Layout::Four => split("h", true, View(1), split("right", false, View(2), View(3))),
            };
        }
        match layout {
            Layout::One => Cycling("all", vec![0, 1, 2, 3]),
            Layout::Two => split("h", true, View(0), Cycling("flat", vec![1, 2, 3])),
            // The top view above the two side views.
            Layout::Three => split("h", true, View(0), split("right", false, View(1), Cycling("sides", vec![2, 3]))),
            // `FourPaneMapView`: 3D over YZ on the left, XY over XZ on the right, so X runs
            // across the screen in both right-hand views.
            Layout::Four => split("h", true, split("left", false, View(0), View(3)), split("right", false, View(1), View(2))),
        }
    }

    /// Where each view goes in `area` this frame, by view number. Draws the splitters and
    /// lets them be dragged.
    pub fn arrange(&mut self, ui: &egui::Ui, area: egui::Rect, layout: Layout) -> Vec<(usize, egui::Rect)> {
        let mut out = Vec::new();
        // A maximised view is a state of an arrangement, not of the views: a new layout,
        // or the 3D view leaving or coming back, starts without one.
        if self.built.replace((layout, self.detached)).is_some_and(|built| built != (layout, self.detached)) {
            self.maximised = None;
        }
        if let Some(view) = self.maximised {
            out.push((view, area));
            return out;
        }
        let tree = self.tree(layout);
        // The two columns of the four-pane layout keep their rows level: dragging one
        // splitter moves both.
        if layout == Layout::Four && !self.detached {
            let right = *self.splits.get("right").unwrap_or(&0.5);
            self.splits.insert("left", right);
        }
        self.place(ui, &tree, area, &mut out);
        out
    }

    fn place(&mut self, ui: &egui::Ui, node: &Node, area: egui::Rect, out: &mut Vec<(usize, egui::Rect)>) {
        match node {
            Node::View(view) => out.push((*view, area)),
            Node::Cycling(id, views) => {
                let at = *self.cycles.get(id).unwrap_or(&0) % views.len();
                out.push((views[at], area));
            }
            Node::Split { id, horizontal, first, second } => {
                let line = 1.0;
                let fraction = *self.splits.get(id).unwrap_or(&0.5);
                let (a, bar, b) = if *horizontal {
                    let x = area.left() + ((area.width() - line) * fraction).round();
                    (
                        egui::Rect::from_min_max(area.min, egui::pos2(x, area.bottom())),
                        egui::Rect::from_min_max(egui::pos2(x, area.top()), egui::pos2(x + line, area.bottom())),
                        egui::Rect::from_min_max(egui::pos2(x + line, area.top()), area.max),
                    )
                } else {
                    let y = area.top() + ((area.height() - line) * fraction).round();
                    (
                        egui::Rect::from_min_max(area.min, egui::pos2(area.right(), y)),
                        egui::Rect::from_min_max(egui::pos2(area.left(), y), egui::pos2(area.right(), y + line)),
                        egui::Rect::from_min_max(egui::pos2(area.left(), y + line), area.max),
                    )
                };
                self.place(ui, first, a, out);
                self.place(ui, second, b, out);
                if let Some(pointer) = splitter(ui, id, bar, *horizontal) {
                    let fraction = if *horizontal {
                        (pointer.x - area.left()) / area.width()
                    } else {
                        (pointer.y - area.top()) / area.height()
                    };
                    let fraction = fraction.clamp(0.1, 0.9);
                    self.splits.insert(id, fraction);
                    // Level rows: the other column follows.
                    if *id == "left" {
                        self.splits.insert("right", fraction);
                    }
                }
            }
        }
    }

    /// Shows the next view in the pane the keyboard is in. A pane that holds one view has
    /// nothing to cycle, which is why the four-pane layout ignores this.
    pub fn cycle(&mut self, layout: Layout) {
        // The 3D view in its own window has no pane here to cycle.
        if self.detached && self.focused == Some(0) {
            return;
        }
        let tree = self.tree(layout);
        let mut panes = Vec::new();
        collect_cycling(&tree, &mut panes);
        let focused = self.focused;
        let pane = panes
            .iter()
            .find(|(_, views)| focused.is_some_and(|f| views.contains(&f)))
            .or_else(|| panes.first());
        if let Some((id, views)) = pane {
            let next = (*self.cycles.get(id).unwrap_or(&0) + 1) % views.len();
            self.cycles.insert(id, next);
            self.focused = Some(views[next]);
        }
    }

    /// Shows one view on its own, or puts them back if that one already is.
    pub fn toggle_maximised(&mut self) {
        self.maximised = match (self.maximised, self.focused) {
            (Some(_), _) | (None, None) => None,
            (None, Some(which)) if self.detached && which == 0 => None,
            (None, Some(which)) => Some(which),
        };
    }
}

fn collect_cycling(node: &Node, out: &mut Vec<(&'static str, Vec<usize>)>) {
    match node {
        Node::View(_) => {}
        Node::Cycling(id, views) => out.push((id, views.clone())),
        Node::Split { first, second, .. } => {
            collect_cycling(first, out);
            collect_cycling(second, out);
        }
    }
}

/// A draggable splitter line. `vertical_line` means it runs top to bottom.
fn splitter(ui: &egui::Ui, id: &str, line: egui::Rect, vertical_line: bool) -> Option<egui::Pos2> {
    let grab = if vertical_line { line.expand2(egui::vec2(4.0, 0.0)) } else { line.expand2(egui::vec2(0.0, 4.0)) };
    let response = ui.interact(grab, ui.id().with(("splitter", id)), egui::Sense::drag());
    let active = response.hovered() || response.dragged();
    ui.painter().rect_filled(line, 0.0, if active { theme::accent() } else { theme::base() });
    if active {
        ui.ctx().set_cursor_icon(if vertical_line { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
    }
    response.dragged().then(|| response.interact_pointer_pos()).flatten()
}

/// A view's name as a small pill in its top-left corner, laid over the name the shared
/// drawing code writes there; the view with the keyboard has its name brighter. Returns
/// the pill.
pub fn view_label(ui: &egui::Ui, rect: egui::Rect, view: usize, focused: bool) -> egui::Rect {
    let name = editor_core::i18n::tr(match view {
        0 => "3D view",
        1 => "Top",
        2 => "Front",
        _ => "Side",
    });
    let font = egui::FontId::proportional(theme::text_size() - 2.0);
    let width = ui.fonts_mut(|f| f.layout_no_wrap(name.to_string(), font.clone(), theme::fg()).size().x);
    // Wide enough to cover the 12px name the 2D drawing puts at (8, 18).
    let pill = egui::Rect::from_min_size(rect.min + egui::vec2(5.0, 5.0), egui::vec2(width + 16.0, 18.0));
    let painter = ui.painter_at(rect);
    painter.rect_filled(pill, egui::CornerRadius::same(9), theme::base());
    painter.rect_stroke(pill, egui::CornerRadius::same(9), egui::Stroke::new(1.0, if focused { theme::accent() } else { theme::border() }), egui::StrokeKind::Inside);
    crate::theme::text(&painter, pill.center(), egui::Align2::CENTER_CENTER, name, font, if focused { theme::fg() } else { theme::dim() });
    pill
}
