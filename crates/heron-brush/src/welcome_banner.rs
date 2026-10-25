//! The welcome screen's banner: the bundled render, tinted in the editor's blue, darkened
//! towards the title, dithered in halftone dots that grow towards its foot until it
//! dissolves into the page, with diffraction rings round the logo and faint scanlines.
//! Worked out on the CPU at the size it is drawn at, in physical pixels, so the dots stay
//! crisp; drawn again only when that size or the page's colour changes.

use eframe::egui::{Color32, ColorImage};

/// The render behind the banner, decoded once.
pub struct Source {
    width: usize,
    height: usize,
    /// One luminance a pixel, 0 to 1.
    light: Vec<f32>,
}

impl Source {
    /// The bundled render.
    pub fn bundled() -> Option<Self> {
        Self::from_png(include_bytes!("../assets/welcome-banner.png"))
    }

    fn from_png(bytes: &[u8]) -> Option<Self> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .ok()?
            .to_rgb8();
        let (width, height) = (image.width() as usize, image.height() as usize);
        let light = image
            .pixels()
            .map(|p| {
                (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]))
                    / 255.0
            })
            .collect();
        Some(Self {
            width,
            height,
            light,
        })
    }
}

/// What the banner is drawn for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// Its size in physical pixels.
    pub size: [usize; 2],
    /// Physical pixels to a point.
    pub scale: f32,
    /// The logo's centre, in points from the banner's corner, and its radius.
    pub logo: (f32, f32, f32),
    /// The page's background, which the foot dissolves into.
    pub page: Color32,
}

/// The editor's blue the render is tinted with, the dark the title stands on and the
/// rings' two colours: the accent and the heron's orange beak.
const TINT: [f32; 3] = [43.0, 95.0, 179.0];
const SHADE: [f32; 3] = [20.0, 21.0, 23.0];
const BLUE_RING: [f32; 3] = [90.0, 157.0, 255.0];
const ORANGE_RING: [f32; 3] = [243.0, 154.0, 43.0];

/// Straight line pieces through `points`, (position, value) with positions rising.
fn ramp(points: &[(f32, f32)], at: f32) -> f32 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    if at <= first.0 {
        return first.1;
    }
    for pair in points.windows(2) {
        let ((a, va), (b, vb)) = (pair[0], pair[1]);
        if at <= b {
            return va + (vb - va) * (at - a) / (b - a).max(1e-6);
        }
    }
    points[points.len() - 1].1
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// How bright a ring of `period` points is at `radius`: 1 on the ring, falling to 0
/// `width` either side.
fn ring(radius: f32, period: f32, width: f32) -> f32 {
    let off = (radius % period - period / 2.0).abs();
    (1.0 - (period / 2.0 - off) / width).clamp(0.0, 1.0)
}

/// The banner as an image.
pub fn draw(source: &Source, layout: Layout) -> ColorImage {
    let [width, height] = layout.size;
    let s = layout.scale.max(0.5);
    let page = [
        f32::from(layout.page.r()),
        f32::from(layout.page.g()),
        f32::from(layout.page.b()),
    ];
    // The render covers the banner, cut from a little above its middle.
    let k = (width as f32 / source.width as f32).max(height as f32 / source.height as f32);
    let shown = height as f32 / k;
    let top = (source.height as f32 - shown).max(0.0) * 0.33;
    let left = (source.width as f32 - width as f32 / k).max(0.0) / 2.0;
    let (lx, ly, lr) = (layout.logo.0 * s, layout.logo.1 * s, layout.logo.2 * s);
    let cell = 6.0 * s;
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let down = y as f32 / height.max(1) as f32;
        let up = 1.0 - down;
        let sy = ((top + y as f32 / k) as usize).min(source.height - 1);
        let scan = (y as f32 / s) % 3.0 < 1.0;
        for x in 0..width {
            let across = x as f32 / width.max(1) as f32;
            let sx = ((left + x as f32 / k) as usize).min(source.width - 1);
            let light = source.light[sy * source.width + sx];
            let light = (((light - 0.5) * 1.25 + 0.5) * 0.95).clamp(0.0, 1.0);
            let mut colour = [TINT[0] * light, TINT[1] * light, TINT[2] * light];
            // Darker towards the title on the left, a little at the far right.
            let dark = ramp(
                &[(0.0, 0.92), (0.42, 0.55), (0.75, 0.05), (1.0, 0.35)],
                across,
            );
            colour = mix(colour, SHADE, dark);

            // Diffraction rings round the logo: two sets a touch apart in spacing and
            // centre, so they beat into moiré, fading outwards.
            let (dx, dy) = (x as f32 - lx, y as f32 - ly);
            let radius = (dx * dx + dy * dy).sqrt();
            let outer = lr + 2.0 * s;
            let fade = ramp(
                &[
                    (lr, 0.0),
                    (outer + 4.0 * s, 1.0),
                    (lr + 110.0 * s, 0.55),
                    (lr + 300.0 * s, 0.0),
                ],
                radius,
            );
            if fade > 0.0 {
                colour = mix(colour, BLUE_RING, 0.30 * fade * ring(radius / s, 5.0, 0.8));
                let (ex, ey) = (dx - 18.0 * s, dy - 8.0 * s);
                let second = (ex * ex + ey * ey).sqrt() / s;
                colour = mix(colour, ORANGE_RING, 0.16 * fade * ring(second, 5.5, 0.7));
            }

            // Halftone: fine dots in the middle band, growing towards the foot, where
            // they take the page's colour so the banner dissolves into it.
            let (cx, cy) = (
                ((x as f32 / cell).floor() + 0.5) * cell,
                ((y as f32 / cell).floor() + 0.5) * cell,
            );
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let band = ramp(
                &[(0.30, 0.0), (0.55, 1.0), (0.75, 1.0), (0.92, 0.0)],
                across,
            );
            let fine = 1.1 * s * band;
            let middle = 1.9 * s * ramp(&[(0.0, 1.0), (0.22, 0.9), (0.48, 0.0)], up);
            let coarse = 2.8 * s * ramp(&[(0.0, 1.0), (0.22, 0.0)], up);
            let dot = |r: f32| (r - d + 0.5).clamp(0.0, 1.0);
            colour = mix(colour, SHADE, dot(fine));
            colour = mix(colour, page, dot(middle.max(coarse)));

            if scan {
                colour = [colour[0] * 0.86, colour[1] * 0.86, colour[2] * 0.86];
            }
            // The last rows are the page itself, under the dots' foot.
            colour = mix(colour, page, ramp(&[(0.0, 1.0), (0.04, 0.0)], up));
            pixels.push(Color32::from_rgb(
                colour[0] as u8,
                colour[1] as u8,
                colour[2] as u8,
            ));
        }
    }
    ColorImage {
        size: [width, height],
        source_size: eframe::egui::vec2(width as f32, height as f32),
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(size: [usize; 2]) -> Layout {
        Layout {
            size,
            scale: 1.0,
            logo: (136.0, 172.0, 60.0),
            page: Color32::from_rgb(27, 28, 31),
        }
    }

    #[test]
    fn the_bundled_render_reads_and_the_banner_has_its_size() {
        let source = Source::bundled().expect("the render reads");
        let image = draw(&source, layout([640, 200]));
        assert_eq!(image.size, [640, 200]);
        assert_eq!(image.pixels.len(), 640 * 200);
    }

    #[test]
    fn the_foot_is_the_page_and_rings_lighten_round_the_logo() {
        let source = Source::bundled().unwrap();
        let banner = layout([1280, 330]);
        let image = draw(&source, banner);
        let at = |x: usize, y: usize| image.pixels[y * 1280 + x];
        assert_eq!(at(640, 329), banner.page, "it ends in the page's colour");
        // Inside the logo nothing rings; just outside a ring is brighter than the gap
        // beside it.
        let blue = |x: usize| at(x, 172).b();
        let ringed = (200..240).map(blue).max().unwrap();
        let gaps = (200..240).map(blue).min().unwrap();
        assert!(ringed > gaps + 20, "{ringed} {gaps}");
    }
}
