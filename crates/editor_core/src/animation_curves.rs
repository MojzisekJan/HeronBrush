//! Versioned independent scalar tracks. Times are seconds and tangents are value/second.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CHANNELS: [&str; 6] = ["X", "Y", "Z", "RX", "RY", "RZ"];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    #[default]
    Linear,
    Hold,
    Bezier,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub id: u64,
    pub time: f64,
    pub value: f64,
    #[serde(default)]
    pub interpolation: Interpolation,
    #[serde(default)]
    pub incoming: f64,
    #[serde(default)]
    pub outgoing: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub version: u32,
    pub duration: f64,
    pub fps: u32,
    pub tracks: [Vec<Key>; 6],
}
impl Clip {
    pub fn from_legacy(track: &crate::keyframe_animation::Track) -> Self {
        let mut id = 0;
        Self {
            version: 1,
            duration: track.keys.last().unwrap().time,
            fps: 30,
            tracks: std::array::from_fn(|channel| {
                track
                    .keys
                    .iter()
                    .map(|key| {
                        id += 1;
                        Key {
                            id,
                            time: key.time,
                            value: if channel < 3 {
                                key.position.v[channel]
                            } else {
                                key.rotation.v[channel - 3]
                            },
                            interpolation: Interpolation::Linear,
                            incoming: 0.0,
                            outgoing: 0.0,
                        }
                    })
                    .collect()
            }),
        }
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 1_000_000 {
            return Err(crate::i18n::translation_key("Animation data is too large.").into());
        }
        let clip: Self =
            serde_json::from_str(text).map_err(|e| format!("Invalid animation curves: {e}"))?;
        clip.validate()?;
        Ok(clip)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !self.duration.is_finite()
            || !(0.001..=100000.0).contains(&self.duration)
            || !(1..=120).contains(&self.fps)
        {
            return Err(crate::i18n::translation_key(
                "Invalid animation version, duration or FPS.",
            )
            .into());
        }
        let mut ids = BTreeSet::new();
        for track in &self.tracks {
            if track.len() > 256 {
                return Err(
                    crate::i18n::translation_key("A track can contain at most 256 keys.").into(),
                );
            }
            for (index, key) in track.iter().enumerate() {
                if key.id == 0
                    || !ids.insert(key.id)
                    || [key.time, key.value, key.incoming, key.outgoing]
                        .iter()
                        .any(|v| !v.is_finite())
                    || key.time < 0.0
                    || key.time > self.duration
                    || key.value.abs() > 100000.0
                    || key.incoming.abs() > 1e9
                    || key.outgoing.abs() > 1e9
                    || (index > 0 && key.time - track[index - 1].time < 1e-6)
                {
                    return Err(crate::i18n::translation_key("Keys need unique IDs, increasing times and finite values inside the animation range.").into());
                }
            }
        }
        Ok(())
    }
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("validated finite animation")
    }
    pub fn sample(&self, channel: usize, time: f64) -> f64 {
        let keys = &self.tracks[channel];
        let Some(first) = keys.first() else {
            return 0.0;
        };
        if time <= first.time {
            return first.value;
        }
        let index = keys
            .partition_point(|key| key.time <= time)
            .saturating_sub(1);
        let a = &keys[index];
        let Some(b) = keys.get(index + 1) else {
            return a.value;
        };
        let dt = b.time - a.time;
        let t = ((time - a.time) / dt).clamp(0.0, 1.0);
        match a.interpolation {
            Interpolation::Linear => a.value + (b.value - a.value) * t,
            Interpolation::Hold => a.value,
            // Cubic Bezier with control times at one and two thirds of the segment.
            // Stored slopes are independent of the segment duration.
            Interpolation::Bezier => {
                let u = 1.0 - t;
                u * u * u * a.value
                    + 3.0 * u * u * t * (a.value + a.outgoing * dt / 3.0)
                    + 3.0 * u * t * t * (b.value - b.incoming * dt / 3.0)
                    + t * t * t * b.value
            }
        }
    }
    pub fn key(&self, id: u64) -> Option<(usize, &Key)> {
        self.tracks
            .iter()
            .enumerate()
            .find_map(|(c, t)| t.iter().find(|k| k.id == id).map(|k| (c, k)))
    }
    pub fn key_mut(&mut self, id: u64) -> Option<&mut Key> {
        self.tracks.iter_mut().flatten().find(|k| k.id == id)
    }
    pub fn insert(&mut self, channel: usize, time: f64, value: f64) -> Result<u64, String> {
        if channel >= 6
            || !time.is_finite()
            || !value.is_finite()
            || time < 0.0
            || time > self.duration
            || value.abs() > 100000.0
        {
            return Err(crate::i18n::translation_key("Key is outside the animation range.").into());
        }
        if let Some(key) = self.tracks[channel]
            .iter_mut()
            .find(|k| (k.time - time).abs() < 1e-6)
        {
            key.value = value;
            return Ok(key.id);
        }
        if self.tracks[channel].len() >= 256 {
            return Err(
                crate::i18n::translation_key("A track can contain at most 256 keys.").into(),
            );
        }
        let id = self
            .tracks
            .iter()
            .flatten()
            .map(|k| k.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(crate::i18n::translation_key("Key ID overflow."))?;
        self.tracks[channel].push(Key {
            id,
            time,
            value,
            interpolation: Interpolation::Linear,
            incoming: 0.0,
            outgoing: 0.0,
        });
        self.tracks[channel].sort_by(|a, b| a.time.total_cmp(&b.time));
        Ok(id)
    }
    /// One position key always records all three coordinates, atomically.
    pub fn insert_position(
        &mut self,
        time: f64,
        position: [f64; 3],
    ) -> Result<BTreeSet<u64>, String> {
        let mut draft = self.clone();
        let mut ids = BTreeSet::new();
        for (channel, value) in position.into_iter().enumerate() {
            if draft.tracks[channel].is_empty() && time > 0.0 {
                draft.insert(channel, 0.0, 0.0)?;
            }
            ids.insert(draft.insert(channel, time, value)?);
        }
        draft.validate()?;
        *self = draft;
        Ok(ids)
    }
    /// Atomic movement: refuse collisions rather than deleting an unselected key.
    pub fn move_keys(&mut self, ids: &BTreeSet<u64>, dt: f64, dv: f64) -> Result<(), String> {
        let mut next = self.clone();
        for key in next
            .tracks
            .iter_mut()
            .flatten()
            .filter(|k| ids.contains(&k.id))
        {
            key.time += dt;
            key.value += dv;
        }
        for track in &mut next.tracks {
            track.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    /// Retime selected keys without changing their order or normalized curve shape.
    pub fn retime(
        &mut self,
        ids: &BTreeSet<u64>,
        scale: f64,
        pivot: f64,
        offset: f64,
        extend: bool,
    ) -> Result<(), String> {
        if ids.is_empty() || ids.iter().any(|id| self.key(*id).is_none()) {
            return Err(crate::i18n::translation_key("Select existing keys to retime.").into());
        }
        if !scale.is_finite() || scale <= 0.0 || !pivot.is_finite() || !offset.is_finite() {
            return Err(crate::i18n::translation_key(
                "Time scale must be positive and all values finite.",
            )
            .into());
        }
        let mut next = self.clone();
        for (old, track) in self.tracks.iter().zip(&mut next.tracks) {
            for key in track.iter_mut().filter(|k| ids.contains(&k.id)) {
                key.time = pivot + (key.time - pivot) * scale + offset;
            }
            for i in 0..track.len().saturating_sub(1) {
                let span = track[i + 1].time - track[i].time;
                if span < 1e-6 {
                    return Err(crate::i18n::translation_key(
                        "Retiming would cross or overlap another key.",
                    )
                    .into());
                }
                let ratio = (old[i + 1].time - old[i].time) / span;
                track[i].outgoing = old[i].outgoing * ratio;
                track[i + 1].incoming = old[i + 1].incoming * ratio;
            }
        }
        if extend {
            next.duration = next
                .tracks
                .iter()
                .flatten()
                .map(|k| k.time)
                .fold(next.duration, f64::max);
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn delete(&mut self, ids: &BTreeSet<u64>) {
        for track in &mut self.tracks {
            track.retain(|k| !ids.contains(&k.id));
        }
    }
    pub fn copy(&self, ids: &BTreeSet<u64>) -> Vec<(usize, Key)> {
        let mut keys: Vec<_> = self
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(c, t)| {
                t.iter()
                    .filter(|k| ids.contains(&k.id))
                    .cloned()
                    .map(move |k| (c, k))
            })
            .collect();
        let first = keys
            .iter()
            .map(|(_, k)| k.time)
            .fold(f64::INFINITY, f64::min);
        for (_, key) in &mut keys {
            key.time -= first;
        }
        keys
    }
    pub fn paste(&mut self, keys: &[(usize, Key)], time: f64) -> Result<BTreeSet<u64>, String> {
        let mut next = self.clone();
        let mut ids = BTreeSet::new();
        for (channel, key) in keys {
            let new_time = key.time + time;
            if *channel >= 6
                || next.tracks[*channel]
                    .iter()
                    .any(|k| (k.time - new_time).abs() < 1e-6)
            {
                return Err(crate::i18n::translation_key(
                    "A key already exists at the paste position.",
                )
                .into());
            }
            let id = next.insert(*channel, new_time, key.value)?;
            let pasted = next.key_mut(id).unwrap();
            pasted.interpolation = key.interpolation;
            pasted.incoming = key.incoming;
            pasted.outgoing = key.outgoing;
            ids.insert(id);
        }
        next.validate()?;
        *self = next;
        Ok(ids)
    }
    /// Compatibility endpoint samples for older editor versions; curves remain authoritative.
    pub fn legacy_text(&self) -> String {
        [0.0, self.duration]
            .iter()
            .map(|t| {
                let mut values = vec![t.to_string()];
                values.extend((0..6).map(|c| self.sample(c, *t).to_string()));
                values.join(" ")
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn clip() -> Clip {
        Clip::from_legacy(
            &crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 2 10 20 0 0 0 360", "once")
                .unwrap(),
        )
    }
    #[test]
    fn retiming_preserves_bezier_shape_and_rejects_crossings_atomically() {
        let mut c = clip();
        c.tracks[0][0].interpolation = Interpolation::Bezier;
        c.tracks[0][0].outgoing = 12.0;
        c.tracks[0][1].incoming = -3.0;
        let original = c.clone();
        let ids = c.tracks.iter().flatten().map(|k| k.id).collect();
        c.retime(&ids, 2.0, 0.0, 0.5, true).unwrap();
        assert_eq!(c.duration, 4.5);
        for i in 0..=100 {
            let t = i as f64 / 50.0;
            assert!((original.sample(0, t) - c.sample(0, t * 2.0 + 0.5)).abs() < 1e-9);
        }
        assert_eq!(Clip::parse(&c.json()).unwrap(), c);
        let before = c.clone();
        let first = BTreeSet::from([c.tracks[0][0].id]);
        for (scale, offset) in [
            (1.0, 4.0),
            (1.0, 5.0),
            (1.0, -1.0),
            (0.0, 0.0),
            (f64::NAN, 0.0),
        ] {
            assert!(c.retime(&first, scale, 0.0, offset, false).is_err());
            assert_eq!(c, before);
        }
        assert!(c
            .retime(&BTreeSet::from([999999]), 1.0, 0.0, 0.0, false)
            .is_err());
        c.retime(&first, 1.0, 0.0, 0.25, false).unwrap();
        // Even an unselected neighbour's incoming tangent follows its new segment length.
        for i in 0..=100 {
            let u = i as f64 / 100.0;
            assert!((before.sample(0, 0.5 + 4.0 * u) - c.sample(0, 0.75 + 3.75 * u)).abs() < 1e-9);
        }
    }
    #[test]
    fn independent_tracks_and_interpolation() {
        let mut c = clip();
        c.insert(0, 1.0, 20.0).unwrap();
        assert_eq!(c.sample(0, 1.0), 20.0);
        assert_eq!(c.sample(1, 1.0), 10.0);
        c.tracks[1][0].interpolation = Interpolation::Hold;
        assert_eq!(c.sample(1, 1.0), 0.0);
        assert_eq!(c.sample(1, 2.0), 20.0);
        c.tracks[5][0].interpolation = Interpolation::Bezier;
        assert!((c.sample(5, 0.5) - 56.25).abs() < 1e-8);
        assert_eq!(Clip::parse(&c.json()).unwrap(), c);
        let track =
            crate::keyframe_animation::Track::from_properties("", "loop", Some(&c.json())).unwrap();
        assert!((track.pose(0.5).1.z() - 56.25).abs() < 1e-8);
        let origin = geometry_math::vec::Vec3d::new(0.0, 0.0, 0.0);
        let end = track.transform_clamped(origin, origin, 2.0, false);
        assert!((end.x() - 10.0).abs() < 1e-8 && (end.y() - 20.0).abs() < 1e-8);
    }
    #[test]
    fn atomic_move_paste_and_validation() {
        let mut c = clip();
        let id = c.insert(0, 1.0, 20.0).unwrap();
        let selected = BTreeSet::from([id]);
        let old = c.clone();
        assert!(c.move_keys(&selected, 1.0, 0.0).is_err());
        assert_eq!(c, old);
        c.move_keys(&selected, 0.5, 5.0).unwrap();
        assert_eq!(c.key(id).unwrap().1.time, 1.5);
        let copied = c.copy(&selected);
        let pasted = c.paste(&copied, 0.5).unwrap();
        assert_eq!(pasted.len(), 1);
        let old = c.clone();
        assert!(c.paste(&copied, 0.5).is_err());
        assert_eq!(c, old);
        c.delete(&selected);
        assert!(c.key(id).is_none());
        c.tracks[0][0].value = f64::NAN;
        assert!(c.validate().is_err());
    }
}
