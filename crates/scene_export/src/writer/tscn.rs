//! Godot text scene writer.

use crate::scene::{GdValue, Node, ResourceIds, Scene};
use indexmap::IndexMap;
use std::fmt::Write as _;

#[derive(Default)]
struct ExtResources {
    /// Path to (type, id), in the order the paths were first seen.
    entries: IndexMap<String, (String, String)>,
}

impl ExtResources {
    fn add(&mut self, ty: &str, path: &str) {
        let next = self.entries.len() + 1;
        self.entries
            .entry(path.to_string())
            .or_insert_with(|| (ty.to_string(), format!("{next}_{}", short_hash(path))));
    }
}

impl ResourceIds for ExtResources {
    fn ext_id(&self, _ty: &str, path: &str) -> String {
        self.entries
            .get(path)
            .map(|(_, id)| id.clone())
            .unwrap_or_else(|| path.to_string())
    }
}

/// Short stable suffix so resource ids survive a rebuild unchanged.
fn short_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut out = String::new();
    for _ in 0..5 {
        out.push(ALPHABET[(h % ALPHABET.len() as u64) as usize] as char);
        h /= ALPHABET.len() as u64;
    }
    out
}

pub fn write(scene: &Scene) -> String {
    let mut ext = ExtResources::default();
    collect_node(&scene.root, &mut ext);
    for sub in &scene.sub_resources {
        for (_, value) in &sub.properties {
            collect_value(value, &mut ext);
        }
    }

    // No `load_steps`: Godot 4.6 and newer ignore it and no longer write it, so leaving
    // it out keeps a scene the engine re-saves free of a spurious diff. Older 4.x
    // versions treat it as optional too.
    let mut out = String::new();
    let _ = writeln!(out, "[gd_scene format=3]\n");

    for (path, (ty, id)) in &ext.entries {
        let _ = writeln!(
            out,
            "[ext_resource type=\"{ty}\" path=\"{path}\" id=\"{id}\"]"
        );
    }
    if !ext.entries.is_empty() {
        out.push('\n');
    }

    for sub in &scene.sub_resources {
        let _ = writeln!(out, "[sub_resource type=\"{}\" id=\"{}\"]", sub.ty, sub.id);
        for (key, value) in &sub.properties {
            let _ = write!(out, "{key} = ");
            value.write(&mut out, &ext);
            out.push('\n');
        }
        out.push('\n');
    }

    write_node(&scene.root, None, &ext, &mut out);

    for c in &scene.connections {
        let _ = writeln!(
            out,
            "[connection signal=\"{}\" from=\"{}\" to=\"{}\" method=\"{}\"]",
            c.signal, c.from, c.to, c.method
        );
    }
    out
}

fn collect_node(node: &Node, ext: &mut ExtResources) {
    if let Some(path) = &node.instance {
        ext.add("PackedScene", path);
    }
    if let Some(path) = &node.script {
        ext.add("Script", path);
    }
    for (_, value) in node.properties.iter().chain(node.meta.iter()) {
        collect_value(value, ext);
    }
    for child in &node.children {
        collect_node(child, ext);
    }
}

fn collect_value(value: &GdValue, ext: &mut ExtResources) {
    match value {
        GdValue::ExtResource { ty, path } => ext.add(ty, path),
        GdValue::Array(items) => items.iter().for_each(|v| collect_value(v, ext)),
        GdValue::Dict(entries) => entries.iter().for_each(|(_, v)| collect_value(v, ext)),
        _ => {}
    }
}

fn write_node(node: &Node, parent_path: Option<&str>, ext: &ExtResources, out: &mut String) {
    let _ = write!(
        out,
        "[node name=\"{}\"",
        crate::scene::escape_string(&node.name)
    );
    if node.instance.is_none() && !node.class.is_empty() {
        let _ = write!(out, " type=\"{}\"", node.class);
    }
    if let Some(parent) = parent_path {
        let _ = write!(out, " parent=\"{parent}\"");
    }
    if let Some(path) = &node.instance {
        let _ = write!(
            out,
            " instance=ExtResource(\"{}\")",
            ext.ext_id("PackedScene", path)
        );
    }
    if !node.groups.is_empty() {
        out.push_str(" groups=[");
        for (i, g) in node.groups.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "\"{}\"", crate::scene::escape_string(g));
        }
        out.push(']');
    }
    out.push_str("]\n");

    if let Some(path) = &node.script {
        let _ = writeln!(
            out,
            "script = ExtResource(\"{}\")",
            ext.ext_id("Script", path)
        );
    }
    for (key, value) in &node.properties {
        let _ = write!(out, "{key} = ");
        value.write(out, ext);
        out.push('\n');
    }
    for (key, value) in &node.meta {
        let _ = write!(out, "metadata/{key} = ");
        value.write(out, ext);
        out.push('\n');
    }
    out.push('\n');

    let child_parent = match parent_path {
        None => ".".to_string(),
        Some(".") => node.name.clone(),
        Some(p) => format!("{p}/{}", node.name),
    };
    for child in &node.children {
        write_node(child, Some(&child_parent), ext, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use crate::scene::{SubResource, IDENTITY_BASIS};

    #[test]
    fn writes_a_scene_with_resources_and_nesting() {
        let mut root = Node::new("testmap", "Node3D");

        let mut body = Node::new("worldspawn", "StaticBody3D");
        let mut mesh_instance = Node::new("mesh", "MeshInstance3D");
        mesh_instance.set(
            "mesh",
            GdValue::ExtResource {
                ty: "ArrayMesh".into(),
                path: "res://maps/testmap/worldspawn.obj".into(),
            },
        );
        mesh_instance.set(
            "surface_material_override/0",
            GdValue::SubResource("Material_stone".into()),
        );
        body.add_child(mesh_instance);

        let mut shape = Node::new("shape_0", "CollisionShape3D");
        shape.set("shape", GdValue::SubResource("Convex_0".into()));
        body.add_child(shape);
        root.add_child(body);

        let mut light = Node::new("light_omni_1", "OmniLight3D");
        light.set(
            "transform",
            GdValue::Transform3D(IDENTITY_BASIS, Vec3::new(0.0, 2.0, 0.0)),
        );
        light.set("light_energy", GdValue::Float(2.5));
        light.groups.push("lights".into());
        light.meta.push((
            "properties".into(),
            GdValue::Dict(vec![(
                "classname".into(),
                GdValue::Str("light_omni".into()),
            )]),
        ));
        root.add_child(light);

        let scene = Scene {
            root,
            sub_resources: vec![SubResource {
                id: "Convex_0".into(),
                ty: "ConvexPolygonShape3D".into(),
                properties: vec![("points".into(), GdValue::PackedVec3(vec![Vec3::ZERO]))],
            }],
            connections: Vec::new(),
        };

        let text = write(&scene);
        assert!(text.starts_with("[gd_scene format=3]"), "{text}");
        assert!(text.contains(
            r#"[ext_resource type="ArrayMesh" path="res://maps/testmap/worldspawn.obj""#
        ));
        assert!(text.contains(r#"[sub_resource type="ConvexPolygonShape3D" id="Convex_0"]"#));
        assert!(text.contains(r#"[node name="testmap" type="Node3D"]"#));
        assert!(text.contains(r#"[node name="worldspawn" type="StaticBody3D" parent="."]"#));
        assert!(text.contains(r#"[node name="mesh" type="MeshInstance3D" parent="worldspawn"]"#));
        assert!(text.contains(
            r#"[node name="light_omni_1" type="OmniLight3D" parent="." groups=["lights"]]"#
        ));
        assert!(text.contains("metadata/properties = {"));
        assert!(text.contains(
            "transform = Transform3D(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 0.0)"
        ));
    }

    #[test]
    fn the_same_path_reuses_one_resource_id() {
        let mut root = Node::new("root", "Node3D");
        for i in 0..2 {
            let mut n = Node::new(format!("m{i}"), "MeshInstance3D");
            n.set(
                "mesh",
                GdValue::ExtResource {
                    ty: "ArrayMesh".into(),
                    path: "res://a.obj".into(),
                },
            );
            root.add_child(n);
        }
        let text = write(&Scene {
            root,
            sub_resources: Vec::new(),
            connections: Vec::new(),
        });
        assert_eq!(text.matches("[ext_resource").count(), 1);
    }

    #[test]
    fn connections_are_written_after_the_nodes() {
        let mut root = Node::new("map", "Node3D");
        root.add_child(Node::new("trigger", "Area3D"));
        root.add_child(Node::new("door", "AnimatableBody3D"));
        let text = write(&Scene {
            root,
            sub_resources: Vec::new(),
            connections: vec![crate::scene::Connection {
                signal: "body_entered".into(),
                from: "trigger".into(),
                to: "door".into(),
                method: "_on_trigger".into(),
            }],
        });
        let line =
            r#"[connection signal="body_entered" from="trigger" to="door" method="_on_trigger"]"#;
        assert!(text.contains(line), "{text}");
        assert!(
            text.find(line).unwrap() > text.find("[node name=\"door\"").unwrap(),
            "connections belong at the end"
        );
    }

    #[test]
    fn instanced_scenes_have_no_type_field() {
        let mut root = Node::new("root", "Node3D");
        let mut n = Node::new("lamp", "");
        n.instance = Some("res://objects/lamp.tscn".into());
        root.add_child(n);
        let text = write(&Scene {
            root,
            sub_resources: Vec::new(),
            connections: Vec::new(),
        });
        assert!(
            text.contains(r#"[node name="lamp" parent="." instance=ExtResource("#),
            "{text}"
        );
    }
}
