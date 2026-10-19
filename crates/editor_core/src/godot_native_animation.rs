//! Native scalar tracks. Hold discontinuities are a separate constant contribution,
//! keeping Bezier tracks continuous without approximating jumps with tiny segments.
use crate::animation_curves::{Clip, Interpolation};
use scene_export::scene::{GdValue as V, SubResource};
const CHANNELS: [&str; 6] = ["x", "y", "z", "rx", "ry", "rz"];
fn floats(values: impl IntoIterator<Item = f64>) -> V {
    V::Raw(format!(
        "PackedFloat64Array({})",
        values
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
pub fn animation(id: &str, name: &str, clip: &Clip, mode: &str) -> SubResource {
    let mut props = vec![
        ("resource_name".into(), V::Str(name.into())),
        ("length".into(), V::Float(clip.duration)),
        ("step".into(), V::Float(1.0 / clip.fps as f64)),
        (
            "loop_mode".into(),
            V::Int(match mode {
                "loop" => 1,
                "pingpong" => 2,
                _ => 0,
            }),
        ),
    ];
    for (channel, keys) in clip.tracks.iter().enumerate() {
        // time, continuous value, incoming X/Y, outgoing X/Y
        let mut points: Vec<[f64; 6]> = Vec::new();
        let mut steps = vec![(0.0, 0.0)];
        let mut shift = 0.0;
        for (i, key) in keys.iter().enumerate() {
            if i > 0 && keys[i - 1].interpolation == Interpolation::Hold {
                shift += key.value - keys[i - 1].value;
                steps.push((key.time, shift));
            }
            points.push([key.time, key.value - shift, 0., 0., 0., 0.]);
        }
        for i in 0..points.len().saturating_sub(1) {
            let dt = (points[i + 1][0] - points[i][0]) / 3.0;
            let (outgoing, incoming) = match keys[i].interpolation {
                Interpolation::Bezier => (keys[i].outgoing, keys[i + 1].incoming),
                Interpolation::Linear => {
                    let slope = (points[i + 1][1] - points[i][1]) / (dt * 3.0);
                    (slope, slope)
                }
                Interpolation::Hold => (0., 0.),
            };
            points[i][4] = dt;
            points[i][5] = outgoing * dt;
            points[i + 1][2] = -dt;
            points[i + 1][3] = -incoming * dt;
        }
        if points.is_empty() {
            points.push([0.; 6]);
        }
        if points[0][0] > 0.0 {
            points.insert(0, [0., points[0][1], 0., 0., 0., 0.]);
        }
        if points.last().unwrap()[0] < clip.duration {
            points.push([clip.duration, points.last().unwrap()[1], 0., 0., 0., 0.]);
        }
        for (index, kind, property, data) in [
            (
                channel * 2,
                "bezier",
                format!("map_{}", CHANNELS[channel]),
                V::Dict(vec![
                    ("times".into(), floats(points.iter().map(|p| p[0]))),
                    (
                        "points".into(),
                        floats(points.iter().flat_map(|p| p[1..].iter().copied())),
                    ),
                    ("handle_modes".into(), V::PackedInt32(vec![0; points.len()])),
                ]),
            ),
            (
                channel * 2 + 1,
                "value",
                format!("step_{}", CHANNELS[channel]),
                V::Dict(vec![
                    ("times".into(), floats(steps.iter().map(|p| p.0))),
                    ("transitions".into(), floats(steps.iter().map(|_| 1.0))),
                    ("update".into(), V::Int(0)),
                    (
                        "values".into(),
                        V::Array(steps.iter().map(|p| V::Float(p.1)).collect()),
                    ),
                ]),
            ),
        ] {
            let prefix = format!("tracks/{index}");
            props.extend([
                (format!("{prefix}/type"), V::Str(kind.into())),
                (
                    format!("{prefix}/path"),
                    V::NodePath(format!(".:{property}")),
                ),
                (
                    format!("{prefix}/interp"),
                    V::Int(if kind == "value" { 0 } else { 1 }),
                ),
                (format!("{prefix}/loop_wrap"), V::Bool(false)),
                (format!("{prefix}/keys"), data),
            ]);
        }
    }
    SubResource {
        id: id.into(),
        ty: "Animation".into(),
        properties: props,
    }
}
