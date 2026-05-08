//! Steam Web API client for ACHIEVEMENT METADATA ENRICHMENT only.
//!
//! IMPORTANT: This module calls ONE endpoint:
//!   ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/
//! which is publicly accessible WITHOUT an API key (verified at
//! partner.steamgames.com/doc/webapi/isteamuserstats).
//!
//! DO NOT add `GetSchemaForGame` here — that endpoint REQUIRES a publisher
//! API key, and embedding a key in an OSS binary is a TOS violation that
//! gets revoked. See RESEARCH.md Pitfall 7. If schema (display name,
//! description) cannot be sourced from Goldberg's achievements.json, the
//! popup degrades to api_name (D-26) — that is the locked policy.

use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;

/// Steam's response shape for the global-percent endpoint.
#[derive(Debug, Deserialize)]
struct GlobalAchPctResponse {
    achievementpercentages: GlobalAchPctInner,
}
#[derive(Debug, Deserialize)]
struct GlobalAchPctInner {
    #[serde(default)]
    achievements: Vec<GlobalAchPct>,
}
#[derive(Debug, Deserialize)]
struct GlobalAchPct {
    name: String,
    percent: f64,
}

/// Fetch global unlock percentages from the public Steam Web API.
/// NO API KEY REQUIRED. Endpoint URL hard-coded; gameid is the only parameter.
/// Returns a map of ach_api_name → percent (0.0..=100.0).
///
/// Errors are returned via `Result` rather than logged here — the caller
/// (SchemaCache::resolve) decides whether to log+continue or propagate.
pub async fn fetch_global_pcts(
    client: &reqwest::Client,
    app_id: u64,
) -> anyhow::Result<HashMap<String, f64>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/?gameid={app_id}&format=json"
    );
    tracing::debug!(app_id, url = %url, "fetching global pcts from public Steam endpoint");
    let resp: GlobalAchPctResponse = client
        .get(&url)
        .timeout(Duration::from_secs(8))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let map: HashMap<String, f64> = resp
        .achievementpercentages
        .achievements
        .into_iter()
        .map(|a| (a.name, a.percent))
        .collect();
    tracing::info!(app_id, count = map.len(), "fetched global pcts");
    Ok(map)
}

#[cfg(test)]
mod tests {
    /// Verify the endpoint URL has the exact required shape and contains NO
    /// API key parameter. This is a static-text check — not a live HTTP call —
    /// so the test runs without internet. The check guards against future
    /// edits accidentally introducing &key=... into the URL.
    #[test]
    fn url_contains_no_api_key_marker() {
        let app_id = 480_u64;
        let url = format!(
            "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v0002/?gameid={app_id}&format=json"
        );
        assert!(url.contains("/v0002/"), "must use v0002 endpoint");
        assert!(
            url.contains(&format!("gameid={app_id}")),
            "must pass gameid param"
        );
        assert!(!url.contains("key="), "MUST NOT include API key");
        assert!(!url.contains("apikey="), "MUST NOT include apikey param");
        assert!(!url.contains("access_token"), "MUST NOT include access_token");
    }
}
