//! Shared reasoning-effort dropdown levels for `/model` and `/effort`.

use xai_grok_shell::sampling::types::{ReasoningEffort, ReasoningEffortOption};

use crate::acp::model_state::EffortTokenError;
use crate::slash::command::ArgItem;

/// Effort levels in the built-in fallback menu (strongest first). `none`/`minimal`
/// are still accepted by `ReasoningEffort::from_str` for power users.
pub(crate) const EFFORT_LEVELS: &[ReasoningEffort] = &[
    ReasoningEffort::Xhigh,
    ReasoningEffort::High,
    ReasoningEffort::Medium,
    ReasoningEffort::Low,
];

pub(crate) fn effort_error_message(error: &EffortTokenError) -> String {
    match error {
        EffortTokenError::Unsupported => xai_grok_i18n::t_or(
            "slash.effort.error_unsupported",
            "current model does not support reasoning effort",
        )
        .to_string(),
        EffortTokenError::UnknownToken { token, offered } if offered.is_empty() => {
            xai_grok_i18n::t_or(
                "slash.effort.error_unknown_no_levels",
                "unknown effort level '{token}'; this model has no selectable effort levels",
            )
            .replace("{token}", token)
        }
        EffortTokenError::UnknownToken { token, offered } => xai_grok_i18n::t_or(
            "slash.effort.error_unknown",
            "unknown effort level '{token}'; use one of: {offered}",
        )
        .replace("{token}", token)
        .replace("{offered}", &offered.join(", ")),
        EffortTokenError::NoActiveModel => xai_grok_i18n::t_or(
            "slash.effort.error_no_active_model",
            "no active model to apply effort to",
        )
        .to_string(),
    }
}

pub(crate) fn effort_description(level: ReasoningEffort) -> &'static str {
    match level {
        ReasoningEffort::None => xai_grok_i18n::t_or("slash.effort.level_none", "No reasoning"),
        ReasoningEffort::Minimal => {
            xai_grok_i18n::t_or("slash.effort.level_minimal", "Minimal reasoning")
        }
        ReasoningEffort::Low => {
            xai_grok_i18n::t_or("slash.effort.level_low", "Faster, lighter reasoning")
        }
        ReasoningEffort::Medium => {
            xai_grok_i18n::t_or("slash.effort.level_medium", "Balanced reasoning")
        }
        ReasoningEffort::High => xai_grok_i18n::t_or("slash.effort.level_high", "Heavy reasoning"),
        ReasoningEffort::Xhigh => {
            xai_grok_i18n::t_or("slash.effort.level_xhigh", "Maximum reasoning")
        }
    }
}

/// The built-in menu used when the server sends no `reasoningEfforts`. Reproduces
/// the historical rows: labels are the lowercase level (via `Display`),
/// descriptions from `effort_description`. The active row is matched by value
/// against the session effort at render time, so `default` is left unset here.
pub(crate) fn legacy_effort_options() -> Vec<ReasoningEffortOption> {
    EFFORT_LEVELS
        .iter()
        .map(|&level| ReasoningEffortOption {
            id: level.as_str().to_string(),
            value: level,
            label: level.to_string(),
            description: Some(effort_description(level).to_string()),
            default: false,
        })
        .collect()
}

/// Build effort rows for autocomplete from a per-model option list.
///
/// - `mark_active` + `current_effort` mark the current session effort with `(active)`.
/// - `insert_text_for` controls what is inserted on select:
///   - `/effort`: the option id (`"deep"`)
///   - `/model` chained phase: `"ModelName deep"`
///
/// `match_text` gets an `a `/`b `/…` sort prefix so the matcher's alphabetical
/// tiebreak preserves the option order.
pub(crate) fn build_effort_arg_items(
    options: &[ReasoningEffortOption],
    current_effort: Option<ReasoningEffort>,
    mark_active: bool,
    insert_text_for: impl Fn(&ReasoningEffortOption) -> String,
) -> Vec<ArgItem> {
    options
        .iter()
        .enumerate()
        .map(|(idx, option)| {
            let active = mark_active && current_effort == Some(option.value);
            let active_suffix = if active {
                xai_grok_i18n::t_or("slash.common.active_suffix", " (active)")
            } else {
                ""
            };
            let insert_text = insert_text_for(option);
            // Sort-key prefix: 'a' for top row, 'b' for next, etc. Only
            // affects matcher tiebreak ordering, never rendered.
            let sort_prefix = char::from(b'a' + idx as u8);
            ArgItem {
                display: format!("{}{active_suffix}", option.label),
                match_text: format!("{sort_prefix} {insert_text}"),
                insert_text,
                description: option.description.clone().unwrap_or_default(),
            }
        })
        .collect()
}
