//! Local model pricing for BYOK / proxy endpoints that omit wire cost.
//!
//! Cost unit: **USD ticks** where `1 USD = 10^10` ticks (same as xAI
//! `cost_in_usd_ticks`). Estimates use:
//!
//! ```text
//! uncached_input = prompt_tokens - cached_prompt_tokens
//! usd = uncached/1e6 * input_price
//!     + cached/1e6 * cached_input_price
//!     + completion/1e6 * output_price
//! ticks = round(usd * 1e10)
//! ```
//!
//! Reasoning tokens are assumed billed inside `completion_tokens` (OpenAI-style).

use serde::{Deserialize, Serialize};

use crate::conversation::TokenUsage;

/// 1 USD = 10^10 ticks (matches wire `cost_in_usd_ticks` / ACP `costUsdTicks`).
pub const USD_TICKS_PER_USD: f64 = 1e10;

/// Where per-call cost should come from when both wire and local prices exist.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSource {
    /// Prefer server ticks when present; otherwise estimate from prices.
    #[default]
    Auto,
    /// Only use server-reported cost (legacy behavior).
    Server,
    /// Always estimate from local prices (ignore server ticks).
    Local,
}

impl CostSource {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("server") => Self::Server,
            Some("local") => Self::Local,
            _ => Self::Auto,
        }
    }
}

/// Per-model price sheet (USD per 1M tokens), typically from `[model.<id>]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelPricing {
    /// USD per 1M uncached input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_price_per_mtok: Option<f64>,
    /// USD per 1M cached input tokens. Falls back to `input_price_per_mtok`
    /// when unset (conservative: treat cache hits like uncached).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input_price_per_mtok: Option<f64>,
    /// USD per 1M output / completion tokens (includes reasoning when the
    /// provider folds reasoning into completion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_price_per_mtok: Option<f64>,
    /// Cost resolution policy. Default [`CostSource::Auto`].
    #[serde(default, skip_serializing_if = "is_auto_cost_source")]
    pub cost_source: CostSource,
}

fn is_auto_cost_source(s: &CostSource) -> bool {
    matches!(s, CostSource::Auto)
}

impl ModelPricing {
    /// True when no price numbers are set (cost_source alone does not count).
    pub fn is_empty(&self) -> bool {
        self.input_price_per_mtok.is_none()
            && self.cached_input_price_per_mtok.is_none()
            && self.output_price_per_mtok.is_none()
    }

    /// True when at least one rate is present so an estimate can be produced.
    pub fn can_estimate(&self) -> bool {
        !self.is_empty()
    }

    /// Whether local estimate should replace / fill `server_ticks`.
    ///
    /// `server_ticks` should already be normalized (`None` when unreported).
    pub fn should_apply_local(&self, server_ticks: Option<i64>) -> bool {
        if !self.can_estimate() {
            return false;
        }
        match self.cost_source {
            CostSource::Local => true,
            CostSource::Server => false,
            CostSource::Auto => server_ticks.is_none(),
        }
    }

    /// Estimate request cost in USD ticks from token usage.
    ///
    /// Returns `None` when no rates are configured. A zero-token request with
    /// rates set still yields `Some(0)` — callers that need “unreported”
    /// semantics should only call this when they intend a local estimate.
    pub fn estimate_usd_ticks(&self, usage: &TokenUsage) -> Option<i64> {
        if self.is_empty() {
            return None;
        }
        let input_rate = self.input_price_per_mtok.unwrap_or(0.0);
        let cached_rate = self
            .cached_input_price_per_mtok
            .unwrap_or(input_rate);
        let output_rate = self.output_price_per_mtok.unwrap_or(0.0);

        let prompt = u64::from(usage.prompt_tokens);
        let cached = u64::from(usage.cached_prompt_tokens).min(prompt);
        let uncached = prompt.saturating_sub(cached);
        let output = u64::from(usage.completion_tokens);

        let usd = (uncached as f64 / 1_000_000.0) * input_rate
            + (cached as f64 / 1_000_000.0) * cached_rate
            + (output as f64 / 1_000_000.0) * output_rate;

        // Non-negative; tiny positive costs round to at least 1 tick when usd > 0
        // so scrubbers that treat 0 as "unreported" keep a positive stamp.
        if usd <= 0.0 {
            return Some(0);
        }
        let ticks = (usd * USD_TICKS_PER_USD).round() as i64;
        Some(ticks.max(1))
    }

    /// Resolve effective cost ticks given optional server stamp.
    pub fn resolve_cost_ticks(&self, usage: &TokenUsage, server_ticks: Option<i64>) -> Option<i64> {
        if self.should_apply_local(server_ticks) {
            self.estimate_usd_ticks(usage)
        } else {
            server_ticks
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(prompt: u32, completion: u32, cached: u32) -> TokenUsage {
        TokenUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            reasoning_tokens: 0,
            cached_prompt_tokens: cached,
        }
    }

    #[test]
    fn estimates_openai_style_sol_rates() {
        // $5 / $0.50 / $30 per 1M
        let p = ModelPricing {
            input_price_per_mtok: Some(5.0),
            cached_input_price_per_mtok: Some(0.5),
            output_price_per_mtok: Some(30.0),
            cost_source: CostSource::Auto,
        };
        // 1M uncached in + 0 cache + 1M out = $5 + $30 = $35
        let ticks = p.estimate_usd_ticks(&usage(1_000_000, 1_000_000, 0)).unwrap();
        assert_eq!(ticks, (35.0 * USD_TICKS_PER_USD) as i64);
    }

    #[test]
    fn auto_prefers_server_when_present() {
        let p = ModelPricing {
            input_price_per_mtok: Some(5.0),
            output_price_per_mtok: Some(30.0),
            ..Default::default()
        };
        let u = usage(1000, 1000, 0);
        assert_eq!(p.resolve_cost_ticks(&u, Some(99)), Some(99));
        assert!(p.resolve_cost_ticks(&u, None).is_some());
    }

    #[test]
    fn local_overrides_server() {
        let p = ModelPricing {
            input_price_per_mtok: Some(1.0),
            output_price_per_mtok: Some(1.0),
            cost_source: CostSource::Local,
            ..Default::default()
        };
        let u = usage(1_000_000, 0, 0);
        assert_eq!(
            p.resolve_cost_ticks(&u, Some(1)),
            Some((1.0 * USD_TICKS_PER_USD) as i64)
        );
    }

    #[test]
    fn server_source_never_estimates() {
        let p = ModelPricing {
            input_price_per_mtok: Some(5.0),
            cost_source: CostSource::Server,
            ..Default::default()
        };
        assert_eq!(p.resolve_cost_ticks(&usage(100, 10, 0), None), None);
    }
}
