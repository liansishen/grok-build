//! CLIProxyAPI (CPA) weekly quota for prompt / `/usage` display.
//!
//! Fetches account pool metadata via Management API and per-account weekly
//! usage via `POST /v0/management/api-call` → ChatGPT `wham/usage`.

use std::time::Duration;

use chrono::{FixedOffset, TimeZone};
use serde::Deserialize;
use xai_grok_shell::agent::config::CpaManagementConfig;

/// Fixed Asia/Shanghai (UTC+8) for reset timestamps.
fn shanghai() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("UTC+8 offset")
}

/// One account's weekly window.
#[derive(Debug, Clone, PartialEq)]
pub struct CpaAccountQuota {
    pub email: String,
    /// Used percent of the weekly window (0–100+).
    pub used_percent: f64,
    /// Unix timestamp when the weekly window resets.
    pub reset_at: i64,
    pub plan_type: Option<String>,
}

/// Snapshot for UI + `/usage`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CpaQuotaSnapshot {
    pub accounts: Vec<CpaAccountQuota>,
    /// Model id this snapshot was fetched for.
    pub model_id: String,
}

impl CpaQuotaSnapshot {
    /// Prompt-line fragment: up to 3 accounts, least remaining first.
    /// Percentages are remaining quota, e.g. `25%(08/05 05:12)`.
    pub fn prompt_status_line(&self) -> Option<String> {
        let top = top_accounts(&self.accounts, 3);
        if top.is_empty() {
            return None;
        }
        Some(
            top.iter()
                .map(|a| format_account_short(a))
                .collect::<Vec<_>>()
                .join(" · "),
        )
    }

    /// Multi-line block for `/usage` scrollback.
    pub fn usage_block_text(&self) -> String {
        let top = top_accounts(&self.accounts, 3);
        if top.is_empty() {
            return xai_grok_i18n::t("usage.cpa.no_accounts").to_string();
        }
        let mut lines = vec![xai_grok_i18n::t("usage.cpa.header").to_string()];
        for a in &top {
            let reset = format_reset_shanghai(a.reset_at);
            let plan = a
                .plan_type
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            let account = format!("{}{}", a.email, plan);
            let remaining = format!("{:.0}", remaining_percent(a.used_percent));
            lines.push(xai_grok_i18n::t_fmt(
                "usage.cpa.account",
                &[
                    ("account", account.as_str()),
                    ("remaining", remaining.as_str()),
                    ("time", reset.as_str()),
                ],
            ));
        }
        if self.accounts.len() > top.len() {
            let count = (self.accounts.len() - top.len()).to_string();
            lines.push(xai_grok_i18n::t_fmt(
                "usage.cpa.more_omitted",
                &[("count", count.as_str())],
            ));
        }
        lines.join("\n")
    }
}

/// Sort by least remaining (highest used%), then earlier reset, then email.
fn top_accounts(accounts: &[CpaAccountQuota], n: usize) -> Vec<&CpaAccountQuota> {
    let mut refs: Vec<&CpaAccountQuota> = accounts.iter().collect();
    refs.sort_by(|a, b| {
        b.used_percent
            .partial_cmp(&a.used_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.reset_at.cmp(&b.reset_at))
            .then_with(|| a.email.cmp(&b.email))
    });
    refs.into_iter().take(n).collect()
}

fn remaining_percent(used_percent: f64) -> f64 {
    if !used_percent.is_finite() {
        return 0.0;
    }
    (100.0 - used_percent).clamp(0.0, 100.0)
}

fn format_account_short(a: &CpaAccountQuota) -> String {
    format!(
        "{:.0}%({})",
        remaining_percent(a.used_percent),
        format_reset_shanghai(a.reset_at)
    )
}

/// `mm/dd HH:MM` in Asia/Shanghai.
fn format_reset_shanghai(reset_at: i64) -> String {
    match shanghai().timestamp_opt(reset_at, 0) {
        chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => {
            dt.format("%m/%d %H:%M").to_string()
        }
        chrono::LocalResult::None => "??/?? ??:??".to_string(),
    }
}

/// Resolve CPA management settings for a model id/key from disk config.
pub fn management_for_model(model_id: &str) -> Option<CpaManagementConfig> {
    let raw = xai_grok_shell::util::config::load_effective_config_disk_only().ok()?;
    let section = raw.get("model")?.as_table()?;
    let needle = model_id.trim();
    if needle.is_empty() {
        return None;
    }
    for (key, value) in section {
        let Some(table) = value.as_table() else {
            continue;
        };
        let model_slug = table
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(key.as_str());
        if key != needle && model_slug != needle {
            continue;
        }
        // Deserialize only the management-related fields via ConfigModelOverride.
        let ov: xai_grok_shell::agent::config::ConfigModelOverride =
            value.clone().try_into().ok()?;
        return ov.resolved_cpa_management();
    }
    None
}

/// Per-request ceiling for management API calls.
const CPA_HTTP_TIMEOUT: Duration = Duration::from_secs(20);

/// Fetch weekly quotas for the configured providers (skips xai).
pub async fn fetch_weekly_quotas(
    settings: &CpaManagementConfig,
) -> Result<Vec<CpaAccountQuota>, String> {
    // Reuse the process-wide TLS client; request timeouts are set per call.
    let client = xai_grok_shell::http::shared_client();

    let auth_files = list_auth_files(&client, settings).await?;
    let mut out = Vec::new();
    for file in auth_files {
        let provider = file.provider.to_ascii_lowercase();
        if provider == "xai" {
            continue;
        }
        if !settings
            .providers
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&provider))
        {
            continue;
        }
        if file.disabled || file.unavailable {
            continue;
        }
        match fetch_wham_usage(&client, settings, &file.auth_index).await {
            Ok(Some(q)) => {
                // Resolve plan before moving `email`/`name` out of `file`.
                let plan_type = q.plan_type.or_else(|| file.resolved_plan());
                let email = file
                    .email
                    .filter(|e| !e.is_empty())
                    .or(file.name)
                    .unwrap_or_else(|| file.auth_index.clone());
                out.push(CpaAccountQuota {
                    email,
                    used_percent: q.used_percent,
                    reset_at: q.reset_at,
                    plan_type,
                });
            }
            Ok(None) => {}
            Err(e) => {
                tracing::debug!(
                    auth_index = %file.auth_index,
                    error = %e,
                    "cpa quota: skip account"
                );
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
struct AuthFilesResponse {
    files: Vec<AuthFile>,
}

#[derive(Debug, Deserialize)]
struct AuthFile {
    auth_index: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    unavailable: bool,
    #[serde(default)]
    id_token: Option<serde_json::Value>,
    #[serde(default)]
    plan_type: Option<String>,
}

impl AuthFile {
    fn resolved_plan(&self) -> Option<String> {
        self.plan_type.clone().or_else(|| {
            self.id_token
                .as_ref()
                .and_then(|v| v.get("plan_type"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }
}

async fn list_auth_files(
    client: &reqwest::Client,
    settings: &CpaManagementConfig,
) -> Result<Vec<AuthFile>, String> {
    let url = format!(
        "{}/v0/management/auth-files",
        settings.url.trim_end_matches('/')
    );
    let resp = client
        .get(&url)
        .timeout(CPA_HTTP_TIMEOUT)
        .header("Authorization", format!("Bearer {}", settings.key))
        .send()
        .await
        .map_err(|e| format!("auth-files request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("auth-files HTTP {}", resp.status()));
    }
    let body: AuthFilesResponse = resp
        .json()
        .await
        .map_err(|e| format!("auth-files parse: {e}"))?;
    Ok(body.files)
}

struct WhamParsed {
    used_percent: f64,
    reset_at: i64,
    plan_type: Option<String>,
}

async fn fetch_wham_usage(
    client: &reqwest::Client,
    settings: &CpaManagementConfig,
    auth_index: &str,
) -> Result<Option<WhamParsed>, String> {
    let url = format!(
        "{}/v0/management/api-call",
        settings.url.trim_end_matches('/')
    );
    let payload = serde_json::json!({
        "auth_index": auth_index,
        "method": "GET",
        "url": "https://chatgpt.com/backend-api/wham/usage",
        "header": {
            "Authorization": "Bearer $TOKEN$",
            "Accept": "application/json",
            "User-Agent": "grok-build-cpa-quota",
        }
    });
    let resp = client
        .post(&url)
        .timeout(CPA_HTTP_TIMEOUT)
        .header("Authorization", format!("Bearer {}", settings.key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("api-call: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("api-call HTTP {}", resp.status()));
    }
    let wrap: ApiCallResponse = resp
        .json()
        .await
        .map_err(|e| format!("api-call parse: {e}"))?;
    if wrap.status_code != 200 {
        return Err(format!("upstream HTTP {}", wrap.status_code));
    }
    let body = wrap.body.unwrap_or_default();
    let usage: WhamUsage = serde_json::from_str(&body).map_err(|e| format!("wham parse: {e}"))?;
    let window = usage
        .rate_limit
        .as_ref()
        .and_then(|r| r.primary_window.as_ref())
        .ok_or_else(|| "no primary_window".to_string())?;
    let used = window.used_percent.unwrap_or(0.0);
    let reset_at = window.reset_at.ok_or_else(|| "no reset_at".to_string())?;
    Ok(Some(WhamParsed {
        used_percent: used,
        reset_at,
        plan_type: usage.plan_type,
    }))
}

#[derive(Debug, Deserialize)]
struct ApiCallResponse {
    status_code: u16,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhamUsage {
    #[serde(default)]
    plan_type: Option<String>,
    #[serde(default)]
    rate_limit: Option<WhamRateLimit>,
}

#[derive(Debug, Deserialize)]
struct WhamRateLimit {
    #[serde(default)]
    primary_window: Option<WhamWindow>,
}

#[derive(Debug, Deserialize)]
struct WhamWindow {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    reset_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(email: &str, used: f64, reset: i64) -> CpaAccountQuota {
        CpaAccountQuota {
            email: email.into(),
            used_percent: used,
            reset_at: reset,
            plan_type: Some("plus".into()),
        }
    }

    #[test]
    fn top_three_least_remaining() {
        let accounts = vec![
            acc("a@x.com", 10.0, 100),
            acc("b@x.com", 90.0, 200),
            acc("c@x.com", 50.0, 150),
            acc("d@x.com", 80.0, 120),
        ];
        let top = top_accounts(&accounts, 3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].email, "b@x.com");
        assert_eq!(top[1].email, "d@x.com");
        assert_eq!(top[2].email, "c@x.com");
    }

    #[test]
    fn prompt_line_format_shanghai() {
        // 2026-08-05 12:00:00 UTC+8 = 2026-08-05 04:00:00 UTC
        let reset = shanghai()
            .with_ymd_and_hms(2026, 8, 5, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp();
        let snap = CpaQuotaSnapshot {
            model_id: "gpt-luna".into(),
            accounts: vec![acc("a@x.com", 24.0, reset)],
        };
        let line = snap.prompt_status_line().unwrap();
        assert_eq!(line, "76%(08/05 12:00)");
    }

    #[test]
    fn usage_block_lists_accounts() {
        let snap = CpaQuotaSnapshot {
            model_id: "m".into(),
            accounts: vec![acc("a@x.com", 24.0, 1_785_903_012)],
        };
        let text = snap.usage_block_text();
        assert!(text.contains("a@x.com"));
        assert!(text.contains("76%"));
        assert!(!text.contains("24% used"));
    }

    #[test]
    fn remaining_percent_is_clamped() {
        assert_eq!(remaining_percent(24.0), 76.0);
        assert_eq!(remaining_percent(-5.0), 100.0);
        assert_eq!(remaining_percent(125.0), 0.0);
        assert_eq!(remaining_percent(f64::NAN), 0.0);
    }
}
