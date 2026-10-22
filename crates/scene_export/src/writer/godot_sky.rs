//! The editor's sky, sun and fog as a Godot `WorldEnvironment`.
//!
//! The sky goes in as an HDR panorama on a `PanoramaSkyMaterial`, which lights the
//! scene as well as filling the background; the sun as a shadowed `DirectionalLight3D`
//! unless the map already has one of its own. Godot's frame is the exported one, so
//! the sun's direction goes in as it is.

use super::sky::Sky;
use crate::math::Vec3;
use crate::scene::{GdValue, Node, Scene, SubResource};

/// The panorama's size in pixels; 2:1, as an equirectangular image is.
pub const PANORAMA_WIDTH: u32 = 1024;
pub const PANORAMA_HEIGHT: u32 = 512;
/// Godot's `Environment.TONE_MAPPER_AGX`, the curve the editor's view uses.
const TONE_MAPPER_AGX: i64 = 4;
/// The view's exposure is applied on top of this, before AgX.
const VIEW_EXPOSURE: f64 = 1.8;

/// The panorama the environment names, as a Radiance `.hdr`.
pub fn panorama(sky: &Sky) -> Vec<u8> {
    sky.panorama_hdr(
        PANORAMA_WIDTH,
        PANORAMA_HEIGHT,
        super::sky::godot_panorama_direction,
    )
}

/// Adds the environment to the scene's root, with the panorama at `panorama_path`
/// (a `res://` path).
pub fn attach(scene: &mut Scene, sky: &Sky, panorama_path: &str) {
    let color = |c: [f32; 3]| GdValue::Color(c[0] as f64, c[1] as f64, c[2] as f64, 1.);
    scene.sub_resources.push(SubResource {
        id: "HeronSkyMaterial".into(),
        ty: "PanoramaSkyMaterial".into(),
        properties: vec![(
            "panorama".into(),
            GdValue::ExtResource {
                ty: "Texture2D".into(),
                path: panorama_path.into(),
            },
        )],
    });
    scene.sub_resources.push(SubResource {
        id: "HeronSky".into(),
        ty: "Sky".into(),
        properties: vec![(
            "sky_material".into(),
            GdValue::SubResource("HeronSkyMaterial".into()),
        )],
    });
    let mut environment = vec![
        ("background_mode".into(), GdValue::Int(2)),
        ("sky".into(), GdValue::SubResource("HeronSky".into())),
        ("ambient_light_source".into(), GdValue::Int(3)),
        (
            "ambient_light_energy".into(),
            GdValue::Float(sky.ambient_intensity as f64),
        ),
        ("reflected_light_source".into(), GdValue::Int(2)),
        ("tonemap_mode".into(), GdValue::Int(TONE_MAPPER_AGX)),
        (
            "tonemap_exposure".into(),
            GdValue::Float(sky.exposure as f64 * VIEW_EXPOSURE),
        ),
    ];
    if let Some((fog, density)) = sky.fog {
        environment.extend([
            ("fog_enabled".into(), GdValue::Bool(true)),
            ("fog_light_color".into(), color(fog)),
            ("fog_density".into(), GdValue::Float(density as f64)),
            // The sky is the map's own and already fades to the horizon; fog over it
            // would paint it out entirely, as it is infinitely far away.
            ("fog_sky_affect".into(), GdValue::Float(0.)),
        ]);
    }
    scene.sub_resources.push(SubResource {
        id: "HeronEnvironment".into(),
        ty: "Environment".into(),
        properties: environment,
    });

    let mut world = Node::new("HeronEnvironment", "WorldEnvironment");
    world.set(
        "environment",
        GdValue::SubResource("HeronEnvironment".into()),
    );
    scene.root.add_child(world);

    if has_class(&scene.root, "DirectionalLight3D") {
        return;
    }
    // A directional light shines along its -Z, so its Z points at the sun.
    let z = Vec3::from_array(sky.to_sun().map(|v| v as f64)).normalize();
    let up = if z.y.abs() > 0.999 {
        Vec3::new(0., 0., 1.)
    } else {
        Vec3::new(0., 1., 0.)
    };
    let x = up.cross(z).normalize();
    let y = z.cross(x);
    let mut sun = Node::new("HeronSun", "DirectionalLight3D");
    // Transform3D takes the basis by rows.
    sun.set(
        "transform",
        GdValue::Transform3D([x.x, y.x, z.x, x.y, y.y, z.y, x.z, y.z, z.z], Vec3::ZERO),
    );
    sun.set("light_color", color(sky.sun_color));
    // The view divides diffuse light by pi and Godot does not.
    sun.set(
        "light_energy",
        GdValue::Float(sky.sun_intensity as f64 / std::f64::consts::PI),
    );
    sun.set("shadow_enabled", GdValue::Bool(true));
    scene.root.add_child(sun);
}

fn has_class(node: &Node, class: &str) -> bool {
    node.class == class || node.children.iter().any(|child| has_class(child, class))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky() -> Sky {
        Sky {
            sky: [0.1, 0.4, 0.85],
            horizon: [0.7, 0.8, 0.9],
            ground: [0.2, 0.18, 0.15],
            sun_color: [1., 0.9, 0.8],
            sun_intensity: 3.,
            ambient_intensity: 0.5,
            sun_elevation: 45.,
            sun_azimuth: 0.,
            exposure: 1.,
            fog: Some(([0.5, 0.6, 0.7], 0.02)),
        }
    }

    #[test]
    fn the_environment_and_sun_go_on_the_root() {
        let mut scene = Scene {
            root: Node::new("map", "Node3D"),
            sub_resources: Vec::new(),
            connections: Vec::new(),
        };
        attach(&mut scene, &sky(), "res://maps/map/sky.hdr");
        let text = crate::writer::tscn::write(&scene);
        assert!(
            text.contains(r#"[ext_resource type="Texture2D" path="res://maps/map/sky.hdr""#),
            "{text}"
        );
        assert!(
            text.contains(r#"[sub_resource type="Environment" id="HeronEnvironment"]"#),
            "{text}"
        );
        assert!(text.contains("fog_enabled = true"), "{text}");
        assert!(
            text.contains(r#"[node name="HeronEnvironment" type="WorldEnvironment" parent="."]"#),
            "{text}"
        );
        assert!(
            text.contains(r#"[node name="HeronSun" type="DirectionalLight3D" parent="."]"#),
            "{text}"
        );
        // Sun at azimuth 0 (map +X, Godot +Z) and 45 degrees up: the light's Z leans to +Z and +Y.
        let sun = scene
            .root
            .children
            .iter()
            .find(|n| n.name == "HeronSun")
            .unwrap();
        let Some(GdValue::Transform3D(b, _)) = sun
            .properties
            .iter()
            .find(|p| p.0 == "transform")
            .map(|p| p.1.clone())
        else {
            panic!()
        };
        let s = std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            (b[2] - 0.).abs() < 1e-6 && (b[5] - s).abs() < 1e-6 && (b[8] - s).abs() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn a_map_with_its_own_sun_keeps_it() {
        let mut root = Node::new("map", "Node3D");
        root.add_child(Node::new("light_environment_1", "DirectionalLight3D"));
        let mut scene = Scene {
            root,
            sub_resources: Vec::new(),
            connections: Vec::new(),
        };
        attach(&mut scene, &sky(), "res://sky.hdr");
        assert!(!scene.root.children.iter().any(|n| n.name == "HeronSun"));
    }
}
