//! Pose authoring shared by 3D handles and agents. Rest geometry is never transformed.
use crate::{animation_curves::Clip, document::Document};
use geometry_math::vec::Vec3d;
use map_model::node::NodeId;

#[derive(Clone)]
pub struct Pose {
    pub entity: NodeId,
    pub clip: Clip,
    pub pivot: Vec3d,
    pub time: f64,
}
impl Pose {
    pub fn selected(d: &Document, time: f64) -> Result<Self, String> {
        let selected = d.selection_ids();
        let entity = map_model::node::entities_with_ids(d.tree_world())
            .into_iter()
            .find(|(id, e)| {
                e.is_brush_entity()
                    && !selected.is_empty()
                    && selected.iter().all(|s| {
                        *s == *id || map_model::node::contents_of(d.tree_world(), *id).contains(s)
                    })
            })
            .map(|(id, _)| id)
            .ok_or(crate::i18n::translation_key("Select brushes from one animated entity."))?;
        Self::read(d, entity, time)
    }
    pub fn read(d: &Document, entity: NodeId, time: f64) -> Result<Self, String> {
        let (_, e) = map_model::node::entities_with_ids(d.tree_world())
            .into_iter()
            .find(|(id, e)| *id == entity && e.is_brush_entity())
            .ok_or(crate::i18n::translation_key("Expected a brush entity"))?;
        let mode = e.property("animation_mode").unwrap_or("loop").to_owned();
        let clip = if let Some(text) = e.property("animation_curves").filter(|s| !s.is_empty()) {
            Clip::parse(text)?
        } else {
            Clip::from_legacy(&crate::keyframe_animation::Track::parse(
                e.property("animation_keys")
                    .ok_or(crate::i18n::translation_key("Add an animation key first."))?,
                &mode,
            )?)
        };
        if !time.is_finite() || time < 0.0 || time > clip.duration {
            return Err(crate::i18n::translation_key("Frame outside animation duration").into());
        }
        let values = e
            .property("animation_pivot")
            .unwrap_or("0 0 0")
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| crate::i18n::translation_key("Invalid pivot"))?;
        if values.len() != 3 || values.iter().any(|v| !v.is_finite()) {
            return Err(crate::i18n::translation_key("Invalid pivot").into());
        }
        Ok(Self {
            entity,
            clip,
            pivot: Vec3d::new(values[0], values[1], values[2]),
            time,
        })
    }
    pub fn centre(&self) -> Vec3d {
        self.pivot
            + Vec3d::new(
                self.clip.sample(0, self.time),
                self.clip.sample(1, self.time),
                self.clip.sample(2, self.time),
            )
    }
    pub fn keyed(&self, channel: usize, delta: f64) -> Result<Clip, String> {
        if channel >= 6 || !delta.is_finite() {
            return Err(crate::i18n::translation_key("Invalid pose channel or offset").into());
        }
        let mut clip = self.clip.clone();
        if channel<3 {
            let mut position=std::array::from_fn(|axis|self.clip.sample(axis,self.time));
            position[channel]+=delta;
            clip.insert_position(self.time,position)?;
            return Ok(clip);
        }
        // Keep the rest pose before the first authored key on an empty track.
        if clip.tracks[channel].is_empty() && self.time > 0.0 {
            clip.insert(channel, 0.0, 0.0)?;
        }
        clip.insert(
            channel,
            self.time,
            self.clip.sample(channel, self.time) + delta,
        )?;
        clip.validate()?;
        Ok(clip)
    }
    pub fn moved_pivot(&self, channel: usize, delta: f64) -> Result<Vec3d, String> {
        if channel >= 3 {
            return Err(crate::i18n::translation_key("Invalid pivot axis").into());
        }
        let mut pivot = self.pivot;
        pivot[channel] += delta;
        Self::validate_pivot(pivot)?;
        Ok(pivot)
    }
    pub fn validate_pivot(pivot: Vec3d) -> Result<(), String> {
        if pivot.v.iter().any(|v| !v.is_finite() || v.abs() > 100000.0) {
            return Err(crate::i18n::translation_key("Pivot must contain finite coordinates between -100000 and 100000.").into());
        }
        Ok(())
    }
    pub fn write_pivot(&self, d: &mut Document, pivot: Vec3d) {
        d.set_entity_property(
            self.entity,
            "animation_pivot",
            &format!("{} {} {}", pivot.x(), pivot.y(), pivot.z()),
        );
    }
    pub fn write(&self, d: &mut Document, clip: &Clip) {
        d.set_entity_property(self.entity, "animation_curves", &clip.json());
        d.set_entity_property(self.entity, "animation_keys", &clip.legacy_text());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_position_keys_make_a_sharp_l_and_pingpong_returns_the_same_way() {
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map");
        let mut d=Document::open(path,Default::default()).unwrap();d.select_all();
        let id=d.create_brush_entity("func_animate").unwrap();
        let mut clip=Clip{version:1,duration:2.0,fps:30,tracks:std::array::from_fn(|_|Vec::new())};
        clip.insert_position(0.0,[0.0;3]).unwrap();
        d.set_entity_property(id,"animation_curves",&clip.json());
        let original=d.to_map_text();let bounds=d.selection_bounds();
        for (time,axis,delta) in [(1.0,0,128.0),(2.0,1,128.0)] {
            let pose=Pose::read(&d,id,time).unwrap();
            assert!(d.begin_entity_transaction(id));
            pose.write(&mut d,&pose.keyed(axis,delta).unwrap());d.commit_entity_transaction();
        }
        let pose=Pose::read(&d,id,2.0).unwrap();
        for axis in 0..3 {assert_eq!(pose.clip.tracks[axis].iter().map(|k|k.time).collect::<Vec<_>>(),vec![0.0,1.0,2.0]);}
        let track=crate::keyframe_animation::Track::from_properties("","pingpong",Some(&pose.clip.json())).unwrap();
        for (t,expected) in [(0.5,[64.0,0.0,0.0]),(1.0,[128.0,0.0,0.0]),(1.5,[128.0,64.0,0.0]),(2.5,[128.0,64.0,0.0]),(3.5,[64.0,0.0,0.0])] {
            assert_eq!(track.pose(t).0,Vec3d::new(expected[0],expected[1],expected[2]));
        }
        assert_eq!(d.selection_bounds(),bounds);d.undo();d.undo();assert_eq!(d.to_map_text(),original);
    }
    #[test]
    fn pose_keys_preserve_geometry_and_form_one_undo() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map");
        let mut d = Document::open(path, Default::default()).unwrap();
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        let clip = Clip {
            version: 1,
            duration: 4.0,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        };
        d.set_entity_property(id, "animation_curves", &clip.json());
        d.set_entity_property(id, "animation_pivot", "16 8 0");
        let pose = Pose::selected(&d, 1.0).unwrap();
        let original = d.to_map_text();
        let bounds = d.selection_bounds();
        assert!(d.begin_entity_transaction(id));
        pose.write(&mut d, &pose.keyed(0, 32.0).unwrap());
        pose.write(&mut d, &pose.keyed(0, 64.0).unwrap());
        d.commit_entity_transaction();
        assert_eq!(d.selection_bounds(), bounds);
        let changed = Pose::read(&d, id, 1.0).unwrap();
        assert_eq!(changed.clip.tracks[0].len(), 2);
        assert_eq!(changed.clip.sample(0, 0.0), 0.0);
        assert_eq!(changed.clip.sample(0, 1.0), 64.0);
        assert!(changed.clip.tracks[1..3].iter().all(|t|t.len()==2));
        assert!(changed.clip.tracks[3..].iter().all(Vec::is_empty));
        assert_eq!(changed.centre(), Vec3d::new(80.0, 8.0, 0.0));
        d.undo();
        assert_eq!(d.to_map_text(), original);
        assert!(d.begin_entity_transaction(id));
        pose.write(&mut d, &pose.keyed(5, 90.0).unwrap());
        d.cancel_transaction();
        assert_eq!(d.to_map_text(), original);
        assert!(d.begin_entity_transaction(id));
        pose.write_pivot(&mut d, pose.moved_pivot(0, 8.0).unwrap());
        pose.write_pivot(&mut d, pose.moved_pivot(0, 16.0).unwrap());
        d.commit_entity_transaction();
        let after = Pose::read(&d, id, 1.0).unwrap();
        assert_eq!(after.pivot, Vec3d::new(32.0, 8.0, 0.0));
        assert_eq!(after.clip, pose.clip);
        assert_eq!(d.selection_bounds(), bounds);
        d.undo();
        assert_eq!(d.to_map_text(), original);
        assert!(pose.moved_pivot(0, f64::INFINITY).is_err());
        assert!(pose.moved_pivot(0, 100001.0).is_err());
        assert!(pose.moved_pivot(3, 1.0).is_err());
        assert!(pose.keyed(0, f64::NAN).is_err());
        assert!(Pose::read(&d, id, 5.0).is_err());
    }
}
