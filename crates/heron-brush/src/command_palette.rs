//! The command palette: every menu command in one list under a search field, opened with
//! F3 or Ctrl+Shift+P, so a command can be found by name and run from the keyboard.

use eframe::egui;

use crate::menu::{Entry, IconTone};
use crate::theme;
use crate::widgets;
use editor_core::i18n::tr;

/// How many matches the list shows at once.
const SHOWN: usize = 12;

/// The palette's state while it is open.
#[derive(Default)]
pub struct Palette {
    /// Whether it is showing.
    pub open: bool,
    /// What has been typed.
    query: String,
    /// Which match is lit, by its place in the list.
    lit: usize,
    /// Whether the field still has to be given the keyboard.
    focus: bool,
}

/// One command the palette offers: where it is in the menus, its name, its shortcut and
/// what it does.
struct Command<A> {
    path: String,
    label: String,
    shortcut: String,
    icon: &'static str,
    icon_tone: Option<IconTone>,
    action: A,
}

/// The enabled items of `entries`, submenus included, each with the menus above it.
fn flatten<A: Clone>(path: &str, entries: &[Entry<A>], out: &mut Vec<Command<A>>) {
    for entry in entries {
        match entry {
            Entry::Item {
                label,
                shortcut,
                action,
                icon,
                icon_tone,
                enabled: true,
                ..
            } => out.push(Command {
                path: path.to_string(),
                label: label.clone(),
                shortcut: shortcut.clone(),
                icon,
                icon_tone: *icon_tone,
                action: action.clone(),
            }),
            Entry::Sub { label, entries, .. } => {
                flatten(&format!("{path} › {label}"), entries, out);
            }
            Entry::Item { .. } | Entry::Separator => {}
        }
    }
}

/// Whether `command` matches `query`: every word of the query is in its name or its
/// menu's, whatever the case.
fn matches<A>(command: &Command<A>, query: &str) -> bool {
    let haystack = format!("{} {}", command.label, command.path).to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

/// Retain the theme's hue while allowing a little of the map to show through.
fn translucent(color: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

impl Palette {
    /// Whether this frame's keys ask for the palette: F3, or Ctrl+Shift+P.
    pub fn requested(ctx: &egui::Context) -> bool {
        ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::F3)
                || i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::P,
                )
        })
    }

    /// Opens the palette empty, with the keyboard in its field.
    pub fn open(&mut self) {
        *self = Self {
            open: true,
            focus: true,
            ..Self::default()
        };
    }

    /// Shows the palette over the top of `screen` while it is open, offering the commands
    /// of `menus`, named by `titles`. Returns the command picked; picking one, Escape or a
    /// click elsewhere closes it.
    #[cfg(test)]
    pub fn show<A: Clone>(
        &mut self,
        ctx: &egui::Context,
        screen: egui::Rect,
        titles: &[&'static str],
        menus: &[Vec<Entry<A>>],
    ) -> Option<A> {
        self.show_with_icons(ctx, screen, titles, menus, |_| None)
    }

    /// Supplies image icons for commands whose existing icon is an SVG rather than a glyph.
    pub fn show_with_icons<A: Clone>(
        &mut self,
        ctx: &egui::Context,
        screen: egui::Rect,
        titles: &[&'static str],
        menus: &[Vec<Entry<A>>],
        image_icon: impl Fn(&A) -> Option<egui::Image<'static>>,
    ) -> Option<A> {
        if !self.open {
            return None;
        }
        let mut commands = Vec::new();
        for (title, entries) in titles.iter().zip(menus) {
            flatten(tr(title), entries, &mut commands);
        }
        let none = egui::Modifiers::NONE;
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::Enter),
                i.consume_key(none, egui::Key::Escape),
            )
        });
        let mut picked = None;
        let width = 520.0_f32.min(screen.width() - 32.0);
        let at = egui::pos2(screen.center().x - width / 2.0, screen.top() + 64.0);
        let area = egui::Area::new(egui::Id::unique("command-palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                // Area remembers its previous size. Give the growing result list the
                // real viewport budget instead of the last query's one-row height.
                ui.set_max_height((screen.bottom() - at.y - 12.0).max(80.0));
                crate::menu::frame(ui.style())
                    .fill(translucent(theme::surface(), 224))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .id(egui::Id::unique("command-palette-field"))
                                .hint_text(tr("Search commands"))
                                .background_color(translucent(theme::base(), 128))
                                .desired_width(f32::INFINITY)
                                .margin(widgets::body_entry_margin(ui, 8, 8))
                                .min_size(egui::vec2(0.0, widgets::row_height(ui))),
                        );
                        let opening = std::mem::take(&mut self.focus);
                        if opening {
                            field.request_focus();
                        }
                        if field.changed() {
                            self.lit = 0;
                        }
                        let found: Vec<&Command<A>> = commands
                            .iter()
                            .filter(|command| matches(command, &self.query))
                            .collect();

                        let count = found.len();
                        if count > 0 {
                            if down {
                                self.lit = (self.lit + 1) % count;
                            }
                            if up {
                                self.lit = (self.lit + count - 1) % count;
                            }
                            self.lit = self.lit.min(count - 1);
                        }
                        picked = enter
                            .then(|| found.get(self.lit))
                            .flatten()
                            .map(|c| c.action.clone());
                        ui.add_space(4.0);
                        if found.is_empty() {
                            ui.label(
                                egui::RichText::new(tr("No matching commands")).color(theme::dim()),
                            );
                        }
                        let row_pitch =
                            widgets::row_height(ui) + 22.0 + ui.spacing().item_spacing.y;
                        let result_height = (row_pitch * count.min(SHOWN) as f32)
                            .min((screen.bottom() - field.rect.bottom() - 16.0).max(0.0));
                        egui::ScrollArea::vertical()
                            .max_height(result_height)
                            .min_scrolled_height(result_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (index, command) in found.iter().enumerate() {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(
                                            ui.available_width(),
                                            widgets::row_height(ui) + 22.0,
                                        ),
                                        egui::Sense::click(),
                                    );
                                    if index == self.lit
                                        && (up || down || opening || field.changed())
                                    {
                                        response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    let lit = index == self.lit || response.hovered();
                                    let painter =
                                        ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                                    if lit {
                                        painter.rect_filled(
                                            rect,
                                            egui::CornerRadius::same(4),
                                            translucent(theme::button_hover(), 112),
                                        );
                                    }
                                    // Reserve the same icon column for every result, including
                                    // commands without a glyph, to keep both text lines aligned.
                                    let text_left = rect.left() + 36.0;
                                    if !command.icon.is_empty() {
                                        theme::icon_text(
                                            &painter,
                                            egui::pos2(rect.left() + 20.0, rect.center().y),
                                            egui::Align2::CENTER_CENTER,
                                            command.icon,
                                            theme::icon(),
                                            command.icon_tone.map_or(theme::fg(), IconTone::color),
                                        );
                                    } else if let Some(image) = image_icon(&command.action) {
                                        image.paint_at(
                                            ui,
                                            egui::Rect::from_center_size(
                                                egui::pos2(rect.left() + 20.0, rect.center().y),
                                                egui::Vec2::splat(20.0),
                                            ),
                                        );
                                    }
                                    theme::text(
                                        &painter,
                                        egui::pos2(text_left, rect.top() + 13.0),
                                        egui::Align2::LEFT_CENTER,
                                        &command.label,
                                        theme::body(),
                                        theme::fg(),
                                    );
                                    // The menu it is in, then its shortcut, at the row's end.
                                    let end = &command.shortcut;
                                    theme::text(
                                        &painter,
                                        egui::pos2(rect.right() - 8.0, rect.bottom() - 11.0),
                                        egui::Align2::RIGHT_CENTER,
                                        end.trim_end(),
                                        theme::body(),
                                        theme::dim(),
                                    );
                                    theme::text(
                                        &painter,
                                        egui::pos2(text_left, rect.bottom() - 11.0),
                                        egui::Align2::LEFT_CENTER,
                                        &command.path,
                                        theme::body(),
                                        theme::dim(),
                                    );
                                    if response.clicked() {
                                        picked = Some(command.action.clone());
                                    }
                                }
                            });
                    });
            });
        let outside = ctx.input(|i| i.pointer.any_pressed())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !area.response.rect.contains(p));
        if picked.is_some() || escape || outside {
            self.open = false;
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    // These tests inspect layout and input without a renderer to upload textures.
    fn run_test_frame(
        ctx: &egui::Context,
        input: egui::RawInput,
        run: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(input, run);
        output.textures_delta.clear();
        output
    }

    use super::*;

    fn command(path: &str, label: &str) -> Command<u8> {
        Command {
            path: path.into(),
            label: label.into(),
            shortcut: String::new(),
            icon: "",
            icon_tone: None,
            action: 0,
        }
    }

    #[test]
    fn every_word_of_the_query_has_to_match_name_or_menu() {
        let save = command("File", "Save as");
        assert!(matches(&save, ""));
        assert!(matches(&save, "save"));
        assert!(matches(&save, "file AS"));
        assert!(!matches(&save, "save open"));
    }

    #[test]
    fn submenus_are_flattened_and_disabled_items_left_out() {
        let entries = vec![
            Entry::item("Undo", 1u8),
            Entry::item("Redo", 2u8).enabled(false),
            Entry::Separator,
            Entry::sub("Grid", vec![Entry::item("Snap", 3u8)]),
        ];
        let mut out = Vec::new();
        flatten("Edit", &entries, &mut out);
        let names: Vec<(&str, &str)> = out
            .iter()
            .map(|c| (c.path.as_str(), c.label.as_str()))
            .collect();
        assert_eq!(names, [("Edit", "Undo"), ("Edit › Grid", "Snap")]);
    }
    #[test]
    fn keyboard_can_reach_commands_beyond_the_first_page() {
        let ctx = egui::Context::default();
        let mut palette = Palette::default();
        palette.open();
        let menus = vec![(0..30u8)
            .map(|i| Entry::item(&format!("Command {i}"), i))
            .collect()];
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
        let run = |palette: &mut Palette, key: Option<egui::Key>| {
            let mut result = None;
            let events = key
                .map(|key| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .into_iter()
                .collect();
            let _ = run_test_frame(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    result = palette.show(ui.ctx(), screen, &["Edit"], &menus);
                },
            );
            result
        };
        run(&mut palette, None);
        for _ in 0..18 {
            run(&mut palette, Some(egui::Key::ArrowDown));
        }
        assert_eq!(run(&mut palette, Some(egui::Key::Enter)), Some(18));
        assert!(!palette.open);
    }
    #[test]
    fn fill_finds_fillet_and_every_toolbar_tool_is_searchable() {
        use crate::app_actions::Action;
        use editor_core::tool::Tool;
        let mut commands = Vec::new();
        flatten("Geometry", &crate::chrome::palette_tools(), &mut commands);
        assert_eq!(commands.len(), Tool::ALL.len());
        for tool in Tool::ALL {
            let command = commands
                .iter()
                .find(|command| command.action == Action::ActivateTool(tool))
                .expect("every toolbar tool is a command");
            assert!(
                matches(command, tool.name()),
                "English tool names remain searchable in translated UIs"
            );
        }
        let fillet = commands
            .iter()
            .find(|command| matches(command, "fill"))
            .expect("fill must find Fillet");
        assert_eq!(fillet.action, Action::ActivateTool(Tool::Fillet));
        assert!(
            !fillet.shortcut.is_empty(),
            "show the configured tool shortcut"
        );
    }
    #[test]
    fn palette_is_translucent_but_command_text_stays_opaque_in_both_schemes() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            theme::use_scheme(&ctx, dark);
            let mut palette = Palette::default();
            palette.open();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
            let menus = vec![vec![Entry::item("Fillet", 1u8)]];
            let mut output = None;
            for frame in 0..3 {
                output = Some(run_test_frame(
                    &ctx,
                    egui::RawInput {
                        screen_rect: Some(screen),
                        time: Some(f64::from(frame)),
                        ..Default::default()
                    },
                    |ui| {
                        palette.show(ui.ctx(), screen, &["Geometry"], &menus);
                    },
                ));
            }
            let shapes = output.expect("rendered palette").shapes;
            fn has_translucent_panel(shape: &egui::Shape) -> bool {
                match shape {
                    egui::Shape::Rect(rect) => {
                        rect.rect.width() > 400.0
                            && rect.rect.height() > 70.0
                            && rect.fill.a() > 0
                            && rect.fill.a() < 255
                    }
                    // A popup frame groups its shadow and background into one shape.
                    egui::Shape::Vec(shapes) => shapes.iter().any(has_translucent_panel),
                    _ => false,
                }
            }
            assert!(
                shapes
                    .iter()
                    .any(|shape| has_translucent_panel(&shape.shape)),
                "palette background should reveal the map in either scheme"
            );
            let text = shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == "Fillet" => Some(text),
                    _ => None,
                })
                .expect("command label");
            assert_eq!(
                text.fallback_color.a(),
                255,
                "readability must not inherit panel transparency"
            );
        }
    }
    #[test]
    fn results_keep_menu_icons_and_accents_in_both_appearances() {
        let glyph = egui_phosphor::regular::PLAY;
        let menus = vec![vec![
            Entry::sub(
                "Build",
                vec![Entry::item("Run", 1u8).icon(glyph).icon_tone(IconTone::Run)],
            ),
            Entry::item("Plain", 2u8),
        ]];
        for dark in [false, true] {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            theme::use_scheme(&ctx, dark);
            let mut palette = Palette::default();
            palette.open();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
            let mut output = None;
            for frame in 0..3 {
                output = Some(run_test_frame(
                    &ctx,
                    egui::RawInput {
                        screen_rect: Some(screen),
                        time: Some(f64::from(frame)),
                        ..Default::default()
                    },
                    |ui| {
                        palette.show(ui.ctx(), screen, &["File"], &menus);
                    },
                ));
            }
            let shapes = output.unwrap().shapes;
            let text = |name: &str| {
                shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == name => Some(text),
                        _ => None,
                    })
                    .unwrap()
            };
            let icon = text(glyph);
            let run = text("Run");
            let plain = text("Plain");
            let path = text("File › Build");
            assert_eq!(icon.fallback_color, IconTone::Run.color());
            assert_eq!(icon.fallback_color.a(), 255);
            assert!(
                icon.pos.x + icon.galley.size().x < run.pos.x,
                "glyph must fit before the label"
            );
            assert_eq!(
                run.pos.x, plain.pos.x,
                "results without icons keep the same text column"
            );
            assert_eq!(
                run.pos.x, path.pos.x,
                "both text lines align after the icon column"
            );
            assert_eq!(
                run.fallback_color,
                theme::fg(),
                "accents color only the icon"
            );
        }
    }

    #[test]
    fn tool_results_draw_the_existing_svg_icon_before_the_label() {
        use crate::app_actions::Action;
        use editor_core::tool::Tool;
        let ctx = egui::Context::default();
        theme::install(&ctx);
        egui_extras::install_image_loaders(&ctx);
        let mut palette = Palette::default();
        palette.open();
        palette.query = "fill".into();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
        let menus = vec![vec![Entry::item(
            "Fillet",
            Action::ActivateTool(Tool::Fillet),
        )]];
        let mut output = None;
        for frame in 0..3 {
            output = Some(run_test_frame(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(f64::from(frame)),
                    ..Default::default()
                },
                |ui| {
                    palette.show_with_icons(
                        ui.ctx(),
                        screen,
                        &["Geometry"],
                        &menus,
                        crate::chrome::palette_image,
                    );
                },
            ));
        }
        let shapes = output.unwrap().shapes;
        let label = shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Fillet" => Some(text),
                _ => None,
            })
            .unwrap();
        let image = shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.brush.as_ref().is_some_and(|brush| {
                        brush.fill_texture_id != egui::TextureId::default()
                    }) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("SVG tool image");
        assert!(image.right() < label.pos.x);
        assert_eq!(image.size(), egui::Vec2::splat(20.0));
    }

    #[test]
    fn results_grow_back_after_a_query_with_only_one_match() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut palette = Palette::default();
        palette.open();
        palette.query = "Fillet".into();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
        let mut entries: Vec<_> = (0..30u8)
            .map(|i| Entry::item(&format!("Tool {i}"), i))
            .collect();
        entries.push(Entry::item("Fillet", 30u8));
        let menus = vec![entries];
        let render = |palette: &mut Palette| {
            run_test_frame(
                &ctx,
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    palette.show(ui.ctx(), screen, &["Geometry"], &menus);
                },
            )
        };
        for _ in 0..3 {
            render(&mut palette);
        }
        palette.query.clear();
        palette.focus = true;
        let mut output = render(&mut palette);
        for _ in 0..3 {
            output = render(&mut palette);
        }
        let visible = output
            .shapes
            .iter()
            .filter(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with("Tool ") => shape
                    .clip_rect
                    .contains(text.pos + text.galley.size() * 0.5),
                _ => false,
            })
            .count();
        assert!(
            visible >= 8,
            "a broad query must show multiple complete results after a narrow one, got {visible}"
        );
    }
}
