use std::time::Duration;
use session_tracker_core::api::{self, ApiError, ApiSnapshot};
use session_tracker_core::stats::{StatSource, STAT_CATALOG};

/// Builds a fresh ureq agent with bounded connect/read timeouts, so a
/// hanging connection can't block the poller thread (and therefore
/// `Poller::stop()`'s `join()` / addon unload) indefinitely. This is called
/// at most once per poll cycle (every 60s), so building fresh each time is
/// cheap enough to not warrant lazy/static caching.
fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(10)))
        .build();
    ureq::Agent::new_with_config(config)
}

/// Derives the achievement-ID query list from the `STAT_CATALOG` catalog
/// instead of hardcoding a second list here, so a stat added to the
/// catalog is automatically included in the API query.
fn achievement_ids_query() -> String {
    STAT_CATALOG
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
        .header("Authorization", &format!("Bearer {api_key}"))
        .call()
        .map_err(|e| ApiError(format!("request to {url} failed: {e}")))?
        .body_mut()
        .read_to_string()
        .map_err(|e| ApiError(format!("failed to read response from {url}: {e}")))
}

/// Like `authorized_get`, but treats a 403 (GW2 API's single status code
/// for *any* auth failure - missing key or missing scope, there's no 401
/// distinction) as "not available" rather than a hard error. Only used for
/// optional stat categories (wallet, pvp) so a key missing those scopes
/// degrades just those stats instead of failing the whole poll. A
/// genuinely invalid key still fails loudly via the mandatory
/// account/achievements/characters calls, which run first.
fn authorized_get_optional(url: &str, api_key: &str) -> Result<Option<String>, ApiError> {
    match agent()
        .get(url)
        .header("Authorization", &format!("Bearer {api_key}"))
        .call()
    {
        Ok(mut response) => response
            .body_mut()
            .read_to_string()
            .map(Some)
            .map_err(|e| ApiError(format!("failed to read response from {url}: {e}"))),
        Err(ureq::Error::StatusCode(403)) => Ok(None),
        Err(e) => Err(ApiError(format!("request to {url} failed: {e}"))),
    }
}

/// Fetches account, achievements, character, wallet, PvP, and item
/// (bank/shared inventory/materials) data from the official GW2 API and
/// combines them into a single [`ApiSnapshot`]. Wallet/PvP/item data is
/// optional - a key without the `wallet`/`pvp`/`inventories` scopes still
/// succeeds, just with those stats reading as zero.
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
    let wallet_json =
        authorized_get_optional("https://api.guildwars2.com/v2/account/wallet", api_key)?;
    let pvp_stats_json =
        authorized_get_optional("https://api.guildwars2.com/v2/pvp/stats", api_key)?;
    let bank_json = authorized_get_optional("https://api.guildwars2.com/v2/account/bank", api_key)?;
    let shared_inventory_json =
        authorized_get_optional("https://api.guildwars2.com/v2/account/inventory", api_key)?;
    let materials_json =
        authorized_get_optional("https://api.guildwars2.com/v2/account/materials", api_key)?;

    let account = api::parse_account(&account_json)?;
    let achievements = api::parse_achievements(&achievements_json)?;
    let characters = api::parse_characters(&characters_json)?;
    let wallet = wallet_json
        .map(|json| api::parse_wallet(&json))
        .transpose()?
        .unwrap_or_default();
    let pvp_stats = pvp_stats_json
        .map(|json| api::parse_pvp_stats(&json))
        .transpose()?;
    let bank = bank_json
        .map(|json| api::parse_bank(&json))
        .transpose()?
        .unwrap_or_default();
    let shared_inventory = shared_inventory_json
        .map(|json| api::parse_shared_inventory(&json))
        .transpose()?
        .unwrap_or_default();
    let materials = materials_json
        .map(|json| api::parse_materials(&json))
        .transpose()?
        .unwrap_or_default();
    Ok(api::build_snapshot(api::FetchedData {
        account,
        achievements,
        characters,
        wallet,
        pvp_stats,
        bank,
        shared_inventory,
        materials,
    }))
}
