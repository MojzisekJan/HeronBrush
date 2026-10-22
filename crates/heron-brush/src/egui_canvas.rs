//! [`Canvas`] on egui's painter, so the views the core draws appear in egui.
//!
//! A path is kept as sub-paths of points in screen space, with the transform at the
//! time of each call applied as Cairo applies it. Strokes become egui lines with the
//! dash pattern, caps and joins applied here; fills become a convex polygon where the
//! path is one convex ring, and otherwise a scanline mesh that honours the fill rule, so
//! even-odd holes stay open. Clips narrow the painter's clip rectangle: exactly for the
//! axis-aligned rectangles the core clips with, to the bounds for any other shape.

use editor_core::canvas::{Canvas, FillRule, LineCap, LineJoin, TextExtents};
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke};

#[derive(Clone)]
struct State {
    /// `x' = a*x + c`, `y' = b*y + d`: translate and scale only, which is all the views
    /// use.
    scale: (f64, f64),
    offset: (f64, f64),
    color: Color32,
    width: f64,
    /// The dash lengths and offset, in user units at the time they were set, as Cairo
    /// keeps them.
    dash: Option<(Vec<f64>, f64)>,
    cap: LineCap,
    join: LineJoin,
    fill_rule: FillRule,
    clip: Rect,
    bold: bool,
}

/// What a capturing canvas recorded, in drawing order: runs of egui shapes, and runs of
/// one-pixel segments it tessellated itself.
#[derive(Default)]
pub struct Capture {
    layers: Vec<Captured>,
    /// The segments already in the current run of lines, by their ends rounded to half
    /// a pixel and colour: zoomed out over a big map, most edges land on top of others
    /// (two brushes sharing a face, a stack of floors seen from above), and drawing each
    /// again only made the picture bigger.
    seen: rustc_hash::FxHashSet<(i32, i32, i32, i32, u32)>,
}

enum Captured {
    Shapes(Vec<Shape>),
    Lines(egui::Mesh),
}

impl Capture {
    fn shapes(&mut self) -> &mut Vec<Shape> {
        if !matches!(self.layers.last(), Some(Captured::Shapes(_))) {
            self.layers.push(Captured::Shapes(Vec::new()));
        }
        match self.layers.last_mut() {
            Some(Captured::Shapes(shapes)) => shapes,
            _ => unreachable!("just pushed"),
        }
    }

    fn lines(&mut self) -> &mut egui::Mesh {
        if !matches!(self.layers.last(), Some(Captured::Lines(_))) {
            self.layers.push(Captured::Lines(egui::Mesh::default()));
            self.seen.clear();
        }
        match self.layers.last_mut() {
            Some(Captured::Lines(mesh)) => mesh,
            _ => unreachable!("just pushed"),
        }
    }

    /// Everything recorded, as meshes ready to paint in order: the shapes tessellated by
    /// egui clipped to `clip`, the segments as they are.
    pub fn into_meshes(
        self,
        ctx: &egui::Context,
        clip: Rect,
        pixels_per_point: f32,
    ) -> Vec<std::sync::Arc<egui::Mesh>> {
        let mut out = Vec::new();
        for layer in self.layers {
            match layer {
                Captured::Shapes(shapes) => {
                    let clipped = shapes
                        .into_iter()
                        .map(|shape| egui::epaint::ClippedShape {
                            clip_rect: clip,
                            shape,
                        })
                        .collect();
                    for primitive in ctx.tessellate(clipped, pixels_per_point) {
                        if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                            if !mesh.is_empty() {
                                out.push(std::sync::Arc::new(mesh));
                            }
                        }
                    }
                }
                Captured::Lines(mesh) => {
                    if !mesh.is_empty() {
                        out.push(std::sync::Arc::new(mesh));
                    }
                }
            }
        }
        out
    }
}

/// An egui painter seen as a [`Canvas`], with the origin at `rect`'s top left.
pub struct EguiCanvas<'a> {
    painter: &'a egui::Painter,
    /// Where the drawing goes instead of the painter, for a picture kept between frames.
    capture: Option<std::cell::RefCell<Capture>>,
    /// Physical pixels per point, for the segments a capture tessellates itself.
    pixels_per_point: f32,
    state: State,
    stack: Vec<State>,
    /// The sub-paths, each with whether it is closed.
    paths: Vec<(Vec<Pos2>, bool)>,
    current: Option<(f64, f64)>,
}

impl<'a> EguiCanvas<'a> {
    /// A canvas over `rect` of the painter.
    pub fn new(painter: &'a egui::Painter, rect: Rect) -> Self {
        Self {
            painter,
            capture: None,
            pixels_per_point: painter.ctx().pixels_per_point(),
            state: State {
                scale: (1.0, 1.0),
                offset: (f64::from(rect.min.x), f64::from(rect.min.y)),
                color: Color32::BLACK,
                width: 2.0,
                dash: None,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                fill_rule: FillRule::Winding,
                clip: rect.intersect(painter.clip_rect()),
                bold: false,
            },
            stack: Vec::new(),
            paths: Vec::new(),
            current: None,
        }
    }

    /// A canvas over `rect` that records what is drawn instead of painting it; see
    /// [`EguiCanvas::into_capture`].
    pub fn capturing(painter: &'a egui::Painter, rect: Rect) -> Self {
        let mut canvas = Self::new(painter, rect);
        canvas.capture = Some(std::cell::RefCell::new(Capture::default()));
        canvas
    }

    /// What a capturing canvas recorded.
    pub fn into_capture(self) -> Capture {
        self.capture
            .map(std::cell::RefCell::into_inner)
            .unwrap_or_default()
    }

    /// Paints `shapes`, or records them when capturing.
    fn emit(&self, shapes: Vec<Shape>) {
        match &self.capture {
            Some(capture) => capture.borrow_mut().shapes().extend(shapes),
            None => self.painter().extend(shapes),
        }
    }

    fn emit_one(&self, shape: Shape) {
        match &self.capture {
            Some(capture) => capture.borrow_mut().shapes().push(shape),
            None => {
                self.painter().add(shape);
            }
        }
    }

    /// One thin straight segment into a capture's own mesh: a core line with a pixel of
    /// fade either side, six vertices -- what egui's feathered stroke gives a hairline,
    /// for a fraction of the work of going through its tessellator a segment at a time.
    /// Returns false (drawing nothing) when not capturing or the stroke is too wide.
    fn capture_segment(&self, from: Pos2, to: Pos2, width: f32, color: Color32) -> bool {
        let Some(capture) = &self.capture else {
            return false;
        };
        let pixel = 1.0 / self.pixels_per_point;
        if width > 2.5 * pixel {
            return false;
        }
        let along = to - from;
        let length = along.length();
        let mut capture = capture.borrow_mut();
        let _ = capture.lines();
        let half = |v: f32| (v * self.pixels_per_point * 2.0).round() as i32;
        let (a, b) = ((half(from.x), half(from.y)), (half(to.x), half(to.y)));
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let colour = u32::from_le_bytes(color.to_array()) ^ width.to_bits().rotate_left(7);
        if !capture.seen.insert((a.0, a.1, b.0, b.1, colour)) {
            return true;
        }
        let mesh = capture.lines();
        // A segment shorter than a pixel is drawn as a pixel-long dash, so a brush far too
        // small to see at this zoom is still a dot rather than nothing.
        let (direction, from, to) = if length < pixel {
            (
                egui::vec2(1.0, 0.0),
                from - egui::vec2(pixel / 2.0, 0.0),
                from + egui::vec2(pixel / 2.0, 0.0),
            )
        } else {
            (along / length, from, to)
        };
        let normal = egui::vec2(-direction.y, direction.x);
        // Thinner than a pixel: one pixel wide, fainter (as egui draws a hairline).
        let (core, color) = if width < pixel {
            (0.0, color.gamma_multiply(width / pixel))
        } else {
            ((width - pixel) / 2.0, color)
        };
        let fade = normal * (core + pixel);
        let inner = normal * core;
        let base = mesh.vertices.len() as u32;
        let clear = Color32::TRANSPARENT;
        let uv = egui::epaint::WHITE_UV;
        if core == 0.0 {
            // A hairline: a bright middle fading out either side, three vertices across.
            for (p, c) in [
                (from + fade, clear),
                (from, color),
                (from - fade, clear),
                (to + fade, clear),
                (to, color),
                (to - fade, clear),
            ] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: p,
                    uv,
                    color: c,
                });
            }
            for (a, b) in [(0, 1), (1, 2)] {
                mesh.indices.extend([
                    base + a,
                    base + b,
                    base + 3 + a,
                    base + b,
                    base + 3 + b,
                    base + 3 + a,
                ]);
            }
            return true;
        }
        for (p, c) in [
            (from + fade, clear),
            (from + inner, color),
            (from - inner, color),
            (from - fade, clear),
            (to + fade, clear),
            (to + inner, color),
            (to - inner, color),
            (to - fade, clear),
        ] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p,
                uv,
                color: c,
            });
        }
        for (a, b) in [(0, 1), (1, 2), (2, 3)] {
            mesh.indices.extend([
                base + a,
                base + b,
                base + 4 + a,
                base + b,
                base + 4 + b,
                base + 4 + a,
            ]);
        }
        true
    }

    fn map(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(
            (x * self.state.scale.0 + self.state.offset.0) as f32,
            (y * self.state.scale.1 + self.state.offset.1) as f32,
        )
    }

    fn unmap(&self, p: Pos2) -> (f64, f64) {
        (
            (f64::from(p.x) - self.state.offset.0) / self.state.scale.0,
            (f64::from(p.y) - self.state.offset.1) / self.state.scale.1,
        )
    }

    fn mean_scale(&self) -> f64 {
        (self.state.scale.0.abs() + self.state.scale.1.abs()) / 2.0
    }

    fn push_point(&mut self, x: f64, y: f64) {
        let p = self.map(x, y);
        match self.paths.last_mut() {
            Some((points, false)) if self.current.is_some() => points.push(p),
            _ => self.paths.push((sub_path(p), false)),
        }
        self.current = Some((x, y));
    }

    fn arc_to(&mut self, xc: f64, yc: f64, radius: f64, a1: f64, a2: f64, positive: bool) {
        let points = editor_core::canvas::arc_points(xc, yc, radius, a1, a2, positive);
        for (i, (x, y)) in points.into_iter().enumerate() {
            if i == 0 && self.current.is_none() {
                self.paths.push((sub_path(self.map(x, y)), false));
                self.current = Some((x, y));
            } else {
                self.push_point(x, y);
            }
        }
    }

    /// Strokes `paths`, adding the shapes to the painter in one go: a flat view strokes
    /// tens of thousands of edges, and adding them one at a time took the context's lock
    /// for each.
    fn stroke_paths(&self, paths: Vec<(Vec<Pos2>, bool)>) {
        let scale = self.mean_scale();
        let mut out = Vec::with_capacity(paths.len());
        for (mut points, closed) in paths {
            if points.len() < 2 {
                continue;
            }
            match &self.state.dash {
                Some((pattern, offset)) => {
                    let pattern: Vec<f32> = pattern.iter().map(|d| (d * scale) as f32).collect();
                    if closed {
                        points.push(points[0]);
                    }
                    for piece in dash_polyline(&points, &pattern, (offset * scale) as f32) {
                        self.stroke_polyline(&mut out, piece, false);
                    }
                }
                None => self.stroke_polyline(&mut out, points, closed),
            }
        }
        if !out.is_empty() {
            self.emit(out);
        }
    }

    fn painter(&self) -> egui::Painter {
        self.painter.with_clip_rect(self.state.clip)
    }

    fn stroke_width(&self) -> f32 {
        (self.state.width * self.mean_scale()) as f32
    }

    fn font(&self, size: f64) -> egui::FontId {
        let bold = egui::FontFamily::Name("bold".into());
        // The bold family exists once the theme has loaded its fonts; plain text stands
        // in before that.
        let family = if self.state.bold
            && self
                .painter
                .ctx()
                .fonts(|f| f.definitions().families.contains_key(&bold))
        {
            bold
        } else {
            egui::FontFamily::Proportional
        };
        egui::FontId::new(size as f32, family)
    }

    fn galley(&self, text: &str, size: f64) -> std::sync::Arc<egui::Galley> {
        self.painter
            .layout_no_wrap(text.to_string(), self.font(size), self.state.color)
    }

    /// One stroked polyline with the state's caps and joins.
    fn stroke_polyline(&self, out: &mut Vec<Shape>, mut points: Vec<Pos2>, closed: bool) {
        let width = self.stroke_width();
        let stroke = Stroke::new(width, self.state.color);
        let half = width / 2.0;
        if points.len() < 2 {
            return;
        }
        if !closed && self.state.cap == LineCap::Square {
            let n = points.len();
            for (end, next) in [(0, 1), (n - 1, n - 2)] {
                let direction = (points[end] - points[next]).normalized();
                if direction.is_finite() {
                    points[end] += direction * half;
                }
            }
        }
        if closed {
            points.push(points[0]);
        }
        let round = |out: &mut Vec<Shape>, at: Pos2| {
            out.push(Shape::circle_filled(at, half, self.state.color))
        };
        if half > 0.75 {
            if self.state.join == LineJoin::Round {
                let interior = if closed {
                    0..points.len() - 1
                } else {
                    1..points.len() - 1
                };
                for i in interior {
                    round(out, points[i]);
                }
            }
            if !closed && self.state.cap == LineCap::Round {
                round(out, points[0]);
                round(out, points[points.len() - 1]);
            }
        }
        // A single segment -- every edge the flat views stroke -- needs no list of its own.
        if let [from, to] = points[..] {
            if self.capture_segment(from, to, width, self.state.color) {
                return;
            }
            out.push(Shape::line_segment([from, to], stroke));
        } else {
            out.push(Shape::line(points, stroke));
        }
    }
}

/// A new sub-path starting at `p`, with room for the line the flat views almost always
/// add to it: one allocation per edge rather than two.
fn sub_path(p: Pos2) -> Vec<Pos2> {
    let mut points = Vec::with_capacity(2);
    points.push(p);
    points
}

/// How far below the top of a laid-out line its baseline sits.
fn baseline(galley: &egui::Galley) -> f32 {
    galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
        .unwrap_or(galley.size().y * 0.8)
}

impl Canvas for EguiCanvas<'_> {
    fn save(&mut self) {
        self.stack.push(self.state.clone());
    }
    fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.state = state;
        }
    }
    fn translate(&mut self, x: f64, y: f64) {
        self.state.offset.0 += x * self.state.scale.0;
        self.state.offset.1 += y * self.state.scale.1;
    }
    fn scale(&mut self, x: f64, y: f64) {
        self.state.scale.0 *= x;
        self.state.scale.1 *= y;
    }
    fn new_path(&mut self) {
        self.paths.clear();
        self.current = None;
    }
    fn new_sub_path(&mut self) {
        self.current = None;
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.paths.push((sub_path(self.map(x, y)), false));
        self.current = Some((x, y));
    }
    fn line_to(&mut self, x: f64, y: f64) {
        if self.current.is_none() {
            self.move_to(x, y);
        } else {
            self.push_point(x, y);
        }
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
        if let Some((points, closed)) = self.paths.last_mut() {
            *closed = true;
            let start = points.first().copied();
            self.current = start.map(|p| self.unmap(p));
        }
    }
    fn set_source_rgba(&mut self, r: f64, g: f64, b: f64, a: f64) {
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.state.color = Color32::from_rgba_unmultiplied(byte(r), byte(g), byte(b), byte(a));
    }
    fn set_line_width(&mut self, width: f64) {
        self.state.width = width;
    }
    fn set_dash(&mut self, dashes: &[f64], offset: f64) {
        // Cairo turns off dashing for an empty pattern and refuses one that is all zero
        // or has a negative length.
        let usable = !dashes.is_empty()
            && dashes.iter().all(|d| *d >= 0.0 && d.is_finite())
            && dashes.iter().any(|d| *d > 0.0);
        self.state.dash = usable.then(|| {
            (
                dashes.to_vec(),
                if offset.is_finite() { offset } else { 0.0 },
            )
        });
    }
    fn set_line_cap(&mut self, cap: LineCap) {
        self.state.cap = cap;
    }
    fn set_line_join(&mut self, join: LineJoin) {
        self.state.join = join;
    }
    fn set_fill_rule(&mut self, rule: FillRule) {
        self.state.fill_rule = rule;
    }
    fn stroke_preserve(&mut self) {
        let paths = self.paths.clone();
        self.stroke_paths(paths);
    }
    fn stroke_segments(
        &mut self,
        segments: &mut dyn Iterator<Item = (geometry_math::vec::Vec2d, geometry_math::vec::Vec2d)>,
    ) {
        let width = self.stroke_width();
        if self.capture.is_some()
            && self.paths.is_empty()
            && self.state.dash.is_none()
            && self.state.cap == LineCap::Butt
            && width <= 2.5 / self.pixels_per_point
        {
            for (from, to) in segments {
                self.capture_segment(
                    self.map(from.x(), from.y()),
                    self.map(to.x(), to.y()),
                    width,
                    self.state.color,
                );
            }
            self.new_path();
        } else {
            for (from, to) in segments {
                self.move_to(from.x(), from.y());
                self.line_to(to.x(), to.y());
            }
            self.stroke();
        }
    }
    fn stroke(&mut self) {
        // The path is not kept, so it is handed over rather than copied.
        let paths = std::mem::take(&mut self.paths);
        self.stroke_paths(paths);
        self.new_path();
    }
    fn fill_preserve(&mut self) {
        let rings: Vec<&[Pos2]> = self
            .paths
            .iter()
            .map(|(points, _)| points.as_slice())
            .filter(|p| p.len() >= 3)
            .collect();
        if rings.is_empty() {
            return;
        }
        let color = self.state.color;
        if rings.len() == 1 && is_convex(rings[0]) {
            self.emit_one(Shape::convex_polygon(
                rings[0].to_vec(),
                color,
                Stroke::NONE,
            ));
            return;
        }
        self.emit_one(Shape::mesh(fill_mesh(&rings, self.state.fill_rule, color)));
        // The mesh has hard edges; an opaque fill gets them smoothed by a hairline of its
        // own colour, which the feathered line draws anti-aliased.
        if color.a() == 255 {
            for ring in rings {
                let mut points = ring.to_vec();
                points.push(ring[0]);
                self.emit_one(Shape::line(points, Stroke::new(0.6, color)));
            }
        }
    }
    fn paint(&mut self) {
        self.emit_one(Shape::rect_filled(self.state.clip, 0.0, self.state.color));
    }
    fn clip(&mut self) {
        let mut bounds = Rect::NOTHING;
        for (points, _) in &self.paths {
            for p in points {
                bounds.extend_with(*p);
            }
        }
        // Exact for a single axis-aligned rectangle, which is every clip the core makes;
        // any other shape clips to its bounds, the nearest egui's rectangle clip comes.
        self.state.clip = self.state.clip.intersect(bounds);
        self.new_path();
    }
    fn paint_tiled_image(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
        origin: (f64, f64),
        scale: f64,
    ) {
        // The upload is kept in egui's memory under the key, with a signature of the
        // pixels, so a material is sent to the GPU once rather than every frame, and again
        // when it is reloaded with new pixels under the same name and size.
        let ctx = self.painter.ctx().clone();
        let id = egui::Id::new(("tiled-image", key));
        let signature = pixel_signature(width, height, pixels);
        let cached = ctx.data_mut(|d| d.get_temp::<(u64, egui::TextureHandle)>(id));
        let texture = match cached {
            Some((seen, texture)) if seen == signature => texture,
            _ => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    pixels,
                );
                let options = egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    wrap_mode: egui::TextureWrapMode::Repeat,
                    ..Default::default()
                };
                let texture = ctx.load_texture(key, image, options);
                ctx.data_mut(|d| d.insert_temp(id, (signature, texture.clone())));
                texture
            }
        };
        let clip = self.state.clip;
        let at = self.map(origin.0, origin.1);
        let tile = egui::vec2(
            (f64::from(width) * scale * self.state.scale.0) as f32,
            (f64::from(height) * scale * self.state.scale.1) as f32,
        );
        if tile.x.abs() < 1e-3 || tile.y.abs() < 1e-3 {
            return;
        }
        let uv = |p: Pos2| Pos2::new((p.x - at.x) / tile.x, (p.y - at.y) / tile.y);
        let mut mesh = egui::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            clip,
            Rect::from_min_max(uv(clip.min), uv(clip.max)),
            Color32::WHITE,
        );
        self.emit_one(Shape::mesh(mesh));
    }
    fn clip_extents(&mut self) -> Option<(f64, f64, f64, f64)> {
        let (x0, y0) = self.unmap(self.state.clip.min);
        let (x1, y1) = self.unmap(self.state.clip.max);
        Some((x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
    }
    fn set_font_bold(&mut self, bold: bool) {
        self.state.bold = bold;
    }
    fn show_text(&mut self, text: &str, size: f64) {
        let Some((x, y)) = self.current else { return };
        let galley = self.galley(text, size * self.state.scale.1.abs());
        let at = self.map(x, y) - egui::vec2(0.0, baseline(&galley));
        let width = galley.size().x;
        self.emit_one(Shape::galley(at, galley, self.state.color));
        self.current = Some((x + f64::from(width) / self.state.scale.0, y));
        self.paths.clear();
    }
    fn show_text_outlined(&mut self, text: &str, size: f64, outline: [f64; 4], width: f64) {
        let Some((x, y)) = self.current else { return };
        let fill = self.state.color;
        self.set_source_rgba(outline[0], outline[1], outline[2], outline[3]);
        let halo = self.galley(text, size * self.state.scale.1.abs());
        let at = self.map(x, y) - egui::vec2(0.0, baseline(&halo));
        // Cairo strokes the glyph outlines `width` wide, so the halo reaches half of that
        // beyond every edge. Copies of the glyphs stamped round a circle of that radius,
        // and round a smaller one to fill the gaps, give the same band.
        for offset in halo_offsets((width * self.mean_scale() / 2.0) as f32) {
            self.emit_one(Shape::galley(at + offset, halo.clone(), self.state.color));
        }
        self.state.color = fill;
        self.show_text(text, size);
    }
    fn text_extents(&mut self, text: &str, size: f64) -> TextExtents {
        let galley = self.galley(text, size);
        let logical = galley.size();
        let base = f64::from(baseline(&galley));
        // The ink is where the glyphs' meshes are, relative to the layout's top left;
        // blank text has none, as Cairo reports.
        let ink = galley.mesh_bounds;
        let (x_bearing, y_bearing, width, height) = if ink.is_positive() {
            (
                f64::from(ink.min.x),
                f64::from(ink.min.y) - base,
                f64::from(ink.width()),
                f64::from(ink.height()),
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        TextExtents {
            x_bearing,
            y_bearing,
            width,
            height,
            x_advance: f64::from(logical.x),
            line_height: f64::from(logical.y),
            baseline: base,
        }
    }
}

/// Where to stamp halo copies of text for an outline reaching `reach` pixels: sixteen
/// directions on the full radius, and eight on half of it when the band is wide enough to
/// show a gap.
fn halo_offsets(reach: f32) -> Vec<egui::Vec2> {
    let reach = reach.max(0.5);
    let ring = |radius: f32, count: usize| {
        (0..count).map(move |i| {
            let angle = std::f32::consts::TAU * i as f32 / count as f32;
            egui::vec2(angle.cos(), angle.sin()) * radius
        })
    };
    let mut offsets: Vec<egui::Vec2> = ring(reach, 16).collect();
    if reach > 1.0 {
        offsets.extend(ring(reach / 2.0, 8));
    }
    offsets
}

/// A signature of an image's pixels: its size, where they live, and a hash of them (every
/// byte for an image up to a megabyte, an even sample of a larger one).
fn pixel_signature(width: u32, height: u32, pixels: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (width, height, pixels.len(), pixels.as_ptr() as usize).hash(&mut hasher);
    if pixels.len() <= 1 << 20 {
        pixels.hash(&mut hasher);
    } else {
        let stride = (pixels.len() / 65_536).max(1);
        for at in (0..pixels.len()).step_by(stride) {
            pixels[at].hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Cuts a polyline into the pieces a dash pattern keeps on, Cairo's way: the pattern
/// repeats (an odd-length one is read twice, on and off swapping), and starts `offset`
/// into itself.
pub(crate) fn dash_polyline(points: &[Pos2], pattern: &[f32], offset: f32) -> Vec<Vec<Pos2>> {
    let mut pattern: Vec<f32> = pattern.iter().map(|d| d.max(0.0)).collect();
    if pattern.len() % 2 == 1 {
        pattern.extend_from_within(..);
    }
    let period: f32 = pattern.iter().sum();
    if pattern.is_empty() || period <= 1e-4 || points.len() < 2 {
        return vec![points.to_vec()];
    }
    // Where in the pattern the line starts.
    let mut index = 0;
    let mut left = pattern[0];
    let mut skip = offset.rem_euclid(period);
    while skip > 0.0 {
        if skip >= left {
            skip -= left;
            index = (index + 1) % pattern.len();
            left = pattern[index];
        } else {
            left -= skip;
            skip = 0.0;
        }
    }
    let mut pieces = Vec::new();
    let mut piece: Vec<Pos2> = Vec::new();
    let on = |index: usize| index.is_multiple_of(2);
    if on(index) {
        piece.push(points[0]);
    }
    for pair in points.windows(2) {
        let (mut from, to) = (pair[0], pair[1]);
        let mut length = from.distance(to);
        while length > 0.0 {
            if left > length {
                left -= length;
                if on(index) {
                    piece.push(to);
                }
                length = 0.0;
            } else {
                let at = from + (to - from) * (left / length);
                if on(index) {
                    piece.push(at);
                    if piece.len() >= 2 {
                        pieces.push(std::mem::take(&mut piece));
                    } else {
                        piece.clear();
                    }
                } else {
                    piece.push(at);
                }
                length -= left;
                from = at;
                index = (index + 1) % pattern.len();
                left = pattern[index];
            }
        }
    }
    if on(index) && piece.len() >= 2 {
        pieces.push(piece);
    }
    pieces
}

/// A mesh covering what `rings` enclose under `rule`, built from the trapezoids between
/// consecutive edge crossings on every band of the scanline sweep. Handles holes,
/// several outlines and self-crossing rings alike.
pub(crate) fn fill_mesh(rings: &[&[Pos2]], rule: FillRule, color: Color32) -> egui::Mesh {
    struct Edge {
        top: Pos2,
        bottom: Pos2,
        winding: i32,
    }
    impl Edge {
        fn x_at(&self, y: f32) -> f32 {
            let span = self.bottom.y - self.top.y;
            self.top.x + (self.bottom.x - self.top.x) * ((y - self.top.y) / span)
        }
    }
    let mut edges = Vec::new();
    for ring in rings {
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            if !(a.is_finite() && b.is_finite()) || (a.y - b.y).abs() < 1e-6 {
                continue;
            }
            let (top, bottom, winding) = if a.y < b.y { (a, b, 1) } else { (b, a, -1) };
            edges.push(Edge {
                top,
                bottom,
                winding,
            });
        }
    }
    // The bands run between every vertex height and every height where two edges cross,
    // so inside a band the edges keep their order.
    let mut ys: Vec<f32> = edges.iter().flat_map(|e| [e.top.y, e.bottom.y]).collect();
    if edges.len() <= 1500 {
        for i in 0..edges.len() {
            for j in i + 1..edges.len() {
                let (p, q) = (&edges[i], &edges[j]);
                let low = p.top.y.max(q.top.y);
                let high = p.bottom.y.min(q.bottom.y);
                if high - low <= 1e-4 {
                    continue;
                }
                let (d0, d1) = (p.x_at(low) - q.x_at(low), p.x_at(high) - q.x_at(high));
                if d0 * d1 < 0.0 {
                    ys.push(low + (high - low) * (d0 / (d0 - d1)));
                }
            }
        }
    }
    ys.sort_by(f32::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    edges.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));

    let mut mesh = egui::Mesh::default();
    let mut active: Vec<usize> = Vec::new();
    let mut next = 0;
    let mut crossings: Vec<(f32, f32, f32, i32)> = Vec::new();
    for band in ys.windows(2) {
        let (y0, y1) = (band[0], band[1]);
        if y1 - y0 < 1e-4 {
            continue;
        }
        let middle = (y0 + y1) / 2.0;
        while next < edges.len() && edges[next].top.y < middle {
            active.push(next);
            next += 1;
        }
        active.retain(|&i| edges[i].bottom.y > middle);
        crossings.clear();
        crossings.extend(active.iter().map(|&i| {
            let e = &edges[i];
            (e.x_at(middle), e.x_at(y0), e.x_at(y1), e.winding)
        }));
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0;
        let mut left: Option<(f32, f32)> = None;
        for &(_, x0, x1, w) in &crossings {
            winding += w;
            let inside = match rule {
                FillRule::Winding => winding != 0,
                FillRule::EvenOdd => winding % 2 != 0,
            };
            match (inside, left) {
                (true, None) => left = Some((x0, x1)),
                (false, Some((l0, l1))) => {
                    let base = mesh.vertices.len() as u32;
                    for p in [
                        Pos2::new(l0, y0),
                        Pos2::new(x0, y0),
                        Pos2::new(x1, y1),
                        Pos2::new(l1, y1),
                    ] {
                        mesh.colored_vertex(p, color);
                    }
                    mesh.add_triangle(base, base + 1, base + 2);
                    mesh.add_triangle(base, base + 2, base + 3);
                    left = None;
                }
                _ => {}
            }
        }
    }
    mesh
}

fn cross(o: Pos2, a: Pos2, b: Pos2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn is_convex(points: &[Pos2]) -> bool {
    let n = points.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let c = cross(points[i], points[(i + 1) % n], points[(i + 2) % n]);
        if c.abs() > 1e-6 {
            if sign == 0.0 {
                sign = c.signum();
            } else if c.signum() != sign {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a point is covered by one of the mesh's triangles.
    fn covered(mesh: &egui::Mesh, p: Pos2) -> bool {
        mesh.indices.chunks(3).any(|t| {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[t[i] as usize].pos);
            let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
            let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
            let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
            !(negative && positive)
        })
    }

    fn square(c: f32, half: f32, clockwise: bool) -> Vec<Pos2> {
        let mut ring = vec![
            Pos2::new(c - half, c - half),
            Pos2::new(c + half, c - half),
            Pos2::new(c + half, c + half),
            Pos2::new(c - half, c + half),
        ];
        if !clockwise {
            ring.reverse();
        }
        ring
    }

    #[test]
    fn even_odd_fill_of_a_ring_leaves_the_hole_empty() {
        let (outer, inner) = (square(50.0, 40.0, true), square(50.0, 15.0, true));
        let mesh = fill_mesh(&[&outer, &inner], FillRule::EvenOdd, Color32::WHITE);
        assert!(
            covered(&mesh, Pos2::new(15.0, 50.0)),
            "the band of the ring is filled"
        );
        assert!(covered(&mesh, Pos2::new(50.0, 80.0)));
        assert!(
            !covered(&mesh, Pos2::new(50.0, 50.0)),
            "the hole stays open"
        );
        assert!(!covered(&mesh, Pos2::new(95.0, 50.0)), "nothing outside");
    }

    #[test]
    fn nonzero_fill_needs_the_hole_wound_the_other_way() {
        let (outer, same, opposite) = (
            square(50.0, 40.0, true),
            square(50.0, 15.0, true),
            square(50.0, 15.0, false),
        );
        let filled = fill_mesh(&[&outer, &same], FillRule::Winding, Color32::WHITE);
        assert!(covered(&filled, Pos2::new(50.0, 50.0)));
        let holed = fill_mesh(&[&outer, &opposite], FillRule::Winding, Color32::WHITE);
        assert!(!covered(&holed, Pos2::new(50.0, 50.0)));
        assert!(covered(&holed, Pos2::new(15.0, 50.0)));
    }

    #[test]
    fn a_self_crossing_star_follows_the_rule() {
        // A pentagram: its centre is wound twice.
        let star: Vec<Pos2> = (0..5)
            .map(|i| {
                let angle =
                    std::f32::consts::TAU * (i as f32 * 2.0) / 5.0 - std::f32::consts::FRAC_PI_2;
                Pos2::new(50.0 + 40.0 * angle.cos(), 50.0 + 40.0 * angle.sin())
            })
            .collect();
        assert!(!covered(
            &fill_mesh(&[&star], FillRule::EvenOdd, Color32::WHITE),
            Pos2::new(50.0, 52.0)
        ));
        assert!(covered(
            &fill_mesh(&[&star], FillRule::Winding, Color32::WHITE),
            Pos2::new(50.0, 52.0)
        ));
    }

    #[test]
    fn dashes_use_the_whole_pattern_and_the_offset() {
        let line = [Pos2::new(0.0, 0.0), Pos2::new(20.0, 0.0)];
        let pieces = dash_polyline(&line, &[4.0, 2.0, 1.0, 3.0], 0.0);
        let spans: Vec<(f32, f32)> = pieces.iter().map(|p| (p[0].x, p[p.len() - 1].x)).collect();
        assert_eq!(
            spans,
            vec![(0.0, 4.0), (6.0, 7.0), (10.0, 14.0), (16.0, 17.0)]
        );
        let shifted = dash_polyline(&line, &[4.0, 2.0], 3.0);
        assert_eq!(
            (shifted[0][0].x, shifted[0][1].x),
            (0.0, 1.0),
            "three units into the first dash"
        );
        assert_eq!(shifted[1][0].x, 3.0);
        // An odd pattern repeats with on and off swapped.
        let odd = dash_polyline(&line, &[3.0], 0.0);
        assert_eq!(odd.len(), 4);
        assert_eq!((odd[1][0].x, odd[1][1].x), (6.0, 9.0));
    }

    #[test]
    fn dashes_continue_round_corners() {
        let corner = [
            Pos2::new(0.0, 0.0),
            Pos2::new(3.0, 0.0),
            Pos2::new(3.0, 3.0),
        ];
        let pieces = dash_polyline(&corner, &[5.0, 1.0], 0.0);
        assert_eq!(
            pieces[0],
            vec![
                Pos2::new(0.0, 0.0),
                Pos2::new(3.0, 0.0),
                Pos2::new(3.0, 2.0)
            ]
        );
    }

    #[test]
    fn the_halo_surrounds_the_text_evenly() {
        let offsets = halo_offsets(2.0);
        assert_eq!(offsets.len(), 24);
        let longest = offsets.iter().map(|o| o.length()).fold(0.0, f32::max);
        assert!((longest - 2.0).abs() < 1e-4);
        let sum = offsets.iter().fold(egui::Vec2::ZERO, |a, b| a + *b);
        assert!(sum.length() < 1e-3, "no side is favoured");
    }

    #[test]
    fn a_reloaded_material_has_a_new_signature() {
        let a = vec![10u8; 16];
        let mut b = a.clone();
        b[5] = 11;
        assert_ne!(pixel_signature(2, 2, &a), pixel_signature(2, 2, &b));
    }

    #[test]
    fn text_extents_report_the_ink_not_the_line() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter().clone();
            let mut canvas = EguiCanvas::new(
                &painter,
                Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0)),
            );
            let dot = canvas.text_extents(".", 20.0);
            let line = canvas.text_extents("Hg", 20.0);
            assert!(
                dot.height < line.height,
                "a full stop is inked lower than a line"
            );
            assert!(
                dot.y_bearing > line.y_bearing,
                "and starts nearer the baseline"
            );
            assert!(line.y_bearing < 0.0);
            assert!(line.width <= line.x_advance + 1.0);
            let blank = canvas.text_extents(" ", 20.0);
            assert_eq!(blank.width, 0.0);
            assert!(blank.x_advance > 0.0);
        });
        output.textures_delta.clear();
    }
}

#[cfg(test)]
mod streamed_segment_tests {
    use super::*;
    use geometry_math::vec::Vec2d;
    #[test]
    fn streamed_segments_match_legacy_meshes_including_style_fallbacks() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::new(20., 10.), egui::vec2(640., 480.));
        let segments = [
            (Vec2d::new(4., 8.), Vec2d::new(140., 12.)),
            (Vec2d::new(140., 12.), Vec2d::new(4., 8.)),
            (Vec2d::new(10., 20.), Vec2d::new(10.01, 20.)),
            (Vec2d::new(-4., 30.), Vec2d::new(64., 40.)),
            (Vec2d::zero(), Vec2d::zero()),
        ];
        for ppp in [1., 2.] {
            ctx.set_pixels_per_point(ppp);
            for width in [0.3, 1., 2., 4.] {
                for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
                    for dashed in [false, true] {
                        for pending in [false, true] {
                            let render = |streamed| {
                                let mut meshes = Vec::new();
                                let mut output = ctx.run_ui(
                                    egui::RawInput {
                                        screen_rect: Some(rect),
                                        ..Default::default()
                                    },
                                    |ui| {
                                        let painter = ui.painter_at(rect);
                                        let mut canvas = EguiCanvas::capturing(&painter, rect);
                                        canvas.translate(3.5, -2.25);
                                        canvas.scale(0.8, 1.2);
                                        canvas.set_source_rgba(0.2, 0.4, 0.8, 0.6);
                                        canvas.set_line_width(width);
                                        canvas.set_line_cap(cap);
                                        if dashed {
                                            canvas.set_dash(&[3., 2.], 1.5);
                                        }
                                        if pending {
                                            canvas.move_to(4., 10.);
                                            canvas.line_to(40., 10.);
                                        }
                                        if streamed {
                                            canvas.stroke_segments(&mut segments.into_iter());
                                        } else {
                                            for (a, b) in segments {
                                                canvas.move_to(a.x(), a.y());
                                                canvas.line_to(b.x(), b.y());
                                            }
                                            canvas.stroke();
                                        }
                                        assert!(canvas.paths.is_empty());
                                        assert!(canvas.current.is_none());
                                        meshes = canvas.into_capture().into_meshes(
                                            &ctx,
                                            rect,
                                            ctx.pixels_per_point(),
                                        );
                                    },
                                );
                                output.textures_delta.clear();
                                meshes
                            };
                            let (legacy, streamed) = (render(false), render(true));
                            assert_eq!(legacy.len(),streamed.len(),"ppp {ppp}, width {width}, cap {cap:?}, dash {dashed}, pending {pending}");
                            for (a, b) in legacy.iter().zip(&streamed) {
                                assert_eq!(a.vertices, b.vertices);
                                assert_eq!(a.indices, b.indices);
                                assert_eq!(a.texture_id, b.texture_id);
                            }
                        }
                    }
                }
            }
        }
    }
}
