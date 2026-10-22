//! The window that writes the map out, for an engine or as a plain `.map`: the engines down a sidebar with their logos, the `.map`
//! last, and the one in front is what Write writes.
//!
//! Every control writes its engine's settings as it changes, so the window opens where it
//! was left and each engine keeps its own answers. The live link's controls lead the pages
//! of the engines that have one (`live_link_ui.rs`).
use std::path::{Path, PathBuf};

use crate::{dialog, editor::Editor, file_dialog::FileDialog, form, theme, widgets};
use editor_core::{
    export::{self, Options, Target},
    export_settings::Settings,
    i18n::{counted, fill, tr, translation_key},
};
use eframe::egui;
use map_model::map_format::MapFormat;

/// One tab of the window: an engine, or the map itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Write the level for this engine.
    Engine(Target),
    /// Write the level as a plain `.map` file.
    Map,
}

impl Tab {
    /// Every tab, in the order the window lists them: the engines, then the map.
    fn all() -> Vec<Tab> {
        Target::ALL
            .iter()
            .map(|target| Tab::Engine(*target))
            .chain(std::iter::once(Tab::Map))
            .collect()
    }

    /// The name beside the tab's logo.
    fn name(self) -> &'static str {
        match self {
            Tab::Engine(target) => target.name(),
            Tab::Map => ".map",
        }
    }
}

/// The tab's logo, a 96 px rounded card (`heron-brush/assets/engines`).
fn logo_png(tab: Tab) -> &'static [u8] {
    match tab {
        Tab::Engine(Target::Godot) => include_bytes!("../assets/engines/godot.png"),
        Tab::Engine(Target::Defold) => include_bytes!("../assets/engines/defold.png"),
        Tab::Engine(Target::ThreeJs) => include_bytes!("../assets/engines/threejs.png"),
        Tab::Engine(Target::Bevy) => include_bytes!("../assets/engines/bevy.png"),
        Tab::Engine(Target::Raylib) => include_bytes!("../assets/engines/raylib.png"),
        Tab::Engine(Target::Unity) => include_bytes!("../assets/engines/unity.png"),
        Tab::Engine(Target::Unreal) => include_bytes!("../assets/engines/unreal.png"),
        Tab::Engine(Target::Flax) => include_bytes!("../assets/engines/flax.png"),
        Tab::Engine(Target::Stride) => include_bytes!("../assets/engines/stride.png"),
        Tab::Engine(Target::Cocos) => include_bytes!("../assets/engines/cocos.png"),
        Tab::Engine(Target::GDevelop) => include_bytes!("../assets/engines/gdevelop.png"),
        Tab::Engine(Target::Obj) => include_bytes!("../assets/engines/obj.png"),
        Tab::Engine(Target::Gltf) => include_bytes!("../assets/engines/gltf.png"),
        Tab::Engine(Target::Glb) => include_bytes!("../assets/engines/glb.png"),
        Tab::Map => include_bytes!("../assets/engines/map.png"),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapDialect {
    Classic(MapFormat),
    Quake3Primitive,
    Doom3Primitive,
}

/// The `.map` dialects the window offers, in the order it lists them. The first is the one
/// the editor itself saves in and the one that can hold everything.
const DIALECTS: [(MapDialect, &str); 6] = [
    (MapDialect::Classic(MapFormat::Valve), "Valve 220"),
    (
        MapDialect::Classic(MapFormat::Standard),
        translation_key("Standard"),
    ),
    (MapDialect::Classic(MapFormat::Quake2), "Quake 2"),
    (
        MapDialect::Classic(MapFormat::Quake2Valve),
        "Quake 2 (Valve)",
    ),
    (MapDialect::Quake3Primitive, "Quake 3 (brush primitives)"),
    (MapDialect::Doom3Primitive, "Doom 3 (brushDef3)"),
];

/// Which parts of an engine's page it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    /// The live link's controls.
    pub live: bool,
    /// "What to write": collision geometry.
    pub collision: bool,
    /// "Optimisations": buried faces, joined faces and joined collision.
    pub optimisations: bool,
    /// Grouping static meshes in cells.
    pub static_cells: bool,
    /// The Godot `res://` texture and material folders.
    pub godot_paths: bool,
    /// Defold's asset directory, model material and sampler.
    pub defold_paths: bool,
    /// The Unreal, Flax and Stride content directory.
    pub content_dir: bool,
    /// Clip, skip and origin.
    pub tool_materials: bool,
    /// The "More" section.
    pub more: bool,
    /// The project folder, in More.
    pub project_folder: bool,
    /// The trigger layer and the lightmap rows, in More.
    pub godot_extras: bool,
    /// "What to write": the sky and sun.
    pub sky: bool,
}

/// What `target`'s page shows.
pub fn shown(target: Target) -> Shown {
    let portable = matches!(target, Target::Gltf | Target::Glb);
    Shown {
        live: matches!(
            target,
            Target::Godot | Target::Unity | Target::Defold | Target::Flax
        ),
        collision: !portable,
        optimisations: !portable,
        static_cells: !portable && target != Target::Obj,
        godot_paths: target == Target::Godot,
        defold_paths: target == Target::Defold,
        content_dir: matches!(target, Target::Unreal | Target::Flax | Target::Stride),
        tool_materials: target != Target::Obj,
        more: target != Target::Obj,
        project_folder: !portable,
        godot_extras: target == Target::Godot,
        sky: target.exports_sky(),
    }
}

/// A text setting of an engine's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    TextureDir,
    MaterialDir,
    AssetDir,
    ModelMaterial,
    ModelSampler,
    ContentDir,
    Clip,
    Skip,
    Origin,
    ProjectDir,
    NameProperty,
}

impl Text {
    /// The row's label, untranslated.
    pub fn label(self) -> &'static str {
        match self {
            Text::TextureDir => translation_key("Textures"),
            Text::MaterialDir => translation_key("Materials"),
            Text::AssetDir => translation_key("Asset directory"),
            Text::ModelMaterial => translation_key("Model material"),
            Text::ModelSampler => translation_key("Texture sampler"),
            Text::ContentDir => translation_key("Content directory"),
            Text::Clip => translation_key("tool material|Clip"),
            Text::Skip => translation_key("tool material|Skip"),
            Text::Origin => translation_key("tool material|Origin"),
            Text::ProjectDir => translation_key("Project folder"),
            Text::NameProperty => translation_key("Name nodes after"),
        }
    }

    /// The field these words edit.
    pub fn value(self, options: &mut Options) -> &mut String {
        match self {
            Text::TextureDir => &mut options.texture_dir,
            Text::MaterialDir => &mut options.material_dir,
            Text::AssetDir => &mut options.asset_dir,
            Text::ModelMaterial => &mut options.model_material,
            Text::ModelSampler => &mut options.model_sampler,
            Text::ContentDir => &mut options.content_dir,
            Text::Clip => &mut options.clip_texture,
            Text::Skip => &mut options.skip_texture,
            Text::Origin => &mut options.origin_texture,
            Text::ProjectDir => &mut options.project_dir,
            Text::NameProperty => &mut options.name_property,
        }
    }

    /// What an empty field means, shown in it.
    pub fn placeholder(self, target: Target) -> String {
        match self {
            Text::TextureDir => "res://textures".into(),
            Text::MaterialDir => tr("none").into(),
            Text::AssetDir => format!("/{}", tr("<level name>")),
            Text::ModelMaterial => "/builtins/materials/model.material".into(),
            Text::ModelSampler => "tex0".into(),
            Text::ContentDir if target == Target::Unreal => "/Game/<level>".into(),
            Text::ContentDir if target == Target::Stride => "Levels/<level>".into(),
            Text::ContentDir => "<level>".into(),
            Text::Clip => "clip".into(),
            Text::Skip => "skip".into(),
            Text::Origin => "origin".into(),
            Text::ProjectDir => tr("found from where the level is written").into(),
            Text::NameProperty => tr("the classname").into(),
        }
    }

    /// The field's tooltip.
    pub fn tooltip(self) -> Option<&'static str> {
        Some(match self {
            Text::TextureDir => tr("The res:// directory texture images are looked up in"),
            Text::MaterialDir => tr("A res:// directory searched for hand-made materials"),
            Text::AssetDir => tr("The resource directory the exported files live under, e.g. /main/arena"),
            Text::Clip => tr("Faces wearing it are collision and nothing else"),
            Text::Skip => tr("Faces wearing it are dropped altogether"),
            Text::Origin => tr("A brush wearing it marks its entity's origin rather than geometry"),
            Text::ProjectDir => tr(
                "The folder this engine's project begins at, which is what turns a texture name into \
                 a file. Left empty it is found by walking up from wherever the level is written",
            ),
            Text::NameProperty => tr("The entity property whose value names the node, such as targetname"),
            _ => return None,
        })
    }
}

/// What an engine gets, in a sentence, so the tab says what pressing Write will leave.
fn what_it_writes(target: Target) -> &'static str {
    match target {
        Target::Godot => translation_key("A .tscn scene with the meshes as .obj beside it, which Godot imports like any other asset."),
        Target::Defold => translation_key("A .collection and the component files it names, with the meshes as .glb."),
        Target::ThreeJs => translation_key("Playable HTML with FPS controls and collisions. Open it in a browser; internet is needed for Three.js. JSON and .glb files are included."),
        Target::Bevy => translation_key("The level as JSON with the meshes as .glb, which Bevy's asset loader reads."),
        Target::Raylib => translation_key("The level as flat text, one keyword to a line, with the meshes as .glb."),
        Target::Unity => translation_key("The meshes as .obj with a generated C# Editor script: drop the folder into Assets and run Heron Brush > Build Level."),
        Target::Unreal => translation_key("The meshes as .glb with a generated Python script: run it inside the Unreal editor to import and place everything."),
        Target::Flax => translation_key("The meshes as .glb with a generated C# editor script: put the folder under Content and run HeronBrushLevel.Build()."),
        Target::Stride => translation_key("Native Stride model, material and sky assets, Bullet colliders, animation and triggers, and a C# loader with project callbacks."),
        Target::GDevelop => translation_key("A GDevelop scene in the project's game.json: 3D models, Physics3D collision, typed variables, the sky and sun, and an event that plays animation and triggers."),
        Target::Cocos => translation_key("The level as JSON with .glb models and the sky, and a Cocos Creator component that builds it with colliders, animation and triggers. Cocos shares this frame, so nothing is converted."),
        Target::Gltf => translation_key("A .gltf scene with a .bin file and PNG textures. Preserves brushes, patches, groups and face paint. Entity properties are stored as metadata; game behaviour and physics require an importer."),
        Target::Glb => translation_key("One .glb file containing the scene, materials and textures. Preserves brushes, patches, groups and face paint. Entity properties are stored as metadata; game behaviour and physics require an importer."),
        Target::Obj => translation_key("One .obj with an .mtl beside it: the geometry and its materials, for a modelling tool."),
    }
}

/// What the live link asks of the user before it starts, per engine.
fn live_hint(target: Target) -> &'static str {
    match target {
        Target::Flax => tr("Install the Flax addon, start the link, then choose Tools > Heron Brush: attach live map in Flax. Updates pause during Play Mode."),
        Target::Defold => tr("Open your project in Defold. Live edits update heron_brush_live/preview.collection without saving the map. No addon is needed."),
        Target::Unity => tr("Export into Assets, install the addon, then open Window > Heron Brush > Live Link in Unity. Unsaved edits sync too."),
        _ => tr("Export once into your Godot project. Install the addon, enable Heron Brush Live in Godot's Plugins settings, then start the link and attach the map in its dock. Updates include unsaved edits."),
    }
}

/// What the export wrote, in a line or a few: the counts, then the first warnings.
pub fn describe(report: &export::Report) -> String {
    let mut said = format!(
        "{} · {}",
        counted(report.files.len(), "file"),
        counted(report.triangles, "triangle")
    );
    if report.saved_faces > 0 {
        said.push_str(", ");
        said.push_str(&fill("{} saved", &[&counted(report.saved_faces, "face")]));
    }
    // The first few warnings; a map with a hundred of them would fill the window.
    for warning in report.warnings.iter().take(3) {
        said.push('\n');
        said.push_str(warning);
    }
    if report.warnings.len() > 3 {
        said.push('\n');
        said.push_str(&fill(
            "… and {} more",
            &[&(report.warnings.len() - 3).to_string()],
        ));
    }
    said
}

/// What a dialect would cost the map, as the `.map` tab says it.
fn loss_text(lost: &[String]) -> String {
    format!(
        "{} {}",
        tr("Written this way,"),
        lost.join(&format!(", {} ", tr("and")))
    )
}

/// The `res://` path of `chosen` inside the Godot project at `root`, or `None` when it is
/// outside the project.
pub fn resource_path(chosen: &Path, root: Option<&Path>) -> Option<String> {
    let relative = chosen.strip_prefix(root?).ok()?;
    Some(format!(
        "res://{}",
        relative.to_string_lossy().replace('\\', "/")
    ))
}

/// Everything a check found, for the window that lists it.
struct Problems {
    target: Target,
    warnings: Vec<String>,
}

pub struct ExportPage {
    pub open: bool,
    was_open: bool,
    /// The tab in front.
    tab: Tab,
    settings: Settings,
    /// The file the settings are written to.
    store: PathBuf,
    profile_map: Option<PathBuf>,
    entity_rules_json: String,
    /// The front engine's settings, as the page edits them.
    options: Options,
    /// The `.map` tab's dialect.
    dialect: usize,
    /// What the last Write or Check said.
    status: String,
    /// The window listing what a check found, while it is open.
    problems: Option<Problems>,
    logos: Vec<Option<egui::TextureHandle>>,
}

impl Default for ExportPage {
    fn default() -> Self {
        // Tests never write over the settings of whoever runs them.
        let store = if cfg!(test) {
            std::env::temp_dir().join(format!(
                "heron-brush-export-settings-{}",
                std::process::id()
            ))
        } else {
            editor_core::export_settings::default_path()
        };
        let settings = Settings::load_from(&store);
        let target = editor_core::preferences::pref(|p| p.export_target);
        Self {
            open: false,
            was_open: false,
            tab: Tab::Engine(target),
            options: settings.for_target(target),
            settings,
            store,
            profile_map: None,
            entity_rules_json: String::new(),
            dialect: 0,
            status: String::new(),
            problems: None,
            logos: Vec::new(),
        }
    }
}

impl ExportPage {
    /// A page whose settings are read from and written to `store`.
    #[cfg(test)]
    fn with_store(store: PathBuf) -> Self {
        let settings = Settings::load_from(&store);
        Self {
            options: settings.for_target(Target::Godot),
            tab: Tab::Engine(Target::Godot),
            settings,
            store,
            ..Self::default()
        }
    }

    /// Brings `tab` to the front, remembering the engine for next time.
    fn pick(&mut self, tab: Tab, document: &editor_core::document::Document) {
        self.tab = tab;
        match tab {
            Tab::Engine(target) => {
                self.options = self.settings.for_target(target);
                self.entity_rules_json.clear();
                editor_core::preferences::edit(|p| p.export_target = target);
            }
            Tab::Map => {
                // The dialect the map was read as, so a Quake map offers to go back out as one.
                self.dialect = DIALECTS
                    .iter()
                    .position(|(format, _)| *format == MapDialect::Classic(document.format))
                    .unwrap_or(0);
            }
        }
    }

    /// Writes the front engine's settings, as every control does when it changes. The
    /// level name is never kept: it is the map's, filled in at export.
    fn commit(&mut self) {
        if let Err(error) = self.settings.validate() {
            self.status = error;
            return;
        }
        let Tab::Engine(target) = self.tab else {
            return;
        };
        let mut options = self.options.clone();
        options.name.clear();
        options.target = target;
        self.settings.set_in(target, options, &self.store);
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        live: &std::rc::Rc<editor_core::godot_live::LiveLink>,
    ) {
        self.show_problems(ctx);
        if !self.open {
            self.was_open = false;
            return;
        }
        if !cfg!(test) {
            let map = editor.document.borrow().path.clone();
            if !self.was_open || self.profile_map != map {
                let store = map
                    .as_deref()
                    .map(editor_core::export_settings::project_path)
                    .unwrap_or_else(editor_core::export_settings::default_path);
                if store != self.store {
                    self.store = store;
                    self.was_open = false;
                }
                self.profile_map = map;
            }
        }
        if !self.was_open {
            // Opened afresh: the settings as they are on disk, the last engine in front.
            self.was_open = true;
            self.settings = Settings::load_from(&self.store);
            if let Tab::Engine(_) = self.tab {
                let target = editor_core::preferences::pref(|p| p.export_target);
                self.tab = Tab::Engine(target);
                self.options = self.settings.for_target(target);
            }
            self.entity_rules_json.clear();
            self.status = self.settings.validate().err().unwrap_or_default();
        }
        if self.logos.is_empty() {
            self.logos = Tab::all()
                .into_iter()
                .map(|tab| load_logo(ctx, tab))
                .collect();
        }
        let embedded = ctx.embed_viewports();
        let mut close = false;
        let mut write = false;
        let mut check = false;
        self.open = dialog::show(ctx, "export", tr("Export"), [760.0, 620.0], |ui, part| {
            if part == dialog::Part::Actions {
                write = form::accent_button(ui, tr("Write"), true).clicked();
                close = dialog::button(ui, tr("Close"), true).clicked();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    check = dialog::button(ui, tr("Check"), true)
                        .on_hover_text(tr("Build the level without writing it, and say what is wrong with the result"))
                        .clicked();
                });
                return;
            }
            self.body(ui, editor, live, embedded);
        }) && !close;
        self.write(editor, write);
        if check {
            self.check(editor);
        }
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        live: &std::rc::Rc<editor_core::godot_live::LiveLink>,
        embedded: bool,
    ) {
        let full = form::body_rect(ui, embedded);
        let side = egui::Rect::from_min_size(
            full.min,
            egui::vec2(184.0_f32.min(full.width() * 0.35), full.height()),
        );
        let content =
            egui::Rect::from_min_max(egui::pos2(side.right() + 12.0, full.top()), full.max);
        let mut side_ui = form::region(ui, side);
        self.sidebar(&mut side_ui, editor);
        let mut content_ui = form::region(ui, content);
        let status_height = if self.status.is_empty() {
            0.0
        } else {
            8.0 + self.status.lines().count().max(1) as f32 * 18.0
        };
        egui::ScrollArea::vertical()
            .id_salt(("export-page", self.tab.name()))
            .auto_shrink([false, false])
            .max_height((content.height() - status_height).max(80.0))
            .show(&mut content_ui, |ui| {
                ui.set_max_width(ui.available_width() - 10.0);
                match self.tab {
                    Tab::Engine(target) => self.engine_page(ui, target, live),
                    Tab::Map => self.map_page(ui, editor),
                }
            });
        if !self.status.is_empty() {
            content_ui.add_space(8.0);
            form::note(&mut content_ui, &self.status);
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        form::sidebar_frame(ui, ui.max_rect());
        let inner = ui.max_rect().shrink(6.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("export-targets")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (index, tab) in Tab::all().into_iter().enumerate() {
                        let logo = self.logos.get(index).and_then(Option::as_ref);
                        if form::sidebar_item(ui, tab.name(), self.tab == tab, logo, 44.0).clicked()
                            && self.tab != tab
                        {
                            self.pick(tab, &editor.document.borrow());
                            self.status.clear();
                        }
                    }
                });
        });
    }

    fn engine_page(
        &mut self,
        ui: &mut egui::Ui,
        target: Target,
        live: &std::rc::Rc<editor_core::godot_live::LiveLink>,
    ) {
        let show = shown(target);
        let portable = matches!(target, Target::Gltf | Target::Glb);
        // Keep large class bindings out of the per-frame comparison/allocation.
        let entities = std::mem::take(&mut self.options.entities);
        let before = self.options.clone();
        self.options.entities = entities;
        let mut bindings_changed = false;
        ui.label(
            egui::RichText::new(target.name())
                .font(egui::FontId::new(theme::text_size() + 4.0, theme::bold()))
                .color(theme::fg()),
        );
        form::note(ui, tr(what_it_writes(target)));
        ui.add_space(8.0);
        if show.live {
            live_link(ui, live, target);
            ui.add_space(10.0);
        }
        widgets::card(ui, tr("Scale"), None, |ui| {
            widgets::row(
                ui,
                tr("Units per metre"),
                Some(tr("Map units to one engine unit")),
                |ui| {
                    widgets::spin_width(
                        ui,
                        &mut self.options.scale,
                        1.0..=1024.0,
                        1.0,
                        0,
                        ui.available_width().min(160.0),
                    );
                },
            );
        });
        ui.add_space(10.0);
        if show.collision {
            widgets::card(ui, tr("What to write"), None, |ui| {
                form::switch_row(
                    ui,
                    tr("Collision geometry"),
                    None,
                    &mut self.options.collision,
                );
                if show.sky {
                    form::switch_row(
                        ui,
                        tr("Sky and sun"),
                        Some(tr(
                            "The 3D view's sky, sun and fog. Leave off when the game sets its own",
                        )),
                        &mut self.options.sky,
                    );
                }
            });
            ui.add_space(10.0);
        }
        if show.optimisations {
            widgets::card(ui, tr("Optimisations"), None, |ui| {
                if show.static_cells {
                    form::switch_row(
                        ui,
                        tr("Group static meshes in 1024-unit cells"),
                        None,
                        &mut self.options.static_cells,
                    );
                }
                form::switch_row(
                    ui,
                    tr("Drop faces buried inside the geometry"),
                    None,
                    &mut self.options.cull_hidden,
                );
                form::switch_row(
                    ui,
                    tr("Join faces that share a plane"),
                    None,
                    &mut self.options.merge_faces,
                );
                form::switch_row(
                    ui,
                    tr("Join collision shapes"),
                    None,
                    &mut self.options.merge_collision,
                );
            });
            ui.add_space(10.0);
        }
        let paths: &[Text] = if show.godot_paths {
            &[Text::TextureDir, Text::MaterialDir]
        } else if show.defold_paths {
            &[Text::AssetDir, Text::ModelMaterial, Text::ModelSampler]
        } else if show.content_dir {
            &[Text::ContentDir]
        } else {
            &[]
        };
        if !paths.is_empty() {
            widgets::card(ui, tr("Project paths"), None, |ui| {
                for field in paths {
                    self.text_row(ui, target, *field);
                }
            });
            ui.add_space(10.0);
        }
        if show.tool_materials {
            widgets::card(ui, tr("Tool materials"), None, |ui| {
                for field in [Text::Clip, Text::Skip, Text::Origin] {
                    self.text_row(ui, target, field);
                }
            });
            ui.add_space(10.0);
        }
        if show.more {
            widgets::card(ui, "", None, |ui| {
                form::fold(
                    ui,
                    ("export-more", target.name()),
                    tr("More"),
                    false,
                    |ui| {
                        if show.project_folder {
                            self.text_row(ui, target, Text::ProjectDir);
                        }
                        if show.godot_extras {
                            widgets::row(
                            ui,
                            tr("Trigger layer"),
                            Some(tr(
                                "The physics layer a trigger's area goes on, 1 to 32. On the level's own layer it \
                                 stops every ray the game traces. 0 leaves it where the engine puts it",
                            )),
                            |ui| {
                                let width = ui.available_width().min(160.0);
                                let mut layer = f64::from(self.options.trigger_layer);
                                if widgets::spin_width(ui, &mut layer, 0.0..=32.0, 1.0, 0, width).changed() {
                                    self.options.trigger_layer = layer.round() as u32;
                                }
                            },
                        );
                        }
                        form::switch_row(
                            ui,
                            tr("Rebuild layers and groups as parent nodes"),
                            None,
                            &mut self.options.group_hierarchy,
                        );
                        form::switch_row(
                            ui,
                            tr("Reverse the triangle winding"),
                            Some(tr("For an importer that does not flip it on the way in")),
                            &mut self.options.flip_faces,
                        );
                        let flip = tr(if portable {
                            "Flip texture coordinates vertically"
                        } else {
                            "Write the V coordinate unflipped"
                        });
                        form::switch_row(
                            ui,
                            flip,
                            Some(tr(
                                "For an importer that measures V from the same end the map does",
                            )),
                            &mut self.options.flip_uv_v,
                        );
                        if show.godot_extras {
                            form::switch_row(
                                ui,
                                tr("Unwrap a second UV set for baked light"),
                                None,
                                &mut self.options.lightmap_uv2,
                            );
                            widgets::row(
                                ui,
                                tr("Lightmap texel"),
                                Some(tr("How big one lightmap texel is, in engine units")),
                                |ui| {
                                    let width = ui.available_width().min(160.0);
                                    widgets::spin_width(
                                        ui,
                                        &mut self.options.lightmap_texel_size,
                                        0.01..=10.0,
                                        0.05,
                                        2,
                                        width,
                                    );
                                },
                            );
                        }
                        self.text_row(ui, target, Text::NameProperty);
                    },
                );
            });
            ui.add_space(10.0);
        }
        widgets::card(ui, tr("Keeping up with the map"), None, |ui| {
            // It needs somewhere to write, which is wherever this engine was last exported to.
            let exported = !self.options.directory.is_empty();
            ui.add_enabled_ui(exported, |ui| {
                form::switch_row(
                    ui,
                    tr("Write again whenever the map is saved"),
                    Some(tr("Available once this engine has been exported once, since that is what says where to write")),
                    &mut self.options.export_on_save,
                );
            });
            let destination = if exported {
                self.options.directory.clone()
            } else {
                tr("Not exported yet").to_string()
            };
            form::note(ui, &destination);
        });
        if !matches!(target, Target::Obj | Target::Gltf | Target::Glb) {
            ui.add_space(10.0);
            ui.collapsing(tr("Entity bindings"), |ui| {
                form::note(
                    ui,
                    tr("Class build rules and typed defaults. Values in the map take precedence."),
                );
                if self.entity_rules_json.is_empty() {
                    self.entity_rules_json = serde_json::to_string_pretty(&self.options.entities)
                        .unwrap_or_else(|_| "{}".into());
                }
                ui.add(
                    egui::TextEdit::multiline(&mut self.entity_rules_json)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(f32::INFINITY),
                );
                if ui.button(tr("Apply")).clicked() {
                    match serde_json::from_str::<
                        std::collections::BTreeMap<String, editor_core::export_bindings::Rule>,
                    >(&self.entity_rules_json)
                    {
                        Ok(rules) => match editor_core::export_bindings::validate(&rules) {
                            Ok(()) => {
                                self.options.entities = rules;
                                bindings_changed = true;
                                self.status.clear();
                            }
                            Err(error) => self.status = error,
                        },
                        Err(error) => self.status = error.to_string(),
                    }
                }
                form::note(ui, &self.store.display().to_string());
            });
        }
        let entities = std::mem::take(&mut self.options.entities);
        let changed = self.options != before;
        self.options.entities = entities;
        if changed || bindings_changed {
            self.commit();
        }
    }

    /// A text setting's row, with a folder button for the folders.
    fn text_row(&mut self, ui: &mut egui::Ui, target: Target, field: Text) {
        let placeholder = field.placeholder(target);
        let folder = matches!(
            field,
            Text::TextureDir | Text::MaterialDir | Text::ProjectDir
        );
        ui.push_id(field.label(), |ui| {
            widgets::row(ui, tr(field.label()), field.tooltip(), |ui| {
                let button = if folder {
                    widgets::ROW_HEIGHT + ui.spacing().item_spacing.x
                } else {
                    0.0
                };
                let width = (ui.available_width() - button).max(60.0);
                let current = field.value(&mut self.options).clone();
                if let Some(text) =
                    widgets::field(ui, ui.id().with("text"), &current, &placeholder, width)
                {
                    *field.value(&mut self.options) = text.trim().to_string();
                }
                if folder {
                    let open = form::folder_button(ui, tr("Choose")).clicked();
                    self.choose_folder(field, open);
                }
            });
        });
    }

    /// Asks for a folder for `field` (`open`), and takes the answer: a `res://` path inside
    /// the Godot project for its folders, and a plain path for the project itself. Called
    /// every frame the row shows, since an in-app chooser answers frames later.
    fn choose_folder(&mut self, field: Text, open: bool) {
        let resource = field != Text::ProjectDir;
        let root = || {
            if !self.options.project_dir.is_empty() {
                Some(PathBuf::from(&self.options.project_dir))
            } else {
                scene_export::materials::find_project_root(Path::new(&self.options.directory))
            }
        };
        let mut chooser = FileDialog::new()
            .id(&format!("export-folder-{field:?}"))
            .set_title(tr("Choose"));
        if open {
            let root = root();
            let current = field.value(&mut self.options.clone()).clone();
            let initial = if let Some(relative) = current.strip_prefix("res://") {
                root.as_ref().map(|root| root.join(relative))
            } else if !current.is_empty() {
                Some(PathBuf::from(&current))
            } else {
                root
            };
            if let Some(path) = initial.filter(|path| path.is_dir()) {
                chooser = chooser.set_directory(path);
            }
        }
        let Some(path) = chooser.pick_folder(open) else {
            return;
        };
        let value = if resource {
            let root = root().or_else(|| scene_export::materials::find_project_root(&path));
            match resource_path(&path, root.as_deref()) {
                Some(value) => value,
                None => {
                    self.status =
                        tr("Choose an export folder inside the Godot project.").to_string();
                    return;
                }
            }
        } else {
            path.to_string_lossy().into_owned()
        };
        *field.value(&mut self.options) = value;
        self.commit();
    }

    fn map_page(&mut self, ui: &mut egui::Ui, editor: &Editor) {
        ui.label(
            egui::RichText::new(".map")
                .font(egui::FontId::new(theme::text_size() + 4.0, theme::bold()))
                .color(theme::fg()),
        );
        form::note(
            ui,
            tr(
                "The map as a plain .map file, the way a compiler or another editor reads \
                it. A layer marked to be left out of exports is written empty. \
                The map open in the editor is not changed either way.",
            ),
        );
        ui.add_space(8.0);
        widgets::card(ui, tr("Dialect"), None, |ui| {
            widgets::row(ui, tr("Write as"), None, |ui| {
                let names: Vec<&str> = DIALECTS.iter().map(|(_, name)| tr(name)).collect();
                let width = ui.available_width().min(240.0);
                widgets::dropdown(ui, "export-dialect", &mut self.dialect, &names, width);
            });
            ui.add_space(4.0);
            let document = editor.document.borrow();
            let lost = match DIALECTS[self.dialect % DIALECTS.len()].0 {
                MapDialect::Classic(format) => document.what_a_dialect_would_drop(format),
                MapDialect::Quake3Primitive => document.quake3_map_export_problems(),
                MapDialect::Doom3Primitive => {
                    let mut notes: Vec<_> = document
                        .quake3_map_export_problems()
                        .into_iter()
                        .map(|s| s.replace("Quake III", "Doom 3"))
                        .collect();
                    notes.push("Doom 3 brush/patch rotation must be baked into geometry and rotation/angle properties reset before export.".into());
                    notes.push("Doom 3 material definitions determine surface/content behavior; numeric face flags are not applied by the engine.".into());
                    notes
                }
            };
            if lost.is_empty() {
                form::note(ui, tr("This map loses nothing in that dialect."));
            } else {
                form::text(ui, &loss_text(&lost));
            }
        });
    }

    /// Write: the `.map` to a file chosen by name, or the engine's level to a folder,
    /// remembered so the next export and writing again on save can go there. `open` asks
    /// where; called every frame the window shows, since an in-app chooser answers later.
    fn write(&mut self, editor: &Editor, open: bool) {
        if let Err(error) = self.settings.validate() {
            if open {
                self.status = error;
            }
            return;
        }
        let Tab::Engine(_) = self.tab else {
            let name = format!("{}.map", editor.document.borrow().level_name());
            let Some(path) = FileDialog::new()
                .id("export-map")
                .set_title(tr("Export"))
                .set_file_name(name)
                .save_file(open)
            else {
                return;
            };
            let format = DIALECTS[self.dialect % DIALECTS.len()].0;
            let document = editor.document.borrow();
            let result = match format {
                MapDialect::Classic(format) => document.export_map_to(&path, format),
                MapDialect::Quake3Primitive => document.export_quake3_map_to(&path),
                MapDialect::Doom3Primitive => document.export_doom3_map_to(&path),
            };
            self.status = match result {
                Ok(()) => fill("wrote {}", &[&path.display().to_string()]),
                Err(why) => why.to_string(),
            };
            return;
        };
        // The browser has no folder to write into: the level is downloaded instead.
        #[cfg(target_arch = "wasm32")]
        {
            if open {
                self.status = self.download(editor);
            }
            return;
        }
        #[cfg_attr(target_arch = "wasm32", allow(unreachable_code))]
        let mut chooser = FileDialog::new()
            .id("export-level")
            .set_title(tr("Where to write the level"));
        // Opening where this engine last wrote: a project is exported to the same folder
        // over and over, and the second export should not have to find it.
        if !self.options.directory.is_empty() {
            chooser = chooser.set_directory(&self.options.directory);
        }
        let Some(path) = chooser.pick_folder(open) else {
            return;
        };
        let mut options = self.options.clone();
        options.name = editor.document.borrow().level_name();
        match export::export(&editor.document.borrow(), &path, &options) {
            Ok(report) => {
                self.status = describe(&report);
                self.options.directory = path.display().to_string();
                self.commit();
            }
            Err(why) => self.status = why.to_string(),
        }
    }

    /// Write in the web build: the level built in memory and handed to the browser as a
    /// download, a zip of the folder the desktop would write when it is more than one file.
    #[cfg(target_arch = "wasm32")]
    fn download(&mut self, editor: &Editor) -> String {
        let Tab::Engine(target) = self.tab else {
            return String::new();
        };
        let mut options = self.options.clone();
        options.name = editor.document.borrow().level_name();
        let directory = Path::new("/heron-brush/export");
        let (mut report, files) =
            match export::export_files(&editor.document.borrow(), directory, &options) {
                Ok(built) => built,
                Err(why) => return why.to_string(),
            };
        let entries: Vec<crate::zip::Entry> = files
            .into_iter()
            .map(|(path, contents)| crate::zip::Entry {
                path: path
                    .strip_prefix(directory)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                contents: match contents {
                    export::Contents::Text(text) => text.into_bytes(),
                    export::Contents::Bytes(bytes) => bytes,
                },
            })
            .collect();
        report.files = entries
            .iter()
            .map(|entry| PathBuf::from(&entry.path))
            .collect();
        match entries.as_slice() {
            [one] => {
                let name = Path::new(&one.path)
                    .file_name()
                    .map_or(one.path.clone(), |name| name.to_string_lossy().into_owned());
                crate::web::download(&name, &one.contents);
            }
            _ => {
                let engine: String = target
                    .name()
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() {
                            c.to_ascii_lowercase()
                        } else {
                            '-'
                        }
                    })
                    .collect();
                crate::web::download(
                    &format!("{}-{engine}.zip", options.name),
                    &crate::zip::write(&entries),
                );
            }
        }
        describe(&report)
    }

    /// The same build with the geometry checks on, and nothing written, for
    /// wherever this engine last wrote.
    fn check(&mut self, editor: &Editor) {
        if let Err(error) = self.settings.validate() {
            self.status = error;
            return;
        }
        let Tab::Engine(target) = self.tab else {
            self.status =
                tr("The .map tab writes the map itself; there is no level to check").to_string();
            return;
        };
        let mut options = self.options.clone();
        // The web build writes no folder to come back to; its downloads build in this one.
        if options.directory.is_empty() && cfg!(target_arch = "wasm32") {
            options.directory = "/heron-brush/export".to_string();
        }
        if options.directory.is_empty() {
            self.status = tr("Write this engine once first: a check needs the project to look its textures up in").to_string();
            return;
        }
        options.name = editor.document.borrow().level_name();
        let directory = PathBuf::from(&options.directory);
        match export::check(&editor.document.borrow(), &directory, &options) {
            Ok(report) if report.warnings.is_empty() => {
                self.status = fill(
                    "nothing wrong with {} triangles",
                    &[&report.triangles.to_string()],
                );
            }
            Ok(report) => {
                self.status = counted(report.warnings.len(), "issue");
                self.problems = Some(Problems {
                    target,
                    warnings: report.warnings,
                });
            }
            Err(why) => self.status = why.to_string(),
        }
    }

    /// The window listing everything a check found, since the status line holds one
    /// sentence.
    fn show_problems(&mut self, ctx: &egui::Context) {
        let Some(problems) = &self.problems else {
            return;
        };
        let mut close = false;
        let heading = fill(
            "{} in the {} level",
            &[
                &counted(problems.warnings.len(), "issue"),
                problems.target.name(),
            ],
        );
        let open = dialog::show(
            ctx,
            "export-problems",
            tr("What the check found"),
            [620.0, 460.0],
            |ui, part| {
                if part == dialog::Part::Actions {
                    close = dialog::button(ui, tr("Close"), true).clicked();
                    return;
                }
                theme::heading(ui, &heading);
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("export-problems")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height() - 48.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        for problem in &problems.warnings {
                            ui.add(egui::Label::new(problem.as_str()).wrap().selectable(true));
                        }
                    });
            },
        );
        if !open || close {
            self.problems = None;
        }
    }
}

/// The live-link section of an engine's page.
fn live_link(
    ui: &mut egui::Ui,
    live: &std::rc::Rc<editor_core::godot_live::LiveLink>,
    target: Target,
) {
    widgets::card(
        ui,
        &fill("Live link with {}", &[target.name()]),
        None,
        |ui| {
            form::note(ui, live_hint(target));
            ui.add_space(4.0);
            if target == Target::Defold {
                let mut hot = live.defold_hot_reload();
                if form::switch_row(
                ui,
                tr("Hot reload running game"),
                Some(tr("Start the game from Defold first. Adding objects or changing collisions may require restarting the level.")),
                &mut hot,
            ) {
                live.set_defold_hot_reload(hot);
            }
            }
            let install = match target {
                Target::Flax => tr("Install Flax addon"),
                Target::Unity => tr("Install Unity addon"),
                _ => tr("Install Godot addon"),
            };
            let mut labels = Vec::new();
            if target != Target::Defold {
                labels.push(install);
            }
            labels.extend([tr("Start live link"), tr("Stop session")]);
            let offset = usize::from(target == Target::Defold);
            match form::button_strip(ui, &labels, true).map(|index| index + offset) {
                Some(0) => live.install_addon(target),
                Some(1) => live.start_or_say(target),
                Some(_) => live.stop("Live link stopped"),
                None => {}
            }
            let status = live.status().message;
            if !status.is_empty() {
                ui.add_space(2.0);
                form::text(ui, &status);
            }
        },
    );
}

/// Decodes a tab's logo into a texture.
fn load_logo(ctx: &egui::Context, tab: Tab) -> Option<egui::TextureHandle> {
    let image = image::load_from_memory_with_format(logo_png(tab), image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ctx.load_texture(
        format!("export-logo-{}", tab.name()),
        pixels,
        egui::TextureOptions::LINEAR,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("heron-brush-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    /// "Name nodes after" names the property a node is named by, not the level, and a
    /// change is written to the store at once with no level name kept.
    #[test]
    fn name_nodes_after_is_the_name_property_and_settings_save_on_change() {
        let store = scratch("store");
        let mut page = ExportPage::with_store(store.clone());
        assert_eq!(Text::NameProperty.label(), "Name nodes after");
        *Text::NameProperty.value(&mut page.options) = "targetname".into();
        assert_eq!(page.options.name_property, "targetname");
        assert_ne!(
            page.options.name, "targetname",
            "the level name is left alone"
        );
        page.options.name = "my-arena".into();
        page.options.export_on_save = true;
        page.commit();
        let read = Settings::load_from(&store).for_target(Target::Godot);
        assert_eq!(read.name_property, "targetname");
        assert!(read.export_on_save);
        assert!(
            !std::fs::read_to_string(&store)
                .unwrap()
                .contains("my-arena"),
            "the level name is the map's, never saved"
        );
        let _ = std::fs::remove_file(&store);
    }

    /// Each engine shows the controls that apply to it.
    #[test]
    fn engines_show_their_own_controls() {
        let godot = shown(Target::Godot);
        assert!(
            godot.live
                && godot.godot_paths
                && godot.godot_extras
                && godot.static_cells
                && godot.more
        );
        let gltf = shown(Target::Gltf);
        assert!(
            !gltf.collision
                && !gltf.optimisations
                && !gltf.project_folder
                && !gltf.live
                && gltf.more
        );
        assert_eq!(shown(Target::Glb), gltf);
        let obj = shown(Target::Obj);
        assert!(obj.collision && !obj.static_cells && !obj.tool_materials && !obj.more);
        assert!(shown(Target::Defold).defold_paths && !shown(Target::Godot).defold_paths);
        assert!(
            shown(Target::Unreal).content_dir
                && shown(Target::Flax).content_dir
                && !shown(Target::Bevy).content_dir
        );
        assert!(!shown(Target::Unity).godot_extras && shown(Target::Unity).live);
        assert_eq!(
            Text::ContentDir.placeholder(Target::Unreal),
            "/Game/<level>"
        );
        assert_eq!(Text::ContentDir.placeholder(Target::Flax), "<level>");
    }

    /// A folder inside the Godot project becomes its `res://` path; one outside is refused.
    #[test]
    fn resource_paths_stay_inside_the_project() {
        let root = Path::new("/projects/game");
        assert_eq!(
            resource_path(Path::new("/projects/game/art/walls"), Some(root)).as_deref(),
            Some("res://art/walls")
        );
        assert_eq!(resource_path(Path::new("/elsewhere/art"), Some(root)), None);
        assert_eq!(resource_path(Path::new("/projects/game/art"), None), None);
    }

    /// The summary counts in words and stops after three warnings.
    #[test]
    fn the_summary_counts_and_caps_the_warnings() {
        let report = export::Report {
            files: vec![PathBuf::from("a"), PathBuf::from("b")],
            warnings: (0..5).map(|n| format!("warning {n}")).collect(),
            triangles: 1,
            saved_faces: 3,
        };
        let said = describe(&report);
        assert!(
            said.starts_with("2 files · 1 triangle, 3 faces saved"),
            "{said}"
        );
        assert!(said.contains("warning 2") && !said.contains("warning 3"));
        assert!(said.ends_with("… and 2 more"), "{said}");
        assert_eq!(
            loss_text(&["a".into(), "b".into()]),
            "Written this way, a, and b"
        );
    }

    /// Every tab's logo decodes.
    #[test]
    fn every_tab_has_a_logo() {
        let ctx = egui::Context::default();
        for tab in Tab::all() {
            assert!(load_logo(&ctx, tab).is_some(), "{} logo", tab.name());
        }
    }
}
