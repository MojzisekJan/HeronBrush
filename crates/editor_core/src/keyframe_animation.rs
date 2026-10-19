//! Keyframe positions are offsets from authored geometry; rotations are XYZ degrees.
use geometry_math::vec::Vec3d;
#[derive(Clone, Debug)]
pub struct Key {
    pub time: f64,
    pub position: Vec3d,
    pub rotation: Vec3d,
}
#[derive(Clone, Debug)]
pub struct Track {
    pub keys: Vec<Key>,
    pub mode: String,
    pub curves: Option<crate::animation_curves::Clip>,
}
impl Track {
    pub fn parse(text: &str, mode: &str) -> Result<Self, String> {
        if !["once", "loop", "pingpong"].contains(&mode) {
            return Err("Use once, loop or pingpong.".into());
        }
        let mut keys = Vec::new();
        for row in text.split(';').map(str::trim).filter(|row| !row.is_empty()) {
            let values: Vec<f64> = row
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()
                .map_err(|_| "Each key needs seven numbers: time, X Y Z, rotation X Y Z.")?;
            if values.len() != 7 || values.iter().any(|v| !v.is_finite()) {
                return Err("Each key needs seven finite numbers.".into());
            }
            if values[0] < 0.0 || keys.last().is_some_and(|key: &Key| key.time >= values[0]) {
                return Err("Key times must increase, starting at zero.".into());
            }
            keys.push(Key {
                time: values[0],
                position: Vec3d::new(values[1], values[2], values[3]),
                rotation: Vec3d::new(values[4], values[5], values[6]),
            });
        }
        if keys.len() < 2 || keys.len() > 256 || keys[0].time != 0.0 {
            return Err("Use 2 to 256 keys, with the first at time zero.".into());
        }
        Ok(Self {
            keys,
            mode: mode.into(),
            curves: None,
        })
    }
    pub fn from_properties(text: &str, mode: &str, curves: Option<&str>) -> Result<Self, String> {
        if let Some(curves) = curves.filter(|s| !s.is_empty()) {
            let clip = crate::animation_curves::Clip::parse(curves)?;
            let mut track = Self::parse(&clip.legacy_text(), mode)?;
            track.curves = Some(clip);
            Ok(track)
        } else {
            Self::parse(text, mode)
        }
    }
    pub fn pose(&self, time: f64) -> (Vec3d, Vec3d) {
        self.pose_time(time, true)
    }
    fn pose_time(&self, time: f64, wrap: bool) -> (Vec3d, Vec3d) {
        let duration = self.keys.last().unwrap().time;
        let time = time.max(0.0);
        let t = match if wrap { self.mode.as_str() } else { "once" } {
            "loop" => time.rem_euclid(duration),
            "pingpong" => {
                let phase = time.rem_euclid(duration * 2.0);
                if phase > duration {
                    duration * 2.0 - phase
                } else {
                    phase
                }
            }
            _ => time.min(duration),
        };
        if let Some(curves) = &self.curves {
            return (
                Vec3d::new(
                    curves.sample(0, t),
                    curves.sample(1, t),
                    curves.sample(2, t),
                ),
                Vec3d::new(
                    curves.sample(3, t),
                    curves.sample(4, t),
                    curves.sample(5, t),
                ),
            );
        }
        let i = self
            .keys
            .partition_point(|key| key.time <= t)
            .saturating_sub(1)
            .min(self.keys.len() - 2);
        let a = &self.keys[i];
        let b = &self.keys[i + 1];
        let f = (t - a.time) / (b.time - a.time);
        (
            a.position + (b.position - a.position) * f,
            a.rotation + (b.rotation - a.rotation) * f,
        )
    }
    pub fn transform(&self, p: Vec3d, pivot: Vec3d, time: f64, normal: bool) -> Vec3d {
        self.transform_time(p, pivot, time, normal, true)
    }
    pub fn transform_clamped(&self, p: Vec3d, pivot: Vec3d, time: f64, normal: bool) -> Vec3d {
        self.transform_time(p, pivot, time, normal, false)
    }
    fn transform_time(&self, p: Vec3d, pivot: Vec3d, time: f64, normal: bool, wrap: bool) -> Vec3d {
        let (offset, r) = self.pose_time(time, wrap);
        let mut v = if normal { p } else { p - pivot };
        for axis in 0..3 {
            let (s, c) = r.v[axis].to_radians().sin_cos();
            v = match axis {
                0 => Vec3d::new(v.x(), c * v.y() - s * v.z(), s * v.y() + c * v.z()),
                1 => Vec3d::new(c * v.x() + s * v.z(), v.y(), -s * v.x() + c * v.z()),
                _ => Vec3d::new(c * v.x() - s * v.y(), s * v.x() + c * v.y(), v.z()),
            };
        }
        if normal { v } else { v + pivot + offset }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translation_rotation_pivot_and_normals() {
        let t = Track::parse("0 0 0 0 0 0 0; 2 10 0 0 0 0 180", "once").unwrap();
        let p = t.transform(
            Vec3d::new(3.0, 0.0, 0.0),
            Vec3d::new(2.0, 0.0, 0.0),
            1.0,
            false,
        );
        assert!((p.x() - 7.0).abs() < 1e-8 && (p.y() - 1.0).abs() < 1e-8);
        let n = t.transform(
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(2.0, 0.0, 0.0),
            1.0,
            true,
        );
        assert!(n.x().abs() < 1e-8 && (n.y() - 1.0).abs() < 1e-8);
    }
    #[test]
    fn playback_and_invalid_keys() {
        let text = "0 0 0 0 0 0 0; 2 10 0 0 0 0 360";
        assert_eq!(Track::parse(text, "once").unwrap().pose(5.0).0.x(), 10.0);
        assert_eq!(Track::parse(text, "loop").unwrap().pose(3.0).0.x(), 5.0);
        assert_eq!(Track::parse(text, "pingpong").unwrap().pose(3.0).0.x(), 5.0);
        for text in [
            "",
            "0 0 0 0 0 0 0",
            "0 0 0 0 0 0 0;0 1 0 0 0 0 0",
            "0 0 0 0 0 0 0;1 NaN 0 0 0 0 0",
        ] {
            assert!(Track::parse(text, "loop").is_err());
        }
    }
}
