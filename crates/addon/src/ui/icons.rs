static CROSSED_SWORDS: &[u8] = include_bytes!("../../assets/icons/crossed-swords.png");
static SKULL_CROSSED_BONES: &[u8] = include_bytes!("../../assets/icons/skull-crossed-bones.png");
static WEIGHT_SCALE: &[u8] = include_bytes!("../../assets/icons/weight-scale.png");
static RANK: &[u8] = include_bytes!("../../assets/icons/rank-3.png");
static WOODEN_CRATE: &[u8] = include_bytes!("../../assets/icons/wooden-crate.png");
static BISON: &[u8] = include_bytes!("../../assets/icons/bison.png");
static CAMPING_TENT: &[u8] = include_bytes!("../../assets/icons/camping-tent.png");
static WATCHTOWER: &[u8] = include_bytes!("../../assets/icons/watchtower.png");
static MILITARY_FORT: &[u8] = include_bytes!("../../assets/icons/military-fort.png");
static CASTLE: &[u8] = include_bytes!("../../assets/icons/castle.png");
static FLAG_OBJECTIVE: &[u8] = include_bytes!("../../assets/icons/flag-objective.png");
static PODIUM: &[u8] = include_bytes!("../../assets/icons/podium.png");
static TROPHY: &[u8] = include_bytes!("../../assets/icons/trophy.png");
static CROSS_MARK: &[u8] = include_bytes!("../../assets/icons/cross-mark.png");

/// Maps a stat id with no natural GW2 icon to a vendored game-icons.org
/// silhouette (see README.md for licensing/attribution). `None` for any
/// stat that has, or falls back to, a real API icon instead.
pub fn embedded_icon_bytes(stat_id: &str) -> Option<&'static [u8]> {
    match stat_id {
        "kills" | "pvp_kills" => Some(CROSSED_SWORDS),
        "deaths" => Some(SKULL_CROSSED_BONES),
        "kdr" | "pvp_kdr" => Some(WEIGHT_SCALE),
        "wvw_rank" | "pvp_rank" => Some(RANK),
        "supply_repair" => Some(WOODEN_CRATE),
        "dolyaks_killed" | "dolyaks_escorted" => Some(BISON),
        "camps_captured" | "camps_defended" => Some(CAMPING_TENT),
        "towers_captured" | "towers_defended" => Some(WATCHTOWER),
        "keeps_captured" | "keeps_defended" => Some(MILITARY_FORT),
        "castles_captured" | "castles_defended" => Some(CASTLE),
        "objectives_captured" | "objectives_defended" => Some(FLAG_OBJECTIVE),
        "pvp_ranking_points" => Some(PODIUM),
        "pvp_wins" | "pvp_ranked_wins" | "pvp_unranked_wins" | "pvp_custom_wins" => Some(TROPHY),
        "pvp_losses" | "pvp_ranked_losses" | "pvp_unranked_losses" | "pvp_custom_losses" => Some(CROSS_MARK),
        _ => None,
    }
}
