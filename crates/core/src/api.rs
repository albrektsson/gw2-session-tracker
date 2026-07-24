use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct AccountResponse {
    pub wvw_rank: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AchievementProgress {
    pub id: u32,
    #[serde(default)]
    pub current: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CharacterCore {
    #[serde(default)]
    pub deaths: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct WalletEntry {
    pub id: u32,
    pub value: u64,
}

#[derive(Debug, Deserialize)]
pub struct PvpAggregate {
    pub wins: u32,
    pub losses: u32,
}

#[derive(Debug, Deserialize)]
pub struct PvpStatsResponse {
    pub pvp_rank: u32,
    pub pvp_rank_points: u32,
    pub pvp_rank_rollovers: u32,
    pub aggregate: PvpAggregate,
    #[serde(default)]
    pub ladders: HashMap<String, PvpAggregate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiSnapshot {
    pub wvw_rank: u64,
    pub achievements: HashMap<u32, u64>,
    pub total_deaths: u64,
    pub currencies: HashMap<u32, u64>,
    pub pvp_rank: u64,
    pub pvp_wins: u64,
    pub pvp_losses: u64,
    pub pvp_ranking_points: u64,
    pub pvp_ranked_wins: u64,
    pub pvp_ranked_losses: u64,
    pub pvp_unranked_wins: u64,
    pub pvp_unranked_losses: u64,
}

#[derive(Debug, PartialEq)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ApiError {}

pub fn parse_account(json: &str) -> Result<AccountResponse, ApiError> {
    serde_json::from_str(json).map_err(|e| ApiError(format!("invalid account response: {e}")))
}

pub fn parse_achievements(json: &str) -> Result<Vec<AchievementProgress>, ApiError> {
    serde_json::from_str(json)
        .map_err(|e| ApiError(format!("invalid achievements response: {e}")))
}

pub fn parse_characters(json: &str) -> Result<Vec<CharacterCore>, ApiError> {
    serde_json::from_str(json)
        .map_err(|e| ApiError(format!("invalid characters response: {e}")))
}

pub fn parse_wallet(json: &str) -> Result<Vec<WalletEntry>, ApiError> {
    serde_json::from_str(json).map_err(|e| ApiError(format!("invalid wallet response: {e}")))
}

pub fn parse_pvp_stats(json: &str) -> Result<PvpStatsResponse, ApiError> {
    serde_json::from_str(json).map_err(|e| ApiError(format!("invalid pvp stats response: {e}")))
}

pub fn build_snapshot(
    account: AccountResponse,
    achievements: Vec<AchievementProgress>,
    characters: Vec<CharacterCore>,
    wallet: Vec<WalletEntry>,
    pvp_stats: Option<PvpStatsResponse>,
) -> ApiSnapshot {
    let achievements = achievements
        .into_iter()
        .map(|a| (a.id, a.current.unwrap_or(0)))
        .collect();
    let total_deaths = characters.iter().map(|c| c.deaths.unwrap_or(0)).sum();
    let currencies = wallet.into_iter().map(|w| (w.id, w.value)).collect();
    let (
        pvp_rank,
        pvp_wins,
        pvp_losses,
        pvp_ranking_points,
        pvp_ranked_wins,
        pvp_ranked_losses,
        pvp_unranked_wins,
        pvp_unranked_losses,
    ) = match pvp_stats {
        Some(stats) => {
            let ranked = stats.ladders.get("ranked");
            let unranked = stats.ladders.get("unranked");
            (
                (stats.pvp_rank + stats.pvp_rank_rollovers) as u64,
                stats.aggregate.wins as u64,
                stats.aggregate.losses as u64,
                stats.pvp_rank_points as u64,
                ranked.map(|l| l.wins).unwrap_or(0) as u64,
                ranked.map(|l| l.losses).unwrap_or(0) as u64,
                unranked.map(|l| l.wins).unwrap_or(0) as u64,
                unranked.map(|l| l.losses).unwrap_or(0) as u64,
            )
        }
        None => (0, 0, 0, 0, 0, 0, 0, 0),
    };
    ApiSnapshot {
        wvw_rank: account.wvw_rank.unwrap_or(0) as u64,
        achievements,
        total_deaths,
        currencies,
        pvp_rank,
        pvp_wins,
        pvp_losses,
        pvp_ranking_points,
        pvp_ranked_wins,
        pvp_ranked_losses,
        pvp_unranked_wins,
        pvp_unranked_losses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_account_wvw_rank() {
        let json = r#"{"wvw_rank": 1234, "id": "ABCD-1234"}"#;
        let account = parse_account(json).unwrap();
        assert_eq!(account.wvw_rank, Some(1234));
    }

    #[test]
    fn parses_achievements_list() {
        let json = r#"[{"id": 283, "current": 500, "max": 1}, {"id": 999, "bits": []}]"#;
        let achievements = parse_achievements(json).unwrap();
        assert_eq!(achievements.len(), 2);
        assert_eq!(achievements[0].id, 283);
        assert_eq!(achievements[0].current, Some(500));
        assert_eq!(achievements[1].current, None);
    }

    #[test]
    fn parses_characters_deaths() {
        let json = r#"[{"name": "A", "deaths": 10}, {"name": "B", "deaths": 5}]"#;
        let characters = parse_characters(json).unwrap();
        assert_eq!(characters[0].deaths, Some(10));
        assert_eq!(characters[1].deaths, Some(5));
    }

    #[test]
    fn build_snapshot_sums_deaths_and_maps_achievements() {
        let account = AccountResponse { wvw_rank: Some(42) };
        let achievements = vec![
            AchievementProgress { id: 283, current: Some(500) },
            AchievementProgress { id: 288, current: None },
        ];
        let characters = vec![
            CharacterCore { deaths: Some(10) },
            CharacterCore { deaths: Some(5) },
        ];
        let snapshot = build_snapshot(account, achievements, characters, vec![], None);
        assert_eq!(snapshot.wvw_rank, 42);
        assert_eq!(snapshot.achievements.get(&283), Some(&500));
        assert_eq!(snapshot.achievements.get(&288), Some(&0));
        assert_eq!(snapshot.total_deaths, 15);
    }

    #[test]
    fn build_snapshot_defaults_currencies_and_pvp_when_omitted() {
        let account = AccountResponse { wvw_rank: None };
        let snapshot = build_snapshot(account, vec![], vec![], vec![], None);
        assert!(snapshot.currencies.is_empty());
        assert_eq!(snapshot.pvp_rank, 0);
        assert_eq!(snapshot.pvp_wins, 0);
        assert_eq!(snapshot.pvp_losses, 0);
        assert_eq!(snapshot.pvp_ranking_points, 0);
        assert_eq!(snapshot.pvp_ranked_wins, 0);
        assert_eq!(snapshot.pvp_ranked_losses, 0);
        assert_eq!(snapshot.pvp_unranked_wins, 0);
        assert_eq!(snapshot.pvp_unranked_losses, 0);
    }

    #[test]
    fn build_snapshot_maps_wallet_and_pvp_stats() {
        let account = AccountResponse { wvw_rank: None };
        let wallet = vec![
            WalletEntry { id: 1, value: 100001 },
            WalletEntry { id: 4, value: 50 },
        ];
        let mut ladders = HashMap::new();
        ladders.insert("ranked".to_string(), PvpAggregate { wins: 10, losses: 4 });
        ladders.insert("unranked".to_string(), PvpAggregate { wins: 30, losses: 20 });
        let pvp_stats = PvpStatsResponse {
            pvp_rank: 45,
            pvp_rank_points: 300,
            pvp_rank_rollovers: 2,
            aggregate: PvpAggregate { wins: 120, losses: 80 },
            ladders,
        };
        let snapshot = build_snapshot(account, vec![], vec![], wallet, Some(pvp_stats));
        assert_eq!(snapshot.currencies.get(&1), Some(&100001));
        assert_eq!(snapshot.currencies.get(&4), Some(&50));
        assert_eq!(snapshot.pvp_rank, 47); // pvp_rank + rollovers
        assert_eq!(snapshot.pvp_wins, 120);
        assert_eq!(snapshot.pvp_losses, 80);
        assert_eq!(snapshot.pvp_ranking_points, 300);
        assert_eq!(snapshot.pvp_ranked_wins, 10);
        assert_eq!(snapshot.pvp_ranked_losses, 4);
        assert_eq!(snapshot.pvp_unranked_wins, 30);
        assert_eq!(snapshot.pvp_unranked_losses, 20);
    }

    #[test]
    fn build_snapshot_defaults_ranked_and_unranked_when_ladder_key_missing() {
        let account = AccountResponse { wvw_rank: None };
        let pvp_stats = PvpStatsResponse {
            pvp_rank: 10,
            pvp_rank_points: 0,
            pvp_rank_rollovers: 0,
            aggregate: PvpAggregate { wins: 0, losses: 0 },
            ladders: HashMap::new(),
        };
        let snapshot = build_snapshot(account, vec![], vec![], vec![], Some(pvp_stats));
        assert_eq!(snapshot.pvp_ranked_wins, 0);
        assert_eq!(snapshot.pvp_ranked_losses, 0);
        assert_eq!(snapshot.pvp_unranked_wins, 0);
        assert_eq!(snapshot.pvp_unranked_losses, 0);
    }

    #[test]
    fn parses_wallet_entries() {
        let json = r#"[{"id": 1, "value": 100001}, {"id": 4, "value": 50}]"#;
        let wallet = parse_wallet(json).unwrap();
        assert_eq!(wallet.len(), 2);
        assert_eq!(wallet[0].id, 1);
        assert_eq!(wallet[0].value, 100001);
    }

    #[test]
    fn parses_pvp_stats() {
        let json = r#"{
            "pvp_rank": 45,
            "pvp_rank_points": 100,
            "pvp_rank_rollovers": 1,
            "aggregate": {"wins": 120, "losses": 80, "desertions": 1, "byes": 0, "forfeits": 0},
            "professions": {},
            "ladders": {"ranked": {"wins": 10, "losses": 4}, "unranked": {"wins": 30, "losses": 20}}
        }"#;
        let stats = parse_pvp_stats(json).unwrap();
        assert_eq!(stats.pvp_rank, 45);
        assert_eq!(stats.pvp_rank_points, 100);
        assert_eq!(stats.pvp_rank_rollovers, 1);
        assert_eq!(stats.aggregate.wins, 120);
        assert_eq!(stats.aggregate.losses, 80);
        assert_eq!(stats.ladders["ranked"].wins, 10);
        assert_eq!(stats.ladders["unranked"].losses, 20);
    }

    #[test]
    fn parses_pvp_stats_with_missing_ladders_key() {
        let json = r#"{
            "pvp_rank": 10,
            "pvp_rank_points": 0,
            "pvp_rank_rollovers": 0,
            "aggregate": {"wins": 0, "losses": 0}
        }"#;
        let stats = parse_pvp_stats(json).unwrap();
        assert!(stats.ladders.is_empty());
    }

    #[test]
    fn parse_account_rejects_invalid_json() {
        let err = parse_account("not json").unwrap_err();
        assert!(err.0.contains("invalid account response"));
    }
}
