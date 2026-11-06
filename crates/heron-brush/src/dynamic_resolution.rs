//! Viewport resolution feedback. Still images never use this controller.
#[derive(Debug)]
pub struct DynamicResolution {
    level: usize,
    average: Option<f64>,
    slow: u32,
    fast: u32,
    last_motion: f64,
    last_change: f64,
}
impl Default for DynamicResolution {
    fn default() -> Self {
        Self {
            level: 0,
            average: None,
            slow: 0,
            fast: 0,
            last_motion: f64::NEG_INFINITY,
            last_change: f64::NEG_INFINITY,
        }
    }
}
impl DynamicResolution {
    pub fn update(
        &mut self,
        now: f64,
        moving: bool,
        enabled: bool,
        milliseconds: Option<f64>,
    ) -> f32 {
        if !enabled {
            *self = Self::default();
            return 1.0;
        }
        if moving {
            self.last_motion = now;
        }
        if now - self.last_motion >= 0.20 {
            *self = Self::default();
            return 1.0;
        }
        if let Some(ms) =
            milliseconds.filter(|ms| moving && ms.is_finite() && *ms > 0.0 && *ms < 150.0)
        {
            let average = self.average.map_or(ms, |old| old * 0.75 + ms * 0.25);
            self.average = Some(average);
            self.slow = if average > 18.5 { self.slow + 1 } else { 0 };
            self.fast = if average < 11.5 { self.fast + 1 } else { 0 };
            if now - self.last_change >= 0.25 {
                if self.slow >= 4 && self.level < 3 {
                    self.level += 1;
                    self.slow = 0;
                    self.fast = 0;
                    self.last_change = now;
                } else if self.fast >= 12 && self.level > 0 {
                    self.level -= 1;
                    self.slow = 0;
                    self.fast = 0;
                    self.last_change = now;
                }
            }
        }
        [1.0, 0.85, 0.7, 0.6][self.level]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduces_under_load_and_restores_native_resolution_after_navigation() {
        let mut controller = DynamicResolution::default();
        let mut scale = 1.0;
        for frame in 0..90 {
            scale = controller.update(frame as f64 / 60.0, true, true, Some(32.0));
        }
        assert_eq!(scale, 0.6);
        assert_eq!(controller.update(1.6, false, true, None), 0.6);
        assert_eq!(controller.update(1.8, false, true, None), 1.0);
        for frame in 0..90 {
            assert_eq!(
                controller.update(2.0 + frame as f64 / 60.0, true, true, Some(7.0)),
                1.0
            );
        }
    }
    #[test]
    fn ignores_bad_measurements_and_can_be_disabled() {
        let mut controller = DynamicResolution::default();
        for value in [f64::NAN, f64::INFINITY, -1.0, 999.0] {
            assert_eq!(controller.update(0.0, true, true, Some(value)), 1.0);
        }
        for frame in 0..90 {
            controller.update(frame as f64 / 60.0, true, true, Some(35.0));
        }
        assert_eq!(controller.update(2.0, true, false, Some(35.0)), 1.0);
    }
    #[test]
    fn recovers_quality_during_sustained_fast_navigation() {
        let mut controller = DynamicResolution::default();
        for frame in 0..90 {
            controller.update(frame as f64 / 60.0, true, true, Some(35.0));
        }
        let mut scale = 0.6;
        for frame in 90..270 {
            scale = controller.update(frame as f64 / 60.0, true, true, Some(5.0));
        }
        assert_eq!(scale, 1.0);
    }
}
