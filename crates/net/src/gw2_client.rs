use std::time::Duration;
use session_tracker_core::api::{self, ApiError, ApiSnapshot};
use session_tracker_core::stats::{StatSource, WVW_STATS};

/// Builds a fresh ureq agent with bounded connect/read timeouts, so a
/// hanging connection can't block the poller thread (and therefore
/// `Poller::stop()`'s `join()` / addon unload) indefinitely. This is called
/// at most once per poll cycle (every 60s), so building fresh each time is
/// cheap enough to not warrant lazy/static caching.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(10))
        .build()
}

/// Derives the achievement-ID query list from the `WVW_STATS` catalog
/// instead of hardcoding a second list here, so a stat added to the
/// catalog is automatically included in the API query.
fn achievement_ids_query() -> String {
    WVW_STATS
        .iter()
        .filter_map(|stat| match stat.source {
            StatSource::Achievement(id) => Some(id.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn authorized_get(url: &str, api_key: &str) -> Result<String, ApiError> {
    agent()
        .get(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .call()
        .map_err(|e| ApiError(format!("request to {url} failed: {e}")))?
        .into_string()
        .map_err(|e| ApiError(format!("failed to read response from {url}: {e}")))
}

/// Fetches account, achievements, and character data from the official GW2
/// API and combines them into a single [`ApiSnapshot`].
pub fn fetch_snapshot(api_key: &str) -> Result<ApiSnapshot, ApiError> {
    let account_json = authorized_get("https://api.guildwars2.com/v2/account", api_key)?;
    let achievements_json = authorized_get(
        &format!(
            "https://api.guildwars2.com/v2/account/achievements?ids={}",
            achievement_ids_query()
        ),
        api_key,
    )?;
    let characters_json =
        authorized_get("https://api.guildwars2.com/v2/characters?ids=all", api_key)?;

    let account = api::parse_account(&account_json)?;
    let achievements = api::parse_achievements(&achievements_json)?;
    let characters = api::parse_characters(&characters_json)?;
    Ok(api::build_snapshot(account, achievements, characters))
}
