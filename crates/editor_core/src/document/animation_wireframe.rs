//! Shared frame cache for all orthographic animation previews.
use super::*;
use crate::{
    motion_preview::Motion,
    view_filters::ViewFilters,
};
type Edges = Vec<(Vec3d, Vec3d)>;
#[derive(Default)]
pub struct Frame {
    pub plain: Edges,
    pub selected: Edges,
    pub locked: Edges,
    pub peers: HashMap<u32, Edges>,
    pub ghosts: Vec<(bool, u8, Edges)>,
}
struct Source {
    edges: Edges,
    motion: Option<Motion>,
    selected: bool,
    locked: bool,
    peer: Option<u32>,
}
#[derive(Default)]
pub(super) struct Cache {
    revision: Option<u64>,
    show_export_hidden: bool,
    sources: Vec<Source>,
    key: Option<(u64, u32, u8, u32, bool, bool)>,
    clock: Option<web_time::Instant>,
    frame: Frame,
}
impl Document {
    pub fn animation_preview_time(&self, playing: bool) -> f64 {
        let mut cache = self.animation_wireframe.borrow_mut();
        if playing {
            cache
                .clock
                .get_or_insert_with(web_time::Instant::now)
                .elapsed()
                .as_secs_f64()
        } else {
            if !self.trigger_preview.active() {cache.clock = None;}
            0.0
        }
    }

    pub fn animation_wireframe(&self, filters: &ViewFilters) -> Ref<'_, Frame> {
        let playing = filters.animation_frame.is_none() && filters.preview_motion;
        let elapsed = self.animation_preview_time(playing);
        self.animation_wireframe_at(filters, elapsed)
    }

    fn animation_wireframe_at(&self, filters: &ViewFilters, elapsed: f64) -> Ref<'_, Frame> {
        let playing = filters.animation_frame.is_none() && filters.preview_motion;
        let mut cache = self.animation_wireframe.borrow_mut();
        if cache.revision != Some(self.revision())
            || cache.show_export_hidden != filters.show_export_hidden
        {
            let mut motions = self.preview_motions();
            let hidden = self.brushes_hidden_by_tags();
            cache.sources = node::brushes_by_layer(&self.tree.world)
                .into_iter()
                .filter_map(|(id, brush, layer)| {
                    if !self.node_is_drawn(id, layer) || hidden.contains(&id) {
                        return None;
                    }
                    Some(Source {
                        edges: filters.brush_edges(brush),
                        motion: motions.remove(&id),
                        selected: self.selection.contains(&id),
                        locked: !self.layer_is_editable(layer),
                        peer: self
                            .peer_selection
                            .iter()
                            .find(|(_, ids)| ids.contains(&id))
                            .map(|(p, _)| *p),
                    })
                })
                .collect();
            cache.revision = Some(self.revision());
            cache.show_export_hidden = filters.show_export_hidden;
            cache.key = None;
        }
        let key = (
            if playing {
                elapsed.to_bits()
            } else {
                filters.animation_frame.unwrap_or(0) as u64
            },
            filters.animation_fps.max(1),
            if playing || filters.animation_frame.is_none() {
                0
            } else {
                filters.onion_frames.min(4)
            },
            filters.onion_step.max(1),
            playing,
            filters.animation_frame.is_some(),
        );
        if cache.key != Some(key) {
            let now = if playing {
                elapsed
            } else {
                key.0 as f64 / key.1 as f64
            };
            let mut frame = Frame::default();
            for source in &cache.sources {
                let target = if source.locked {
                    &mut frame.locked
                } else if source.selected {
                    &mut frame.selected
                } else if let Some(peer) = source.peer {
                    frame.peers.entry(peer).or_default()
                } else {
                    &mut frame.plain
                };
                target.extend(source.edges.iter().map(|(a, b)| {
                    let transform = |p| match &source.motion {
                        Some(Motion::Keyframes { pivot, track, .. } | Motion::Playback {pivot, authored:track,..})
                            if !playing && filters.animation_frame.is_some() =>
                        {
                            track.transform_clamped(p, *pivot, now, false)
                        }
                        Some(motion) if playing || filters.animation_frame.is_some() => {
                            motion.transform(p, now, false)
                        }
                        Some(_) => p,
                        None => p,
                    };
                    (transform(*a), transform(*b))
                }));
                if source.selected && !source.locked {
                    if let Some(Motion::Keyframes { pivot, track, .. } | Motion::Playback {pivot, authored:track,..}) = &source.motion {
                        for offset in 1..=key.2 {
                            for past in [true, false] {
                                let time = now
                                    + if past { -1.0 } else { 1.0 } * offset as f64 * key.3 as f64
                                        / key.1 as f64;
                                if time < 0.0 || time > track.keys.last().map_or(0.0, |k| k.time) {
                                    continue;
                                }
                                let edges = source
                                    .edges
                                    .iter()
                                    .map(|(a, b)| {
                                        (
                                            track.transform_clamped(*a, *pivot, time, false),
                                            track.transform_clamped(*b, *pivot, time, false),
                                        )
                                    })
                                    .collect();
                                frame.ghosts.push((past, offset, edges));
                            }
                        }
                    }
                }
            }
            cache.frame = frame;
            cache.key = Some(key);
        }
        drop(cache);
        Ref::map(self.animation_wireframe.borrow(), |cache| &cache.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_preview_loops_and_respects_autoplay_without_a_timeline() {
        let mut d = Document::new(Default::default());
        let brush = map_model::shapes::create_cuboid(
            &d.world_bounds(),
            &Bbox3d::from_scalars(0.0, 16.0),
            "stone",
        )
        .unwrap();
        d.add_brushes(vec![brush]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 64 32 16 0 0 0");
        d.set_entity_property(id, "animation_mode", "loop");
        let rest = d.selected_edges().clone();
        let text = d.to_map_text();
        let filters = ViewFilters {
            preview_motion: true,
            onion_frames: 3,
            ..Default::default()
        };
        for time in [1.0, 3.0, 5.0] {
            let frame = d.animation_wireframe_at(&filters, time);
            for ((a, b), (ra, rb)) in frame.selected.iter().zip(&rest) {
                assert_eq!(*a, *ra + Vec3d::new(32.0, 16.0, 8.0));
                assert_eq!(*b, *rb + Vec3d::new(32.0, 16.0, 8.0));
            }
            assert!(frame.ghosts.is_empty());
        }
        assert_eq!(d.to_map_text(), text);
        d.set_entity_property(id, "start_on", "0");
        assert_eq!(d.animation_wireframe_at(&filters, 1.0).selected, rest);
        let manual = ViewFilters {
            animation_frame: Some(30),
            animation_fps: 30,
            ..filters
        };
        assert_ne!(d.animation_wireframe_at(&manual, 1.0).selected, rest);
        d.animation_preview_time(true);
        assert!(d.animation_wireframe.borrow().clock.is_some());
        d.animation_preview_time(false);
        assert!(d.animation_wireframe.borrow().clock.is_none());
    }

    #[test]
    fn wireframe_frames_share_cache_preserve_document_and_follow_selection() {
        let mut d = Document::new(Default::default());
        let bounds = Bbox3d::from_scalars(0.0, 16.0);
        let brush = map_model::shapes::create_cuboid(&d.world_bounds(), &bounds, "stone").unwrap();
        d.add_brushes(vec![brush]);
        d.select_all();
        let id = d.create_brush_entity("func_animate").unwrap();
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 64 32 16 0 0 0");
        d.set_entity_property(id, "animation_pivot", "0 0 0");
        let original = d.to_map_text();
        let revision = d.revision();
        let rest = d.selected_edges().clone();
        let mut f = ViewFilters {
            animation_frame: Some(30),
            animation_fps: 30,
            onion_frames: 1,
            onion_step: 15,
            ..Default::default()
        };
        let frame = d.animation_wireframe(&f);
        assert_eq!(frame.selected.len(), rest.len());
        for ((a, b), (ra, rb)) in frame.selected.iter().zip(&rest) {
            assert_eq!(*a, *ra + Vec3d::new(32.0, 16.0, 8.0));
            assert_eq!(*b, *rb + Vec3d::new(32.0, 16.0, 8.0));
        }
        assert_eq!(frame.ghosts.len(), 2);
        let pointer = frame.selected.as_ptr();
        drop(frame);
        assert_eq!(d.animation_wireframe(&f).selected.as_ptr(), pointer);
        let sources = d.animation_wireframe.borrow().sources.as_ptr();
        f.animation_frame = Some(0);
        let frame = d.animation_wireframe(&f);
        assert_eq!(frame.selected, rest);
        assert_eq!(frame.ghosts.len(), 1);
        assert!(!frame.ghosts[0].0);
        drop(frame);
        assert_eq!(d.animation_wireframe.borrow().sources.as_ptr(), sources);
        assert_eq!(d.to_map_text(), original);
        assert_eq!(d.revision(), revision);
        d.set_entity_property(id, "animation_keys", "0 0 0 0 0 0 0; 2 0 0 0 0 0 90");
        d.set_entity_property(id, "animation_pivot", "8 8 8");
        f.animation_frame = Some(60);
        let rotated = d.animation_wireframe(&f);
        for ((a, b), (ra, rb)) in rotated.selected.iter().zip(&rest) {
            let expected = |p: Vec3d| Vec3d::new(16.0 - p.y(), p.x(), p.z());
            assert!(vec::length(*a - expected(*ra)) < 1e-8);
            assert!(vec::length(*b - expected(*rb)) < 1e-8);
        }
        assert!(rotated.ghosts.iter().all(|g| g.0));
        drop(rotated);
        d.clear_selection();
        assert!(d.animation_wireframe(&f).selected.is_empty());
        assert!(!d.animation_wireframe(&f).plain.is_empty());
        assert!(d.animation_wireframe(&f).ghosts.is_empty());
        d.select_all();
        d.hide_selection();
        assert!(d.animation_wireframe(&f).selected.is_empty());
        assert!(d.animation_wireframe(&f).plain.is_empty());
    }
}
