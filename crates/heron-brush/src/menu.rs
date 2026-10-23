//! Flat drop-down menus: the menu bar's menus, the grid and view menus, the detached 3D
//! window's menu and the right-click menu all draw their rows here, so they read the same.
//!
//! A menu is a list of [`Entry`]s. Rows are a fixed height with a column on the left for
//! a check mark or an icon, the label, and the shortcut right-aligned and dimmed; separators are hairlines;
//! a disabled row is greyed and takes no clicks; a submenu opens beside its row on hover.

use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Ui};

use crate::theme;

/// The height of one row.
pub const ROW: f32 = 24.0;
/// The space left of a label, where a check mark or an icon goes.
const CHECK_COLUMN: f32 = 28.0;
/// The space right of a label before its shortcut or submenu arrow.
const SHORTCUT_GAP: f32 = 28.0;
/// The padding at a row's right end.
const RIGHT_PAD: f32 = 10.0;

/// One line of a menu.
#[derive(Clone)]
pub enum Entry<A> {
    /// Something to do. `checked` makes it a toggle that shows its state; `icon` is a
    /// glyph from `egui_phosphor::regular`, or empty for none.
    Item {
        label: String,
        shortcut: String,
        action: A,
        enabled: bool,
        checked: Option<bool>,
        icon: &'static str,
    },
    /// A hairline between groups.
    Separator,
    /// A menu opened beside this row.
    Sub {
        label: String,
        entries: Vec<Entry<A>>,
    },
}

impl<A> Entry<A> {
    /// An enabled item without a shortcut.
    pub fn item(label: impl Into<String>, action: A) -> Self {
        Entry::Item {
            label: label.into(),
            shortcut: String::new(),
            action,
            enabled: true,
            checked: None,
            icon: "",
        }
    }
    /// Gives an item an icon from `egui_phosphor::regular`. A toggle's check mark takes
    /// the same place, so a checked item shows the check instead.
    pub fn icon(mut self, glyph: &'static str) -> Self {
        if let Entry::Item { icon, .. } = &mut self {
            *icon = glyph;
        }
        self
    }
    /// Gives an item a shortcut.
    pub fn keys(mut self, keys: &str) -> Self {
        if let Entry::Item { shortcut, .. } = &mut self {
            *shortcut = keys.to_string();
        }
        self
    }
    /// Enables or disables an item.
    pub fn enabled(mut self, on: bool) -> Self {
        if let Entry::Item { enabled, .. } = &mut self {
            *enabled = on;
        }
        self
    }
    /// Makes an item a toggle showing `on`.
    pub fn checked(mut self, on: bool) -> Self {
        if let Entry::Item { checked, .. } = &mut self {
            *checked = Some(on);
        }
        self
    }
}

/// The menus' frame: a raised surface with a hairline and a soft shadow.
pub fn frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::new()
        .fill(theme::surface())
        .stroke(Stroke::new(1.0, theme::border()))
        .corner_radius(CornerRadius::same(6))
        .shadow(style.visuals.popup_shadow)
        .inner_margin(egui::Margin::same(4))
}

/// The rectangles every menu took this frame, so a press outside all of them closes the
/// menu that is open.
fn rects_id() -> Id {
    Id::unique("heron-menu-rects")
}

/// Forgets last frame's menu rectangles. Call before showing this frame's menus.
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp::<Vec<Rect>>(rects_id(), Vec::new()));
}

fn remember(ctx: &egui::Context, rect: Rect) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<Rect>>(rects_id())
            .push(rect)
    });
}

/// Whether a press this frame landed outside every menu shown this frame and outside
/// `also` (the button that opened it).
pub fn pressed_outside(ctx: &egui::Context, also: Option<Rect>) -> bool {
    let rects = ctx
        .data(|d| d.get_temp::<Vec<Rect>>(rects_id()))
        .unwrap_or_default();
    ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer.press_origin().is_some_and(|p| {
                !rects.iter().any(|r| r.contains(p)) && !also.is_some_and(|r| r.contains(p))
            })
    })
}

/// Shows `entries` in a popup with its top-left at `at`. Returns the action picked.
pub fn popup<A: Clone>(ctx: &egui::Context, id: Id, at: Pos2, entries: &[Entry<A>]) -> Option<A> {
    popup_aligned(ctx, id, at, egui::Align2::LEFT_TOP, entries)
}

/// [`popup`] with the corner of the popup that goes at `at` chosen by `pivot`:
/// `RIGHT_TOP` hangs it from a button at the end of a bar.
pub fn popup_aligned<A: Clone>(
    ctx: &egui::Context,
    id: Id,
    at: Pos2,
    pivot: egui::Align2,
    entries: &[Entry<A>],
) -> Option<A> {
    let mut picked = None;
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .pivot(pivot)
        .fixed_pos(at)
        .constrain(true)
        .show(ctx, |ui| {
            let tall = ui.ctx().content_rect().height() * 0.8;
            frame(ui.style()).show(ui, |ui| {
                if entries.len() > 24 {
                    egui::ScrollArea::vertical()
                        .max_height(tall)
                        .show(ui, |ui| picked = show(ui, id, entries));
                } else {
                    picked = show(ui, id, entries);
                }
            });
        });
    remember(ctx, area.response.rect);
    picked
}

/// The rows of `entries` the keyboard can land on: items that are enabled, and submenus.
fn landings<A>(entries: &[Entry<A>]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| match entry {
            Entry::Item { enabled, .. } => *enabled,
            Entry::Sub { .. } => true,
            Entry::Separator => false,
        })
        .map(|(index, _)| index)
        .collect()
}

/// Where the keyboard is in the menu `id`, as an index into its entries.
fn keyboard_id(id: Id) -> Id {
    Id::unique((id, "keyboard"))
}

/// Puts the keyboard on the first row of the menu `id` showing `entries`, for a menu
/// opened from the keyboard.
pub fn focus_first<A>(ctx: &egui::Context, id: Id, entries: &[Entry<A>]) {
    if let Some(first) = landings(entries).first() {
        ctx.data_mut(|d| d.insert_temp(keyboard_id(id), *first));
    }
}

/// Lays `entries` out as rows in `ui`. Returns the action picked. While no submenu of it
/// is open, Up and Down (Home, End) move a highlight through the rows, wrapping round;
/// Enter or Space runs the highlighted item, and they or Right open a highlighted
/// submenu with its first row highlighted. Moving the pointer drops the highlight.
pub fn show<A: Clone>(ui: &mut Ui, id: Id, entries: &[Entry<A>]) -> Option<A> {
    let width = width(ui, entries);
    ui.spacing_mut().item_spacing.y = 0.0;
    let open_sub = Id::unique((id, "open-sub"));
    let mut picked = None;
    let keyboard = keyboard_id(id);
    let mut current = ui.data(|d| d.get_temp::<usize>(keyboard));
    let mut activate = false;
    let mut open_by_keys = false;
    let stops = landings(entries);
    if ui.data(|d| d.get_temp::<usize>(open_sub)).is_none() && !stops.is_empty() {
        if ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
            current = None;
        }
        let none = egui::Modifiers::NONE;
        let (down, up, home, end, enter, right) = ui.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::Home),
                i.consume_key(none, egui::Key::End),
                i.consume_key(none, egui::Key::Enter) || i.consume_key(none, egui::Key::Space),
                current.is_some_and(|c| matches!(entries.get(c), Some(Entry::Sub { .. })))
                    && i.consume_key(none, egui::Key::ArrowRight),
            )
        });
        let at = current.and_then(|c| stops.iter().position(|s| *s == c));
        let last = stops.len() - 1;
        current = if down {
            Some(stops[at.map_or(0, |a| if a == last { 0 } else { a + 1 })])
        } else if up {
            Some(stops[at.map_or(last, |a| if a == 0 { last } else { a - 1 })])
        } else if home {
            Some(stops[0])
        } else if end {
            Some(stops[last])
        } else {
            current
        };
        activate = enter && current.is_some();
        open_by_keys = (enter || right) && current.is_some();
        match current {
            Some(c) => ui.data_mut(|d| {
                d.insert_temp(keyboard, c);
            }),
            None => ui.data_mut(|d| d.remove::<usize>(keyboard)),
        }
    }
    for (index, entry) in entries.iter().enumerate() {
        match entry {
            Entry::Separator => {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 9.0), Sense::hover());
                ui.painter().hline(
                    rect.x_range().shrink(6.0),
                    rect.center().y,
                    Stroke::new(1.0, theme::border()),
                );
            }
            Entry::Item {
                label,
                shortcut,
                action,
                enabled,
                checked,
                icon,
            } => {
                let lit = current == Some(index);
                let response = row(
                    ui, width, label, shortcut, icon, *enabled, *checked, false, false, lit,
                );
                if response.hovered() {
                    ui.data_mut(|d| d.remove::<usize>(open_sub));
                }
                if *enabled && (response.clicked() || (lit && activate)) {
                    picked = Some(action.clone());
                }
            }
            Entry::Sub { label, entries } => {
                let lit = current == Some(index);
                if lit && open_by_keys {
                    ui.data_mut(|d| d.insert_temp(open_sub, index));
                    focus_first(ui.ctx(), Id::unique((id, "sub", index)), entries);
                }
                let open = ui.data(|d| d.get_temp::<usize>(open_sub)) == Some(index);
                let response = row(ui, width, label, "", "", true, None, true, open, lit);
                if response.hovered() || response.clicked() {
                    ui.data_mut(|d| d.insert_temp(open_sub, index));
                }
                if open || response.hovered() {
                    let at = egui::pos2(response.rect.right() + 6.0, response.rect.top() - 4.0);
                    if let Some(action) =
                        popup(ui.ctx(), Id::unique((id, "sub", index)), at, entries)
                    {
                        picked = Some(action);
                    }
                }
            }
        }
    }
    picked
}

/// The width every row of `entries` takes: the widest label and shortcut, with room for
/// the check column and the gap between them.
fn width<A>(ui: &Ui, entries: &[Entry<A>]) -> f32 {
    let measure = |text: &str| {
        if text.is_empty() {
            0.0
        } else {
            ui.fonts_mut(|f| {
                f.layout_no_wrap(text.to_string(), theme::body(), theme::fg())
                    .size()
                    .x
            })
        }
    };
    let mut widest: f32 = 0.0;
    let mut shortcut: f32 = 0.0;
    for entry in entries {
        match entry {
            Entry::Item {
                label,
                shortcut: keys,
                ..
            } => {
                widest = widest.max(measure(label));
                shortcut = shortcut.max(measure(keys));
            }
            Entry::Sub { label, .. } => {
                widest = widest.max(measure(label));
                shortcut = shortcut.max(10.0);
            }
            Entry::Separator => {}
        }
    }
    let gap = if shortcut > 0.0 { SHORTCUT_GAP } else { 12.0 };
    (CHECK_COLUMN + widest + gap + shortcut + RIGHT_PAD)
        .max(200.0)
        .ceil()
}

/// One row. `arrow` marks a submenu row, `open` one whose submenu is showing, and `lit`
/// the row the keyboard is on, highlighted as the pointer's is.
#[allow(clippy::too_many_arguments)]
fn row(
    ui: &mut Ui,
    width: f32,
    label: &str,
    shortcut: &str,
    icon: &str,
    enabled: bool,
    checked: Option<bool>,
    arrow: bool,
    open: bool,
    lit: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, ROW), sense);
    // WinUI marks the row under the pointer with a subtle fill; the others with the accent.
    let pointed = response.hovered() || lit;
    let hot = enabled && pointed && !theme::win();
    let subtle = enabled && pointed && theme::win();
    let painter = ui.painter();
    if hot {
        painter.rect_filled(rect, CornerRadius::same(4), theme::accent());
    } else if open || subtle {
        painter.rect_filled(rect, CornerRadius::same(4), theme::button_hover());
    }
    let text = if !enabled {
        theme::disabled()
    } else if hot {
        theme::on_accent()
    } else {
        theme::fg()
    };
    let weak = if !enabled {
        theme::disabled()
    } else if hot {
        theme::on_accent().gamma_multiply(0.8)
    } else {
        theme::dim()
    };
    let column = egui::pos2(rect.left() + CHECK_COLUMN / 2.0, rect.center().y);
    if checked == Some(true) {
        check_mark(painter, column, text);
    } else if !icon.is_empty() {
        crate::theme::icon_text(
            painter,
            column,
            egui::Align2::CENTER_CENTER,
            icon,
            theme::icon(),
            weak,
        );
    }
    crate::theme::text(
        painter,
        egui::pos2(rect.left() + CHECK_COLUMN, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::body(),
        text,
    );
    if !shortcut.is_empty() {
        crate::theme::text(
            painter,
            egui::pos2(rect.right() - RIGHT_PAD, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            theme::body(),
            weak,
        );
    }
    if arrow {
        let c = egui::pos2(rect.right() - RIGHT_PAD - 3.0, rect.center().y);
        painter.add(egui::Shape::line(
            vec![
                c + egui::vec2(-3.0, -4.0),
                c + egui::vec2(1.0, 0.0),
                c + egui::vec2(-3.0, 4.0),
            ],
            Stroke::new(1.4, weak),
        ));
    }
    response
}

/// A check mark centred on `c`.
pub fn check_mark(painter: &egui::Painter, c: Pos2, color: Color32) {
    painter.add(egui::Shape::line(
        vec![
            c + egui::vec2(-4.0, 0.0),
            c + egui::vec2(-1.2, 3.0),
            c + egui::vec2(4.0, -3.5),
        ],
        Stroke::new(1.6, color),
    ));
}

/// A key as a menu prints it: the punctuation keys by their character ("Comma" is ",").
pub fn key_label(key: &str) -> &str {
    match key {
        "Comma" => ",",
        "Period" => ".",
        "Minus" => "-",
        "Plus" => "+",
        "Equals" => "=",
        "Slash" => "/",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Quote" => "'",
        "Backtick" => "`",
        "OpenBracket" => "[",
        "CloseBracket" => "]",
        other => other,
    }
}

/// The shortcut text for a command key with `rest`: ⌘ on macOS, Ctrl elsewhere.
pub fn ctrl(rest: &str) -> String {
    let (modifiers, key) = rest.rsplit_once('+').map_or(("", rest), |(m, k)| (m, k));
    let key = if key.is_empty() { "+" } else { key_label(key) };
    if cfg!(target_os = "macos") {
        let alt = if modifiers.contains("Alt") { "⌥" } else { "" };
        let shift = if modifiers.contains("Shift") {
            "⇧"
        } else {
            ""
        };
        format!("{alt}{shift}⌘{key}")
    } else if modifiers.is_empty() {
        format!("Ctrl+{key}")
    } else {
        format!("Ctrl+{modifiers}+{key}")
    }
}
