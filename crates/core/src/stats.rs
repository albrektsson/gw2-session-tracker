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
}
