use crate::api::ApiSnapshot;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatSource {
    Achievement(u32),
    WvwRank,
    Deaths,
    Kdr,
}

#[derive(Debug, Clone, Copy)]
pub struct StatDef {
    pub id: &'static str,
    pub display_name: &'static str,
    pub source: StatSource,
}

pub const WVW_STATS: &[StatDef] = &[
    StatDef { id: "kills", display_name: "Kills", source: StatSource::Achievement(283) },
    StatDef { id: "deaths", display_name: "Deaths", source: StatSource::Deaths },
    StatDef { id: "kdr", display_name: "KDR", source: StatSource::Kdr },
    StatDef { id: "wvw_rank", display_name: "WvW Rank", source: StatSource::WvwRank },
    StatDef { id: "supply_repair", display_name: "Supply (Repair)", source: StatSource::Achievement(306) },
    StatDef { id: "dolyaks_killed", display_name: "Dolyaks Killed", source: StatSource::Achievement(288) },
    StatDef { id: "dolyaks_escorted", display_name: "Dolyaks Escorted", source: StatSource::Achievement(285) },
    StatDef { id: "camps_captured", display_name: "Camps Captured", source: StatSource::Achievement(291) },
    StatDef { id: "camps_defended", display_name: "Camps Defended", source: StatSource::Achievement(310) },
    StatDef { id: "towers_captured", display_name: "Towers Captured", source: StatSource::Achievement(297) },
    StatDef { id: "towers_defended", display_name: "Towers Defended", source: StatSource::Achievement(322) },
    StatDef { id: "keeps_captured", display_name: "Keeps Captured", source: StatSource::Achievement(300) },
    StatDef { id: "keeps_defended", display_name: "Keeps Defended", source: StatSource::Achievement(316) },
    StatDef { id: "castles_captured", display_name: "Castles Captured", source: StatSource::Achievement(294) },
    StatDef { id: "castles_defended", display_name: "Castles Defended", source: StatSource::Achievement(313) },
    StatDef { id: "objectives_captured", display_name: "Objectives Captured", source: StatSource::Achievement(303) },
    StatDef { id: "objectives_defended", display_name: "Objectives Defended", source: StatSource::Achievement(319) },
];

pub fn compute_lifetime_values(snapshot: &ApiSnapshot) -> HashMap<&'static str, f64> {
    let mut values = HashMap::new();
    for stat in WVW_STATS {
        let value = match stat.source {
            StatSource::Achievement(id) => {
                snapshot.achievements.get(&id).copied().unwrap_or(0) as f64
            }
            StatSource::WvwRank => snapshot.wvw_rank as f64,
            StatSource::Deaths => snapshot.total_deaths as f64,
            StatSource::Kdr => continue, // computed below once kills/deaths are known
        };
        values.insert(stat.id, value);
    }

    let kills = values.get("kills").copied().unwrap_or(0.0);
    let deaths = values.get("deaths").copied().unwrap_or(0.0);
    let kdr = if deaths > 0.0 { kills / deaths } else { kills };
    values.insert("kdr", kdr);

    values
}

/// Resolves persisted selected-stat ids into catalog entries, in the
/// user's chosen order. Ids that no longer exist in `WVW_STATS` (e.g. a
/// stat later removed from the catalog) are silently dropped.
pub fn resolve_selected_stats(selected_ids: &[String]) -> Vec<&'static StatDef> {
    selected_ids
        .iter()
        .filter_map(|id| WVW_STATS.iter().find(|s| s.id == id.as_str()))
        .collect()
}

/// Toggles `id` in `selected`: appends if absent, removes if present.
/// No-op if `id` isn't a valid `WVW_STATS` id.
pub fn toggle_stat(selected: &mut Vec<String>, id: &str) {
    if !WVW_STATS.iter().any(|s| s.id == id) {
        return;
    }
    match selected.iter().position(|s| s == id) {
        Some(pos) => {
            selected.remove(pos);
        }
        None => selected.push(id.to_string()),
    }
}

/// Selects every stat in the catalog, in catalog order.
pub fn select_all(selected: &mut Vec<String>) {
    *selected = WVW_STATS.iter().map(|s| s.id.to_string()).collect();
}

/// Clears the selection.
pub fn unselect_all(selected: &mut Vec<String>) {
    selected.clear();
}

/// Swaps `id` with its predecessor in `selected`. No-op if `id` is
/// already first, or isn't present.
pub fn move_stat_up(selected: &mut [String], id: &str) {
    if let Some(pos) = selected.iter().position(|s| s == id) {
        if pos > 0 {
            selected.swap(pos, pos - 1);
        }
    }
}

/// Swaps `id` with its successor in `selected`. No-op if `id` is already
/// last, or isn't present.
pub fn move_stat_down(selected: &mut [String], id: &str) {
    if let Some(pos) = selected.iter().position(|s| s == id) {
        if pos + 1 < selected.len() {
            selected.swap(pos, pos + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiSnapshot;
    use std::collections::HashMap as StdHashMap;

    fn snapshot(wvw_rank: u64, achievements: &[(u32, u64)], total_deaths: u64) -> ApiSnapshot {
        let mut map = StdHashMap::new();
        for (id, value) in achievements {
            map.insert(*id, *value);
        }
        ApiSnapshot { wvw_rank, achievements: map, total_deaths }
    }

    #[test]
    fn maps_achievement_ids_to_stat_values() {
        let snap = snapshot(0, &[(283, 500), (306, 12000)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kills"], 500.0);
        assert_eq!(values["supply_repair"], 12000.0);
    }

    #[test]
    fn missing_achievement_defaults_to_zero() {
        let snap = snapshot(0, &[], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["dolyaks_killed"], 0.0);
    }

    #[test]
    fn computes_kdr_from_kills_and_deaths() {
        let snap = snapshot(0, &[(283, 100)], 25);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kdr"], 4.0);
    }

    #[test]
    fn kdr_falls_back_to_kills_when_no_deaths() {
        let snap = snapshot(0, &[(283, 7)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kdr"], 7.0);
    }

    #[test]
    fn maps_wvw_rank_and_deaths() {
        let snap = snapshot(1500, &[], 42);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["wvw_rank"], 1500.0);
        assert_eq!(values["deaths"], 42.0);
    }

    #[test]
    fn catalog_has_all_seventeen_stats() {
        assert_eq!(WVW_STATS.len(), 17);
    }

    #[test]
    fn resolve_selected_stats_preserves_order() {
        let selected = vec!["kdr".to_string(), "kills".to_string()];
        let resolved = resolve_selected_stats(&selected);
        assert_eq!(resolved.iter().map(|s| s.id).collect::<Vec<_>>(), vec!["kdr", "kills"]);
    }

    #[test]
    fn resolve_selected_stats_skips_unknown_ids() {
        let selected = vec!["kills".to_string(), "not_a_real_stat".to_string(), "deaths".to_string()];
        let resolved = resolve_selected_stats(&selected);
        assert_eq!(resolved.iter().map(|s| s.id).collect::<Vec<_>>(), vec!["kills", "deaths"]);
    }

    #[test]
    fn resolve_selected_stats_empty_input_yields_empty_output() {
        assert!(resolve_selected_stats(&[]).is_empty());
    }

    #[test]
    fn toggle_stat_adds_then_removes() {
        let mut selected = vec![];
        toggle_stat(&mut selected, "kills");
        assert_eq!(selected, vec!["kills"]);
        toggle_stat(&mut selected, "kills");
        assert!(selected.is_empty());
    }

    #[test]
    fn toggle_stat_ignores_unknown_id() {
        let mut selected = vec![];
        toggle_stat(&mut selected, "not_a_real_stat");
        assert!(selected.is_empty());
    }

    #[test]
    fn select_all_yields_full_catalog_in_order() {
        let mut selected = vec![];
        select_all(&mut selected);
        assert_eq!(selected, WVW_STATS.iter().map(|s| s.id.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn unselect_all_clears() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        unselect_all(&mut selected);
        assert!(selected.is_empty());
    }

    #[test]
    fn move_stat_up_swaps_with_predecessor() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_up(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "kdr", "deaths"]);
    }

    #[test]
    fn move_stat_down_swaps_with_successor() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_down(&mut selected, "kills");
        assert_eq!(selected, vec!["deaths", "kills", "kdr"]);
    }

    #[test]
    fn move_stat_up_is_noop_when_already_first() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_up(&mut selected, "kills");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_up_is_noop_for_absent_id() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_up(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_down_is_noop_when_already_last() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_down(&mut selected, "deaths");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_down_is_noop_for_absent_id() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_down(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }
}
