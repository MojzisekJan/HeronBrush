//! Timeline and curve interaction. A drag edits a draft and commits once on release.
use crate::canvas::Canvas;
use crate::{
    animation_curves::{Clip, Interpolation, CHANNELS},
    i18n::tr,
};
use std::collections::BTreeSet;
/// The classic label column's width.
pub const LEFT: f64 = 132.0;
#[cfg(test)]
const TOP: f64 = 28.0;

/// Where the timeline's parts sit: the drawing and the pointer's hit tests both use it,
/// so a key is picked where it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineGeometry {
    /// The label column's width; the time axis starts here.
    pub left: f64,
    /// The ruler's height; the lanes start below it.
    pub top: f64,
    /// Space between the label column and the visible span's start, so keys and the
    /// playhead at the first frame are drawn whole.
    pub pad: f64,
    /// Space kept after the visible span's end.
    pub right: f64,
    /// Space kept under the last lane.
    pub bottom: f64,
    /// Space between the ruler and the curve view's highest value.
    pub curve_top: f64,
    /// Space between the curve view's lowest value and the bottom edge.
    pub curve_bottom: f64,
}
impl TimelineGeometry {
    /// The plain layout: a label column, a ruler along the top and even margins.
    pub const CLASSIC: Self = Self {
        left: LEFT,
        top: 28.0,
        pad: 0.0,
        right: 12.0,
        bottom: 12.0,
        curve_top: 12.0,
        curve_bottom: 20.0,
    };
}
impl TimelineGeometry {
    /// Where the visible span starts.
    fn axis(&self) -> f64 {
        self.left + self.pad
    }
    /// How wide the visible span is in a `w` wide timeline.
    fn axis_width(&self, w: f64) -> f64 {
        (w - self.left - self.pad - self.right).max(1.0)
    }
}
impl Default for TimelineGeometry {
    fn default() -> Self {
        Self::CLASSIC
    }
}
#[derive(Clone)]
enum Drag {
    Keys {
        before: Clip,
        anchor: f64,
    },
    Handle {
        before: Clip,
        id: u64,
        incoming: bool,
    },
    Select {
        x: f64,
        y: f64,
        base: BTreeSet<u64>,
        end: (f64, f64),
    },
    Scrub,
    Pan {
        start: f64,
    },
}
pub struct State {
    pub clip: Clip,
    pub selected: BTreeSet<u64>,
    pub active: usize,
    pub frame: f64,
    pub curves: bool,
    pub start: f64,
    pub span: f64,
    pub low: f64,
    pub high: f64,
    pub snap: bool,
    pub mode: String,
    pub pivot: [f64; 3],
    /// The layout the pointer is hit-tested against: set it to the drawing style's
    /// geometry.
    pub geometry: TimelineGeometry,
    drag: Option<Drag>,
    pointer_start: (f64, f64),
}
impl State {
    pub fn new(clip: Clip, mode: String, pivot: [f64; 3]) -> Self {
        let span = clip.duration * 1.1;
        let mut state = Self {
            clip,
            selected: BTreeSet::new(),
            active: 0,
            frame: 0.0,
            curves: false,
            start: 0.0,
            span,
            low: -1.0,
            high: 1.0,
            snap: true,
            mode,
            pivot,
            geometry: TimelineGeometry::CLASSIC,
            drag: None,
            pointer_start: (0.0, 0.0),
        };
        state.fit_values();
        state
    }
    pub fn position_time(&self) -> Option<f64> {
        if self.curves || self.selected.is_empty() {
            return None;
        }
        let mut times = self.selected.iter().filter_map(|id| self.clip.key(*id));
        let (channel, key) = times.next()?;
        if channel >= 3 {
            return None;
        }
        times
            .all(|(c, k)| c < 3 && (k.time - key.time).abs() < 1e-6)
            .then_some(key.time)
    }
    pub fn key_group(&self, channel: usize, id: u64) -> BTreeSet<u64> {
        if self.curves || channel >= 3 {
            return BTreeSet::from([id]);
        }
        let Some((_, key)) = self.clip.key(id) else {
            return BTreeSet::new();
        };
        self.clip.tracks[..3]
            .iter()
            .flatten()
            .filter(|k| (k.time - key.time).abs() < 1e-6)
            .map(|k| k.id)
            .collect()
    }
    pub fn group_selection(&mut self) {
        self.selected = self
            .selected
            .iter()
            .filter_map(|id| self.clip.key(*id).map(|(c, _)| (c, *id)))
            .flat_map(|(c, id)| self.key_group(c, id))
            .collect();
    }
    pub fn insert_key(&mut self, channel: usize, time: f64, value: f64) -> Result<(), String> {
        self.selected = if channel < 3 && !self.curves {
            let mut position = std::array::from_fn(|axis| self.clip.sample(axis, time));
            position[channel] = value;
            self.clip.insert_position(time, position)?
        } else {
            BTreeSet::from([self.clip.insert(channel, time, value)?])
        };
        self.active = channel;
        Ok(())
    }
    fn lane_count(&self) -> f64 {
        if self.curves {
            6.0
        } else {
            4.0
        }
    }
    fn row(&self, channel: usize) -> usize {
        if self.curves {
            channel
        } else if channel < 3 {
            0
        } else {
            channel - 2
        }
    }
    pub fn fit_values(&mut self) {
        let mut low = 0.0_f64;
        let mut high = 0.0_f64;
        for i in 0..=256 {
            let value = self
                .clip
                .sample(self.active, self.clip.duration * i as f64 / 256.0);
            low = low.min(value);
            high = high.max(value);
        }
        let padding = ((high - low) * 0.15).max(1.0);
        self.low = low - padding;
        self.high = high + padding;
    }
    pub fn zoom(&mut self, factor: f64) {
        let center = self.start + self.span * 0.5;
        self.span = (self.span * factor).clamp(2.0 / self.clip.fps as f64, 100000.0);
        self.start = (center - self.span * 0.5).max(0.0);
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn cancel(&mut self) -> bool {
        match self.drag.take() {
            Some(Drag::Keys { before, .. } | Drag::Handle { before, .. }) => {
                self.clip = before;
                true
            }
            Some(_) => true,
            None => false,
        }
    }
    fn time(&self, x: f64, w: f64) -> f64 {
        let g = &self.geometry;
        let time = (self.start + (x - g.axis()) / g.axis_width(w) * self.span)
            .clamp(0.0, self.clip.duration);
        if self.snap {
            ((time * self.clip.fps as f64).round() / self.clip.fps as f64)
                .clamp(0.0, self.clip.duration)
        } else {
            time
        }
    }
    /// How many seconds `dx` pixels along the time axis span.
    fn seconds(&self, dx: f64, w: f64) -> f64 {
        let g = &self.geometry;
        dx / g.axis_width(w) * self.span
    }
    fn x(&self, time: f64, w: f64) -> f64 {
        self.x_in(&self.geometry, time, w)
    }
    fn x_in(&self, g: &TimelineGeometry, time: f64, w: f64) -> f64 {
        g.axis() + (time - self.start) / self.span * g.axis_width(w)
    }
    fn y(&self, channel: usize, value: f64, h: f64) -> f64 {
        self.y_in(&self.geometry, channel, value, h)
    }
    /// The height the curve view's value range spans.
    fn value_height(&self, g: &TimelineGeometry, h: f64) -> f64 {
        (h - g.top - g.curve_top - g.curve_bottom).max(1.0)
    }
    /// The value at height `y` of the curve view.
    fn value_at(&self, y: f64, h: f64) -> f64 {
        let g = &self.geometry;
        self.high - (y - g.top - g.curve_top) / self.value_height(g, h) * (self.high - self.low)
    }
    /// The height of one lane of the key view, or of one label row.
    fn lane_height(&self, g: &TimelineGeometry, h: f64) -> f64 {
        (h - g.top - g.bottom) / self.lane_count()
    }
    fn y_in(&self, g: &TimelineGeometry, channel: usize, value: f64, h: f64) -> f64 {
        if self.curves {
            g.top
                + g.curve_top
                + (self.high - value) / (self.high - self.low) * self.value_height(g, h)
        } else {
            g.top + (self.row(channel) as f64 + 0.5) * self.lane_height(g, h)
        }
    }
    fn lane(&self, y: f64, h: f64) -> usize {
        let g = &self.geometry;
        let row = (((y - g.top) / (h - g.top - g.bottom) * self.lane_count()).floor() as usize)
            .min(self.lane_count() as usize - 1);
        if self.curves || row == 0 {
            row
        } else {
            row + 2
        }
    }
    fn handles(&self, g: &TimelineGeometry, w: f64, h: f64) -> Vec<(u64, bool, f64, f64)> {
        let mut result = Vec::new();
        if !self.curves {
            return result;
        }
        let keys = &self.clip.tracks[self.active];
        for (i, key) in keys
            .iter()
            .enumerate()
            .filter(|(_, k)| self.selected.contains(&k.id))
        {
            if i > 0 && keys[i - 1].interpolation == Interpolation::Bezier {
                let dt = (key.time - keys[i - 1].time) / 3.0;
                result.push((
                    key.id,
                    true,
                    self.x_in(g, key.time - dt, w),
                    self.y_in(g, self.active, key.value - key.incoming * dt, h),
                ));
            }
            if key.interpolation == Interpolation::Bezier {
                if let Some(next) = keys.get(i + 1) {
                    let dt = (next.time - key.time) / 3.0;
                    result.push((
                        key.id,
                        false,
                        self.x_in(g, key.time + dt, w),
                        self.y_in(g, self.active, key.value + key.outgoing * dt, h),
                    ));
                }
            }
        }
        result
    }
}
/// A colour, red, green, blue and alpha from 0 to 1.
pub type Rgba = [f64; 4];

/// How the timeline looks: colours, sizes and which decorations it draws.
/// [`State::draw`] uses the plain [`TimelineStyle::classic`]; a frontend can pass its own
/// through [`State::draw_styled`], and set [`State::geometry`] to the style's geometry so the
/// pointer picks what is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineStyle {
    /// Where the label column, the ruler and the lanes sit.
    pub geometry: TimelineGeometry,
    /// Behind everything.
    pub background: Rgba,
    /// The label column's own fill, if it has one.
    pub label_background: Option<Rgba>,
    /// The ruler band's fill, if it has one.
    pub ruler_background: Option<Rgba>,
    /// Laid over every other row, so rows alternate.
    pub track_alt: Option<Rgba>,
    /// Hairlines between the label column, the ruler and the lanes.
    pub divider: Option<Rgba>,
    /// The active row's highlight in the label column.
    pub active_row: Rgba,
    /// How far short of the time axis that highlight stops.
    pub active_row_inset: f64,
    /// A bar at the active row's left edge.
    pub active_bar: Option<Rgba>,
    /// Track names.
    pub label_text: Rgba,
    /// The values beside the names.
    pub value_text: Rgba,
    /// Whether the values sit against the time axis rather than after the names.
    pub values_right_aligned: bool,
    /// Size of the label column's text.
    pub font_size: f64,
    /// The label's baseline below the row's centre.
    pub label_baseline: f64,
    /// The colour swatch's top relative to the row's centre; it is 12 high.
    pub swatch_top: f64,
    /// Whether the position row's swatch shows the three axes' colours.
    pub position_swatch_split: bool,
    /// Frame numbers on the ruler.
    pub ruler_text: Rgba,
    /// Their size.
    pub ruler_font_size: f64,
    /// Their baseline from the top edge.
    pub ruler_baseline: f64,
    /// Tick marks along the ruler's lower edge, if drawn.
    pub ruler_tick: Option<Rgba>,
    /// Whether the labelled steps are whole frames (1, 2, 5, 10 …) rather than seconds.
    pub frame_steps: bool,
    /// The least room between two labelled steps, in pixels.
    pub tick_spacing: f64,
    /// Lines at the labelled steps.
    pub grid_major: Rgba,
    /// Lines between them, where there is room.
    pub grid_minor: Option<Rgba>,
    /// Over the time after the clip's end.
    pub range_shade: Rgba,
    /// Whether that shade covers the ruler too.
    pub range_shade_ruler: bool,
    /// A line at the clip's end.
    pub range_edge: Option<Rgba>,
    /// The curve view's value lines.
    pub value_grid: Rgba,
    /// Their labels.
    pub value_grid_text: Rgba,
    /// A line at value zero in the curve view.
    pub zero_line: Option<Rgba>,
    /// Each channel's colour: X, Y, Z, then the three rotations.
    pub curve_colours: [Rgba; 6],
    /// The curve's stroke width.
    pub curve_width: f64,
    /// Bezier handles and their lines.
    pub handle: Rgba,
    /// A position key in the key view, which stands for all three axes; without it the
    /// key has its axis's colour.
    pub position_key: Option<Rgba>,
    /// A key's half-diagonal.
    pub key_size: f64,
    /// A selected key.
    pub key_selected: Rgba,
    /// A key's outline, if it has one.
    pub key_outline: Option<Rgba>,
    /// A selected key's outline, where keys have outlines.
    pub key_selected_outline: Option<Rgba>,
    /// Inside the rubber band.
    pub selection_fill: Rgba,
    /// Its border.
    pub selection_stroke: Rgba,
    /// The playhead line.
    pub playhead: Rgba,
    /// The frame number's colour in a flag on the ruler; without it the playhead has a
    /// small diamond on top.
    pub playhead_flag: Option<Rgba>,
    /// Whether 1-pixel lines are set on pixel centres, so they stay sharp.
    pub crisp: bool,
}

impl TimelineStyle {
    /// The plain look, flat and undecorated, for text in `fg`.
    pub fn classic(fg: [f64; 3]) -> Self {
        let dark = fg.iter().sum::<f64>() > 1.5;
        let bg = if dark { 0.12 } else { 0.94 };
        let [red, green, blue] = [
            [0.9, 0.3, 0.3, 1.0],
            [0.25, 0.7, 0.35, 1.0],
            [0.25, 0.55, 0.95, 1.0],
        ];
        let text = [fg[0], fg[1], fg[2], 0.85];
        Self {
            geometry: TimelineGeometry::CLASSIC,
            background: [bg, bg, bg, 1.0],
            label_background: None,
            ruler_background: None,
            track_alt: None,
            divider: None,
            active_row: [0.3, 0.55, 0.85, 0.18],
            active_row_inset: 4.0,
            active_bar: None,
            label_text: text,
            value_text: text,
            values_right_aligned: false,
            font_size: 12.0,
            label_baseline: 5.0,
            swatch_top: -5.0,
            position_swatch_split: false,
            ruler_text: text,
            ruler_font_size: 12.0,
            ruler_baseline: 17.0,
            ruler_tick: None,
            frame_steps: false,
            tick_spacing: 85.0,
            grid_major: [fg[0], fg[1], fg[2], 0.12],
            grid_minor: None,
            range_shade: [0.5, 0.5, 0.5, 0.12],
            range_shade_ruler: false,
            range_edge: None,
            value_grid: [0.5, 0.5, 0.5, 0.18],
            value_grid_text: text,
            zero_line: None,
            curve_colours: [red, green, blue, red, green, blue],
            curve_width: 2.0,
            handle: [0.6, 0.65, 0.7, 0.9],
            position_key: None,
            key_size: 5.0,
            key_selected: [1.0, 0.7, 0.15, 1.0],
            key_outline: None,
            key_selected_outline: None,
            selection_fill: [0.35, 0.6, 1.0, 0.18],
            selection_stroke: [0.35, 0.6, 1.0, 0.8],
            playhead: [1.0, 0.45, 0.2, 1.0],
            playhead_flag: None,
            crisp: false,
        }
    }
}

fn set(cr: &mut dyn Canvas, c: Rgba) {
    cr.set_source_rgba(c[0], c[1], c[2], c[3]);
}

fn diamond_path(cr: &mut dyn Canvas, x: f64, y: f64, size: f64) {
    cr.move_to(x, y - size);
    cr.line_to(x + size, y);
    cr.line_to(x, y + size);
    cr.line_to(x - size, y);
    cr.close_path();
}

fn diamond(cr: &mut dyn Canvas, x: f64, y: f64, size: f64) {
    diamond_path(cr, x, y, size);
    cr.fill();
}

/// A rectangle with rounded corners.
fn rounded(cr: &mut dyn Canvas, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::{FRAC_PI_2, PI};
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, PI + FRAC_PI_2);
    cr.close_path();
}

/// The ruler's labelled step in seconds, and the step between minor lines if there is one.
fn ruler_steps(s: &State, st: &TimelineStyle, w: f64) -> (f64, Option<f64>) {
    let g = &st.geometry;
    let fps = s.clip.fps as f64;
    let target = s.span / ((w - g.left) / st.tick_spacing).max(1.0);
    if !st.frame_steps {
        let power = 10.0_f64.powf(target.log10().floor());
        let step = [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .map(|n| n * power)
            .find(|n| *n >= target)
            .unwrap_or(power * 10.0)
            .max(1.0 / fps);
        return (step, None);
    }
    let frames = (target * fps).max(1.0);
    let power = 10.0_f64.powf(frames.log10().floor());
    let (base, step) = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|n| (n, n * power))
        .find(|(_, n)| *n >= frames)
        .unwrap_or((1.0, power * 10.0));
    let minor = step / if base == 2.0 { 2.0 } else { 5.0 };
    let minor = (minor >= 1.0 && minor.fract() == 0.0).then_some(minor / fps);
    (step / fps, minor)
}

impl State {
    /// How many pixels a second spans.
    fn pixels_per_second(&self, g: &TimelineGeometry, w: f64) -> f64 {
        g.axis_width(w) / self.span
    }
    /// Draws the timeline or the curve view in the plain look, [`TimelineStyle::classic`].
    pub fn draw(&self, cr: &mut dyn Canvas, w: f64, h: f64, fg: [f64; 3]) {
        self.draw_styled(cr, w, h, &TimelineStyle::classic(fg));
    }

    /// Draws the timeline or the curve view in `st`'s look, `w` by `h` pixels.
    pub fn draw_styled(&self, cr: &mut dyn Canvas, w: f64, h: f64, st: &TimelineStyle) {
        let s = self;
        let g = &st.geometry;
        let px = |v: f64| if st.crisp { v.floor() + 0.5 } else { v };
        set(cr, st.background);
        cr.paint();
        if let Some(c) = st.label_background {
            set(cr, c);
            cr.rectangle(0.0, 0.0, g.left, h);
            cr.fill();
        }
        if let Some(c) = st.ruler_background {
            set(cr, c);
            cr.rectangle(0.0, 0.0, w, g.top);
            cr.fill();
        }
        let lanes = s.lane_count();
        let lane = s.lane_height(g, h);
        if let Some(c) = st.track_alt {
            // The curve view shows one curve over every row, so only the labels alternate.
            let width = if s.curves { g.left } else { w };
            set(cr, c);
            for row in (1..lanes as usize).step_by(2) {
                let top = (g.top + row as f64 * lane).round();
                let bottom = (g.top + (row + 1) as f64 * lane).round();
                cr.rectangle(0.0, top, width, bottom - top);
            }
            cr.fill();
        }
        let colours = &st.curve_colours;
        let text = |cr: &mut dyn Canvas, x: f64, y: f64, label: &str, colour: Rgba, size: f64| {
            set(cr, colour);
            cr.move_to(x, y);
            cr.show_text(label, size);
        };
        for (channel, label) in CHANNELS.iter().enumerate() {
            if !s.curves && (channel == 1 || channel == 2) {
                continue;
            }
            let label = if !s.curves && channel == 0 {
                tr("Position")
            } else {
                *label
            };
            let y = g.top + (s.row(channel) as f64 + 0.5) * (h - g.top - g.bottom) / lanes;
            if s.row(channel) == s.row(s.active) {
                let top = y - (h - g.top - g.bottom) / (lanes * 2.0);
                let height = (h - g.top - g.bottom) / lanes;
                let (top, height) = if st.crisp {
                    (top.round(), (top + height).round() - top.round())
                } else {
                    (top, height)
                };
                set(cr, st.active_row);
                cr.rectangle(0.0, top, g.left - st.active_row_inset, height);
                cr.fill();
                if let Some(c) = st.active_bar {
                    set(cr, c);
                    cr.rectangle(0.0, top, 2.0, height);
                    cr.fill();
                }
            }
            let swatch = if st.crisp {
                (y + st.swatch_top).round()
            } else {
                y + st.swatch_top
            };
            if st.position_swatch_split && !s.curves && channel == 0 {
                for (i, c) in colours[..3].iter().enumerate() {
                    set(cr, *c);
                    cr.rectangle(8.0 + 3.0 * i as f64, swatch, 2.0, 12.0);
                    cr.fill();
                }
            } else {
                set(cr, colours[channel]);
                cr.rectangle(8.0, swatch, 3.0, 12.0);
                cr.fill();
            }
            let baseline = if st.crisp {
                (y + st.label_baseline).round()
            } else {
                y + st.label_baseline
            };
            let label_x = if st.position_swatch_split { 22.0 } else { 18.0 };
            text(cr, label_x, baseline, label, st.label_text, st.font_size);
            if s.curves || channel >= 3 {
                let value = format!("{:.1}", s.clip.sample(channel, s.frame / s.clip.fps as f64));
                let x = if st.values_right_aligned {
                    (g.left - 10.0 - cr.text_extents(&value, st.font_size).x_advance).round()
                } else {
                    52.0
                };
                text(cr, x, baseline, &value, st.value_text, st.font_size);
            }
        }
        if let Some(c) = st.divider {
            set(cr, c);
            cr.set_line_width(1.0);
            cr.move_to(g.left - 0.5, 0.0);
            cr.line_to(g.left - 0.5, h);
            cr.move_to(0.0, g.top - 0.5);
            cr.line_to(w, g.top - 0.5);
            cr.stroke();
        }
        cr.save();
        cr.rectangle(g.left, 0.0, w - g.left, h);
        cr.clip();
        if st.crisp {
            cr.set_line_width(1.0);
        }
        // Adaptive ticks retain readable labels at every zoom level.
        let (step, minor) = ruler_steps(s, st, w);
        if let (Some(minor), Some(colour)) = (minor, st.grid_minor) {
            if minor * s.pixels_per_second(g, w) >= 6.0 {
                let first = (s.start / minor).floor() as i64;
                let per = (step / minor).round() as i64;
                let count = ((s.span / minor).ceil() as i64 + 2).min(2000);
                set(cr, colour);
                for i in (first..=first + count).filter(|i| i % per != 0) {
                    let x = px(s.x_in(g, i as f64 * minor, w));
                    cr.move_to(x, g.top);
                    cr.line_to(x, h);
                }
                cr.stroke();
                if let Some(c) = st.ruler_tick {
                    set(cr, c);
                    for i in (first..=first + count).filter(|i| i % per != 0) {
                        let x = px(s.x_in(g, i as f64 * minor, w));
                        cr.move_to(x, g.top - 4.0);
                        cr.line_to(x, g.top - 1.0);
                    }
                    cr.stroke();
                }
            }
        }
        let playhead = s.x_in(g, s.frame / s.clip.fps as f64, w);
        // The playhead's flag: its frame number, the number's width, the flag's left
        // edge and width.
        let flag = st.playhead_flag.map(|_| {
            let label = format!("{:.0}", s.frame);
            cr.set_font_bold(true);
            let advance = cr.text_extents(&label, st.ruler_font_size).x_advance;
            cr.set_font_bold(false);
            let width = (advance + 10.0).max(18.0).round();
            (label, advance, (px(playhead) - width / 2.0).round(), width)
        });
        let first = (s.start / step).floor() * step;
        for tick in 0..=(((s.span / step).ceil() as usize) + 2).min(500) {
            let t = first + tick as f64 * step;
            let x = s.x_in(g, t, w);
            set(cr, st.grid_major);
            cr.move_to(px(x), g.top);
            cr.line_to(px(x), h);
            cr.stroke();
            if let Some(c) = st.ruler_tick {
                set(cr, c);
                cr.move_to(px(x), g.top - 8.0);
                cr.line_to(px(x), g.top - 1.0);
                cr.stroke();
            }
            let dx = if st.crisp { 4.0 } else { 3.0 };
            let label = format!("{:.0}", t * s.clip.fps as f64);
            let at = (x + dx).round_if(st.crisp);
            if let Some((_, _, left, width)) = &flag {
                // A number the flag would cover is left out rather than shown cut.
                let advance = cr.text_extents(&label, st.ruler_font_size).x_advance;
                if at < left + width + 2.0 && at + advance > left - 2.0 {
                    continue;
                }
            }
            text(
                cr,
                at,
                st.ruler_baseline,
                &label,
                st.ruler_text,
                st.ruler_font_size,
            );
        }
        let end = s.x_in(g, s.clip.duration, w);
        let end = if st.crisp { end.round() } else { end };
        let shade_top = if st.range_shade_ruler { 0.0 } else { g.top };
        set(cr, st.range_shade);
        cr.rectangle(end, shade_top, (w - end).max(0.0), h - shade_top);
        cr.fill();
        if let Some(c) = st.range_edge {
            set(cr, c);
            cr.move_to(end - 0.5, 0.0);
            cr.line_to(end - 0.5, h);
            cr.stroke();
        }
        if s.curves {
            for i in 0..=4 {
                let value = s.low + (s.high - s.low) * i as f64 / 4.0;
                let y = s.y_in(g, s.active, value, h);
                set(cr, st.value_grid);
                cr.move_to(g.left, px(y));
                cr.line_to(w, px(y));
                cr.stroke();
                let label = format!("{value:.1}");
                let x = if st.values_right_aligned {
                    (w - 6.0 - cr.text_extents(&label, st.ruler_font_size).x_advance).round()
                } else {
                    g.left + 4.0
                };
                text(
                    cr,
                    x,
                    (y - 3.0).round_if(st.crisp),
                    &label,
                    st.value_grid_text,
                    st.ruler_font_size,
                );
            }
            if let Some(c) = st.zero_line.filter(|_| s.low < 0.0 && s.high > 0.0) {
                let y = px(s.y_in(g, s.active, 0.0, h));
                set(cr, c);
                cr.move_to(g.left, y);
                cr.line_to(w, y);
                cr.stroke();
            }
            set(cr, colours[s.active]);
            cr.set_line_width(st.curve_width);
            // The curve starts at the clip's start, where padding leaves room before it.
            let first = (s.x_in(g, 0.0, w) - g.left).clamp(0.0, g.pad).floor() as usize;
            for i in first..=(w - g.left).max(0.0) as usize {
                let x = g.left + i as f64;
                let t = s.start + (i as f64 - g.pad) / g.axis_width(w) * s.span;
                let y = s.y_in(g, s.active, s.clip.sample(s.active, t), h);
                if i == first {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.stroke();
            cr.set_line_width(1.0);
            for (id, _, x, y) in s.handles(g, w, h) {
                let (_, key) = s.clip.key(id).unwrap();
                set(cr, st.handle);
                cr.move_to(s.x_in(g, key.time, w), s.y_in(g, s.active, key.value, h));
                cr.line_to(x, y);
                cr.stroke();
                cr.arc(x, y, 4.0, 0.0, std::f64::consts::TAU);
                cr.fill();
                if let Some(c) = st.key_outline {
                    set(cr, c);
                    cr.arc(x, y, 4.0, 0.0, std::f64::consts::TAU);
                    cr.stroke();
                }
            }
        }
        if st.playhead_flag.is_some() {
            // Under the keys, so a key at the playhead stays whole.
            set(cr, st.playhead);
            cr.move_to(px(playhead), g.top - 4.0);
            cr.line_to(px(playhead), h);
            cr.stroke();
        }
        let mut drawn_positions = BTreeSet::new();
        for (channel, keys) in s.clip.tracks.iter().enumerate() {
            if s.curves && channel != s.active {
                continue;
            }
            for key in keys {
                if !s.curves && channel < 3 && !drawn_positions.insert(key.time.to_bits()) {
                    continue;
                }
                let x = s.x_in(g, key.time, w);
                let y = s.y_in(g, channel, key.value, h);
                let (x, y) = if st.crisp { (px(x), px(y)) } else { (x, y) };
                let selected = s
                    .key_group(channel, key.id)
                    .iter()
                    .any(|id| s.selected.contains(id));
                if selected {
                    set(cr, st.key_selected);
                } else if let Some(c) = st.position_key.filter(|_| !s.curves && channel < 3) {
                    set(cr, c);
                } else {
                    set(cr, colours[channel]);
                }
                match st.key_outline {
                    None => diamond(cr, x, y, st.key_size),
                    Some(c) => {
                        let c = st.key_selected_outline.filter(|_| selected).unwrap_or(c);
                        diamond_path(cr, x, y, st.key_size);
                        cr.fill_preserve();
                        set(cr, c);
                        cr.stroke();
                    }
                }
            }
        }
        if let Some(Drag::Select { x, y, end, .. }) = &s.drag {
            set(cr, st.selection_fill);
            if st.crisp {
                let (left, top) = (px(x.min(end.0)), px(y.min(end.1)));
                cr.rectangle(
                    left,
                    top,
                    (px(x.max(end.0)) - left).max(1.0),
                    (px(y.max(end.1)) - top).max(1.0),
                );
            } else {
                cr.rectangle(
                    x.min(end.0),
                    y.min(end.1),
                    (x - end.0).abs(),
                    (y - end.1).abs(),
                );
            }
            cr.fill_preserve();
            set(cr, st.selection_stroke);
            cr.stroke();
        }
        let x = playhead;
        set(cr, st.playhead);
        match st.playhead_flag {
            None => {
                cr.move_to(x, g.top - 7.0);
                cr.line_to(x, h);
                cr.stroke();
                diamond(cr, x, g.top - 8.0, 4.0);
            }
            Some(label_colour) => {
                let x = px(x);
                let (label, advance, left, width) = flag.unwrap_or_default();
                let height = (g.top - 8.0).max(10.0);
                rounded(cr, left, 3.0, width, height, 3.0);
                cr.fill();
                // A small notch points from the flag down to the line.
                cr.move_to(x - 4.0, 3.0 + height);
                cr.line_to(x + 4.0, 3.0 + height);
                cr.line_to(x, 3.0 + height + 4.0);
                cr.close_path();
                cr.fill();
                cr.set_font_bold(true);
                text(
                    cr,
                    (left + (width - advance) / 2.0).round(),
                    st.ruler_baseline,
                    &label,
                    label_colour,
                    st.ruler_font_size,
                );
                cr.set_font_bold(false);
            }
        }
        cr.restore();
    }
}

trait RoundIf {
    fn round_if(self, on: bool) -> Self;
}
impl RoundIf for f64 {
    fn round_if(self, on: bool) -> Self {
        if on {
            self.round()
        } else {
            self
        }
    }
}

impl State {
    pub fn begin(&mut self, button: u32, x: f64, y: f64, w: f64, h: f64, extend: bool) {
        self.pointer_start = (x, y);
        let s = self;
        if button == 2 {
            s.drag = Some(Drag::Pan { start: s.start });
            return;
        }
        if button != 1 {
            return;
        }
        if x < s.geometry.left {
            s.active = s.lane(y, h);
            s.fit_values();
            return;
        }
        if y < s.geometry.top {
            s.frame = s.time(x, w) * s.clip.fps as f64;
            s.drag = Some(Drag::Scrub);
            return;
        }
        if let Some((id, incoming, ..)) = s
            .handles(&s.geometry, w, h)
            .into_iter()
            .find(|(_, _, a, b)| (a - x).hypot(b - y) < 8.0)
        {
            s.drag = Some(Drag::Handle {
                before: s.clip.clone(),
                id,
                incoming,
            });
            return;
        }
        let hit = s
            .clip
            .tracks
            .iter()
            .enumerate()
            .filter(|(c, _)| !s.curves || *c == s.active)
            .find_map(|(c, keys)| {
                keys.iter()
                    .find(|k| (s.x(k.time, w) - x).hypot(s.y(c, k.value, h) - y) < 9.0)
                    .map(|k| (c, k.id, k.time))
            });
        if let Some((channel, id, anchor)) = hit {
            s.active = channel;
            let group = s.key_group(channel, id);
            if extend {
                if group.iter().all(|id| s.selected.contains(id)) {
                    s.selected.retain(|id| !group.contains(id));
                } else {
                    s.selected.extend(group);
                }
            } else if !group.iter().all(|id| s.selected.contains(id)) {
                s.selected = group;
            }
            s.drag = Some(Drag::Keys {
                before: s.clip.clone(),
                anchor,
            });
        } else {
            let base = if extend {
                s.selected.clone()
            } else {
                BTreeSet::new()
            };
            s.selected = base.clone();
            s.drag = Some(Drag::Select {
                x,
                y,
                base,
                end: (x, y),
            });
        }
    }
    pub fn update(&mut self, dx: f64, dy: f64, w: f64, h: f64) {
        let s = self;
        match s.drag.clone() {
            Some(Drag::Pan { start }) => {
                s.start = (start - s.seconds(dx, w)).max(0.0);
            }
            Some(Drag::Scrub) => {
                let (x, _) = s.pointer_start;
                s.frame = s.time(x + dx, w) * s.clip.fps as f64;
            }
            Some(Drag::Keys { before, anchor, .. }) => {
                let raw = s.seconds(dx, w);
                let dt = if s.snap {
                    ((anchor + raw) * s.clip.fps as f64).round() / s.clip.fps as f64 - anchor
                } else {
                    raw
                };
                let dv = if s.curves {
                    -dy / s.value_height(&s.geometry, h) * (s.high - s.low)
                } else {
                    0.0
                };
                let mut draft = before;
                if draft.move_keys(&s.selected, dt, dv).is_ok() {
                    s.clip = draft;
                }
            }
            Some(Drag::Handle {
                before,
                id,
                incoming,
            }) => {
                let (_, start_y) = s.pointer_start;
                let mut draft = before;
                let channel = s.active;
                let keys = &draft.tracks[channel];
                let index = keys.iter().position(|k| k.id == id).unwrap();
                let key = &keys[index];
                let dt = if incoming {
                    (key.time - keys[index - 1].time) / 3.0
                } else {
                    (keys[index + 1].time - key.time) / 3.0
                };
                let value = s.value_at(start_y + dy, h);
                let slope = if incoming {
                    (key.value - value) / dt
                } else {
                    (value - key.value) / dt
                };
                let key = draft.key_mut(id).unwrap();
                if incoming {
                    key.incoming = slope.clamp(-1e9, 1e9);
                } else {
                    key.outgoing = slope.clamp(-1e9, 1e9);
                }
                s.clip = draft;
            }
            Some(Drag::Select { x, y, base, .. }) => {
                let end = (x + dx, y + dy);
                let mut selected = base.clone();
                for (c, keys) in s
                    .clip
                    .tracks
                    .iter()
                    .enumerate()
                    .filter(|(c, _)| !s.curves || *c == s.active)
                {
                    for k in keys {
                        let (a, b) = (s.x(k.time, w), s.y(c, k.value, h));
                        if a >= x.min(end.0)
                            && a <= x.max(end.0)
                            && b >= y.min(end.1)
                            && b <= y.max(end.1)
                        {
                            selected.insert(k.id);
                        }
                    }
                }
                s.selected = selected;
                s.drag = Some(Drag::Select { x, y, base, end });
            }
            None => {}
        }
    }
    pub fn end(&mut self) -> bool {
        match self.drag.take() {
            Some(Drag::Keys { before, .. } | Drag::Handle { before, .. }) => before != self.clip,
            _ => false,
        }
    }
    pub fn double_click(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<bool, String> {
        if x < self.geometry.left || y < self.geometry.top {
            return Ok(false);
        }
        let time = self.time(x, w);
        let channel = if self.curves {
            self.active
        } else {
            self.lane(y, h)
        };
        let value = if self.curves {
            self.value_at(y, h)
        } else {
            self.clip.sample(channel, time)
        };
        self.insert_key(channel, time, value)?;
        self.drag = None;
        Ok(true)
    }
    pub fn scroll(&mut self, dx: f64, dy: f64, control: bool) {
        if control {
            self.zoom(1.2_f64.powf(dy));
        } else {
            self.start = (self.start + (dx + dy) * self.span * 0.08).max(0.0);
        }
    }
}
/// Playback and clip settings shared by the frontends' animation panels.
impl State {
    /// The last frame of the clip.
    pub fn end_frame(&self) -> f64 {
        self.clip.duration * self.clip.fps as f64
    }
    /// The playhead time in seconds.
    pub fn playhead_time(&self) -> f64 {
        self.frame / self.clip.fps.max(1) as f64
    }
    /// Starting playback at the end restarts from the first frame.
    pub fn start_playback(&mut self) {
        if self.frame >= self.end_frame() {
            self.frame = 0.0;
        }
    }
    /// Advances playback by `seconds` honouring the clip's mode: once stops at the end,
    /// ping-pong reverses at both ends, loop wraps. `direction` is +1 or -1 and persists
    /// between calls. Returns true when playback should stop.
    pub fn advance_playback(&mut self, seconds: f64, direction: &mut f64) -> bool {
        let end = self.end_frame();
        let step = seconds * self.clip.fps as f64;
        let next = self.frame + step * *direction;
        match self.mode.as_str() {
            "once" => {
                self.frame = next.min(end);
                *direction = 1.0;
                next >= end
            }
            "pingpong" => {
                if next >= end {
                    self.frame = end;
                    *direction = -1.0;
                } else if next <= 0.0 {
                    self.frame = 0.0;
                    *direction = 1.0;
                } else {
                    self.frame = next;
                }
                false
            }
            _ => {
                self.frame = (self.frame + step).rem_euclid(end.max(1e-9));
                *direction = 1.0;
                false
            }
        }
    }
    /// Changes the frame rate keeping the playhead at the same time.
    pub fn set_fps(&mut self, fps: u32) {
        let time = self.playhead_time();
        self.clip.fps = fps.clamp(1, 120);
        self.frame = (time * self.clip.fps as f64).min(self.end_frame());
    }
    /// Changes the duration, never cutting off the last key.
    pub fn set_duration(&mut self, duration: f64) {
        let last = self
            .clip
            .tracks
            .iter()
            .flatten()
            .map(|k| k.time)
            .fold(0.001, f64::max);
        self.clip.duration = duration.max(last);
        self.frame = self.frame.min(self.end_frame());
    }
    /// Selects every key, or every key of the active curve in the curve editor.
    pub fn select_all(&mut self) {
        self.selected = self
            .clip
            .tracks
            .iter()
            .enumerate()
            .filter(|(c, _)| !self.curves || *c == self.active)
            .flat_map(|(_, keys)| keys.iter().map(|k| k.id))
            .collect();
    }
    /// The playhead moved to `time`, selecting the position keys there, as a click on a
    /// motion-path key in a view does.
    pub fn select_position_key(&mut self, time: f64) {
        self.cancel();
        self.frame = time.clamp(0.0, self.clip.duration) * self.clip.fps as f64;
        self.selected = self.clip.tracks[..3]
            .iter()
            .flatten()
            .filter(|k| (k.time - time).abs() < 1e-6)
            .map(|k| k.id)
            .collect();
        self.active = (0..3)
            .find(|axis| {
                self.clip.tracks[*axis]
                    .iter()
                    .any(|k| self.selected.contains(&k.id))
            })
            .unwrap_or(0);
    }
    /// Takes newer data from the document while keeping the view: zoom, scroll, the
    /// playhead's time and the keys that still exist stay selected. A different clip
    /// (`renamed`) or an emptied active track resets selection and framing.
    pub fn reload(&mut self, clip: Clip, mode: String, pivot: [f64; 3], renamed: bool) {
        let time = self.playhead_time();
        self.clip = clip;
        self.mode = mode;
        self.pivot = pivot;
        if renamed || self.clip.tracks[self.active].is_empty() {
            self.selected.clear();
            if self.clip.tracks[self.active].is_empty() {
                self.active = self
                    .clip
                    .tracks
                    .iter()
                    .position(|t| !t.is_empty())
                    .unwrap_or(0);
            }
            self.fit_values();
            self.start = 0.0;
            self.span = self.clip.duration * 1.1;
        }
        let ids: BTreeSet<u64> = self.clip.tracks.iter().flatten().map(|k| k.id).collect();
        self.selected.retain(|id| ids.contains(id));
        self.frame = time.min(self.clip.duration) * self.clip.fps as f64;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// The drawing as it was before styles, kept to check the classic style against.
    fn old_draw(s: &State, cr: &mut dyn Canvas, w: f64, h: f64, fg: [f64; 3]) {
        let dark = fg.iter().sum::<f64>() > 1.5;
        let bg = if dark { 0.12 } else { 0.94 };
        cr.set_source_rgb(bg, bg, bg);
        cr.paint();
        let colours = [(0.9, 0.3, 0.3), (0.25, 0.7, 0.35), (0.25, 0.55, 0.95)];
        let text = |cr: &mut dyn Canvas, x: f64, y: f64, label: &str| {
            cr.set_source_rgba(fg[0], fg[1], fg[2], 0.85);
            cr.move_to(x, y);
            cr.show_text(label, 12.0);
        };
        for (channel, label) in CHANNELS.iter().enumerate() {
            if !s.curves && (channel == 1 || channel == 2) {
                continue;
            }
            let label = if !s.curves && channel == 0 {
                tr("Position")
            } else {
                *label
            };
            let y = TOP + (s.row(channel) as f64 + 0.5) * (h - TOP - 12.0) / s.lane_count();
            if s.row(channel) == s.row(s.active) {
                cr.set_source_rgba(0.3, 0.55, 0.85, 0.18);
                cr.rectangle(
                    0.0,
                    y - (h - TOP - 12.0) / (s.lane_count() * 2.0),
                    LEFT - 4.0,
                    (h - TOP - 12.0) / s.lane_count(),
                );
                cr.fill();
            }
            let c = colours[channel % 3];
            cr.set_source_rgb(c.0, c.1, c.2);
            cr.rectangle(8.0, y - 5.0, 3.0, 12.0);
            cr.fill();
            text(cr, 18.0, y + 5.0, label);
            if s.curves || channel >= 3 {
                text(
                    cr,
                    52.0,
                    y + 5.0,
                    &format!("{:.1}", s.clip.sample(channel, s.frame / s.clip.fps as f64)),
                );
            }
        }
        cr.save();
        cr.rectangle(LEFT, 0.0, w - LEFT, h);
        cr.clip();
        // Adaptive ticks retain readable labels at every zoom level.
        let target = s.span / ((w - LEFT) / 85.0).max(1.0);
        let power = 10.0_f64.powf(target.log10().floor());
        let step = [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .map(|n| n * power)
            .find(|n| *n >= target)
            .unwrap_or(power * 10.0)
            .max(1.0 / s.clip.fps as f64);
        let first = (s.start / step).floor() * step;
        for tick in 0..=(((s.span / step).ceil() as usize) + 2).min(500) {
            let t = first + tick as f64 * step;
            let x = s.x(t, w);
            cr.set_source_rgba(fg[0], fg[1], fg[2], 0.12);
            cr.move_to(x, TOP);
            cr.line_to(x, h);
            cr.stroke();
            text(cr, x + 3.0, 17.0, &format!("{:.0}", t * s.clip.fps as f64));
        }
        let end = s.x(s.clip.duration, w);
        cr.set_source_rgba(0.5, 0.5, 0.5, 0.12);
        cr.rectangle(end, TOP, (w - end).max(0.0), h - TOP);
        cr.fill();
        if s.curves {
            for i in 0..=4 {
                let value = s.low + (s.high - s.low) * i as f64 / 4.0;
                let y = s.y(s.active, value, h);
                cr.set_source_rgba(0.5, 0.5, 0.5, 0.18);
                cr.move_to(LEFT, y);
                cr.line_to(w, y);
                cr.stroke();
                text(cr, LEFT + 4.0, y - 3.0, &format!("{value:.1}"));
            }
            let c = colours[s.active % 3];
            cr.set_source_rgb(c.0, c.1, c.2);
            cr.set_line_width(2.0);
            for i in 0..=(w - LEFT).max(0.0) as usize {
                let x = LEFT + i as f64;
                let t = s.start + i as f64 / (w - LEFT - 12.0).max(1.0) * s.span;
                let y = s.y(s.active, s.clip.sample(s.active, t), h);
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.stroke();
            cr.set_line_width(1.0);
            for (id, _, x, y) in s.handles(&TimelineGeometry::CLASSIC, w, h) {
                let (_, key) = s.clip.key(id).unwrap();
                cr.set_source_rgba(0.6, 0.65, 0.7, 0.9);
                cr.move_to(s.x(key.time, w), s.y(s.active, key.value, h));
                cr.line_to(x, y);
                cr.stroke();
                cr.arc(x, y, 4.0, 0.0, std::f64::consts::TAU);
                cr.fill();
            }
        }
        let mut drawn_positions = BTreeSet::new();
        for (channel, keys) in s.clip.tracks.iter().enumerate() {
            if s.curves && channel != s.active {
                continue;
            }
            for key in keys {
                if !s.curves && channel < 3 && !drawn_positions.insert(key.time.to_bits()) {
                    continue;
                }
                let x = s.x(key.time, w);
                let y = s.y(channel, key.value, h);
                if s.key_group(channel, key.id)
                    .iter()
                    .any(|id| s.selected.contains(id))
                {
                    cr.set_source_rgb(1.0, 0.7, 0.15);
                } else {
                    let c = colours[channel % 3];
                    cr.set_source_rgb(c.0, c.1, c.2);
                }
                diamond(cr, x, y, 5.0);
            }
        }
        if let Some(Drag::Select { x, y, end, .. }) = &s.drag {
            cr.set_source_rgba(0.35, 0.6, 1.0, 0.18);
            cr.rectangle(
                x.min(end.0),
                y.min(end.1),
                (x - end.0).abs(),
                (y - end.1).abs(),
            );
            cr.fill_preserve();
            cr.set_source_rgba(0.35, 0.6, 1.0, 0.8);
            cr.stroke();
        }
        let x = s.x(s.frame / s.clip.fps as f64, w);
        cr.set_source_rgb(1.0, 0.45, 0.2);
        cr.move_to(x, TOP - 7.0);
        cr.line_to(x, h);
        cr.stroke();
        diamond(cr, x, TOP - 8.0, 4.0);
        cr.restore();
    }

    /// A canvas that writes down every call, numbers rounded so the same drawing
    /// computed in a different order compares equal.
    #[derive(Default)]
    struct Recorder(Vec<String>);
    impl Canvas for Recorder {
        fn save(&mut self) {
            self.0.push("save".into());
        }
        fn restore(&mut self) {
            self.0.push("restore".into());
        }
        fn translate(&mut self, x: f64, y: f64) {
            self.0.push(format!("translate {x:.6} {y:.6}"));
        }
        fn scale(&mut self, x: f64, y: f64) {
            self.0.push(format!("scale {x:.6} {y:.6}"));
        }
        fn new_path(&mut self) {
            self.0.push("new_path".into());
        }
        fn new_sub_path(&mut self) {
            self.0.push("new_sub_path".into());
        }
        fn move_to(&mut self, x: f64, y: f64) {
            self.0.push(format!("move_to {x:.6} {y:.6}"));
        }
        fn line_to(&mut self, x: f64, y: f64) {
            self.0.push(format!("line_to {x:.6} {y:.6}"));
        }
        fn arc(&mut self, xc: f64, yc: f64, r: f64, a: f64, b: f64) {
            self.0
                .push(format!("arc {xc:.6} {yc:.6} {r:.6} {a:.6} {b:.6}"));
        }
        fn arc_negative(&mut self, xc: f64, yc: f64, r: f64, a: f64, b: f64) {
            self.0
                .push(format!("arc_negative {xc:.6} {yc:.6} {r:.6} {a:.6} {b:.6}"));
        }
        fn rectangle(&mut self, x: f64, y: f64, w: f64, h: f64) {
            self.0
                .push(format!("rectangle {x:.6} {y:.6} {w:.6} {h:.6}"));
        }
        fn close_path(&mut self) {
            self.0.push("close_path".into());
        }
        fn set_source_rgba(&mut self, r: f64, g: f64, b: f64, a: f64) {
            self.0.push(format!("rgba {r:.6} {g:.6} {b:.6} {a:.6}"));
        }
        fn set_line_width(&mut self, w: f64) {
            self.0.push(format!("line_width {w:.6}"));
        }
        fn set_dash(&mut self, d: &[f64], o: f64) {
            self.0.push(format!("dash {d:?} {o}"));
        }
        fn set_line_cap(&mut self, c: crate::canvas::LineCap) {
            self.0.push(format!("cap {c:?}"));
        }
        fn set_line_join(&mut self, j: crate::canvas::LineJoin) {
            self.0.push(format!("join {j:?}"));
        }
        fn set_fill_rule(&mut self, r: crate::canvas::FillRule) {
            self.0.push(format!("rule {r:?}"));
        }
        fn stroke_preserve(&mut self) {
            self.0.push("stroke_preserve".into());
        }
        fn fill_preserve(&mut self) {
            self.0.push("fill_preserve".into());
        }
        fn paint(&mut self) {
            self.0.push("paint".into());
        }
        fn clip(&mut self) {
            self.0.push("clip".into());
        }
        fn paint_tiled_image(
            &mut self,
            key: &str,
            _: u32,
            _: u32,
            _: &[u8],
            _: (f64, f64),
            _: f64,
        ) {
            self.0.push(format!("image {key}"));
        }
        fn set_font_bold(&mut self, b: bool) {
            self.0.push(format!("bold {b}"));
        }
        fn show_text(&mut self, t: &str, s: f64) {
            self.0.push(format!("text {t} {s:.6}"));
        }
        fn show_text_outlined(&mut self, t: &str, s: f64, _: [f64; 4], _: f64) {
            self.0.push(format!("outlined {t} {s:.6}"));
        }
        fn text_extents(&mut self, t: &str, s: f64) -> crate::canvas::TextExtents {
            let w = t.chars().count() as f64 * s * 0.55;
            crate::canvas::TextExtents {
                width: w,
                x_advance: w,
                height: s,
                line_height: s * 1.2,
                baseline: s,
                ..Default::default()
            }
        }
    }
    fn styled_clip() -> State {
        let track = crate::keyframe_animation::Track::parse(
            "0 0 0 0 0 0 0; 1 64 -16 8 0 45 0; 2 128 32 16 0 0 90",
            "loop",
        )
        .unwrap();
        let mut state = State::new(Clip::from_legacy(&track), "loop".into(), [0.0; 3]);
        // A position key on Y alone, which the key view draws in Y's colour.
        let id = state.clip.insert(1, 1.5, 3.0).unwrap();
        state.selected.insert(id);
        state.frame = 37.0;
        state
    }
    #[test]
    fn the_classic_style_draws_exactly_what_the_old_timeline_drew() {
        for fg in [[0.9, 0.9, 0.9], [0.1, 0.1, 0.1]] {
            for curves in [false, true] {
                for (w, h) in [(800.0, 240.0), (461.0, 199.0)] {
                    let mut state = styled_clip();
                    state.curves = curves;
                    if curves {
                        state.active = 0;
                        state.clip.tracks[0][1].interpolation = Interpolation::Bezier;
                        state.selected = BTreeSet::from([state.clip.tracks[0][1].id]);
                        state.fit_values();
                    }
                    state.begin(1, LEFT + 30.0, TOP + 20.0, w, h, false);
                    state.update(120.0, 40.0, w, h);
                    assert!(matches!(state.drag, Some(Drag::Select { .. })));
                    let mut old = Recorder::default();
                    old_draw(&state, &mut old, w, h, fg);
                    let mut new = Recorder::default();
                    state.draw(&mut new, w, h, fg);
                    assert!(old.0.len() > 50);
                    assert_eq!(old.0, new.0, "curves {curves}, {w}x{h}");
                }
            }
        }
    }
    #[test]
    fn keys_are_hit_where_a_modern_geometry_draws_them() {
        let geometry = TimelineGeometry {
            left: 160.0,
            top: 26.0,
            pad: 10.0,
            right: 16.0,
            bottom: 0.0,
            curve_top: 14.0,
            curve_bottom: 14.0,
        };
        let mut state = styled_clip();
        state.selected.clear();
        state.geometry = geometry;
        let (w, h) = (700.0, 220.0);
        let id = state.clip.tracks[3][1].id;
        let key = state.clip.tracks[3][1].clone();
        // The rotation X row is the second of four lanes under the 26-pixel ruler.
        let x = 170.0 + (key.time - state.start) / state.span * (w - 186.0);
        let y = 26.0 + 1.5 * (h - 26.0) / 4.0;
        assert!(
            (state.x(key.time, w) - x).abs() < 1e-9 && (state.y(3, key.value, h) - y).abs() < 1e-9
        );
        // Under the classic layout that point misses the key.
        let mut classic = styled_clip();
        classic.selected.clear();
        classic.begin(1, x, y, w, h, false);
        assert!(classic.selected.is_empty());
        state.begin(1, x, y, w, h, false);
        assert_eq!(state.selected, BTreeSet::from([id]));
        assert_eq!(state.active, 3);
        assert!(!state.end());
        // A click in the label column picks that row; one in the ruler scrubs.
        state.begin(1, 150.0, 26.0 + 3.5 * (h - 26.0) / 4.0, w, h, false);
        assert_eq!(state.active, 5);
        state.begin(1, x, 20.0, w, h, false);
        assert!((state.frame - key.time * state.clip.fps as f64).abs() < 1e-6);
        state.end();
        // The curve view maps values over the modern padding.
        state.curves = true;
        state.active = 3;
        state.fit_values();
        let top = state.y(3, state.high, h);
        assert!((top - 40.0).abs() < 1e-9 && (state.y(3, state.low, h) - (h - 14.0)).abs() < 1e-9);
        assert!((state.value_at(top, h) - state.high).abs() < 1e-9);
        state.selected.clear();
        state.begin(1, x, state.y(3, key.value, h), w, h, false);
        assert_eq!(state.selected, BTreeSet::from([id]));
    }
    #[test]
    fn a_modern_style_draws_a_ruler_flag_and_outlined_keys() {
        let mut style = TimelineStyle::classic([0.9; 3]);
        style.geometry = TimelineGeometry {
            left: 160.0,
            top: 26.0,
            pad: 10.0,
            right: 16.0,
            bottom: 0.0,
            curve_top: 14.0,
            curve_bottom: 14.0,
        };
        style.playhead_flag = Some([1.0; 4]);
        style.key_outline = Some([0.0, 0.0, 0.0, 1.0]);
        style.crisp = true;
        style.frame_steps = true;
        style.grid_minor = Some([1.0, 1.0, 1.0, 0.05]);
        style.ruler_tick = Some([1.0, 1.0, 1.0, 0.3]);
        let mut state = styled_clip();
        state.geometry = style.geometry;
        let mut out = Recorder::default();
        state.draw_styled(&mut out, 700.0, 220.0, &style);
        assert!(
            out.0
                .iter()
                .any(|c| c == "text 37 11.000000" || c == "text 37 12.000000"),
            "the flag shows the frame"
        );
        assert!(
            out.0.iter().any(|c| c == "stroke_preserve"),
            "keys are outlined"
        );
        // Every vertical grid line sits on a pixel centre.
        for pair in out.0.windows(2) {
            if let (Some(a), Some(b)) = (
                pair[0].strip_prefix("move_to "),
                pair[1].strip_prefix("line_to "),
            ) {
                let (a, b): (Vec<f64>, Vec<f64>) = (
                    a.split(' ').map(|v| v.parse().unwrap()).collect(),
                    b.split(' ').map(|v| v.parse().unwrap()).collect(),
                );
                if a[0] == b[0] && b[1] == 220.0 {
                    assert_eq!(a[0].fract(), 0.5, "{pair:?}");
                }
            }
        }
    }
    fn two_second_clip(mode: &str) -> State {
        let track =
            crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 2 128 0 0 0 0 0", "loop")
                .unwrap();
        State::new(Clip::from_legacy(&track), mode.into(), [0.0; 3])
    }
    #[test]
    fn playback_honours_once_loop_and_ping_pong() {
        let mut direction = 1.0;
        let mut once = two_second_clip("once");
        once.frame = 50.0;
        assert!(!once.advance_playback(0.25, &mut direction));
        assert!(once.advance_playback(1.0, &mut direction));
        assert_eq!(once.frame, 60.0);
        once.start_playback();
        assert_eq!(once.frame, 0.0);
        let mut looped = two_second_clip("loop");
        looped.frame = 55.0;
        assert!(!looped.advance_playback(0.5, &mut direction));
        assert!((looped.frame - 10.0).abs() < 1e-9);
        let mut pong = two_second_clip("pingpong");
        pong.frame = 55.0;
        assert!(!pong.advance_playback(0.5, &mut direction));
        assert_eq!((pong.frame, direction), (60.0, -1.0));
        pong.advance_playback(0.5, &mut direction);
        assert_eq!(pong.frame, 45.0);
        pong.advance_playback(5.0, &mut direction);
        assert_eq!((pong.frame, direction), (0.0, 1.0));
    }
    #[test]
    fn fps_keeps_the_playhead_time_and_duration_keeps_the_last_key() {
        let mut state = two_second_clip("loop");
        state.frame = 30.0;
        state.set_fps(60);
        assert_eq!(state.frame, 60.0);
        state.set_duration(0.5);
        assert_eq!(state.clip.duration, 2.0);
        state.set_duration(4.0);
        assert_eq!(state.clip.duration, 4.0);
    }
    #[test]
    fn reload_keeps_zoom_selection_and_time() {
        let mut state = two_second_clip("loop");
        state.zoom(0.5);
        state.frame = 30.0;
        let id = state.clip.tracks[0][1].id;
        state.selected = BTreeSet::from([id]);
        let (start, span) = (state.start, state.span);
        let mut clip = state.clip.clone();
        clip.fps = 60;
        state.reload(clip, "once".into(), [1.0; 3], false);
        assert_eq!((state.start, state.span), (start, span));
        assert_eq!(state.selected, BTreeSet::from([id]));
        assert_eq!(state.frame, 60.0);
        assert_eq!(state.mode, "once");
    }
    #[test]
    fn pointer_drag_groups_position_and_cancel_restores_all_axes() {
        let track =
            crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 2 128 32 16 0 0 90", "loop")
                .unwrap();
        let clip = Clip::from_legacy(&track);
        let mut state = State::new(clip.clone(), "loop".into(), [0.0; 3]);
        let (w, h) = (800.0, 240.0);
        let x = state.x(2.0, w);
        let y = state.y(0, 128.0, h);
        state.begin(1, x, y, w, h, false);
        assert_eq!(state.selected.len(), 3);
        state.update(-(w - LEFT - 12.0) / state.span * 0.5, 80.0, w, h);
        for c in 0..3 {
            assert!((state.clip.tracks[c][1].time - 1.5).abs() < 1e-6);
            assert_eq!(state.clip.tracks[c][1].value, clip.tracks[c][1].value);
        }
        assert!(state.cancel());
        assert_eq!(state.clip, clip);
        assert!(!state.end());
        state.begin(1, x, y, w, h, false);
        state.update(-(w - LEFT - 12.0) / state.span * 0.5, 0.0, w, h);
        assert!(state.end());
        assert!(!state.end());
    }
    #[test]
    fn bezier_handle_drag_edits_only_its_slope() {
        let track =
            crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 2 128 0 0 0 0 90", "loop")
                .unwrap();
        let mut state = State::new(Clip::from_legacy(&track), "loop".into(), [0.0; 3]);
        state.curves = true;
        let key = &mut state.clip.tracks[0][0];
        key.interpolation = Interpolation::Bezier;
        state.selected.insert(key.id);
        state.fit_values();
        let before = state.clip.clone();
        let (_, incoming, x, y) = state.handles(&TimelineGeometry::CLASSIC, 800.0, 240.0)[0];
        assert!(!incoming);
        state.begin(1, x, y, 800.0, 240.0, false);
        state.update(0.0, -20.0, 800.0, 240.0);
        assert!(state.clip.tracks[0][0].outgoing > before.tracks[0][0].outgoing);
        assert_eq!(state.clip.tracks[0][0].time, before.tracks[0][0].time);
        assert_eq!(state.clip.tracks[0][0].value, before.tracks[0][0].value);
        assert!(state.end());
    }
    #[test]
    fn position_lane_selects_copies_retimes_and_deletes_whole_positions() {
        let clip = Clip {
            version: 1,
            duration: 4.0,
            fps: 30,
            tracks: std::array::from_fn(|_| Vec::new()),
        };
        let mut s = State::new(clip, "pingpong".into(), [0.0; 3]);
        s.insert_key(0, 1.0, 128.0).unwrap();
        assert_eq!(s.selected.len(), 3);
        assert_eq!(s.position_time(), Some(1.0));
        assert_eq!(s.lane(TOP + 10.0, 240.0), 0);
        assert_eq!(s.lane(239.0, 240.0), 5);
        assert_eq!(s.y(0, 128.0, 240.0), s.y(2, 0.0, 240.0));
        let copy = s.clip.copy(&s.selected);
        assert_eq!(copy.len(), 3);
        let pasted = s.clip.paste(&copy, 2.0).unwrap();
        assert_eq!(pasted.len(), 3);
        s.clip.retime(&pasted, 1.0, 0.0, 1.0, false).unwrap();
        for axis in 0..3 {
            assert!(s.clip.tracks[axis].iter().any(|k| k.time == 3.0));
        }
        s.clip.delete(&pasted);
        for axis in 0..3 {
            assert_eq!(s.clip.tracks[axis].len(), 2);
        }
        let id = s.clip.tracks[0][1].id;
        s.selected = BTreeSet::from([id]);
        s.group_selection();
        assert_eq!(s.selected.len(), 3);
        s.curves = true;
        assert_eq!(s.key_group(0, id), BTreeSet::from([id]));
    }
    #[test]
    fn cancel_restores_draft_and_frame_snap_stays_inside_duration() {
        let track =
            crate::keyframe_animation::Track::parse("0 0 0 0 0 0 0; 2 128 0 0 0 0 90", "loop")
                .unwrap();
        let clip = Clip::from_legacy(&track);
        let mut state = State::new(clip.clone(), "loop".into(), [0.0; 3]);
        state.selected.insert(clip.tracks[0][1].id);
        state.drag = Some(Drag::Keys {
            before: clip.clone(),
            anchor: 2.0,
        });
        state.clip.move_keys(&state.selected, -0.5, 10.0).unwrap();
        assert_ne!(state.clip, clip);
        assert!(state.cancel());
        assert_eq!(state.clip, clip);
        state.clip.duration = 2.02;
        state.span = 3.0;
        assert!(state.time(1000.0, 1000.0) <= 2.02);
        assert!(!state.cancel());
    }
}
