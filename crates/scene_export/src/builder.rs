//! Ties everything together: a parsed map plus entity definitions in, a Godot
//! scene and a set of `.obj` meshes out.

use crate::classdb::{self, PropType};
use crate::fgd::{ClassKind, EntityClass, Property};
use crate::geometry::{self, SolidGeometry, SolidOptions};
use crate::level::{self, Level};
use crate::map::{Entity, GroupKind, MapFile};
use crate::materials::{self, Material, MaterialLibrary};
use crate::math::{self, Vec3};
use crate::scene::{self, Connection, GdValue, Node, Scene, SubResource};
use crate::settings::{CollisionKind, OriginKind, Settings, SpawnKind};
use crate::writer::obj::ObjOptions;
use indexmap::{IndexMap, IndexSet};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Node name of the scene root, normally the map file's stem.
    pub scene_name: String,
    /// `res://` directory the `.obj` files go into.
    pub mesh_dir: String,
}

/// The most surfaces one Godot mesh holds (`RenderingServer::MAX_MESH_SURFACES`).
const GODOT_MAX_SURFACES: usize = 256;

#[derive(Debug, Clone)]
pub struct MeshFile {
    pub file_name: String,
    pub contents: String,
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Faces thrown away because another brush of the same solid was flat
    /// against them.
    pub buried_faces: usize,
    /// Faces dropped because two brushes press together back to back.
    pub pressed_faces: usize,
    pub welded_corners: usize,
    /// Whole brushes thrown away because another brush of the same solid had
    /// swallowed them.
    pub buried_brushes: usize,
    /// Collision shapes saved by joining neighbouring brushes.
    pub merged_hulls: usize,
    /// Faces saved by joining neighbours that share a plane and a texture.
    pub merged_faces: usize,
    pub entities: usize,
    pub brushes: usize,
    pub surfaces: usize,
    pub triangles: usize,
    pub collision_shapes: usize,
    pub occluders: usize,
}

#[derive(Debug)]
pub struct BuildResult {
    pub scene: Scene,
    pub meshes: Vec<MeshFile>,
    pub warnings: Vec<String>,
    pub stats: Stats,
}

/// How one entity class turns into a node.
#[derive(Debug, Clone)]
struct Recipe {
    node_class: String,
    instance: Option<String>,
    script: Option<String>,
    collision: CollisionKind,
    origin: OriginKind,
    spawn: SpawnKind,
    build_visuals: bool,
    /// Build an OccluderInstance3D so this geometry hides what is behind it.
    occluder: bool,
    groups: Vec<String>,
    /// Write every property straight onto the node, not only the known ones.
    auto: bool,
    /// Entity property naming another entity, `target` by default.
    link_property: String,
    /// Entity property this entity is known by, `targetname` by default.
    link_name_property: String,
    /// Signals to wire to the linked entity, as (signal, method) pairs.
    connect: Vec<(String, String)>,
    name_property: Option<String>,
    is_solid: bool,
}

fn resolve_recipe(entity: &Entity, class: Option<&EntityClass>, settings: &Settings) -> Recipe {
    let classname = entity.classname();
    let external = !entity.external_surfaces.is_empty();
    let is_solid = external || match class {
        Some(c) => c.kind == ClassKind::Solid,
        None => !entity.brushes.is_empty() || !entity.patches.is_empty(),
    };

    let node_class = default_node_class(classname, is_solid && !external, settings);
    let mut recipe = Recipe {
        node_class,
        instance: None,
        script: None,
        collision: if is_solid && !external {
            settings.default_collision
        } else {
            CollisionKind::None
        },
        origin: settings.default_origin,
        spawn: SpawnKind::Entity,
        build_visuals: true,
        occluder: false,
        groups: Vec::new(),
        auto: false,
        link_property: "target".to_string(),
        link_name_property: "targetname".to_string(),
        connect: Vec::new(),
        name_property: settings.name_property.clone(),
        is_solid,
    };

    let Some(class) = class else { return recipe };
    let d = &class.directives;

    if let Some(v) = d.get("node") {
        recipe.node_class = v.to_string();
    }
    if let Some(v) = d.get("scene") {
        recipe.instance = Some(as_res_path(v));
    } else if let Some(model) = class.model_path() {
        if is_instanceable(&model) {
            recipe.instance = Some(as_res_path(&model));
        }
    }
    if let Some(v) = d.get("script") {
        recipe.script = Some(as_res_path(v));
    }
    if let Some(v) = d.get("collision").and_then(CollisionKind::parse) {
        recipe.collision = v;
    }
    if let Some(v) = d.get("origin").and_then(OriginKind::parse) {
        recipe.origin = v;
    }
    if let Some(v) = d.get("spawn").and_then(SpawnKind::parse) {
        recipe.spawn = v;
    }
    if let Some(v) = d.bool("visuals") {
        recipe.build_visuals = v;
    }
    if let Some(v) = d.bool("occluder") {
        recipe.occluder = v;
    }
    if let Some(v) = d.bool("auto") {
        recipe.auto = v;
    }
    if let Some(v) = d.get("name_property") {
        recipe.name_property = Some(v.to_string());
    }
    recipe.groups = d.list("groups");
    if let Some(v) = d.get("link_property") {
        recipe.link_property = v.to_string();
    }
    if let Some(v) = d.get("link_name_property") {
        recipe.link_name_property = v.to_string();
    }
    // connect=body_entered:_on_trigger,body_exited:_on_leave
    for pair in d.list("connect") {
        if let Some((signal, method)) = pair.split_once(':') {
            recipe
                .connect
                .push((signal.trim().to_string(), method.trim().to_string()));
        }
    }

    // An entity can point at its own script, the way it can at its own scene.
    if let Some(value) = entity.get("script") {
        let value = value.trim();
        if !value.is_empty() && value != "res://" {
            recipe.script = Some(as_res_path(value));
        }
    }

    // An entity can also name its own scene, which beats anything the class says.
    let scene_property = d.get("scene_property").unwrap_or("scene");
    if let Some(value) = entity.get(scene_property) {
        let value = value.trim();
        if !value.is_empty() && value != "res://" && is_instanceable(value) {
            recipe.instance = Some(as_res_path(value));
        }
    }
    recipe
}

fn default_node_class(classname: &str, is_solid: bool, settings: &Settings) -> String {
    let lower = classname.to_ascii_lowercase();
    if is_solid {
        if lower.starts_with("trigger") {
            return "Area3D".to_string();
        }
        return settings.default_solid_class.clone();
    }
    if lower.starts_with("light") || lower.contains("_light") {
        if lower.contains("spot") {
            return "SpotLight3D".to_string();
        }
        if lower.contains("sun") || lower.contains("directional") || lower.contains("environment") {
            return "DirectionalLight3D".to_string();
        }
        return "OmniLight3D".to_string();
    }
    settings.default_point_class.clone()
}

fn is_instanceable(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".tscn", ".scn", ".glb", ".gltf", ".dae", ".fbx", ".blend", ".obj",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

fn as_res_path(path: &str) -> String {
    if path.starts_with("res://") || path.starts_with("uid://") {
        path.to_string()
    } else {
        format!(
            "res://{}",
            path.trim_start_matches("./").trim_start_matches('/')
        )
    }
}

/// Everything both writers need before either of them makes anything: the
/// entities that survive the omitted layers, the build recipe for each one,
/// the geometry thrown away as buried, and the material library.
///
/// The Godot path goes on to build nodes out of this, and the neutral one
/// builds a `Level`. Splitting here is what keeps the two from drifting apart.
struct Prepared {
    entities: Vec<Entity>,
    recipes: Vec<Recipe>,
    library: MaterialLibrary,
    warnings: Vec<String>,
    stats: Stats,
}

fn prepare(
    map: &MapFile,
    classes: &IndexMap<String, EntityClass>,
    settings: &Settings,
) -> Prepared {
    let mut warnings: Vec<String> = map.warnings.clone();
    let mut stats = Stats::default();

    // Entities from layers marked "omit from export" never reach the scene.
    let omitted: Vec<i64> = map
        .groups
        .iter()
        .filter(|g| g.omit_from_export)
        .map(|g| g.id)
        .collect();

    let mut entities: Vec<Entity> = Vec::with_capacity(map.entities.len());
    let mut recipes: Vec<Recipe> = Vec::with_capacity(map.entities.len());
    let mut missing_classes: IndexSet<String> = IndexSet::new();

    for entity in &map.entities {
        if let Some(group) = map.group_of(entity) {
            if omitted.contains(&group.id) {
                continue;
            }
        }
        let class = classes.get(entity.classname());
        if class.is_none() && !entity.classname().is_empty() {
            missing_classes.insert(entity.classname().to_string());
        }
        recipes.push(resolve_recipe(entity, class, settings));
        entities.push(entity.clone());
    }

    for name in &missing_classes {
        warnings.push(format!(
            "no entity definition for `{name}`, falling back to defaults"
        ));
    }

    // Solid classes asking to merge hand their brushes to worldspawn.
    let mut merged_indices: Vec<usize> = Vec::new();
    for i in (1..entities.len()).rev() {
        if recipes[i].spawn == SpawnKind::MergeWorldspawn {
            let brushes = std::mem::take(&mut entities[i].brushes);
            if let Some(world) = entities.first_mut() {
                world.brushes.extend(brushes);
            }
            merged_indices.push(i);
        }
    }
    for i in merged_indices {
        entities.remove(i);
        recipes.remove(i);
    }

    // Geometry buried inside a solid, thrown away rather than reported: two
    // brushes of one solid never move apart.
    let mut buried_faces = 0usize;
    let mut buried_brushes = 0usize;
    if settings.cull_hidden {
        for entity in entities.iter_mut() {
            for index in crate::hidden::swallowed(&entity.brushes).into_iter().rev() {
                entity.brushes.remove(index);
                buried_brushes += 1;
            }
            let gone = crate::hidden::within(&entity.brushes, |t| settings.is_tool_texture(t));
            for (brush, face) in gone {
                // Marked with the texture that already means "do not draw
                // this", so the rest of the build needs to know nothing new.
                entity.brushes[brush].faces[face].texture = settings.skip_texture.clone();
                buried_faces += 1;
            }
        }
        stats.buried_faces = buried_faces;
        stats.buried_brushes = buried_brushes;

        // Whatever is left is buried by another solid, which might move.
        for hidden in crate::hidden::find(&entities, |t| settings.is_tool_texture(t)) {
            warnings.push(format!("hidden face: {hidden}"));
        }
    }

    // Keep collision/logic entities independent. World brushes in a cell share
    // the existing per-material mesh builder and are culled as a local chunk.
    if settings.static_cell_size.is_finite() && settings.static_cell_size>0.0 && entities.first().is_some_and(|e|e.classname()=="worldspawn") {
        let size=settings.static_cell_size;
        let cull=entities[0].get(&settings.cull_interior_property).map(|v|v!="0"&&!v.is_empty()&&!v.eq_ignore_ascii_case("false")).unwrap_or(settings.cull_hidden);
        if cull {
            for (brush,face) in crate::hidden::covered_faces(&entities[0].brushes, |t| settings.is_tool_texture(t)) {
                if !settings.is_tool_texture(&entities[0].brushes[brush].faces[face].texture) {
                    entities[0].brushes[brush].faces[face].texture=settings.skip_texture.clone();stats.buried_faces+=1;
                }
            }
        }
        let mut cells:std::collections::BTreeMap<[i64;3],Vec<crate::map::Brush>>=Default::default();
        for brush in std::mem::take(&mut entities[0].brushes) {
            let points=crate::overlap::corners_of(&brush.planes());
            let centre=if points.is_empty(){crate::math::Vec3::ZERO}else{points.iter().copied().sum::<crate::math::Vec3>()/points.len() as f64};
            let key=[(centre.x/size).floor() as i64,(centre.y/size).floor() as i64,(centre.z/size).floor() as i64];
            cells.entry(key).or_default().push(brush);
        }
        if cells.len()<=1 {entities[0].brushes=cells.into_values().flatten().collect();}
        else {
            for (cell,brushes) in cells {
                let mut entity=entities[0].clone();entity.brushes=brushes;entity.patches.clear();
                entity.properties.retain(|k,_|k=="classname" || k==&settings.smoothing_property || k==&settings.smoothing_angle_property || k==&settings.vertex_merge_property || k==&settings.cull_interior_property);
                entity.properties.insert("classname".into(),"func_static".into());
                entity.properties.insert("_tb_name".into(),format!("world_{}_{}_{}",cell[0],cell[1],cell[2]));
                let mut recipe=recipes[0].clone();recipe.name_property=Some("_tb_name".into());
                recipe.script=None;recipe.instance=None;recipe.groups.clear();recipe.connect.clear();recipe.auto=false;
                entities.push(entity);recipes.push(recipe);
            }
        }
    }

    // Every texture the map actually uses, tool textures aside.
    let mut texture_names: IndexSet<String> = IndexSet::new();
    for entity in &entities {
        for brush in &entity.brushes {
            stats.brushes += 1;
            for face in &brush.faces {
                if !settings.is_tool_texture(&face.texture) {
                    texture_names.insert(face.texture.clone());
                }
            }
        }
        for surface in &entity.external_surfaces { texture_names.insert(surface.texture.clone()); }
        for patch in &entity.patches {
            if !settings.is_tool_texture(&patch.texture) {
                texture_names.insert(patch.texture.clone());
            }
        }
    }

    // Quake keeps its textures in packages rather than loose files, and the map
    // says which. Without them the sizes are guessed and every UV is wrong.
    let wads = match entities.first().and_then(|w| w.get("wad")) {
        Some(key) if !key.trim().is_empty() => {
            let (wads, trouble) = crate::wad::open_all(key, &settings.wad_dirs);
            warnings.extend(trouble);
            wads
        }
        _ => Vec::new(),
    };

    let texture_list: Vec<String> = texture_names.iter().cloned().collect();
    let library = materials::build(&texture_list, settings, &wads);
    warnings.extend(library.warnings.iter().cloned());

    Prepared {
        entities,
        recipes,
        library,
        warnings,
        stats,
    }
}

pub fn build(
    map: &MapFile,
    classes: &IndexMap<String, EntityClass>,
    settings: &Settings,
    options: &BuildOptions,
) -> BuildResult {
    let Prepared {
        entities,
        recipes,
        library,
        mut warnings,
        mut stats,
    } = prepare(map, classes, settings);

    let mut sub_resources: Vec<SubResource> = Vec::new();
    let material_refs = build_material_resources(&library, &mut sub_resources);

    let mut root = Node::new(options.scene_name.clone(), "Node3D");
    let mut meshes: Vec<MeshFile> = Vec::new();

    // Layers and groups become Node3D parents, when asked for.
    let mut group_nodes: IndexMap<i64, Node> = IndexMap::new();
    if settings.group_hierarchy {
        for group in &map.groups {
            if group.omit_from_export {
                continue;
            }
            let mut node = Node::new(group.name.clone(), "Node3D");
            if group.kind == GroupKind::Layer {
                node.meta.push(("heron_brush_layer".into(), GdValue::Int(group.id)));
            }
            group_nodes.insert(group.id, node);
        }
    }

    let mut used_names: IndexMap<String, usize> = IndexMap::new();
    let mut placements: Vec<(Option<i64>, Node)> = Vec::new();
    // Scene path of each entity's node, so targets can be turned into NodePaths.
    let mut node_paths: Vec<String> = Vec::with_capacity(entities.len());

    for (index, entity) in entities.iter().enumerate() {
        let recipe = &recipes[index];
        let class = classes.get(entity.classname());
        let is_worldspawn = index == 0 && entity.classname() == "worldspawn";

        let node_name = entity_node_name(entity, recipe, index, &mut used_names);

        let mut node = build_entity_node(
            entity,
            class,
            recipe,
            settings,
            &node_name,
            is_worldspawn,
            &library,
            &material_refs,
            options,
            &mut sub_resources,
            &mut meshes,
            &mut stats,
            &mut warnings,
        );

        if node.class.is_empty() && node.instance.is_none() {
            node.class = "Node3D".to_string();
        }
        stats.entities += 1;

        let group_id = if settings.group_hierarchy {
            map.group_of(entity).map(|g| g.id)
        } else {
            None
        };
        node_paths.push(match group_id {
            Some(id) => format!("{}/{}", group_path(map, id), node.name),
            None => node.name.clone(),
        });
        placements.push((group_id, node));
    }

    // Resolve target and targetname into real paths, then wire the signals.
    let connections = link_entities(&entities, &recipes, &node_paths, &mut placements);

    // Groups first, so entity nodes can be dropped into them.
    let group_order: Vec<i64> = group_nodes.keys().copied().collect();
    for (group_id, node) in placements {
        match group_id.and_then(|id| group_nodes.get_mut(&id)) {
            Some(parent) => {
                parent.add_child(node);
            }
            None => {
                root.add_child(node);
            }
        }
    }

    // Nest groups inside their parents, deepest last so parents already exist.
    let mut remaining: IndexMap<i64, Node> = group_nodes;
    let mut roots: Vec<Node> = Vec::new();
    for id in group_order.iter().rev() {
        let Some(node) = remaining.shift_remove(id) else {
            continue;
        };
        let parent_id = map
            .groups
            .iter()
            .find(|g| g.id == *id)
            .and_then(|g| g.parent_id);
        match parent_id.and_then(|p| remaining.get_mut(&p)) {
            Some(parent) => parent.children.insert(0, node),
            None => roots.push(node),
        }
    }
    for node in roots.into_iter().rev() {
        root.add_child(node);
    }

    root.make_names_unique();

    let aliases = crate::instancing::aliases(meshes.iter().map(|m| (m.file_name.as_str(),m.contents.as_bytes())));
    meshes.retain(|m| !aliases.contains_key(&m.file_name));
    fn redirect(node: &mut Node, aliases: &std::collections::HashMap<String,String>, mesh_dir: &str) {
        for (_,value) in &mut node.properties {
            if let GdValue::ExtResource { path, ty } = value {
                if let Some((dir,file)) = path.rsplit_once('/') {
                    if ty == "ArrayMesh" && dir == mesh_dir {
                        if let Some(original) = aliases.get(file) { *path = format!("{dir}/{original}"); }
                    }
                }
            }
        }
        for child in &mut node.children { redirect(child,aliases,mesh_dir); }
    }
    redirect(&mut root,&aliases,&options.mesh_dir);

    BuildResult {
        scene: Scene {
            root,
            sub_resources,
            connections,
        },
        meshes,
        warnings,
        stats,
    }
}

/// Path of a layer or group node, relative to the scene root.
fn group_path(map: &MapFile, id: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = Some(id);
    let mut guard = 0;
    while let Some(gid) = current {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(group) = map.groups.iter().find(|g| g.id == gid) else {
            break;
        };
        parts.push(group.name.clone());
        current = group.parent_id;
    }
    parts.reverse();
    parts.join("/")
}

/// Path from one node to another, both given relative to the scene root.
fn relative_path(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|p| !p.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|p| !p.is_empty()).collect();

    let mut common = 0;
    while common < from_parts.len()
        && common < to_parts.len()
        && from_parts[common] == to_parts[common]
    {
        common += 1;
    }

    let mut out = String::new();
    for _ in common..from_parts.len() {
        out.push_str("../");
    }
    out.push_str(&to_parts[common..].join("/"));
    if out.is_empty() {
        ".".to_string()
    } else {
        out.trim_end_matches('/').to_string()
    }
}

/// Turns `target` and `targetname` into NodePaths and signal connections.
///
/// The map only knows names, so they are resolved here while the whole
/// scene is in hand, and nothing has to look anything up at run time.
fn link_entities(
    entities: &[Entity],
    recipes: &[Recipe],
    node_paths: &[String],
    placements: &mut [(Option<i64>, Node)],
) -> Vec<Connection> {
    // Several entities may share one targetname, as they do in Quake.
    let mut by_name: IndexMap<String, Vec<usize>> = IndexMap::new();
    for (i, entity) in entities.iter().enumerate() {
        let key = entity
            .get(&recipes[i].link_name_property)
            .unwrap_or_default()
            .trim();
        if !key.is_empty() {
            by_name.entry(key.to_string()).or_default().push(i);
        }
    }

    let mut connections: Vec<Connection> = Vec::new();
    for (i, entity) in entities.iter().enumerate() {
        let recipe = &recipes[i];
        let target = entity.get(&recipe.link_property).unwrap_or_default().trim();
        if target.is_empty() {
            continue;
        }
        let Some(targets) = by_name.get(target) else {
            continue;
        };

        // The NodePath sits on this node, so it has to be written relative to
        // this node, not to the scene root.
        let paths: Vec<GdValue> = targets
            .iter()
            .map(|t| GdValue::NodePath(relative_path(&node_paths[i], &node_paths[*t])))
            .collect();
        placements[i]
            .1
            .meta
            .push(("target_paths".into(), GdValue::Array(paths)));

        for (signal, method) in &recipe.connect {
            for t in targets {
                // A connection to a node with no script would fail to load.
                if recipes[*t].script.is_none() && recipes[*t].instance.is_none() {
                    continue;
                }
                connections.push(Connection {
                    signal: signal.clone(),
                    from: node_paths[i].clone(),
                    to: node_paths[*t].clone(),
                    method: method.clone(),
                });
            }
        }
    }
    connections
}

fn entity_node_name(
    entity: &Entity,
    recipe: &Recipe,
    index: usize,
    used: &mut IndexMap<String, usize>,
) -> String {
    let mut base = if let Some(prop) = &recipe.name_property {
        entity
            .get(prop)
            .filter(|v| !v.is_empty())
            .map(crate::map::sanitize_node_name)
    } else {
        None
    };
    if base.is_none() {
        let classname = entity.classname();
        base = Some(if classname.is_empty() {
            format!("entity_{index}")
        } else {
            crate::map::sanitize_node_name(classname)
        });
    }
    let base = base.unwrap();
    let count = used.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}_{}", *count - 1)
    }
}

#[allow(clippy::too_many_arguments)]
fn build_entity_node(
    entity: &Entity,
    class: Option<&EntityClass>,
    recipe: &Recipe,
    settings: &Settings,
    node_name: &str,
    is_worldspawn: bool,
    library: &MaterialLibrary,
    material_refs: &IndexMap<String, GdValue>,
    options: &BuildOptions,
    sub_resources: &mut Vec<SubResource>,
    meshes: &mut Vec<MeshFile>,
    stats: &mut Stats,
    warnings: &mut Vec<String>,
) -> Node {
    // Water is a sensor whatever the class says, and swims in its closed brushes.
    let water = crate::water::enabled(entity) && recipe.instance.is_none() && recipe.is_solid;
    let water_recipe;
    let recipe = if water {
        water_recipe = Recipe {
            node_class: "Area3D".into(),
            collision: if recipe.collision == CollisionKind::None { CollisionKind::None } else { CollisionKind::Convex },
            ..recipe.clone()
        };
        &water_recipe
    } else {
        recipe
    };
    let mut node = Node::new(node_name.to_string(), recipe.node_class.clone());
    node.instance = recipe.instance.clone();
    node.script = recipe.script.clone();
    node.groups = recipe.groups.clone();

    let mut merged = merge_properties(entity, class);

    if recipe.is_solid && !(entity.brushes.is_empty() && entity.patches.is_empty() && entity.external_surfaces.is_empty()) {
        // Occluders off unless the map asks. They are drawn on the processor
        // every frame, and on an open level they cost without saving, so it is
        // the mapper's call and not a default.
        let wants_occluders = entity
            .get("occluders")
            .map(|v| v != "0" && !v.is_empty())
            .unwrap_or(false);
        let smallest_wall = entity.get_f64("occluder_min_size").unwrap_or(64.0).max(1.0);
        let mut geo = geometry::build_solid(
            entity,
            settings,
            SolidOptions {
                origin_kind: recipe.origin,
                collision: recipe.collision,
                build_visuals: recipe.build_visuals,
                force_zero_origin: is_worldspawn,
                // Given as the side of the smallest square that counts, which
                // is easier to picture on a grid than an area.
                occluder_min_area: if wants_occluders {
                    smallest_wall * smallest_wall
                } else {
                    0.0
                },
            },
            &library.sizes,
        );
        let recipe = &Recipe {
            occluder: recipe.occluder || wants_occluders,
            ..recipe.clone()
        };

        for brush in &geo.unbounded_brushes {
            warnings.push(format!(
                "{}: brush {brush} does not close into a solid, so its open faces were \
                 dropped; one of its planes probably faces the wrong way",
                entity.classname()
            ));
        }

        let shared = if recipe.origin == OriginKind::Brush && !recipe.occluder {
            crate::instancing::apply(entity, &mut geo, settings.scale_factor)
        } else { None };
        let transform = shared.unwrap_or_else(|| level::Transform {
            translation: (math::to_godot(geo.origin)*settings.scale_factor).to_array(), ..Default::default()
        });
        set_transform(&mut node, Vec3::from_array(transform.translation), Vec3::from_array(transform.rotation_yxz), Vec3::from_array(transform.scale));

        let water = water.then(|| crate::water::build(entity, &level::Entity { transform: transform.clone(), ..Default::default() }, settings.scale_factor)).flatten();
        if crate::water::enabled(entity) && recipe.instance.is_none() && water.is_none() {
            warnings.push(format!("Water {node_name}: no valid closed brush volume; water runtime omitted"));
        }
        // The baseline draws the volume's upward faces; the sensor keeps every brush.
        let baseline = water.as_ref().filter(|water| water.surface_material);
        if baseline.is_some() {
            for surface in &mut geo.mesh.surfaces {
                surface.indices = surface.indices.chunks_exact(3).filter(|triangle| triangle.iter().all(|i| surface.vertices[*i as usize].normal.y > 0.5)).flatten().copied().collect();
            }
            geo.mesh.surfaces.retain(|surface| !surface.indices.is_empty());
        }

        if !geo.mesh.is_empty() {
            // Godot's meshes stop at 256 surfaces and drop the rest, which a solid with
            // a gradient painted on every face can pass. Such a solid becomes several
            // meshes, the first still called `mesh`.
            let parts = geo.mesh.surfaces.chunks(GODOT_MAX_SURFACES).collect::<Vec<_>>();
            for (part, surfaces) in parts.iter().enumerate() {
                let suffix = if part == 0 { String::new() } else { format!("_{}", part + 1) };
                let file_name = format!("{node_name}{suffix}.obj");
                let contents = crate::writer::obj::write(
                    &geometry::Mesh { surfaces: surfaces.to_vec() },
                    &options.scene_name,
                    ObjOptions {
                        flip_faces: settings.flip_faces,
                        flip_uv_v: !settings.flip_uv_v,
                    },
                );
                meshes.push(MeshFile {
                    file_name: file_name.clone(),
                    contents,
                });

                let mut mesh_node = Node::new(format!("mesh{suffix}"), "MeshInstance3D");
                mesh_node.set(
                    "mesh",
                    GdValue::ExtResource {
                        ty: "ArrayMesh".into(),
                        path: format!("{}/{file_name}", options.mesh_dir),
                    },
                );
                for (i, surface) in surfaces.iter().enumerate() {
                    if let Some(water) = baseline {
                        let albedo = match library.materials.get(&surface.texture) {
                            Some(Material::FromTextures { albedo, .. }) => Some(albedo.as_str()),
                            _ => None,
                        };
                        let id = format!("HeronWater_{node_name}{suffix}_{i}");
                        let material = crate::writer::godot_water::material(id, water, albedo, sub_resources);
                        mesh_node.set(format!("surface_material_override/{i}"), material);
                    } else if let Some(material) = material_refs.get(&surface.texture) {
                        mesh_node.set(format!("surface_material_override/{i}"), material.clone());
                    }
                }
                node.add_child(mesh_node);
            }
            stats.surfaces += geo.mesh.surfaces.len();
            stats.merged_faces += geo.merged_faces;
            stats.pressed_faces += geo.pressed_faces;
            stats.welded_corners += geo.welded_corners;
            for problem in &geo.problems {
                warnings.push(format!("{node_name}: {problem}"));
            }
            stats.merged_hulls += geo.merged_hulls;
            stats.triangles += geo.mesh.triangle_count();

            if recipe.occluder {
                add_occluder(&mut node, &geo, node_name, sub_resources, stats);
            }
        }

        // Match the editor's implicit keyframe pivot, even when an origin brush
        // or explicit entity origin makes the exported mesh origin different.
        if merged.contains_key("animation_keys") && entity.get("animation_pivot").is_none() {
            if let Some(bounds) = geo.bounds {
                let p = bounds.center();
                merged.insert("animation_pivot".into(), format!("{} {} {}", p.x, p.y, p.z));
            }
        }
        add_collision(&mut node, &geo, node_name, sub_resources, stats);
        if let Some(water) = &water {
            crate::writer::godot_water::attach(&mut node, water, sub_resources);
        }
    } else {
        let position = entity
            .get_vec3("origin")
            .map(|o| math::to_godot(o) * settings.scale_factor)
            .unwrap_or(Vec3::ZERO);
        set_transform(
            &mut node,
            position,
            entity_rotation(entity),
            entity_scale(entity),
        );
    }

    apply_properties(&mut node, &merged, class, recipe);
    put_areas_on_their_own_layer(&mut node, recipe, settings);
    node
}

/// An Area3D on layer 1 sits on the level's own layer, so every ray the game
/// traces hits it and a shot stops at the edge of a checkpoint. Only the layer
/// moves; the mask, which decides what the trigger notices, is left alone.
///
/// Instanced scenes keep their own layers. An enemy is an Area3D too, and it is
/// supposed to stop bullets.
fn put_areas_on_their_own_layer(node: &mut Node, recipe: &Recipe, settings: &Settings) {
    if recipe.node_class != "Area3D" || recipe.instance.is_some() || settings.trigger_layer == 0 {
        return;
    }
    if node.properties.iter().any(|(k, _)| k == "collision_layer") {
        return;
    }
    let bit = 1i64 << (settings.trigger_layer.min(32) - 1);
    node.set("collision_layer".to_string(), GdValue::Int(bit));
}

fn add_collision(
    node: &mut Node,
    geo: &SolidGeometry,
    node_name: &str,
    sub_resources: &mut Vec<SubResource>,
    stats: &mut Stats,
) {
    if !classdb::inherits(&node.class, "CollisionObject3D") && classdb::is_known(&node.class) {
        // A plain Node3D has nowhere to hang a shape.
        return;
    }
    for (i, hull) in geo.convex_hulls.iter().enumerate() {
        let id = format!("Convex_{node_name}_{i}");
        sub_resources.push(SubResource {
            id: id.clone(),
            ty: "ConvexPolygonShape3D".into(),
            properties: vec![("points".into(), GdValue::PackedVec3(hull.clone()))],
        });
        let mut shape = Node::new(format!("shape_{i}"), "CollisionShape3D");
        shape.set("shape", GdValue::SubResource(id));
        node.add_child(shape);
        stats.collision_shapes += 1;
    }
    if !geo.concave_triangles.is_empty() {
        let id = format!("Concave_{node_name}");
        sub_resources.push(SubResource {
            id: id.clone(),
            ty: "ConcavePolygonShape3D".into(),
            properties: vec![(
                "data".into(),
                GdValue::PackedVec3(geo.concave_triangles.clone()),
            )],
        });
        let mut shape = Node::new("shape", "CollisionShape3D");
        shape.set("shape", GdValue::SubResource(id));
        node.add_child(shape);
        stats.collision_shapes += 1;
    }
}

/// Builds an occluder from the entity's visible geometry.
fn add_occluder(
    node: &mut Node,
    geo: &crate::geometry::SolidGeometry,
    node_name: &str,
    sub_resources: &mut Vec<SubResource>,
    stats: &mut Stats,
) {
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut indices: Vec<i32> = Vec::new();
    if geo.occluder_faces.is_empty() {
        // Everything the solid draws, which is what a small piece of scenery
        // built to block sight wants.
        for surface in &geo.mesh.surfaces {
            let base = vertices.len() as i32;
            vertices.extend(surface.vertices.iter().map(|v| v.position));
            indices.extend(surface.indices.iter().map(|i| *i as i32 + base));
        }
    } else {
        // Only the walls big enough to be worth drawing on the processor.
        for face in &geo.occluder_faces {
            let base = vertices.len() as i32;
            vertices.extend(face.iter().copied());
            for i in 1..face.len() - 1 {
                indices.push(base);
                indices.push(base + i as i32);
                indices.push(base + i as i32 + 1);
            }
        }
    }
    if indices.is_empty() {
        return;
    }

    let id = format!("Occluder_{node_name}");
    sub_resources.push(SubResource {
        id: id.clone(),
        ty: "ArrayOccluder3D".into(),
        properties: vec![
            ("vertices".into(), GdValue::PackedVec3(vertices)),
            ("indices".into(), GdValue::PackedInt32(indices)),
        ],
    });

    let mut occluder = Node::new("occluder", "OccluderInstance3D");
    occluder.set("occluder", GdValue::SubResource(id));
    node.add_child(occluder);
    stats.occluders += 1;
}

fn set_transform(node: &mut Node, position: Vec3, rotation: Vec3, scale: Vec3) {
    let basis = scene::scale_basis(scene::basis_from_euler_yxz(rotation), scale);
    if position == Vec3::ZERO && rotation == Vec3::ZERO && scale == Vec3::ONE {
        return;
    }
    node.set("transform", GdValue::Transform3D(basis, position));
}

/// Quake angles to Godot euler angles, in radians.
fn entity_rotation(entity: &Entity) -> Vec3 {
    let classname = entity.classname();
    let mut degrees = Vec3::ZERO;

    let key = if entity.properties.contains_key("angles") {
        Some("angles")
    } else if entity.properties.contains_key("mangle") {
        Some("mangle")
    } else {
        None
    };

    if let Some(key) = key {
        if let Some(raw) = entity.get_vec3(key) {
            degrees = Vec3::new(-raw.x, raw.y, -raw.z);
            if key == "mangle" {
                if classname.starts_with("light") {
                    degrees = Vec3::new(raw.y, raw.x, -raw.z);
                } else if classname == "info_intermission" {
                    degrees = Vec3::new(raw.x, raw.y, -raw.z);
                }
            }
        }
    } else if let Some(angle) = entity.get_f64("angle") {
        if (angle + 1.0).abs() < 1e-6 {
            degrees.x = 90.0;
        } else if (angle + 2.0).abs() < 1e-6 {
            degrees.x = -90.0;
        } else {
            degrees.y = angle;
        }
    }

    degrees.y += 180.0;
    Vec3::new(
        degrees.x.to_radians(),
        degrees.y.to_radians(),
        degrees.z.to_radians(),
    )
}

fn entity_scale(entity: &Entity) -> Vec3 {
    let Some(raw) = entity.get("scale") else {
        return Vec3::ONE;
    };
    let parts: Vec<f64> = raw
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    match parts.len() {
        1 => Vec3::splat(parts[0]),
        3 => math::to_godot(Vec3::new(parts[0], parts[1], parts[2])),
        _ => Vec3::ONE,
    }
}

/// Class defaults from the FGD, overridden by whatever the map wrote.
fn merge_properties(entity: &Entity, class: Option<&EntityClass>) -> IndexMap<String, String> {
    let mut merged: IndexMap<String, String> = IndexMap::new();
    if let Some(class) = class {
        for (name, prop) in &class.properties {
            let default = match (&prop.default, prop.ty.as_str()) {
                (Some(d), _) => Some(d.clone()),
                (None, "flags") => Some(prop.default_flag_bits().to_string()),
                _ => None,
            };
            if let Some(d) = default {
                merged.insert(name.clone(), d);
            }
        }
    }
    for (key, value) in &entity.properties {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

fn apply_properties(
    node: &mut Node,
    merged: &IndexMap<String, String>,
    class: Option<&EntityClass>,
    recipe: &Recipe,
) {
    let mut meta: Vec<(String, GdValue)> = Vec::new();

    for (key, raw) in merged {
        if key.starts_with("_tb_") {
            continue;
        }
        let definition = class.and_then(|c| c.properties.get(key));
        let value = value_from_fgd(definition, raw);
        meta.push((key.clone(), value.clone()));

        // Transform keys are already folded into the node transform.
        if matches!(
            key.as_str(),
            "origin" | "angle" | "angles" | "mangle" | "scale" | "classname"
        ) {
            continue;
        }
        if (recipe.instance.is_some() && key == "scene")
            || (recipe.script.is_some() && key == "script")
        {
            continue;
        }

        match classdb::property_type(&recipe.node_class, key) {
            Some(ty) => {
                if let Some(typed) = coerce(ty, definition, raw) {
                    node.set(key.clone(), typed);
                }
            }
            // Under `auto`, anything the table does not know still goes on the
            // node, except the keys that exist purely for the map editor and
            // the build. Those would only produce "property not found" noise.
            None if recipe.auto && !is_bookkeeping(key) => {
                node.set(key.clone(), value);
            }
            None => {}
        }
    }

    if !meta.is_empty() {
        node.meta.push(("properties".into(), GdValue::Dict(meta)));
    }
}

/// The value as the FGD describes it, used for the metadata dictionary. The
/// reading is `level::Value`'s, so the `.tscn` and the neutral level file
/// never disagree about what a property meant.
fn value_from_fgd(definition: Option<&Property>, raw: &str) -> GdValue {
    from_level_value(level::Value::from_fgd(definition, raw))
}

fn from_level_value(value: level::Value) -> GdValue {
    match value {
        level::Value::Bool(v) => GdValue::Bool(v),
        level::Value::Int(v) => GdValue::Int(v),
        level::Value::Float(v) => GdValue::Float(v),
        level::Value::String(v) => GdValue::Str(v),
        level::Value::Vec2([x, y]) => GdValue::Vec2(crate::math::Vec2::new(x, y)),
        level::Value::Vec3([x, y, z]) => GdValue::Vec3(Vec3::new(x, y, z)),
        level::Value::Color([r, g, b, a]) => GdValue::Color(r, g, b, a),
        level::Value::Array(items) => {
            GdValue::Array(items.into_iter().map(from_level_value).collect())
        }
    }
}

/// The value shaped for a specific Godot property.
fn coerce(ty: PropType, definition: Option<&Property>, raw: &str) -> Option<GdValue> {
    let trimmed = raw.trim();
    match ty {
        PropType::Bool => Some(GdValue::Bool(!matches!(
            trimmed.to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        ))),
        PropType::Int => trimmed.parse::<f64>().ok().map(|v| GdValue::Int(v as i64)),
        PropType::Float => trimmed.parse::<f64>().ok().map(GdValue::Float),
        PropType::String => Some(GdValue::Str(trimmed.to_string())),
        PropType::NodePath => Some(GdValue::NodePath(trimmed.to_string())),
        PropType::Color => {
            let scale = match definition.map(|d| d.ty.as_str()) {
                Some("color1") => 1.0,
                _ => 255.0,
            };
            Some(parse_color(trimmed, scale))
        }
        PropType::Vector2 => {
            let n: Vec<f64> = trimmed
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            (n.len() >= 2).then(|| GdValue::Vec2(crate::math::Vec2::new(n[0], n[1])))
        }
        PropType::Vector3 => {
            let n: Vec<f64> = trimmed
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            (n.len() >= 3).then(|| GdValue::Vec3(math::to_godot(Vec3::new(n[0], n[1], n[2]))))
        }
        // A path becomes a real reference; anything else would fail to load.
        PropType::Resource(class) => {
            let looks_like_a_path = trimmed.starts_with("res://") || trimmed.starts_with("uid://");
            (looks_like_a_path && trimmed.len() > 6).then(|| GdValue::ExtResource {
                ty: class.to_string(),
                path: trimmed.to_string(),
            })
        }
    }
}

/// Keys that describe the entity to the editor or the build, not to Godot.
fn is_bookkeeping(key: &str) -> bool {
    key.starts_with('_')
        || matches!(
            key,
            "targetname" | "target" | "spawnflags" | "scene" | "wad"
        )
}

fn parse_color(raw: &str, scale: f64) -> GdValue {
    from_level_value(level::parse_color(raw, scale))
}

/// One material resource per texture, shared by every mesh that uses it.
fn build_material_resources(
    library: &MaterialLibrary,
    sub_resources: &mut Vec<SubResource>,
) -> IndexMap<String, GdValue> {
    let mut refs = IndexMap::new();
    for (texture, material) in &library.materials {
        match material {
            Material::External { res_path } => {
                refs.insert(
                    texture.clone(),
                    GdValue::ExtResource {
                        ty: "Material".into(),
                        path: res_path.clone(),
                    },
                );
            }
            Material::FromTextures { albedo, maps } => {
                let id = format!("Material_{}", crate::map::sanitize_node_name(texture));
                let mut properties = vec![
                    ("resource_name".to_string(), GdValue::Str(texture.clone())),
                    (
                        "albedo_texture".to_string(),
                        GdValue::ExtResource {
                            ty: "Texture2D".into(),
                            path: albedo.clone(),
                        },
                    ),
                ];
                if let Some(image) = library.generated.get(texture) {
                    if let Some([roughness, metallic, emission]) = image.surface {
                        properties.push(("roughness".into(), GdValue::Float(roughness as f64)));
                        properties.push(("metallic".into(), GdValue::Float(metallic as f64)));
                        if emission > 0.0 {
                            properties.push(("emission_enabled".into(), GdValue::Bool(true)));
                            properties.push(("emission".into(), GdValue::Color(1.0,1.0,1.0,1.0)));
                            properties.push(("emission_energy_multiplier".into(), GdValue::Float(emission as f64)));
                            properties.push(("emission_texture".into(), GdValue::ExtResource { ty: "Texture2D".into(), path: albedo.clone() }));
                        }
                    }
                    properties.push(("texture_repeat".into(), GdValue::Bool(image.repeat)));
                    if let Some(pbr)=&image.pbr {
                        properties.push(("roughness".into(),GdValue::Float(pbr.roughness as f64)));
                        properties.push(("metallic".into(),GdValue::Float(pbr.metallic as f64)));
                        properties.push(("normal_scale".into(),GdValue::Float(pbr.normal_scale as f64)));
                        properties.push(("ao_light_affect".into(),GdValue::Float(0.0)));
                        // Baking only occlusion strength into a Godot companion is handled by the editor.
                        let e=pbr.emissive_factor.map(|c| {
                            let c=c as f64;
                            if c<=0.0031308 {c*12.92} else {1.055*c.powf(1.0/2.4)-0.055}
                        });
                        properties.push(("emission_enabled".into(),GdValue::Bool(e.iter().any(|v|*v>0.))));
                        properties.push(("emission".into(),GdValue::Color(e[0] as f64,e[1] as f64,e[2] as f64,1.)));
                        properties.push(("emission_operator".into(),GdValue::Int(1)));
                        for image in &pbr.images {
                            let texture=GdValue::ExtResource {ty:"Texture2D".into(),path:image.path.clone()};
                            match image.slot {
                                "normal"=>{properties.push(("normal_enabled".into(),GdValue::Bool(true)));properties.push(("normal_texture".into(),texture));},
                                "mr"=>{properties.push(("roughness_texture".into(),texture.clone()));properties.push(("metallic_texture".into(),texture));properties.push(("roughness_texture_channel".into(),GdValue::Int(1)));properties.push(("metallic_texture_channel".into(),GdValue::Int(2)));},
                                "ao"=>{properties.push(("ao_enabled".into(),GdValue::Bool(true)));properties.push(("ao_texture".into(),texture));},
                                "emissive"=>properties.push(("emission_texture".into(),texture)),
                                _=>{},
                            }
                        }
                    }
                    if let Some(pbr) = &image.pbr {
                        let transparency = match pbr.alpha_mode {
                            crate::materials::AlphaMode::Opaque => 0,
                            crate::materials::AlphaMode::Mask => 2,
                            crate::materials::AlphaMode::Blend => 4,
                        };
                        properties.push(("transparency".into(), GdValue::Int(transparency)));
                        if pbr.alpha_mode == crate::materials::AlphaMode::Mask {
                            properties.push(("alpha_scissor_threshold".into(), GdValue::Float(pbr.alpha_cutoff as f64)));
                        }
                        properties.push(("cull_mode".into(), GdValue::Int(if pbr.double_sided { 2 } else { 0 })));
                    } else if image.alpha {
                        properties.push(("transparency".into(), GdValue::Int(1)));
                    }
                }
                for (property, path) in maps {
                    // Enabling the feature is what makes the map take effect.
                    match *property {
                        "normal_texture" => {
                            properties.push(("normal_enabled".into(), GdValue::Bool(true)))
                        }
                        "emission_texture" => {
                            properties.push(("emission_enabled".into(), GdValue::Bool(true)))
                        }
                        "ao_texture" => properties.push(("ao_enabled".into(), GdValue::Bool(true))),
                        "orm_texture" => {
                            properties.push(("orm_texture_channel".into(), GdValue::Int(0)))
                        }
                        _ => {}
                    }
                    properties.push((
                        property.to_string(),
                        GdValue::ExtResource {
                            ty: "Texture2D".into(),
                            path: path.clone(),
                        },
                    ));
                }
                sub_resources.push(SubResource {
                    id: id.clone(),
                    ty: "StandardMaterial3D".into(),
                    properties,
                });
                refs.insert(texture.clone(), GdValue::SubResource(id));
            }
            Material::Missing => {}
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_cells_preserve_geometry_collision_and_entity_links() {
        fn cube(x:f64)->crate::map::Brush {
            use crate::map::{Face,UvFormat};
            use crate::math::{Plane,Vec2};
            let faces=[(-Vec3::X,-x),(Vec3::X,x+64.),(-Vec3::Y,0.),(Vec3::Y,64.),(-Vec3::Z,0.),(Vec3::Z,64.)]
                .into_iter().map(|(normal,d)|Face{plane:Plane{normal,d},points:[Vec3::ZERO;3],texture:"stone".into(),format:UvFormat::Standard,uv_axes:[Vec3::X,Vec3::Y],offset:Vec2::ZERO,rotation:0.,scale:Vec2::ONE,surface:None}).collect();
            crate::map::Brush{faces}
        }
        let mut world=Entity::default();world.properties.insert("classname".into(),"worldspawn".into());
        world.brushes=vec![cube(-2048.),cube(0.),cube(128.),cube(2048.)];
        let mut mover=Entity::default();mover.properties.insert("classname".into(),"func_animate".into());mover.properties.insert("targetname".into(),"platform".into());mover.brushes=vec![cube(4096.)];
        let map=MapFile{entities:vec![world,mover],..Default::default()};
        let classes=IndexMap::new();let mut settings=Settings::default();settings.default_collision=CollisionKind::Concave;
        let options=LevelOptions{inline_collision:true,..Default::default()};
        let original=build_level(&map,&classes,&settings,&options);
        settings.static_cell_size=1024.;
        let chunked=build_level(&map,&classes,&settings,&options);
        assert_eq!(chunked.stats.triangles,original.stats.triangles);
        assert_eq!(chunked.stats.brushes,original.stats.brushes);
        assert_eq!(chunked.level.entities.iter().filter(|e|e.classname=="func_static").count(),3);
        assert_eq!(chunked.level.entities.iter().filter(|e|e.classname=="func_animate").count(),1);
        let mover=|result:&LevelResult|result.level.entities.iter().find(|e|e.classname=="func_animate").unwrap().transform.clone();
        assert_eq!(mover(&original),mover(&chunked));
        // Compare collision triangles in world coordinates, independent of chunk origins.
        let triangles=|result:&LevelResult|{
            let mut points:Vec<_>=result.level.entities.iter().zip(&result.collision).flat_map(|(e,c)|c.triangles.iter().map(move |p|std::array::from_fn::<_,3,_>(|a|((p[a] as f64+e.transform.translation[a])*1000.).round() as i64))).collect();
            points.sort();points
        };
        assert!(!triangles(&original).is_empty());assert_eq!(triangles(&original),triangles(&chunked));
        // A shared face on a cell boundary must not reappear after partitioning.
        let mut map=map;map.entities.truncate(1);map.entities[0].brushes=vec![cube(960.),cube(1024.)];
        settings.cull_hidden=true;settings.static_cell_size=0.;
        let joined=build_level(&map,&classes,&settings,&options);
        settings.static_cell_size=1024.;let split=build_level(&map,&classes,&settings,&options);
        assert!(joined.stats.triangles<=split.stats.triangles);assert_eq!(split.stats.triangles,20);

    }

    #[test]
    fn a_solid_past_godots_surface_limit_becomes_several_meshes() {
        use crate::map::{Face,UvFormat};
        use crate::math::{Plane,Vec2};
        let cube=|i:usize|{
            let x=i as f64*128.;
            let faces=[(-Vec3::X,-x),(Vec3::X,x+64.),(-Vec3::Y,0.),(Vec3::Y,64.),(-Vec3::Z,0.),(Vec3::Z,64.)]
                .into_iter().map(|(normal,d)|Face{plane:Plane{normal,d},points:[Vec3::ZERO;3],texture:format!("paint_{i}"),format:UvFormat::Standard,uv_axes:[Vec3::X,Vec3::Y],offset:Vec2::ZERO,rotation:0.,scale:Vec2::ONE,surface:None}).collect();
            crate::map::Brush{faces}
        };
        let mut world=Entity::default();world.properties.insert("classname".into(),"worldspawn".into());
        world.brushes=(0..300).map(cube).collect();
        let map=MapFile{entities:vec![world],..Default::default()};
        let built=build(&map,&IndexMap::new(),&Settings::default(),&BuildOptions{scene_name:"wide".into(),mesh_dir:"res://wide".into()});
        let names:Vec<_>=built.meshes.iter().map(|m|m.file_name.as_str()).collect();
        assert_eq!(names,["worldspawn.obj","worldspawn_2.obj"]);
        let surfaces:Vec<_>=built.meshes.iter().map(|m|m.contents.matches("\nusemtl ").count()).collect();
        assert_eq!(surfaces,[256,44]);
        let world=&built.scene.root.children[0];
        let parts:Vec<_>=world.children.iter().filter(|n|n.class=="MeshInstance3D").map(|n|n.name.as_str()).collect();
        assert_eq!(parts,["mesh","mesh_2"]);
    }

    #[test]
    fn a_water_volume_is_a_sensor_with_its_top_drawn_as_water() {
        use crate::map::{Face,UvFormat};
        use crate::math::{Plane,Vec2};
        let faces=[(-Vec3::X,64.),(Vec3::X,64.),(-Vec3::Y,64.),(Vec3::Y,64.),(-Vec3::Z,64.),(Vec3::Z,0.)]
            .into_iter().map(|(normal,d)|Face{plane:Plane{normal,d},points:[Vec3::ZERO;3],texture:"water".into(),format:UvFormat::Standard,uv_axes:[Vec3::X,Vec3::Y],offset:Vec2::ZERO,rotation:0.,scale:Vec2::ONE,surface:None}).collect();
        let mut pool=Entity::default();
        pool.properties.insert("classname".into(),"water_volume".into());
        pool.properties.insert("water_priority".into(),"3".into());
        pool.brushes=vec![crate::map::Brush{faces}];
        let mut world=Entity::default();world.properties.insert("classname".into(),"worldspawn".into());
        let map=MapFile{entities:vec![world,pool],..Default::default()};
        let built=build(&map,&IndexMap::new(),&Settings::default(),&BuildOptions{scene_name:"pool".into(),mesh_dir:"res://pool".into()});
        let pool=built.scene.root.children.iter().find(|n|n.name=="water_volume").unwrap();
        assert_eq!(pool.class,"Area3D");
        assert_eq!(pool.children.iter().filter(|n|n.class=="CollisionShape3D").count(),1);
        // Only the top: two triangles.
        let mesh=built.meshes.iter().find(|m|m.file_name=="water_volume.obj").unwrap();
        assert_eq!(mesh.contents.lines().filter(|l|l.starts_with("f ")).count(),2);
        let water=pool.children.iter().find(|n|n.name=="heron_water").unwrap();
        assert_eq!(water.groups,["heron_water"]);
        let Some((_,GdValue::Str(json)))=water.properties.iter().find(|p|p.0=="data_json") else {panic!()};
        let data:serde_json::Value=serde_json::from_str(json).unwrap();
        assert_eq!(data["priority"],3);
        assert_eq!(data["volumes"][0]["planes"].as_array().unwrap().len(),6);
        assert!(crate::writer::godot_water::used(&built.scene));
        let text=crate::writer::tscn::write(&built.scene);
        assert!(text.contains(r#"[sub_resource type="ShaderMaterial" id="HeronWater_water_volume_0"]"#),"{text}");
        assert!(text.contains("surface_material_override/0 = SubResource(\"HeronWater_water_volume_0\")"),"{text}");
        assert!(text.find("id=\"HeronWaterShader\"").unwrap()<text.find("id=\"HeronWater_water_volume_0\"").unwrap());
    }

    #[test]
    fn a_sibling_is_one_step_up() {
        assert_eq!(relative_path("trigger", "door"), "../door");
    }

    #[test]
    fn a_shared_layer_is_not_walked_past() {
        assert_eq!(relative_path("layer_7/trigger", "layer_7/door"), "../door");
    }

    #[test]
    fn reaching_into_another_layer_goes_up_then_down() {
        assert_eq!(relative_path("trigger", "layer_7/door"), "../layer_7/door");
        assert_eq!(
            relative_path("layer_1/a/trigger", "layer_7/door"),
            "../../../layer_7/door"
        );
    }

    #[test]
    fn a_node_pointing_at_itself_stays_put() {
        assert_eq!(relative_path("door", "door"), ".");
    }
}

// ---------------------------------------------------------------------------
// The engine-neutral path.
//
// Godot reads `.tscn` itself, so the writer above hands it finished nodes.
// Nothing else does, so everything else goes out as a `Level` and a `.glb` per
// solid, and the engine side decides what a `func_door` turns into.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LevelOptions {
    /// Name of the level, normally the map file's stem.
    pub name: String,
    /// Directory the mesh files go into, relative to the level file. Empty
    /// puts them alongside it.
    pub mesh_dir: String,
    /// Write collision geometry into each `.glb` and note it in the level file.
    pub collision: bool,
    /// Also keep collision geometry as plain triangles and hulls in
    /// `LevelResult`, for a writer that cannot point a loader at the `.glb`
    /// collision mesh.
    pub inline_collision: bool,
    /// Write the collision geometry into the `.glb` as a mesh of its own.
    /// Off for an engine that would draw every mesh in the file (Defold's
    /// model component); it takes the collision from `LevelResult` instead.
    pub mesh_collision: bool,
    /// Give every solid a per-brush convex hull collision instead of a concave
    /// triangle soup, for an engine whose physics only takes convex shapes
    /// (Defold). A brush is always convex, so a room built of brushes still
    /// collides right; only a curved patch is lost.
    pub convex_collision: bool,
    /// What the mesh files are written as. `.glb` for an engine that reads
    /// glTF; `.obj` for one that does not (Unity, out of the box), which then
    /// takes its collision from `LevelResult` since an `.obj` carries none.
    pub mesh_format: MeshFormat,
}

/// The file format `build_level` writes each solid's mesh in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeshFormat {
    /// Binary glTF, textures and (optionally) collision embedded.
    #[default]
    Glb,
    /// Wavefront OBJ: geometry and material names only, no collision inside.
    Obj,
}

impl Default for LevelOptions {
    fn default() -> Self {
        LevelOptions {
            name: "level".to_string(),
            mesh_dir: String::new(),
            collision: true,
            inline_collision: false,
            mesh_collision: true,
            convex_collision: false,
            mesh_format: MeshFormat::default(),
        }
    }
}

#[derive(Debug)]
pub struct LevelResult {
    pub level: Level,
    pub meshes: Vec<level::MeshFile>,
    /// Collision geometry per entity, aligned with `level.entities`. Empty
    /// entries for entities with no collision. Only filled when
    /// `LevelOptions.inline_collision` was set.
    pub collision: Vec<level::CollisionMesh>,
    pub warnings: Vec<String>,
    pub stats: Stats,
}

pub fn build_level(
    map: &MapFile,
    classes: &IndexMap<String, EntityClass>,
    settings: &Settings,
    options: &LevelOptions,
) -> LevelResult {
    let Prepared {
        entities,
        recipes,
        library,
        mut warnings,
        mut stats,
    } = prepare(map, classes, settings);

    // Resolve image bytes once per level, not once per brush/entity GLB.
    let (albedos, image_warnings) = if matches!(options.mesh_format, MeshFormat::Glb) {
        crate::writer::gltf::prepare_albedos(&library, settings)
    } else {
        (IndexMap::new(), Vec::new())
    };
    warnings.extend(image_warnings);

    let mut used_names: IndexMap<String, usize> = IndexMap::new();
    let mut names: Vec<String> = Vec::with_capacity(entities.len());
    let mut built: Vec<level::Entity> = Vec::with_capacity(entities.len());
    let mut meshes: Vec<level::MeshFile> = Vec::new();
    let mut collision_meshes: Vec<level::CollisionMesh> = Vec::with_capacity(entities.len());

    for (index, entity) in entities.iter().enumerate() {
        let recipe = &recipes[index];
        let class = classes.get(entity.classname());
        let is_worldspawn = index == 0 && entity.classname() == "worldspawn";
        let name = entity_node_name(entity, recipe, index, &mut used_names);
        let mut collision_mesh = level::CollisionMesh::default();

        let mut animation_pivot=None;
        let mut out = level::Entity {
            name: name.clone(),
            classname: entity.classname().to_string(),
            group: if settings.group_hierarchy {
                map.group_of(entity)
                    .map(|g| group_names(map, g.id))
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            solid: recipe.is_solid,
            scene: recipe.instance.clone(),
            tags: recipe.groups.clone(),
            ..Default::default()
        };

        if recipe.is_solid && !(entity.brushes.is_empty() && entity.patches.is_empty() && entity.external_surfaces.is_empty()) {
            let mut geo = geometry::build_solid(
                entity,
                settings,
                SolidOptions {
                    origin_kind: recipe.origin,
                    collision: match (options.collision, recipe.collision) {
                        (false, _) => CollisionKind::None,
                        (true, kind) if crate::water::enabled(entity) && kind != CollisionKind::None => CollisionKind::Convex,
                        (true, CollisionKind::Concave) if options.convex_collision => {
                            CollisionKind::Convex
                        }
                        (true, kind) => kind,
                    },
                    build_visuals: recipe.build_visuals,
                    force_zero_origin: is_worldspawn,
                    occluder_min_area: 0.0,
                },
                &library.sizes,
            );

            for brush in &geo.unbounded_brushes {
                warnings.push(format!(
                    "{}: brush {brush} does not close into a solid, so its open faces were \
                     dropped; one of its planes probably faces the wrong way",
                    entity.classname()
                ));
            }
            for problem in &geo.problems {
                warnings.push(format!("{name}: {problem}"));
            }

            // The baseline water draws a surface; its closed hull remains a sensor.
            if crate::water::enabled(entity) && crate::water::baseline(entity) {
                for surface in &mut geo.mesh.surfaces {
                    surface.indices=surface.indices.chunks_exact(3).filter(|triangle|triangle.iter().all(|i|surface.vertices[*i as usize].normal.y>0.5)).flatten().copied().collect();
                }
                geo.mesh.surfaces.retain(|surface|!surface.indices.is_empty());
            }
            let shared = if recipe.origin == OriginKind::Brush && !recipe.occluder {
                crate::instancing::apply(entity, &mut geo, settings.scale_factor)
            } else { None };
            animation_pivot=geo.bounds.map(|b|b.center());
            let origin = math::to_godot(geo.origin) * settings.scale_factor;
            out.transform = shared.unwrap_or_else(|| level::Transform { translation: [origin.x, origin.y, origin.z], ..Default::default() });

            if options.inline_collision {
                collision_mesh.triangles = geo
                    .concave_triangles
                    .iter()
                    .map(|v| [v.x as f32, v.y as f32, v.z as f32])
                    .collect();
                for hull in &geo.convex_hulls {
                    if hull.len() < 4 {
                        continue;
                    }
                    let mut lo = [f32::MAX; 3];
                    let mut hi = [f32::MIN; 3];
                    for p in hull {
                        for (axis, v) in
                            [p.x as f32, p.y as f32, p.z as f32].into_iter().enumerate()
                        {
                            lo[axis] = lo[axis].min(v);
                            hi[axis] = hi[axis].max(v);
                        }
                    }
                    collision_mesh.boxes.push([lo, hi]);
                    collision_mesh.hulls.push(
                        hull.iter()
                            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
                            .collect(),
                    );
                }
            }

            let collision = crate::writer::gltf::Collision {
                hulls: geo.convex_hulls.clone(),
                triangles: geo.concave_triangles.clone(),
            };
            if options.mesh_format == MeshFormat::Obj {
                // An `.obj` per solid, for an engine that reads no glTF. Collision
                // cannot ride inside it, so it stays in `LevelResult` (the caller sets
                // `inline_collision`), and the block after this one records it.
                if !geo.mesh.is_empty() {
                    let text = crate::writer::obj::write(
                        &geo.mesh,
                        if entity.external_asset.is_some() { "imported_model" } else { &name },
                        crate::writer::obj::ObjOptions {
                            flip_faces: settings.flip_faces,
                            flip_uv_v: !settings.flip_uv_v,
                        },
                    );
                    let file_name = format!("{name}.obj");
                    out.surfaces = geo
                        .mesh
                        .surfaces
                        .iter()
                        .enumerate()
                        .map(|(primitive, surface)| level::Surface {
                            primitive,
                            texture: surface.texture.clone(),
                        })
                        .collect();
                    out.mesh = Some(if options.mesh_dir.is_empty() {
                        file_name.clone()
                    } else {
                        format!("{}/{file_name}", options.mesh_dir.trim_end_matches('/'))
                    });
                    meshes.push(level::MeshFile {
                        file_name,
                        bytes: text.into_bytes(),
                    });
                    stats.surfaces += out.surfaces.len();
                    stats.triangles += geo.mesh.triangle_count();
                }
            } else {
            let empty_collision = crate::writer::gltf::Collision::default();
            let written = crate::writer::gltf::write(
                &geo.mesh,
                if options.mesh_collision {
                    &collision
                } else {
                    &empty_collision
                },
                if entity.external_asset.is_some() { "imported_model" } else { &name },
                crate::writer::gltf::GltfOptions {
                    flip_faces: settings.flip_faces,
                    flip_uv_v: settings.flip_uv_v,
                },
            );
            if let Some(mut glb) = written {
                crate::writer::gltf::embed_materials(&mut glb, &albedos, settings.flip_uv_v);
                let file_name = format!("{name}.glb");
                out.surfaces = glb
                    .textures
                    .iter()
                    .enumerate()
                    .map(|(primitive, texture)| level::Surface {
                        primitive,
                        texture: texture.clone(),
                    })
                    .collect();
                if let Some((mesh, parts)) = glb.collision {
                    out.collision = Some(level::Collision {
                        kind: collision.kind(),
                        mesh,
                        parts,
                        // An Area3D reports overlaps and lets bodies through;
                        // anything else is a wall. `default_node_class` gives
                        // every `trigger_*` an Area3D, and the FGD names the
                        // rest with `node=Area3D`.
                        sensor: recipe.node_class == "Area3D",
                    });
                    stats.collision_shapes += parts;
                }
                out.mesh = Some(if options.mesh_dir.is_empty() {
                    file_name.clone()
                } else {
                    format!("{}/{file_name}", options.mesh_dir.trim_end_matches('/'))
                });
                meshes.push(level::MeshFile {
                    file_name,
                    bytes: glb.bytes,
                });
                stats.surfaces += out.surfaces.len();
                stats.triangles += geo.mesh.triangle_count();
            }
            }
            if !options.mesh_collision && out.collision.is_none() && !collision.is_empty() {
                // The collision stayed out of the `.glb`; the level record
                // should still say the entity has some, and the report should
                // still count it. The writer takes the geometry itself from
                // `LevelResult.collision`.
                let parts = collision.parts();
                out.collision = Some(level::Collision {
                    kind: collision.kind(),
                    mesh: 0,
                    parts,
                    sensor: recipe.node_class == "Area3D",
                });
                stats.collision_shapes += parts;
            }
            stats.merged_faces += geo.merged_faces;
            stats.pressed_faces += geo.pressed_faces;
            stats.welded_corners += geo.welded_corners;
            stats.merged_hulls += geo.merged_hulls;
        } else {
            let position = entity
                .get_vec3("origin")
                .map(|o| math::to_godot(o) * settings.scale_factor)
                .unwrap_or(Vec3::ZERO);
            let rotation = entity_rotation(entity);
            let scale = entity_scale(entity);
            out.transform = level::Transform {
                translation: [position.x, position.y, position.z],
                rotation_yxz: [rotation.x, rotation.y, rotation.z],
                scale: [scale.x, scale.y, scale.z],
            };
        }

        for (key, raw) in merge_properties(entity, class) {
            if key.starts_with("_tb_") {
                continue;
            }
            let definition = class.and_then(|c| c.properties.get(&key));
            out.properties
                .insert(key, level::Value::from_fgd(definition, &raw));
        }

        if out.properties.contains_key("animation_keys") && entity.get("animation_pivot").is_none() {
            if let Some(p)=animation_pivot {out.properties.insert("animation_pivot".into(),level::Value::Vec3([p.x,p.y,p.z]));}
        }
        if matches!(entity.classname(),"func_button"|"trigger_once"|"trigger_multiple") {
            let volumes:Vec<Vec<[f64;4]>>=entity.brushes.iter().map(|b|b.faces.iter().map(|f|[f.plane.normal.y,f.plane.normal.z,f.plane.normal.x,f.plane.d*settings.scale_factor]).collect()).collect();
            out.properties.insert("_heron_trigger_volumes".into(),level::Value::String(serde_json::to_string(&volumes).unwrap()));
        }
        crate::water::attach(entity,&mut out,settings.scale_factor,&mut warnings);
        if let Some(water)=crate::water::build(entity,&out,settings.scale_factor) {
            if let Some(mesh)=meshes.last_mut().filter(|mesh|out.mesh.as_ref().is_some_and(|name|name.rsplit('/').next()==Some(mesh.file_name.as_str()))) {
                crate::water::glb_material(&mut mesh.bytes,&water);
            }
        }
        stats.entities += 1;
        names.push(name);
        built.push(out);
        collision_meshes.push(collision_mesh);
    }

    link_names(&entities, &recipes, &names, &mut built);

    let (textures, texture_warnings) = level_textures(&library, settings);
    warnings.extend(texture_warnings);

    let aliases = crate::instancing::aliases(meshes.iter().map(|m| (m.file_name.as_str(),m.bytes.as_slice())));
    meshes.retain(|m| !aliases.contains_key(&m.file_name));
    for entity in &mut built {
        if let Some(path) = &mut entity.mesh {
            let (dir,file) = path.rsplit_once('/').map_or(("",path.as_str()), |v| v);
            if let Some(original) = aliases.get(file) {
                *path = if dir.is_empty() { original.clone() } else { format!("{dir}/{original}") };
            }
        }
    }

    LevelResult {
        level: Level {
            format: level::FORMAT_VERSION,
            generator: format!("heron_brush {}", env!("CARGO_PKG_VERSION")),
            name: options.name.clone(),
            scale: 1.0 / settings.scale_factor,
            textures,
            entities: built,
        },
        meshes,
        collision: collision_meshes,
        warnings,
        stats,
    }
}

/// Layer and group names for one group, outermost first.
fn group_names(map: &MapFile, id: i64) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = Some(id);
    let mut guard = 0;
    while let Some(gid) = current {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(group) = map.groups.iter().find(|g| g.id == gid) else {
            break;
        };
        parts.push(group.name.clone());
        current = group.parent_id;
    }
    parts.reverse();
    parts
}

/// Resolves `target` against every `targetname` in the map, so the engine side
/// gets entity names it can look up in the level file and nothing else.
fn link_names(
    entities: &[Entity],
    recipes: &[Recipe],
    names: &[String],
    built: &mut [level::Entity],
) {
    // Several entities may share one targetname, as they do in Quake.
    let mut by_name: IndexMap<&str, Vec<usize>> = IndexMap::new();
    for (i, entity) in entities.iter().enumerate() {
        let key = entity
            .get(&recipes[i].link_name_property)
            .unwrap_or_default()
            .trim();
        if !key.is_empty() {
            by_name.entry(key).or_default().push(i);
        }
    }

    for (i, entity) in entities.iter().enumerate() {
        let target = entity
            .get(&recipes[i].link_property)
            .unwrap_or_default()
            .trim();
        if target.is_empty() {
            continue;
        }
        let Some(targets) = by_name.get(target) else {
            continue;
        };
        built[i].targets = targets.iter().map(|t| names[*t].clone()).collect();
    }
}

/// What was found on disk for each texture, with Godot's `res://` taken off so
/// the paths are relative to the project and any engine can prefix its own
/// asset root.
fn level_textures(
    library: &MaterialLibrary,
    settings: &Settings,
) -> (Vec<level::Texture>, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();
    let mut external = 0usize;
    let mut textures: Vec<level::Texture> = Vec::new();

    for (name, material) in &library.materials {
        let (image, maps) = match material {
            Material::FromTextures { albedo, maps } => (
                Some(strip_res(albedo)),
                maps.iter()
                    .map(|(slot, path)| (slot.trim_start_matches('_').to_string(), strip_res(path)))
                    .collect(),
            ),
            // A `.tres` is a Godot resource and holds no image an engine
            // without Godot's importer could read.
            Material::External { .. } => {
                external += 1;
                (None, Vec::new())
            }
            Material::Missing => (None, Vec::new()),
        };
        let size = library.sizes.get(name);
        textures.push(level::Texture {
            name: name.clone(),
            image,
            maps,
            size: [size.x, size.y],
            surface: library.generated.get(name).and_then(|m| m.surface),
        });
    }

    if external > 0 {
        warnings.push(format!(
            "{external} texture(s) resolved to a Godot material in {}, which only Godot \
             can read; the level file names the texture and leaves the image out",
            settings
                .material_dir
                .clone()
                .unwrap_or_else(|| "the material directory".into())
        ));
    }
    (textures, warnings)
}

fn strip_res(path: &str) -> String {
    path.trim_start_matches("res://").to_string()
}
