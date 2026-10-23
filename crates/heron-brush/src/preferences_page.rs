//! The Preferences window, backed by editor_core's typed settings.
//!
//! A sidebar lists the categories under a search field; the chosen category shows its
//! settings as inspector rows in cards, the label on the left and the control on the
//! right, with a reset button at the end of every row whose value is not the default.
//! A change applies at once and is written to the store.
use std::cell::RefCell;

use crate::{colour_picker, dialog, theme, widgets};
use editor_core::{
    document::Document,
    games::Game,
    i18n::{tr, translation_key},
    preferences::{self, Color, ColorScheme, Keymap, Layout, Preferences},
};
use eframe::egui::{self, Rect, Sense, Stroke};

/// One preference row: the `Preferences` field and its label.
type Row = (&'static str, &'static str);
/// A heading and the rows under it. An empty heading draws no title.
type Section = (&'static str, &'static [Row]);

/// The categories after Overview, in sidebar order, with their headings and labels.
const PAGES: [(&str, &[Section]); 7] = [
    (
        translation_key("Games"),
        &[(
            translation_key("Game"),
            &[
                ("game", translation_key("Game")),
                ("game_path", translation_key("Where it is")),
                ("game_mods", translation_key("Mods")),
                ("collab_nickname", translation_key("Collaboration name")),
            ],
        )],
    ),
    (
        translation_key("View"),
        &[
            (
                translation_key("Window"),
                &[
                    ("interface_scale", translation_key("Interface scale (%)")),
                    ("map_view_layout", translation_key("Layout")),
                    ("color_scheme", translation_key("Colour scheme")),
                    ("language", translation_key("Language")),
                    (
                        "show_simulation_preview_button",
                        translation_key("Show animation preview button"),
                    ),
                ],
            ),
            (
                translation_key("Axes"),
                &[("show_axes", translation_key("Show world axes"))],
            ),
            (
                translation_key("Grid"),
                &[("grid_alpha", translation_key("Grid opacity on faces"))],
            ),
            (
                translation_key("Leak check"),
                &[("leak_check_cell_size", translation_key("Cell size"))],
            ),
            (
                translation_key("Editing"),
                &[
                    (
                        "alignment_lock",
                        translation_key("Keep the mapping when a brush moves"),
                    ),
                    (
                        "uv_lock",
                        translation_key("Keep the mapping when a brush turns"),
                    ),
                ],
            ),
        ],
    ),
    (
        translation_key("Colours"),
        &[
            (
                translation_key("The view"),
                &[
                    ("background_color", translation_key("Background")),
                    ("grid_color_2d", translation_key("Grid")),
                    ("x_axis_color", translation_key("X axis")),
                    ("y_axis_color", translation_key("Y axis")),
                    ("z_axis_color", translation_key("Z axis")),
                    ("portal_file_color", translation_key("Portals")),
                    ("point_file_color", translation_key("Leak trace")),
                    ("soft_map_bounds_color", translation_key("Soft map bounds")),
                ],
            ),
            (
                translation_key("Geometry"),
                &[
                    ("face_color", translation_key("Faces")),
                    ("selected_face_color", translation_key("Selected faces")),
                    ("locked_face_color", translation_key("Locked faces")),
                    ("edge_color", translation_key("Edges")),
                    ("selected_edge_color", translation_key("Selected edges")),
                    ("locked_edge_color", translation_key("Locked edges")),
                    (
                        "undefined_entity_color",
                        translation_key("Entity with no definition"),
                    ),
                    (
                        "selection_bounds_color",
                        translation_key("Selection bounds"),
                    ),
                    (
                        "info_overlay_text_color",
                        translation_key("Info overlay text"),
                    ),
                    (
                        "info_overlay_background_color",
                        translation_key("Info overlay background"),
                    ),
                    (
                        "selected_info_overlay_text_color",
                        translation_key("Picked info overlay text"),
                    ),
                    (
                        "selected_info_overlay_background_color",
                        translation_key("Picked info overlay background"),
                    ),
                    (
                        "weak_info_overlay_background_alpha",
                        translation_key("Size label background opacity"),
                    ),
                    ("entity_link_color", translation_key("Entity link")),
                    (
                        "selected_entity_link_color",
                        translation_key("Entity link near the selection"),
                    ),
                    (
                        "kill_entity_link_color",
                        translation_key("Entity link that removes its target"),
                    ),
                    ("group_color", translation_key("Groups")),
                    ("linked_group_color", translation_key("Linked groups")),
                ],
            ),
            (
                translation_key("Handles"),
                &[
                    ("handle_color", translation_key("Handle")),
                    ("selected_handle_color", translation_key("Selected handle")),
                    ("clip_handle_color", translation_key("Clip handle")),
                    ("clip_face_color", translation_key("Clip plane")),
                    ("resize_handle_color", translation_key("Extrude handle")),
                    ("rotate_handle_color", translation_key("Rotate ring")),
                    ("scale_handle_color", translation_key("Scale handle")),
                    ("shear_outline_color", translation_key("Shear outline")),
                ],
            ),
        ],
    ),
    (
        translation_key("Mouse"),
        &[
            (
                translation_key("Looking around"),
                &[
                    ("camera_look_speed", translation_key("Look speed")),
                    (
                        "invert_horizontal_look",
                        translation_key("Invert horizontal look"),
                    ),
                    (
                        "invert_vertical_look",
                        translation_key("Invert vertical look"),
                    ),
                ],
            ),
            (
                translation_key("Panning"),
                &[
                    ("camera_pan_speed", translation_key("Pan speed")),
                    (
                        "invert_horizontal_pan",
                        translation_key("Invert horizontal pan"),
                    ),
                    (
                        "invert_vertical_pan",
                        translation_key("Invert vertical pan"),
                    ),
                    ("invert_mouse_wheel", translation_key("Invert mouse wheel")),
                ],
            ),
            (
                translation_key("Moving"),
                &[
                    ("camera_move_speed", translation_key("Move speed")),
                    (
                        "camera_enable_alt_move",
                        translation_key("Alt + middle-drag moves the camera"),
                    ),
                    (
                        "camera_alt_move_invert",
                        translation_key("Invert that Alt move"),
                    ),
                    ("camera_fly_move_speed", translation_key("Fly speed")),
                    (
                        "camera_move_in_cursor_dir",
                        translation_key("Move towards the pointer"),
                    ),
                    ("camera_fov", translation_key("Field of view")),
                    ("link_2d_cameras", translation_key("Link 2D views")),
                ],
            ),
            (
                translation_key("Handles"),
                &[
                    (
                        "drag_occluded_vertices_2d",
                        translation_key("Move occluded vertices in 2D"),
                    ),
                    ("handle_radius", translation_key("Handle radius")),
                    (
                        "rotate_handle_radius",
                        translation_key("Rotate ring radius"),
                    ),
                ],
            ),
        ],
    ),
    (
        translation_key("Keyboard"),
        &[(
            translation_key("Shortcuts"),
            &[
                ("keymap", translation_key("Keymap")),
                ("keyboard_bindings", translation_key("Custom shortcuts")),
            ],
        )],
    ),
    (
        translation_key("Renderer"),
        &[
            (
                "",
                &[
                    ("realistic_shading", translation_key("Realistic shading")),
                    ("selection_outline", translation_key("Outline selection")),
                ],
            ),
            (
                translation_key("Light"),
                &[
                    ("sun_color", translation_key("Sun")),
                    ("sun_elevation", translation_key("Sun elevation")),
                    ("sun_azimuth", translation_key("Sun azimuth")),
                    ("sun_intensity", translation_key("Sun brightness")),
                    ("procedural_sky", translation_key("Procedural sky")),
                    ("sky_color", translation_key("Sky")),
                    ("horizon_color", translation_key("Horizon")),
                    ("ground_color", translation_key("Ground")),
                    (
                        "ambient_intensity",
                        translation_key("Sky and ground brightness"),
                    ),
                    ("exposure", translation_key("Exposure")),
                    ("tone_mapper", translation_key("Tone mapping")),
                    ("entity_lights", translation_key("Entity lights")),
                    ("texture_memory_mb", translation_key("Texture memory (MB)")),
                ],
            ),
            (
                translation_key("Shadows"),
                &[
                    ("shadows", translation_key("Cast shadows")),
                    ("shadow_map_size", translation_key("Shadow map size")),
                    ("shadow_bias", translation_key("Shadow bias")),
                    ("shadow_softness", translation_key("Shadow softness")),
                    ("contact_shadows", translation_key("Contact shadows")),
                    (
                        "contact_shadow_distance",
                        translation_key("Contact distance"),
                    ),
                    ("contact_shadow_samples", translation_key("Contact samples")),
                ],
            ),
            (
                translation_key("Ambient occlusion"),
                &[
                    (
                        "ambient_occlusion",
                        translation_key("Darken creases and corners"),
                    ),
                    ("ao_radius", translation_key("Radius")),
                    ("ao_strength", translation_key("Strength")),
                ],
            ),
            (
                translation_key("Reflections"),
                &[
                    ("reflections", translation_key("Reflect what is on screen")),
                    ("reflection_distance", translation_key("Distance")),
                    ("reflection_samples", translation_key("Samples")),
                    ("reflection_strength", translation_key("Strength")),
                ],
            ),
            (
                translation_key("Bloom"),
                &[
                    ("bloom", translation_key("Let bright surfaces bleed")),
                    ("bloom_threshold", translation_key("Threshold")),
                    ("bloom_intensity", translation_key("Intensity")),
                ],
            ),
            (
                translation_key("Surfaces"),
                &[
                    (
                        "default_roughness",
                        translation_key("Roughness with no material"),
                    ),
                    (
                        "default_metalness",
                        translation_key("Metalness with no material"),
                    ),
                    ("accumulate", translation_key("Keep sampling a still view")),
                    ("accumulation_samples", translation_key("Samples")),
                    ("antialiasing", translation_key("Smooth edges")),
                ],
            ),
        ],
    ),
    (
        translation_key("Updates"),
        &[(
            translation_key("Application updates"),
            &[
                ("update_repository", translation_key("GitHub repository")),
                (
                    "update_on_startup",
                    translation_key("Check for updates on startup"),
                ),
                ("update_prereleases", translation_key("Include prereleases")),
            ],
        )],
    ),
];

/// The Overview's cards: the settings most often changed, gathered from the other
/// categories, which the sidebar reaches.
const OVERVIEW: [Section; 4] = [
    (
        translation_key("Appearance"),
        &[
            ("interface_scale", translation_key("Interface scale (%)")),
            ("panel_opacity", translation_key("Panel opacity (%)")),
            ("color_scheme", translation_key("Colour scheme")),
            ("map_view_layout", translation_key("Layout")),
            ("language", translation_key("Language")),
        ],
    ),
    (
        translation_key("Editing"),
        &[
            (
                "drag_occluded_vertices_2d",
                translation_key("Move vertices behind the selected vertex"),
            ),
            (
                "alignment_lock",
                translation_key("Keep the mapping when a brush moves"),
            ),
            (
                "uv_lock",
                translation_key("Keep the mapping when a brush turns"),
            ),
        ],
    ),
    (
        translation_key("View"),
        &[
            ("show_axes", translation_key("Show world axes")),
            ("camera_fov", translation_key("Field of view")),
        ],
    ),
    (
        translation_key("Manual"),
        &[
            ("manual_font_size", translation_key("Manual font size")),
            ("manual_show_contents", translation_key("Show contents")),
        ],
    ),
];

/// Fields edited outside Preferences (the export window and the Assets tab). A search
/// finds them too, so nothing in the shared type is out of reach.
const ELSEWHERE: [Section; 2] = [
    (
        translation_key("Export"),
        &[("export_target", translation_key("Engine"))],
    ),
    (
        translation_key("Assets"),
        &[("asset_library", translation_key("Asset library"))],
    ),
];

/// The sidebar's categories: Overview, then the pages.
const CATEGORIES: usize = PAGES.len() + 1;
/// Where [`search`] files the [`ELSEWHERE`] rows: after the last category.
const ELSEWHERE_PAGE: usize = CATEGORIES;

/// The note under the keymap choice.
const KEYMAP_NOTE: &str =
    "Blender's bindings, as near as a tool-based editor comes to them: G selects \
     and moves, R turns, S scales, E sweeps, Tab and 1/2/3 are the vertex, edge \
     and face tools, K clips, B selects, A selects everything and Alt+A drops it, \
     X deletes, Shift+D duplicates, H hides and Alt+H shows again, Ctrl+Shift+Z \
     redoes. Everything else is the same in both.";

/// The three colour schemes, in the order the chooser lists them.
const COLOR_SCHEMES: [(ColorScheme, &str); 3] = [
    (
        ColorScheme::System,
        translation_key("colour scheme|Follow the desktop"),
    ),
    (ColorScheme::Light, translation_key("colour scheme|Light")),
    (ColorScheme::Dark, translation_key("colour scheme|Dark")),
];

/// The window's size, which fits inside the editor's own 1360x760 window when dialogs are
/// drawn inside it.
const SIZE: [f32; 2] = [940.0, 660.0];
/// The sidebar's width.
const SIDEBAR_WIDTH: f32 = 196.0;
/// The widest a row's control grows.
const CONTROL_MAX: f32 = 280.0;
/// The widest a number grows: a number needs less room than a name.
const SPIN_MAX: f32 = 160.0;
/// The room at a row's end for its reset button, kept whether or not it shows so the
/// controls line up.
const RESET_SLOT: f32 = 24.0;

thread_local! {
    /// The games found when the window opened: reading every configuration on each frame
    /// would be far too slow.
    static GAMES: RefCell<Vec<Game>> = const { RefCell::new(Vec::new()) };
    /// Whether the Overview is being drawn, whose field of view stops at 120.
    static IN_OVERVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What a [`field`] call does with the field it names.
enum Action<'a> {
    /// Draws its control.
    Draw(&'a mut egui::Ui),
    /// Puts it back to its value in the defaults.
    Reset(&'a Preferences),
    /// Says whether it is at its value in the defaults.
    IsDefault(&'a Preferences, &'a mut bool),
}

/// Does `action` to the field `name`. Returns false for a name that is not a field,
/// which the tests use to catch a typo in the tables.
fn field(settings: &mut Preferences, name: &str, action: Action) -> bool {
    macro_rules! fields {
        ($($f:ident),* $(,)?) => {
            match name {
                $(stringify!($f) => {
                    match action {
                        Action::Draw(ui) => settings.$f.control(ui, name),
                        Action::Reset(defaults) => settings.$f = defaults.$f.clone(),
                        Action::IsDefault(defaults, same) => *same = settings.$f.same(&defaults.$f),
                    }
                    true
                })*
                _ => false,
            }
        };
    }
    fields!(
        update_repository,
        update_on_startup,
        update_prereleases,
        manual_font_size,
        manual_show_contents,
        show_simulation_preview_button,
        panel_opacity,
        interface_scale,
        color_scheme,
        game,
        game_path,
        game_mods,
        export_target,
        map_view_layout,
        language,
        keymap,
        keyboard_bindings,
        asset_library,
        collab_nickname,
        show_axes,
        background_color,
        x_axis_color,
        y_axis_color,
        z_axis_color,
        soft_map_bounds_color,
        face_color,
        selected_face_color,
        locked_face_color,
        edge_color,
        selected_edge_color,
        locked_edge_color,
        undefined_entity_color,
        selection_bounds_color,
        info_overlay_text_color,
        info_overlay_background_color,
        selected_info_overlay_text_color,
        selected_info_overlay_background_color,
        weak_info_overlay_background_alpha,
        entity_link_color,
        selected_entity_link_color,
        kill_entity_link_color,
        group_color,
        linked_group_color,
        handle_color,
        selected_handle_color,
        clip_handle_color,
        clip_face_color,
        resize_handle_color,
        rotate_handle_color,
        scale_handle_color,
        shear_outline_color,
        grid_color_2d,
        grid_alpha,
        camera_look_speed,
        invert_horizontal_look,
        invert_vertical_look,
        camera_pan_speed,
        invert_horizontal_pan,
        invert_vertical_pan,
        camera_move_speed,
        camera_enable_alt_move,
        camera_alt_move_invert,
        camera_fly_move_speed,
        camera_fov,
        camera_move_in_cursor_dir,
        invert_mouse_wheel,
        link_2d_cameras,
        drag_occluded_vertices_2d,
        handle_radius,
        rotate_handle_radius,
        point_file_color,
        portal_file_color,
        leak_check_cell_size,
        alignment_lock,
        uv_lock,
        realistic_shading,
        selection_outline,
        shadows,
        shadow_map_size,
        shadow_bias,
        shadow_softness,
        contact_shadows,
        contact_shadow_distance,
        contact_shadow_samples,
        ambient_occlusion,
        ao_radius,
        ao_strength,
        reflections,
        reflection_distance,
        reflection_samples,
        reflection_strength,
        bloom,
        bloom_threshold,
        bloom_intensity,
        exposure,
        tone_mapper,
        entity_lights,
        texture_memory_mb,
        accumulate,
        accumulation_samples,
        antialiasing,
        procedural_sky,
        sky_color,
        horizon_color,
        ground_color,
        sun_elevation,
        sun_azimuth,
        sun_color,
        sun_intensity,
        ambient_intensity,
        default_roughness,
        default_metalness,
    )
}

/// The defaults as the current colour scheme has them: the view colours that follow the
/// scheme are on its side, so they do not read as changed in the light scheme.
fn scheme_defaults() -> Preferences {
    let saved = preferences::current();
    preferences::set_current(Preferences::default());
    preferences::follow_color_scheme(theme::is_dark());
    let defaults = preferences::current();
    preferences::set_current(saved);
    defaults
}

/// Takes the tags and face-flag names of `game` and reloads the entity models after a
/// change: the game is what names the tags, and its path is where a class's model is
/// looked for. With no game the tags stay as they are.
pub fn apply_game(document: &mut Document, game: Option<&Game>) {
    if let Some(game) = game {
        document.set_tags(game.brush_tags.clone(), game.face_tags.clone());
        document.set_face_flag_names(
            game.surface_flag_names.clone(),
            game.content_flag_names.clone(),
        );
    }
    document.load_entity_models();
}

/// One row a search found, with the category and heading it is under.
#[derive(Debug, Clone, Copy)]
struct Hit {
    page: usize,
    heading: &'static str,
    row: Row,
}

#[derive(Default)]
pub struct PreferencesPage {
    pub open: bool,
    query: String,
    /// The category in front: 0 is Overview, then the pages.
    page: usize,
    /// Why the store could not be written, after a change that still applies.
    error: String,
    /// Whether the window was open last frame, so the games are read once per opening.
    was_open: bool,
    /// The fields drawn last frame, for the tests.
    drawn: Vec<&'static str>,
    /// The settings as they were before the last Restore, and until when its Undo is
    /// offered.
    restored: Option<(Preferences, f64)>,
}

/// How long the Undo after a Restore stays, in seconds.
const UNDO_RESTORE: f64 = 10.0;
impl PreferencesPage {
    /// Shows the window while it is open. Returns true when the game or its path changed,
    /// which the caller passes on to the open map with [`apply_game`].
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.open {
            self.was_open = false;
            return false;
        }
        if !self.was_open {
            self.was_open = true;
            GAMES.with(|games| *games.borrow_mut() = editor_core::games::all());
        }
        let mut settings = preferences::current();
        let before = settings.clone();
        let defaults = scheme_defaults();
        let mut close = false;
        let mut restore = false;
        let mut undo = false;
        let now = ctx.input(|i| i.time);
        if self
            .restored
            .as_ref()
            .is_some_and(|(_, until)| now > *until)
        {
            self.restored = None;
        }
        // The button says how much it puts back: the page in front, or what a search
        // shows.
        let restore_label = if searching(&self.query) {
            tr("Restore shown settings")
        } else {
            tr("Restore page defaults")
        };
        let offer_undo = self.restored.is_some();
        let embedded = ctx.embed_viewports();
        self.drawn.clear();
        self.open = dialog::show(ctx, "preferences", tr("Preferences"), SIZE, |ui, part| {
            if part == dialog::Part::Actions {
                close = dialog::button(ui, tr("Close"), true).clicked();
                // The rest of the strip from the left: the button puts back only the
                // category in front of the user, or the rows a search shows.
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    restore = dialog::button(ui, restore_label, true).clicked();
                    if offer_undo {
                        undo = dialog::button(ui, tr("Undo"), true).clicked();
                    }
                    if !self.error.is_empty() {
                        ui.add_space(8.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&self.error).color(theme::error()),
                            )
                            .truncate(),
                        );
                    }
                });
                return;
            }
            self.body(ui, &mut settings, &defaults, embedded);
        }) && !close;
        if restore {
            let earlier = settings.clone();
            for name in self.visible_fields() {
                field(&mut settings, name, Action::Reset(&defaults));
            }
            if earlier != settings {
                self.restored = Some((earlier, now + UNDO_RESTORE));
            }
        } else if undo {
            if let Some((earlier, _)) = self.restored.take() {
                settings = earlier;
            }
        }
        if offer_undo {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        if before == settings {
            return false;
        }
        ctx.request_repaint();
        self.commit(&before, settings, &Preferences::default_path())
    }

    /// Puts `settings` in force and writes them to `path`. A store that cannot be written
    /// does not hold the change back: it applies for this session and the
    /// window says why it will not outlast it. Returns whether the game or its path
    /// changed.
    fn commit(
        &mut self,
        before: &Preferences,
        settings: Preferences,
        path: &std::path::Path,
    ) -> bool {
        let game_changed = before.game != settings.game
            || before.game_path != settings.game_path
            || before.game_mods != settings.game_mods;
        preferences::set_current(settings);
        // A view colour put back to its default lands on the scheme's side of it.
        preferences::follow_color_scheme(theme::is_dark());
        match preferences::current().save(path) {
            Ok(()) => self.error.clear(),
            Err(error) => {
                self.error = format!(
                    "the preferences could not be written to {}: {error}",
                    path.display()
                );
                editor_core::complain!("{}", self.error);
            }
        }
        game_changed
    }

    /// The fields the current category or search shows, which Restore defaults puts back.
    fn visible_fields(&self) -> Vec<&'static str> {
        if searching(&self.query) {
            search(&self.query)
                .into_iter()
                .map(|hit| hit.row.0)
                .collect()
        } else {
            sections(self.page)
                .iter()
                .flat_map(|(_, rows)| rows.iter().map(|(name, _)| *name))
                .collect()
        }
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        settings: &mut Preferences,
        defaults: &Preferences,
        embedded: bool,
    ) {
        // `dialog::show` already keeps the button strip's room out of the body.
        let full = crate::form::body_rect(ui, embedded);
        let height = full.height();
        let side = Rect::from_min_size(
            full.min,
            egui::vec2(SIDEBAR_WIDTH.min(full.width() * 0.4), height),
        );
        let content = Rect::from_min_max(egui::pos2(side.right() + 12.0, full.top()), full.max);
        let hits = if searching(&self.query) {
            search(&self.query)
        } else {
            Vec::new()
        };
        let layout = egui::Layout::top_down(egui::Align::Min);
        let mut side_ui = ui.new_child(egui::UiBuilder::new().max_rect(side).layout(layout));
        self.sidebar(&mut side_ui, &hits);
        let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content).layout(layout));
        content_ui.set_clip_rect(content.intersect(ui.clip_rect()));
        self.content(&mut content_ui, settings, defaults, &hits);
    }

    /// The search field over the list of categories. While searching, each category
    /// says how many of its rows match.
    fn sidebar(&mut self, ui: &mut egui::Ui, hits: &[Hit]) {
        let rect = ui.max_rect();
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(6),
            theme::surface(),
            Stroke::new(1.0, theme::border()),
            egui::StrokeKind::Inside,
        );
        let inner = rect.shrink(6.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            let edit = egui::TextEdit::singleline(&mut self.query)
                .id(egui::Id::unique("preferences-search"))
                .hint_text(tr("Search by name or description"))
                .desired_width(f32::INFINITY)
                .margin(crate::widgets::body_entry_margin(ui, 26, 6))
                .min_size(egui::vec2(0.0, widgets::ROW_HEIGHT));
            let response = ui.add(edit);
            magnifier(ui.painter(), response.rect, theme::dim());
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.query.clear();
            }
            ui.add_space(6.0);
            let searching = searching(&self.query);
            let mut rows = Vec::with_capacity(CATEGORIES);
            for index in 0..CATEGORIES {
                let count = hits.iter().filter(|hit| hit.page == index).count();
                let selected = !searching && self.page == index;
                let badge = searching.then_some(count);
                let row = category(ui, tr(category_title(index)), selected, badge);
                if row.clicked() {
                    self.page = index;
                    self.query.clear();
                }
                rows.push(row);
            }
            // Up and Down go through the categories while one has the keyboard.
            if let Some(next) = widgets::list_keys(ui, &rows) {
                self.page = next;
                self.query.clear();
            }
        });
    }

    fn content(
        &mut self,
        ui: &mut egui::Ui,
        settings: &mut Preferences,
        defaults: &Preferences,
        hits: &[Hit],
    ) {
        let salt = if hits.is_empty() && !searching(&self.query) {
            self.page
        } else {
            usize::MAX
        };
        egui::ScrollArea::vertical()
            .id_salt(("preferences-content", salt))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Clear of the scroll bar, which floats over the right edge.
                ui.set_max_width(ui.available_width() - 10.0);
                if searching(&self.query) {
                    if hits.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(tr("No matching settings")).color(theme::dim()),
                            )
                        });
                        return;
                    }
                    let mut start = 0;
                    while start < hits.len() {
                        let (page, heading) = (hits[start].page, hits[start].heading);
                        let end = hits[start..]
                            .iter()
                            .position(|hit| hit.page != page || hit.heading != heading)
                            .map_or(hits.len(), |n| start + n);
                        let rows: Vec<Row> = hits[start..end].iter().map(|hit| hit.row).collect();
                        let title = match (page, heading) {
                            (ELSEWHERE_PAGE, heading) => tr(heading).to_string(),
                            (page, "") => tr(category_title(page)).to_string(),
                            (page, heading) => {
                                format!("{}  ›  {}", tr(category_title(page)), tr(heading))
                            }
                        };
                        self.card(ui, &title, &rows, settings, defaults);
                        start = end;
                    }
                    return;
                }
                let title = tr(category_title(self.page));
                ui.label(
                    egui::RichText::new(title)
                        .font(egui::FontId::new(theme::text_size() + 4.0, theme::bold()))
                        .color(theme::fg()),
                );
                ui.add_space(8.0);
                IN_OVERVIEW.set(self.page == 0);
                for (heading, rows) in sections(self.page) {
                    self.card(ui, tr(heading), rows, settings, defaults);
                }
                IN_OVERVIEW.set(false);
            });
    }

    /// A card of rows, with notes under some of them.
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        rows: &[Row],
        settings: &mut Preferences,
        defaults: &Preferences,
    ) {
        widgets::card(ui, title, None, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (name, label) in rows.iter().filter(|(name, _)| offered(name)) {
                setting_row(ui, settings, defaults, name, label);
                self.drawn.push(name);
                notes(ui, name);
            }
        });
        ui.add_space(10.0);
    }
}

/// Whether this build has a use for the field: the tone map, entity lights and the
/// texture budget are the wgpu renderer's alone, so a GL build does not show a control that would do nothing.
fn offered(name: &str) -> bool {
    !matches!(name, "tone_mapper" | "entity_lights" | "texture_memory_mb") || cfg!(feature = "wgpu")
}

/// Whether `query` asks for a search rather than a category.
fn searching(query: &str) -> bool {
    !query.trim().is_empty()
}

/// A category's sections: the Overview's cards, or a page's.
fn sections(page: usize) -> &'static [Section] {
    match page {
        0 => &OVERVIEW,
        ELSEWHERE_PAGE => &ELSEWHERE,
        n => PAGES[n - 1].1,
    }
}

/// A category's title, untranslated.
fn category_title(page: usize) -> &'static str {
    match page {
        0 => translation_key("Overview"),
        ELSEWHERE_PAGE => "",
        n => PAGES[n - 1].0,
    }
}

/// The rows whose label (English or translated), field name or heading contains
/// `query`, in sidebar order. A field that two categories show is listed once, under its
/// own page rather than the Overview.
fn search(query: &str) -> Vec<Hit> {
    let query = query.trim().to_lowercase();
    let order = (1..CATEGORIES).chain([0, ELSEWHERE_PAGE]);
    let mut found: Vec<Hit> = vec![];
    for page in order {
        for (heading, rows) in sections(page) {
            let heading_hit = !heading.is_empty()
                && (heading.to_lowercase().contains(&query)
                    || tr(heading).to_lowercase().contains(&query));
            for &(name, label) in rows.iter().filter(|(name, _)| offered(name)) {
                let hit = heading_hit
                    || label.to_lowercase().contains(&query)
                    || tr(label).to_lowercase().contains(&query)
                    || name.replace('_', " ").contains(&query);
                if hit && !found.iter().any(|seen| seen.row.0 == name) {
                    found.push(Hit {
                        page,
                        heading,
                        row: (name, label),
                    });
                }
            }
        }
    }
    found.sort_by_key(|hit| hit.page);
    found
}

/// One setting: its label, its control, and a reset button while it is not the default.
fn setting_row(
    ui: &mut egui::Ui,
    settings: &mut Preferences,
    defaults: &Preferences,
    name: &'static str,
    label: &'static str,
) {
    ui.push_id(name, |ui| {
        widgets::row(ui, tr(label), None, |ui| {
            // Every row's control area is as wide, so the reset buttons after them line up.
            let width = (ui.available_width() - RESET_SLOT).clamp(40.0, CONTROL_MAX);
            let area = Rect::from_min_size(ui.cursor().min, egui::vec2(width, widgets::ROW_HEIGHT));
            let layout = egui::Layout::left_to_right(egui::Align::Center);
            ui.scope_builder(egui::UiBuilder::new().max_rect(area).layout(layout), |ui| {
                field(settings, name, Action::Draw(ui));
            });
            ui.advance_cursor_after_rect(area);
            let mut same = true;
            field(settings, name, Action::IsDefault(defaults, &mut same));
            if !same && reset_button(ui).clicked() {
                field(settings, name, Action::Reset(defaults));
            }
        });
    });
}

/// The lines under a row: which game configuration is in force and what a game
/// decides, under the game's path; what the Blender keymap means, under the keymap.
fn notes(ui: &mut egui::Ui, name: &str) {
    let note = |ui: &mut egui::Ui, text: &str| {
        ui.add(egui::Label::new(egui::RichText::new(text).color(theme::dim())).wrap());
    };
    match name {
        "game_path" => {
            let wanted = preferences::pref(|p| p.game.clone());
            let directory = GAMES.with(|games| {
                games
                    .borrow()
                    .iter()
                    .find(|game| game.name == wanted)
                    .map(|game| game.directory.display().to_string())
            });
            ui.add_space(4.0);
            if let Some(directory) = directory {
                note(ui, &directory);
            }
            note(
                ui,
                &format!("{} / {}", tr("Entity definitions"), tr("Materials")),
            );
            ui.add_space(6.0);
        }
        "game_mods" => {
            let paths = GAMES.with(|games| {
                let wanted = preferences::pref(|p| p.game.clone());
                games
                    .borrow()
                    .iter()
                    .find(|game| game.name == wanted)
                    .map(|game| {
                        preferences::pref(|p| {
                            game.asset_roots(std::path::Path::new(&p.game_path), &p.game_mods)
                        })
                    })
                    .unwrap_or_default()
            });
            ui.add_space(4.0);
            note(ui, tr("Project paths"));
            for (i, path) in paths.iter().rev().enumerate() {
                note(ui, &format!("{}. {}", i + 1, path.display()));
            }
            ui.add_space(6.0);
        }
        "keymap" => {
            ui.add_space(4.0);
            note(ui, KEYMAP_NOTE);
            ui.add_space(2.0);
        }
        _ => {}
    }
}

/// A category in the sidebar: the current one carries the accent, and while searching a
/// count of its matching rows sits at its end.
fn category(
    ui: &mut egui::Ui,
    title: &str,
    selected: bool,
    badge: Option<usize>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), widgets::ROW_HEIGHT),
        Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, selected, title)
    });
    widgets::row_focus(ui, &response);
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(theme::radius());
    // On macOS the current one is filled with the accent, as System Settings' sidebar is.
    let filled = selected && theme::mac();
    if filled {
        painter.rect_filled(rect, radius, theme::accent());
    } else if selected {
        // WinUI's navigation keeps the current item's fill neutral; the accent is the bar.
        let fill = if theme::win() {
            theme::button_hover()
        } else {
            theme::accent().gamma_multiply(0.22)
        };
        painter.rect_filled(rect, radius, fill);
        let bar = Rect::from_min_size(
            rect.min + egui::vec2(0.0, 6.0),
            egui::vec2(3.0, rect.height() - 12.0),
        );
        painter.rect_filled(bar, egui::CornerRadius::same(2), theme::accent());
    } else if response.hovered() {
        painter.rect_filled(rect, radius, theme::button_hover());
    }
    let empty = badge == Some(0);
    let color = if empty {
        theme::disabled()
    } else if filled {
        theme::on_accent()
    } else if selected || response.hovered() {
        theme::fg()
    } else {
        theme::dim()
    };
    let font = if selected {
        theme::body_bold()
    } else {
        theme::body()
    };
    let galley = painter.layout_no_wrap(title.to_string(), font, color);
    painter.galley(
        egui::pos2(
            rect.min.x + 12.0,
            crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
        ),
        galley,
        color,
    );
    if let Some(count) = badge.filter(|count| *count > 0) {
        let weak = if filled {
            egui::Color32::from_white_alpha(200)
        } else {
            theme::dim()
        };
        let galley = painter.layout_no_wrap(count.to_string(), theme::body(), weak);
        painter.galley(
            egui::pos2(
                rect.max.x - 10.0 - galley.size().x,
                crate::theme::galley_top(painter.ctx(), &galley, rect.center().y),
            ),
            galley,
            weak,
        );
    }
    response
}

/// A magnifying glass at the start of the search field.
fn magnifier(painter: &egui::Painter, field: Rect, color: egui::Color32) {
    let centre = egui::pos2(field.min.x + 13.0, field.center().y - 1.0);
    painter.circle_stroke(centre, 4.5, Stroke::new(1.5, color));
    painter.line_segment(
        [centre + egui::vec2(3.3, 3.3), centre + egui::vec2(6.5, 6.5)],
        Stroke::new(1.5, color),
    );
}

/// The small button that puts a row back to its default: an anticlockwise arrow.
fn reset_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
    let response = response.on_hover_text(tr("Restore defaults"));
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(theme::radius()),
            theme::button_hover(),
        );
    }
    let color = if response.hovered() {
        theme::fg()
    } else {
        theme::dim()
    };
    let centre = rect.center();
    let radius = 5.5;
    let start = -std::f32::consts::FRAC_PI_2;
    let points: Vec<egui::Pos2> = (0..=20)
        .map(|i| {
            let angle = start + 1.6 * std::f32::consts::PI * i as f32 / 20.0;
            centre + radius * egui::vec2(angle.cos(), angle.sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    let tip = centre - egui::vec2(0.0, radius);
    painter.add(egui::Shape::line(
        vec![tip + egui::vec2(3.0, -3.0), tip, tip + egui::vec2(3.0, 3.0)],
        Stroke::new(1.5, color),
    ));
    response
}

/// The width a row's control takes in the space left for it.
fn control_width(ui: &egui::Ui) -> f32 {
    ui.available_width().min(CONTROL_MAX)
}

/// A drop-down over `all`, named by `name`.
fn choose<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    all: &[T],
    name: impl Fn(T) -> String,
) {
    let names: Vec<String> = all.iter().map(|v| name(*v)).collect();
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut index = all.iter().position(|v| v == value).unwrap_or(0);
    if widgets::dropdown(
        ui,
        ui.scope_id().with("choice"),
        &mut index,
        &borrowed,
        control_width(ui),
    ) {
        *value = all[index];
    }
}

trait Control: PartialEq {
    fn control(&mut self, ui: &mut egui::Ui, field: &str);
    /// Whether this is the value `other`, as far as anyone could tell.
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
impl Control for bool {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        widgets::switch(ui, self);
    }
}
impl Control for String {
    fn control(&mut self, ui: &mut egui::Ui, field: &str) {
        let width = control_width(ui);
        match field {
            "keyboard_bindings" => crate::shortcut_editor::control(ui, self),
            "game" => {
                // "None" first, then every game found; a game
                // that is no longer there shows as None.
                GAMES.with(|found| {
                    let games = found.borrow();
                    let mut index = games
                        .iter()
                        .position(|game| game.name == *self)
                        .map_or(0, |at| at + 1);
                    if widgets::game_dropdown(
                        ui,
                        ui.scope_id().with("game"),
                        &mut index,
                        &games,
                        width,
                    ) {
                        *self = if index == 0 {
                            String::new()
                        } else {
                            games[index - 1].name.clone()
                        };
                    }
                });
            }
            "game_path" | "asset_library" => {
                let browse = tr("Browse");
                let button = ui.fonts_mut(|f| {
                    f.layout_no_wrap(browse.to_string(), theme::body(), theme::fg())
                        .size()
                        .x
                }) + 24.0;
                // Said at the field rather than when the folder is first needed. The
                // browser build sees no folders to check.
                let missing = !cfg!(target_arch = "wasm32")
                    && !self.is_empty()
                    && !std::path::Path::new(self.as_str()).is_dir();
                let warning = if missing {
                    crate::chrome::text_width(ui, egui_phosphor::regular::WARNING, theme::icon())
                        + ui.spacing().item_spacing.x
                } else {
                    0.0
                };
                let text_width = (width - button - warning - ui.spacing().item_spacing.x).max(60.0);
                if let Some(text) = widgets::field(
                    ui,
                    ui.scope_id().with("path"),
                    self,
                    tr("Choose"),
                    text_width,
                ) {
                    *self = text;
                }
                let browse = theme::button(ui, browse, button, true).clicked();
                let mut chooser =
                    crate::file_dialog::FileDialog::new().id(&format!("preference-{field}"));
                if field == "game_path" {
                    chooser = chooser.set_title(tr("Where the game's own files are"));
                }
                if let Some(path) = chooser.pick_folder(browse) {
                    *self = path.display().to_string();
                }
                if missing {
                    ui.label(
                        egui::RichText::new(egui_phosphor::regular::WARNING)
                            .font(theme::icon())
                            .color(theme::error()),
                    )
                    .on_hover_text(tr("This folder does not exist"));
                }
            }
            "game_mods" => {
                if let Some(text) =
                    widgets::field(ui, ui.scope_id().with("text"), self, "mod_a;mod_b", width)
                {
                    *self = text;
                }
            }
            "collab_nickname" => {
                let hint = editor_core::collab::default_nickname();
                if let Some(text) =
                    widgets::field(ui, ui.scope_id().with("text"), self, &hint, width)
                {
                    *self = text;
                }
            }
            _ => {
                if let Some(text) = widgets::field(ui, ui.scope_id().with("text"), self, "", width)
                {
                    *self = text;
                }
            }
        }
    }
}
impl Control for f32 {
    fn control(&mut self, ui: &mut egui::Ui, field: &str) {
        let (min, max, step) = number_range(field, IN_OVERVIEW.get());
        // The digits follow the step: 0.0005 needs four, 1 needs none.
        let decimals = if step >= 1.0 {
            0
        } else {
            (-step.log10().floor()) as usize
        };
        // A value outside this row's range (a wider field of view set on the Mouse page)
        // shows at the nearest end and is left alone until the user changes it here.
        let shown = f64::from(*self).clamp(min, max);
        let mut value = shown;
        if widgets::spin_width(
            ui,
            &mut value,
            min..=max,
            step,
            decimals,
            control_width(ui).min(SPIN_MAX),
        )
        .changed()
            && value != shown
        {
            *self = value as f32;
        }
    }
}
impl Control for u32 {
    fn control(&mut self, ui: &mut egui::Ui, field: &str) {
        // One of a fixed set; a size the list does not have shows
        // as the nearest one above it.
        let sizes: &[u32] = if field == "shadow_map_size" {
            &[512, 1024, 2048, 4096]
        } else {
            &[4, 8, 16, 32, 64]
        };
        let names: Vec<String> = sizes.iter().map(u32::to_string).collect();
        let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut index = sizes
            .iter()
            .position(|size| *size >= *self)
            .unwrap_or(sizes.len() - 1);
        if widgets::dropdown(
            ui,
            ui.scope_id().with("size"),
            &mut index,
            &borrowed,
            control_width(ui),
        ) {
            *self = sizes[index];
        }
    }
}
/// A colour's channels as the picker holds them.
fn bytes(colour: &Color) -> ([u8; 3], u8) {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        [byte(colour.r), byte(colour.g), byte(colour.b)],
        byte(colour.a),
    )
}
impl Control for Color {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        let (mut rgb, _) = bytes(self);
        let mut alpha = self.a;
        if colour_picker::button(ui, ui.scope_id().with("colour"), &mut rgb, Some(&mut alpha))
            .changed()
        {
            *self = Color::rgba(
                f32::from(rgb[0]) / 255.0,
                f32::from(rgb[1]) / 255.0,
                f32::from(rgb[2]) / 255.0,
                alpha,
            );
        }
        let shown = colour_picker::hex(rgb, (alpha < 1.0).then_some(alpha));
        ui.label(
            egui::RichText::new(shown)
                .font(egui::FontId::monospace(theme::text_size() * 0.92))
                .color(theme::dim()),
        );
    }
    /// Colours are the same when the picker cannot tell them apart.
    fn same(&self, other: &Self) -> bool {
        bytes(self) == bytes(other)
    }
}
impl Control for Keymap {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        choose(ui, self, &Keymap::ALL, |v| v.name().to_string());
    }
}
impl Control for Layout {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        choose(ui, self, &Layout::ALL, |v| tr(v.name()).to_string());
    }
}
impl Control for editor_core::i18n::Language {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        // Following the desktop and every complete language, from `i18n::LANGUAGES`.
        let choices = editor_core::i18n::Language::choices(*self);
        choose(ui, self, &choices, |v| v.name().to_string());
    }
}
impl Control for editor_core::export::Target {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        choose(ui, self, &editor_core::export::Target::ALL, |v| {
            v.name().to_string()
        });
    }
}
impl Control for preferences::ToneMapper {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        choose(ui, self, &preferences::ToneMapper::ALL, |v| {
            v.name().to_string()
        });
    }
}
impl Control for ColorScheme {
    fn control(&mut self, ui: &mut egui::Ui, _field: &str) {
        let all = COLOR_SCHEMES.map(|(scheme, _)| scheme);
        choose(ui, self, &all, |v| {
            let name = COLOR_SCHEMES
                .iter()
                .find(|(scheme, _)| *scheme == v)
                .map_or("", |(_, name)| name);
            tr(name).to_string()
        });
    }
}

/// A number's range and step.
fn number_range(field: &str, overview: bool) -> (f64, f64, f64) {
    match field {
        "grid_alpha" => (0.0, 1.0, 0.05),
        "leak_check_cell_size" => (1.0, 64.0, 1.0),
        "weak_info_overlay_background_alpha" => (0.0, 1.0, 0.05),
        "camera_look_speed" => (0.05, 2.0, 0.05),
        "camera_pan_speed" => (0.05, 2.0, 0.05),
        "camera_move_speed" => (0.05, 4.0, 0.05),
        "camera_fly_move_speed" => (0.1, 10.0, 0.05),
        // The Overview stops at 120, the Mouse page at 140.
        "camera_fov" if overview => (30.0, 120.0, 1.0),
        "camera_fov" => (30.0, 140.0, 1.0),
        "handle_radius" => (1.0, 16.0, 0.5),
        "rotate_handle_radius" => (8.0, 512.0, 1.0),
        "sun_elevation" => (-89.0, 89.0, 1.0),
        "sun_azimuth" => (0.0, 360.0, 1.0),
        "sun_intensity" => (0.0, 20.0, 0.1),
        "ambient_intensity" => (0.0, 8.0, 0.05),
        "exposure" => (0.05, 8.0, 0.05),
        "shadow_bias" => (0.0, 0.02, 0.0005),
        "shadow_softness" => (0.0, 4.0, 0.1),
        "contact_shadow_distance" => (1.0, 128.0, 1.0),
        "contact_shadow_samples" => (2.0, 48.0, 1.0),
        "ao_radius" => (1.0, 256.0, 1.0),
        "ao_strength" => (0.0, 4.0, 0.05),
        "reflection_distance" => (16.0, 4096.0, 16.0),
        "reflection_samples" => (4.0, 96.0, 1.0),
        "reflection_strength" => (0.0, 2.0, 0.05),
        "bloom_threshold" => (0.0, 8.0, 0.05),
        "bloom_intensity" => (0.0, 2.0, 0.05),
        "default_roughness" => (0.0, 1.0, 0.05),
        "default_metalness" => (0.0, 1.0, 0.05),
        "manual_font_size" => (10.0, 28.0, 1.0),
        "panel_opacity" => (80.0, 100.0, 1.0),
        "interface_scale" => (
            theme::INTERFACE_SCALE.0 as f64,
            theme::INTERFACE_SCALE.1 as f64,
            5.0,
        ),
        _ => (0.0, 1000.0, 0.05),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::games::{Tag, TagRule};

    /// Every table row names a real field, and every field is somewhere a user can reach.
    #[test]
    fn tables_name_real_fields_and_cover_them_all() {
        let mut settings = Preferences::default();
        let defaults = Preferences::default();
        let rows: Vec<Row> = (0..=ELSEWHERE_PAGE)
            .flat_map(sections)
            .flat_map(|(_, rows)| rows.iter().copied())
            .collect();
        for (name, _) in &rows {
            assert!(
                field(&mut settings, name, Action::Reset(&defaults)),
                "{name} is not a field"
            );
        }
        // Every field of the store is in a table: the store writes one `key=value` line
        // per field after its version line, and the tables name as many distinct fields.
        let mut names: Vec<&str> = rows.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        let store =
            std::env::temp_dir().join(format!("heron-brush-prefs-count-{}", std::process::id()));
        defaults.save(&store).unwrap();
        let stored = std::fs::read_to_string(&store).unwrap().lines().count() - 1;
        let _ = std::fs::remove_file(&store);
        assert_eq!(names.len(), stored, "a preference has no row");
        let titles: Vec<&str> = (0..CATEGORIES).map(category_title).collect();
        assert_eq!(
            titles,
            ["Overview", "Games", "View", "Colours", "Mouse", "Keyboard", "Renderer", "Updates"]
        );
    }

    /// Search finds by label, heading or field name, lists a field once under its own page,
    /// and leaves out what does not match.
    #[test]
    fn search_filters_rows() {
        let names = |query: &str| {
            search(query)
                .into_iter()
                .map(|hit| hit.row.0)
                .collect::<Vec<_>>()
        };
        let shadow = names("shadow");
        for wanted in [
            "shadows",
            "shadow_map_size",
            "shadow_bias",
            "contact_shadows",
        ] {
            assert!(shadow.contains(&wanted), "{wanted} missing from {shadow:?}");
        }
        assert!(!shadow.contains(&"camera_fov"));
        // A heading finds the rows under it whatever they are called.
        assert!(names("bloom").contains(&"bloom_threshold"));
        // Shown by the Overview and by View, found once, under View.
        let axes = search("world axes");
        assert_eq!(
            axes.iter().filter(|hit| hit.row.0 == "show_axes").count(),
            1
        );
        assert_eq!(
            axes.iter()
                .find(|hit| hit.row.0 == "show_axes")
                .map(|hit| hit.page),
            Some(2)
        );
        assert!(names("asset").contains(&"asset_library"));
        assert!(names("no such setting").is_empty());
    }

    /// The window draws only what a search finds, and a category's rows otherwise.
    #[test]
    fn the_window_draws_the_rows_a_search_finds() {
        preferences::set_current(Preferences::default());
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        theme::install(&ctx);
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1360.0, 2000.0),
            )),
            ..Default::default()
        };
        let mut page = PreferencesPage {
            open: true,
            query: "shadow".into(),
            ..Default::default()
        };
        for _ in 0..2 {
            ctx.run_ui(input(), |ui| {
                page.show(ui.ctx());
            })
            .textures_delta
            .clear();
        }
        let found: Vec<&str> = search("shadow").into_iter().map(|hit| hit.row.0).collect();
        assert_eq!(page.drawn, found);
        page.query.clear();
        page.page = 3;
        for _ in 0..2 {
            ctx.run_ui(input(), |ui| {
                page.show(ui.ctx());
            })
            .textures_delta
            .clear();
        }
        assert!(page.drawn.contains(&"face_color") && !page.drawn.contains(&"shadows"));
        assert_eq!(
            preferences::current(),
            Preferences::default(),
            "drawing changed a preference"
        );
    }

    /// Choosing a game gives the map its tags and face-flag names.
    #[test]
    fn changing_the_game_reapplies_its_tags() {
        let tag = |name: &str| Tag {
            name: name.into(),
            rule: TagRule::Material,
            pattern: format!("{name}*"),
            transparent: false,
        };
        let game = Game {
            name: "Example".into(),
            icon: None,
            search_path: None,
            directory: std::env::temp_dir(),
            formats: Vec::new(),
            definitions: Vec::new(),
            materials_root: None,
            materials_palette: None,
            clip_material: None,
            skip_material: None,
            brush_tags: vec![tag("trigger")],
            face_tags: vec![tag("clip"), tag("skip")],
            surface_flag_names: vec!["light".into(), "slick".into()],
            content_flag_names: vec!["solid".into()],
        };
        let mut document = Document::new(Default::default());
        apply_game(&mut document, Some(&game));
        let (brush, face) = document.tags();
        assert_eq!((brush, face), (&game.brush_tags[..], &game.face_tags[..]));
        assert_eq!(document.surface_flag_names(), &game.surface_flag_names[..]);
        assert_eq!(document.content_flag_names(), &game.content_flag_names[..]);
    }

    /// A store that cannot be written still lets the change apply, says why, and a game
    /// change is reported so the map is brought up to date.
    #[test]
    fn a_change_applies_even_when_the_store_cannot_be_written() {
        preferences::set_current(Preferences::default());
        let blocker =
            std::env::temp_dir().join(format!("heron-brush-prefs-blocker-{}", std::process::id()));
        std::fs::write(&blocker, "a file where a directory should be").unwrap();
        let mut page = PreferencesPage::default();
        let before = preferences::current();
        let mut after = before.clone();
        after.game = "Quake".into();
        assert!(page.commit(&before, after, &blocker.join("preferences")));
        assert_eq!(preferences::pref(|p| p.game.clone()), "Quake");
        assert!(
            page.error.contains("could not be written"),
            "{}",
            page.error
        );
        let mut again = preferences::current();
        let before = again.clone();
        again.show_axes = !again.show_axes;
        assert!(!page.commit(&before, again, &blocker.join("preferences")));
        let _ = std::fs::remove_file(&blocker);
        preferences::set_current(Preferences::default());
    }

    /// Rows whose ranges are easy to get wrong keep the intended ones.
    #[test]
    fn number_rows_keep_their_ranges() {
        assert_eq!(number_range("panel_opacity", false), (80.0, 100.0, 1.0));
        assert_eq!(number_range("manual_font_size", true), (10.0, 28.0, 1.0));
        assert_eq!(number_range("camera_fov", true).1, 120.0);
        assert_eq!(number_range("camera_fov", false).1, 140.0);
    }
}
