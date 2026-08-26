use crate::config::{AutomaticResetMode, Weekday};

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;
const SECONDS_PER_WEEK: u64 = SECONDS_PER_DAY * 7;

/// Whether a persisted Session (last saved at `saved_at_unix`) should carry
/// forward into this addon load, or be discarded in favor of a fresh one -
/// the whole point of Automatic Reset Mode. Evaluated once at load, against
/// wall-clock `now_unix` (both Unix seconds, UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetDecision {
    Restore,
    Discard,
}

pub struct WeeklySchedule {
    pub day: Weekday,
    pub hour: u32,
    pub minute: u32,
}

pub fn decide(
    mode: AutomaticResetMode,
    minutes: u32,
    weekly: WeeklySchedule,
    saved_at_unix: u64,
    now_unix: u64,
) -> ResetDecision {
    match mode {
        AutomaticResetMode::OnLoad => ResetDecision::Discard,
        AutomaticResetMode::Never => ResetDecision::Restore,
        AutomaticResetMode::MinutesAfterUnload => {
            let threshold = u64::from(minutes) * SECONDS_PER_MINUTE;
            if now_unix.saturating_sub(saved_at_unix) >= threshold {
                ResetDecision::Discard
            } else {
                ResetDecision::Restore
            }
        }
        AutomaticResetMode::Daily => boundary_decision(most_recent_daily_boundary(now_unix, 0, 0), saved_at_unix),
        AutomaticResetMode::Weekly => {
            let boundary = most_recent_weekly_boundary(now_unix, weekly.day, weekly.hour, weekly.minute);
            boundary_decision(boundary, saved_at_unix)
        }
    }
}

/// A scheduled boundary that happened after the Session was last saved
/// means it's due for an automatic reset; one that happened before (or
/// exactly at) the save means nothing's changed since.
fn boundary_decision(most_recent_boundary_unix: u64, saved_at_unix: u64) -> ResetDecision {
    if most_recent_boundary_unix > saved_at_unix {
        ResetDecision::Discard
    } else {
        ResetDecision::Restore
    }
}

/// The most recent instant at `hour:minute` UTC that is `<= now_unix`.
fn most_recent_daily_boundary(now_unix: u64, hour: u32, minute: u32) -> u64 {
    let day = now_unix / SECONDS_PER_DAY;
    let candidate = day * SECONDS_PER_DAY + u64::from(hour) * SECONDS_PER_HOUR + u64::from(minute) * SECONDS_PER_MINUTE;
    if candidate <= now_unix {
        candidate
    } else {
        candidate - SECONDS_PER_DAY
    }
}

/// The most recent instant at `day hour:minute` UTC that is `<= now_unix`.
/// 1970-01-01 (Unix day 0) was a Thursday, index 3 in the Monday=0..Sunday=6
/// scheme `Weekday::index` uses.
fn most_recent_weekly_boundary(now_unix: u64, day: Weekday, hour: u32, minute: u32) -> u64 {
    const EPOCH_WEEKDAY_INDEX: u64 = 3;
    let today = now_unix / SECONDS_PER_DAY;
    let today_weekday_index = (today + EPOCH_WEEKDAY_INDEX) % 7;
    let target_index = u64::from(day.index());
    let days_since_target = (today_weekday_index + 7 - target_index) % 7;
    let candidate_day = today - days_since_target;
    let candidate =
        candidate_day * SECONDS_PER_DAY + u64::from(hour) * SECONDS_PER_HOUR + u64::from(minute) * SECONDS_PER_MINUTE;
    if candidate <= now_unix {
        candidate
    } else {
        candidate - SECONDS_PER_WEEK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weekly(day: Weekday, hour: u32, minute: u32) -> WeeklySchedule {
        WeeklySchedule { day, hour, minute }
    }

    fn default_weekly() -> WeeklySchedule {
        weekly(Weekday::Monday, 7, 30)
    }

    #[test]
    fn on_load_always_discards() {
        assert_eq!(decide(AutomaticResetMode::OnLoad, 30, default_weekly(), 1_000, 1_000), ResetDecision::Discard);
        assert_eq!(decide(AutomaticResetMode::OnLoad, 30, default_weekly(), 1_000, 0), ResetDecision::Discard);
    }

    #[test]
    fn never_always_restores() {
        assert_eq!(decide(AutomaticResetMode::Never, 30, default_weekly(), 0, 10_000_000), ResetDecision::Restore);
    }

    #[test]
    fn minutes_after_unload_restores_before_the_threshold() {
        let saved_at = 1_000;
        let now = saved_at + 29 * 60;
        assert_eq!(
            decide(AutomaticResetMode::MinutesAfterUnload, 30, default_weekly(), saved_at, now),
            ResetDecision::Restore
        );
    }

    #[test]
    fn minutes_after_unload_discards_at_the_threshold() {
        let saved_at = 1_000;
        let now = saved_at + 30 * 60;
        assert_eq!(
            decide(AutomaticResetMode::MinutesAfterUnload, 30, default_weekly(), saved_at, now),
            ResetDecision::Discard
        );
    }

    #[test]
    fn minutes_after_unload_discards_past_the_threshold() {
        let saved_at = 1_000;
        let now = saved_at + 45 * 60;
        assert_eq!(
            decide(AutomaticResetMode::MinutesAfterUnload, 30, default_weekly(), saved_at, now),
            ResetDecision::Discard
        );
    }

    #[test]
    fn most_recent_daily_boundary_is_today_when_past_midnight() {
        // 2024-01-02 12:00:00 UTC
        let now = 1_704_196_800;
        let boundary = most_recent_daily_boundary(now, 0, 0);
        // 2024-01-02 00:00:00 UTC
        assert_eq!(boundary, 1_704_153_600);
    }

    #[test]
    fn daily_restores_when_saved_after_todays_boundary() {
        let today_midnight = 1_704_153_600;
        let saved_at = today_midnight + 3600;
        let now = today_midnight + 7200;
        assert_eq!(decide(AutomaticResetMode::Daily, 30, default_weekly(), saved_at, now), ResetDecision::Restore);
    }

    #[test]
    fn daily_discards_when_saved_before_todays_boundary() {
        let today_midnight = 1_704_153_600;
        let saved_at = today_midnight - 3600;
        let now = today_midnight + 3600;
        assert_eq!(decide(AutomaticResetMode::Daily, 30, default_weekly(), saved_at, now), ResetDecision::Discard);
    }

    #[test]
    fn epoch_day_zero_is_thursday() {
        // 1970-01-01 was a Thursday - Weekday::Thursday's boundary for
        // "today" (unix day 0) should land within that same day.
        let noon_on_epoch_day = 12 * SECONDS_PER_HOUR;
        let boundary = most_recent_weekly_boundary(noon_on_epoch_day, Weekday::Thursday, 0, 0);
        assert_eq!(boundary, 0);
    }

    #[test]
    fn monday_0730_utc_matches_a_known_monday() {
        // 2024-01-01 00:00:00 UTC was a Monday.
        let known_monday_midnight = 1_704_067_200;
        let now = known_monday_midnight + 8 * SECONDS_PER_HOUR; // 08:00 that Monday
        let boundary = most_recent_weekly_boundary(now, Weekday::Monday, 7, 30);
        assert_eq!(boundary, known_monday_midnight + 7 * SECONDS_PER_HOUR + 30 * SECONDS_PER_MINUTE);
    }

    #[test]
    fn weekly_boundary_before_this_weeks_scheduled_time_falls_back_a_week() {
        // Same known Monday, but before 07:30 UTC - the most recent
        // boundary should be the *previous* Monday's, not today's (which
        // hasn't happened yet).
        let known_monday_midnight = 1_704_067_200;
        let now = known_monday_midnight + 6 * SECONDS_PER_HOUR; // 06:00, before 07:30
        let boundary = most_recent_weekly_boundary(now, Weekday::Monday, 7, 30);
        assert_eq!(boundary, known_monday_midnight - SECONDS_PER_WEEK + 7 * SECONDS_PER_HOUR + 30 * SECONDS_PER_MINUTE);
    }

    #[test]
    fn weekly_restores_when_saved_after_this_weeks_boundary() {
        let known_monday_0730 = 1_704_067_200 + 7 * SECONDS_PER_HOUR + 30 * SECONDS_PER_MINUTE;
        let saved_at = known_monday_0730 + 60;
        let now = known_monday_0730 + 3600;
        assert_eq!(decide(AutomaticResetMode::Weekly, 30, default_weekly(), saved_at, now), ResetDecision::Restore);
    }

    #[test]
    fn weekly_discards_when_saved_before_this_weeks_boundary() {
        let known_monday_0730 = 1_704_067_200 + 7 * SECONDS_PER_HOUR + 30 * SECONDS_PER_MINUTE;
        let saved_at = known_monday_0730 - 60;
        let now = known_monday_0730 + 3600;
        assert_eq!(decide(AutomaticResetMode::Weekly, 30, default_weekly(), saved_at, now), ResetDecision::Discard);
    }

    #[test]
    fn weekly_respects_a_different_configured_day() {
        // Friday 18:00 UTC (roughly the EU WvW reset), same reference week.
        let known_monday_midnight = 1_704_067_200;
        let friday_1800 = known_monday_midnight + 4 * SECONDS_PER_DAY + 18 * SECONDS_PER_HOUR;
        let saved_at = friday_1800 - 60;
        let now = friday_1800 + 60;
        assert_eq!(
            decide(AutomaticResetMode::Weekly, 30, weekly(Weekday::Friday, 18, 0), saved_at, now),
            ResetDecision::Discard
        );
    }
}
