//! Water volumes for Godot: the volume as an `Area3D` sensor, a `heron_water` child
//! that answers where the water is and how a body swims in it, and the baseline
//! surface as a wave shader. `heron_water_view.gd`, written beside the level, gives a
//! camera under water the water's fog.

use crate::scene::{GdValue, Node, Scene, SubResource};
use crate::water::Water;

const SCRIPT: &str = include_str!("godot_water.gd");
const SHADER: &str = include_str!("godot_water.gdshader");
/// Written as `<level>/heron_water_view.gd` when the level has water.
pub const VIEW_SCRIPT: &str = include_str!("godot_water_view.gd");
pub const VIEW_FILE: &str = "heron_water_view.gd";
/// The group every volume's `heron_water` node is in.
pub const GROUP: &str = "heron_water";
const SCRIPT_ID: &str = "HeronWaterScript";
const SHADER_ID: &str = "HeronWaterShader";

/// Adds the `heron_water` child that holds the volume.
pub fn attach(node: &mut Node, water: &Water, sub_resources: &mut Vec<SubResource>) {
    shared(
        sub_resources,
        SCRIPT_ID,
        "GDScript",
        "script/source",
        SCRIPT,
    );
    let mut child = Node::new("heron_water", "Node");
    child.groups.push(GROUP.into());
    // The script before its variables, or Godot drops them.
    child.set("script", GdValue::SubResource(SCRIPT_ID.into()));
    child.set(
        "data_json",
        GdValue::Str(serde_json::to_string(water).unwrap()),
    );
    node.add_child(child);
}

/// The baseline surface material for one of the volume's surfaces, keeping the
/// surface's texture when it has one.
pub fn material(
    id: String,
    water: &Water,
    albedo: Option<&str>,
    sub_resources: &mut Vec<SubResource>,
) -> GdValue {
    shared(sub_resources, SHADER_ID, "Shader", "code", SHADER);
    let [r, g, b] = water.color;
    let mut properties = vec![
        ("render_priority".into(), GdValue::Int(0)),
        ("shader".into(), GdValue::SubResource(SHADER_ID.into())),
        (
            "shader_parameter/tint".into(),
            GdValue::Color(r, g, b, water.opacity),
        ),
        (
            "shader_parameter/wave_height".into(),
            GdValue::Float(water.wave_height),
        ),
        (
            "shader_parameter/wave_speed".into(),
            GdValue::Float(water.wave_speed),
        ),
    ];
    if let Some(path) = albedo {
        properties.push((
            "shader_parameter/albedo_texture".into(),
            GdValue::ExtResource {
                ty: "Texture2D".into(),
                path: path.into(),
            },
        ));
    }
    sub_resources.push(SubResource {
        id: id.clone(),
        ty: "ShaderMaterial".into(),
        properties,
    });
    GdValue::SubResource(id)
}

/// Whether the scene has a volume, and so wants the view script beside it.
pub fn used(scene: &Scene) -> bool {
    fn visit(node: &Node) -> bool {
        node.groups.iter().any(|g| g == GROUP) || node.children.iter().any(visit)
    }
    visit(&scene.root)
}

fn shared(sub_resources: &mut Vec<SubResource>, id: &str, ty: &str, key: &str, text: &str) {
    if !sub_resources.iter().any(|r| r.id == id) {
        sub_resources.push(SubResource {
            id: id.into(),
            ty: ty.into(),
            properties: vec![(key.into(), GdValue::Str(text.into()))],
        });
    }
}
