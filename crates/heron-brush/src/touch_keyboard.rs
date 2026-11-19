//! Keyboard occlusion in native UIKit points, converted to egui points.
use eframe::egui::Rect;

pub(super) fn docked_inset(view: Rect, keyboard: Rect, zoom: f32) -> f32 {
    if !view.is_finite()
        || !keyboard.is_finite()
        || !zoom.is_finite()
        || zoom <= 0.0
        || view.width() <= 0.0
        || view.height() <= 0.0
    {
        return 0.0;
    }
    let overlap = view.intersect(keyboard);
    // An undocked or floating keyboard must not shrink the whole editor.
    if overlap.width() < view.width() * 0.9 || overlap.bottom() < view.bottom() - 1.0 {
        return 0.0;
    }
    overlap.height().max(0.0).min(view.height()) / zoom
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};
    #[test]
    fn keyboard_occlusion_tracks_rotation_and_ui_zoom() {
        for size in [vec2(1210., 834.), vec2(834., 1210.)] {
            let view = Rect::from_min_size(pos2(0., 0.), size);
            let keyboard = Rect::from_min_max(pos2(0., size.y - 320.), view.max);
            assert_eq!(docked_inset(view, keyboard, 1.), 320.);
            assert_eq!(docked_inset(view, keyboard, 2.), 160.);
        }
    }
    #[test]
    fn floating_hidden_and_invalid_keyboards_do_not_cover_the_editor() {
        let view = Rect::from_min_size(pos2(0., 0.), vec2(1210., 834.));
        for keyboard in [
            Rect::from_min_size(pos2(400., 400.), vec2(300., 200.)),
            Rect::from_min_size(pos2(0., 834.), vec2(1210., 320.)),
            Rect::NOTHING,
        ] {
            assert_eq!(docked_inset(view, keyboard, 1.), 0.);
        }
        assert_eq!(docked_inset(view, view, 0.), 0.);
        assert_eq!(docked_inset(view, view, f32::NAN), 0.);
    }
}
