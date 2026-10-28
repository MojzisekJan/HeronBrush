//! Writing the map out for a game engine.
//!
//! The building is [`scene_export`]; this is what the editor hands it and
//! what it does with the files that come back. The document is written out as `.map` text
//! and parsed there, which is the one format both halves already speak -- a shared set of
//! structures would be two things to keep in step instead of one, and the map file is what
//! the export is defined against anyway.
//!
//! Each engine gets what it can actually load. Godot takes a `.tscn` scene with `.obj`
//! meshes beside it; Defold a collection and its component files; three.js, Bevy and
//! raylib take the engine-neutral level -- JSON for the first two, flat text for raylib --
//! with the meshes as `.glb`, which all three read natively. The plain `.obj` and `.glb`
//! targets are for taking the geometry somewhere else entirely.

use std::path::{Path, PathBuf};

use scene_export::builder::{self, BuildOptions, LevelOptions};
use scene_export::settings::Settings;
use scene_export::writer;

use crate::document::Document;

#[path = "gltf_export.rs"]
mod gltf_export;
#[path = "paint_export.rs"]
pub mod paint_export;

/// Which engine the level is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Target {
    /// A `.tscn` scene and `.obj` meshes.
    #[default]
    Godot,
    /// A `.collection` and the component files it names, with `.glb` meshes.
    Defold,
    /// The level as JSON, with `.glb` meshes, laid out for `GLTFLoader`.
    ThreeJs,
    /// The level as JSON with `.glb` meshes, a Babylon.js runtime module and a playable page.
    Babylon,
    /// The level as JSON with `.glb` meshes, a PlayCanvas runtime module and a playable page.
    PlayCanvas,
    /// The level as JSON with `.glb` meshes, which Bevy's asset loader reads.
    Bevy,
    /// The level as flat text, one keyword to a line, with `.glb` meshes.
    Raylib,
    /// `.obj` meshes and a generated C# Editor script that builds the level.
    Unity,
    /// `.glb` meshes and a generated Python script run inside the Unreal editor.
    Unreal,
    /// `.glb` meshes and a generated C# editor script that builds the level in Flax.
    Flax,
    /// GLB assets, complete JSON and a project-configurable Stride C# importer.
    Stride,
    /// The level as JSON with `.glb` meshes, and a Cocos Creator component that reads it.
    Cocos,
    /// A GDevelop 5 scene of 3D models in the project's `game.json`, with physics,
    /// variables, lights, sky and an event that plays animation and triggers.
    GDevelop,
    /// Nothing but the geometry, as one `.obj` per solid.
    Obj,
    /// A portable scene with external binary data and PNG textures.
    Gltf,
    /// A portable scene with all resources in one binary file.
    Glb,
}

impl Target {
    /// Every target, in the order the chooser lists them.
    pub const ALL: [Target; 16] = [
        Target::Godot,
        Target::Defold,
        Target::ThreeJs,
        Target::Babylon,
        Target::PlayCanvas,
        Target::Bevy,
        Target::Raylib,
        Target::Unity,
        Target::Unreal,
        Target::Flax,
        Target::Stride,
        Target::Cocos,
        Target::GDevelop,
        Target::Obj,
        Target::Gltf,
        Target::Glb,
    ];

    /// The files that say "an engine project begins in this directory".
    ///
    /// Walked up from wherever a level is being written, to find the folder textures are
    /// named relative to. Each engine's own: `project.godot`, Defold's `game.project`,
    /// the `Cargo.toml` of a Bevy game, a web project's `package.json` or the page
    /// itself, Unity's `ProjectSettings` folder, Unreal's `.uproject`. A name ending in
    /// `/` is a directory rather than a file.
    ///
    /// raylib has no marker of its own -- a raylib game is a C program with whatever
    /// layout its author chose -- so it takes the ones a C project usually has. Where
    /// the guess is wrong, the export window's project folder is there to say so.
    pub fn project_markers(self) -> &'static [&'static str] {
        match self {
            Target::Godot => &["project.godot"],
            Target::Defold => &["game.project"],
            Target::Bevy => &["Cargo.toml"],
            Target::ThreeJs | Target::Babylon | Target::PlayCanvas => {
                &["package.json", "index.html"]
            }
            Target::Unity => &["ProjectSettings/", "Assets/"],
            Target::Unreal => &[".uproject"],
            // A Flax project keeps a `<name>.flaxproj` beside its Content folder; a
            // Cocos Creator project keeps its settings in `project.json`.
            Target::Flax => &["Content/", ".flaxproj"],
            Target::Stride => &[".sdpkg", ".sln"],
            Target::Cocos => &["project.json", "assets/"],
            Target::GDevelop => &["game.json"],
            Target::Raylib => &["Makefile", "CMakeLists.txt"],
            // A plain `.obj` is written face by face and looks nothing up.
            Target::Obj | Target::Gltf | Target::Glb => &[],
        }
    }

    /// Whether the export writes the 3D view's sky, sun and fog for this engine.
    pub fn exports_sky(self) -> bool {
        matches!(
            self,
            Target::Godot
                | Target::Unity
                | Target::Flax
                | Target::ThreeJs
                | Target::Babylon
                | Target::PlayCanvas
                | Target::Bevy
                | Target::Raylib
                | Target::Defold
                | Target::Stride
                | Target::Cocos
                | Target::GDevelop
        )
    }

    /// The name shown in the chooser.
    pub fn name(self) -> &'static str {
        match self {
            Target::Godot => "Godot",
            Target::Defold => "Defold",
            Target::ThreeJs => "three.js",
            Target::Babylon => "Babylon.js",
            Target::PlayCanvas => "PlayCanvas",
            Target::Bevy => "Bevy",
            Target::Raylib => "raylib",
            Target::Unity => "Unity",
            Target::Unreal => "Unreal Engine",
            Target::Flax => "Flax Engine",
            Target::Stride => "Stride",
            Target::Cocos => "Cocos Creator",
            Target::GDevelop => "GDevelop",
            Target::Obj => "Wavefront .obj",
            Target::Gltf => "glTF .gltf",
            Target::Glb => "GLB .glb",
        }
    }
}

fn sky_default() -> bool {
    true
}

/// What to write, and how.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Per-class project build instructions and typed defaults for this target.
    #[serde(default)]
    pub entities: std::collections::BTreeMap<String, crate::export_bindings::Rule>,
    /// Which engine the level is for.
    pub target: Target,
    /// The level's name, which names the files.
    pub name: String,
    /// Map units per engine unit.
    pub scale: f64,
    /// Write collision geometry.
    pub collision: bool,
    /// Write the 3D view's sky, sun and fog for the engines that take them.
    #[serde(default = "sky_default")]
    pub sky: bool,
    /// Throw away faces buried inside the geometry.
    pub cull_hidden: bool,
    /// Join neighbouring faces that share a plane and a material.
    pub merge_faces: bool,
    #[serde(default)]
    pub static_cells: bool,
    /// Join neighbouring brushes into single collision shapes.
    pub merge_collision: bool,
    /// The `res://` directory Godot texture images are looked up in. Empty for the
    /// builder's default (`res://textures`).
    pub texture_dir: String,
    /// The `res://` directory searched for hand-made materials. Empty for none.
    pub material_dir: String,
    /// The tool-material names -- what marks a face as collision-only, invisible, or the
    /// origin. Empty keeps the chosen game's answer, or the builder's.
    pub clip_texture: String,
    /// See [`Options::clip_texture`].
    pub skip_texture: String,
    /// See [`Options::clip_texture`].
    pub origin_texture: String,
    /// Defold: the resource directory the exported files live under, e.g. `/main/arena`.
    /// Empty for `/<name>`.
    pub asset_dir: String,
    /// Unreal/Flax content path; empty uses the level name.
    #[serde(default)]
    pub content_dir: String,
    /// Defold model material resource and colour sampler.
    #[serde(default)]
    pub model_material: String,
    #[serde(default)]
    pub model_sampler: String,
    /// Where this engine's level was last written, so the next export can go there
    /// without asking again. Empty until one has been written.
    pub directory: String,
    /// The folder the engine's project begins at, which is what turns a texture name
    /// into a file on disk. Empty for "find it", which walks up from wherever the level
    /// is being written looking for that engine's own marker file.
    pub project_dir: String,
    /// Physics layer, 1 to 32, for the areas a trigger becomes. On the level's own layer
    /// a trigger stops every ray the game traces; 0 leaves it where the engine puts it.
    pub trigger_layer: u32,
    /// Ask the engine's importer to unwrap a second UV set, which baked light needs.
    pub lightmap_uv2: bool,
    /// How big one lightmap texel is, in engine units.
    pub lightmap_texel_size: f64,
    /// The entity property whose value names the node, such as `targetname`. Empty for
    /// the classname.
    pub name_property: String,
    /// Rebuild the map's layers and groups as parent nodes, rather than putting every
    /// entity straight under the root.
    pub group_hierarchy: bool,
    /// Reverse the triangle winding in the written meshes. For an importer that does not
    /// flip it on the way in.
    pub flip_faces: bool,
    /// Write the V texture coordinate as it is rather than flipped. For an importer that
    /// measures V from the same end the map does.
    pub flip_uv_v: bool,
    /// Write this engine's level again whenever the map is saved.
    ///
    /// Saving rewrites the level in the configured engine project. Engines that watch
    /// their project directory can then reimport it.
    pub export_on_save: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            target: Target::default(),
            name: "level".to_string(),
            scale: 32.0,
            collision: true,
            sky: true,
            cull_hidden: true,
            merge_faces: true,
            static_cells: false,
            merge_collision: true,
            texture_dir: String::new(),
            material_dir: String::new(),
            clip_texture: String::new(),
            skip_texture: String::new(),
            origin_texture: String::new(),
            asset_dir: String::new(),
            content_dir: String::new(),
            model_material: String::new(),
            model_sampler: String::new(),
            project_dir: String::new(),
            // The builder's own defaults, so an engine nobody has set up behaves the way
            // `scene_export::Settings::default` says.
            trigger_layer: 3,
            lightmap_uv2: false,
            lightmap_texel_size: 0.2,
            name_property: String::new(),
            group_hierarchy: true,
            flip_faces: false,
            flip_uv_v: false,
            directory: String::new(),
            export_on_save: false,
        }
    }
}

impl Options {
    /// The settings the builder works from.
    ///
    /// The chosen game's own tool materials are handed over where it names them: a face
    /// wearing the clip material is collision and nothing else, and one wearing skip is
    /// neither. Which materials those are is the game's to say, which is what its
    /// configuration is for.
    pub fn settings(&self, directory: &Path) -> Settings {
        let game = crate::games::current();
        let mut settings = Settings {
            scale_factor: 1.0 / self.scale.max(1.0),
            cull_hidden: self.cull_hidden,
            merge_faces: self.merge_faces,
            static_cell_size: if self.static_cells { 1024.0 } else { 0.0 },
            merge_collision: self.merge_collision,
            ..Settings::default()
        };
        if let Some(game) = &game {
            if let Some(clip) = &game.clip_material {
                settings.clip_texture = clip.clone();
            }
            if let Some(skip) = &game.skip_material {
                settings.skip_texture = skip.clone();
            }
        }
        // What the export dialog names beats the game's answer: it is set per engine,
        // for the project actually being written into.
        if !self.texture_dir.is_empty() {
            settings.texture_dir = self.texture_dir.clone();
        }
        if !self.material_dir.is_empty() {
            settings.material_dir = Some(self.material_dir.clone());
        }
        if !self.clip_texture.is_empty() {
            settings.clip_texture = self.clip_texture.clone();
        }
        if !self.skip_texture.is_empty() {
            settings.skip_texture = self.skip_texture.clone();
        }
        if !self.origin_texture.is_empty() {
            settings.origin_texture = self.origin_texture.clone();
        }
        settings.trigger_layer = self.trigger_layer;
        settings.lightmap_uv2 = self.lightmap_uv2;
        settings.lightmap_texel_size = self.lightmap_texel_size;
        settings.name_property =
            (!self.name_property.is_empty()).then(|| self.name_property.clone());
        settings.group_hierarchy = self.group_hierarchy;
        settings.flip_faces = self.flip_faces;
        settings.flip_uv_v = self.flip_uv_v;
        // Where the project the export is being written into begins. Without it no
        // texture can be looked up at all: a material is never written and every UV
        // divides by the assumed texture size instead of the real one, so a 64 pixel
        // texture on a 64 unit floor repeats twice rather than once. The builder warns
        // about this and the warning was going unread -- which is how it survived for
        // Godot, and how every other engine went on having no marker to look for at all.
        settings.project_root = if self.project_dir.is_empty() {
            scene_export::materials::find_root_marked_by(directory, self.target.project_markers())
        } else {
            Some(PathBuf::from(&self.project_dir))
        };
        settings
    }
}

/// What an export wrote, and anything it wants to say about it.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The files written, in the order they were written.
    pub files: Vec<PathBuf>,
    /// What the builder had to say: a texture it could not find, a brush it dropped.
    pub warnings: Vec<String>,
    /// How much geometry came out.
    pub triangles: usize,
    /// How many faces the optimisations saved.
    pub saved_faces: usize,
}

/// Why an export could not be written.
#[derive(Debug)]
pub enum Error {
    /// The document could not be turned into a map to build from.
    Map(String),
    /// A file could not be written.
    Write(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Map(why) => write!(f, "the map could not be built: {why}"),
            Error::Write(path, why) => write!(f, "{} could not be written: {why}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

/// Writes the document into `directory` for the engine the options name.
pub fn export(document: &Document, directory: &Path, options: &Options) -> Result<Report, Error> {
    std::fs::create_dir_all(directory).map_err(|why| Error::Write(directory.to_path_buf(), why))?;
    let (mut report, files) = export_files(document, directory, options)?;

    for (path, contents) in files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|why| Error::Write(parent.to_path_buf(), why))?;
        }
        let written = match &contents {
            Contents::Text(text) => std::fs::write(&path, text),
            Contents::Bytes(bytes) => std::fs::write(&path, bytes),
        };
        written.map_err(|why| Error::Write(path.clone(), why))?;
        report.files.push(path);
    }

    Ok(report)
}

/// A warning when entity property `values` name files by Godot's `res://` paths, which an
/// export for `target`, another engine, passes on but that engine cannot find. Lists a
/// few of them.
fn godot_paths_warning<'a>(
    values: impl Iterator<Item = &'a scene_export::level::Value>,
    target: Target,
) -> Option<String> {
    const LISTED: usize = 5;
    let paths: std::collections::BTreeSet<&str> = values
        .filter_map(|value| match value {
            scene_export::level::Value::String(text) if text.starts_with("res://") => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    if paths.is_empty() {
        return None;
    }
    let mut listed: Vec<&str> = paths.iter().copied().take(LISTED).collect();
    if paths.len() > LISTED {
        listed.push("…");
    }
    Some(format!(
        "{} entity properties name Godot project files (res://), which {} cannot load: {}",
        paths.len(),
        target.name(),
        listed.join(", ")
    ))
}

/// What [`export`] would write into `directory`, as paths and contents, without writing
/// any of it: the web build zips them for a download.
pub fn export_files(
    document: &Document,
    directory: &Path,
    options: &Options,
) -> Result<(Report, Vec<(PathBuf, Contents)>), Error> {
    if !options.scale.is_finite() || options.scale <= 0. {
        return Err(Error::Map(
            "Units per metre must be finite and greater than zero".into(),
        ));
    }
    build(document, directory, options, false)
}

/// Builds the level and says what is wrong with it, writing nothing.
///
/// Runs the same build as an export, with the geometry checks switched on --
/// they cost a pass over every triangle of the finished mesh, which is why an ordinary
/// export leaves them off. What they catch is what only shows up once the brushes have
/// become a mesh: a surface with triangles wound both ways, which draws inside out in
/// patches; triangles with no area; and a corner sitting in the middle of a neighbour's
/// edge, which is the hairline crack a T-junction draws as. The report's file list is
/// empty, since nothing was written.
///
/// `directory` is still wanted: it is what finds the engine project, and without it no
/// texture can be looked up, so a check would report the sizes wrong.
pub fn check(document: &Document, directory: &Path, options: &Options) -> Result<Report, Error> {
    Ok(build(document, directory, options, true)?.0)
}

/// The shared half: everything up to the point where files exist but are not on disk.
fn build(
    document: &Document,
    directory: &Path,
    options: &Options,
    checking: bool,
) -> Result<(Report, Vec<(PathBuf, Contents)>), Error> {
    for (_, entity, layer) in map_model::node::point_entities_by_layer(document.tree_world()) {
        if !document.tree_world().layers[layer].omit_from_export
            && entity.property("_heron_model").is_some()
            && document.imported_model(entity).is_none()
        {
            return Err(Error::Map(
                "Imported model is unavailable or has invalid transforms".into(),
            ));
        }
    }
    let has_animation = map_model::node::entities_with_ids(document.tree_world())
        .iter()
        .any(|(_, entity)| entity.property("animation_keys").is_some());
    const ANIMATION_WARNING: &str = "This export target does not generate a keyframe playback controller. Godot, raylib, Defold, Bevy, Three.js, Babylon.js, PlayCanvas, Flax, Unity, Stride, GDevelop and Cocos Creator exports support keyframe playback.";
    if matches!(options.target, Target::Gltf | Target::Glb) {
        let (mut report, files) = gltf_export::build(document, directory, options)?;
        if has_animation {
            report.warnings.push(ANIMATION_WARNING.into());
        }
        return Ok((report, files));
    }
    // The document as a `.map`, which is what the builder reads. Writing it out and
    // parsing it back is a round trip the map I/O tests already cover.
    let mut settings = options.settings(directory);
    let (text, mut paint_files) =
        paint_export::prepare(document, directory, options, &mut settings)?;
    let mut map = scene_export::map::parse(&text, &options.name)
        .map_err(|why| Error::Map(why.to_string()))?;

    let painted_materials = settings
        .generated_materials
        .keys()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    crate::imported_export::prepare(document, &mut map, &mut settings)?;
    // Imported skins are registered after face paint. Package them beside the
    // other generated images and use the same project-relative resource paths.
    let root = settings.project_root.as_deref().unwrap_or(directory);
    for (name, material) in &mut settings.generated_materials {
        if painted_materials.contains(name) {
            continue;
        }
        let path = directory
            .join(format!("{}_paint", options.name))
            .join(format!("{name}.png"));
        material.path = scene_export::materials::fs_to_res(root, &path).ok_or_else(|| {
            Error::Map("export directory must be inside the chosen engine project".into())
        })?;
        paint_files.push((path, Contents::Bytes(material.png.clone())));
        if let Some(pbr) = &mut material.pbr {
            for image in &mut pbr.images {
                let path = directory
                    .join(format!("{}_paint", options.name))
                    .join(format!("{name}_{}.png", image.slot));
                image.path = scene_export::materials::fs_to_res(root, &path)
                    .ok_or_else(|| Error::Map("PBR image is outside project".into()))?;
                paint_files.push((
                    path,
                    Contents::Bytes(if options.target == Target::Godot {
                        image.godot_png.as_ref().unwrap_or(&image.png).to_vec()
                    } else {
                        image.png.to_vec()
                    }),
                ));
            }
        }
    }
    let (mut classes, class_warnings) = entity_classes(document);
    crate::export_bindings::apply(&map, &mut classes, &options.entities).map_err(Error::Map)?;
    settings.check_geometry = checking;

    let mut report = Report::default();
    if !matches!(
        options.target,
        Target::Godot
            | Target::Raylib
            | Target::Defold
            | Target::Bevy
            | Target::Unity
            | Target::ThreeJs
            | Target::Babylon
            | Target::PlayCanvas
            | Target::Flax
            | Target::Stride
            | Target::GDevelop
            | Target::Cocos
    ) && has_animation
    {
        report.warnings.push(ANIMATION_WARNING.into());
    }
    let mut files: Vec<(PathBuf, Contents)> = paint_files;

    match options.target {
        Target::Godot => {
            let mut built = builder::build(
                &map,
                &classes,
                &settings,
                &BuildOptions {
                    scene_name: options.name.clone(),
                    mesh_dir: options.name.clone(),
                },
            );
            crate::godot_animation::attach(
                &mut built.scene,
                settings.scale_factor,
                &mut built.warnings,
            );
            // The sky reads only worldspawn, which the scene builder does not keep as a
            // level entity, so it is handed over on its own.
            let mut world = scene_export::level::Entity {
                name: "worldspawn".into(),
                classname: "worldspawn".into(),
                ..Default::default()
            };
            if let Some(entity) = map.entities.first().filter(|e| {
                e.properties
                    .get("classname")
                    .is_some_and(|c| c == "worldspawn")
            }) {
                for (key, value) in &entity.properties {
                    world.properties.insert(
                        key.clone(),
                        scene_export::level::Value::String(value.clone()),
                    );
                }
            }
            let sky = map_sky(
                document,
                &scene_export::level::Level {
                    format: 2,
                    generator: String::new(),
                    name: options.name.clone(),
                    scale: options.scale,
                    textures: Vec::new(),
                    entities: vec![world],
                },
            );
            let panorama = directory.join(&options.name).join("sky.hdr");
            let root = settings.project_root.as_deref().unwrap_or(directory);
            if let Some(res) =
                scene_export::materials::fs_to_res(root, &panorama).filter(|_| options.sky)
            {
                writer::godot_sky::attach(&mut built.scene, &sky, &res);
                files.push((panorama, Contents::Bytes(writer::godot_sky::panorama(&sky))));
            }
            if writer::godot_water::used(&built.scene) {
                files.push((
                    directory
                        .join(&options.name)
                        .join(writer::godot_water::VIEW_FILE),
                    Contents::Text(writer::godot_water::VIEW_SCRIPT.into()),
                ));
            }
            files.push((
                directory.join(format!("{}.tscn", options.name)),
                Contents::Text(writer::tscn::write(&built.scene)),
            ));
            for mesh in &built.meshes {
                let path = directory.join(&options.name).join(&mesh.file_name);
                if let Some(import) = godot_import_defaults(&path) {
                    files.push((import.0, Contents::Text(import.1)));
                }
                files.push((path, Contents::Text(mesh.contents.clone())));
            }
            report.warnings = built.warnings.clone();
            report.triangles = built.stats.triangles;
            report.saved_faces =
                built.stats.buried_faces + built.stats.pressed_faces + built.stats.merged_faces;
        }

        Target::Obj => {
            // The editor's own writer rather than the builder's: one `.obj` with an
            // `.mtl` beside it, materials and all, which is what a modelling tool wants.
            // The engine targets want a mesh per solid instead, which is what the builder
            // gives them.
            let path = directory.join(format!("{}.obj", options.name));
            if checking {
                // This target does not go through the builder, so there is no mesh for
                // the geometry checks to look at: a face becomes a polygon and that is
                // the whole of it.
                report.warnings.push(
                    "a plain .obj is written face by face, so there is nothing here for \
                     the geometry checks to find"
                        .to_string(),
                );
                return Ok((report, Vec::new()));
            }
            let map = scene_export::map::parse(&document.to_map_text(), &options.name)
                .map_err(|error| Error::Map(error.to_string()))?;
            let (mut classes, class_warnings) = entity_classes(document);
            crate::export_bindings::apply(&map, &mut classes, &options.entities)
                .map_err(Error::Map)?;
            report.warnings.extend(class_warnings);
            let (written, obj_files) = crate::obj_export::files_scaled(
                document,
                &path,
                crate::obj_export::ObjOptions {
                    selection_only: false,
                    y_up: true,
                },
                1. / options.scale,
                &settings,
                &classes,
            );
            report.triangles = written.triangles;
            return Ok((
                report,
                obj_files
                    .into_iter()
                    .map(|(path, contents)| (path, Contents::Bytes(contents)))
                    .collect(),
            ));
        }

        target => {
            // Everything else is built once, as the engine-neutral level, and written by
            // the writer that engine reads.
            let level_options = LevelOptions {
                name: options.name.clone(),
                mesh_dir: options.name.clone(),
                collision: options.collision,
                // Where the collision goes, and it is one place or the other rather than
                // both. A loader that reads it from the level file does not want it in
                // the mesh as well: the triangles are then written twice, once as text
                // and once as glTF, and the mesh a three.js page downloads is roughly
                // double what it draws. Defold and raylib cannot follow a collision mesh
                // inside a `.glb` at all, and three.js is handed the triangles because a
                // page has a loader for the level file and not for a mesh it never
                // draws; Bevy reads the glTF mesh directly, so it takes neither the
                // triangles nor a copy of them. Flax exposes collision points in JSON
                // so game callbacks can create bodies without decoding a GLB.
                inline_collision: matches!(
                    target,
                    Target::Defold
                        | Target::Raylib
                        | Target::ThreeJs
                        | Target::Babylon
                        | Target::PlayCanvas
                        | Target::Cocos
                        | Target::GDevelop
                        | Target::Unity
                        | Target::Flax
                        | Target::Stride
                ),
                // An `.obj` cannot carry collision, so Unity's rides beside the level.
                // Writing one for an engine that is reading the triangles instead also
                // hands a trigger volume a mesh, and a loader following the level format
                // then draws the trigger.
                mesh_collision: !matches!(
                    target,
                    Target::Defold
                        | Target::Raylib
                        | Target::Unity
                        | Target::ThreeJs
                        | Target::Babylon
                        | Target::PlayCanvas
                        | Target::Cocos
                        | Target::Flax
                        | Target::Stride
                        | Target::GDevelop
                ),
                convex_collision: matches!(target, Target::Defold),
                mesh_format: if target == Target::Unity {
                    scene_export::builder::MeshFormat::Obj
                } else {
                    scene_export::builder::MeshFormat::Glb
                },
            };
            let mut built = builder::build_level(&map, &classes, &settings, &level_options);
            if matches!(
                target,
                Target::Defold
                    | Target::Bevy
                    | Target::Unity
                    | Target::ThreeJs
                    | Target::Babylon
                    | Target::PlayCanvas
                    | Target::Flax
                    | Target::Stride
                    | Target::GDevelop
                    | Target::Cocos
            ) {
                crate::portable_animation::prepare(&mut built.level, &mut report.warnings);
            }

            crate::portable_animation::prepare_triggers(&mut built.level);
            report.warnings.extend(godot_paths_warning(
                built
                    .level
                    .entities
                    .iter()
                    .flat_map(|entity| entity.properties.values()),
                target,
            ));
            let unity_mtl = format!("{}_paint.mtl", options.name);
            for mesh in &built.meshes {
                let bytes = if target == Target::Unity && mesh.file_name.ends_with(".obj") {
                    let mut bytes = if settings.generated_materials.is_empty() {
                        Vec::new()
                    } else {
                        format!("mtllib {unity_mtl}\n").into_bytes()
                    };
                    bytes.extend_from_slice(
                        String::from_utf8_lossy(&mesh.bytes)
                            .replace("\no ", "\ng ")
                            .as_bytes(),
                    );
                    bytes
                } else {
                    mesh.bytes.clone()
                };
                files.push((
                    directory.join(&options.name).join(&mesh.file_name),
                    Contents::Bytes(bytes),
                ));
            }
            if target == Target::Unity && !settings.generated_materials.is_empty() {
                let mtl = paint_mtl(&settings, &format!("../{}_paint", options.name), true);
                files.push((
                    directory.join(&options.name).join(unity_mtl),
                    Contents::Text(mtl),
                ));
                // The importer turns these into the materials' normal maps; they are
                // in the OpenGL convention Unity uses.
                for name in unity_normal_maps(&settings) {
                    files.push((
                        directory
                            .join(format!("{}_paint", options.name))
                            .join(format!("{name}_normal.png")),
                        Contents::Bytes(settings.normal_maps[name.as_str()].to_vec()),
                    ));
                }
            }

            match target {
                Target::ThreeJs | Target::Babylon | Target::PlayCanvas | Target::Bevy => {
                    if target == Target::Bevy {
                        files.push((directory.join("heron_lights.rs"), Contents::Text(include_str!("../../scene_export/src/writer/bevy_lights.rs.txt").into())));
                        files.push((
                            directory.join("heron_water_render.rs"),
                            Contents::Text(
                                include_str!(
                                    "../../scene_export/src/writer/bevy_water_render.rs.txt"
                                )
                                .into(),
                            ),
                        ));
                        files.push((
                            directory.join("heron_water.rs"),
                            Contents::Text(
                                include_str!("../../scene_export/src/writer/bevy_water.rs.txt")
                                    .into(),
                            ),
                        ));
                        files.push((
                            directory.join("heron_entities.rs"),
                            Contents::Text(
                                include_str!("../../scene_export/src/writer/bevy_entities.rs.txt")
                                    .into(),
                            ),
                        ));
                        files.push((
                            directory.join("heron_animation.rs"),
                            Contents::Text(
                                include_str!("../../scene_export/src/writer/bevy_animation.rs.txt")
                                    .into(),
                            ),
                        ));
                        files.push((directory.join("heron_animation_bevy.md"),Contents::Text("# Bevy animation\n\nCopy heron_animation.rs into src/, declare mod heron_animation, and add HeronAnimationPlugin to your Bevy 0.19 app. Requires serde with derive and serde_json. For each exported entity, read properties._heron_animation.value (a JSON string in the tagged property map) and attach HeronAnimation::from_json(data)? to the SAME root as the rest Transform and its mesh/collision children. The plugin updates that Transform in Update before transform propagation. The root must use the exported parent coordinate frame; keep scene-wide transforms on a common parent.\n\nplay(name), pause(), resume(), seek(seconds), stop() control playback. Autoplay respects start_on. stop restores the rest pose. For Avian with its default FixedPostUpdate schedule, add HeronAnimationFixedPlugin instead and run support carrying after HeronAnimationSet in FixedPreUpdate. Use a kinematic body, never a dynamic body. Other physics adapters must synchronize animation with their simulation schedule. Exported func_button, trigger_once and trigger_multiple links are stored in _heron_trigger. Attach HeronAnimationName(entity.name) to animated roots, HeronAnimationTrigger::from_json(data)? to trigger roots, and HeronActivator to the player root. The plugin tests entry into the exported convex volumes. Call trigger.activate() from interaction code for button presses. Once-only behavior and wait cooldowns are shared with the other runtimes.\n".into())));
                    }
                    let sky = options.sky.then(|| map_sky(document, &built.level));
                    let mut level_json = if matches!(target, Target::Babylon | Target::PlayCanvas) {
                        writer::babylon::write(&built.level, &built.collision)
                    } else {
                        writer::three::write(&built.level, &built.collision)
                    };
                    if let Some(sky) = &sky {
                        let mut value: serde_json::Value =
                            serde_json::from_str(&level_json).expect("level JSON");
                        if matches!(
                            target,
                            Target::ThreeJs | Target::Babylon | Target::PlayCanvas
                        ) {
                            let panorama = format!("{}/sky.hdr", options.name);
                            value["environment"] = writer::three::environment(sky, Some(&panorama));
                            files.push((
                                directory.join(panorama),
                                Contents::Bytes(sky.panorama_hdr(
                                    1024,
                                    512,
                                    match target {
                                        Target::Babylon => writer::sky::babylon_panorama_direction,
                                        Target::PlayCanvas => {
                                            writer::sky::playcanvas_panorama_direction
                                        }
                                        _ => writer::sky::three_panorama_direction,
                                    },
                                )),
                            ));
                        } else {
                            // Bevy reads KTX2 with its 3D features; the cube is named relative to
                            // the level file, like the meshes.
                            let cubemap = format!("{}/sky.ktx2", options.name);
                            let mut environment = writer::three::environment(sky, None);
                            environment["cubemap"] = serde_json::json!(cubemap);
                            value["environment"] = environment;
                            // Bevy samples its cubes with Z negated (skybox.wgsl), so the faces hold the
                            // mirrored sky.
                            files.push((
                                directory.join(cubemap),
                                Contents::Bytes(sky.cube_ktx2(256, |face, u, v| {
                                    let d = writer::sky::cube_direction(face, u, v);
                                    [d[0], d[1], -d[2]]
                                })),
                            ));
                            files.push((
                                directory.join("heron_sky.rs"),
                                Contents::Text(
                                    include_str!("../../scene_export/src/writer/bevy_sky.rs.txt")
                                        .into(),
                                ),
                            ));
                        }
                        level_json = serde_json::to_string_pretty(&value).expect("level JSON");
                    }
                    files.push((
                        directory.join(format!("{}.json", options.name)),
                        Contents::Text(level_json),
                    ));
                    if target == Target::Babylon {
                        files.push((
                            directory.join("heron_babylon.js"),
                            Contents::Text(writer::babylon::RUNTIME.into()),
                        ));
                        files.push((
                            directory.join(format!("{}.html", options.name)),
                            Contents::Text(writer::babylon::write_html(
                                &built.level,
                                &built.collision,
                                &built.meshes,
                                sky.as_ref(),
                            )),
                        ));
                    }
                    if target == Target::PlayCanvas {
                        files.push((
                            directory.join("heron_playcanvas.js"),
                            Contents::Text(writer::playcanvas::RUNTIME.into()),
                        ));
                        files.push((
                            directory.join(format!("{}.html", options.name)),
                            Contents::Text(writer::playcanvas::write_html(
                                &built.level,
                                &built.collision,
                                &built.meshes,
                                sky.as_ref(),
                            )),
                        ));
                    }
                    if target == Target::ThreeJs {
                        files.push((directory.join("heron_lights.js"), Contents::Text(include_str!("../../scene_export/src/writer/three_lights.js").into())));
                        files.push((
                            directory.join("heron_water.js"),
                            Contents::Text(
                                include_str!("../../scene_export/src/writer/three_water.js").into(),
                            ),
                        ));
                        files.push((
                            directory.join("heron_animation.js"),
                            Contents::Text(
                                include_str!("../../scene_export/src/writer/three_animation.js")
                                    .into(),
                            ),
                        ));
                        let mut textures = std::collections::BTreeMap::new();
                        for texture in &built.level.textures {
                            let Some(image) = &texture.image else {
                                continue;
                            };
                            let path = settings
                                .project_root
                                .as_deref()
                                .unwrap_or(directory)
                                .join(image);
                            let mime = match path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .unwrap_or("")
                                .to_ascii_lowercase()
                                .as_str()
                            {
                                "png" => "image/png",
                                "jpg" | "jpeg" => "image/jpeg",
                                "webp" => "image/webp",
                                "gif" => "image/gif",
                                _ => {
                                    report.warnings.push(format!(
                                        "HTML preview: unsupported texture format: {}",
                                        path.display()
                                    ));
                                    continue;
                                }
                            };
                            let bytes = settings
                                .generated_materials
                                .get(&texture.name)
                                .map(|image| Ok(image.png.clone()))
                                .unwrap_or_else(|| crate::files::read(&path));
                            match bytes {
                                Ok(bytes) => {
                                    textures.insert(
                                        texture.name.clone(),
                                        format!(
                                            "data:{mime};base64,{}",
                                            scene_export::encode_base64(&bytes)
                                        ),
                                    );
                                }
                                Err(error) => report
                                    .warnings
                                    .push(format!("HTML preview: {}: {error}", path.display())),
                            }
                        }
                        files.push((
                            directory.join(format!("{}.html", options.name)),
                            Contents::Text(writer::three::write_html(
                                &built.level,
                                &built.collision,
                                &built.meshes,
                                &textures,
                                sky.as_ref(),
                            )),
                        ));
                    }
                }
                Target::Raylib => {
                    let header = format!("{}_animation.h", options.name);
                    files.push((
                        directory.join(&header),
                        Contents::Text(crate::raylib_animation::write(
                            &built.level,
                            &mut report.warnings,
                        )),
                    ));
                    // raylib draws a texture's colours as they are, so its sky is baked as the
                    // view shows it, exposure and tone map included.
                    let sky = options.sky.then(|| map_sky(document, &built.level));
                    let sky_path = format!("{}/sky.png", options.name);
                    if let Some(sky) = &sky {
                        files.push((
                            directory.join(&sky_path),
                            Contents::Bytes(sky.display_strip(256, writer::sky::cube_direction)),
                        ));
                    }
                    let lighting_header = format!("{}_lighting.h",options.name);
                    files.push((directory.join(&lighting_header),Contents::Text(writer::raylib::lighting(&built.level,sky.as_ref()))));
                    let water = crate::raylib_water::write(&built.level);
                    let water_header = format!("{}_water.h", options.name);
                    if let Some(text) = &water {
                        files.push((directory.join(&water_header), Contents::Text(text.clone())));
                    }
                    files.push((
                        directory.join(format!("{}_preview.c", options.name)),
                        Contents::Text(crate::raylib_animation::preview(
                            &built.level,
                            &header,
                            sky.as_ref().map(|_| sky_path.as_str()),
                            water.as_ref().map(|_| water_header.as_str()),
                            Some(&lighting_header),
                        )),
                    ));
                    files.push((directory.join(format!("{}_raylib.md",options.name)),Contents::Text(format!(
                        "# Raylib animation export\n\nBuild with raylib 5+ installed:\n\n```sh\ncc -std=c99 -O2 \"{}_preview.c\" -o preview $(pkg-config --cflags --libs raylib) -lm\n./preview\n```\n\nRun from this exported directory. SPACE pauses, R plays/restarts all default clips (including disabled autoplay), N selects the next named clip. This is a free-camera visual preview, without gameplay or physics.\n\nInclude `{header}` in your game. HbInit initializes each HbPlayer and respects start_on. HbUpdate advances seconds. HbPlay selects a named clip; HbSeek, HbPause, HbResume, HbStop control playback. HbStop restores rest geometry; once holds its final pose. hb_animations[a].entity is the entity index in the .level/.json. HbMatrix yields the animation transform to apply AFTER the entity rest transform. HbPoint transforms WORLD engine-space vertices; use the same transform for collision vertices/hulls and update your physics body's pose/broadphase each frame. Never transform the previous frame's vertices again.\n\nLinear, Hold and Bezier curves, pivots, once/loop/ping-pong, and named clips are included. Group members in one animated brush entity share playback. Exported func_button, trigger_once and trigger_multiple targets are included. Keep HbTriggerState states[HB_TRIGGER_COUNT+1] = {{0}} and call HbCheckTriggersSpan(states, players, feet, height, seconds) each frame for a standing player, so a volume that starts above the floor still catches it, or HbCheckTriggers(states, players, point, seconds) for a single point. The preview uses its camera position. HbActivateTrigger handles explicit button presses, wait cooldowns and trigger_once. HbActivateAnimation toggles named open/close clips or restarts the selected clip.\n\nWith Sky and sun on, `{name}/sky.png` is the map's sky as the editor's view shows it, tone mapped, in raylib's CUBEMAP_LAYOUT_LINE_VERTICAL for LoadTextureCubemap; sample it with the world direction, as the preview does. The level JSON's `environment` holds the sun's direction (to_sun), colour and intensity, the ambient intensity and any fog.\n\nWith water volumes, `{name}_water.h` holds them as C data. HbWaterFind(point, &depth) gives the volume a world point is in (higher water_priority wins, then the deeper) and HbWaterSample its depth below the surface; HbWaterVelocity(water, velocity, walk, rise, dive, dt, out) is the swim velocity. The volumes' collision is marked sensor. Included after raylib.h, HbWaterLoadShader, HbWaterView and HbWaterDrawModel draw the baseline surface (waving, see-through, after the opaque models) and fog the view under water, as the preview does.\n",options.name,name=options.name))));
                    files.push((
                        directory.join(format!("{}.level", options.name)),
                        Contents::Text(writer::raylib::write(&built.level, &built.collision)),
                    ));
                    // The compact C-friendly format flattens arrays and newlines.
                    // Keep a lossless sidecar for custom game integrations.
                    let mut level_json = writer::three::write(&built.level, &built.collision);
                    if let Some(sky) = &sky {
                        let mut value: serde_json::Value =
                            serde_json::from_str(&level_json).expect("level JSON");
                        value["environment"] = writer::three::environment(sky, None);
                        value["environment"]["cubemap"] = serde_json::json!(sky_path);
                        level_json = serde_json::to_string_pretty(&value).expect("level JSON");
                    }
                    files.push((
                        directory.join(format!("{}.json", options.name)),
                        Contents::Text(level_json),
                    ));
                }
                Target::Defold => {
                    writer::defold::validate(&built.level).map_err(Error::Map)?;
                    let mut defold = writer::defold::DefoldOptions {
                        asset_dir: if options.asset_dir.is_empty() {
                            defold_asset_dir(
                                directory,
                                &options.name,
                                settings.project_root.as_deref(),
                            )
                        } else {
                            options.asset_dir.clone()
                        },
                        ..writer::defold::DefoldOptions::default()
                    };
                    if !options.model_material.is_empty() {
                        defold.model_material = options.model_material.clone();
                    }
                    if !options.model_sampler.is_empty() {
                        defold.model_sampler = options.model_sampler.clone();
                    }
                    let project=settings.project_root.as_deref().unwrap_or(directory);
                    for (path,bytes) in configure_defold_materials(&built.level,&settings,directory,&options.name,project,&mut defold)? {
                        if !files.iter().any(|(p,_)|p==&path) {files.push((path,Contents::Bytes(bytes)));}
                    }
                    writer::defold::validate_material_bindings(&built.level,&defold).map_err(Error::Map)?;
                    defold.sky = options.sky.then(|| map_sky(document, &built.level));
                    for file in writer::defold::write(&built.level, &built.collision, &defold) {
                        files.push((
                            directory.join(&file.path),
                            Contents::Text(file.contents.clone()),
                        ));
                    }
                    if let Some(sky) = &defold.sky {
                        for (path, bytes) in writer::defold::sky_binaries(&options.name, sky) {
                            files.push((directory.join(path), Contents::Bytes(bytes)));
                        }
                    }
                }
                Target::Unity => {
                    writer::unity::validate(&built.level).map_err(Error::Map)?;
                    let mut unity = writer::unity::UnityOptions {
                        sky: options.sky.then(|| map_sky(document, &built.level)),
                        normal_maps: unity_normal_maps(&settings),
                        ..Default::default()
                    };
                    if let Some(sky) = &unity.sky {
                        files.push((directory.join(&options.name).join("sky.hdr"), Contents::Bytes(sky.panorama_hdr(1024,512,writer::sky::unity_panorama_direction))));
                    }
                    if let Some(assets) = directory
                        .ancestors()
                        .find(|path| path.file_name().is_some_and(|name| name == "Assets"))
                    {
                        if let Some(project) = assets.parent() {
                            unity.assets_hint = directory
                                .strip_prefix(project)
                                .unwrap_or(directory)
                                .to_string_lossy()
                                .replace('\\', "/");
                        }
                    }
                    for file in writer::unity::write_with_materials(
                        &built.level,
                        &built.collision,
                        &unity,
                        &settings.generated_materials,
                    ) {
                        files.push((directory.join(&file.path), Contents::Text(file.contents)));
                    }
                }
                Target::Unreal => {
                    let mut unreal = writer::unreal::UnrealOptions::for_level(&options.name);
                    if !options.content_dir.is_empty() {
                        unreal.content_dir = options.content_dir.clone();
                    }
                    files.push((
                        directory.join(format!("{}.json", options.name)),
                        Contents::Text(built.level.to_json()),
                    ));
                    files.push((
                        directory.join(format!("import_{}.py", options.name)),
                        Contents::Text(writer::unreal::write(&built.level, &unreal)),
                    ));
                }
                Target::Flax => {
                    writer::flax::validate(&built.level).map_err(Error::Map)?;
                    let mut flax = writer::flax::FlaxOptions::for_level(&options.name);
                    flax.sky = options.sky.then(|| map_sky(document, &built.level));
                    if let Some(sky) = &flax.sky {
                        let (cube,scale) = sky.cube_dds_normalized(128,true);
                        flax.sky_scale = scale;
                        files.push((directory.join(&options.name).join("sky.dds"),Contents::Bytes(cube)));
                    }
                    if !options.content_dir.is_empty() {
                        flax.content_dir = options.content_dir.clone();
                    }
                    for file in
                        writer::flax::write_with_collision(&built.level, &flax, &built.collision)
                    {
                        files.push((directory.join(&file.path), Contents::Text(file.contents)));
                    }
                }
                Target::Cocos => {
                    let cocos = writer::cocos::CocosOptions {
                        sky: options.sky.then(|| map_sky(document, &built.level)),
                    };
                    for file in writer::cocos::write(&built.level, &built.collision, &cocos) {
                        files.push((directory.join(&file.path), Contents::Bytes(file.contents)));
                    }
                }
                Target::GDevelop => {
                    let meshes: Vec<_> = built
                        .meshes
                        .iter()
                        .filter(|mesh| mesh.file_name.ends_with(".glb"))
                        .map(|mesh| writer::gdevelop::GDevelopMesh {
                            file_name: &mesh.file_name,
                            glb: &mesh.bytes,
                        })
                        .collect();
                    // The scene goes into the project already in the export folder, if any.
                    let existing = std::fs::read_to_string(directory.join("game.json")).ok();
                    // A project further up is not looked into: GDevelop would not see a
                    // second game.json, so the export would start a project of its own.
                    if let Some(root) = settings.project_root.as_deref().filter(|root| {
                        existing.is_none()
                            && root.join("game.json").is_file()
                            && root.canonicalize().ok() != directory.canonicalize().ok()
                    }) {
                        report.warnings.push(format!(
                            "GDevelop: the project's game.json is in {}, not in the export \
                             folder, so this export starts a separate project in {}. Export \
                             into the folder holding game.json.",
                            root.display(),
                            directory.display()
                        ));
                    }
                    let gdevelop = writer::gdevelop::GDevelopOptions {
                        sky: options.sky.then(|| map_sky(document, &built.level)),
                    };
                    for file in writer::gdevelop::write(
                        &built.level,
                        &built.collision,
                        &gdevelop,
                        &meshes,
                        existing.as_deref(),
                    )
                    .map_err(Error::Map)?
                    {
                        files.push((directory.join(file.path), Contents::Bytes(file.contents)));
                    }
                }
                Target::Stride => {
                    writer::stride::validate(&built.level).map_err(Error::Map)?;
                    let mut stride = writer::stride::StrideOptions::for_level(&options.name);
                    stride.sky = options.sky.then(|| map_sky(document, &built.level));
                    let package = stride_package(&directory.join(&options.name));
                    if let Some((_, _, url)) = &package {
                        stride.content_dir = url.clone();
                    }
                    if !options.content_dir.is_empty() {
                        if package.as_ref().is_some_and(|(_, _, url)| {
                            url.trim_matches('/') != options.content_dir.trim_matches('/')
                        }) {
                            report.warnings.push(format!("Stride: the content directory {} does not match where the level lands in the package ({}); models will not load by URL.",options.content_dir,package.as_ref().unwrap().2));
                        }
                        stride.content_dir = options.content_dir.clone();
                    }
                    let meshes: Vec<_> = built
                        .meshes
                        .iter()
                        .filter(|mesh| mesh.file_name.ends_with(".glb"))
                        .map(|mesh| writer::stride::StrideMesh {
                            file_name: &mesh.file_name,
                            glb: &mesh.bytes,
                        })
                        .collect();
                    let stride_files =
                        writer::stride::write(&built.level, &built.collision, &stride, &meshes);
                    match &package {
                        Some((path,text,_)) => {
                            let roots=writer::stride::root_assets(&stride_files,&stride);
                            let updated=writer::stride::update_package(text,&roots,&stride.content_dir);
                            if &updated!=text {files.push((path.clone(),Contents::Text(updated)));}
                        }
                        None => report.warnings.push("Stride: no .sdpkg package contains the export directory. Export into a folder under the game package's Assets folder, or add the generated .sdm3d models to the package's root assets yourself.".into()),
                    }
                    for file in stride_files {
                        files.push((directory.join(file.path), Contents::Bytes(file.contents)));
                    }
                }
                Target::Godot | Target::Obj | Target::Gltf | Target::Glb => {
                    unreachable!("both are handled above")
                }
            }

            report.warnings.extend(built.warnings.clone());
            report.triangles = built.stats.triangles;
            report.saved_faces =
                built.stats.buried_faces + built.stats.pressed_faces + built.stats.merged_faces;
        }
    }

    if matches!(options.target, Target::Defold | Target::Raylib)
        && settings
            .generated_materials
            .values()
            .any(|m| m.surface.is_some())
    {
        report.warnings.push("PBR surface settings are stored in GLB and level metadata. The default sample shaders do not implement full glTF PBR; use a compatible project shader for unsupported material features.".into());
    }
    report.warnings.splice(0..0, class_warnings);
    Ok((report, files))
}

/// Shared by final and live exports. Explicit project samplers see the same
/// map paths, including a portable OpenGL normal companion when requested.
pub(crate) fn configure_defold_materials(
    level:&scene_export::level::Level, settings:&Settings, directory:&Path, name:&str,
    project:&Path, options:&mut writer::defold::DefoldOptions,
) -> Result<Vec<(PathBuf,Vec<u8>)>,Error> {
    let mut files=Vec::new(); let mut normals=std::collections::BTreeMap::new();
    if level.entities.iter().any(|e|e.properties.contains_key("defold_sampler_normal")) {
        for (material,png) in &settings.normal_maps {
            let path=directory.join(format!("{name}_paint")).join(format!("{material}_normal.png"));
            let resource=scene_export::materials::fs_to_res(project,&path).ok_or_else(||Error::Map("Defold normal map must be inside the project".into()))?;
            normals.insert(material.clone(),resource.trim_start_matches("res://").to_owned());files.push((path,png.to_vec()));
        }
    }
    for texture in &level.textures {
        let mut maps:Vec<_>=texture.maps.iter().map(|(slot,path)|(if slot=="emission" {"emissive".into()} else {slot.clone()},path.clone())).collect();
        if let Some(pbr)=settings.generated_materials.get(&texture.name).and_then(|m|m.pbr.as_ref()) {
            for image in &pbr.images {maps.retain(|(slot,_)|slot != image.slot);maps.push((image.slot.to_owned(),image.path.trim_start_matches("res://").to_owned()));}
        }
        if let Some(path)=normals.get(&texture.name) {maps.retain(|(slot,_)|slot != "normal");maps.push(("normal".into(),path.clone()));}
        else if let Some(path)=settings.normal_map_paths.get(&texture.name) {
            if !maps.iter().any(|(slot,_)|slot=="normal") {maps.push(("normal".into(),path.trim_start_matches("res://").into()));}
        }
        options.material_maps.insert(texture.name.clone(),maps);
    }
    Ok(files)
}

/// The sky and sun the map is lit with: its first render camera's, which a map saves,
/// or the editor's own when it has no camera, with the sky and fog its worldspawn sets.
pub fn map_sky(document: &Document, level: &scene_export::level::Level) -> writer::sky::Sky {
    // Without a saved render camera the level is lit as the 3D view is, from the
    // preferences. A default camera would bring its own sun and exposure instead.
    let render = crate::render_camera::cameras(document)
        .into_iter()
        .next()
        .map_or_else(crate::render_settings::from_preferences, |camera| {
            camera.2.render()
        });
    writer::sky::Sky {
        sky: render.sky_color,
        horizon: render.horizon_color,
        ground: render.ground_color,
        sun_color: render.sun_color,
        sun_intensity: render.sun_intensity,
        ambient_intensity: render.ambient_intensity,
        sun_elevation: render.sun_elevation,
        sun_azimuth: render.sun_azimuth,
        exposure: render.exposure,
        fog: None,
    }
    .with_worldspawn(level)
}

/// The Stride package that will compile `folder`: its `.sdpkg` path and text, and the
/// content URL `folder` has inside one of its asset folders. None when no package above
/// `folder` lists an asset folder containing it.
fn stride_package(folder: &Path) -> Option<(PathBuf, String, String)> {
    let folder = folder
        .parent()?
        .canonicalize()
        .ok()?
        .join(folder.file_name()?);
    for directory in folder.ancestors().skip(1) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("sdpkg") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines() {
                let Some(asset_folder) = line
                    .trim()
                    .strip_prefix("-   Path: !dir ")
                    .or_else(|| line.trim().strip_prefix("- Path: !dir "))
                else {
                    continue;
                };
                let Ok(relative) = folder.strip_prefix(directory.join(asset_folder.trim())) else {
                    continue;
                };
                let url = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if !url.is_empty() {
                    return Some((path, text, url));
                }
            }
        }
    }
    None
}

/// The generated Unity materials that have a normal map, written beside their
/// albedo as `<name>_normal.png`.
fn unity_normal_maps(settings: &Settings) -> Vec<String> {
    settings
        .generated_materials
        .keys()
        .filter(|name| settings.normal_maps.contains_key(name.as_str()))
        .cloned()
        .collect()
}

/// The MTL for generated materials. With `normals`, a material with a normal map
/// names its `<name>_normal.png` on a `norm` line, which the caller must write.
pub fn paint_mtl(settings: &Settings, image_directory: &str, normals: bool) -> String {
    use std::fmt::Write;
    let mut mtl = String::from("# Heron Brush face paints\n");
    for (name, material) in &settings.generated_materials {
        let _ = writeln!(
            mtl,
            "newmtl {name}\nKd 1 1 1\nmap_Kd {image_directory}/{name}.png"
        );
        if normals && settings.normal_maps.contains_key(name.as_str()) {
            let _ = writeln!(mtl, "norm {image_directory}/{name}_normal.png");
        }
        if let Some([roughness, metallic, emission]) = material.surface {
            let _ = writeln!(
                mtl,
                "Pr {roughness}\nPm {metallic}\nNs {}",
                (2.0 / roughness.max(0.05).powi(4) - 2.0).min(1000.0)
            );
            if emission > 0.0 {
                let _ = writeln!(
                    mtl,
                    "Ke {emission} {emission} {emission}\nmap_Ke {image_directory}/{name}.png"
                );
            }
        }
        mtl.push('\n');
    }
    mtl
}

/// What goes into a file: a writer gives back one or the other.
pub enum Contents {
    /// A scene, a mesh or a level, as text.
    Text(String),
    /// A `.glb`, which is binary.
    Bytes(Vec<u8>),
}

/// Writes every engine that is set to keep up with the map, and returns what each one
/// wrote or why it could not.
///
/// A successful map save triggers exports into the configured engine projects.
/// Engines with project watchers can then reload the updated files.
///
/// An engine qualifies when it has been exported once (which is what says where to
/// write) and its switch is on. A failure is reported and the others still go: one
/// engine's project having moved should not stop the rest.
pub fn export_on_save(
    document: &Document,
    settings: &crate::export_settings::Settings,
) -> Vec<Result<SavedExport, String>> {
    if let Err(error) = settings.validate() {
        return vec![Err(error)];
    }
    let mut said = Vec::new();
    for target in Target::ALL {
        let mut options = settings.for_target(target);
        if !options.export_on_save || options.directory.is_empty() {
            continue;
        }
        let directory = std::path::PathBuf::from(&options.directory);
        options.name = document.level_name();
        match export(document, &directory, &options) {
            Ok(report) => said.push(Ok(SavedExport {
                target,
                directory,
                files: report.files.len(),
            })),
            Err(why) => said.push(Err(format!("{}: {why}", target.name()))),
        }
    }
    said
}

/// One engine's export that [`export_on_save`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedExport {
    /// The engine it was for.
    pub target: Target,
    /// The folder it was written into.
    pub directory: std::path::PathBuf,
    /// How many files it wrote.
    pub files: usize,
}

impl std::fmt::Display for SavedExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({} files)",
            self.target.name(),
            self.directory.display(),
            self.files
        )
    }
}

/// Where a Defold level's meshes will sit, as the `/`-rooted resource path Defold names
/// them by.
///
/// Defold refers to a component by its path from the project root, so the collection has
/// to say where the files it points at actually are. Worked out from the destination
/// rather than typed: the meshes go in `<destination>/<name>/`, so the resource path is
/// that folder's path from the project root. Getting it wrong by hand costs nothing
/// visible at export time and produces a collection whose every component is missing
/// when the project is opened, which is a poor way to find out.
///
/// Without a project root there is nothing to be relative to, so it falls back to
/// `/<name>`, which is right when the level is written at the top of the project.
fn defold_asset_dir(directory: &Path, name: &str, project_root: Option<&Path>) -> String {
    let meshes = directory.join(name);
    let from_root = project_root.and_then(|root| {
        scene_export::materials::make_absolute(&meshes)?
            .strip_prefix(root)
            .ok()
            .map(Path::to_path_buf)
    });
    match from_root {
        Some(relative) => format!("/{}", relative.to_string_lossy().replace('\\', "/")),
        None => format!("/{name}"),
    }
}

/// The `.import` file Godot should read a level mesh with, where there is not one yet.
///
/// Godot's own default for a Wavefront mesh compresses its vertex attributes, which
/// quantises the texture coordinates: a floor whose corner should land exactly on the
/// fourth repeat of its texture comes in at 4.0004 instead, and on a large surface that
/// drift is visible as the material sliding across it. A level mesh is the one case where
/// that trade is wrong -- the geometry is authored to the grid and the texture alignment
/// is the point of it -- so an export turns compression off.
///
/// Only where the project has no `.import` for this mesh already. Godot writes one the
/// first time it sees a file and a person may have tuned it since; an export that
/// rewrites it every time would undo their work, and the setting only matters once.
pub fn godot_import_defaults(mesh_path: &Path) -> Option<(PathBuf, String)> {
    let import_path = {
        let mut name = mesh_path.file_name()?.to_os_string();
        name.push(".import");
        mesh_path.with_file_name(name)
    };
    if import_path.exists() {
        return None;
    }
    // Only `[params]`; Godot fills in the `[remap]` and `[deps]` sections itself when it
    // imports the mesh, and writing a stale uid into them would be worse than leaving
    // them out.
    Some((
        import_path,
        "[params]\n\nforce_disable_mesh_compression=true\n".to_string(),
    ))
}

/// The entity classes the builder works from: the same `.fgd` the editor has loaded.
///
/// Read again here rather than converted from the editor's own definitions: each half has
/// a reader for the file, and a conversion between two shapes of the same thing is a third
/// thing to keep right. Without a file the map still exports -- the builder falls back to
/// the bundled HERON_BRUSH definitions -- so a failure to read one is a warning rather than an error.
/// How much of an entity the portable OBJ, glTF and GLB writers draw. Engine targets
/// decide this in the builder, which also writes the collision and sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortableVisuals {
    All,
    /// A class with `visuals=false`, such as a trigger.
    Nothing,
    /// A water volume: only the faces that point up, which are its surface. With
    /// `water_surface_material=0` a water volume draws in full, as in the engines.
    WaterSurface,
}

pub(crate) fn portable_visuals(
    entity: &map_model::node::EntityNode,
    classes: &scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
) -> PortableVisuals {
    let classname = entity.classname().unwrap_or_default();
    if classes
        .get(classname)
        .and_then(|class| class.directives.bool("visuals"))
        == Some(false)
    {
        PortableVisuals::Nothing
    } else if scene_export::water::enabled_by(classname, entity.property("water_enabled"))
        && scene_export::water::baseline_by(entity.property("water_surface_material"))
    {
        PortableVisuals::WaterSurface
    } else {
        PortableVisuals::All
    }
}

pub fn entity_classes(
    document: &Document,
) -> (
    scene_export::indexmap::IndexMap<String, scene_export::fgd::EntityClass>,
    Vec<String>,
) {
    if let Some(classes) = &document.export_classes_override {
        return classes.clone();
    }
    let mut common =
        scene_export::fgd::parse(crate::builtin_entities::SOURCE, "<built-in HERON_BRUSH>")
            .expect("bundled HERON_BRUSH export FGD must be valid");
    if let Some(path) = document.definitions_path() {
        match scene_export::fgd::parse_file(path) {
            Ok(project) => {
                common.classes.extend(project.classes);
                common.warnings.extend(project.warnings);
            }
            Err(why) => common.warnings.push(format!("{}: {why}", path.display())),
        }
    }
    (common.resolved(), common.warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldBounds;

    #[test]
    fn godot_paths_in_properties_are_named_for_other_engines() {
        let values = [
            scene_export::level::Value::String("res://models/grass.glb".into()),
            scene_export::level::Value::String("models/tree.glb".into()),
        ];
        let warning = godot_paths_warning(values.iter(), Target::Defold).expect("a warning");
        assert!(warning.contains("res://models/grass.glb") && warning.contains("Defold"));
        assert!(!warning.contains("tree.glb"));
        assert!(godot_paths_warning(std::iter::empty(), Target::Bevy).is_none());
    }

    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;

    /// A document holding one 64 unit cube.
    fn one_cube() -> Document {
        let mut document = Document::new(WorldBounds::default());
        let world = document.world_bounds();
        let brush = map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(Vec3d::zero(), Vec3d::fill(64.0)),
            "stone",
        )
        .expect("the cube is built");
        document.add_brushes(vec![brush]);
        document
    }

    #[test]
    fn hidden_faces_roundtrip_export_one_quad_and_keep_collision() {
        let mut doc = one_cube();
        doc.select_all();
        let before = doc.to_map_text();
        let bounds = doc.selection_bounds();
        let mut index = 0;
        doc.edit_faces(|face| {
            face.export_hidden = index != 0;
            index += 1;
        });
        // Hidden metadata must coexist with both shared paint and per-face PBR comments.
        doc.set_face_color(Some(map_model::brush_face::FaceColor::solid(
            map_model::brush_face::Rgb::new(80, 120, 160),
        )));
        doc.set_surface_material(Some(map_model::brush_face::SurfaceMaterial {
            roughness: 40,
            metallic: 10,
            emission: 0,
        }));
        let painted = doc.to_map_text();
        let paint_map = map_model::io::map_reader::read_map(&painted, None).unwrap();
        assert_eq!(map_model::io::map_writer::write_map(&paint_map), painted);
        doc.undo();
        doc.undo();
        let hidden = doc.to_map_text();
        assert!(hidden.contains("export_hidden"));
        let parsed = map_model::io::map_reader::read_map(&hidden, None).unwrap();
        assert_eq!(
            parsed.entities[0].brushes[0]
                .faces
                .iter()
                .filter(|f| f.export_hidden)
                .count(),
            5
        );
        assert_eq!(map_model::io::map_writer::write_map(&parsed), hidden);
        assert_eq!(doc.selection_bounds(), bounds);
        doc.undo();
        assert_eq!(doc.to_map_text(), before);
        doc.redo();
        assert_eq!(doc.to_map_text(), hidden);
        for target in Target::ALL {
            let scratch = Scratch::new(&format!("hidden-face-{target:?}"));
            let options = Options {
                target,
                name: "single_face".into(),
                ..Options::default()
            };
            let report = export(&doc, &scratch.0, &options).unwrap();
            assert_eq!(report.triangles, 2, "{target:?}");
            assert_eq!(doc.to_map_text(), hidden);
        }
        let scratch = Scratch::new("hidden-collision");
        let mut settings = scene_export::settings::Settings::default();
        let (text, _) =
            paint_export::prepare(&doc, &scratch.0, &Options::default(), &mut settings).unwrap();
        let map = scene_export::map::parse(&text, "test").unwrap();
        for collision in [
            scene_export::settings::CollisionKind::Convex,
            scene_export::settings::CollisionKind::Concave,
        ] {
            let geometry = scene_export::geometry::build_solid(
                &map.entities[0],
                &settings,
                scene_export::geometry::SolidOptions {
                    collision,
                    ..Default::default()
                },
                &scene_export::geometry::TextureSizes::new(32.0),
            );
            if collision == scene_export::settings::CollisionKind::Convex {
                assert_eq!(geometry.convex_hulls.len(), 1);
                assert_eq!(geometry.convex_hulls[0].len(), 8);
            } else {
                assert_eq!(geometry.concave_triangles.len(), 36);
            }
        }
    }

    #[test]
    fn surface_material_obj_keeps_distinct_mtl_settings() {
        use map_model::brush_face::{FaceColor, Rgb, SurfaceMaterial};
        let mut doc = one_cube();
        doc.select_all();
        doc.set_face_color(Some(FaceColor::solid(Rgb::new(100, 120, 150))));
        let id = *doc.selection_ids().iter().next().unwrap();
        doc.set_surface_material_on(
            &[crate::document::FaceRef { node: id, face: 0 }],
            Some(SurfaceMaterial {
                roughness: 20,
                metallic: 80,
                emission: 250,
            }),
        );
        let scratch = Scratch::new("surface-mtl");
        export(
            &doc,
            &scratch.0,
            &Options {
                target: Target::Obj,
                name: "surface".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let mtl = std::fs::read_to_string(scratch.0.join("surface.mtl")).unwrap();
        assert_eq!(mtl.matches("newmtl ").count(), 2);
        assert!(mtl.contains("Pr 0.2"));
        assert!(mtl.contains("Pm 0.8"));
        assert!(mtl.contains("Ke 2.5 2.5 2.5"));
    }
    #[test]
    fn surface_material_exports_to_godot_unity_and_portable_level() {
        use map_model::brush_face::{FaceColor, Rgb, SurfaceMaterial};
        let mut doc = one_cube();
        doc.select_all();
        doc.set_face_color(Some(FaceColor::solid(Rgb::new(200, 100, 20))));
        let id = *doc.selection_ids().iter().next().unwrap();
        doc.set_surface_material_on(
            &[crate::document::FaceRef { node: id, face: 0 }],
            Some(SurfaceMaterial {
                roughness: 20,
                metallic: 80,
                emission: 250,
            }),
        );
        let before = doc.to_map_text();
        for target in [
            Target::Godot,
            Target::Unity,
            Target::Bevy,
            Target::ThreeJs,
            Target::Defold,
            Target::Flax,
            Target::Raylib,
            Target::Unreal,
        ] {
            let scratch = Scratch::new(&format!("surface-{target:?}"));
            let options = Options {
                target,
                name: "surface".into(),
                ..Options::default()
            };
            let report = export(&doc, &scratch.0, &options).unwrap();
            assert_eq!(report.triangles, 12);
            assert_eq!(doc.to_map_text(), before);
            let texts = report
                .files
                .iter()
                .filter_map(|p| std::fs::read_to_string(p).ok())
                .collect::<Vec<_>>()
                .join("\n");
            if target == Target::Godot {
                assert!(texts.contains("roughness = 0.2"));
                assert!(texts.contains("metallic = 0.8"));
                assert!(texts.contains("emission_energy_multiplier = 2.5"));
            } else if target == Target::Unity {
                assert!(texts.contains("0.200000f, 0.800000f, 2.500000f"));
                assert!(texts.contains("1f - roughness"));
            } else {
                assert!(
                    texts.contains("\"surface\": ["),
                    "{target:?} loses PBR metadata"
                );
            }
            assert_eq!(
                report
                    .files
                    .iter()
                    .filter(|p| p.extension().is_some_and(|s| s == "png")
                        && !p
                            .file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with("sky")))
                    .count(),
                2,
                "different surfaces must not share one material: {target:?}"
            );
        }
    }
    #[test]
    fn every_engine_preserves_mixed_face_paints_and_emits_complete_materials() {
        use crate::document::FaceRef;
        use map_model::brush_face::{FaceColor, Rgb};
        let mut document = one_cube();
        document.select_all();
        let id = *document.selection_ids().iter().next().unwrap();
        document.set_face_color(Some(FaceColor::solid(Rgb::new(230, 40, 20))));
        document.set_face_color_on(
            &[FaceRef { node: id, face: 0 }],
            Some(FaceColor::solid(Rgb::new(20, 40, 230))),
        );
        document.set_face_color_on(
            &[FaceRef { node: id, face: 1 }],
            Some(FaceColor::gradient(
                Rgb::new(230, 40, 20),
                Rgb::new(20, 230, 40),
                90.0,
            )),
        );
        let before = document.to_map_text();
        for target in Target::ALL {
            let scratch = Scratch::new(&format!("face-paints-{target:?}"));
            let options = Options {
                target,
                name: "painted".into(),
                ..Options::default()
            };
            // Checking must not create files, including generated textures.
            check(&document, &scratch.0, &options).unwrap();
            assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 0);
            let report = export(&document, &scratch.0, &options).unwrap();
            assert_eq!(report.triangles, 12, "{target:?}");
            assert!(
                !report
                    .warnings
                    .iter()
                    .any(|w| w.contains("bbpaint_") && w.contains("no texture")),
                "{:?}",
                report.warnings
            );
            assert_eq!(
                document.to_map_text(),
                before,
                "export must not alter face paint in the editor"
            );
            if matches!(target, Target::Gltf | Target::Glb) {
                continue;
            }
            // Stride also writes each GLB's images beside its texture assets.
            let stride_copy = |p: &std::path::Path| {
                (target == Target::Stride && p.parent().and_then(|d| d.file_name()).is_some_and(|d| d == "Textures"))
                // The sky is images of its own.
                || p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("sky"))
            };
            let pngs: Vec<_> = report
                .files
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == "png") && !stride_copy(p))
                .collect();
            assert_eq!(
                pngs.len(),
                3,
                "same solid colour must share one image: {target:?}"
            );
            let mut solids = Vec::new();
            for path in pngs {
                let image = image::open(path).unwrap().to_rgba8();
                if image.width() == 1 {
                    solids.push(image.get_pixel(0, 0).0);
                } else {
                    // The gradient is a strip of its colours, shared by every face that has it.
                    assert_eq!(image.height(), 1, "{target:?}");
                    assert_eq!(image.get_pixel(0, 0).0, [230, 40, 20, 255]);
                    assert_eq!(image.get_pixel(image.width() - 1, 0).0, [20, 230, 40, 255]);
                }
            }
            assert!(solids.contains(&[230, 40, 20, 255]) && solids.contains(&[20, 40, 230, 255]));
            if target == Target::Godot {
                let scene = std::fs::read_to_string(scratch.0.join("painted.tscn")).unwrap();
                assert_eq!(scene.matches("albedo_texture =").count(), 3);
            }
            if matches!(target, Target::Unity | Target::Obj) {
                let mtls: Vec<_> = report
                    .files
                    .iter()
                    .filter(|p| p.extension().is_some_and(|e| e == "mtl"))
                    .collect();
                assert!(!mtls.is_empty());
                for mtl in mtls {
                    let text = std::fs::read_to_string(mtl).unwrap();
                    assert_eq!(text.matches("map_Kd ").count(), 3);
                    for line in text.lines().filter_map(|l| l.strip_prefix("map_Kd ")) {
                        assert!(mtl.parent().unwrap().join(line).is_file(), "missing {line}");
                    }
                }
            } else {
                let glbs: Vec<_> = report
                    .files
                    .iter()
                    .filter(|p| p.extension().is_some_and(|e| e == "glb"))
                    .collect();
                if target != Target::Godot {
                    assert!(!glbs.is_empty(), "{target:?}");
                }
                for path in glbs {
                    let bytes = std::fs::read(path).unwrap();
                    let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
                    let json: serde_json::Value =
                        serde_json::from_slice(&bytes[20..20 + size]).unwrap();
                    if json.get("materials").is_none() {
                        continue;
                    } // separate collision mesh
                    assert_eq!(json["images"].as_array().unwrap().len(), 3, "{target:?}");
                    let binary = 20 + size + 8;
                    for image in json["images"].as_array().unwrap() {
                        let view =
                            &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
                        let start = binary + view["byteOffset"].as_u64().unwrap() as usize;
                        let size = view["byteLength"].as_u64().unwrap() as usize;
                        image::load_from_memory(&bytes[start..start + size]).unwrap();
                    }
                    for material in json["materials"].as_array().unwrap() {
                        assert!(
                            material["pbrMetallicRoughness"]["baseColorTexture"]["index"]
                                .is_number()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn defold_project_shader_receives_requested_normal_texture() {
        let scratch=Scratch::new("defold-normal-sampler");
        let directory=std::env::var_os("HERON_DEFOLD_SAMPLER_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(||scratch.0.join("out"));
        std::fs::create_dir_all(&directory).unwrap();
        let textures=scratch.0.join("textures");std::fs::create_dir_all(&textures).unwrap();
        let png=|p:[u8;4]|crate::assets::encode_png(2,2,&p.repeat(4)).unwrap();
        std::fs::write(textures.join("stone.png"),png([90,90,90,255])).unwrap();
        std::fs::write(textures.join("stone_normal_dx.png"),png([140,200,250,255])).unwrap();
        let mut document=one_cube();document.load_materials(&textures).unwrap();
        let options=Options {
            target:Target::Defold,name:"arena".into(),project_dir:directory.to_string_lossy().into_owned(),model_material:"/normal.material".into(),
            entities:[("worldspawn".into(),crate::export_bindings::Rule{defaults:[("defold_sampler_normal".into(),serde_json::json!("tex_normal"))].into(),..Default::default()})].into(),
            ..Default::default()
        };
        let (_,files)=export_files(&document,&directory,&options).unwrap();
        let model=files.iter().find(|(p,_)|p.ends_with("worldspawn.model")).unwrap();
        let Contents::Text(model)=&model.1 else {panic!("model text")};
        let texture=model.split("sampler: \"tex_normal\"").nth(1).unwrap().split("texture: \"").nth(1).unwrap().split('"').next().unwrap();
        let expected=directory.join(texture.trim_start_matches('/'));
        let (_,Contents::Bytes(bytes))=files.iter().find(|(p,_)|p==&expected).unwrap() else {panic!("normal image")};
        let pixels=image::load_from_memory(bytes).unwrap().to_rgba8();assert_eq!(pixels.get_pixel(0,0).0,[140,55,250,255]);
        if std::env::var_os("HERON_DEFOLD_SAMPLER_OUTPUT").is_some() {
            for (path,data) in files {std::fs::create_dir_all(path.parent().unwrap()).unwrap();match data {Contents::Text(s)=>std::fs::write(path,s).unwrap(),Contents::Bytes(b)=>std::fs::write(path,b).unwrap()}}
        }
    }

    #[test]
    fn companion_normal_maps_reach_the_glb_in_the_opengl_convention_with_tangents() {
        let scratch = Scratch::new("normal-maps");
        let textures = scratch.0.join("textures");
        std::fs::create_dir_all(&textures).unwrap();
        let png = |rgba: [u8; 4]| crate::assets::encode_png(2, 2, &rgba.repeat(4)).unwrap();
        std::fs::write(textures.join("stone.png"), png([90, 90, 90, 255])).unwrap();
        // A DirectX map: green grows downwards, so glTF wants it inverted.
        std::fs::write(
            textures.join("stone_normal_dx.png"),
            png([140, 200, 250, 255]),
        )
        .unwrap();
        let mut document = one_cube();
        document.load_materials(&textures).unwrap();
        assert!(document
            .materials()
            .get("stone")
            .unwrap()
            .normal_map
            .is_some());

        for target in [Target::Bevy, Target::Glb] {
            let out = scratch.0.join(format!("{target:?}"));
            let options = Options {
                target,
                name: "arena".into(),
                project_dir: scratch.0.to_string_lossy().into_owned(),
                texture_dir: "textures".into(),
                ..Options::default()
            };
            let report = export(&document, &out, &options).unwrap();
            let mut checked = 0;
            for path in report
                .files
                .iter()
                .filter(|p| p.extension().is_some_and(|e| e == "glb"))
            {
                let bytes = std::fs::read(path).unwrap();
                let size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
                let json: serde_json::Value =
                    serde_json::from_slice(&bytes[20..20 + size]).unwrap();
                let Some(material) = json["materials"].as_array().and_then(|m| m.first()) else {
                    continue;
                };
                let texture = material["normalTexture"]["index"]
                    .as_u64()
                    .expect("normalTexture") as usize;
                let image =
                    &json["images"][json["textures"][texture]["source"].as_u64().unwrap() as usize];
                let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
                let start = 28 + size + view["byteOffset"].as_u64().unwrap() as usize;
                let end = start + view["byteLength"].as_u64().unwrap() as usize;
                let decoded = image::load_from_memory(&bytes[start..end])
                    .unwrap()
                    .to_rgba8();
                assert_eq!(decoded.get_pixel(0, 0).0, [140, 55, 250, 255], "{target:?}");
                for primitive in json["meshes"][0]["primitives"].as_array().unwrap() {
                    let attributes = &primitive["attributes"];
                    let tangent = &json["accessors"]
                        [attributes["TANGENT"].as_u64().expect("TANGENT") as usize];
                    let position =
                        &json["accessors"][attributes["POSITION"].as_u64().unwrap() as usize];
                    assert_eq!(tangent["type"], "VEC4");
                    assert_eq!(tangent["count"], position["count"], "{target:?}");
                }
                checked += 1;
            }
            assert!(checked > 0, "{target:?} wrote no textured GLB");
        }
    }

    #[test]
    fn unity_gets_brush_normal_maps_beside_the_albedo_in_the_opengl_convention() {
        let scratch = Scratch::new("unity-normal-maps");
        let textures = scratch.0.join("textures");
        std::fs::create_dir_all(&textures).expect("texture folder");
        let png = |rgba: [u8; 4]| crate::assets::encode_png(2, 2, &rgba.repeat(4)).expect("png");
        std::fs::write(textures.join("stone.png"), png([90, 90, 90, 255])).expect("albedo");
        // A DirectX map: Unity, like glTF, wants its green channel inverted.
        std::fs::write(
            textures.join("stone_normal_dx.png"),
            png([140, 200, 250, 255]),
        )
        .expect("normal map");
        let mut document = one_cube();
        document.load_materials(&textures).expect("materials load");
        let out = scratch.0.join("unity");
        let options = Options {
            target: Target::Unity,
            name: "arena".into(),
            project_dir: scratch.0.to_string_lossy().into_owned(),
            texture_dir: "textures".into(),
            ..Options::default()
        };
        let report = export(&document, &out, &options).expect("export");
        let normals: Vec<_> = report
            .files
            .iter()
            .filter(|p| p.to_string_lossy().ends_with("_normal.png"))
            .collect();
        assert_eq!(normals.len(), 1, "{:?}", report.files);
        let image = image::open(normals[0]).expect("normal PNG").to_rgba8();
        assert_eq!(image.get_pixel(0, 0).0, [140, 55, 250, 255]);
        let name = normals[0]
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix("_normal.png"))
            .expect("named after its material");
        assert!(out.join(format!("arena_paint/{name}.png")).is_file());
        let mtl = std::fs::read_to_string(out.join("arena/arena_paint.mtl")).expect("MTL");
        assert!(mtl.contains(&format!("norm ../arena_paint/{name}_normal.png")));
        let script = std::fs::read_to_string(out.join("Editor/HeronBrushLevelImporter.cs"))
            .expect("importer");
        assert!(script.contains(&format!(
            "ApplyNormal(root, here, \"{name}\", \"arena_paint\");"
        )));
    }

    #[test]
    fn godot_takes_the_normal_map_the_editor_paired_and_flips_a_directx_one() {
        let scratch = Scratch::new("godot-normal-maps");
        let textures = scratch.0.join("textures");
        std::fs::create_dir_all(&textures).expect("texture folder");
        let png = |rgba: [u8; 4]| crate::assets::encode_png(2, 2, &rgba.repeat(4)).expect("png");
        std::fs::write(textures.join("stone.png"), png([90, 90, 90, 255])).expect("albedo");
        let export_with = |normal: &str| {
            for old in ["stone_n.png", "stone_normal_dx.png"] {
                let _ = std::fs::remove_file(textures.join(old));
            }
            std::fs::write(textures.join(normal), png([140, 200, 250, 255])).expect("normal");
            let mut document = one_cube();
            document.load_materials(&textures).expect("materials");
            let out = scratch.0.join("level");
            let options = Options {
                target: Target::Godot,
                name: "arena".into(),
                project_dir: scratch.0.to_string_lossy().into_owned(),
                texture_dir: "textures".into(),
                ..Options::default()
            };
            export(&document, &out, &options).expect("export");
            std::fs::read_to_string(out.join("arena.tscn")).expect("scene")
        };

        // `_n` is one of the editor's suffixes; the image is used where it is.
        let scene = export_with("stone_n.png");
        assert!(
            scene.contains(r#"path="res://textures/stone_n.png""#),
            "{scene}"
        );
        assert!(scene.contains("normal_enabled = true"), "{scene}");
        assert!(!scratch
            .0
            .join("level/arena_paint/stone_normal.png")
            .exists());

        // A DirectX map goes out converted, its green flipped for Godot's OpenGL convention.
        let scene = export_with("stone_normal_dx.png");
        assert!(
            scene.contains(r#"path="res://level/arena_paint/stone_normal.png""#),
            "{scene}"
        );
        let written = image::open(scratch.0.join("level/arena_paint/stone_normal.png"))
            .expect("converted normal map")
            .to_rgba8();
        assert_eq!(written.get_pixel(0, 0).0, [140, 55, 250, 255]);
    }

    /// A directory of this test's own, cleaned up as the test ends.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("heron-export-{name}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the scratch directory is made");
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Exports the cube for one engine and gives back what was written.
    fn export_cube(target: Target, name: &str) -> (Report, std::path::PathBuf, Scratch) {
        let document = one_cube();
        let scratch = Scratch::new(name);
        let options = Options {
            target,
            name: "arena".to_string(),
            ..Options::default()
        };
        let report = export(&document, &scratch.0, &options).expect("the export is written");
        let directory = scratch.0.clone();
        (report, directory, scratch)
    }

    #[test]
    fn gdevelop_warns_when_its_project_is_above_the_export_folder() {
        // Found exporting the demos with the project folder above maps/: the GDevelop
        // export wrote a fresh maps/game.json and said nothing, while the project's
        // own game.json was left without the scene.
        let scratch = Scratch::new("gdevelop-above");
        let game = std::fs::read_to_string(
            export_cube(Target::GDevelop, "gdevelop-source")
                .1
                .join("game.json"),
        )
        .unwrap();
        std::fs::write(scratch.0.join("game.json"), game).unwrap();
        let options = Options {
            target: Target::GDevelop,
            name: "arena".to_string(),
            ..Options::default()
        };
        let report = export(&one_cube(), &scratch.0.join("maps"), &options).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("separate project")),
            "{:?}",
            report.warnings
        );
        let report = export(&one_cube(), &scratch.0, &options).unwrap();
        assert!(!report
            .warnings
            .iter()
            .any(|w| w.contains("separate project")));
    }

    #[test]
    fn every_engine_gets_files_it_can_read() {
        for (target, expected) in [
            (Target::Godot, "arena.tscn"),
            (Target::ThreeJs, "arena.json"),
            (Target::Babylon, "arena.html"),
            (Target::PlayCanvas, "arena.html"),
            (Target::Bevy, "arena.json"),
            (Target::Raylib, "arena.level"),
            (Target::Unity, "Editor/HeronBrushLevelImporter.cs"),
            (Target::Unreal, "import_arena.py"),
            (Target::Obj, "arena.obj"),
            (Target::Gltf, "arena.gltf"),
            (Target::Glb, "arena.glb"),
        ] {
            let (report, directory, _scratch) =
                export_cube(target, &target.name().replace('.', ""));
            let named = directory.join(expected);
            assert!(
                report.files.contains(&named),
                "{} wrote {:?}, which does not include {expected}",
                target.name(),
                report.files
            );
            if target == Target::Raylib {
                let metadata: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(directory.join("arena.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(metadata["format"], 2);
                assert!(!metadata["entities"].as_array().unwrap().is_empty());
            }
            let written = std::fs::read(&named).expect("the file is on disk");
            assert!(!written.is_empty(), "{expected} is empty");
        }
    }

    #[test]
    fn flax_keeps_collision_in_json_and_out_of_visual_glb() {
        let (_, directory, _scratch) = export_cube(Target::Flax, "flax-inline-collision");
        let level: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(directory.join("arena.level.json")).unwrap(),
        )
        .unwrap();
        let world = &level["entities"][0];
        assert!(!world["collision"]["triangles"]
            .as_array()
            .unwrap()
            .is_empty());
        let bytes = std::fs::read(directory.join(world["mesh"].as_str().unwrap())).unwrap();
        let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let gltf: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
        assert_eq!(gltf["meshes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn stride_exports_visual_assets_inline_collision_and_native_model_assets() {
        let (report, directory, _scratch) = export_cube(Target::Stride, "stride-preparation");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("no .sdpkg package")),
            "{:?}",
            report.warnings
        );
        let level: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(directory.join("arena.json")).unwrap())
                .unwrap();
        let world = &level["entities"][0];
        assert!(!world["collision"]["triangles"]
            .as_array()
            .unwrap()
            .is_empty());
        let bytes = std::fs::read(directory.join(world["mesh"].as_str().unwrap())).unwrap();
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let glb: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
        assert_eq!(
            glb["meshes"].as_array().unwrap().len(),
            1,
            "no duplicate collision mesh in visual GLB"
        );
        for file in [
            "arena/HeronBrushStrideLevel.cs",
            "arena/HeronBrushStrideRuntime.cs",
            "arena/HeronBrushStridePhysics.cs",
            "arena/worldspawn.sdm3d",
            "arena/Sky/Skybox.sdsky",
            "arena/Sky/Sky.dds",
            "STRIDE-README.md",
        ] {
            assert!(directory.join(file).is_file(), "{file}");
        }
    }
    #[test]
    fn babylon_writes_its_runtime_a_page_and_the_level_with_inline_collision() {
        let (report, directory, _scratch) = export_cube(Target::Babylon, "babylon-files");
        for file in [
            "arena.json",
            "arena.html",
            "heron_babylon.js",
            "arena/sky.hdr",
        ] {
            assert!(
                report.files.contains(&directory.join(file)),
                "{file} missing from {:?}",
                report.files
            );
        }
        let level: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(directory.join("arena.json")).unwrap())
                .unwrap();
        // The page builds Havok bodies from the level file, as three.js builds its
        // Octree, so the triangles are inline and not in the mesh as well.
        let collision = &level["entities"][0]["collision"];
        assert!(
            collision["triangles"].is_array() || collision["hulls"].is_array(),
            "{collision}"
        );
        assert_eq!(
            gltf_mesh_count(&directory.join("arena/worldspawn.glb")),
            1,
            "a collision mesh was written into the GLB"
        );
        let runtime = std::fs::read_to_string(directory.join("heron_babylon.js")).unwrap();
        assert!(runtime.contains("export async function loadLevel"));
        let page = std::fs::read_to_string(directory.join("arena.html")).unwrap();
        assert!(page.contains("PhysicsCharacterController"));
        assert!(page.contains(&format!(
            "babylonjs@{}/babylon.js",
            writer::babylon::BABYLON_VERSION
        )));
    }

    #[test]
    fn playcanvas_writes_its_runtime_a_page_and_the_level_with_inline_collision() {
        let (report, directory, _scratch) = export_cube(Target::PlayCanvas, "playcanvas-files");
        for file in [
            "arena.json",
            "arena.html",
            "heron_playcanvas.js",
            "arena/sky.hdr",
        ] {
            assert!(
                report.files.contains(&directory.join(file)),
                "{file} missing from {:?}",
                report.files
            );
        }
        let level: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(directory.join("arena.json")).unwrap())
                .unwrap();
        // The runtime builds ammo.js bodies from the level file, so the triangles are
        // inline and not in the mesh as well.
        let collision = &level["entities"][0]["collision"];
        assert!(
            collision["triangles"].is_array() || collision["hulls"].is_array(),
            "{collision}"
        );
        assert_eq!(
            gltf_mesh_count(&directory.join("arena/worldspawn.glb")),
            1,
            "a collision mesh was written into the GLB"
        );
        let runtime = std::fs::read_to_string(directory.join("heron_playcanvas.js")).unwrap();
        assert!(runtime.contains("export async function loadLevel"));
        let page = std::fs::read_to_string(directory.join("arena.html")).unwrap();
        assert!(page.contains(&format!(
            "playcanvas@{}/build/playcanvas.min.js",
            writer::playcanvas::PLAYCANVAS_VERSION
        )));
    }

    #[test]
    fn bevy_and_three_name_their_sky_in_the_level_environment() {
        for (target, file) in [
            (Target::Bevy, "arena/sky.ktx2"),
            (Target::ThreeJs, "arena/sky.hdr"),
            (Target::Babylon, "arena/sky.hdr"),
            (Target::PlayCanvas, "arena/sky.hdr"),
        ] {
            let (_, directory, _scratch) = export_cube(target, "portable-sky");
            let level: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(directory.join("arena.json")).unwrap(),
            )
            .unwrap();
            let environment = &level["environment"];
            assert!(
                environment["to_sun"]
                    .as_array()
                    .is_some_and(|v| v.len() == 3),
                "{target:?}"
            );
            assert!(directory.join(file).is_file(), "{target:?}");
            assert_eq!(
                environment[if target == Target::Bevy {
                    "cubemap"
                } else {
                    "panorama"
                }],
                file
            );
        }
        // Babylon.js reads its panorama a quarter turn on from three.js's layout, so the
        // two targets' sky.hdr differ.
        let three = export_cube(Target::ThreeJs, "three-sky-turn");
        let babylon = export_cube(Target::Babylon, "babylon-sky-turn");
        assert_ne!(
            std::fs::read(three.1.join("arena/sky.hdr")).unwrap(),
            std::fs::read(babylon.1.join("arena/sky.hdr")).unwrap()
        );
        // PlayCanvas's skybox reads it half a turn on from Babylon.js's.
        let playcanvas = export_cube(Target::PlayCanvas, "playcanvas-sky-turn");
        assert_ne!(
            std::fs::read(babylon.1.join("arena/sky.hdr")).unwrap(),
            std::fs::read(playcanvas.1.join("arena/sky.hdr")).unwrap()
        );
        let (_, directory, _scratch) = export_cube(Target::Bevy, "bevy-sky-module");
        assert!(std::fs::read_to_string(directory.join("heron_sky.rs"))
            .unwrap()
            .contains("pub struct HeronSky"));
    }
    #[test]
    fn a_map_without_a_render_camera_takes_the_sun_and_exposure_of_the_3d_view() {
        let view = crate::render_settings::from_preferences();
        let level = scene_export::level::Level {
            format: 2,
            generator: String::new(),
            name: "arena".into(),
            scale: 32.,
            textures: Vec::new(),
            entities: Vec::new(),
        };
        let sky = map_sky(&one_cube(), &level);
        assert_eq!(sky.sun_azimuth, view.sun_azimuth);
        assert_eq!(sky.sun_elevation, view.sun_elevation);
        assert_eq!(sky.exposure, view.exposure);
        // Not a default render camera's, which differ from the view's own defaults.
        assert_ne!(
            sky.sun_azimuth,
            crate::render_camera::Settings::default().sun_azimuth
        );
    }
    #[test]
    fn godot_exports_the_sky_as_a_world_environment_unless_asked_not_to() {
        let (_, directory, _scratch) = export_cube(Target::Godot, "godot-sky");
        let scene = std::fs::read_to_string(directory.join("arena.tscn")).unwrap();
        assert!(
            scene.contains(r#"[node name="HeronEnvironment" type="WorldEnvironment" parent="."]"#),
            "{scene}"
        );
        assert!(scene.contains("res://arena/sky.hdr"), "{scene}");
        assert!(std::fs::read(directory.join("arena/sky.hdr"))
            .unwrap()
            .starts_with(b"#?RADIANCE"));
        let scratch = Scratch::new("godot-no-sky");
        export(
            &one_cube(),
            &scratch.0,
            &Options {
                target: Target::Godot,
                name: "arena".into(),
                sky: false,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(!std::fs::read_to_string(scratch.0.join("arena.tscn"))
            .unwrap()
            .contains("WorldEnvironment"));
        assert!(!scratch.0.join("arena/sky.hdr").exists());
    }
    #[test]
    fn stride_export_inside_a_package_takes_its_content_url_and_root_assets() {
        let scratch = Scratch::new("stride-package");
        let package = scratch.0.join("Game");
        std::fs::create_dir_all(package.join("Assets")).unwrap();
        std::fs::write(package.join("Game.sdpkg"),"!Package\nAssetFolders:\n    -   Path: !dir Assets\nRootAssets:\n    - AAAA:MainScene\n").unwrap();
        let options = Options {
            target: Target::Stride,
            name: "arena".into(),
            ..Options::default()
        };
        let directory = package.join("Assets/Maps");
        let report = export(&one_cube(), &directory, &options).unwrap();
        assert!(
            !report.warnings.iter().any(|w| w.contains("Stride:")),
            "{:?}",
            report.warnings
        );
        let text = std::fs::read_to_string(package.join("Game.sdpkg")).unwrap();
        let id = scene_export::writer::stride::asset_id("Maps/arena/worldspawn");
        assert!(
            text.contains("    - AAAA:MainScene\n")
                && text.contains(&format!("    - {id}:Maps/arena/worldspawn\n")),
            "{text}"
        );
        assert!(
            std::fs::read_to_string(directory.join("arena/HeronBrushStrideLevel.cs"))
                .unwrap()
                .contains("\"Maps/arena/worldspawn\"")
        );
        export(&one_cube(), &directory, &options).unwrap();
        assert_eq!(
            std::fs::read_to_string(package.join("Game.sdpkg")).unwrap(),
            text,
            "re-export leaves the package as it was"
        );
    }
    #[test]
    fn three_exports_a_playable_html_with_embedded_glb() {
        let scratch = Scratch::new("three-html");
        let directory = &scratch.0;
        std::fs::create_dir_all(directory.join("textures")).unwrap();
        let png = include_bytes!("../../heron-brush/assets/engines/threejs.png");
        std::fs::write(directory.join("textures/stone.png"), png).unwrap();
        let report = export(
            &one_cube(),
            directory,
            &Options {
                target: Target::ThreeJs,
                name: "arena".into(),
                project_dir: directory.to_string_lossy().into_owned(),
                texture_dir: "res://textures".into(),
                ..Options::default()
            },
        )
        .unwrap();
        let path = directory.join("arena.html");
        assert!(report.files.contains(&path));
        let html = std::fs::read_to_string(path).unwrap();
        let json = html
            .split("id=\"level-data\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let embedded = data["meshes"]["arena/worldspawn.glb"].as_str().unwrap();
        let preview = data_encoding::BASE64.decode(embedded.as_bytes()).unwrap();
        let original = std::fs::read(directory.join("arena/worldspawn.glb")).unwrap();
        assert!(
            preview.len() < original.len(),
            "albedo is stored once in the HTML texture table"
        );
        let json_of = |bytes: &[u8]| {
            let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            serde_json::from_slice::<serde_json::Value>(&bytes[20..20 + length]).unwrap()
        };
        let compact = json_of(&preview);
        let complete = json_of(&original);
        assert_eq!(compact["meshes"], complete["meshes"]);
        assert_eq!(compact["accessors"], complete["accessors"]);
        assert_eq!(compact["images"].as_array().unwrap().len(), 0);
        assert!(!complete["images"].as_array().unwrap().is_empty());
        assert!(data["level"]["entities"][0]["collision"].is_object());
        let image = data["textures"]["stone"].as_str().expect("embedded albedo");
        assert_eq!(
            data_encoding::BASE64
                .decode(
                    image
                        .strip_prefix("data:image/png;base64,")
                        .unwrap()
                        .as_bytes()
                )
                .unwrap(),
            png
        );
    }

    #[test]
    fn defold_writes_a_collection_and_its_components() {
        let (report, _directory, _scratch) = export_cube(Target::Defold, "defold");
        let names: Vec<String> = report
            .files
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|name| name.ends_with(".collection")),
            "no collection in {names:?}"
        );
    }

    #[test]
    fn the_geometry_actually_comes_out() {
        let (report, _directory, _scratch) = export_cube(Target::ThreeJs, "triangles");
        // A cube is six quads, two triangles each.
        assert_eq!(report.triangles, 12, "warnings: {:?}", report.warnings);
    }

    #[test]
    fn a_map_with_nothing_in_it_still_writes_a_level() {
        let document = Document::new(WorldBounds::default());
        let scratch = Scratch::new("empty");
        let report = export(
            &document,
            &scratch.0,
            &Options {
                target: Target::ThreeJs,
                name: "empty".to_string(),
                ..Options::default()
            },
        )
        .expect("an empty map is not an error");
        assert_eq!(report.triangles, 0);
        assert!(!report.files.is_empty());
    }

    #[test]
    fn the_entity_classes_reach_the_builder() {
        // A solid class the FGD names must come out as an entity of that class rather
        // than as whatever the builder falls back to.
        let mut document = one_cube();
        let scratch = Scratch::new("classes");
        let fgd = scratch.0.join("test.fgd");
        std::fs::write(
            &fgd,
            "@SolidClass = func_detail : \"Detail\" []\n\
             @PointClass size(-16 -16 -16, 16 16 16) = info_player_start : \"Start\" []\n",
        )
        .expect("the fgd is written");
        document.load_definitions(&fgd).expect("it loads");

        let report = export(
            &document,
            &scratch.0,
            &Options {
                target: Target::ThreeJs,
                name: "classes".to_string(),
                ..Options::default()
            },
        )
        .expect("it exports");

        assert!(
            report
                .warnings
                .iter()
                .all(|warning| !warning.contains("test.fgd")),
            "the fgd was not read: {:?}",
            report.warnings
        );
        let written =
            std::fs::read_to_string(scratch.0.join("classes.json")).expect("the level is there");
        assert!(written.contains("worldspawn"), "{written}");
    }

    #[test]
    fn a_save_writes_the_engines_that_are_set_to_keep_up() {
        let dir = std::env::temp_dir().join(format!("heron-on-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let out = dir.join("levels");
        std::fs::create_dir_all(&out).expect("a temp folder");

        // Written the way the export window writes it: godot on and pointed at the
        // folder, three.js exported once but with the switch off.
        let store = dir.join("export-settings");
        std::fs::write(
            &store,
            format!(
                "godot=32 1 1 1 1|||||||{}|1\nthree=32 1 1 1 1|||||||{}|0\n",
                out.display(),
                out.display()
            ),
        )
        .expect("a settings file");
        let settings = crate::export_settings::Settings::load_from(&store);

        let said = export_on_save(&one_cube(), &settings);
        assert_eq!(
            said.len(),
            1,
            "only the engine with the switch on: {said:?}"
        );
        let written = said[0].as_ref().expect("the export was written");
        assert!(written.to_string().starts_with("Godot"), "{said:?}");
        assert!(out.join("level.tscn").is_file(), "the scene was written");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_engine_that_has_never_been_exported_is_left_alone() {
        // The switch on with nowhere to write is not a place to guess at.
        let dir = std::env::temp_dir().join(format!("heron-on-save-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp folder");
        let store = dir.join("export-settings");
        std::fs::write(&store, "godot=32 1 1 1 1|||||||\n").expect("a settings file");

        let settings = crate::export_settings::Settings::load_from(&store);
        assert!(export_on_save(&one_cube(), &settings).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_check_writes_nothing_and_still_reports_the_geometry() {
        let dir = std::env::temp_dir().join(format!("heron-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp folder");

        let options = Options {
            target: Target::Godot,
            name: "level".to_string(),
            ..Options::default()
        };
        let report = check(&one_cube(), &dir, &options).expect("the check runs");

        assert!(report.files.is_empty(), "a check writes nothing");
        assert!(report.triangles > 0, "it still built the mesh");
        // A cube is sound, so nothing the checks look for is there to find. What is
        // left is the build's ordinary grumbling: no FGD and no `project.godot` in a
        // bare temp folder.
        assert!(
            report
                .warnings
                .iter()
                .all(|w| w.contains("project root") || w.contains("entity definition")),
            "{:?}",
            report.warnings
        );
        assert!(
            std::fs::read_dir(&dir)
                .expect("the folder is there")
                .next()
                .is_none(),
            "the folder is still empty"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_geometry_checks_are_off_for_an_ordinary_export() {
        // They cost a pass over every triangle of the finished mesh, so an export that
        // nobody asked to check does not pay for it. The difference is visible from
        // outside: `check` turns them on and `export` does not.
        let dir = std::env::temp_dir().join(format!("heron-check-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp folder");

        let options = Options {
            target: Target::Godot,
            name: "level".to_string(),
            ..Options::default()
        };
        assert!(!options.settings(&dir).check_geometry);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// How many glTF meshes a `.glb` holds.
    fn gltf_mesh_count(path: &std::path::Path) -> usize {
        let bytes = std::fs::read(path).expect("the mesh is there");
        let json_length =
            u32::from_le_bytes(bytes[12..16].try_into().expect("a glb header")) as usize;
        let json: serde_json::Value =
            serde_json::from_slice(&bytes[20..20 + json_length]).expect("the glTF json");
        json["meshes"].as_array().map(Vec::len).unwrap_or(0)
    }

    #[test]
    fn collision_is_written_once_per_engine_not_twice() {
        // Every triangle of the collision can go in the level file as text or in the
        // `.glb` as a mesh, and writing both doubles what a loader downloads for
        // geometry it draws none of. Which one an engine gets is what this pins down.
        let dir = std::env::temp_dir().join(format!("heron-collision-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        // three.js reads the triangles out of the level file, so the mesh holds only
        // what it draws.
        let three = dir.join("three");
        std::fs::create_dir_all(&three).expect("a temp folder");
        export(
            &one_cube(),
            &three,
            &Options {
                target: Target::ThreeJs,
                name: "level".to_string(),
                ..Options::default()
            },
        )
        .expect("it exports");
        let level: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(three.join("level.json")).unwrap())
                .expect("the level reads");
        // Inline, which for a shape made of convex hulls is `hulls` and for one concave
        // shape is `triangles`.
        let collision = &level["entities"][0]["collision"];
        assert!(
            collision["triangles"].is_array() || collision["hulls"].is_array(),
            "{collision}"
        );
        assert_eq!(
            gltf_mesh_count(&three.join("level/worldspawn.glb")),
            1,
            "what it draws, and nothing else"
        );

        let raylib=dir.join("raylib");std::fs::create_dir_all(&raylib).unwrap();
        export(&one_cube(),&raylib,&Options{target:Target::Raylib,name:"level".into(),..Default::default()}).unwrap();
        let data:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(raylib.join("level.json")).unwrap()).unwrap();
        assert!(data["entities"][0]["collision"]["triangles"].is_array());
        assert_eq!(gltf_mesh_count(&raylib.join("level/worldspawn.glb")),1,"raylib consumes inline collision, not an orphan mesh");

        // Bevy reads the collision out of the glTF mesh, so the level file does not
        // carry a copy of it.
        let bevy = dir.join("bevy");
        std::fs::create_dir_all(&bevy).expect("a temp folder");
        export(
            &one_cube(),
            &bevy,
            &Options {
                target: Target::Bevy,
                name: "level".to_string(),
                ..Options::default()
            },
        )
        .expect("it exports");
        let level: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bevy.join("level.json")).unwrap())
                .expect("the level reads");
        let collision = &level["entities"][0]["collision"];
        assert!(
            collision["triangles"].is_null() && collision["hulls"].is_null(),
            "the geometry is in the mesh, not here: {collision}"
        );
        assert_eq!(
            gltf_mesh_count(&bevy.join("level/worldspawn.glb")),
            2,
            "what it draws, and the collision beside it"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_defold_resource_path_follows_where_the_files_go() {
        use std::path::Path;

        let root = Path::new("/home/somebody/game");
        // The meshes land in `<destination>/<name>`, and Defold names them from the
        // project root, so the path is that folder seen from there.
        assert_eq!(
            defold_asset_dir(
                Path::new("/home/somebody/game/main/levels"),
                "arena",
                Some(root)
            ),
            "/main/levels/arena"
        );
        // Written at the top of the project.
        assert_eq!(defold_asset_dir(root, "arena", Some(root)), "/arena");
        // No project to be relative to: the best guess is the top of one.
        assert_eq!(
            defold_asset_dir(Path::new("/tmp/out"), "arena", None),
            "/arena"
        );
    }
}
