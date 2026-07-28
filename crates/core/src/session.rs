use std::collections::HashMap;
use std::time::{Duration, Instant};

// Real per-frame movement is well under this even at max mount speed; only a
// teleport (waypoint, portal, map change) can jump farther.
const MAX_PLAUSIBLE_METERS_PER_SAMPLE: f64 = 25.0;

fn distance3(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = (b[0] - a[0]) as f64;
    let dy = (b[1] - a[1]) as f64;
    let dz = (b[2] - a[2]) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[derive(Debug, Default)]
pub struct SessionTracker {
    baseline: Option<HashMap<&'static str, f64>>,
    lifetime: HashMap<&'static str, f64>,
    started_at: Option<Instant>,
    distance_meters: f64,
    last_position: Option<[f32; 3]>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, mut lifetime: HashMap<&'static str, f64>) {
        for (id, value) in lifetime.iter_mut() {
            if let Some(&old) = self.lifetime.get(id) {
                if *value < old && crate::stats::is_regression_guarded(id) {
                    *value = old;
                }
            }
        }
        if self.baseline.is_none() {
            self.baseline = Some(lifetime.clone());
            self.started_at = Some(Instant::now());
        }
        self.lifetime = lifetime;
    }

    pub fn lifetime_value(&self, id: &str) -> f64 {
        self.lifetime.get(id).copied().unwrap_or(0.0)
    }

    pub fn session_value(&self, id: &str) -> f64 {
        let current = self.lifetime_value(id);
        let base = self
            .baseline
            .as_ref()
            .and_then(|baseline| baseline.get(id))
            .copied()
            .unwrap_or(current);
        current - base
    }

    pub fn has_data(&self) -> bool {
        self.baseline.is_some()
    }

    /// Elapsed time since the session started (the first successful poll,
    /// or the last `reset()`). Zero if the session hasn't started yet.
    pub fn elapsed(&self) -> Duration {
        self.started_at.map(|t| t.elapsed()).unwrap_or_default()
    }

    /// Feeds in a live player position (MumbleLink `avatar.position`,
    /// meters) and accumulates the distance moved since the last sample.
    /// A delta past `MAX_PLAUSIBLE_METERS_PER_SAMPLE` is treated as a
    /// teleport (waypoint, portal, character switch) rather than real
    /// movement and is not added to the total, though `last_position`
    /// still updates so tracking resumes correctly from the new spot.
    pub fn sample_position(&mut self, position: [f32; 3]) {
        if let Some(last) = self.last_position {
            let delta = distance3(last, position);
            if delta <= MAX_PLAUSIBLE_METERS_PER_SAMPLE {
                self.distance_meters += delta;
            }
        }
        self.last_position = Some(position);
    }

    pub fn distance_traveled_meters(&self) -> f64 {
        self.distance_meters
    }

    /// Re-baselines to the current lifetime values, so every stat's
    /// session value restarts at zero immediately (rather than waiting
    /// for the next poll to naturally re-baseline, which only happens
    /// when there's no baseline at all yet).
    pub fn reset(&mut self) {
        self.baseline = Some(self.lifetime.clone());
        self.started_at = Some(Instant::now());
        self.distance_meters = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&'static str, f64)]) -> std::collections::HashMap<&'static str, f64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn first_update_sets_baseline_so_session_starts_at_zero() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        assert_eq!(tracker.lifetime_value("kills"), 100.0);
        assert_eq!(tracker.session_value("kills"), 0.0);
    }

    #[test]
    fn later_update_computes_delta_from_baseline() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        assert_eq!(tracker.lifetime_value("kills"), 107.0);
        assert_eq!(tracker.session_value("kills"), 7.0);
    }

    #[test]
    fn unknown_stat_id_defaults_to_zero() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.lifetime_value("unknown"), 0.0);
        assert_eq!(tracker.session_value("unknown"), 0.0);
    }

    #[test]
    fn has_data_false_until_first_update() {
        let mut tracker = SessionTracker::new();
        assert!(!tracker.has_data());
        tracker.update(values(&[("kills", 1.0)]));
        assert!(tracker.has_data());
    }

    #[test]
    fn guarded_stat_ignores_a_lower_value_from_a_later_update() {
        // "kills" is an Achievement-sourced stat, one of the two sources
        // (Achievement, Deaths) known to occasionally regress due to a
        // transient GW2 API bug.
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 90.0)]));
        assert_eq!(tracker.lifetime_value("kills"), 100.0);
    }

    #[test]
    fn unguarded_stat_accepts_a_lower_value_from_a_later_update() {
        // "gold" is a Currency-sourced stat - spending is real, a drop
        // must not be clamped away.
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("gold", 100.0)]));
        tracker.update(values(&[("gold", 90.0)]));
        assert_eq!(tracker.lifetime_value("gold"), 90.0);
    }

    #[test]
    fn reset_restarts_session_value_at_zero_immediately() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        assert_eq!(tracker.session_value("kills"), 7.0);

        tracker.reset();
        assert_eq!(tracker.session_value("kills"), 0.0);
        assert_eq!(tracker.lifetime_value("kills"), 107.0);
    }

    #[test]
    fn reset_then_update_computes_delta_from_new_baseline() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.reset();
        tracker.update(values(&[("kills", 105.0)]));
        assert_eq!(tracker.session_value("kills"), 5.0);
    }

    #[test]
    fn reset_keeps_has_data_true() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        tracker.reset();
        assert!(tracker.has_data());
    }

    #[test]
    fn elapsed_is_zero_before_first_update() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.elapsed(), std::time::Duration::ZERO);
    }

    #[test]
    fn elapsed_is_near_zero_right_after_first_update() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        assert!(tracker.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn reset_restarts_elapsed_near_zero() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        std::thread::sleep(std::time::Duration::from_millis(20));
        tracker.reset();
        assert!(tracker.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn first_position_sample_adds_no_distance() {
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        assert_eq!(tracker.distance_traveled_meters(), 0.0);
    }

    #[test]
    fn second_sample_accumulates_exact_distance() {
        // 3-4-5 right triangle - exact distance is 5.0.
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([3.0, 4.0, 0.0]);
        assert_eq!(tracker.distance_traveled_meters(), 5.0);
    }

    #[test]
    fn multiple_samples_sum_path_length_not_displacement() {
        // Out 5m, back 5m: total path is 10m even though start == end.
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([3.0, 4.0, 0.0]);
        tracker.sample_position([0.0, 0.0, 0.0]);
        assert_eq!(tracker.distance_traveled_meters(), 10.0);
    }

    #[test]
    fn implausible_jump_is_not_counted_but_resumes_tracking() {
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([1000.0, 0.0, 0.0]); // teleport - dropped
        assert_eq!(tracker.distance_traveled_meters(), 0.0);
        tracker.sample_position([1003.0, 4.0, 0.0]); // real movement from the new spot
        assert_eq!(tracker.distance_traveled_meters(), 5.0);
    }

    #[test]
    fn reset_zeroes_distance_but_keeps_last_position() {
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([3.0, 4.0, 0.0]);
        assert_eq!(tracker.distance_traveled_meters(), 5.0);

        tracker.reset();
        assert_eq!(tracker.distance_traveled_meters(), 0.0);

        tracker.sample_position([3.0, 4.0, 0.0]);
        assert_eq!(tracker.distance_traveled_meters(), 0.0);
    }
}
