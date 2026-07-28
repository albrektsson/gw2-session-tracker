static KILL: &[u8] = include_bytes!("../../assets/icons/kill.png");
static DEATH: &[u8] = include_bytes!("../../assets/icons/death.png");
static KDR: &[u8] = include_bytes!("../../assets/icons/kdr.png");
static WVW_RANK: &[u8] = include_bytes!("../../assets/icons/wvwRank.png");
static PVP_RANK: &[u8] = include_bytes!("../../assets/icons/pvpRank.png");
static DOLYAK: &[u8] = include_bytes!("../../assets/icons/dolyak.png");
static DOLYAK_DEFENDED: &[u8] = include_bytes!("../../assets/icons/dolyakDefended.png");
static CAMP: &[u8] = include_bytes!("../../assets/icons/camp.png");
static CAMP_DEFENDED: &[u8] = include_bytes!("../../assets/icons/campDefended.png");
static TOWER: &[u8] = include_bytes!("../../assets/icons/tower.png");
static TOWER_DEFENDED: &[u8] = include_bytes!("../../assets/icons/towerDefended.png");
static KEEP: &[u8] = include_bytes!("../../assets/icons/keep.png");
static KEEP_DEFENDED: &[u8] = include_bytes!("../../assets/icons/keepDefended.png");
static CASTLE: &[u8] = include_bytes!("../../assets/icons/castle.png");
static CASTLE_DEFENDED: &[u8] = include_bytes!("../../assets/icons/castleDefended.png");
static OBJECTIVE: &[u8] = include_bytes!("../../assets/icons/objective.png");
static OBJECTIVE_DEFENDED: &[u8] = include_bytes!("../../assets/icons/objectiveDefended.png");
static SUPPLY_SPEND: &[u8] = include_bytes!("../../assets/icons/supplySpend.png");
static PVP_RANKING_POINTS: &[u8] = include_bytes!("../../assets/icons/pvpRankingPoints.png");
static PVP_WINS: &[u8] = include_bytes!("../../assets/icons/pvpWins.png");
static PVP_LOSSES: &[u8] = include_bytes!("../../assets/icons/pvpLosses.png");
static STOPWATCH: &[u8] = include_bytes!("../../assets/icons/stopwatch.png");
static RUN: &[u8] = include_bytes!("../../assets/icons/run.png");

/// Maps a stat id with no natural GW2 icon to a vendored icon adapted from
/// BlishHud-SessionTracker (see README.md for licensing/attribution). `None`
/// for any stat that has, or falls back to, a real API icon instead.
pub fn embedded_icon_bytes(stat_id: &str) -> Option<&'static [u8]> {
    match stat_id {
        "kills" | "pvp_kills" => Some(KILL),
        "deaths" => Some(DEATH),
        "kdr" | "pvp_kdr" => Some(KDR),
        "wvw_rank" => Some(WVW_RANK),
        "pvp_rank" => Some(PVP_RANK),
        "dolyaks_killed" => Some(DOLYAK),
        "dolyaks_escorted" => Some(DOLYAK_DEFENDED),
        "camps_captured" => Some(CAMP),
        "camps_defended" => Some(CAMP_DEFENDED),
        "towers_captured" => Some(TOWER),
        "towers_defended" => Some(TOWER_DEFENDED),
        "keeps_captured" => Some(KEEP),
        "keeps_defended" => Some(KEEP_DEFENDED),
        "castles_captured" => Some(CASTLE),
        "castles_defended" => Some(CASTLE_DEFENDED),
        "objectives_captured" => Some(OBJECTIVE),
        "objectives_defended" => Some(OBJECTIVE_DEFENDED),
        "supply_repair" => Some(SUPPLY_SPEND),
        "pvp_ranking_points" => Some(PVP_RANKING_POINTS),
        "pvp_wins" | "pvp_ranked_wins" | "pvp_unranked_wins" | "pvp_custom_wins" => Some(PVP_WINS),
        "pvp_losses" | "pvp_ranked_losses" | "pvp_unranked_losses" | "pvp_custom_losses" => Some(PVP_LOSSES),
        "session_timer" => Some(STOPWATCH),
        "distance_traveled" => Some(RUN),
        _ => None,
    }
}
