//! The `.heron_brush.json` level file, field for field.
//!
//! Every field `docs/level-format.md` describes, including the ones this
//! viewer never looks at, so whoever writes a real loader can see what
//! arrives. The only Bevy in here is turning a transform into one.

#![allow(dead_code)]

use bevy::prelude::{EulerRot, Quat, Resource, Transform, Vec3};
use serde::{Deserialize,Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The version this loader was written against. A file that claims a higher
/// one may mean something else by the same fields, so it is refused rather
/// than half understood.
///
/// 2 added `collision` on solid entities, consumed by the Avian adapter.
pub const FORMAT: u32 = 2;

#[derive(Serialize,Deserialize, Resource)]
pub struct Level {
    pub format: u32,
    #[serde(default)]
    pub generator: String,
    pub name: String,
    /// Map units in one metre. Nothing is scaled by it; the geometry arrives in
    /// metres already. It is here so a position can be shown back in the units
    /// the mapper reads off the editor's grid.
    pub scale: f32,
    #[serde(default)]
    pub textures: Vec<Texture>,
    #[serde(default)]
    pub entities: Vec<Entity>,
    /// The map's sky, sun and fog, when the export carried them (`heron_sky`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<serde_json::Value>,
}

#[derive(Serialize,Deserialize)]
pub struct Texture {
    /// Roughness, metallic and albedo-coloured emission from painted faces.
    #[serde(default)]
    pub surface: Option<[f32; 3]>,
    pub name: String,
    /// Relative to the project root, which for a Bevy game is `assets/`.
    /// Absent when nothing matched on disk.
    pub image: Option<String>,
    /// Files found next to the albedo image: `normal`, `roughness`,
    /// `metallic`, `emission`, `ao`, `orm`.
    #[serde(default)]
    pub maps: Vec<(String, String)>,
    #[serde(default)]
    pub size: [f32; 2],
    /// Baked face paint: draw it clamped to its edges, not repeated.
    #[serde(default)]
    pub clamp: bool,
}

#[derive(Serialize,Deserialize)]
pub struct Entity {
    pub name: String,
    pub classname: String,
    #[serde(default)]
    pub group: Vec<String>,
    pub transform: Placement,
    #[serde(default)]
    pub solid: bool,
    /// Relative to the level file's own directory.
    pub mesh: Option<String>,
    #[serde(default)]
    pub surfaces: Vec<Surface>,
    /// Collision geometry, in a glTF mesh of its own in the entity's `.glb`.
    /// Absent for `skip` brushwork, point entities, and `collision=none`.
    pub collision: Option<Collision>,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub scene: Option<String>,
}

impl Entity {
    pub fn float(&self, key: &str, fallback: f32) -> f32 {
        self.properties
            .get(key)
            .and_then(Value::as_f32)
            .unwrap_or(fallback)
    }

    pub fn colour(&self, key: &str) -> Option<[f32; 4]> {
        match self.properties.get(key) {
            Some(Value::Color(c)) => Some(*c),
            _ => None,
        }
    }
}

#[derive(Serialize,Deserialize)]
pub struct Surface {
    /// Index of a primitive in the entity's `.glb`. They run 0, 1, 2 with no
    /// gaps, all in glTF mesh 0.
    pub primitive: usize,
    pub texture: String,
}

#[derive(Serialize,Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CollisionShape {
    /// One concave triangle soup: static world geometry.
    Trimesh,
    /// A set of convex hulls: a mover the physics engine pushes around.
    Convex,
}

/// Where an entity's collision geometry sits in its `.glb`. Load it in Bevy
/// with Avian: `ColliderConstructor::TrimeshFromMesh` for a `Trimesh`,
/// `ConvexHullFromMesh` per part for `Convex`, reading
/// `GltfAssetLabel::Primitive { mesh, primitive }` for `primitive` in
/// `0..parts`.
#[derive(Serialize,Deserialize)]
pub struct Collision {
    pub kind: CollisionShape,
    /// glTF mesh index: 1 when the entity also draws, 0 when it does not.
    pub mesh: usize,
    /// Primitive count in `mesh`, `0..parts`.
    pub parts: usize,
    /// Sensors report overlaps without blocking movement.
    #[serde(default)]
    pub sensor: bool,
}

/// Full placement of the local geometry, including rotation and nonuniform scale.
#[derive(Serialize,Deserialize)]
pub struct Placement {
    pub translation: [f32; 3],
    /// Euler angles in radians, in x, y, z order, applied Y first, then X,
    /// then Z. The name is the order they are applied in, not the order they
    /// are written in.
    pub rotation_yxz: [f32; 3],
    pub scale: [f32; 3],
}

impl Placement {
    pub fn rotation(&self) -> Quat {
        let [x, y, z] = self.rotation_yxz;
        Quat::from_euler(EulerRot::YXZ, y, x, z)
    }

    pub fn to_transform(&self) -> Transform {
        Transform {
            translation: Vec3::from(self.translation),
            rotation: self.rotation(),
            scale: Vec3::from(self.scale),
        }
    }

    pub fn yaw(&self) -> f32 {
        self.rotation_yxz[1]
    }
}

/// A property, with what the FGD said its type was. A key the FGD never
/// declared arrives as a `String`, because that is all the map knew.
#[derive(Serialize,Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    /// Components run 0..1 whichever way the FGD wrote them.
    Color([f32; 4]),
    Array(Vec<serde_json::Value>),
}

impl Value {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Int(i) => Some(*i as f32),
            Value::Float(f) => Some(*f as f32),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

pub fn load(path: &Path) -> Result<Level, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let level: Level =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if level.format > FORMAT {
        return Err(format!(
            "{}: format {} is newer than the {FORMAT} this demo reads",
            path.display(),
            level.format
        ));
    }
    Ok(level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_placement_preserves_all_axes_and_scale() {
        let p = Placement {
            translation: [1.0, 2.0, 3.0],
            rotation_yxz: [0.3, 0.7, -0.2],
            scale: [2.0, 3.0, 4.0],
        };
        let t = p.to_transform();
        let expected =
            Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.3) * Quat::from_rotation_z(-0.2);
        assert!(t.rotation.dot(expected).abs() > 0.999999);
        assert_eq!(t.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.scale, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn sensors_and_custom_game_properties_survive_loading() {
        let level: Level = serde_json::from_str(include_str!("../assets/maps/demo.json")).unwrap();
        assert!(level
            .entities
            .iter()
            .any(|e| e.collision.as_ref().is_some_and(|c| c.sensor)));
        let door = level
            .entities
            .iter()
            .find(|e| e.classname == "func_door")
            .unwrap();
        assert_eq!(door.float("travel", 0.0), 96.0);
        let portal = level
            .entities
            .iter()
            .find(|e| e.classname == "fps_portal")
            .unwrap();
        assert!(level
            .entities
            .iter()
            .any(|e| portal.targets.contains(&e.name)));
        let custom: Value =
            serde_json::from_str(r#"{"type":"string","value":"my_game:quest/42"}"#).unwrap();
        assert_eq!(custom.as_str(), Some("my_game:quest/42"));
    }
}
