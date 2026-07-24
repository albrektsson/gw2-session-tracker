use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct SessionTracker {
    baseline: Option<HashMap<&'static str, f64>>,
    lifetime: HashMap<&'static str, f64>,
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

    /// Re-baselines to the current lifetime values, so every stat's
    /// session value restarts at zero immediately (rather than waiting
    /// for the next poll to naturally re-baseline, which only happens
    /// when there's no baseline at all yet).
    pub fn reset(&mut self) {
        self.baseline = Some(self.lifetime.clone());
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
}
