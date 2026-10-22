//! Spatial translation path of the selected animation, shared by flat and 3D views.
use crate::{animation_curves::Interpolation, animation_pose::Pose};
use geometry_math::vec::{Vec2d, Vec3d};

pub struct Path {
    pub keys: Vec<(f64, Vec3d)>,
    segments: Vec<(Vec3d, Vec3d, bool)>,
}
#[derive(Default)]
pub struct Cache {
    key: Option<(u64, map_model::node::NodeId)>,
    path: std::rc::Rc<Option<Path>>,
}
impl Cache {
    pub fn get(&mut self, revision: u64, pose: &Pose) -> std::rc::Rc<Option<Path>> {
        let key = (revision, pose.entity);
        if self.key != Some(key) {
            self.path = std::rc::Rc::new(Path::new(pose));
            self.key = Some(key);
        }
        self.path.clone()
    }
}
fn position(pose: &Pose, time: f64) -> Vec3d {
    pose.pivot
        + Vec3d::new(
            pose.clip.sample(0, time),
            pose.clip.sample(1, time),
            pose.clip.sample(2, time),
        )
}
impl Path {
    pub fn new(pose: &Pose) -> Option<Self> {
        let mut times: Vec<_> = pose.clip.tracks[..3]
            .iter()
            .flatten()
            .map(|k| k.time)
            .collect();
        times.sort_by(f64::total_cmp);
        times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        if times.len() < 2 {
            return None;
        }
        let keys: Vec<_> = times.iter().map(|t| (*t, position(pose, *t))).collect();
        let mut segments = Vec::new();
        for pair in times.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let end = position(pose, b);
            let mut left = end;
            // A hold is a teleport at the next key, not a diagonal travel segment.
            for axis in 0..3 {
                let track = &pose.clip.tracks[axis];
                if let Some(k) = track.iter().rev().find(|k| k.time < b - 1e-7) {
                    if k.interpolation == Interpolation::Hold {
                        left[axis] = pose.pivot[axis] + k.value;
                    }
                }
            }
            let steps = if pose.clip.tracks[..3].iter().any(|track| {
                track
                    .iter()
                    .rev()
                    .find(|k| k.time <= a)
                    .is_some_and(|k| k.interpolation == Interpolation::Bezier)
            }) {
                16
            } else {
                1
            };
            let mut previous = position(pose, a);
            for n in 1..=steps {
                let next = if n == steps {
                    left
                } else {
                    position(pose, a + (b - a) * n as f64 / steps as f64)
                };
                segments.push((previous, next, false));
                previous = next;
            }
            if geometry_math::vec::length(left - end) > 1e-8 {
                segments.push((left, end, true));
            }
        }
        let first = keys[0].1;
        if segments.iter().all(|(a, b, _)| {
            geometry_math::vec::length(*a - first) < 1e-8
                && geometry_math::vec::length(*b - first) < 1e-8
        }) {
            return None;
        }
        Some(Self { keys, segments })
    }
    pub fn pick(
        &self,
        cursor: Vec2d,
        current: f64,
        project: impl Fn(Vec3d) -> Option<Vec2d>,
    ) -> Option<f64> {
        self.keys
            .iter()
            .filter_map(|(t, p)| {
                let p = project(*p)?;
                let d = (p.x() - cursor.x()).powi(2) + (p.y() - cursor.y()).powi(2);
                (d <= 64.0).then_some((*t, d))
            })
            .min_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then_with(|| (a.0 - current).abs().total_cmp(&(b.0 - current).abs()))
            })
            .map(|(t, _)| t)
    }
    pub fn draw(
        &self,
        cr: &mut dyn crate::canvas::Canvas,
        time: f64,
        fps: u32,
        project: impl Fn(Vec3d) -> Option<Vec2d>,
    ) {
        cr.save();
        for jump in [false, true] {
            cr.new_path();
            cr.set_dash(if jump { &[4.0, 4.0] } else { &[] }, 0.0);
            for (a, b, segment_jump) in &self.segments {
                if *segment_jump != jump {
                    continue;
                }
                let (Some(a), Some(b)) = (project(*a), project(*b)) else {
                    continue;
                };
                cr.move_to(a.x(), a.y());
                cr.line_to(b.x(), b.y());
            }
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.85);
            cr.set_line_width(4.0);
            cr.stroke_preserve();
            cr.set_source_rgb(0.2, 0.8, 0.9);
            cr.set_line_width(1.5);
            cr.stroke();
        }
        cr.set_dash(&[], 0.0);
        let clip = cr.clip_extents();
        for (t, p) in &self.keys {
            let Some(p) = project(*p) else {
                continue;
            };
            if clip.is_some_and(|(x0, y0, x1, y1)| {
                p.x() < x0 - 32.0 || p.x() > x1 + 16.0 || p.y() < y0 - 16.0 || p.y() > y1 + 24.0
            }) {
                continue;
            }
            cr.new_path();
            let selected = (*t - time).abs() <= 0.5 / fps.max(1) as f64 + 1e-7;
            cr.arc(
                p.x(),
                p.y(),
                if selected { 6.0 } else { 4.5 },
                0.0,
                std::f64::consts::TAU,
            );
            if selected {
                cr.set_source_rgb(1.0, 0.7, 0.15);
            } else {
                cr.set_source_rgb(0.2, 0.8, 0.9);
            }
            cr.fill_preserve();
            cr.set_source_rgb(0.05, 0.05, 0.05);
            cr.set_line_width(1.5);
            cr.stroke();
            cr.set_source_rgb(0.85, 0.95, 1.0);
            cr.move_to(p.x() + 9.0, p.y() - 7.0);
            cr.show_text(&format!("{:.0}", t * fps.max(1) as f64), 11.0);
        }
        cr.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pose() -> Pose {
        let mut d = crate::document::Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &geometry_math::bbox::Bbox3d::from_scalars(0.0, 16.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 128 64 32 0 0 90");
        Pose::selected(&d, 1.0).unwrap()
    }
    #[test]
    fn path_cache_is_shared_across_frames_and_invalidated_by_edits() {
        let mut pose = pose();
        let mut cache = Cache::default();
        let first = cache.get(1, &pose);
        pose.time = 2.0;
        assert!(std::rc::Rc::ptr_eq(&first, &cache.get(1, &pose)));
        pose.pivot = Vec3d::new(16.0, 0.0, 0.0);
        let changed = cache.get(2, &pose);
        assert!(!std::rc::Rc::ptr_eq(&first, &changed));
        assert_eq!(changed.as_ref().as_ref().unwrap().keys[0].1, pose.pivot);
    }
    #[test]
    fn spatial_path_uses_position_keys_and_shows_hold_jumps() {
        let mut pose = pose();
        pose.clip.insert(1, 1.0, 16.0).unwrap();
        pose.clip.insert(5, 0.75, 45.0).unwrap();
        let path = Path::new(&pose).unwrap();
        assert_eq!(
            path.keys.iter().map(|k| k.0).collect::<Vec<_>>(),
            vec![0.0, 1.0, 2.0]
        );
        assert_eq!(path.keys[1].1, pose.pivot + Vec3d::new(64.0, 16.0, 16.0));
        assert_eq!(
            path.pick(Vec2d::new(64.0, 16.0), 0.0, |p| Some(Vec2d::new(
                p.x(),
                p.y()
            ))),
            Some(1.0)
        );
        assert!(path
            .pick(Vec2d::new(90.0, 90.0), 0.0, |p| Some(Vec2d::new(
                p.x(),
                p.y()
            )))
            .is_none());
        pose.clip.tracks[0][0].interpolation = Interpolation::Hold;
        let hold = Path::new(&pose).unwrap();
        assert_eq!(hold.segments.iter().filter(|s| s.2).count(), 1);
        assert!(hold
            .segments
            .iter()
            .filter(|s| !s.2)
            .all(|(a, b, _)| a.x() == b.x()));
    }
    #[test]
    fn bezier_path_includes_excursions_with_coincident_endpoints() {
        let mut pose = pose();
        for axis in 0..3 {
            for key in &mut pose.clip.tracks[axis] {
                key.value = 0.0;
            }
        }
        assert!(Path::new(&pose).is_none());
        pose.clip.tracks[0][0].interpolation = Interpolation::Bezier;
        pose.clip.tracks[0][0].outgoing = 128.0;
        let path = Path::new(&pose).unwrap();
        assert!(path.segments.iter().any(|(_, b, _)| b.x() > 1.0));
        assert_eq!(
            path.pick(Vec2d::zero(), 2.0, |p| Some(Vec2d::new(p.x(), p.y()))),
            Some(2.0)
        );
        use crate::canvas::Canvas;
        let mut cr = crate::canvas::Raster::new(480, 180).unwrap();
        cr.set_source_rgb(0.08, 0.08, 0.08);
        cr.paint();
        path.draw(&mut cr, 1.0, 30, |p| {
            Some(Vec2d::new(p.x() * 4.0 + 30.0, 90.0 - p.y()))
        });
        cr.into_rgba()
            .save_png(&std::env::temp_dir().join("animation-spatial-path.png"))
            .unwrap();
    }
}
