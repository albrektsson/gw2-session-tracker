use crate::stats::{StatDef, PVP_RANK_TIERS, STAT_CATALOG};

/// Resolves persisted selected-stat ids into catalog entries, in the
/// user's chosen order. Ids that no longer exist in `STAT_CATALOG` (e.g. a
/// stat later removed from the catalog) are silently dropped.
pub fn resolve_selected_stats(selected_ids: &[String]) -> Vec<&'static StatDef> {
    selected_ids
        .iter()
        .filter_map(|id| STAT_CATALOG.iter().find(|s| s.id == id.as_str()))
        .collect()
}

/// Every real GW2 icon URL a currently selected stat could render, for
/// prefetching into the on-disk icon cache. Includes all 9 PvP rank tier
/// badges whenever "pvp_rank" is selected, since the tier shown depends on
/// live rank and can change mid-session (e.g. a rank-up) - caching only the
/// currently active tier would leave a gap the next time the player crosses
/// a tier threshold.
pub fn icon_urls_for_selected(selected_ids: &[String]) -> Vec<&'static str> {
    let selected = resolve_selected_stats(selected_ids);
    let mut urls: Vec<&'static str> = selected.iter().filter_map(|s| s.icon_url).collect();
    if selected.iter().any(|s| s.id == "pvp_rank") {
        urls.extend(PVP_RANK_TIERS.iter().map(|t| t.icon_url));
    }
    urls
}

/// Toggles `id` in `selected`: appends if absent, removes if present.
/// No-op if `id` isn't a valid `STAT_CATALOG` id.
pub fn toggle_stat(selected: &mut Vec<String>, id: &str) {
    if !STAT_CATALOG.iter().any(|s| s.id == id) {
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
    *selected = STAT_CATALOG.iter().map(|s| s.id.to_string()).collect();
}

/// Clears the selection.
pub fn unselect_all(selected: &mut Vec<String>) {
    selected.clear();
}

/// Adds every id in `ids` to `selected` that isn't already present
/// (appended in `ids` order). Used for "select all in category" buttons,
/// where `ids` is a subset of the catalog rather than all of it.
pub fn select_ids(selected: &mut Vec<String>, ids: &[&str]) {
    for id in ids {
        if !selected.iter().any(|s| s == id) {
            selected.push(id.to_string());
        }
    }
}

/// Removes every id in `ids` from `selected`. Used for "unselect all in
/// category" buttons.
pub fn unselect_ids(selected: &mut Vec<String>, ids: &[&str]) {
    selected.retain(|s| !ids.contains(&s.as_str()));
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

/// Moves `id` to sit immediately before `before_id` in `selected`. No-op
/// if they're equal, or either isn't present.
pub fn move_stat_to(selected: &mut Vec<String>, id: &str, before_id: &str) {
    if id == before_id {
        return;
    }
    let Some(from) = selected.iter().position(|s| s == id) else {
        return;
    };
    if !selected.iter().any(|s| s == before_id) {
        return;
    }
    let item = selected.remove(from);
    let insert_at = selected.iter().position(|s| s == before_id).unwrap();
    selected.insert(insert_at, item);
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn icon_urls_for_selected_includes_real_icon_stats_only() {
        let selected = vec!["gold".to_string(), "kills".to_string()];
        let urls = icon_urls_for_selected(&selected);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap().icon_url.unwrap());
    }

    #[test]
    fn icon_urls_for_selected_adds_all_pvp_rank_tiers_when_pvp_rank_selected() {
        let selected = vec!["pvp_rank".to_string()];
        let urls = icon_urls_for_selected(&selected);
        assert_eq!(urls.len(), PVP_RANK_TIERS.len());
        for tier in PVP_RANK_TIERS {
            assert!(urls.contains(&tier.icon_url));
        }
    }

    #[test]
    fn icon_urls_for_selected_empty_when_nothing_selected() {
        assert!(icon_urls_for_selected(&[]).is_empty());
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
    fn select_ids_adds_only_missing_ids() {
        let mut selected = vec!["kills".to_string()];
        select_ids(&mut selected, &["kills", "deaths", "kdr"]);
        assert_eq!(selected, vec!["kills", "deaths", "kdr"]);
    }

    #[test]
    fn unselect_ids_removes_only_listed_ids() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        unselect_ids(&mut selected, &["deaths", "kdr"]);
        assert_eq!(selected, vec!["kills"]);
    }

    #[test]
    fn select_all_yields_full_catalog_in_order() {
        let mut selected = vec![];
        select_all(&mut selected);
        assert_eq!(selected, STAT_CATALOG.iter().map(|s| s.id.to_string()).collect::<Vec<_>>());
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

    #[test]
    fn move_stat_to_moves_forward() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_to(&mut selected, "kills", "kdr");
        assert_eq!(selected, vec!["deaths", "kills", "kdr"]);
    }

    #[test]
    fn move_stat_to_moves_backward() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_to(&mut selected, "kdr", "deaths");
        assert_eq!(selected, vec!["kills", "kdr", "deaths"]);
    }

    #[test]
    fn move_stat_to_is_noop_for_same_id() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_to(&mut selected, "kills", "kills");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_to_is_noop_for_absent_ids() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_to(&mut selected, "kdr", "deaths");
        assert_eq!(selected, vec!["kills", "deaths"]);
        move_stat_to(&mut selected, "kills", "kdr");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }
}
