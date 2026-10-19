//! Non-destructive preview of continuously moving brush entities.
use geometry_math::{
    bbox::Bbox3d,
    vec::{Vec3d, cross, dot, normalize},
};
use map_model::node::{EntityNode, Node, NodeId, NodeKind, World};
use render::Triangle;
use std::{collections::HashMap, ops::Range};

#[derive(Clone, Debug)]
pub enum Motion {
    Playback {pivot:Vec3d,authored:std::sync::Arc<crate::keyframe_animation::Track>,track:std::sync::Arc<crate::keyframe_animation::Track>,offset:f64,epoch:f64,playing:bool,posed:bool},
    Keyframes { autoplay:bool, pivot:Vec3d, track:std::sync::Arc<crate::keyframe_animation::Track> },
    Rotate {
        pivot: Vec3d,
        axis: Vec3d,
        speed: f64,
    },
    Shuttle {
        direction: Vec3d,
        travel: f64,
        speed: f64,
        wait: f64,
    },
}
impl Motion {
    pub fn transform(&self, point: Vec3d, time: f64, normal: bool) -> Vec3d {
        match self.clone() {
            Self::Playback{pivot,track,offset,epoch,playing,posed,..}=>if posed{track.transform(point,pivot,offset+if playing{(time-epoch).max(0.0)}else{0.0},normal)}else{point},
            Self::Keyframes {pivot,track,autoplay} => if autoplay {track.transform(point,pivot,time,normal)}else{point},
            Self::Rotate { pivot, axis, speed } => {
                let angle = (speed * time).rem_euclid(360.0).to_radians();
                let v = if normal { point } else { point - pivot };
                let rotated = v * angle.cos()
                    + cross(axis, v) * angle.sin()
                    + axis * dot(axis, v) * (1.0 - angle.cos());
                if normal { rotated } else { rotated + pivot }
            }
            Self::Shuttle {
                direction,
                travel,
                speed,
                wait,
            } => {
                if normal || travel == 0.0 || speed == 0.0 {
                    return point;
                }
                let duration = travel.abs() / speed;
                let phase = time.rem_euclid(2.0 * (duration + wait));
                let fraction = if phase < duration + wait {
                    (phase / duration).min(1.0)
                } else {
                    (1.0 - (phase - duration - wait) / duration).max(0.0)
                };
                point + direction * travel * fraction
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub motions: Vec<(Range<usize>, Motion)>,
}
impl Mesh {
    pub fn at_frame(&self, time: f64) -> Vec<Triangle> {
        let mut triangles = self.triangles.clone();
        for (range, motion) in &self.motions {
            pose(&mut triangles[range.clone()], motion, time, true);
        }
        triangles
    }
    pub fn at(&self, time: f64) -> Vec<Triangle> {
        let mut triangles = self.triangles.clone();
        for (range, motion) in &self.motions {
            pose(&mut triangles[range.clone()], motion, time, false);
        }
        triangles
    }
}

/// Moves `triangles` where `motion` has them `time` seconds in: a scrubbed frame
/// (`scrubbed`, keyframes clamped to the track) or playback.
pub fn pose(triangles: &mut [Triangle], motion: &Motion, time: f64, scrubbed: bool) {
    let transform = |p, normal| match motion {
        Motion::Keyframes { pivot, track, .. } | Motion::Playback { pivot, authored: track, .. } if scrubbed => {
            track.transform_clamped(p, *pivot, time, normal)
        }
        _ => motion.transform(p, time, normal),
    };
    for triangle in triangles {
        triangle.positions = triangle.positions.map(|p| transform(p, false));
        triangle.normal = transform(triangle.normal, true);
        triangle.corner_normals = triangle.corner_normals.map(|normals| normals.map(|n| transform(n, true)));
    }
}
fn number(entity: &EntityNode, key: &str, default: f64) -> f64 {
    entity
        .property(key)
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}
fn vector(entity: &EntityNode, key: &str, default: Vec3d) -> Vec3d {
    let values: Vec<f64> = entity
        .property(key)
        .unwrap_or("")
        .split_whitespace()
        .filter_map(|v| v.parse::<f64>().ok())
        .collect();
    if values.len() != 3 || values.iter().any(|v| !v.is_finite()) {
        return default;
    }
    Vec3d::new(values[0], values[1], values[2])
}
fn collect_brushes<'a>(nodes: &'a [Node], output: &mut Vec<(NodeId, &'a map_model::brush::Brush)>) {
    for node in nodes {
        match &node.kind {
            NodeKind::Brush(brush) => output.push((node.id, brush)),
            NodeKind::Group(group) => collect_brushes(&group.children, output),
            _ => {}
        }
    }
}
fn entity_motion(entity: &EntityNode) -> Option<Motion> {
    if let Some(text) = entity.property("animation_keys") {
        let track = crate::keyframe_animation::Track::from_properties(text, entity.property("animation_mode").unwrap_or("loop"), entity.property("animation_curves")).ok()?;
        let mut parts=Vec::new(); collect_brushes(&entity.children,&mut parts);
        let center=Bbox3d::merge_all(parts.iter().flat_map(|(_,b)| [b.bounds().min,b.bounds().max])).map(|b| b.center())?;
        let pivot=vector(entity,"animation_pivot",center);
        return Some(Motion::Keyframes{autoplay:number(entity,"start_on",1.0)!=0.0,pivot,track:std::sync::Arc::new(track)});
    }
    if number(entity,"start_on",1.0)==0.0 {return None;}
    match entity.classname()? {
        "func_rotate" => {
            let axis = vector(entity, "axis", Vec3d::new(0.0, 0.0, 1.0));
            let speed = number(entity, "speed", 90.0);
            if dot(axis, axis) < 1e-12 || speed == 0.0 {
                return None;
            }
            let mut parts = Vec::new();
            collect_brushes(&entity.children, &mut parts);
            let bounds = |origin_only: bool| {
                Bbox3d::merge_all(
                    parts
                        .iter()
                        .filter(|(_, b)| {
                            !origin_only
                                || b.faces().iter().all(|f| {
                                    f.material_name
                                        .rsplit('/')
                                        .next()
                                        .is_some_and(|n| n.eq_ignore_ascii_case("origin"))
                                })
                        })
                        .flat_map(|(_, b)| [b.bounds().min, b.bounds().max]),
                )
            };
            let pivot = bounds(true)
                .map(|b| b.center())
                .or_else(|| {
                    entity
                        .origin()
                        .filter(|v| v.v.iter().all(|x| x.is_finite()))
                })
                .or_else(|| bounds(false).map(|b| b.center()))?;
            Some(Motion::Rotate {
                pivot,
                axis: normalize(axis),
                speed,
            })
        }
        "func_shuttle" => {
            let direction = vector(entity, "direction", Vec3d::new(1.0, 0.0, 0.0));
            let speed = number(entity, "speed", 64.0).max(0.0);
            let travel = number(entity, "travel", 128.0);
            if dot(direction, direction) < 1e-12 || speed == 0.0 || travel == 0.0 {
                return None;
            }
            Some(Motion::Shuttle {
                direction: normalize(direction),
                travel,
                speed,
                wait: number(entity, "wait", 0.5).max(0.0),
            })
        }
        _ => None,
    }
}
pub fn motions(world: &World) -> HashMap<NodeId, Motion> {
    fn visit(nodes: &[Node], result: &mut HashMap<NodeId, Motion>) {
        for node in nodes {
            match &node.kind {
                NodeKind::Entity(entity) => {
                    if let Some(motion) = entity_motion(entity) {
                        let mut parts = Vec::new();
                        collect_brushes(&entity.children, &mut parts);
                        result.extend(parts.into_iter().map(|(id, _)| (id, motion.clone())));
                    }
                }
                NodeKind::Group(group) => visit(&group.children, result),
                NodeKind::Layer(layer) => visit(&layer.children, result),
                _ => {}
            }
        }
    }
    let mut result = HashMap::new();
    for layer in &world.layers {
        visit(&layer.children, &mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: Vec3d, b: Vec3d) {
        assert!(dot(a - b, a - b) < 1e-12, "{a:?} != {b:?}");
    }
    #[test]
    fn rotation_respects_pivot_and_rotates_normals_without_translation() {
        let motion = Motion::Rotate {
            pivot: Vec3d::new(10.0, 20.0, 30.0),
            axis: Vec3d::new(0.0, 0.0, 1.0),
            speed: 90.0,
        };
        close(
            motion.transform(Vec3d::new(12.0, 20.0, 30.0), 1.0, false),
            Vec3d::new(10.0, 22.0, 30.0),
        );
        close(
            motion.transform(Vec3d::new(1.0, 0.0, 0.0), 1.0, true),
            Vec3d::new(0.0, 1.0, 0.0),
        );
    }
    #[test]
    fn shuttle_moves_both_ways_and_waits_at_endpoints() {
        let motion = Motion::Shuttle {
            direction: Vec3d::new(1.0, 0.0, 0.0),
            travel: 128.0,
            speed: 64.0,
            wait: 0.5,
        };
        for (time, x) in [
            (0.0, 0.0),
            (1.0, 64.0),
            (2.25, 128.0),
            (3.5, 64.0),
            (4.75, 0.0),
            (5.0, 0.0),
        ] {
            close(
                motion.transform(Vec3d::new(0.0, 0.0, 0.0), time, false),
                Vec3d::new(x, 0.0, 0.0),
            );
        }
    }
    #[test]
    fn triggered_or_invalid_motion_is_not_played() {
        let entity = |class: &str, key: &str, value: &str| EntityNode {
            properties: vec![
                ("classname".into(), class.into()),
                (key.into(), value.into()),
            ],
            children: vec![],
        };
        assert!(entity_motion(&entity("func_shuttle", "start_on", "0")).is_none());
        assert!(entity_motion(&entity("func_door", "start_on", "1")).is_none());
        assert!(entity_motion(&entity("func_shuttle", "direction", "0 0 0")).is_none());
    }
}

#[cfg(test)]
mod asset_tests {
    #[test]
    fn animated_assets_move_only_their_moving_parts_without_changing_the_map() {
        for name in ["fan", "windmill", "radar", "gear_machine", "industrial_piston"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../assets/{name}.map"));
            let document = crate::document::Document::open(path, Default::default()).unwrap();
            let before = document.to_map_text();
            let mesh = document.preview_mesh(&Default::default());
            assert!(!mesh.motions.is_empty());
            if name == "industrial_piston" {
                for (time, distance) in [(0.0, 0.0), (1.5, 32.0), (2.5, 16.0), (3.5, 0.0)] {
                    let pose = mesh.at(time);
                    for (range, _) in &mesh.motions {
                        for index in range.clone() {
                            for (before, after) in mesh.triangles[index].positions.into_iter().zip(pose[index].positions) {
                                let expected = geometry_math::vec::Vec3d::new(distance, 0.0, 0.0);
                                assert!(geometry_math::vec::length(after - before - expected) < 1e-9);
                            }
                        }
                    }
                }
            }
            let frame = mesh.at(1.0);
            let mut changed = 0;
            let mut unchanged = 0;
            for (index, (base, animated)) in mesh.triangles.iter().zip(&frame).enumerate() {
                if mesh.motions.iter().any(|(range, _)| range.contains(&index)) {
                    if base.positions != animated.positions {
                        changed += 1;
                    }
                } else {
                    assert_eq!(base.positions, animated.positions);
                    unchanged += 1;
                }
                assert_eq!(base.uv, animated.uv);
                assert_eq!(base.corner_colors, animated.corner_colors);
            }
            assert!(changed > 0 && unchanged > 0);
            assert_eq!(before, document.to_map_text());
            for (base, reset) in mesh.triangles.iter().zip(mesh.at(0.0)) {
                for (a, b) in base.positions.into_iter().zip(reset.positions) {
                    assert!(geometry_math::vec::length(a - b) < 1e-9);
                }
            }
        }
    }
}
