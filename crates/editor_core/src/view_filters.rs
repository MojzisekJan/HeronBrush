//! What the views draw.
//!
//! Map-view visibility options, including filters for tags from the active game configuration.
//!
//! Filters are a view's business, not the document's. The document is asked what to draw
//! and is handed the filters to answer with, rather than holding them: two views could
//! want different answers, and a filter is not something a saved map remembers.

use crate::i18n::translation_key;
use geometry_math::vec::Vec3d;

/// Which entity links are drawn.
///
/// `Preferences::EntityLinkMode`. A busy map has hundreds of links and drawing them all
/// hides the map behind them; showing only the selection's is what makes them readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityLinkMode {
    /// Every link in the map.
    All,
    /// The links the selection reaches by following links, however many hops away.
    Transitive,
    /// Only the links touching what is selected.
    #[default]
    Connected,
    /// None.
    None,
}

impl EntityLinkMode {
    /// Every mode, in the order the panel lists them.
    pub const ALL: [EntityLinkMode; 4] = [
        EntityLinkMode::All,
        EntityLinkMode::Transitive,
        EntityLinkMode::Connected,
        EntityLinkMode::None,
    ];

    /// The label the panel shows.
    pub fn name(self) -> &'static str {
        match self {
            EntityLinkMode::All => translation_key("All links"),
            EntityLinkMode::Transitive => translation_key("The selection's chain"),
            EntityLinkMode::Connected => translation_key("Near selection"),
            EntityLinkMode::None => translation_key("No links"),
        }
    }
}

/// What the views draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFilters {
    /// Whether brushes are drawn.
    pub show_brushes: bool,
    /// Whether point entities are drawn.
    pub show_point_entities: bool,
    /// Whether a point entity is drawn as the model its class names, where there is one.
    ///
    /// `View Filter > Toggle show point entity models`. Off draws the box instead, which
    /// is what a class with no model, or one whose file is not on this machine, gets
    /// either way.
    pub show_point_entity_models: bool,
    /// Whether an entity's class is written beside it in the 2D views.
    pub show_entity_classnames: bool,
    /// Whether a group's outline is drawn around what is in it.
    pub show_group_bounds: bool,
    /// Whether Bézier patches are drawn.
    pub show_patches: bool,
    /// Whether a face is drawn with its material or in a flat colour.
    ///
    /// `Preferences::FaceRenderMode`, less the mode that draws nothing at all.
    pub textured_faces: bool,
    /// Debug perspective view: triangle edges, no filled surfaces or textures.
    pub wireframe: bool,
    /// Play continuous brush motion in the perspective view only.
    pub preview_motion: bool,
    pub animation_frame: Option<u32>,
    pub animation_fps: u32,
    pub animation_auto_key: bool,
    pub animation_rotate: bool,
    pub animation_edit_pivot: bool,
    pub onion_frames: u8,
    pub onion_step: u32,
    /// Whether the brushes' edges are drawn over their faces.
    pub show_edges: bool,
    pub show_export_hidden: bool,
    /// Whether a box is drawn around each brush entity's brushes.
    pub show_brush_entity_bounds: bool,
    /// Which entity links are drawn.
    pub entity_links: EntityLinkMode,
}

impl ViewFilters {
    pub fn brush_edges(
        &self,
        brush: &map_model::brush::Brush,
    ) -> Vec<(geometry_math::vec::Vec3d, geometry_math::vec::Vec3d)> {
        if self.show_export_hidden {
            return brush.edge_positions();
        }
        let mut edges = Vec::new();
        brush.for_each_geometry_face(|_, face, polygon| {
            if !face.export_hidden {
                for i in 0..polygon.len() {
                    edges.push((polygon[i], polygon[(i + 1) % polygon.len()]));
                }
            }
        });
        edges
    }

    pub fn geometry_only(mut self) -> Self {
        self.animation_frame = None;
        self.animation_fps = 30;
        self.animation_auto_key = false;
        self.animation_rotate = false;
        self.animation_edit_pivot = false;
        self.onion_frames = 0;
        self.onion_step = 5;
        self
    }
}
impl Default for ViewFilters {
    fn default() -> Self {
        Self {
            show_brushes: true,
            show_point_entities: true,
            show_point_entity_models: true,
            show_entity_classnames: true,
            show_group_bounds: true,
            show_patches: true,
            textured_faces: true,
            wireframe: false,
            preview_motion: false,
            animation_frame: None,
            animation_fps: 30,
            animation_auto_key: false,
            animation_rotate: false,
            animation_edit_pivot: false,
            onion_frames: 0,
            onion_step: 5,
            show_edges: true,
            show_export_hidden: true,
            show_brush_entity_bounds: true,
            entity_links: EntityLinkMode::default(),
        }
    }
}

/// One entity aiming at another.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityLink {
    /// Where the link starts, at the entity doing the targeting.
    pub from: Vec3d,
    /// Where it ends, at the entity being targeted.
    pub to: Vec3d,
    /// Whether either end is selected, which is what `Connected` keeps.
    pub touches_selection: bool,
    /// Whether the selection reaches either end by following links, which is what
    /// `Transitive` keeps. A link that touches the selection reaches it too.
    pub reaches_selection: bool,
    /// The targeting key the link is made by (`target`, `killtarget2`, ...).
    pub key: String,
    /// The name it aims at.
    pub name: String,
}

impl EntityLink {
    /// Whether the link removes what it aims at (`killtarget`) rather than firing it,
    /// which the views draw in their own colour.
    pub fn kills(&self) -> bool {
        self.key
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .eq_ignore_ascii_case("killtarget")
    }

    /// What the views write by a link that touches the selection: `killtarget → lamp`.
    pub fn label(&self) -> String {
        format!("{} → {}", self.key, self.name)
    }
}

impl EntityLinkMode {
    /// Whether a link is drawn under this mode, `getLinks`' choice of visitor.
    pub fn shows(self, link: &EntityLink) -> bool {
        match self {
            EntityLinkMode::All => true,
            EntityLinkMode::Transitive => link.reaches_selection,
            EntityLinkMode::Connected => link.touches_selection,
            EntityLinkMode::None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_drawn_until_something_is_turned_off() {
        let filters = ViewFilters::default();
        assert!(filters.show_brushes);
        assert!(filters.show_point_entities);
        assert!(filters.show_point_entity_models);
        assert!(filters.show_entity_classnames);
        assert!(filters.show_group_bounds);
        assert!(filters.show_patches);
    }

    #[test]
    fn links_start_out_showing_only_what_is_selected() {
        // A busy map has hundreds, and all of them at once hides the map behind them.
        assert_eq!(
            ViewFilters::default().entity_links,
            EntityLinkMode::Connected
        );
    }

    #[test]
    fn every_link_mode_has_a_label() {
        for mode in EntityLinkMode::ALL {
            assert!(!mode.name().is_empty());
        }
    }
}

/// Tells the console what turning a tag on or off did.
pub fn report_tag_outcome(name: &str, outcome: Option<crate::document::TagOutcome>) {
    use crate::document::TagOutcome;
    match outcome {
        Some(TagOutcome::Changed(0)) => {
            crate::say!("the selection already matches `{name}`")
        }
        Some(TagOutcome::Changed(count)) => {
            crate::say!("changed {count} faces for `{name}`")
        }
        Some(TagOutcome::Ambiguous { used, changed }) => {
            crate::say!("`{name}` matches several materials; put `{used}` on {changed} faces")
        }
        Some(TagOutcome::NoMaterial) => {
            crate::complain!("no loaded material matches `{name}`")
        }
        Some(TagOutcome::NoSelection) => {
            crate::complain!("select some faces or brushes first")
        }
        Some(TagOutcome::Unsupported) | None => {
            crate::complain!("`{name}` cannot be changed from here")
        }
    }
}
