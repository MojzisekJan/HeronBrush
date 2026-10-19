//! An in-memory Godot scene, plus the value formatting the `.tscn` writer needs.

use crate::math::{Vec2, Vec3};
use std::fmt::Write as _;

/// A value as it appears on the right hand side of a `.tscn` assignment.
#[derive(Debug, Clone)]
pub enum GdValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Vec2(Vec2),
    Vec3(Vec3),
    /// Red, green, blue and alpha in the 0..1 range.
    Color(f64, f64, f64, f64),
    /// Row major basis followed by the origin, the order `.tscn` uses.
    Transform3D([f64; 9], Vec3),
    PackedVec3(Vec<Vec3>),
    PackedInt32(Vec<i32>),
    Array(Vec<GdValue>),
    Dict(Vec<(String, GdValue)>),
    NodePath(String),
    ExtResource {
        ty: String,
        path: String,
    },
    SubResource(String),
    /// Written through untouched, for anything the enum does not cover.
    Raw(String),
}

impl GdValue {
    pub fn color_from_bytes(r: u8, g: u8, b: u8) -> GdValue {
        GdValue::Color(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0)
    }
}

/// Formats a float the way Godot's own writer does: no trailing zeros, but
/// never bare integers, which the parser would read as a different type.
pub fn push_float(out: &mut String, v: f64) {
    if !v.is_finite() {
        out.push_str(if v.is_nan() {
            "nan"
        } else if v > 0.0 {
            "inf"
        } else {
            "-inf"
        });
        return;
    }
    let rounded = (v * 1e6).round() / 1e6;
    let v = if rounded == 0.0 { 0.0 } else { rounded };
    let start = out.len();
    let _ = write!(out, "{v}");
    let written = &out[start..];
    if !written.contains('.') && !written.contains('e') {
        out.push_str(".0");
    }
}

pub fn format_float(v: f64) -> String {
    if !v.is_finite() {
        return if v.is_nan() {
            "nan".into()
        } else if v > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let rounded = (v * 1e6).round() / 1e6;
    let v = if rounded == 0.0 { 0.0 } else { rounded };
    let mut s = format!("{v}");
    if !s.contains('.') && !s.contains('e') && !s.contains("inf") && !s.contains("nan") {
        s.push_str(".0");
    }
    s
}

pub fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

/// Resolves an external resource path to the id the writer assigned it.
pub trait ResourceIds {
    fn ext_id(&self, ty: &str, path: &str) -> String;
}

impl GdValue {
    pub fn write(&self, out: &mut String, ids: &dyn ResourceIds) {
        match self {
            GdValue::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            GdValue::Int(v) => {
                let _ = write!(out, "{v}");
            }
            GdValue::Float(v) => out.push_str(&format_float(*v)),
            GdValue::Str(v) => {
                let _ = write!(out, "\"{}\"", escape_string(v));
            }
            GdValue::Vec2(v) => {
                let _ = write!(out, "Vector2({}, {})", format_float(v.x), format_float(v.y));
            }
            GdValue::Vec3(v) => {
                let _ = write!(
                    out,
                    "Vector3({}, {}, {})",
                    format_float(v.x),
                    format_float(v.y),
                    format_float(v.z)
                );
            }
            GdValue::Color(r, g, b, a) => {
                let _ = write!(
                    out,
                    "Color({}, {}, {}, {})",
                    format_float(*r),
                    format_float(*g),
                    format_float(*b),
                    format_float(*a)
                );
            }
            GdValue::Transform3D(basis, origin) => {
                out.push_str("Transform3D(");
                for v in basis {
                    out.push_str(&format_float(*v));
                    out.push_str(", ");
                }
                let _ = write!(
                    out,
                    "{}, {}, {})",
                    format_float(origin.x),
                    format_float(origin.y),
                    format_float(origin.z)
                );
            }
            GdValue::PackedVec3(points) => {
                out.push_str("PackedVector3Array(");
                for (i, p) in points.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    push_float(out, p.x);
                    out.push_str(", ");
                    push_float(out, p.y);
                    out.push_str(", ");
                    push_float(out, p.z);
                }
                out.push(')');
            }
            GdValue::PackedInt32(values) => {
                out.push_str("PackedInt32Array(");
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{v}");
                }
                out.push(')');
            }
            GdValue::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write(out, ids);
                }
                out.push(']');
            }
            GdValue::Dict(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (i, (k, v)) in entries.iter().enumerate() {
                    let _ = write!(out, "\"{}\": ", escape_string(k));
                    v.write(out, ids);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push('}');
            }
            GdValue::NodePath(p) => {
                let _ = write!(out, "NodePath(\"{}\")", escape_string(p));
            }
            GdValue::ExtResource { ty, path } => {
                let _ = write!(out, "ExtResource(\"{}\")", ids.ext_id(ty, path));
            }
            GdValue::SubResource(id) => {
                let _ = write!(out, "SubResource(\"{id}\")");
            }
            GdValue::Raw(s) => out.push_str(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubResource {
    pub id: String,
    pub ty: String,
    pub properties: Vec<(String, GdValue)>,
}

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub name: String,
    /// Node class. Left empty when `instance` provides the type.
    pub class: String,
    /// `res://` path of a PackedScene to instance instead of creating a node.
    pub instance: Option<String>,
    pub script: Option<String>,
    pub groups: Vec<String>,
    pub properties: Vec<(String, GdValue)>,
    /// Written as `metadata/<key>`.
    pub meta: Vec<(String, GdValue)>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(name: impl Into<String>, class: impl Into<String>) -> Node {
        Node {
            name: name.into(),
            class: class.into(),
            ..Default::default()
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: GdValue) -> &mut Self {
        self.properties.push((key.into(), value));
        self
    }

    pub fn add_child(&mut self, child: Node) -> &mut Self {
        self.children.push(child);
        self
    }

    /// Renames children so no two siblings collide, the way the editor would.
    pub fn make_names_unique(&mut self) {
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for child in &mut self.children {
            let count = seen.entry(child.name.clone()).or_insert(0);
            if *count > 0 {
                child.name = format!("{}{}", child.name, *count + 1);
            }
            *count += 1;
            child.make_names_unique();
        }
    }
}

/// One `[connection]` line: a signal on `from` wired to a method on `to`.
#[derive(Debug, Clone)]
pub struct Connection {
    pub signal: String,
    /// Node path relative to the scene root.
    pub from: String,
    pub to: String,
    pub method: String,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub root: Node,
    pub sub_resources: Vec<SubResource>,
    pub connections: Vec<Connection>,
}

/// Godot's YXZ euler order, as a row major basis.
pub fn basis_from_euler_yxz(euler_radians: Vec3) -> [f64; 9] {
    let (sx, cx) = euler_radians.x.sin_cos();
    let (sy, cy) = euler_radians.y.sin_cos();
    let (sz, cz) = euler_radians.z.sin_cos();

    // Y * X * Z, matching Basis::from_euler with EULER_ORDER_YXZ.
    let x = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let y = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let z = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];

    let yx = mul3(&y, &x);
    let m = mul3(&yx, &z);
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    ]
}

fn mul3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}

/// Row major basis scaled along each local axis.
pub fn scale_basis(basis: [f64; 9], scale: Vec3) -> [f64; 9] {
    [
        basis[0] * scale.x,
        basis[1] * scale.y,
        basis[2] * scale.z,
        basis[3] * scale.x,
        basis[4] * scale.y,
        basis[5] * scale.z,
        basis[6] * scale.x,
        basis[7] * scale.y,
        basis[8] * scale.z,
    ]
}

pub const IDENTITY_BASIS: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

#[cfg(test)]
mod tests {
    use super::*;

    struct NoIds;
    impl ResourceIds for NoIds {
        fn ext_id(&self, _ty: &str, path: &str) -> String {
            path.to_string()
        }
    }

    fn render(v: &GdValue) -> String {
        let mut s = String::new();
        v.write(&mut s, &NoIds);
        s
    }

    #[test]
    fn floats_never_lose_their_decimal_point() {
        assert_eq!(format_float(1.0), "1.0");
        assert_eq!(format_float(-0.0), "0.0");
        assert_eq!(format_float(0.5), "0.5");
        assert_eq!(format_float(1.0 / 3.0), "0.333333");
    }

    #[test]
    fn values_render_the_way_godot_writes_them() {
        assert_eq!(
            render(&GdValue::Vec3(Vec3::new(1.0, 2.0, 3.0))),
            "Vector3(1.0, 2.0, 3.0)"
        );
        assert_eq!(
            render(&GdValue::Color(1.0, 0.5, 0.0, 1.0)),
            "Color(1.0, 0.5, 0.0, 1.0)"
        );
        assert_eq!(render(&GdValue::Bool(true)), "true");
        assert_eq!(render(&GdValue::Str("a\"b".into())), "\"a\\\"b\"");
        assert_eq!(
            render(&GdValue::PackedVec3(vec![Vec3::ZERO, Vec3::X])),
            "PackedVector3Array(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)"
        );
    }

    #[test]
    fn packed_int_arrays_render_flat() {
        assert_eq!(
            render(&GdValue::PackedInt32(vec![0, 1, 2])),
            "PackedInt32Array(0, 1, 2)"
        );
        assert_eq!(render(&GdValue::PackedInt32(vec![])), "PackedInt32Array()");
    }

    #[test]
    fn a_quarter_turn_around_y_gives_the_basis_godot_stores() {
        let b = basis_from_euler_yxz(Vec3::new(0.0, std::f64::consts::FRAC_PI_2, 0.0));
        // Row 0 is (cos, 0, sin), which is what a .tscn shows for a 90 degree yaw.
        assert!((b[0] - 0.0).abs() < 1e-12);
        assert!((b[2] - 1.0).abs() < 1e-12);
        assert!((b[6] + 1.0).abs() < 1e-12);
        assert!((b[8] - 0.0).abs() < 1e-12);
    }

    #[test]
    fn identity_euler_is_the_identity_basis() {
        let b = basis_from_euler_yxz(Vec3::ZERO);
        for (a, e) in b.iter().zip(IDENTITY_BASIS.iter()) {
            assert!((a - e).abs() < 1e-15);
        }
    }

    #[test]
    fn sibling_names_are_made_unique() {
        let mut root = Node::new("root", "Node3D");
        root.add_child(Node::new("wall", "StaticBody3D"));
        root.add_child(Node::new("wall", "StaticBody3D"));
        root.add_child(Node::new("wall", "StaticBody3D"));
        root.make_names_unique();
        let names: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["wall", "wall2", "wall3"]);
    }
}
