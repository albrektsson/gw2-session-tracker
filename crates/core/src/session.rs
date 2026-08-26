use crate::map_context::MapGroup;
use crate::stats::ratio_with_fallback;
use serde::{Deserialize, Serialize};
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
/// drift out of sync with the formula used everywhere else. `group_elapsed`
/// and `group_values` are the same idea, per `MapGroup`, for map-scoped
/// rates.
#[derive(Debug, Clone)]
pub struct HistorySnapshot {
    pub elapsed: Duration,
    pub values: HashMap<&'static str, f64>,
    pub group_elapsed: HashMap<MapGroup, Duration>,
    pub group_values: HashMap<MapGroup, HashMap<&'static str, f64>>,
}

impl HistorySnapshot {
    fn to_owned(&self) -> HistorySnapshotOwned {
        HistorySnapshotOwned {
            elapsed: self.elapsed,
            values: owned_map(&self.values),
            group_elapsed: self.group_elapsed.clone(),
            group_values: self.group_values.iter().map(|(&group, values)| (group, owned_map(values))).collect(),
        }
    }
}

/// `HistorySnapshot` with owned string keys instead of `&'static str`, for
/// serializing into a persisted Session state file. See `owned_map`/
/// `interned_map` for the conversion at either boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySnapshotOwned {
    pub elapsed: Duration,
    pub values: HashMap<String, f64>,
    pub group_elapsed: HashMap<MapGroup, Duration>,
    pub group_values: HashMap<MapGroup, HashMap<String, f64>>,
}

impl HistorySnapshotOwned {
    fn into_interned(self) -> HistorySnapshot {
        HistorySnapshot {
            elapsed: self.elapsed,
            values: interned_map(self.values),
            group_elapsed: self.group_elapsed,
            group_values: self.group_values.into_iter().map(|(group, values)| (group, interned_map(values))).collect(),
        }
    }
}

fn owned_map(map: &HashMap<&'static str, f64>) -> HashMap<String, f64> {
    map.iter().map(|(&id, &value)| (id.to_string(), value)).collect()
}

/// The inverse of `owned_map` - looks each id up against `STAT_CATALOG` to
/// recover the matching `&'static str`, silently dropping an id the
/// catalog no longer has (a stat removed since the file was written).
fn interned_map(map: HashMap<String, f64>) -> HashMap<&'static str, f64> {
    map.into_iter().filter_map(|(id, value)| crate::stats::static_id(&id).map(|id| (id, value))).collect()
}

/// A `SessionTracker`'s state, serialized to disk so a Session can survive
/// an addon unload/reload (see `SessionTracker::snapshot`/`restore` and
/// `crate::session_state`). Doesn't include the transient MumbleLink
/// sample state (`last_position`, `combat_sample`, `group_sample`) -
/// those re-establish themselves from the next live sample after restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub baseline: HashMap<String, f64>,
    pub lifetime: HashMap<String, f64>,
    pub elapsed: Duration,
    pub distance_meters: f64,
    pub combat_duration: Duration,
    pub group_durations: HashMap<MapGroup, Duration>,
    pub group_attributed: HashMap<MapGroup, HashMap<String, f64>>,
    pub history: Vec<HistorySnapshotOwned>,
    pub poll_count: u64,
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
    /// Elapsed time accumulated across any prior run(s) of this same
    /// Session, before `started_at` (this run's anchor). Set from a
    /// restored `SessionSnapshot`; zero for a Session that started this
    /// run. Keeps `elapsed()` from counting the gap while the addon
    /// wasn't loaded - see `restore`.
    elapsed_before_this_run: Duration,
    distance_meters: f64,
    last_position: Option<[f32; 3]>,
    combat_duration: Duration,
    combat_sample: Option<(Instant, bool)>,
    group_durations: HashMap<MapGroup, Duration>,
    group_sample: Option<(Instant, Option<MapGroup>)>,
    group_attributed: HashMap<MapGroup, HashMap<&'static str, f64>>,
    history: SessionHistory,
    poll_count: u64,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, mut lifetime: HashMap<&'static str, f64>) {
        let current_group = self.current_group();
        for (id, value) in lifetime.iter_mut() {
            if let Some(&old) = self.lifetime.get(id) {
                if *value < old && crate::stats::is_regression_guarded(id) {
                    *value = old;
                }
                if let Some(group) = current_group {
                    *self.group_attributed.entry(group).or_default().entry(*id).or_insert(0.0) += *value - old;
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
        let group_elapsed = MapGroup::ALL.into_iter().map(|g| (g, self.group_elapsed(g))).collect();
        let group_values = MapGroup::ALL
            .into_iter()
            .map(|g| {
                let values = crate::stats::STAT_CATALOG
                    .iter()
                    .map(|stat| (stat.id, self.group_session_amount(g, stat.id)))
                    .collect();
                (g, values)
            })
            .collect();
        self.history.entries.push(HistorySnapshot { elapsed: self.elapsed(), values, group_elapsed, group_values });
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
            "time_in_wvw" => self.group_elapsed(MapGroup::Wvw).as_secs_f64(),
            "time_in_pvp" => self.group_elapsed(MapGroup::Pvp).as_secs_f64(),
            "time_in_pve" => self.group_elapsed(MapGroup::Pve).as_secs_f64(),
            "kdr" => ratio_with_fallback(self.session_value("kills"), self.session_value("deaths")),
            "pvp_kdr" => ratio_with_fallback(self.session_value("pvp_kills"), self.session_value("deaths")),
            _ => self.session_value(id),
        }
    }

    /// The `MapGroup`-scoped equivalent of `session_value`: the sum of
    /// lifetime deltas attributed to `group` across every poll where it was
    /// the current group (see `update`), rather than the whole session's
    /// diff-from-baseline.
    pub fn group_session_value(&self, group: MapGroup, id: &str) -> f64 {
        self.group_attributed.get(&group).and_then(|values| values.get(id)).copied().unwrap_or(0.0)
    }

    /// The `MapGroup`-scoped equivalent of `session_amount`. The
    /// MumbleLink-sourced ids have no per-group variant of their own (they
    /// aren't lifetime-delta-attributed) and just delegate to the plain
    /// whole-session number; Ratio Stats compute from group-attributed
    /// inputs so e.g. a WvW-scoped KDR doesn't mix in PvP/PvE deaths.
    pub fn group_session_amount(&self, group: MapGroup, id: &str) -> f64 {
        match id {
            "session_timer" | "combat_time" | "distance_traveled" | "time_in_wvw" | "time_in_pvp" | "time_in_pve" => {
                self.session_amount(id)
            }
            "kdr" => ratio_with_fallback(self.group_session_value(group, "kills"), self.group_session_value(group, "deaths")),
            "pvp_kdr" => {
                ratio_with_fallback(self.group_session_value(group, "pvp_kills"), self.group_session_value(group, "deaths"))
            }
            _ => self.group_session_value(group, id),
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

    /// The `MapGroup`-scoped equivalent of `session_rate`: `group_session_amount(group, id) /
    /// group_elapsed(group)`.
    pub fn group_session_rate(&self, group: MapGroup, id: &str) -> f64 {
        Self::rate_over(self.group_session_amount(group, id), self.group_elapsed(group))
    }

    /// `session_rate`, but sampled from the most recent History Snapshot
    /// instead of the live elapsed time, so it only changes once per
    /// Snapshot (~5 minutes) instead of drifting every frame. Reads `0.0`
    /// before the first Snapshot exists rather than falling back to the
    /// live `session_rate`, which would be wildly unstable this early -
    /// dividing by a handful of seconds of elapsed time swings hugely for
    /// even a small change in value.
    pub fn displayed_rate(&self, id: &str) -> f64 {
        match self.history.entries.last() {
            Some(snapshot) => Self::rate_over(snapshot.values.get(id).copied().unwrap_or(0.0), snapshot.elapsed),
            None => 0.0,
        }
    }

    /// The `MapGroup`-scoped equivalent of `displayed_rate`, reading the
    /// last History Snapshot's `group_values`/`group_elapsed` for `group`
    /// instead of the whole-session ones - same stability rationale.
    /// Delegates to the plain `displayed_rate` for ids with no group-scoped
    /// variant (see `group_session_amount`).
    pub fn group_displayed_rate(&self, group: MapGroup, id: &str) -> f64 {
        if matches!(id, "distance_traveled" | "time_in_wvw" | "time_in_pvp" | "time_in_pve") {
            return self.displayed_rate(id);
        }
        match self.history.entries.last() {
            Some(snapshot) => {
                let elapsed = snapshot.group_elapsed.get(&group).copied().unwrap_or_default();
                let value = snapshot.group_values.get(&group).and_then(|values| values.get(id)).copied().unwrap_or(0.0);
                Self::rate_over(value, elapsed)
            }
            None => 0.0,
        }
    }

    pub fn has_data(&self) -> bool {
        self.baseline.is_some()
    }

    /// Elapsed time since the session started (the first successful poll,
    /// or the last `reset()`), plus whatever a restored `SessionSnapshot`
    /// carried over from prior runs (`elapsed_before_this_run`) - so time
    /// spent with the addon unloaded between runs of the same Session
    /// doesn't count. Zero if the session hasn't started yet.
    pub fn elapsed(&self) -> Duration {
        self.elapsed_before_this_run + self.started_at.map(|t| t.elapsed()).unwrap_or_default()
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

    /// Feeds in the live `MapGroup` (derived every frame from MumbleLink's
    /// map id) and accumulates the elapsed time since the last sample into
    /// whichever group was current *during that interval* - same boundary
    /// rule as `sample_combat_state`: this call's own group only takes
    /// effect starting from the *next* sample. `None` (loading screens,
    /// character select, unrecognized maps) accumulates into nothing.
    pub fn sample_map_group(&mut self, group: Option<MapGroup>) {
        let now = Instant::now();
        if let Some((last_at, Some(last_group))) = self.group_sample {
            *self.group_durations.entry(last_group).or_default() += now.duration_since(last_at);
        }
        self.group_sample = Some((now, group));
    }

    fn current_group(&self) -> Option<MapGroup> {
        self.group_sample.and_then(|(_, group)| group)
    }

    pub fn group_elapsed(&self, group: MapGroup) -> Duration {
        self.group_durations.get(&group).copied().unwrap_or_default()
    }

    /// Re-baselines to the current lifetime values, so every stat's
    /// session value restarts at zero immediately (rather than waiting
    /// for the next poll to naturally re-baseline, which only happens
    /// when there's no baseline at all yet).
    pub fn reset(&mut self) {
        self.baseline = Some(self.lifetime.clone());
        self.started_at = Some(Instant::now());
        self.elapsed_before_this_run = Duration::ZERO;
        self.distance_meters = 0.0;
        if let Some((_, was_in_combat)) = self.combat_sample {
            self.combat_sample = Some((Instant::now(), was_in_combat));
        }
        self.combat_duration = Duration::ZERO;
        self.group_durations.clear();
        self.group_attributed.clear();
        if let Some((_, last_group)) = self.group_sample {
            self.group_sample = Some((Instant::now(), last_group));
        }
        self.history.entries.clear();
        self.poll_count = 0;
    }

    /// Captures this Session's state for a persisted `SessionSnapshot`, or
    /// `None` before the Session has any data (`has_data`) - nothing
    /// meaningful to carry across a restart yet.
    pub fn snapshot(&self) -> Option<SessionSnapshot> {
        let baseline = self.baseline.as_ref()?;
        Some(SessionSnapshot {
            baseline: owned_map(baseline),
            lifetime: owned_map(&self.lifetime),
            elapsed: self.elapsed(),
            distance_meters: self.distance_meters,
            combat_duration: self.combat_duration,
            group_durations: self.group_durations.clone(),
            group_attributed: self.group_attributed.iter().map(|(&group, values)| (group, owned_map(values))).collect(),
            history: self.history.entries.iter().map(HistorySnapshot::to_owned).collect(),
            poll_count: self.poll_count,
        })
    }

    /// Restores a Session from a `SessionSnapshot` (see `snapshot`),
    /// carrying its elapsed time, baseline, and history forward while
    /// anchoring `started_at` to now - transient MumbleLink sample state
    /// (`last_position`, `combat_sample`, `group_sample`) is left unset,
    /// re-establishing itself from the next live sample.
    pub fn restore(&mut self, snapshot: SessionSnapshot) {
        self.baseline = Some(interned_map(snapshot.baseline));
        self.lifetime = interned_map(snapshot.lifetime);
        self.started_at = Some(Instant::now());
        self.elapsed_before_this_run = snapshot.elapsed;
        self.distance_meters = snapshot.distance_meters;
        self.last_position = None;
        self.combat_duration = snapshot.combat_duration;
        self.combat_sample = None;
        self.group_durations = snapshot.group_durations;
        self.group_sample = None;
        self.group_attributed = snapshot.group_attributed.into_iter().map(|(group, values)| (group, interned_map(values))).collect();
        self.history = SessionHistory { entries: snapshot.history.into_iter().map(HistorySnapshotOwned::into_interned).collect() };
        self.poll_count = snapshot.poll_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&'static str, f64)]) -> std::collections::HashMap<&'static str, f64> {
        pairs.iter().copied().collect()
    }

    fn snapshot(elapsed: Duration, values: HashMap<&'static str, f64>) -> HistorySnapshot {
        HistorySnapshot { elapsed, values, group_elapsed: HashMap::new(), group_values: HashMap::new() }
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
        tracker.history.entries.push(snapshot(Duration::from_secs(1800), values(&[("kills", 15.0)])));
        assert_eq!(tracker.displayed_rate("kills"), 30.0);
    }

    #[test]
    fn displayed_rate_uses_the_last_snapshot_when_several_exist() {
        let mut tracker = SessionTracker::new();
        tracker.history.entries.push(snapshot(Duration::from_secs(1800), values(&[("kills", 15.0)])));
        tracker.history.entries.push(snapshot(Duration::from_secs(3600), values(&[("kills", 40.0)])));
        assert_eq!(tracker.displayed_rate("kills"), 40.0);
    }

    #[test]
    fn displayed_rate_is_zero_before_any_snapshot_exists() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.displayed_rate("gold"), 0.0);
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

    #[test]
    fn first_map_group_sample_adds_no_duration() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert_eq!(tracker.group_elapsed(MapGroup::Wvw), Duration::ZERO);
    }

    #[test]
    fn two_same_group_samples_accumulate_elapsed_time() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert!(tracker.group_elapsed(MapGroup::Wvw) >= Duration::from_millis(20));
        assert!(tracker.group_elapsed(MapGroup::Wvw) < Duration::from_secs(1));
    }

    #[test]
    fn switching_group_attributes_the_interval_to_the_group_active_at_the_start_of_it() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Pvp));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Pvp));

        assert!(tracker.group_elapsed(MapGroup::Wvw) >= Duration::from_millis(20));
        assert!(tracker.group_elapsed(MapGroup::Wvw) < Duration::from_millis(40));
        assert!(tracker.group_elapsed(MapGroup::Pvp) >= Duration::from_millis(20));
        assert!(tracker.group_elapsed(MapGroup::Pvp) < Duration::from_millis(40));
    }

    #[test]
    fn sample_during_map_group_none_adds_no_duration_to_any_group() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(None);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert_eq!(tracker.group_elapsed(MapGroup::Wvw), Duration::ZERO);
    }

    #[test]
    fn leaving_a_group_for_none_stops_further_accumulation() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(None);
        let after_leaving = tracker.group_elapsed(MapGroup::Wvw);
        assert!(after_leaving >= Duration::from_millis(20));

        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(None);
        assert_eq!(tracker.group_elapsed(MapGroup::Wvw), after_leaving);
    }

    #[test]
    fn reset_mid_group_zeroes_duration_without_leaking_pre_reset_gap() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert!(tracker.group_elapsed(MapGroup::Wvw) >= Duration::from_millis(20));

        tracker.reset();
        assert_eq!(tracker.group_elapsed(MapGroup::Wvw), Duration::ZERO);

        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert!(tracker.group_elapsed(MapGroup::Wvw) < Duration::from_millis(20));
    }

    #[test]
    fn update_attributes_lifetime_delta_to_the_current_group() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 7.0);
    }

    #[test]
    fn update_with_no_current_group_attributes_nothing() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 0.0);
    }

    #[test]
    fn first_update_never_attributes_a_delta() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 0.0);
    }

    #[test]
    fn switching_group_between_polls_attributes_each_polls_delta_to_the_group_current_at_that_poll() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));

        tracker.sample_map_group(Some(MapGroup::Pve));
        tracker.update(values(&[("kills", 107.0)]));

        assert_eq!(tracker.group_session_value(MapGroup::Pve, "kills"), 7.0);
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 0.0);
    }

    #[test]
    fn regression_guarded_rollback_attributes_zero_delta() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 90.0)]));
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 0.0);
    }

    #[test]
    fn reset_clears_group_attributed_values() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 7.0);

        tracker.reset();
        assert_eq!(tracker.group_session_value(MapGroup::Wvw, "kills"), 0.0);
    }

    #[test]
    fn group_session_amount_falls_through_to_group_session_value_for_ordinary_stats() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("gold", 100.0)]));
        tracker.update(values(&[("gold", 130.0)]));
        assert_eq!(tracker.group_session_amount(MapGroup::Wvw, "gold"), 30.0);
    }

    #[test]
    fn group_session_amount_computes_group_scoped_kdr() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0), ("deaths", 20.0)]));
        tracker.update(values(&[("kills", 108.0), ("deaths", 22.0)]));
        assert_eq!(tracker.group_session_amount(MapGroup::Wvw, "kdr"), 4.0);
    }

    #[test]
    fn group_session_amount_computes_group_scoped_pvp_kdr() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Pvp));
        tracker.update(values(&[("pvp_kills", 50.0), ("deaths", 10.0)]));
        tracker.update(values(&[("pvp_kills", 60.0), ("deaths", 15.0)]));
        assert_eq!(tracker.group_session_amount(MapGroup::Pvp, "pvp_kdr"), 2.0);
    }

    #[test]
    fn group_session_amount_is_independent_across_groups() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("gold", 100.0)]));
        tracker.sample_map_group(Some(MapGroup::Pve));
        tracker.update(values(&[("gold", 150.0)]));

        assert_eq!(tracker.group_session_amount(MapGroup::Pve, "gold"), 50.0);
        assert_eq!(tracker.group_session_amount(MapGroup::Wvw, "gold"), 0.0);
    }

    #[test]
    fn group_session_amount_delegates_time_in_group_ids_to_session_amount_regardless_of_group_param() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));

        assert_eq!(tracker.group_session_amount(MapGroup::Pvp, "time_in_wvw"), tracker.session_amount("time_in_wvw"));
    }

    #[test]
    fn session_amount_uses_group_elapsed_seconds_for_time_in_wvw() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));

        let amount = tracker.session_amount("time_in_wvw");
        assert!(amount >= 0.02);
        assert_eq!(tracker.session_amount("time_in_pvp"), 0.0);
    }

    #[test]
    fn history_snapshot_captures_group_session_amount_for_every_stat_and_group() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        for i in 0..5 {
            tracker.update(values(&[("kills", 100.0 + i as f64)]));
        }
        let snapshot = &tracker.history().entries()[0];
        assert_eq!(
            snapshot.group_values[&MapGroup::Wvw]["kills"],
            tracker.group_session_amount(MapGroup::Wvw, "kills")
        );
    }

    #[test]
    fn group_displayed_rate_uses_the_most_recent_history_snapshots_group_elapsed() {
        let mut tracker = SessionTracker::new();
        let mut group_elapsed = HashMap::new();
        group_elapsed.insert(MapGroup::Wvw, Duration::from_secs(1800));
        let mut group_values = HashMap::new();
        group_values.insert(MapGroup::Wvw, values(&[("kills", 15.0)]));
        tracker.history.entries.push(HistorySnapshot {
            elapsed: Duration::from_secs(1800),
            values: values(&[("kills", 15.0)]),
            group_elapsed,
            group_values,
        });
        assert_eq!(tracker.group_displayed_rate(MapGroup::Wvw, "kills"), 30.0);
    }

    #[test]
    fn group_displayed_rate_is_zero_before_any_snapshot_exists() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.group_displayed_rate(MapGroup::Wvw, "gold"), 0.0);
    }

    #[test]
    fn reset_clears_group_durations_along_with_history() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));
        assert!(tracker.group_elapsed(MapGroup::Wvw) >= Duration::from_millis(20));

        tracker.reset();
        assert_eq!(tracker.group_elapsed(MapGroup::Wvw), Duration::ZERO);
    }

    #[test]
    fn snapshot_is_none_before_the_session_has_data() {
        let tracker = SessionTracker::new();
        assert!(tracker.snapshot().is_none());
    }

    #[test]
    fn snapshot_is_some_once_the_session_has_data() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 5.0)]));
        assert!(tracker.snapshot().is_some());
    }

    #[test]
    fn restore_round_trips_baseline_and_lifetime() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));
        let snapshot = tracker.snapshot().unwrap();

        let mut restored = SessionTracker::new();
        restored.restore(snapshot);
        assert_eq!(restored.lifetime_value("kills"), 107.0);
        assert_eq!(restored.session_value("kills"), 7.0);
    }

    #[test]
    fn restore_carries_elapsed_time_forward_without_double_counting_the_gap() {
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        std::thread::sleep(Duration::from_millis(20));
        let snapshot = tracker.snapshot().unwrap();
        assert!(snapshot.elapsed >= Duration::from_millis(20));

        // Simulate a gap while the addon was unloaded - restoring right
        // away shouldn't add anything beyond the snapshot's own elapsed.
        let mut restored = SessionTracker::new();
        restored.restore(snapshot.clone());
        assert!(restored.elapsed() >= snapshot.elapsed);
        assert!(restored.elapsed() < snapshot.elapsed + Duration::from_millis(500));
    }

    #[test]
    fn restore_round_trips_distance_combat_and_group_durations() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.sample_position([0.0, 0.0, 0.0]);
        tracker.sample_position([3.0, 4.0, 0.0]);
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_combat_state(true);
        std::thread::sleep(Duration::from_millis(20));
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 1.0)]));

        let snapshot = tracker.snapshot().unwrap();
        let mut restored = SessionTracker::new();
        restored.restore(snapshot);

        assert_eq!(restored.distance_traveled_meters(), 5.0);
        assert!(restored.combat_time_elapsed() >= Duration::from_millis(20));
        assert!(restored.group_elapsed(MapGroup::Wvw) >= Duration::from_millis(20));
    }

    #[test]
    fn restore_round_trips_group_attributed_values() {
        let mut tracker = SessionTracker::new();
        tracker.sample_map_group(Some(MapGroup::Wvw));
        tracker.update(values(&[("kills", 100.0)]));
        tracker.update(values(&[("kills", 107.0)]));

        let snapshot = tracker.snapshot().unwrap();
        let mut restored = SessionTracker::new();
        restored.restore(snapshot);

        assert_eq!(restored.group_session_value(MapGroup::Wvw, "kills"), 7.0);
    }

    #[test]
    fn restore_round_trips_history_log() {
        let mut tracker = SessionTracker::new();
        for i in 0..5 {
            tracker.update(values(&[("kills", i as f64)]));
        }
        assert_eq!(tracker.history().entries().len(), 1);

        let snapshot = tracker.snapshot().unwrap();
        let mut restored = SessionTracker::new();
        restored.restore(snapshot);

        assert_eq!(restored.history().entries().len(), 1);
        assert_eq!(restored.history().entries()[0].values["kills"], tracker.history().entries()[0].values["kills"]);
    }

    #[test]
    fn restore_leaves_transient_mumble_sample_state_unset() {
        // After restore, distance/combat/group tracking should resume
        // cleanly from the next live sample rather than carrying over a
        // stale last-known point from before the restart.
        let mut tracker = SessionTracker::new();
        tracker.sample_position([3.0, 4.0, 0.0]);
        tracker.update(values(&[("kills", 1.0)]));
        let snapshot = tracker.snapshot().unwrap();

        let mut restored = SessionTracker::new();
        restored.restore(snapshot);
        restored.sample_position([3.0, 4.0, 0.0]);
        assert_eq!(restored.distance_traveled_meters(), 0.0);
    }

    #[test]
    fn snapshot_drops_a_stat_id_the_catalog_no_longer_has() {
        let mut map = HashMap::new();
        map.insert("no_longer_a_real_stat".to_string(), 42.0);
        assert!(interned_map(map).is_empty());
    }
}
