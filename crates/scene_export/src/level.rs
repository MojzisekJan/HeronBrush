//! An engine-neutral description of a built map.
//!
//! The Godot side turns a map straight into `.tscn` nodes, because Godot reads
//! that format itself. No other engine does, so everything else goes through
//! here: entities with a transform, typed properties and a mesh file, and
//! nothing that names a node class or a `res://` path.
//!
//! Bevy shares Godot's axes and units, so the numbers written here need no
//! further conversion: Y is up, forward is -Z, and one unit is one metre.

use crate::fgd::Property;
use indexmap::IndexMap;
use serde::Serialize;

/// The version of this file's layout, so a reader can refuse a newer one.
///
/// 2 added the `collision` key on solid entities. A version 1 reader misses
/// collision and nothing else.
pub const FORMAT_VERSION: u32 = 2;

/// An entity property, typed by what the FGD said it was.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    /// Red, green, blue and alpha in the 0..1 range.
    Color([f64; 4]),
    Array(Vec<Value>),
}

impl Value {
    /// The value as the FGD describes it. Without a definition every value is
    /// the string the mapper typed, which is what the map stores.
    pub fn from_fgd(definition: Option<&Property>, raw: &str) -> Value {
        let Some(def) = definition else {
            return Value::String(raw.to_string());
        };
        match def.ty.as_str() {
            "boolean" | "bool" => match raw.trim() {
                "true" | "1" => Value::Bool(true),
                "false" | "0" => Value::Bool(false),
                _ => Value::String(raw.into()),
            },
            "json" => {
                fn convert(value: serde_json::Value) -> Option<Value> {
                    Some(match value {
                        serde_json::Value::Bool(v) => Value::Bool(v),
                        serde_json::Value::String(v) => Value::String(v),
                        serde_json::Value::Number(v) => {
                            if let Some(i) = v.as_i64() {
                                Value::Int(i)
                            } else {
                                Value::Float(v.as_f64()?)
                            }
                        }
                        serde_json::Value::Array(values) => Value::Array(
                            values
                                .into_iter()
                                .map(convert)
                                .collect::<Option<Vec<_>>>()?,
                        ),
                        _ => return None,
                    })
                }
                serde_json::from_str(raw)
                    .ok()
                    .and_then(convert)
                    .unwrap_or_else(|| Value::String(raw.into()))
            }
            "integer" => raw
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|v| Value::Int(v as i64))
                .unwrap_or_else(|| Value::String(raw.into())),
            "float" => raw
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Value::Float)
                .unwrap_or_else(|| Value::String(raw.into())),
            "flags" => raw
                .trim()
                .parse::<f64>()
                .map(|v| Value::Int(v as i64))
                .unwrap_or(Value::Int(0)),
            "choices" => {
                let numeric = def
                    .choices
                    .iter()
                    .all(|c| c.value.trim().parse::<i64>().is_ok());
                if numeric {
                    raw.trim()
                        .parse::<i64>()
                        .map(Value::Int)
                        .unwrap_or_else(|_| Value::String(raw.into()))
                } else {
                    Value::String(raw.to_string())
                }
            }
            "color255" => parse_color(raw, 255.0),
            "color1" => parse_color(raw, 1.0),
            _ => Value::String(raw.to_string()),
        }
    }
}

/// Colour components are written 0..255 in Quake FGDs and 0..1 in some others,
/// so the scale comes from the declared type.
pub fn parse_color(raw: &str, scale: f64) -> Value {
    let n: Vec<f64> = raw
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    if n.len() < 3 {
        return Value::Color([1.0, 1.0, 1.0, 1.0]);
    }
    if !scale.is_finite() || scale <= 0. || n.iter().any(|value| !value.is_finite()) {
        return Value::String(raw.into());
    }
    let a = if n.len() > 3 { n[3] / scale } else { 1.0 };
    Value::Color([n[0] / scale, n[1] / scale, n[2] / scale, a])
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Transform {
    pub translation: [f64; 3],
    /// Euler angles in radians, applied Y first, then X, then Z. That is
    /// Godot's `EULER_ORDER_YXZ` and Bevy's `EulerRot::YXZ`.
    pub rotation_yxz: [f64; 3],
    pub scale: [f64; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            translation: [0.0; 3],
            rotation_yxz: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

/// One drawn part of an entity's mesh, and the texture it wants.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Surface {
    /// Index of the primitive inside the entity's mesh file. Always in glTF
    /// mesh 0; collision lives in a mesh of its own.
    pub primitive: usize,
    pub texture: String,
}

/// What shape an entity's collision geometry takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionShape {
    /// One concave triangle soup, for static world geometry.
    Trimesh,
    /// A set of convex hulls, for a mover a physics engine has to push.
    Convex,
}

/// Where an entity's collision geometry sits in its `.glb`, and what it is.
///
/// The geometry is a glTF mesh with no normals, no texture coordinates and no
/// place in the file's scene, so drawing the scene never draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Collision {
    pub kind: CollisionShape,
    /// glTF mesh index holding it: 1 when the entity also draws something,
    /// 0 when it does not.
    pub mesh: usize,
    /// Primitives the shape is spread over, `0..parts` in `mesh`. A trimesh is
    /// one; a convex body is one primitive per hull.
    pub parts: usize,
    /// True when the shape is a sensor, not a wall: it reports what enters it
    /// and lets it pass. Every `trigger_*` volume, and anything the FGD gave a
    /// `node=Area3D`. A loader must not push the player out of one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sensor: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Entity {
    /// Unique across the level, so `targets` can name entities directly.
    pub name: String,
    pub classname: String,
    /// The map layers and groups this entity sits in, outermost first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub group: Vec<String>,
    pub transform: Transform,
    /// True when the entity was drawn as brushes rather than placed as a point.
    pub solid: bool,
    /// File name of the entity's `.glb`, next to this level file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<Surface>,
    /// The entity's collision geometry, when it has any. Absent for `skip`
    /// brushwork, point entities, and anything the FGD marked `collision=none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collision: Option<Collision>,
    pub properties: IndexMap<String, Value>,
    /// Names of the entities this one's `target` resolves to, already matched
    /// against the map, so nothing has to look anything up at run time.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// What `@heron_brush scene=` named, left as written for the engine side to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    /// What `@heron_brush groups=` named. Godot calls these groups, Bevy has no
    /// equivalent, so they come through as plain tags.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// One texture the level uses, and whatever was found on disk for it.
#[derive(Debug, Clone, Serialize)]
pub struct Texture {
    /// Roughness, metallic and albedo-coloured emission intensity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<[f32; 3]>,
    pub name: String,
    /// Image file, relative to the project root. None means nothing matched
    /// and the engine should fall back to its own default material.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Extra maps found next to the albedo image, as (slot, path) pairs.
    /// Slots are `normal`, `roughness`, `metallic`, `emission`, `ao`, `orm`;
    /// imported glTF materials also use `mr` and `emissive`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub maps: Vec<(String, String)>,
    /// Pixel size the UVs were worked out against.
    pub size: [f64; 2],
    /// Baked face paint, drawn clamped to its edges rather than repeated, as the
    /// GLBs' samplers say. A loader that sets its own wrapping reads it here.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub clamp: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Level {
    pub format: u32,
    pub generator: String,
    pub name: String,
    /// Map units per engine unit. 32 puts a 32 unit grid square on one metre.
    pub scale: f64,
    pub textures: Vec<Texture>,
    pub entities: Vec<Entity>,
}

impl Level {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// A mesh file to write next to the level. Binary, unlike the `.obj` the Godot
/// path writes, so this carries bytes rather than text.
#[derive(Debug, Clone)]
pub struct MeshFile {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// One entity's collision geometry as plain triangles, for a loader that
/// cannot read the glTF collision mesh (the raylib path). Metres, engine axes,
/// entity-local: add the entity's `transform.translation` for world space.
///
/// Populated only when `LevelOptions.inline_collision` is set. The glTF path
/// leaves it empty and writes collision into the `.glb` instead.
#[derive(Debug, Clone, Default)]
pub struct CollisionMesh {
    /// Three points per triangle, from a trimesh body (static world geometry).
    pub triangles: Vec<[f32; 3]>,
    /// The axis-aligned box around each convex hull's corners. heron_brush does not
    /// triangulate its hulls, and the movers that carry them are box-shaped,
    /// so a box is a fair stand-in for a viewer that only walks around.
    pub boxes: Vec<[[f32; 3]; 2]>,
    /// The corner cloud of each convex hull, for an engine that takes a hull
    /// shape directly (Defold's `TYPE_HULL`). Same axes and origin as
    /// `triangles`.
    pub hulls: Vec<Vec<[f32; 3]>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fgd::{Choice, Property};

    fn property(ty: &str) -> Property {
        Property {
            name: "p".into(),
            ty: ty.into(),
            label: String::new(),
            description: String::new(),
            default: None,
            choices: Vec::new(),
            flags: Vec::new(),
        }
    }

    #[test]
    fn nonfinite_fgd_values_preserve_text_instead_of_generating_invalid_native_numbers() {
        for ty in ["integer", "float", "color1", "color255"] {
            let raw = if ty.starts_with("color") {
                "NaN 1 0"
            } else {
                "NaN"
            };
            assert_eq!(
                Value::from_fgd(Some(&property(ty)), raw),
                Value::String(raw.into())
            );
        }
    }
    #[test]
    fn a_value_with_no_definition_stays_the_text_the_mapper_typed() {
        assert_eq!(Value::from_fgd(None, "3"), Value::String("3".into()));
    }

    #[test]
    fn fgd_types_decide_how_a_value_is_read() {
        assert_eq!(
            Value::from_fgd(Some(&property("integer")), " 7 "),
            Value::Int(7)
        );
        assert_eq!(
            Value::from_fgd(Some(&property("float")), "0.5"),
            Value::Float(0.5)
        );
        assert_eq!(
            Value::from_fgd(Some(&property("string")), "hello"),
            Value::String("hello".into())
        );
    }

    #[test]
    fn a_number_that_will_not_parse_falls_back_to_its_text() {
        assert_eq!(
            Value::from_fgd(Some(&property("integer")), "soon"),
            Value::String("soon".into())
        );
    }

    #[test]
    fn colours_come_out_in_the_zero_to_one_range() {
        assert_eq!(
            Value::from_fgd(Some(&property("color255")), "255 128 0"),
            Value::Color([1.0, 128.0 / 255.0, 0.0, 1.0])
        );
    }

    #[test]
    fn choices_stay_text_unless_every_choice_is_a_number() {
        let mut numeric = property("choices");
        numeric.choices = vec![
            Choice {
                value: "0".into(),
                label: "off".into(),
            },
            Choice {
                value: "1".into(),
                label: "on".into(),
            },
        ];
        assert_eq!(Value::from_fgd(Some(&numeric), "1"), Value::Int(1));

        let mut worded = property("choices");
        worded.choices = vec![Choice {
            value: "loud".into(),
            label: "loud".into(),
        }];
        assert_eq!(
            Value::from_fgd(Some(&worded), "loud"),
            Value::String("loud".into())
        );
    }

    #[test]
    fn flags_that_will_not_parse_read_as_no_flags_set() {
        assert_eq!(Value::from_fgd(Some(&property("flags")), ""), Value::Int(0));
    }
}
