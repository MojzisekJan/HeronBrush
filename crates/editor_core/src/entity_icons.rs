//! Shared editor-only symbols. Never added to exported entity geometry.
use crate::canvas::{Canvas, LineCap, LineJoin, Raster};

/// One straight stroke of an icon.
fn line(cr: &mut dyn Canvas, a: f64, b: f64, c: f64, d: f64) {
    cr.move_to(a, b);
    cr.line_to(c, d);
}
pub fn kind(name: &str) -> Option<&'static str> {
    Some(match name {
        "ambient_sound" => "sound",
        "water_volume" => "water",
        "light" | "light_spot" => "light",
        "light_sun" => "sun",
        "info_player_start" | "info_player_deathmatch" => "player",
        "info_camera" => "camera",
        "info_waypoint" => "pin",
        "prop_static" | "reflection_probe" => "cube",
        "trigger_once" | "trigger_multiple" | "func_button" => "trigger",
        "func_door" => "door",
        "func_platform" | "func_shuttle" | "func_animate" => "move",
        "func_rotate" => "rotate",
        "worldspawn"
        | "func_group"
        | "func_geo"
        | "func_detail"
        | "func_illusionary"
        | "func_detail_illusionary"
        | "func_clip"
        | "func_wall"
        | "func_physics" => "cube",
        _ => return None,
    })
}
pub fn draw(cr: &mut dyn Canvas, name: &str, x: f64, y: f64, size: f64) {
    let Some(k) = kind(name) else {
        return;
    };
    let _ = cr.save();
    cr.translate(x, y);
    cr.scale(size / 24., size / 24.);
    cr.set_line_width(1.8);
    cr.set_line_cap(LineCap::Round);
    cr.set_line_join(LineJoin::Round);
    match k {
        "water" => {
            for y in [7.,12.,17.] {cr.move_to(2.,y);for i in 1..=5 {cr.line_to(2.+i as f64*4.,y+if i%2==0{0.}else{-2.});}cr.new_sub_path();}
        }
        "sound" => {
            cr.move_to(3., 9.);
            cr.line_to(7., 9.);
            cr.line_to(12., 5.);
            cr.line_to(12., 19.);
            cr.line_to(7., 15.);
            cr.line_to(3., 15.);
            cr.close_path();
            cr.new_sub_path();
            cr.arc(12., 12., 5., -0.8, 0.8);
            cr.new_sub_path();
            cr.arc(12., 12., 9., -0.8, 0.8);
        }
        "light" | "sun" => {
            cr.arc(12., 10., 4., 0., std::f64::consts::TAU);
            for i in 0..8 {
                let a = i as f64 * std::f64::consts::TAU / 8.;
                line(cr, 12. + 6. * a.cos(),
                    10. + 6. * a.sin(),
                    12. + 8. * a.cos(),
                    10. + 8. * a.sin(),
                );
            }
            if k == "light" {
                line(cr, 9., 19., 15., 19.);
                line(cr, 10., 22., 14., 22.);
            }
        }
        "player" => {
            cr.arc(12., 5., 3., 0., std::f64::consts::TAU);
            line(cr, 12., 9., 12., 16.);
            line(cr, 5., 12., 19., 12.);
            line(cr, 12., 16., 7., 22.);
            line(cr, 12., 16., 17., 22.);
        }
        "camera" => {
            cr.rectangle(3., 7., 12., 11.);
            cr.move_to(15., 10.);
            cr.line_to(21., 7.);
            cr.line_to(21., 18.);
            cr.line_to(15., 15.);
        }
        "pin" => {
            cr.arc(12., 8., 5., 0., std::f64::consts::TAU);
            line(cr, 8., 12., 12., 22.);
            line(cr, 16., 12., 12., 22.);
        }
        "door" => {
            cr.rectangle(5., 3., 14., 19.);
            line(cr, 15., 13., 16., 13.);
        }
        "trigger" => {
            cr.move_to(13., 2.);
            cr.line_to(5., 14.);
            cr.line_to(11., 14.);
            cr.line_to(10., 22.);
            cr.line_to(20., 9.);
            cr.line_to(13., 9.);
            cr.close_path();
        }
        "move" => {
            line(cr, 3., 12., 21., 12.);
            line(cr, 3., 12., 7., 8.);
            line(cr, 3., 12., 7., 16.);
            line(cr, 21., 12., 17., 8.);
            line(cr, 21., 12., 17., 16.);
        }
        "rotate" => {
            cr.arc(12., 12., 8., 0.5, 5.8);
            line(cr, 19., 8., 21., 4.);
            line(cr, 19., 8., 15., 7.);
        }
        _ => {
            cr.move_to(3., 7.);
            cr.line_to(12., 3.);
            cr.line_to(21., 7.);
            cr.line_to(21., 17.);
            cr.line_to(12., 22.);
            cr.line_to(3., 17.);
            cr.close_path();
            line(cr, 3., 7., 12., 12.);
            line(cr, 21., 7., 12., 12.);
            line(cr, 12., 12., 12., 22.);
        }
    }
    let _ = cr.stroke();
    let _ = cr.restore();
}
pub fn sprite(name: &str) -> map_model::sprite::Sprite {
    let mut canvas = Raster::new(48, 48).expect("a 48 pixel canvas");
    canvas.set_source_rgb(1., 1., 1.);
    draw(&mut canvas, name, 2., 2., 44.);
    // White throughout, with the icon in the alpha channel.
    let mut pixels = canvas.into_rgba().pixels;
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[..3].copy_from_slice(&[255, 255, 255]);
    }
    map_model::sprite::Sprite {
        width: 48,
        height: 48,
        pixels,
    }
}
