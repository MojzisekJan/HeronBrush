//! The sculpt tool's settings: what a dab of the brush does to the terrain under it.
use crate::i18n::translation_key;

/// What a dab does to the surface under the brush.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SculptMode {
    Raise,
    Lower,
    /// Pulls each point towards the average height of its neighbours.
    Smooth,
    /// Pulls each point towards the height where the stroke began.
    Flatten,
}

impl SculptMode {
    pub const ALL: [Self; 4] = [Self::Raise, Self::Lower, Self::Smooth, Self::Flatten];

    pub fn name(self) -> &'static str {
        match self {
            Self::Raise => translation_key("Raise"),
            Self::Lower => translation_key("Lower"),
            Self::Smooth => translation_key("Smooth"),
            Self::Flatten => translation_key("Flatten"),
        }
    }

    /// Raise and Lower swapped, as Shift does while sculpting.
    pub fn inverted(self) -> Self {
        match self {
            Self::Raise => Self::Lower,
            Self::Lower => Self::Raise,
            other => other,
        }
    }
}

/// Which way a stroke moves the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SculptDirection {
    /// Straight up and down, within a column standing on the ground: terrain.
    Up,
    /// Towards and away from the camera, within a ball: whatever the view looks at, a
    /// floor, a wall or a ceiling.
    Camera,
    /// Out of and into the face under the pointer where the stroke begins, within a ball.
    Normal,
}

impl SculptDirection {
    pub const ALL: [Self; 3] = [Self::Up, Self::Camera, Self::Normal];

    pub fn name(self) -> &'static str {
        match self {
            Self::Up => translation_key("Up"),
            Self::Camera => translation_key("Camera"),
            Self::Normal => translation_key("Normal"),
        }
    }
}

/// The shape of the columns Split into columns cuts a brush into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnShape {
    /// Two triangles per grid cell. Each top stays flat however its corners move.
    Triangles,
    /// One column per grid cell. A sculpted top bends along a diagonal.
    Squares,
    /// Hexagons two grid cells across, each cut into six triangles round its centre, so
    /// every top stays flat. The corners lie on half grid steps, which makes a hexagon a
    /// little taller than a regular one but keeps neighbouring brushes' corners shared.
    Hexagons,
}

impl ColumnShape {
    pub const ALL: [Self; 3] = [Self::Triangles, Self::Squares, Self::Hexagons];

    pub fn name(self) -> &'static str {
        match self {
            Self::Triangles => translation_key("Triangle"),
            Self::Squares => translation_key("Square"),
            Self::Hexagons => translation_key("Hexagon"),
        }
    }

    /// How many columns one grid cell becomes, on average for hexagons: one covers three
    /// cells and makes six.
    pub fn per_cell(self) -> usize {
        match self {
            Self::Triangles | Self::Hexagons => 2,
            Self::Squares => 1,
        }
    }
}

/// The sculpt brush.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SculptParameters {
    pub mode: SculptMode,
    /// Which way a stroke moves the surface.
    pub direction: SculptDirection,
    /// What Split into columns cuts a brush into.
    pub columns: ColumnShape,
    /// Whether a stroke moves square columns as blocks, by whole grid steps with
    /// upright sides, rather than bending their tops.
    pub blocks: bool,
    /// The brush's radius in map units, measured across the ground.
    pub radius: f64,
    /// How much one dab does, from 0 to 1: Raise and Lower move the centre by a tenth of
    /// the radius at full strength, Smooth and Flatten go all the way there.
    pub strength: f64,
}

impl Default for SculptParameters {
    fn default() -> Self {
        Self {
            mode: SculptMode::Raise,
            direction: SculptDirection::Up,
            columns: ColumnShape::Triangles,
            blocks: false,
            radius: 128.0,
            strength: 0.25,
        }
    }
}

impl SculptParameters {
    pub const RADIUS: std::ops::RangeInclusive<f64> = 8.0..=4096.0;
    pub const STRENGTH: std::ops::RangeInclusive<f64> = 0.01..=1.0;

    /// How far apart dabs fall along a stroke.
    pub fn spacing(&self) -> f64 {
        self.radius * 0.25
    }
}

/// How much of a dab reaches a point `distance` from its centre: all of it at the centre,
/// none at the rim, easing in and out.
pub fn falloff(distance: f64, radius: f64) -> f64 {
    if radius <= 0.0 || distance >= radius {
        return 0.0;
    }
    let t = distance / radius;
    (1.0 - t * t).powi(2)
}

thread_local! {
    /// The brush every view sculpts with, set from the tool's settings strip.
    static PARAMETERS: std::cell::Cell<SculptParameters> = std::cell::Cell::new(SculptParameters::default());
}

/// The sculpt brush as the settings strip last set it.
pub fn parameters() -> SculptParameters {
    PARAMETERS.with(std::cell::Cell::get)
}

pub fn set_parameters(parameters: SculptParameters) {
    PARAMETERS.with(|cell| cell.set(parameters));
}
