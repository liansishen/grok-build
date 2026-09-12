//! `StatusLineContext` is the payload clients receive.
//! What each field means is documented once, in `xai-grok-pager/docs/user-guide/25-status-line.md`, and a test holds that guide to this type.
//! The comments here record only what that guide cannot.
//!
//! Two rules hold it together.
//! A value Grok cannot source is `None` rather than zero.
//! Fields are snake_case, the one exception to the camelCase rule in `xai-grok-pager/docs/internal/28-extension-methods.md`.
//! They stay that way because renaming one silently breaks every script that reads it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// This number names the payload's shape, which a script branches on instead of the release in `version`.
/// Adding a field never bumps it; removing or retyping one does.
pub const STATUS_LINE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineContext {
    /// The one field whose own `default` matters: `Default` sets the current version, so without this an old payload would claim to be current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The client fills this, not the agent, since the name is renameable locally.
    /// It is absent from the notification and present on a command row's stdin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    pub model: StatusLineModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_usage: Option<BTreeMap<String, StatusLineModelUsage>>,
    /// Cumulative usage since the current agent process started or resumed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_model_usage: Option<BTreeMap<String, StatusLineModelUsage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<StatusLineBilling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<StatusLineQuota>,
    pub workspace: StatusLineWorkspace,
    pub version: String,
    pub cost: StatusLineCost,
    pub context_window: StatusLineContextWindow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<StatusLineEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<StatusLineWorktree>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<StatusLineTurn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<StatusLineGeneration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<StatusLineTrigger>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusLineTrigger {
    State,
    #[serde(rename = "refresh_interval")]
    RefreshInterval,
}

impl Default for StatusLineContext {
    fn default() -> Self {
        Self {
            schema_version: Some(STATUS_LINE_SCHEMA_VERSION),
            cwd: String::new(),
            session_id: None,
            session_name: None,
            prompt_id: None,
            transcript_path: None,
            model: StatusLineModel::default(),
            model_usage: None,
            process_model_usage: None,
            billing: None,
            quota: None,
            workspace: StatusLineWorkspace::default(),
            version: String::new(),
            cost: StatusLineCost::default(),
            context_window: StatusLineContextWindow::default(),
            effort: None,
            worktree: None,
            turn: None,
            trigger: None,
            generation: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineTurn {
    /// The value is Unix milliseconds, so a client subtracts it from its own clock.
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineGeneration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_token_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f64>,
    pub estimated: bool,
    pub stale: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineWorktree {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_worktree_root: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineModelUsage {
    /// Uncached prompt tokens. Add both cache buckets for the full prompt.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub model_calls: u64,
    pub api_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd_ticks: Option<i64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cost_is_partial: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineEffort {
    pub level: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineWorkspace {
    pub current_dir: String,
    /// This is not `project_dir`, which names a launch directory elsewhere.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_worktree: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<StatusLineRepo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineRepo {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineCost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// The clock starts when this process attached, not when the session was created.
    pub total_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_api_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineBilling {
    pub usage_percentage: Option<f64>,
    pub effective_usage_percentage: Option<f64>,
    pub period_type: Option<String>,
    pub period_end: Option<String>,
    pub pay_as_you_go: Option<bool>,
    pub on_demand_cap_cents: Option<i64>,
    pub on_demand_used_cents: Option<i64>,
    pub prepaid_balance_cents: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineQuota {
    pub model_id: String,
    pub accounts: Vec<StatusLineQuotaAccount>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineQuotaAccount {
    pub email: String,
    pub used_percentage: f64,
    pub remaining_percentage: f64,
    pub reset_at: i64,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineContextWindow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    /// This is not `total_*`, which is used elsewhere for the live window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_output_tokens: Option<u64>,
    /// This is cumulative, where `current_usage` elsewhere is one call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_usage: Option<StatusLineSessionUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percentage: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_threshold_percent: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineSessionUsage {
    /// This count is disjoint from the cache buckets, so the three sum without overlap.
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
