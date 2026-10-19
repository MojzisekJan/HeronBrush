//! The right-click menu in egui: `editor_core::context_menu`'s sections drawn as a popup
//! at the pointer, in the same rows as the menu bar's menus.

use eframe::egui;
use editor_core::context_menu::{Action, Item};

use crate::menu::{self, Entry};

/// A menu open at a point.
pub struct Open {
    /// Where it opened, in screen points.
    pub at: egui::Pos2,
    /// What it offers.
    pub sections: Vec<Vec<Item>>,
    /// The frame it opened in, whose own click must not close it again.
    pub frame: u64,
}

fn entries(items: &[Item]) -> Vec<Entry<Action>> {
    items
        .iter()
        .map(|item| match item {
            Item::Action(label, action) => Entry::item(label.clone(), action.clone()),
            Item::Submenu(label, children) => Entry::Sub { label: label.clone(), entries: entries(children) },
        })
        .collect()
}

/// Draws the menu. Returns the action picked, and whether the menu should close.
pub fn show(ctx: &egui::Context, open: &Open) -> (Option<Action>, bool) {
    let mut all = Vec::new();
    for (index, section) in open.sections.iter().enumerate() {
        if index > 0 {
            all.push(Entry::Separator);
        }
        all.extend(entries(section));
    }
    let picked = menu::popup(ctx, egui::Id::new("map-context-menu"), open.at, &all);
    let opening = ctx.cumulative_frame_nr() == open.frame;
    let dismissed = !opening && (menu::pressed_outside(ctx, None) || ctx.input(|i| i.key_pressed(egui::Key::Escape)));
    let close = picked.is_some() || dismissed;
    (picked, close)
}
