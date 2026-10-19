//! Editor-only point-light symbol, contained inside the entity's picking bounds.
use geometry_math::{
    bbox::Bbox3d,
    vec::{self, Vec2f, Vec3d},
};
use map_model::{brush_face::SurfaceMaterial, node::EntityNode};
use render::Triangle;

fn light_color(entity: &EntityNode) -> [f32; 3] {
    for (key, scale) in [("light_color", 255.0), ("_color", 1.0)] {
        if let Some(text) = entity.property(key) {
            let values: Vec<_> = text.split_whitespace().map(str::parse::<f32>).collect();
            if let [Ok(r), Ok(g), Ok(b)] = values.as_slice() {
                if [r, g, b].iter().all(|v| v.is_finite()) {
                    // A dark light still needs a readable editor symbol.
                    return [*r, *g, *b].map(|v| 0.18 + 0.82 * (v / scale).clamp(0.0, 1.0));
                }
            }
        }
    }
    [1.0, 0.88, 0.48]
}

pub(super) fn triangles(
    entity: &EntityNode,
    bounds: &Bbox3d,
    selected: bool,
    locked: bool,
) -> Option<Vec<Triangle>> {
    if entity.classname()? != "light" {
        return None;
    }
    let size = bounds.size();
    let scale = (size.x().min(size.y()).min(size.z()) / 16.0).clamp(0.001, 1.0);
    let center = bounds.center();
    let color = light_color(entity);
    let mut output = Vec::with_capacity(256);
    let mut face = |a: Vec3d, b: Vec3d, c: Vec3d, color, emission| {
        let normal = vec::cross(b - a, c - a);
        if vec::length(normal) < 1e-9 {
            return;
        }
        output.push(Triangle {
            corner_normals: None,
            positions: [a, b, c].map(|p| center + p * scale),
            normal: vec::normalize(normal),
            uv: [Vec2f::zero(); 3],
            material: None,
            color,
            alpha: 1.0,
            corner_colors: None,
            selected,
            painted: false,
            locked,
            surface_material: Some(SurfaceMaterial {
                roughness: 38,
                metallic: 0,
                emission,
            }),
        });
    };
    // A compact bulb with a pinched neck, stepped socket and domed diffuser.
    let rings = [
        (-7.5, 0.0),
        (-7.0, 1.65),
        (-6.4, 2.1),
        (-5.5, 2.1),
        (-5.2, 2.45),
        (-4.4, 2.45),
        (-4.0, 2.0),
        (-2.7, 2.4),
        (-1.0, 4.3),
        (1.2, 5.1),
        (3.3, 4.5),
        (4.9, 2.9),
        (5.6, 0.0),
    ];
    const SIDES: usize = 12;
    for (band, pair) in rings.windows(2).enumerate() {
        let [(z0, r0), (z1, r1)] = [pair[0], pair[1]];
        let point = |z, r: f64, i: usize| {
            let a = i as f64 * std::f64::consts::TAU / SIDES as f64;
            Vec3d::new(r * a.cos(), r * a.sin(), z)
        };
        let tint = if band < 6 {
            if band % 2 == 0 {
                [0.19, 0.25, 0.31]
            } else {
                [0.48, 0.56, 0.62]
            }
        } else {
            color
        };
        for i in 0..SIDES {
            let (a, b, c, d) = (
                point(z0, r0, i),
                point(z0, r0, i + 1),
                point(z1, r1, i + 1),
                point(z1, r1, i),
            );
            face(a, b, c, tint, if band < 6 { 12 } else { 65 });
            face(a, c, d, tint, if band < 6 { 12 } else { 65 });
        }
    }
    // Short solid rays remain legible from the side, unlike a flat billboard.
    for direction in [
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(-1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
        Vec3d::new(0.0, -1.0, 0.0),
        Vec3d::new(0.0, 0.0, 1.0),
    ] {
        let u = vec::normalize(vec::cross(
            direction,
            if direction.z().abs() > 0.9 {
                Vec3d::new(1.0, 0.0, 0.0)
            } else {
                Vec3d::new(0.0, 0.0, 1.0)
            },
        )) * 0.24;
        let v = vec::cross(direction, u);
        let a = direction * 6.4;
        let b = direction * 7.8;
        let rim = [u, v, -u, -v];
        for i in 0..4 {
            let j = (i + 1) % 4;
            face(a + rim[i], a + rim[j], b + rim[j], color, 65);
            face(a + rim[i], b + rim[j], b + rim[i], color, 65);
            face(a, a + rim[j], a + rim[i], color, 65);
            face(b, b + rim[i], b + rim[j], color, 65);
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "writes a manual shape preview to /tmp"]
    fn preview_point_light_symbol() {
        let mut document = super::super::Document::new(crate::world::WorldBounds::default());
        document.create_point_entity("light", Vec3d::zero());
        document.clear_selection();
        let mut mesh = document.triangles(&crate::view_filters::ViewFilters::default());
        let eye = vec::normalize(Vec3d::new(0.5, -0.86, 0.2));
        let right = vec::normalize(vec::cross(eye, Vec3d::new(0.0, 0.0, 1.0)));
        let up = vec::cross(right, eye);
        mesh.sort_by(|a, b| {
            let depth = |t: &Triangle| {
                vec::dot(
                    eye,
                    t.positions
                        .iter()
                        .copied()
                        .fold(Vec3d::zero(), |a, b| a + b),
                )
            };
            depth(a).total_cmp(&depth(b))
        });
        use crate::canvas::Canvas;
        let mut cr = crate::canvas::Raster::new(320, 320).unwrap();
        cr.set_source_rgb(0.09, 0.11, 0.14);
        cr.paint();
        for t in mesh {
            if vec::dot(t.normal, eye) <= 0.0 {
                continue;
            }
            let shade = 0.6
                + 0.4 * vec::dot(t.normal, vec::normalize(Vec3d::new(-1.0, -2.0, 3.0))).max(0.0);
            cr.set_source_rgb(
                t.color[0] as f64 * shade,
                t.color[1] as f64 * shade,
                t.color[2] as f64 * shade,
            );
            for (i, p) in t.positions.iter().enumerate() {
                let x = 160.0 + vec::dot(*p, right) * 17.0;
                let y = 160.0 - vec::dot(*p, up) * 17.0;
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.close_path();
            cr.fill();
        }
        cr.into_rgba()
            .save_png(&std::env::temp_dir().join("heron-light-marker.png"))
            .unwrap();
    }
}
