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

/// The sculpt brush.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SculptParameters {
    pub mode: SculptMode,
    /// The brush's radius in map units, measured across the ground.
    pub radius: f64,
    /// How much one dab does, from 0 to 1: Raise and Lower move the centre by a tenth of
    /// the radius at full strength, Smooth and Flatten go all the way there.
    pub strength: f64,
}

impl Default for SculptParameters {
    fn default() -> Self {
        Self { mode: SculptMode::Raise, radius: 128.0, strength: 0.25 }
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
