static SKULL_SHIELD: &[u8] = include_bytes!("../../assets/icons/skull-shield.png");
static SKULL_X: &[u8] = include_bytes!("../../assets/icons/skull-x.png");
static KD_RATIO: &[u8] = include_bytes!("../../assets/icons/kd-ratio.png");
static WVW_RANK_BADGE: &[u8] = include_bytes!("../../assets/icons/wvw-rank-badge.png");
static DOLYAK: &[u8] = include_bytes!("../../assets/icons/dolyak.png");
static DOLYAK_SHIELD: &[u8] = include_bytes!("../../assets/icons/dolyak-shield.png");
static OBJECTIVE_BANNER: &[u8] = include_bytes!("../../assets/icons/objective-banner.png");
static OBJECTIVE_BANNER_SHIELD: &[u8] = include_bytes!("../../assets/icons/objective-banner-shield.png");
static RANKING_MEDAL: &[u8] = include_bytes!("../../assets/icons/ranking-medal.png");
static VICTORY_BADGE: &[u8] = include_bytes!("../../assets/icons/victory-badge.png");
static DEFEAT_BADGE: &[u8] = include_bytes!("../../assets/icons/defeat-badge.png");

/// Maps a stat id with no natural GW2 icon to a vendored icon adapted from
/// BlishHud-SessionTracker (see README.md for licensing/attribution). `None`
/// for any stat that has, or falls back to, a real API icon instead.
pub fn embedded_icon_bytes(stat_id: &str) -> Option<&'static [u8]> {
    match stat_id {
        "kills" | "pvp_kills" => Some(SKULL_SHIELD),
        "deaths" => Some(SKULL_X),
        "kdr" | "pvp_kdr" => Some(KD_RATIO),
        "wvw_rank" | "pvp_rank" => Some(WVW_RANK_BADGE),
        "dolyaks_killed" => Some(DOLYAK),
        "dolyaks_escorted" => Some(DOLYAK_SHIELD),
        "objectives_captured" => Some(OBJECTIVE_BANNER),
        "objectives_defended" => Some(OBJECTIVE_BANNER_SHIELD),
        "pvp_ranking_points" => Some(RANKING_MEDAL),
        "pvp_wins" | "pvp_ranked_wins" | "pvp_unranked_wins" | "pvp_custom_wins" => Some(VICTORY_BADGE),
        "pvp_losses" | "pvp_ranked_losses" | "pvp_unranked_losses" | "pvp_custom_losses" => Some(DEFEAT_BADGE),
        _ => None,
    }
}
