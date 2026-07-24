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

#[derive(Debug, Clone, PartialEq)]
pub struct ApiSnapshot {
    pub wvw_rank: u64,
    pub achievements: HashMap<u32, u64>,
    pub total_deaths: u64,
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

pub fn build_snapshot(
    account: AccountResponse,
    achievements: Vec<AchievementProgress>,
    characters: Vec<CharacterCore>,
) -> ApiSnapshot {
    let achievements = achievements
        .into_iter()
        .map(|a| (a.id, a.current.unwrap_or(0)))
        .collect();
    let total_deaths = characters.iter().map(|c| c.deaths.unwrap_or(0)).sum();
    ApiSnapshot {
        wvw_rank: account.wvw_rank.unwrap_or(0) as u64,
        achievements,
        total_deaths,
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
        let snapshot = build_snapshot(account, achievements, characters);
        assert_eq!(snapshot.wvw_rank, 42);
        assert_eq!(snapshot.achievements.get(&283), Some(&500));
        assert_eq!(snapshot.achievements.get(&288), Some(&0));
        assert_eq!(snapshot.total_deaths, 15);
    }

    #[test]
    fn parse_account_rejects_invalid_json() {
        let err = parse_account("not json").unwrap_err();
        assert!(err.0.contains("invalid account response"));
    }
}
