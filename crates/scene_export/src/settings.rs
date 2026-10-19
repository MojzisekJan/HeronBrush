//! Knobs for a build, and the defaults they start from.

pub const EXPORT_HIDDEN_TEXTURE: &str = "__heron_export_hidden";

use crate::gameconfig::GameConfig;
use std::path::PathBuf;

/// How collision is generated for a solid entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionKind {
    None,
    /// One `ConvexPolygonShape3D` per brush.
    Convex,
    /// A single `ConcavePolygonShape3D` built from every triangle.
    Concave,
}

impl CollisionKind {
    pub fn parse(s: &str) -> Option<CollisionKind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" | "off" => Some(CollisionKind::None),
            "convex" => Some(CollisionKind::Convex),
            "concave" | "trimesh" => Some(CollisionKind::Concave),
            _ => None,
        }
    }
}

/// Where a solid entity puts its own origin, which its geometry is then made
/// relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginKind {
    /// Centre of the brushes textured with the origin texture, falling back to
    /// the bounding box centre when there are none.
    Brush,
    BoundsCenter,
    BoundsMins,
    BoundsMaxs,
    /// Average of every vertex.
    Averaged,
    /// The `origin` property, in world space.
    Absolute,
    /// The `origin` property, added to the bounding box centre.
    Relative,
}

impl OriginKind {
    pub fn parse(s: &str) -> Option<OriginKind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "brush" => Some(OriginKind::Brush),
            "bounds_center" | "center" => Some(OriginKind::BoundsCenter),
            "bounds_mins" | "mins" => Some(OriginKind::BoundsMins),
            "bounds_maxs" | "maxs" => Some(OriginKind::BoundsMaxs),
            "averaged" | "average" => Some(OriginKind::Averaged),
            "absolute" => Some(OriginKind::Absolute),
            "relative" => Some(OriginKind::Relative),
            _ => None,
        }
    }
}

/// What happens to a solid entity's brushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnKind {
    /// Its own node, positioned at its origin.
    Entity,
    /// Brushes are handed to worldspawn and the entity disappears.
    MergeWorldspawn,
}

impl SpawnKind {
    pub fn parse(s: &str) -> Option<SpawnKind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "entity" => Some(SpawnKind::Entity),
            "merge_worldspawn" | "merge" | "group" => Some(SpawnKind::MergeWorldspawn),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    /// Generated face paints supplied by the editor, available before files are written.
    pub generated_materials: indexmap::IndexMap<String, crate::materials::GeneratedMaterial>,
    /// Tangent-space normal maps supplied by the editor, keyed by texture or generated
    /// material name: PNG bytes already in the glTF/OpenGL convention (+Y points along
    /// -V, so a DirectX map arrives with its green channel inverted). GLB writers embed
    /// them as `normalTexture` and add a `TANGENT` attribute to the primitives using them.
    pub normal_maps: indexmap::IndexMap<String, std::sync::Arc<[u8]>>,
    /// Map units to Godot units. 1/32 puts a 32 unit Quake grid on 1 metre.
    pub scale_factor: f64,
    /// Textures with these names drive tool behaviour rather than looks.
    pub clip_texture: String,
    pub skip_texture: String,
    pub origin_texture: String,
    /// `res://` directory holding texture images.
    pub texture_dir: String,
    /// `res://` directory searched for hand-made `.tres` materials.
    pub material_dir: Option<String>,
    pub texture_extensions: Vec<String>,
    pub material_extensions: Vec<String>,
    /// Size assumed for textures that cannot be found on disk.
    pub fallback_texture_size: f64,
    /// Rebuild map layers and groups as Node3D parents.
    pub group_hierarchy: bool,
    /// Throw away faces and brushes buried inside the same solid, and report
    /// the ones buried by a different one. Both come from the same look at the
    /// map, so they go on and off together.
    pub cull_hidden: bool,
    /// Join neighbouring brushes into single collision shapes where the result
    /// is still convex. The mesh is untouched; this is only what the physics
    /// engine holds.
    pub merge_collision: bool,
    /// Join neighbouring faces that share a plane, a texture and an alignment
    /// into single polygons, where the result is still convex.
    pub merge_faces: bool,
    /// Static world mesh cells in map units; zero keeps one world mesh.
    pub static_cell_size: f64,
    /// Entity property that switches on smooth shading.
    pub smoothing_property: String,
    pub smoothing_angle_property: String,
    /// Entity property holding a vertex snapping distance in map units.
    pub vertex_merge_property: String,
    /// Entity property that switches on removal of faces buried between brushes.
    pub cull_interior_property: String,
    /// Entity property whose value names the generated node.
    pub name_property: Option<String>,
    /// Default node class for point entities with no `@heron_brush node=` directive.
    pub default_point_class: String,
    /// Default node class for solid entities.
    pub default_solid_class: String,
    /// Physics layer for the areas heron_brush makes, 1 to 32, or 0 to leave it alone. On
    /// the level's own layer a trigger stops every ray the game traces. The mask
    /// is untouched, so it still notices the player.
    pub trigger_layer: u32,
    /// How finely a bezier patch is divided, per three by three piece of its
    /// control grid, when the map does not say. `patchDef3` says.
    pub patch_subdivisions: usize,
    /// Check the finished geometry and report what is wrong with it. Costs
    /// time, so `heron_brush check` asks for it and an ordinary build does not.
    pub check_geometry: bool,
    /// Ask Godot's importer to unwrap a second UV set, which baked light needs.
    pub lightmap_uv2: bool,
    /// How big one lightmap texel is, in Godot units.
    pub lightmap_texel_size: f64,
    /// Folders searched for the `.wad` packages a map's `wad` key names.
    pub wad_dirs: Vec<std::path::PathBuf>,
    /// A `palette.lmp` to colour WAD2 textures with. WAD3 carries its own.
    pub palette: Option<std::path::PathBuf>,
    /// Write textures found in a package out as PNG, so Godot has an image to
    /// import. Off leaves the sizes right and the materials missing.
    pub extract_wad_textures: bool,
    pub default_collision: CollisionKind,
    pub default_origin: OriginKind,
    /// Reverse triangle winding in the written `.obj` files. The importer
    /// flips winding on its way in, so this only matters if a Godot version
    /// stops doing that.
    pub flip_faces: bool,
    /// Write the V texture coordinate as-is. OBJ measures V from the bottom of
    /// the image and Godot's importer flips it back, so this is normally left
    /// off. Turn it on if textures come out upside down.
    pub flip_uv_v: bool,
    /// Godot project root, used to turn file paths into `res://` paths.
    pub project_root: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scale_factor: 1.0 / 32.0,
            clip_texture: "clip".into(),
            skip_texture: "skip".into(),
            origin_texture: "origin".into(),
            texture_dir: "res://textures".into(),
            material_dir: None,
            texture_extensions: [
                "png", "jpg", "jpeg", "bmp", "tga", "webp", "svg", "exr", "hdr",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            material_extensions: ["tres", "res", "material"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            fallback_texture_size: 32.0,
            generated_materials: Default::default(),
            normal_maps: Default::default(),
            group_hierarchy: true,
            cull_hidden: true,
            merge_collision: true,
            merge_faces: true,
            static_cell_size: 0.0,
            smoothing_property: "_phong".into(),
            smoothing_angle_property: "_phong_angle".into(),
            vertex_merge_property: "_vertex_merge_distance".into(),
            cull_interior_property: "_cull_interior_faces".into(),
            name_property: None,
            trigger_layer: 3,
            patch_subdivisions: 6,
            check_geometry: false,
            lightmap_uv2: false,
            lightmap_texel_size: 0.2,
            wad_dirs: Vec::new(),
            palette: None,
            extract_wad_textures: true,
            default_point_class: "Marker3D".into(),
            default_solid_class: "StaticBody3D".into(),
            default_collision: CollisionKind::Convex,
            default_origin: OriginKind::Brush,
            flip_faces: false,
            flip_uv_v: false,
            project_root: None,
        }
    }
}

impl Settings {
    /// Fills in what the `GameConfig.cfg` already knows: tool textures,
    /// the texture directory and the world scale.
    pub fn apply_game_config(&mut self, config: &GameConfig) {
        if let Some(t) = config.tool_texture("clip") {
            self.clip_texture = t;
        }
        if let Some(t) = config.tool_texture("skip") {
            self.skip_texture = t;
        }
        if let Some(t) = config.tool_texture("origin") {
            self.origin_texture = t;
        }
        if let Some(root) = &config.materials_root {
            self.texture_dir = if root.starts_with("res://") {
                root.clone()
            } else {
                format!(
                    "res://{}",
                    root.trim_start_matches("./").trim_start_matches('/')
                )
            };
        }
        if !config.materials_extensions.is_empty() {
            self.texture_extensions = config.materials_extensions.clone();
        }
        if let Some(scale) = config.entity_scale {
            if scale > 0.0 {
                self.scale_factor = 1.0 / scale;
                self.fallback_texture_size = scale;
            }
        }
    }

    pub fn is_clip(&self, texture: &str) -> bool {
        matches_tool(texture, &self.clip_texture)
    }

    pub fn is_skip(&self, texture: &str) -> bool {
        texture == EXPORT_HIDDEN_TEXTURE || matches_tool(texture, &self.skip_texture)
    }

    pub fn is_origin(&self, texture: &str) -> bool {
        matches_tool(texture, &self.origin_texture)
    }

    pub fn is_tool_texture(&self, texture: &str) -> bool {
        self.is_clip(texture) || self.is_skip(texture) || self.is_origin(texture)
    }
}

/// Matches a tool texture by its file name, whichever folder it sits in, so
/// `clip`, `tools/clip` and `special/CLIP` all count. Globs are ignored.
fn matches_tool(texture: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_matches('*');
    if pattern.is_empty() {
        return false;
    }
    if texture.eq_ignore_ascii_case(pattern) {
        return true;
    }
    let leaf = |s: &str| s.rsplit('/').next().unwrap_or(s).to_string();
    leaf(texture).eq_ignore_ascii_case(&leaf(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_textures_match_in_any_folder() {
        let s = Settings::default();
        assert!(s.is_clip("clip"));
        assert!(s.is_clip("CLIP"));
        assert!(s.is_clip("tools/clip"));
        assert!(s.is_clip("special/tb/clip"));
        assert!(s.is_skip("tools/skip"));
        assert!(s.is_origin("tools/origin"));
    }

    #[test]
    fn ordinary_textures_are_left_alone() {
        let s = Settings::default();
        assert!(!s.is_clip("clipboard"));
        assert!(!s.is_clip("metal/clip_panel"));
        assert!(!s.is_skip("skipping_stones"));
        assert!(!s.is_tool_texture("wall"));
    }

    #[test]
    fn a_configured_folder_still_matches_bare_names() {
        // A game config that names the texture with its folder keeps working
        // for maps painted before the tool textures were moved.
        let s = Settings {
            clip_texture: "tools/clip".into(),
            ..Default::default()
        };
        assert!(s.is_clip("tools/clip"));
        assert!(s.is_clip("clip"));
    }

    #[test]
    fn a_glob_in_the_pattern_is_ignored() {
        let s = Settings {
            skip_texture: "*skip".into(),
            ..Default::default()
        };
        assert!(s.is_skip("skip"));
        assert!(s.is_skip("tools/skip"));
    }
}
