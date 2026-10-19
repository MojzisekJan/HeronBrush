//! Shared, visible distinction between pose editing and map geometry editing.
use crate::{i18n::tr,view_filters::ViewFilters};
pub fn label(f:&ViewFilters)->String {
    match f.animation_frame {
        Some(frame)=>format!("{} · {} {}",tr("Edit animation"),tr("Frame"),frame),
        None=>tr("Edit map").into(),
    }
}
pub fn draw(cr:&mut dyn crate::canvas::Canvas,f:&ViewFilters,height:f64) {
    if f.animation_frame.is_none() { return; }
    cr.save();
    cr.translate(0.0,(height-44.0).max(0.0));
    cr.set_font_bold(true);
    let text=label(f);
    let width=cr.text_extents(&text,12.0).x_advance;
    cr.set_source_rgba(0.08,0.09,0.11,0.9);cr.rectangle(8.0,8.0,width+20.0,28.0);cr.fill();
    cr.set_source_rgb(1.0,0.7,0.25);
    cr.move_to(18.0,27.0);cr.show_text(&text,12.0);
    cr.set_font_bold(false);
    cr.restore();
}
