//! The inspector column beside the views: the Map, Entity, Face and Assets tabs.
//!
//! Each tab delegates to its frontend panels and shared document operations.

use eframe::egui::{self, Pos2, Rect, Sense, Stroke, Ui};

use crate::theme;

/// Which tab is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Map,
    Entity,
    Face,
    Assets,
}

/// The inspector's own state.
#[derive(Default)]
pub struct Inspector {
    /// The tab showing.
    pub tab: Tab,
    observed_face: Option<editor_core::document::FaceRef>,
    open_uv_section: bool,
    trigger: crate::trigger_panel::TriggerPanel,
    /// The Map tab.
    map: crate::map_page::MapPage,
    /// The Entity tab.
    entity: crate::entity_page::EntityPage,
    /// The Face tab's UV editor.
    uv: crate::uv_panel::UvPanel,
    /// The Face tab's material browser.
    pub materials: crate::material_page::MaterialPage,
    /// Face colour and surface controls.
    colour: crate::face_tools::ColourPage,
    /// The Face tab's sweep page.
    sweep: crate::face_tools::SweepPage,
    /// The Face tab's fillet panel.
    fillet: crate::face_tools::FilletPanel,
    /// Face inset, bevel and panel controls.
    bevel: crate::geometry_pages::BevelPage,
    /// The Assets tab.
    assets: crate::assets_page::AssetsPage,
}

impl Inspector {
    fn follow_face_selection(&mut self, document: &editor_core::document::Document) {
        let face = document.single_selected_face();
        if face.is_some() && face != self.observed_face {
            self.tab = Tab::Face;
        }
        self.observed_face = face;
    }

    pub fn uv_keyboard(
        &mut self,
        ui: &egui::Ui,
        document: &std::rc::Rc<std::cell::RefCell<editor_core::document::Document>>,
    ) {
        if self.tab == Tab::Face || self.uv.window_open() {
            self.uv.keyboard_input(ui, document);
        }
    }
    pub fn uv_window_open(&self) -> bool {
        self.uv.window_open()
    }
    pub fn toggle_uv_window(
        &mut self,
        document: &std::rc::Rc<std::cell::RefCell<editor_core::document::Document>>,
    ) {
        self.uv.toggle_window(document);
        if !self.uv.window_open() {
            self.tab = Tab::Face;
            self.open_uv_section = true;
        }
    }
    pub fn show_uv_window(&mut self, ctx: &egui::Context, editor: &crate::editor::Editor) -> bool {
        editor.sync_face_edit_targets();
        self.materials.show_settings_window(ctx, &editor.document);
        let was_open = self.uv.window_open();
        self.uv
            .show_window(ctx, &editor.document, &mut self.materials);
        let returned = was_open && !self.uv.window_open();
        if returned {
            self.tab = Tab::Face;
            self.open_uv_section = true;
        }
        returned
    }
    pub(crate) fn plugin_cards(&mut self) -> &mut crate::plugin_page::Cards {
        &mut self.map.plugin_cards
    }
    pub fn corner_requested(&mut self) -> bool {
        std::mem::take(&mut self.bevel.corner_requested)
    }
    pub fn plugins_requested(&mut self) -> bool {
        std::mem::take(&mut self.map.plugins_requested)
    }

    /// Draws the column into `rect`.
    pub fn show(&mut self, ui: &mut Ui, rect: Rect, editor: &crate::editor::Editor) {
        editor.sync_face_edit_targets();
        let document = &editor.document;
        self.follow_face_selection(&document.borrow());
        let filters = &editor.filters;
        let grid = &editor.grid;
        let handles = &editor.handles;
        ui.painter().rect_filled(rect, 0.0, theme::panel_layer());
        let tabs = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 32.0));
        self.tabs(ui, tabs);

        let body = Rect::from_min_max(
            Pos2::new(rect.left() + 8.0, tabs.bottom() + 8.0),
            rect.max - egui::vec2(4.0, 0.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        // Assets owns a scrollable grid that fills this bounded panel.
        if self.tab == Tab::Assets {
            child.set_width(body.width() - 12.0);
            self.assets.show(&mut child, document, grid);
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.set_width(body.width() - 12.0);
                match self.tab {
                    Tab::Map => self.map.show(ui, document, filters),
                    // The trigger panel heads the tab, above the properties.
                    Tab::Entity => {
                        self.trigger.show(ui, editor);
                        self.entity.show(ui, document, grid);
                    }
                    Tab::Face => {
                        use crate::widgets::section;
                        use editor_core::i18n::tr;
                        if std::mem::take(&mut self.open_uv_section) {
                            crate::widgets::open_section(ui, "face-uv");
                        }
                        let open = self.uv.window_open();
                        let mut detach = false;
                        crate::widgets::section_with_action(
                            ui,
                            "face-uv",
                            tr("UV"),
                            true,
                            |ui| {
                                detach = crate::uv_panel::UvPanel::window_button(ui, open);
                            },
                            |ui| self.uv.inspector(ui, document),
                        );
                        if detach {
                            self.toggle_uv_window(document);
                            ui.ctx().request_repaint();
                        }
                        self.uv.material_browser(ui, document, &mut self.materials);
                        section(
                            ui,
                            "face-uv-attributes",
                            tr("Face attributes"),
                            true,
                            |ui| self.uv.controls(ui, document),
                        );
                        section(ui, "face-colour", tr("Colour"), true, |ui| {
                            self.colour.show(ui, editor)
                        });
                        section(ui, "face-sweep", tr("Sweep"), true, |ui| {
                            self.sweep.show(ui, editor)
                        });
                        section(ui, "face-fillet", tr("Fillet"), true, |ui| {
                            ui.push_id("inspector-fillet", |ui| self.fillet.show(ui, editor))
                        });
                        section(ui, "face-patch", tr("Patch"), true, |ui| {
                            crate::geometry_pages::patch(ui, &mut document.borrow_mut())
                        });
                        section(ui, "face-bevel", tr("Inset & bevel"), true, |ui| {
                            self.bevel
                                .show(ui, &mut document.borrow_mut(), &handles.borrow())
                        });
                    }
                    Tab::Assets => self.assets.show(ui, document, grid),
                }
            });
    }

    /// Four compact tabs along the column's top, the current one marked by an accent
    /// underline and brighter text.
    fn tabs(&mut self, ui: &mut Ui, rect: Rect) {
        use editor_core::i18n::tr;
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::panel_base());
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::border()),
        );
        let all = [
            (Tab::Map, tr("Map")),
            (Tab::Entity, tr("Entity")),
            (Tab::Face, tr("Face")),
            (Tab::Assets, tr("Assets")),
        ];
        let mut x = rect.left() + 6.0;
        let mut responses = Vec::with_capacity(all.len());
        for (index, (tab, label)) in all.into_iter().enumerate() {
            let width = crate::chrome::text_width(ui, label, theme::body()) + 22.0;
            let slot =
                Rect::from_min_size(Pos2::new(x, rect.top()), egui::vec2(width, rect.height()));
            x += width;
            let response = ui.interact(slot, ui.scope_id().with(("tab", index)), Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::Tab, true, self.tab == tab, label)
            });
            if response.clicked() {
                self.tab = tab;
            }
            responses.push((tab, response.clone()));
            let painter = ui.painter();
            let current = self.tab == tab;
            if current {
                painter.rect_filled(
                    slot.shrink2(egui::vec2(0.0, 0.0))
                        .with_min_y(rect.top() + 4.0),
                    egui::CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    theme::bg(),
                );
                let line = Rect::from_min_max(
                    Pos2::new(slot.left() + 6.0, slot.bottom() - 2.0),
                    Pos2::new(slot.right() - 6.0, slot.bottom()),
                );
                painter.rect_filled(line, 1.0, theme::accent());
            } else if response.hovered() {
                painter.rect_filled(
                    slot.with_min_y(rect.top() + 4.0),
                    egui::CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    theme::row_hover(),
                );
            }
            let colour = if current {
                theme::fg()
            } else if response.hovered() {
                theme::fg().gamma_multiply(0.9)
            } else {
                theme::dim()
            };
            let font = if current {
                theme::body_bold()
            } else {
                theme::body()
            };
            crate::theme::text(
                painter,
                slot.center(),
                egui::Align2::CENTER_CENTER,
                label,
                font,
                colour,
            );
            // Over the tab's own fill, or the current tab would hide its ring.
            crate::widgets::row_focus(ui, &response);
        }
        // Left and Right go through the tabs while one has the keyboard.
        let rows: Vec<egui::Response> = responses.iter().map(|(_, r)| r.clone()).collect();
        if let Some(next) = crate::widgets::tab_keys(ui, &rows) {
            self.tab = responses[next].0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::document::{Document, FaceRef};

    #[test]
    fn a_new_single_face_selects_face_tab_without_overriding_later_manual_navigation() {
        let mut d = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        let id = d.selectable_brushes()[0].0;
        let first = FaceRef { node: id, face: 0 };
        let second = FaceRef { node: id, face: 1 };
        let mut inspector = Inspector::default();
        d.select_only(id);
        inspector.follow_face_selection(&d);
        assert_eq!(
            inspector.tab,
            Tab::Map,
            "whole brush is not a single face selection"
        );
        d.select_face_only(first);
        inspector.follow_face_selection(&d);
        assert_eq!(inspector.tab, Tab::Face);
        inspector.tab = Tab::Entity;
        inspector.follow_face_selection(&d);
        assert_eq!(
            inspector.tab,
            Tab::Entity,
            "unchanged selection must not steal the tab back"
        );
        d.select_face_only(second);
        inspector.follow_face_selection(&d);
        assert_eq!(inspector.tab, Tab::Face);
        d.clear_selection();
        inspector.tab = Tab::Assets;
        inspector.follow_face_selection(&d);
        assert_eq!(inspector.tab, Tab::Assets);
        d.select_face_only(second);
        inspector.follow_face_selection(&d);
        assert_eq!(inspector.tab, Tab::Face);
    }

    #[test]
    fn a_single_face_tool_handle_also_selects_face_tab() {
        let mut d = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(-32.0, 32.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        let id = d.selectable_brushes()[0].0;
        d.select_only(id);
        d.set_face_edit_targets(vec![FaceRef { node: id, face: 0 }]);
        let mut inspector = Inspector::default();
        inspector.follow_face_selection(&d);
        assert_eq!(inspector.tab, Tab::Face);
    }
}
