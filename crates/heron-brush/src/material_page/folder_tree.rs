//! Directory hierarchy over the browser's exact-directory visibility preferences.
use editor_core::i18n::tr;
use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Folder {
    path: String,
    direct: bool,
    children: BTreeMap<String, Folder>,
}
impl Folder {
    pub(super) fn from_paths(paths: &[String]) -> Self {
        let mut root = Self::default();
        for path in paths {
            let mut node = &mut root;
            if !path.is_empty() {
                let mut prefix = String::new();
                for name in path.split('/') {
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    prefix.push_str(name);
                    node = node
                        .children
                        .entry(name.to_owned())
                        .or_insert_with(|| Self {
                            path: prefix.clone(),
                            ..Self::default()
                        });
                }
            }
            node.direct = true;
        }
        root
    }
    fn selection(&self, hidden: &BTreeSet<String>) -> (usize, usize) {
        self.children.values().fold(
            (
                usize::from(self.direct && !hidden.contains(&self.path)),
                usize::from(self.direct),
            ),
            |(shown, total), child| {
                let (s, t) = child.selection(hidden);
                (shown + s, total + t)
            },
        )
    }
    fn set_visible(&self, hidden: &mut BTreeSet<String>, visible: bool) {
        if self.direct {
            if visible {
                hidden.remove(&self.path);
            } else {
                hidden.insert(self.path.clone());
            }
        }
        for child in self.children.values() {
            child.set_visible(hidden, visible);
        }
    }
    fn checkbox(&self, ui: &mut egui::Ui, label: &str, hidden: &mut BTreeSet<String>) -> bool {
        let (shown, total) = self.selection(hidden);
        let mut visible = shown == total;
        let response = ui.add(
            egui::Checkbox::new(&mut visible, label).indeterminate(shown > 0 && shown < total),
        );
        let changed = response.changed();
        response.on_hover_text(if self.path.is_empty() {
            tr("Root folder")
        } else {
            &self.path
        });
        if changed {
            self.set_visible(hidden, visible);
        }
        changed
    }
    fn node(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        hidden: &mut BTreeSet<String>,
        depth: usize,
    ) -> bool {
        let mut changed = false;
        if self.children.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                changed |= self.checkbox(ui, name, hidden);
            });
        } else {
            let id = ui.make_persistent_id(("material-directory", &self.path));
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                id,
                depth == 0,
            )
            .show_header(ui, |ui| {
                changed |= self.checkbox(ui, name, hidden);
            })
            .body(|ui| {
                for (name, child) in &self.children {
                    changed |= child.node(ui, name, hidden, depth + 1);
                }
            });
        }
        changed
    }
    pub(super) fn show(&self, ui: &mut egui::Ui, hidden: &mut BTreeSet<String>) -> bool {
        let mut changed = false;
        ui.scope(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            // Textures without a directory remain independent of all top-level folders.
            if self.direct {
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    let mut visible = !hidden.contains("");
                    if ui.checkbox(&mut visible, tr("Root folder")).changed() {
                        if visible {
                            hidden.remove("");
                        } else {
                            hidden.insert(String::new());
                        }
                        changed = true;
                    }
                });
            }
            for (name, child) in &self.children {
                changed |= child.node(ui, name, hidden, 0);
            }
        });
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree() -> Folder {
        Folder::from_paths(
            &[
                "",
                "walls/stone",
                "walls/stone/details",
                "walls/stonework",
                "floors/tile",
            ]
            .map(str::to_owned),
        )
    }
    #[test]
    fn missing_parent_directories_are_visible_as_branches() {
        let root = tree();
        let walls = &root.children["walls"];
        assert!(!walls.direct);
        assert_eq!(walls.children["stone"].path, "walls/stone");
        assert_eq!(
            walls.children["stone"].children["details"].path,
            "walls/stone/details"
        );
        assert_eq!(walls.selection(&BTreeSet::new()), (3, 3));
    }
    #[test]
    fn parent_toggles_descendants_with_mixed_selection_and_preserves_siblings() {
        let root = tree();
        let walls = &root.children["walls"];
        let stone = &walls.children["stone"];
        let mut hidden = BTreeSet::new();
        stone.set_visible(&mut hidden, false);
        assert_eq!(stone.selection(&hidden), (0, 2));
        assert_eq!(walls.selection(&hidden), (1, 3));
        assert!(!hidden.contains("walls/stonework"));
        assert!(!hidden.contains("floors/tile"));
        assert!(!hidden.contains(""));
        walls.set_visible(&mut hidden, true);
        assert!(hidden.is_empty());
        root.set_visible(&mut hidden, false);
        assert_eq!(root.selection(&hidden), (0, 5));
        let saved = serde_json::to_string(&hidden).unwrap();
        assert_eq!(
            hidden,
            serde_json::from_str::<BTreeSet<String>>(&saved).unwrap()
        );
    }
}
