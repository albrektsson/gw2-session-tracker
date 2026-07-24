use crate::api::ApiSnapshot;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatSource {
    Achievement(u32),
    WvwRank,
    Deaths,
    Kdr,
    Currency(u32),
    PvpRank,
    PvpWins,
    PvpLosses,
    PvpRankingPoints,
    PvpRankedWins,
    PvpRankedLosses,
    PvpUnrankedWins,
    PvpUnrankedLosses,
    PvpCustomWins,
    PvpCustomLosses,
    PvpKdr,
}

/// A stat's browsing category in the Select Stats picker. Purely a UI
/// grouping concern - a stat can belong to more than one (e.g. every
/// currency is in `Currency`, and some are *also* cross-tagged into the
/// activity that earns them, like `Wvw` or `Pvp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Misc,
    Currency,
    Festival,
    Wvw,
    Pvp,
    OpenWorld,
    Fractal,
    Raid,
    Strike,
}

/// UI grouping of categories into supercategories - not attached to
/// individual stats. (display name, subcategories)
pub const SUPERCATEGORIES: &[(&str, &[Category])] = &[
    ("General", &[Category::Misc, Category::Currency, Category::Festival]),
    ("Competitive", &[Category::Wvw, Category::Pvp]),
    ("PvE", &[Category::OpenWorld, Category::Fractal, Category::Raid, Category::Strike]),
];

#[derive(Debug, Clone, Copy)]
pub struct StatDef {
    pub id: &'static str,
    pub display_name: &'static str,
    pub source: StatSource,
    pub categories: &'static [Category],
}

use Category::{Currency as Cur, Fractal, Misc, OpenWorld, Pvp, Raid, Strike, Wvw};

pub const STAT_CATALOG: &[StatDef] = &[
    // WvW
    StatDef { id: "kills", display_name: "Kills", source: StatSource::Achievement(283), categories: &[Wvw] },
    StatDef { id: "deaths", display_name: "Deaths", source: StatSource::Deaths, categories: &[Wvw, Pvp, Misc] },
    StatDef { id: "kdr", display_name: "KDR", source: StatSource::Kdr, categories: &[Wvw] },
    StatDef { id: "wvw_rank", display_name: "WvW Rank", source: StatSource::WvwRank, categories: &[Wvw] },
    StatDef { id: "supply_repair", display_name: "Supply (Repair)", source: StatSource::Achievement(306), categories: &[Wvw] },
    StatDef { id: "dolyaks_killed", display_name: "Dolyaks Killed", source: StatSource::Achievement(288), categories: &[Wvw] },
    StatDef { id: "dolyaks_escorted", display_name: "Dolyaks Escorted", source: StatSource::Achievement(285), categories: &[Wvw] },
    StatDef { id: "camps_captured", display_name: "Camps Captured", source: StatSource::Achievement(291), categories: &[Wvw] },
    StatDef { id: "camps_defended", display_name: "Camps Defended", source: StatSource::Achievement(310), categories: &[Wvw] },
    StatDef { id: "towers_captured", display_name: "Towers Captured", source: StatSource::Achievement(297), categories: &[Wvw] },
    StatDef { id: "towers_defended", display_name: "Towers Defended", source: StatSource::Achievement(322), categories: &[Wvw] },
    StatDef { id: "keeps_captured", display_name: "Keeps Captured", source: StatSource::Achievement(300), categories: &[Wvw] },
    StatDef { id: "keeps_defended", display_name: "Keeps Defended", source: StatSource::Achievement(316), categories: &[Wvw] },
    StatDef { id: "castles_captured", display_name: "Castles Captured", source: StatSource::Achievement(294), categories: &[Wvw] },
    StatDef { id: "castles_defended", display_name: "Castles Defended", source: StatSource::Achievement(313), categories: &[Wvw] },
    StatDef { id: "objectives_captured", display_name: "Objectives Captured", source: StatSource::Achievement(303), categories: &[Wvw] },
    StatDef { id: "objectives_defended", display_name: "Objectives Defended", source: StatSource::Achievement(319), categories: &[Wvw] },
    // PvP
    StatDef { id: "pvp_kills", display_name: "PvP Kills", source: StatSource::Achievement(239), categories: &[Pvp] },
    StatDef { id: "pvp_kdr", display_name: "PvP KDR", source: StatSource::PvpKdr, categories: &[Pvp] },
    StatDef { id: "pvp_rank", display_name: "PvP Rank", source: StatSource::PvpRank, categories: &[Pvp] },
    StatDef { id: "pvp_ranking_points", display_name: "PvP Ranking Points", source: StatSource::PvpRankingPoints, categories: &[Pvp] },
    StatDef { id: "pvp_wins", display_name: "PvP Total Wins", source: StatSource::PvpWins, categories: &[Pvp] },
    StatDef { id: "pvp_losses", display_name: "PvP Total Losses", source: StatSource::PvpLosses, categories: &[Pvp] },
    StatDef { id: "pvp_ranked_wins", display_name: "PvP Ranked Wins", source: StatSource::PvpRankedWins, categories: &[Pvp] },
    StatDef { id: "pvp_ranked_losses", display_name: "PvP Ranked Losses", source: StatSource::PvpRankedLosses, categories: &[Pvp] },
    StatDef { id: "pvp_unranked_wins", display_name: "PvP Unranked Wins", source: StatSource::PvpUnrankedWins, categories: &[Pvp] },
    StatDef { id: "pvp_unranked_losses", display_name: "PvP Unranked Losses", source: StatSource::PvpUnrankedLosses, categories: &[Pvp] },
    StatDef { id: "pvp_custom_wins", display_name: "PvP Custom Wins", source: StatSource::PvpCustomWins, categories: &[Pvp] },
    StatDef { id: "pvp_custom_losses", display_name: "PvP Custom Losses", source: StatSource::PvpCustomLosses, categories: &[Pvp] },
    // Currencies (all non-obsolete GW2 currencies; some are also
    // cross-tagged into the activity category that earns them)
    StatDef { id: "gold", display_name: "Gold", source: StatSource::Currency(1), categories: &[Cur, Wvw, Pvp, Fractal, Raid, OpenWorld, Strike] },
    StatDef { id: "karma", display_name: "Karma", source: StatSource::Currency(2), categories: &[Cur, Wvw, OpenWorld] },
    StatDef { id: "laurels", display_name: "Laurels", source: StatSource::Currency(3), categories: &[Cur] },
    StatDef { id: "gems", display_name: "Gems", source: StatSource::Currency(4), categories: &[Cur] },
    StatDef { id: "fractal_relic", display_name: "Fractal Relic", source: StatSource::Currency(7), categories: &[Cur, Fractal] },
    StatDef { id: "badges_of_honor", display_name: "Badges of Honor", source: StatSource::Currency(15), categories: &[Cur, Wvw] },
    StatDef { id: "guild_commendation", display_name: "Guild Commendation", source: StatSource::Currency(16), categories: &[Cur] },
    StatDef { id: "transmutation_charge", display_name: "Transmutation Charge", source: StatSource::Currency(18), categories: &[Cur] },
    StatDef { id: "airship_part", display_name: "Airship Part", source: StatSource::Currency(19), categories: &[Cur] },
    StatDef { id: "ley_line_crystal", display_name: "Ley Line Crystal", source: StatSource::Currency(20), categories: &[Cur] },
    StatDef { id: "lump_of_aurillium", display_name: "Lump of Aurillium", source: StatSource::Currency(22), categories: &[Cur] },
    StatDef { id: "spirit_shard", display_name: "Spirit Shard", source: StatSource::Currency(23), categories: &[Cur, OpenWorld] },
    StatDef { id: "pristine_fractal_relic", display_name: "Pristine Fractal Relic", source: StatSource::Currency(24), categories: &[Cur, Fractal] },
    StatDef { id: "geode", display_name: "Geode", source: StatSource::Currency(25), categories: &[Cur] },
    StatDef { id: "wvw_skirmish_tickets", display_name: "WvW Skirmish Claim Tickets", source: StatSource::Currency(26), categories: &[Cur, Wvw] },
    StatDef { id: "bandit_crest", display_name: "Bandit Crest", source: StatSource::Currency(27), categories: &[Cur] },
    StatDef { id: "magnetite_shard", display_name: "Magnetite Shard", source: StatSource::Currency(28), categories: &[Cur, Raid] },
    StatDef { id: "provisioner_token", display_name: "Provisioner Token", source: StatSource::Currency(29), categories: &[Cur] },
    StatDef { id: "pvp_league_tickets", display_name: "PvP League Tickets", source: StatSource::Currency(30), categories: &[Cur, Pvp] },
    StatDef { id: "proof_of_heroics", display_name: "Proof of Heroics", source: StatSource::Currency(31), categories: &[Cur] },
    StatDef { id: "unbound_magic", display_name: "Unbound Magic", source: StatSource::Currency(32), categories: &[Cur, OpenWorld] },
    StatDef { id: "ascended_shards_of_glory", display_name: "Ascended Shards of Glory", source: StatSource::Currency(33), categories: &[Cur, Pvp] },
    StatDef { id: "trade_contract", display_name: "Trade Contract", source: StatSource::Currency(34), categories: &[Cur] },
    StatDef { id: "elegy_mosaic", display_name: "Elegy Mosaic", source: StatSource::Currency(35), categories: &[Cur] },
    StatDef { id: "testimony_of_desert_heroics", display_name: "Testimony of Desert Heroics", source: StatSource::Currency(36), categories: &[Cur] },
    StatDef { id: "exalted_key", display_name: "Exalted Key", source: StatSource::Currency(37), categories: &[Cur] },
    StatDef { id: "machete", display_name: "Machete", source: StatSource::Currency(38), categories: &[Cur] },
    StatDef { id: "bandit_skeleton_key", display_name: "Bandit Skeleton Key", source: StatSource::Currency(40), categories: &[Cur] },
    StatDef { id: "pact_crowbar", display_name: "Pact Crowbar", source: StatSource::Currency(41), categories: &[Cur] },
    StatDef { id: "vial_of_chak_acid", display_name: "Vial of Chak Acid", source: StatSource::Currency(42), categories: &[Cur] },
    StatDef { id: "zephyrite_lockpick", display_name: "Zephyrite Lockpick", source: StatSource::Currency(43), categories: &[Cur] },
    StatDef { id: "traders_key", display_name: "Trader's Key", source: StatSource::Currency(44), categories: &[Cur] },
    StatDef { id: "volatile_magic", display_name: "Volatile Magic", source: StatSource::Currency(45), categories: &[Cur, OpenWorld] },
    StatDef { id: "pvp_tournament_voucher", display_name: "PvP Tournament Voucher", source: StatSource::Currency(46), categories: &[Cur, Pvp] },
    StatDef { id: "racing_medallion", display_name: "Racing Medallion", source: StatSource::Currency(47), categories: &[Cur] },
    StatDef { id: "mistborn_key", display_name: "Mistborn Key", source: StatSource::Currency(49), categories: &[Cur] },
    StatDef { id: "festival_token", display_name: "Festival Token", source: StatSource::Currency(50), categories: &[Cur] },
    StatDef { id: "cache_key", display_name: "Cache Key", source: StatSource::Currency(51), categories: &[Cur] },
    StatDef { id: "green_prophet_shard", display_name: "Green Prophet Shard", source: StatSource::Currency(53), categories: &[Cur, Strike] },
    StatDef { id: "blue_prophet_crystal", display_name: "Blue Prophet Crystal", source: StatSource::Currency(54), categories: &[Cur, Strike] },
    StatDef { id: "green_prophet_crystal", display_name: "Green Prophet Crystal", source: StatSource::Currency(55), categories: &[Cur] },
    StatDef { id: "blue_prophet_shard", display_name: "Blue Prophet Shard", source: StatSource::Currency(57), categories: &[Cur, Strike] },
    StatDef { id: "war_supplies", display_name: "War Supplies", source: StatSource::Currency(58), categories: &[Cur] },
    StatDef { id: "unstable_fractal_essence", display_name: "Unstable Fractal Essence", source: StatSource::Currency(59), categories: &[Cur, Fractal] },
    StatDef { id: "tyrian_defense_seal", display_name: "Tyrian Defense Seal", source: StatSource::Currency(60), categories: &[Cur] },
    StatDef { id: "research_note", display_name: "Research Note", source: StatSource::Currency(61), categories: &[Cur] },
    StatDef { id: "unusual_coin", display_name: "Unusual Coin", source: StatSource::Currency(62), categories: &[Cur] },
    StatDef { id: "astral_acclaim", display_name: "Astral Acclaim", source: StatSource::Currency(63), categories: &[Cur] },
    StatDef { id: "jade_sliver", display_name: "Jade Sliver", source: StatSource::Currency(64), categories: &[Cur] },
    StatDef { id: "testimony_of_jade_heroics", display_name: "Testimony of Jade Heroics", source: StatSource::Currency(65), categories: &[Cur] },
    StatDef { id: "ancient_coin", display_name: "Ancient Coin", source: StatSource::Currency(66), categories: &[Cur] },
    StatDef { id: "canach_coins", display_name: "Canach Coins", source: StatSource::Currency(67), categories: &[Cur] },
    StatDef { id: "imperial_favor", display_name: "Imperial Favor", source: StatSource::Currency(68), categories: &[Cur] },
    StatDef { id: "tales_of_dungeon_delving", display_name: "Tales of Dungeon Delving", source: StatSource::Currency(69), categories: &[Cur] },
    StatDef { id: "legendary_insight", display_name: "Legendary Insight", source: StatSource::Currency(70), categories: &[Cur, Raid] },
    StatDef { id: "jade_miners_keycard", display_name: "Jade Miner's Keycard", source: StatSource::Currency(71), categories: &[Cur] },
    StatDef { id: "static_charge", display_name: "Static Charge", source: StatSource::Currency(72), categories: &[Cur] },
    StatDef { id: "pinch_of_stardust", display_name: "Pinch of Stardust", source: StatSource::Currency(73), categories: &[Cur] },
    StatDef { id: "calcified_gasp", display_name: "Calcified Gasp", source: StatSource::Currency(75), categories: &[Cur] },
    StatDef { id: "ursus_oblige", display_name: "Ursus Oblige", source: StatSource::Currency(76), categories: &[Cur] },
    StatDef { id: "gaeting_crystal", display_name: "Gaeting Crystal", source: StatSource::Currency(77), categories: &[Cur] },
    StatDef { id: "fine_rift_essence", display_name: "Fine Rift Essence", source: StatSource::Currency(78), categories: &[Cur] },
    StatDef { id: "rare_rift_essence", display_name: "Rare Rift Essence", source: StatSource::Currency(79), categories: &[Cur] },
    StatDef { id: "masterwork_rift_essence", display_name: "Masterwork Rift Essence", source: StatSource::Currency(80), categories: &[Cur] },
    StatDef { id: "antiquated_ducat", display_name: "Antiquated Ducat", source: StatSource::Currency(81), categories: &[Cur] },
    StatDef { id: "testimony_of_castoran_heroics", display_name: "Testimony of Castoran Heroics", source: StatSource::Currency(82), categories: &[Cur] },
    StatDef { id: "aether_rich_sap", display_name: "Aether-Rich Sap", source: StatSource::Currency(83), categories: &[Cur] },
];

pub fn compute_lifetime_values(snapshot: &ApiSnapshot) -> HashMap<&'static str, f64> {
    let mut values = HashMap::new();
    for stat in STAT_CATALOG {
        let value = match stat.source {
            StatSource::Achievement(id) => {
                snapshot.achievements.get(&id).copied().unwrap_or(0) as f64
            }
            StatSource::WvwRank => snapshot.wvw_rank as f64,
            StatSource::Deaths => snapshot.total_deaths as f64,
            StatSource::Currency(id) => snapshot.currencies.get(&id).copied().unwrap_or(0) as f64,
            StatSource::PvpRank => snapshot.pvp_rank as f64,
            StatSource::PvpWins => snapshot.pvp_wins as f64,
            StatSource::PvpLosses => snapshot.pvp_losses as f64,
            StatSource::PvpRankingPoints => snapshot.pvp_ranking_points as f64,
            StatSource::PvpRankedWins => snapshot.pvp_ranked_wins as f64,
            StatSource::PvpRankedLosses => snapshot.pvp_ranked_losses as f64,
            StatSource::PvpUnrankedWins => snapshot.pvp_unranked_wins as f64,
            StatSource::PvpUnrankedLosses => snapshot.pvp_unranked_losses as f64,
            // computed below once their inputs are known
            StatSource::Kdr
            | StatSource::PvpKdr
            | StatSource::PvpCustomWins
            | StatSource::PvpCustomLosses => continue,
        };
        values.insert(stat.id, value);
    }

    let kills = values.get("kills").copied().unwrap_or(0.0);
    let deaths = values.get("deaths").copied().unwrap_or(0.0);
    let kdr = if deaths > 0.0 { kills / deaths } else { kills };
    values.insert("kdr", kdr);

    let pvp_kills = values.get("pvp_kills").copied().unwrap_or(0.0);
    let pvp_kdr = if deaths > 0.0 { pvp_kills / deaths } else { pvp_kills };
    values.insert("pvp_kdr", pvp_kdr);

    let pvp_custom_wins = snapshot.pvp_wins as f64
        - snapshot.pvp_ranked_wins as f64
        - snapshot.pvp_unranked_wins as f64;
    let pvp_custom_losses = snapshot.pvp_losses as f64
        - snapshot.pvp_ranked_losses as f64
        - snapshot.pvp_unranked_losses as f64;
    values.insert("pvp_custom_wins", pvp_custom_wins);
    values.insert("pvp_custom_losses", pvp_custom_losses);

    values
}

/// Resolves persisted selected-stat ids into catalog entries, in the
/// user's chosen order. Ids that no longer exist in `STAT_CATALOG` (e.g. a
/// stat later removed from the catalog) are silently dropped.
pub fn resolve_selected_stats(selected_ids: &[String]) -> Vec<&'static StatDef> {
    selected_ids
        .iter()
        .filter_map(|id| STAT_CATALOG.iter().find(|s| s.id == id.as_str()))
        .collect()
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
        ApiSnapshot {
            wvw_rank,
            achievements: map,
            total_deaths,
            currencies: StdHashMap::new(),
            pvp_rank: 0,
            pvp_wins: 0,
            pvp_losses: 0,
            pvp_ranking_points: 0,
            pvp_ranked_wins: 0,
            pvp_ranked_losses: 0,
            pvp_unranked_wins: 0,
            pvp_unranked_losses: 0,
        }
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
    fn catalog_has_ninety_six_stats() {
        // 17 WvW + 12 PvP + 67 currencies
        assert_eq!(STAT_CATALOG.len(), 96);
    }

    #[test]
    fn catalog_has_sixty_seven_currencies() {
        let count = STAT_CATALOG
            .iter()
            .filter(|s| matches!(s.source, StatSource::Currency(_)))
            .count();
        assert_eq!(count, 67);
    }

    #[test]
    fn deaths_is_tagged_wvw_pvp_and_misc() {
        let deaths = STAT_CATALOG.iter().find(|s| s.id == "deaths").unwrap();
        assert!(deaths.categories.contains(&Category::Wvw));
        assert!(deaths.categories.contains(&Category::Pvp));
        assert!(deaths.categories.contains(&Category::Misc));
    }

    #[test]
    fn gold_is_cross_tagged_into_every_activity_category() {
        let gold = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        for cat in [
            Category::Currency,
            Category::Wvw,
            Category::Pvp,
            Category::Fractal,
            Category::Raid,
            Category::OpenWorld,
            Category::Strike,
        ] {
            assert!(gold.categories.contains(&cat), "gold missing {cat:?}");
        }
    }

    #[test]
    fn laurels_is_currency_only() {
        let laurels = STAT_CATALOG.iter().find(|s| s.id == "laurels").unwrap();
        assert_eq!(laurels.categories, &[Category::Currency]);
    }

    #[test]
    fn maps_currency_ids_to_stat_values() {
        let mut snap = snapshot(0, &[], 0);
        snap.currencies.insert(1, 100001);
        snap.currencies.insert(4, 50);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["gold"], 100001.0);
        assert_eq!(values["gems"], 50.0);
    }

    #[test]
    fn missing_currency_defaults_to_zero() {
        let snap = snapshot(0, &[], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["karma"], 0.0);
    }

    #[test]
    fn maps_pvp_rank_wins_and_losses() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_rank = 45;
        snap.pvp_wins = 120;
        snap.pvp_losses = 80;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_rank"], 45.0);
        assert_eq!(values["pvp_wins"], 120.0);
        assert_eq!(values["pvp_losses"], 80.0);
    }

    #[test]
    fn maps_pvp_ranking_points_and_ranked_unranked_splits() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_ranking_points = 300;
        snap.pvp_ranked_wins = 10;
        snap.pvp_ranked_losses = 4;
        snap.pvp_unranked_wins = 30;
        snap.pvp_unranked_losses = 20;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_ranking_points"], 300.0);
        assert_eq!(values["pvp_ranked_wins"], 10.0);
        assert_eq!(values["pvp_ranked_losses"], 4.0);
        assert_eq!(values["pvp_unranked_wins"], 30.0);
        assert_eq!(values["pvp_unranked_losses"], 20.0);
    }

    #[test]
    fn computes_pvp_custom_wins_and_losses_as_remainder() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_wins = 120;
        snap.pvp_losses = 80;
        snap.pvp_ranked_wins = 10;
        snap.pvp_ranked_losses = 4;
        snap.pvp_unranked_wins = 30;
        snap.pvp_unranked_losses = 20;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_custom_wins"], 80.0); // 120 - 10 - 30
        assert_eq!(values["pvp_custom_losses"], 56.0); // 80 - 4 - 20
    }

    #[test]
    fn computes_pvp_kdr_from_pvp_kills_achievement_and_shared_deaths() {
        let snap = snapshot(0, &[(239, 50)], 25);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_kills"], 50.0);
        assert_eq!(values["pvp_kdr"], 2.0);
    }

    #[test]
    fn pvp_kdr_falls_back_to_kills_when_no_deaths() {
        let snap = snapshot(0, &[(239, 7)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_kdr"], 7.0);
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
}
