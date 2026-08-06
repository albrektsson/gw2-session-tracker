use crate::stats::ratio_with_fallback;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Real per-frame movement is well under this even at max mount speed; only a
// teleport (waypoint, portal, map change) can jump farther.
const MAX_PLAUSIBLE_METERS_PER_SAMPLE: f64 = 25.0;

/// A History Snapshot is captured every Nth successful poll, riding the
/// addon's existing ~60s poll cadence for a ~5 minute interval rather than
/// needing its own timer.
const HISTORY_SNAPSHOT_INTERVAL_TICKS: u64 = 5;

/// A History Snapshot never stores Session Rate alongside `values` - it's
/// always derived at read time as `value / (elapsed.as_secs_f64() /
/// 3600.0)` for the stats `stats::has_rate` allows it for, so it can never
/// drift out of sync with the formula used everywhere else.
#[derive(Debug, Clone)]
pub struct HistorySnapshot {
    pub elapsed: Duration,
    pub values: HashMap<&'static str, f64>,
}

/// The Session's history log: one `HistorySnapshot` of the full Stat
/// Catalog every 5th successful poll (~5 minutes), captured by
/// `SessionTracker::update`. Cleared on `reset()`.
#[derive(Debug, Default)]
pub struct SessionHistory {
    entries: Vec<HistorySnapshot>,
}

impl SessionHistory {
    pub fn entries(&self) -> &[HistorySnapshot] {
        &self.entries
    }
}

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
    combat_duration: Duration,
    combat_sample: Option<(Instant, bool)>,
    history: SessionHistory,
    poll_count: u64,
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

        self.poll_count += 1;
        if self.poll_count.is_multiple_of(HISTORY_SNAPSHOT_INTERVAL_TICKS) {
            self.record_history_snapshot();
        }
    }

    fn record_history_snapshot(&mut self) {
        let values = crate::stats::STAT_CATALOG
            .iter()
            .map(|stat| (stat.id, self.session_amount(stat.id)))
            .collect();
        self.history.entries.push(HistorySnapshot { elapsed: self.elapsed(), values });
    }

    pub fn history(&self) -> &SessionHistory {
        &self.history
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

    /// The session-scoped number for any stat id - the single place that
    /// knows about the MumbleLink-sourced stats (Session Timer, Combat
    /// Time, Distance Traveled aren't diffed against a lifetime baseline
    /// like everything else) and the Ratio Stats (KDR, PvP KDR are
    /// computed from their own session-scoped inputs, not diffed
    /// directly). Everything else falls through to `session_value`.
    pub fn session_amount(&self, id: &str) -> f64 {
        match id {
            "session_timer" => self.elapsed().as_secs_f64(),
            "combat_time" => self.combat_time_elapsed().as_secs_f64(),
            "distance_traveled" => self.distance_traveled_meters(),
            "kdr" => ratio_with_fallback(self.session_value("kills"), self.session_value("deaths")),
            "pvp_kdr" => ratio_with_fallback(self.session_value("pvp_kills"), self.session_value("deaths")),
            _ => self.session_value(id),
        }
    }

    fn rate_over(value: f64, elapsed: Duration) -> f64 {
        let elapsed_hours = elapsed.as_secs_f64() / 3600.0;
        if elapsed_hours <= 0.0 {
            0.0
        } else {
            value / elapsed_hours
        }
    }

    /// Session Rate: `session_amount(id) / elapsed_hours`, `0.0` before the
    /// session has accumulated any elapsed time. Not meaningful for every
    /// stat - see `stats::has_rate` for which ids should actually display
    /// this. Recomputed against the live elapsed time on every call, so it
    /// changes continuously - see `displayed_rate` for the stabler number
    /// UI should actually show.
    pub fn session_rate(&self, id: &str) -> f64 {
        Self::rate_over(self.session_amount(id), self.elapsed())
    }

    /// `session_rate`, but sampled from the most recent History Snapshot
    /// instead of the live elapsed time, so it only changes once per
    /// Snapshot (~5 minutes) instead of drifting every frame. Falls back
    /// to the live `session_rate` before the first Snapshot exists.
    pub fn displayed_rate(&self, id: &str) -> f64 {
        match self.history.entries.last() {
            Some(snapshot) => Self::rate_over(snapshot.values.get(id).copied().unwrap_or(0.0), snapshot.elapsed),
            None => self.session_rate(id),
        }
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

    /// Feeds in the live in-combat flag (MumbleLink `context.ui_state &
    /// IS_IN_COMBAT`) and accumulates the time spent in combat since the
    /// last sample. The interval since the *previous* sample is added only
    /// if the player was in combat for that whole interval (i.e.
    /// `in_combat` was true on the previous call) - this call's own
    /// `in_combat` only takes effect starting from the *next* sample.
    pub fn sample_combat_state(&mut self, in_combat: bool) {
        let now = Instant::now();
        if let Some((last_at, was_in_combat)) = self.combat_sample {
            if was_in_combat {
                self.combat_duration += now.duration_since(last_at);
            }
        }
        self.combat_sample = Some((now, in_combat));
    }

    pub fn combat_time_elapsed(&self) -> Duration {
        self.combat_duration
    }

    /// Re-baselines to the current lifetime values, so every stat's
    /// session value restarts at zero immediately (rather than waiting
    /// for the next poll to naturally re-baseline, which only happens
    /// when there's no baseline at all yet).
    pub fn reset(&mut self) {
        self.baseline = Some(self.lifetime.clone());
        self.started_at = Some(Instant::now());
        self.distance_meters = 0.0;
        if let Some((_, was_in_combat)) = self.combat_sample {
            self.combat_sample = Some((Instant::now(), was_in_combat));
        }
        self.combat_duration = Duration::ZERO;
        self.history.entries.clear();
        self.poll_count = 0;
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

    #[test]
    fn first_combat_sample_adds_no_duration() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(true);
        assert_eq!(tracker.combat_time_elapsed(), Duration::ZERO);
    }

    #[test]
    fn two_in_combat_samples_accumulate_elapsed_time() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(true);
        assert!(tracker.combat_time_elapsed() >= Duration::from_millis(20));
        assert!(tracker.combat_time_elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn sample_after_out_of_combat_adds_no_duration() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(false);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(true);
        assert_eq!(tracker.combat_time_elapsed(), Duration::ZERO);
    }

    #[test]
    fn leaving_combat_stops_further_accumulation() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(false);
        let after_leaving = tracker.combat_time_elapsed();
        assert!(after_leaving >= Duration::from_millis(20));

        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(false);
        assert_eq!(tracker.combat_time_elapsed(), after_leaving);
    }

    #[test]
    fn reset_mid_combat_zeroes_duration_without_leaking_pre_reset_gap() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(true);
        assert!(tracker.combat_time_elapsed() >= Duration::from_millis(20));

        tracker.reset();
        assert_eq!(tracker.combat_time_elapsed(), Duration::ZERO);

        tracker.sample_combat_state(true);
        assert!(tracker.combat_time_elapsed() < Duration::from_millis(20));
    }

    #[test]
    fn session_amount_falls_through_to_session_value_for_ordinary_stats() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("gold", 100.0)]));
        tracker.update(values(&[("gold", 130.0)]));
        assert_eq!(tracker.session_amount("gold"), 30.0);
    }

    #[test]
    fn session_amount_uses_elapsed_seconds_for_session_timer() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        let amount = tracker.session_amount("session_timer");
        assert!((0.0..1.0).contains(&amount));
    }

    #[test]
    fn session_amount_uses_combat_time_elapsed_seconds_for_combat_time() {
        let mut tracker = SessionTracker::new();
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(true);
        assert!(tracker.session_amount("combat_time") >= 0.02);
    }

    #[test]
    fn session_amount_uses_distance_traveled_meters_for_distance_traveled() {
        let mut tracker = SessionTracker::new();
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([3.0, 4.0, 0.0]);
        assert_eq!(tracker.session_amount("distance_traveled"), 5.0);
    }

    #[test]
    fn session_amount_computes_kdr_from_session_kills_and_deaths() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0), ("deaths", 20.0)]));
        tracker.update(values(&[("kills", 108.0), ("deaths", 22.0)]));
        assert_eq!(tracker.session_amount("kdr"), 4.0);
    }

    #[test]
    fn session_amount_computes_pvp_kdr_from_session_pvp_kills_and_shared_deaths() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("pvp_kills", 50.0), ("deaths", 10.0)]));
        tracker.update(values(&[("pvp_kills", 60.0), ("deaths", 15.0)]));
        assert_eq!(tracker.session_amount("pvp_kdr"), 2.0);
    }

    #[test]
    fn session_rate_is_zero_before_the_session_has_started() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.session_rate("gold"), 0.0);
    }

    #[test]
    fn displayed_rate_uses_the_most_recent_history_snapshot_not_live_elapsed() {
        let mut tracker = SessionTracker::new();
        tracker.history.entries.push(HistorySnapshot { elapsed: Duration::from_secs(1800), values: values(&[("kills", 15.0)]) });
        assert_eq!(tracker.displayed_rate("kills"), 30.0);
    }

    #[test]
    fn displayed_rate_uses_the_last_snapshot_when_several_exist() {
        let mut tracker = SessionTracker::new();
        tracker.history.entries.push(HistorySnapshot { elapsed: Duration::from_secs(1800), values: values(&[("kills", 15.0)]) });
        tracker.history.entries.push(HistorySnapshot { elapsed: Duration::from_secs(3600), values: values(&[("kills", 40.0)]) });
        assert_eq!(tracker.displayed_rate("kills"), 40.0);
    }

    #[test]
    fn displayed_rate_falls_back_to_the_live_session_rate_before_any_snapshot_exists() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.displayed_rate("gold"), tracker.session_rate("gold"));
    }

    #[test]
    fn history_has_no_entries_before_the_fifth_update() {
        let mut tracker = SessionTracker::new();
        for i in 0..4 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert!(tracker.history().entries().is_empty());
    }

    #[test]
    fn history_records_a_snapshot_on_the_fifth_update() {
        let mut tracker = SessionTracker::new();
        for i in 0..5 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert_eq!(tracker.history().entries().len(), 1);
    }

    #[test]
    fn history_records_a_snapshot_every_fifth_update_thereafter() {
        let mut tracker = SessionTracker::new();
        for i in 0..10 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert_eq!(tracker.history().entries().len(), 2);
    }

    #[test]
    fn history_snapshot_captures_session_amount_for_every_stat() {
        let mut tracker = SessionTracker::new();
        for i in 0..5 {
            tracker.update(values(&[("kills", 100.0 + i as f64)]));
        }
        let snapshot = &tracker.history().entries()[0];
        assert_eq!(snapshot.values["kills"], tracker.session_amount("kills"));
    }

    #[test]
    fn reset_clears_history_and_poll_count() {
        let mut tracker = SessionTracker::new();
        for i in 0..5 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert_eq!(tracker.history().entries().len(), 1);

        tracker.reset();
        assert!(tracker.history().entries().is_empty());

        for i in 0..4 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert!(tracker.history().entries().is_empty());
    }
}
