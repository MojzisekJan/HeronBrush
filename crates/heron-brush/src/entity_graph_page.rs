//! The Entity links window: which entity fires which, as boxes and arrows, the way
//! Hammer's inputs and outputs read. Every entity that has a name others aim at, or aims
//! at one, is a box; every link (`target`, `killtarget` and the FGD's target keys) an
//! arrow from the one that fires to the one fired, labelled with its key.
//!
//! Click a box to select the entity in the map; drag a box to move it; drag from the dot
//! on a box's right edge onto another box to link them (the target is named from its
//! class if it has no name); right-click a box to remove one of its links. Both are
//! undo steps. Drag the empty canvas to pan and use the wheel to zoom. The graph comes
//! from `Document::entity_graph` (document/links.rs).

use std::collections::HashMap;

use editor_core::document::EntityGraph;
use editor_core::i18n::{fill, tr};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};
use map_model::node::NodeId;

use crate::{dialog, editor::Editor, theme};

/// A box's size and the spacing of the layout's columns and rows, in graph units.
const NODE: Vec2 = Vec2::new(176.0, 46.0);
const COLUMN: f32 = 260.0;
const ROW: f32 = 70.0;

/// Each entity's column and row in the graph, as [`EntityGraph::layout`] gives them.
type Layout = HashMap<NodeId, (usize, usize)>;

/// The window's state.
pub struct EntityGraphPage {
    pub open: bool,
    /// Where the graph's origin is on the canvas, and how big a graph unit is.
    offset: Vec2,
    zoom: f32,
    /// Boxes the person moved, by entity, in graph units.
    moved: HashMap<NodeId, Pos2>,
    /// The graph and the map state it was made from.
    cache: Option<(u64, EntityGraph, Layout)>,
    /// The box a link is being dragged from.
    linking: Option<NodeId>,
    status: String,
}

impl Default for EntityGraphPage {
    fn default() -> Self {
        Self {
            open: false,
            offset: Vec2::new(24.0, 24.0),
            zoom: 1.0,
            moved: HashMap::new(),
            cache: None,
            linking: None,
            status: String::new(),
        }
    }
}

impl EntityGraphPage {
    pub fn show(&mut self, ctx: &egui::Context, editor: &Editor) {
        if !self.open {
            return;
        }
        let mut close = false;
        let mut arrange = false;
        let open = dialog::show(
            ctx,
            "entity-graph",
            tr("Entity links"),
            [960.0, 620.0],
            |ui, part| match part {
                dialog::Part::Actions => {
                    close = dialog::button(ui, tr("Close"), true).clicked();
                    arrange = dialog::button(ui, tr("Arrange"), true).clicked();
                    ui.label(egui::RichText::new(&self.status).color(theme::dim()));
                }
                dialog::Part::Body => self.canvas(ui, editor),
            },
        );
        if arrange {
            self.moved.clear();
            self.offset = Vec2::new(24.0, 24.0);
            self.zoom = 1.0;
        }
        self.open = open && !close;
    }

    /// The graph, made again when the map or the selection changed.
    fn graph(&mut self, editor: &Editor) -> (EntityGraph, Layout) {
        let document = editor.document.borrow();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            document.modification_count().hash(&mut hasher);
            let mut ids: Vec<u64> = document.selection_ids().iter().map(|id| id.0).collect();
            ids.sort_unstable();
            ids.hash(&mut hasher);
            hasher.finish()
        };
        if self.cache.as_ref().map(|(made, _, _)| *made) != Some(key) {
            let graph = document.entity_graph();
            let layout = graph.layout();
            self.cache = Some((key, graph, layout));
        }
        let (_, graph, layout) = self.cache.as_ref().expect("made above");
        (graph.clone(), layout.clone())
    }

    fn canvas(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        let (graph, layout) = self.graph(editor);
        let (rect, background) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(4), theme::base());
        if graph.nodes.is_empty() {
            crate::theme::text(
                &painter,
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Nothing in this map names another entity or is named by one."),
                theme::body(),
                theme::dim(),
            );
            return;
        }

        // Pan by dragging the empty canvas, zoom about the pointer with the wheel.
        if background.dragged() && self.linking.is_none() {
            self.offset += background.drag_delta();
        }
        if let Some(pointer) = background.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let before = self.zoom;
                self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.25, 3.0);
                let at = pointer - rect.min - self.offset;
                self.offset -= at * (self.zoom / before - 1.0);
            }
        }

        let place = |id: NodeId, moved: &HashMap<NodeId, Pos2>| -> Pos2 {
            moved.get(&id).copied().unwrap_or_else(|| {
                let (column, row) = layout.get(&id).copied().unwrap_or((0, 0));
                Pos2::new(column as f32 * COLUMN, row as f32 * ROW)
            })
        };
        let to_screen = |p: Pos2, offset: Vec2, zoom: f32| rect.min + offset + p.to_vec2() * zoom;
        let boxes: HashMap<NodeId, Rect> = graph
            .nodes
            .iter()
            .map(|node| {
                let at = to_screen(place(node.id, &self.moved), self.offset, self.zoom);
                (node.id, Rect::from_min_size(at, NODE * self.zoom))
            })
            .collect();

        // The arrows first, under the boxes: from a box's right edge to the left edge of
        // the box it fires, with the key at the middle.
        for edge in &graph.edges {
            let (Some(from), Some(to)) = (boxes.get(&edge.from), boxes.get(&edge.to)) else {
                continue;
            };
            let start = from.right_center();
            let end = to.left_center();
            let colour = link_colour(&edge.key);
            let bend = ((end.x - start.x).abs() * 0.5).max(40.0 * self.zoom);
            let curve = egui::epaint::CubicBezierShape::from_points_stroke(
                [
                    start,
                    start + Vec2::new(bend, 0.0),
                    end - Vec2::new(bend, 0.0),
                    end,
                ],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.5, colour),
            );
            painter.add(curve);
            let tip = 8.0 * self.zoom;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    end,
                    end + Vec2::new(-tip, -tip * 0.5),
                    end + Vec2::new(-tip, tip * 0.5),
                ],
                colour,
                Stroke::NONE,
            ));
            // A plain `target` is what an arrow means; any other key is written by it.
            if edge.key != "target" {
                let middle = curve_middle(start, end);
                painter.text(
                    middle,
                    egui::Align2::CENTER_BOTTOM,
                    &edge.key,
                    egui::FontId::proportional(11.0 * self.zoom.max(0.8)),
                    colour,
                );
            }
        }

        // The boxes.
        let mut clicked = None;
        let mut released_on = None;
        for node in &graph.nodes {
            let area = boxes[&node.id];
            let response = ui.interact(
                area,
                ui.scope_id().with(("graph-node", node.id.0)),
                Sense::click_and_drag(),
            );
            if response.dragged() && self.linking.is_none() {
                let moved = place(node.id, &self.moved) + response.drag_delta() / self.zoom;
                self.moved.insert(node.id, moved);
            }
            if response.clicked() {
                clicked = Some(node.id);
            }
            if response.hovered()
                && ui.input(|i| i.pointer.primary_released())
                && self.linking.is_some_and(|from| from != node.id)
            {
                released_on = Some(node.id);
            }
            let background = if response.hovered() {
                theme::button_hover()
            } else {
                theme::surface()
            };
            let stroke = if node.selected {
                Stroke::new(2.0, theme::accent())
            } else {
                Stroke::new(1.0, theme::border())
            };
            painter.rect(
                area,
                CornerRadius::same(6),
                background,
                stroke,
                egui::StrokeKind::Inside,
            );
            let text_size = 12.0 * self.zoom.max(0.8);
            painter.text(
                area.left_top() + Vec2::new(10.0, 8.0) * self.zoom,
                egui::Align2::LEFT_TOP,
                &node.classname,
                egui::FontId::new(text_size, theme::bold()),
                theme::fg(),
            );
            if let Some(name) = &node.name {
                painter.text(
                    area.left_bottom() + Vec2::new(10.0, -8.0) * self.zoom,
                    egui::Align2::LEFT_BOTTOM,
                    name,
                    egui::FontId::proportional(text_size * 0.92),
                    theme::dim(),
                );
            }

            // The dot a new link is dragged from.
            let dot = area.right_center();
            let dot_response = ui.interact(
                Rect::from_center_size(dot, Vec2::splat(14.0)),
                ui.scope_id().with(("graph-dot", node.id.0)),
                Sense::drag(),
            );
            painter.circle(
                dot,
                5.0,
                if dot_response.hovered() {
                    theme::accent()
                } else {
                    theme::surface()
                },
                Stroke::new(1.5, theme::accent()),
            );
            if dot_response.drag_started() {
                self.linking = Some(node.id);
            }

            // Its links, to remove one.
            response.context_menu(|ui| {
                let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from == node.id).collect();
                if outgoing.is_empty() {
                    ui.label(egui::RichText::new(tr("It fires nothing")).color(theme::dim()));
                }
                for edge in outgoing {
                    if ui
                        .button(fill(
                            "Remove {}",
                            &[&format!("{} → {}", edge.key, edge.value)],
                        ))
                        .clicked()
                    {
                        editor
                            .document
                            .borrow_mut()
                            .unlink_entities(edge.from, &edge.key);
                        (editor.redraw)();
                        ui.close();
                    }
                }
            });
        }

        // A link being dragged: a line from its dot to the pointer, made when it is let go
        // on another box.
        if let Some(from) = self.linking {
            if let (Some(start), Some(pointer)) = (
                boxes.get(&from).map(|b| b.right_center()),
                ui.input(|i| i.pointer.latest_pos()),
            ) {
                painter.line_segment([start, pointer], Stroke::new(1.5, theme::accent()));
            }
            if let Some(to) = released_on {
                self.status = match editor.document.borrow_mut().link_entities(from, to) {
                    Ok(name) => fill("Linked to {}", &[&name]),
                    Err(why) => why,
                };
                (editor.redraw)();
            }
            if ui.input(|i| !i.pointer.primary_down()) {
                self.linking = None;
            }
            ui.ctx().request_repaint();
        }
        if let Some(id) = clicked {
            editor.document.borrow_mut().select_only(id);
            (editor.redraw)();
        }
        painter.text(rect.left_bottom() + Vec2::new(10.0, -8.0), egui::Align2::LEFT_BOTTOM, tr("Drag from a box's dot onto another box to link them. Right-click a box to remove its links."), theme::body(), theme::dim());
    }
}

/// An arrow's colour: the views' link colours, the kill one for a killtarget.
fn link_colour(key: &str) -> Color32 {
    let colour = if key
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .eq_ignore_ascii_case("killtarget")
    {
        editor_core::preferences::pref(|p| p.kill_entity_link_color)
    } else {
        editor_core::preferences::pref(|p| p.entity_link_color)
    };
    let [r, g, b, _] = colour.rgba_f32();
    // The views' default link green is bright for a light canvas: darkened a little here.
    Color32::from_rgb((r * 200.0) as u8, (g * 200.0) as u8, (b * 200.0) as u8)
}

/// Where an arrow's label goes: its middle, a little above.
fn curve_middle(start: Pos2, end: Pos2) -> Pos2 {
    let middle = start + (end - start) * 0.5;
    middle - Vec2::new(0.0, 4.0)
}
