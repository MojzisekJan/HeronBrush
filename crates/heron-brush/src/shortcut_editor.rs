//! Human-readable shortcut controls; the stored JSON never appears in the UI.
use editor_core::{
    custom_shortcuts::{self, Bindings, COMMANDS},
    i18n::tr,
    preferences,
};
use eframe::egui;
pub fn control(ui: &mut egui::Ui, stored: &mut String) {
    let button = ui.button(tr("Edit shortcuts"));
    egui::Popup::menu(&button).width(590.0).show(|ui|{
        let id=ui.id().with("shortcut-draft");
        let mut draft=ui.ctx().data_mut(|data|data.get_temp::<Bindings>(id)).unwrap_or_else(||custom_shortcuts::load(stored).unwrap_or_default());
        ui.label(tr("Global commands also work in text fields. Map commands leave text entry alone."));
        ui.label(tr("Example: Ctrl+Shift+B or F5. Clear a binding to disable it; Reset restores the preset."));
        let preset=preferences::pref(|p|p.keymap);
        egui::ScrollArea::vertical().max_height(420.0).show(ui,|ui|{
            for command in COMMANDS {
                ui.push_id(command.id,|ui|{
                    ui.horizontal(|ui|{
                        ui.add_sized([155.0,24.0],egui::Label::new(tr(command.label)));
                        ui.add_sized([60.0,24.0],egui::Label::new(tr(if command.global{"Global"}else{"Map"})));
                        let mut text=draft.get(command.id).cloned().unwrap_or_else(||custom_shortcuts::default_chord(command,preset).into());
                        if ui.add(egui::TextEdit::singleline(&mut text).desired_width(160.0).margin(crate::widgets::body_entry_margin(ui, 6, 6)).min_size(egui::vec2(0.0, crate::widgets::ROW_HEIGHT))).changed(){draft.insert(command.id.into(),text.trim().into());}
                        if ui.add_enabled(draft.contains_key(command.id),egui::Button::new(tr("Reset"))).clicked(){draft.remove(command.id);}
                    });
                });
            }
        });
        let valid=custom_shortcuts::validate(&draft,preset);
        if let Err(error)=&valid{ui.colored_label(crate::theme::error(),error);}
        let mut closed=false;
        ui.horizontal(|ui|{
            if ui.add_enabled(valid.is_ok(),egui::Button::new(tr("Apply"))).clicked(){
                *stored=serde_json::to_string(&draft).unwrap();ui.ctx().data_mut(|data|data.remove::<Bindings>(id));closed=true;ui.close();
            }
            if ui.button(tr("Cancel")).clicked(){ui.ctx().data_mut(|data|data.remove::<Bindings>(id));closed=true;ui.close();}
        });
        if !closed && ui.is_enabled(){ui.ctx().data_mut(|data|data.insert_temp(id,draft));}
    });
}
