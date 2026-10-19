//! A small slice of Godot's ClassDB: enough inheritance and property types to
//! write typed values into a `.tscn` without guessing. Anything not listed
//! still reaches the node through `metadata/properties`.

/// The Godot type a property expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Bool,
    Int,
    Float,
    String,
    Color,
    Vector2,
    Vector3,
    NodePath,
    /// A `res://` path loaded as a resource of the named class.
    Resource(&'static str),
}

const PARENTS: &[(&str, &str)] = &[
    ("Node3D", "Node"),
    ("Marker3D", "Node3D"),
    ("VisualInstance3D", "Node3D"),
    ("GeometryInstance3D", "VisualInstance3D"),
    ("MeshInstance3D", "GeometryInstance3D"),
    ("GPUParticles3D", "GeometryInstance3D"),
    ("Light3D", "VisualInstance3D"),
    ("OmniLight3D", "Light3D"),
    ("SpotLight3D", "Light3D"),
    ("DirectionalLight3D", "Light3D"),
    ("ReflectionProbe", "VisualInstance3D"),
    ("Decal", "VisualInstance3D"),
    ("CollisionObject3D", "Node3D"),
    ("PhysicsBody3D", "CollisionObject3D"),
    ("StaticBody3D", "PhysicsBody3D"),
    ("AnimatableBody3D", "StaticBody3D"),
    ("RigidBody3D", "PhysicsBody3D"),
    ("CharacterBody3D", "PhysicsBody3D"),
    ("Area3D", "CollisionObject3D"),
    ("CollisionShape3D", "Node3D"),
    ("Camera3D", "Node3D"),
    ("AudioStreamPlayer3D", "Node3D"),
    ("Path3D", "Node3D"),
    ("NavigationRegion3D", "Node3D"),
    ("OccluderInstance3D", "VisualInstance3D"),
];

const PROPERTIES: &[(&str, &str, PropType)] = &[
    ("Node", "process_mode", PropType::Int),
    ("Node3D", "visible", PropType::Bool),
    ("VisualInstance3D", "layers", PropType::Int),
    ("GeometryInstance3D", "cast_shadow", PropType::Int),
    ("GeometryInstance3D", "gi_mode", PropType::Int),
    ("GeometryInstance3D", "gi_lightmap_scale", PropType::Int),
    (
        "GeometryInstance3D",
        "visibility_range_begin",
        PropType::Float,
    ),
    (
        "GeometryInstance3D",
        "visibility_range_end",
        PropType::Float,
    ),
    ("Light3D", "light_color", PropType::Color),
    ("Light3D", "light_energy", PropType::Float),
    ("Light3D", "light_indirect_energy", PropType::Float),
    ("Light3D", "light_volumetric_fog_energy", PropType::Float),
    ("Light3D", "light_specular", PropType::Float),
    ("Light3D", "light_size", PropType::Float),
    ("Light3D", "light_angular_distance", PropType::Float),
    ("Light3D", "light_negative", PropType::Bool),
    ("Light3D", "light_bake_mode", PropType::Int),
    ("Light3D", "light_cull_mask", PropType::Int),
    ("Light3D", "shadow_enabled", PropType::Bool),
    ("Light3D", "shadow_bias", PropType::Float),
    ("Light3D", "shadow_normal_bias", PropType::Float),
    ("Light3D", "shadow_opacity", PropType::Float),
    ("Light3D", "shadow_blur", PropType::Float),
    ("Light3D", "distance_fade_enabled", PropType::Bool),
    ("Light3D", "distance_fade_begin", PropType::Float),
    ("Light3D", "distance_fade_length", PropType::Float),
    ("Light3D", "editor_only", PropType::Bool),
    ("OmniLight3D", "omni_range", PropType::Float),
    ("OmniLight3D", "omni_attenuation", PropType::Float),
    ("OmniLight3D", "omni_shadow_mode", PropType::Int),
    ("SpotLight3D", "spot_range", PropType::Float),
    ("SpotLight3D", "spot_attenuation", PropType::Float),
    ("SpotLight3D", "spot_angle", PropType::Float),
    ("SpotLight3D", "spot_angle_attenuation", PropType::Float),
    (
        "DirectionalLight3D",
        "directional_shadow_mode",
        PropType::Int,
    ),
    (
        "DirectionalLight3D",
        "directional_shadow_max_distance",
        PropType::Float,
    ),
    (
        "DirectionalLight3D",
        "directional_shadow_blend_splits",
        PropType::Bool,
    ),
    ("DirectionalLight3D", "sky_mode", PropType::Int),
    ("CollisionObject3D", "collision_layer", PropType::Int),
    ("CollisionObject3D", "collision_mask", PropType::Int),
    ("CollisionObject3D", "collision_priority", PropType::Float),
    ("CollisionObject3D", "input_ray_pickable", PropType::Bool),
    ("CollisionObject3D", "disable_mode", PropType::Int),
    ("Area3D", "monitoring", PropType::Bool),
    ("Area3D", "monitorable", PropType::Bool),
    ("Area3D", "priority", PropType::Int),
    ("Area3D", "gravity", PropType::Float),
    ("Area3D", "gravity_point", PropType::Bool),
    ("Area3D", "angular_damp", PropType::Float),
    ("Area3D", "linear_damp", PropType::Float),
    ("RigidBody3D", "mass", PropType::Float),
    ("RigidBody3D", "gravity_scale", PropType::Float),
    ("RigidBody3D", "freeze", PropType::Bool),
    ("RigidBody3D", "can_sleep", PropType::Bool),
    ("RigidBody3D", "continuous_cd", PropType::Bool),
    ("PhysicsBody3D", "axis_lock_linear_x", PropType::Bool),
    ("PhysicsBody3D", "axis_lock_linear_y", PropType::Bool),
    ("PhysicsBody3D", "axis_lock_linear_z", PropType::Bool),
    ("Camera3D", "fov", PropType::Float),
    ("Camera3D", "near", PropType::Float),
    ("Camera3D", "far", PropType::Float),
    ("Camera3D", "current", PropType::Bool),
    ("Camera3D", "cull_mask", PropType::Int),
    ("AudioStreamPlayer3D", "volume_db", PropType::Float),
    ("AudioStreamPlayer3D", "unit_size", PropType::Float),
    ("AudioStreamPlayer3D", "max_distance", PropType::Float),
    ("AudioStreamPlayer3D", "autoplay", PropType::Bool),
    ("AudioStreamPlayer3D", "pitch_scale", PropType::Float),
    (
        "AudioStreamPlayer3D",
        "stream",
        PropType::Resource("AudioStream"),
    ),
    ("MeshInstance3D", "mesh", PropType::Resource("Mesh")),
    ("Decal", "texture_albedo", PropType::Resource("Texture2D")),
    ("Decal", "texture_emission", PropType::Resource("Texture2D")),
    (
        "GPUParticles3D",
        "process_material",
        PropType::Resource("Material"),
    ),
    ("GPUParticles3D", "draw_pass_1", PropType::Resource("Mesh")),
    (
        "NavigationRegion3D",
        "navigation_mesh",
        PropType::Resource("NavigationMesh"),
    ),
    ("ReflectionProbe", "intensity", PropType::Float),
    ("ReflectionProbe", "max_distance", PropType::Float),
    ("ReflectionProbe", "size", PropType::Vector3),
    ("Decal", "size", PropType::Vector3),
    ("Decal", "modulate", PropType::Color),
    ("Decal", "albedo_mix", PropType::Float),
    ("GPUParticles3D", "amount", PropType::Int),
    ("GPUParticles3D", "lifetime", PropType::Float),
    ("GPUParticles3D", "emitting", PropType::Bool),
    ("GPUParticles3D", "one_shot", PropType::Bool),
];

/// True when `class` is `ancestor` or inherits from it.
pub fn inherits(class: &str, ancestor: &str) -> bool {
    let mut current = class;
    loop {
        if current == ancestor {
            return true;
        }
        match PARENTS.iter().find(|(c, _)| *c == current) {
            Some((_, parent)) => current = parent,
            None => return false,
        }
    }
}

/// The type `property` expects on `class`, if the table knows about it.
pub fn property_type(class: &str, property: &str) -> Option<PropType> {
    PROPERTIES
        .iter()
        .find(|(owner, name, _)| *name == property && inherits(class, owner))
        .map(|(_, _, ty)| *ty)
}

/// True when the class is one this tool knows how to describe.
pub fn is_known(class: &str) -> bool {
    class == "Node" || PARENTS.iter().any(|(c, _)| *c == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inheritance_walks_the_whole_chain() {
        assert!(inherits("OmniLight3D", "Light3D"));
        assert!(inherits("OmniLight3D", "Node3D"));
        assert!(inherits("OmniLight3D", "Node"));
        assert!(inherits("AnimatableBody3D", "CollisionObject3D"));
        assert!(!inherits("OmniLight3D", "Area3D"));
    }

    #[test]
    fn property_types_come_from_the_owning_class() {
        assert_eq!(
            property_type("OmniLight3D", "light_energy"),
            Some(PropType::Float)
        );
        assert_eq!(
            property_type("OmniLight3D", "omni_range"),
            Some(PropType::Float)
        );
        assert_eq!(property_type("SpotLight3D", "omni_range"), None);
        assert_eq!(
            property_type("Area3D", "collision_layer"),
            Some(PropType::Int)
        );
        assert_eq!(property_type("Marker3D", "light_energy"), None);
    }
}
