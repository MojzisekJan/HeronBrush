//! Flat drawing the frontends share: the 2D views, overlays, icons and previews.
//!
//! [`Canvas`] has the part of Cairo's API the editor draws with, under the same names and
//! with the same meaning: a current path built with `move_to`/`line_to`/`arc`, then
//! `stroke` or `fill`, with a transform and state stack. Code written against it runs on
//! egui's painter in the editor's window and on [`Raster`] where there is no window at
//! all.

use geometry_math::vec::Vec2d;

/// How the ends of a stroked line are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Cut square at the end point.
    #[default]
    Butt,
    /// A half circle beyond the end point.
    Round,
    /// A half square beyond the end point.
    Square,
}

/// How two stroked segments meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// A sharp corner.
    #[default]
    Miter,
    /// A rounded corner.
    Round,
    /// A cut-off corner.
    Bevel,
}

/// Which parts of a self-overlapping path a fill covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    /// Everything with a non-zero winding number.
    #[default]
    Winding,
    /// Everything enclosed an odd number of times.
    EvenOdd,
}

/// The measurements of a run of text, as Cairo's `text_extents` gives them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextExtents {
    /// How far the text is inked to the right of the current point where it starts.
    pub x_bearing: f64,
    /// How far above the baseline the ink starts (negative is above).
    pub y_bearing: f64,
    /// The inked width.
    pub width: f64,
    /// The inked height.
    pub height: f64,
    /// How far the current point moves.
    pub x_advance: f64,
    /// The height of a line of this text: what a label box around it is sized by.
    pub line_height: f64,
    /// How far below the top of that line the baseline sits.
    pub baseline: f64,
}

/// A surface to draw on, with Cairo's model and names. See the module documentation.
pub trait Canvas {
    /// Pushes the drawing state: transform, colour, line settings.
    fn save(&mut self);
    /// Pops the drawing state.
    fn restore(&mut self);
    /// Moves the origin.
    fn translate(&mut self, x: f64, y: f64);
    /// Scales the user space.
    fn scale(&mut self, x: f64, y: f64);

    /// Clears the current path.
    fn new_path(&mut self);
    /// Starts a new sub-path without a current point.
    fn new_sub_path(&mut self);
    /// Starts a sub-path at a point.
    fn move_to(&mut self, x: f64, y: f64);
    /// Adds a straight segment to a point.
    fn line_to(&mut self, x: f64, y: f64);
    /// Adds a clockwise arc (in a y-down space), angles in radians from +x.
    fn arc(&mut self, xc: f64, yc: f64, radius: f64, angle1: f64, angle2: f64);
    /// Adds a counter-clockwise arc.
    fn arc_negative(&mut self, xc: f64, yc: f64, radius: f64, angle1: f64, angle2: f64);
    /// Adds a closed rectangle.
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64);
    /// Closes the current sub-path.
    fn close_path(&mut self);

    /// Sets an opaque colour.
    fn set_source_rgb(&mut self, r: f64, g: f64, b: f64) {
        self.set_source_rgba(r, g, b, 1.0);
    }
    /// Sets a colour with alpha.
    fn set_source_rgba(&mut self, r: f64, g: f64, b: f64, a: f64);
    /// Sets the stroke width, in user units.
    fn set_line_width(&mut self, width: f64);
    /// Sets the dash pattern; an empty pattern draws solid lines.
    fn set_dash(&mut self, dashes: &[f64], offset: f64);
    /// Sets how line ends are drawn.
    fn set_line_cap(&mut self, cap: LineCap);
    /// Sets how line corners are drawn.
    fn set_line_join(&mut self, join: LineJoin);
    /// Sets which parts of a path a fill covers.
    fn set_fill_rule(&mut self, rule: FillRule);

    /// Strokes the current path and keeps it.
    fn stroke_preserve(&mut self);
    /// Fills the current path and keeps it.
    fn fill_preserve(&mut self);
    /// Strokes the current path and clears it.
    fn stroke(&mut self) {
        self.stroke_preserve();
        self.new_path();
    }
    /// Strokes independent segments and clears the path. Backends may stream these
    /// without allocating a separate path for every segment.
    fn stroke_segments(&mut self, segments: &mut dyn Iterator<Item = (Vec2d, Vec2d)>) {
        for (from, to) in segments {
            self.move_to(from.x(), from.y());
            self.line_to(to.x(), to.y());
        }
        self.stroke();
    }
    /// Fills the current path and clears it.
    fn fill(&mut self) {
        self.fill_preserve();
        self.new_path();
    }
    /// Fills the whole clip area with the current colour.
    fn paint(&mut self);
    /// Intersects the clip area with the current path, and clears the path.
    fn clip(&mut self);

    /// Fills the clip area with an RGBA image repeated in both directions, its top left
    /// at `origin` and each of its pixels `scale` user units across, sampled without
    /// smoothing: the UV editor's material behind the face. `key` names the image, so a
    /// backend that uploads it can keep the upload while the key stays the same.
    fn paint_tiled_image(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
        origin: (f64, f64),
        scale: f64,
    );

    /// The clip area's bounds in user space, where the backend can tell them.
    fn clip_extents(&mut self) -> Option<(f64, f64, f64, f64)> {
        None
    }

    /// Whether the text that follows is bold.
    fn set_font_bold(&mut self, bold: bool);
    /// Draws `text` with its baseline at the current point, `size` pixels high, in the
    /// current colour, and leaves no path behind.
    fn show_text(&mut self, text: &str, size: f64);
    /// Draws `text` as [`Self::show_text`] does, over an outline `width` wide in `outline`
    /// (RGBA), so a label stays legible over the map: Cairo's `text_path` stroked, then
    /// filled.
    fn show_text_outlined(&mut self, text: &str, size: f64, outline: [f64; 4], width: f64);
    /// Measures `text` at `size`.
    fn text_extents(&mut self, text: &str, size: f64) -> TextExtents;
}

/// How many straight segments approximate an arc of `sweep` radians at `radius` pixels.
pub fn arc_segments(radius: f64, sweep: f64) -> usize {
    ((sweep.abs() * radius.max(1.0).sqrt() * 2.0).ceil() as usize).clamp(2, 256)
}

/// The points along an arc, from `angle1` to `angle2`, clockwise when `positive`.
pub fn arc_points(
    xc: f64,
    yc: f64,
    radius: f64,
    angle1: f64,
    mut angle2: f64,
    positive: bool,
) -> Vec<(f64, f64)> {
    let tau = std::f64::consts::TAU;
    if positive {
        while angle2 < angle1 {
            angle2 += tau;
        }
    } else {
        while angle2 > angle1 {
            angle2 -= tau;
        }
    }
    let count = arc_segments(radius, angle2 - angle1);
    (0..=count)
        .map(|i| {
            let t = angle1 + (angle2 - angle1) * i as f64 / count as f64;
            (xc + radius * t.cos(), yc + radius * t.sin())
        })
        .collect()
}

/// A CPU canvas over a `tiny-skia` pixmap, for pictures made without a window: entity
/// sprites, asset previews, headless renders. Text is not drawn yet.
pub struct Raster {
    pixmap: tiny_skia::Pixmap,
    path: tiny_skia::PathBuilder,
    state: RasterState,
    stack: Vec<RasterState>,
    /// Whether there is no current point, so the next segment starts a sub-path.
    no_point: bool,
}

#[derive(Clone)]
struct RasterState {
    transform: tiny_skia::Transform,
    color: tiny_skia::Color,
    stroke: tiny_skia::Stroke,
    fill_rule: tiny_skia::FillRule,
    clip: Option<tiny_skia::Mask>,
}

impl Raster {
    /// A transparent canvas `width` by `height` pixels.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        Some(Self {
            pixmap: tiny_skia::Pixmap::new(width, height)?,
            path: tiny_skia::PathBuilder::new(),
            state: RasterState {
                transform: tiny_skia::Transform::identity(),
                color: tiny_skia::Color::BLACK,
                stroke: tiny_skia::Stroke {
                    width: 2.0,
                    ..Default::default()
                },
                fill_rule: tiny_skia::FillRule::Winding,
                clip: None,
            },
            stack: Vec::new(),
            no_point: true,
        })
    }

    /// The picture, straight-alpha RGBA rows from the top.
    pub fn into_rgba(self) -> crate::assets::Rgba {
        let (width, height) = (self.pixmap.width(), self.pixmap.height());
        let pixels = self
            .pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect();
        crate::assets::Rgba {
            width,
            height,
            pixels,
        }
    }

    fn take_path(&mut self) -> Option<tiny_skia::Path> {
        self.no_point = true;
        std::mem::replace(&mut self.path, tiny_skia::PathBuilder::new()).finish()
    }

    fn current_path(&mut self) -> Option<tiny_skia::Path> {
        let path = self.take_path()?;
        // Put it back, so the preserve variants keep it.
        let mut builder = tiny_skia::PathBuilder::new();
        builder.push_path(&path);
        self.path = builder;
        self.no_point = false;
        Some(path)
    }

    fn paint_style(&self) -> tiny_skia::Paint<'static> {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(self.state.color);
        paint.anti_alias = true;
        paint
    }

    fn user(&self, x: f64, y: f64) -> (f32, f32) {
        let mut point = tiny_skia::Point::from_xy(x as f32, y as f32);
        self.state.transform.map_point(&mut point);
        (point.x, point.y)
    }

    fn arc_to(&mut self, xc: f64, yc: f64, radius: f64, a1: f64, a2: f64, positive: bool) {
        // Points are mapped here and the path is kept in device space, so the transform
        // at the time of each call is the one that applies, as in Cairo.
        let points = arc_points(xc, yc, radius, a1, a2, positive);
        for (i, (x, y)) in points.into_iter().enumerate() {
            let (x, y) = self.user(x, y);
            if i == 0 && self.no_point {
                self.path.move_to(x, y);
            } else {
                self.path.line_to(x, y);
            }
        }
        self.no_point = false;
    }
}

impl Canvas for Raster {
    fn save(&mut self) {
        self.stack.push(self.state.clone());
    }
    fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.state = state;
        }
    }
    fn translate(&mut self, x: f64, y: f64) {
        self.state.transform = self.state.transform.pre_translate(x as f32, y as f32);
    }
    fn scale(&mut self, x: f64, y: f64) {
        self.state.transform = self.state.transform.pre_scale(x as f32, y as f32);
    }
    fn new_path(&mut self) {
        self.path = tiny_skia::PathBuilder::new();
        self.no_point = true;
    }
    fn new_sub_path(&mut self) {
        self.no_point = true;
    }
    fn move_to(&mut self, x: f64, y: f64) {
        let (x, y) = self.user(x, y);
        self.path.move_to(x, y);
        self.no_point = false;
    }
    fn line_to(&mut self, x: f64, y: f64) {
        let (x, y) = self.user(x, y);
        if self.no_point {
            self.path.move_to(x, y);
        } else {
            self.path.line_to(x, y);
        }
        self.no_point = false;
    }
    fn arc(&mut self, xc: f64, yc: f64, radius: f64, angle1: f64, angle2: f64) {
        self.arc_to(xc, yc, radius, angle1, angle2, true);
    }
    fn arc_negative(&mut self, xc: f64, yc: f64, radius: f64, angle1: f64, angle2: f64) {
        self.arc_to(xc, yc, radius, angle1, angle2, false);
    }
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.move_to(x, y);
        self.line_to(x + width, y);
        self.line_to(x + width, y + height);
        self.line_to(x, y + height);
        self.close_path();
    }
    fn close_path(&mut self) {
        self.path.close();
    }
    fn set_source_rgba(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.state.color = tiny_skia::Color::from_rgba(
            r.clamp(0.0, 1.0) as f32,
            g.clamp(0.0, 1.0) as f32,
            b.clamp(0.0, 1.0) as f32,
            a.clamp(0.0, 1.0) as f32,
        )
        .unwrap_or(tiny_skia::Color::BLACK);
    }
    fn set_line_width(&mut self, width: f64) {
        // The path is in device space, so the width is scaled here instead.
        let scale = self.state.transform.get_scale();
        self.state.stroke.width = (width as f32) * (scale.0.abs() + scale.1.abs()) / 2.0;
    }
    fn set_dash(&mut self, dashes: &[f64], offset: f64) {
        self.state.stroke.dash = if dashes.is_empty() {
            None
        } else {
            let mut pattern: Vec<f32> = dashes.iter().map(|d| *d as f32).collect();
            if pattern.len() % 2 == 1 {
                pattern.extend_from_within(..);
            }
            tiny_skia::StrokeDash::new(pattern, offset as f32)
        };
    }
    fn set_line_cap(&mut self, cap: LineCap) {
        self.state.stroke.line_cap = match cap {
            LineCap::Butt => tiny_skia::LineCap::Butt,
            LineCap::Round => tiny_skia::LineCap::Round,
            LineCap::Square => tiny_skia::LineCap::Square,
        };
    }
    fn set_line_join(&mut self, join: LineJoin) {
        self.state.stroke.line_join = match join {
            LineJoin::Miter => tiny_skia::LineJoin::Miter,
            LineJoin::Round => tiny_skia::LineJoin::Round,
            LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        };
    }
    fn set_fill_rule(&mut self, rule: FillRule) {
        self.state.fill_rule = match rule {
            FillRule::Winding => tiny_skia::FillRule::Winding,
            FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
        };
    }
    fn stroke_preserve(&mut self) {
        if let Some(path) = self.current_path() {
            let paint = self.paint_style();
            self.pixmap.stroke_path(
                &path,
                &paint,
                &self.state.stroke,
                tiny_skia::Transform::identity(),
                self.state.clip.as_ref(),
            );
        }
    }
    fn fill_preserve(&mut self) {
        if let Some(path) = self.current_path() {
            let paint = self.paint_style();
            self.pixmap.fill_path(
                &path,
                &paint,
                self.state.fill_rule,
                tiny_skia::Transform::identity(),
                self.state.clip.as_ref(),
            );
        }
    }
    fn paint(&mut self) {
        let rect = tiny_skia::Rect::from_xywh(
            0.0,
            0.0,
            self.pixmap.width() as f32,
            self.pixmap.height() as f32,
        );
        if let Some(rect) = rect {
            let paint = self.paint_style();
            self.pixmap.fill_rect(
                rect,
                &paint,
                tiny_skia::Transform::identity(),
                self.state.clip.as_ref(),
            );
        }
    }
    fn clip(&mut self) {
        if let Some(path) = self.take_path() {
            let mut mask = tiny_skia::Mask::new(self.pixmap.width(), self.pixmap.height());
            if let Some(mask) = mask.as_mut() {
                mask.fill_path(
                    &path,
                    self.state.fill_rule,
                    true,
                    tiny_skia::Transform::identity(),
                );
            }
            // Intersected with the clip already in force, as Cairo's `clip` does.
            if let (Some(mask), Some(old)) = (mask.as_mut(), self.state.clip.as_ref()) {
                for (new, old) in mask.data_mut().iter_mut().zip(old.data()) {
                    *new = ((u16::from(*new) * u16::from(*old)) / 255) as u8;
                }
            }
            self.state.clip = mask;
        }
    }
    fn paint_tiled_image(
        &mut self,
        _key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
        origin: (f64, f64),
        scale: f64,
    ) {
        let Some(size) = tiny_skia::IntSize::from_wh(width, height) else {
            return;
        };
        let premultiplied: Vec<u8> = pixels
            .chunks_exact(4)
            .flat_map(|p| {
                let a = u16::from(p[3]);
                [
                    (u16::from(p[0]) * a / 255) as u8,
                    (u16::from(p[1]) * a / 255) as u8,
                    (u16::from(p[2]) * a / 255) as u8,
                    p[3],
                ]
            })
            .collect();
        let Some(image) = tiny_skia::Pixmap::from_vec(premultiplied, size) else {
            return;
        };
        let transform = self
            .state
            .transform
            .pre_translate(origin.0 as f32, origin.1 as f32)
            .pre_scale(scale as f32, scale as f32);
        let mut paint = tiny_skia::Paint::default();
        paint.shader = tiny_skia::Pattern::new(
            image.as_ref(),
            tiny_skia::SpreadMode::Repeat,
            tiny_skia::FilterQuality::Nearest,
            1.0,
            transform,
        );
        let rect = tiny_skia::Rect::from_xywh(
            0.0,
            0.0,
            self.pixmap.width() as f32,
            self.pixmap.height() as f32,
        );
        if let Some(rect) = rect {
            self.pixmap.fill_rect(
                rect,
                &paint,
                tiny_skia::Transform::identity(),
                self.state.clip.as_ref(),
            );
        }
    }
    fn set_font_bold(&mut self, _bold: bool) {}
    fn show_text(&mut self, _text: &str, _size: f64) {}
    fn show_text_outlined(&mut self, _text: &str, _size: f64, _outline: [f64; 4], _width: f64) {}
    fn text_extents(&mut self, text: &str, size: f64) -> TextExtents {
        // An estimate, for layout only: text is not drawn here.
        let width = text.chars().count() as f64 * size * 0.55;
        TextExtents {
            x_bearing: 0.0,
            y_bearing: -size * 0.75,
            width,
            height: size,
            x_advance: width,
            line_height: size * 1.2,
            baseline: size * 0.95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_square_covers_its_pixels() {
        let mut canvas = Raster::new(10, 10).unwrap();
        canvas.set_source_rgb(1.0, 0.0, 0.0);
        canvas.rectangle(2.0, 2.0, 6.0, 6.0);
        canvas.fill();
        let image = canvas.into_rgba();
        let at = |x: usize, y: usize| &image.pixels[(y * 10 + x) * 4..(y * 10 + x) * 4 + 4];
        assert_eq!(at(5, 5), &[255, 0, 0, 255]);
        assert_eq!(at(0, 0)[3], 0);
    }

    #[test]
    fn an_arc_is_approximated_closely() {
        let points = arc_points(0.0, 0.0, 100.0, 0.0, std::f64::consts::PI, true);
        assert!(points.len() > 8);
        for (x, y) in points {
            assert!(((x * x + y * y).sqrt() - 100.0).abs() < 1e-9);
        }
    }
}
