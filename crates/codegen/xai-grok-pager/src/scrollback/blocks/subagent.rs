//! SubagentBlock — scrollback entries for subagent lifecycle.
//!
//! Similar to BgTaskBlock: always collapsed, animated bullet while running,
//! colored bullet when done. Enter / Ctrl-F opens the subagent view.
//!
//! Two modes:
//! - **Blocking** (sync): Single `Started` block. Blinks while running,
//!   turns green/red when done. Text: `Subagent "description"`
//! - **Background** (async): `Started` block stays forever (turns gray).
//!   A separate `Completed`/`Failed` block is added when done.
//!   Started text: `Subagent started: "description"`
//!   Completed text: `Subagent completed in 43s: "description"`

use std::time::Duration;

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::app::subagent::format_subagent_meta;
use crate::render::color::blend_color;
use crate::render::line_utils::truncate_str;
use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockOutput, DisplayMode};
use crate::theme::Theme;
use crate::util::format_duration;

/// What kind of subagent lifecycle event this block represents.
#[derive(Debug, Clone)]
pub enum SubagentBlockKind {
    /// Subagent is running (or was running — `finish_running` stops animation).
    Started,
    /// Subagent completed successfully.
    Completed { elapsed: Duration },
    /// Subagent failed.
    Failed {
        elapsed: Duration,
        error: Option<String>,
    },
    /// Subagent was cancelled.
    Cancelled { elapsed: Duration },
}

/// Subagent scrollback block.
///
/// Always collapsed, not foldable, groupable, selectable.
/// Enter / Ctrl-F opens the subagent view.
#[derive(Debug, Clone)]
pub struct SubagentBlock {
    /// Human-readable description of the task.
    pub description: String,
    /// Child session ID (for opening the subagent view).
    pub child_session_id: String,
    /// Subagent type (e.g. "general-purpose", "explore").
    pub subagent_type: String,
    /// Named persona applied to this subagent, if any.
    pub persona: Option<String>,
    /// Role that supplied defaults for this subagent, if any.
    pub role: Option<String>,
    /// Effective model ID used by the subagent, if available.
    pub model: Option<String>,
    /// Whether the subagent was launched in background mode.
    pub is_background: bool,
    /// Lifecycle kind.
    pub kind: SubagentBlockKind,
    /// Live activity label from the child session's turn tracker.
    ///
    /// Updated on each `SubagentProgress` tick while the subagent is running.
    /// Shown inline in the collapsed scrollback line (e.g. "Thinking",
    /// "Running: cargo build") so the user sees interactive progress without
    /// opening the subagent view.
    pub activity_label: Option<String>,
}

impl SubagentBlock {
    /// Create a "Subagent started" block (for both sync and async).
    pub fn started(
        description: impl Into<String>,
        child_session_id: impl Into<String>,
        subagent_type: impl Into<String>,
        persona: Option<String>,
        role: Option<String>,
        model: Option<String>,
        is_background: bool,
    ) -> Self {
        Self {
            description: description.into(),
            child_session_id: child_session_id.into(),
            subagent_type: subagent_type.into(),
            persona,
            role,
            model,
            is_background,
            kind: SubagentBlockKind::Started,
            activity_label: None,
        }
    }

    /// Create a "Subagent completed" block (background mode only).
    pub fn completed(
        description: impl Into<String>,
        child_session_id: impl Into<String>,
        elapsed: Duration,
    ) -> Self {
        Self {
            description: description.into(),
            child_session_id: child_session_id.into(),
            subagent_type: String::new(),
            persona: None,
            role: None,
            model: None,
            is_background: true,
            kind: SubagentBlockKind::Completed { elapsed },
            activity_label: None,
        }
    }

    /// Create a "Subagent failed" block (background mode only).
    pub fn failed(
        description: impl Into<String>,
        child_session_id: impl Into<String>,
        elapsed: Duration,
        error: Option<String>,
    ) -> Self {
        Self {
            description: description.into(),
            child_session_id: child_session_id.into(),
            subagent_type: String::new(),
            persona: None,
            role: None,
            model: None,
            is_background: true,
            kind: SubagentBlockKind::Failed { elapsed, error },
            activity_label: None,
        }
    }

    /// Create a "Subagent cancelled" block (background mode only).
    pub fn cancelled(
        description: impl Into<String>,
        child_session_id: impl Into<String>,
        elapsed: Duration,
    ) -> Self {
        Self {
            description: description.into(),
            child_session_id: child_session_id.into(),
            subagent_type: String::new(),
            persona: None,
            role: None,
            model: None,
            is_background: true,
            kind: SubagentBlockKind::Cancelled { elapsed },
            activity_label: None,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.kind, SubagentBlockKind::Started)
    }
}

/// Truncate description and wrap in quotes for display.
fn quoted_desc(desc: &str, max_width: usize) -> String {
    // Reserve 2 chars for quotes
    if max_width <= 2 {
        return "\u{201C}\u{2026}\u{201D}".to_string(); // "…"
    }
    let inner = truncate_str(desc, max_width - 2);
    format!("\u{201C}{inner}\u{201D}")
}

impl BlockContent for SubagentBlock {
    fn output(&self, ctx: &BlockContext) -> BlockOutput {
        let theme = Theme::current();
        let muted = theme.muted();
        let w = ctx.width as usize;

        let localized_line = |key: &str, args: &[(&str, &str)]| {
            Line::from(Span::styled(xai_grok_i18n::t_fmt(key, args), muted))
        };

        let line = match (&self.kind, self.is_background) {
            (SubagentBlockKind::Started, bg) => {
                let activity = self
                    .activity_label
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("");
                let meta = format_subagent_meta(
                    self.persona.as_deref(),
                    self.role.as_deref(),
                    self.model.as_deref(),
                );
                let key = match (bg, activity.is_empty()) {
                    (true, true) => "scrollback.subagent.started",
                    (true, false) => "scrollback.subagent.started_activity",
                    (false, true) => "scrollback.subagent.running",
                    (false, false) => "scrollback.subagent.running_activity",
                };
                let overhead = xai_grok_i18n::t_fmt(
                    key,
                    &[("description", ""), ("activity", activity), ("meta", &meta)],
                )
                .width();
                let desc = quoted_desc(&self.description, w.saturating_sub(overhead));
                localized_line(
                    key,
                    &[
                        ("description", &desc),
                        ("activity", activity),
                        ("meta", &meta),
                    ],
                )
            }
            (SubagentBlockKind::Completed { elapsed }, _) => {
                let time_str = format_duration(*elapsed);
                let key = "scrollback.subagent.completed";
                let overhead =
                    xai_grok_i18n::t_fmt(key, &[("duration", &time_str), ("description", "")])
                        .width();
                let desc = quoted_desc(&self.description, w.saturating_sub(overhead));
                localized_line(key, &[("duration", &time_str), ("description", &desc)])
            }
            (SubagentBlockKind::Failed { elapsed, error }, _) => {
                let time_str = format_duration(*elapsed);
                let key = if error.is_some() {
                    "scrollback.subagent.failed_error"
                } else {
                    "scrollback.subagent.failed"
                };
                let error = error.as_deref().unwrap_or("");
                let overhead = xai_grok_i18n::t_fmt(
                    key,
                    &[
                        ("duration", &time_str),
                        ("error", error),
                        ("description", ""),
                    ],
                )
                .width();
                let desc = quoted_desc(&self.description, w.saturating_sub(overhead));
                localized_line(
                    key,
                    &[
                        ("duration", &time_str),
                        ("error", error),
                        ("description", &desc),
                    ],
                )
            }
            (SubagentBlockKind::Cancelled { elapsed }, _) => {
                let time_str = format_duration(*elapsed);
                let key = "scrollback.subagent.cancelled";
                let overhead =
                    xai_grok_i18n::t_fmt(key, &[("duration", &time_str), ("description", "")])
                        .width();
                let desc = quoted_desc(&self.description, w.saturating_sub(overhead));
                localized_line(key, &[("duration", &time_str), ("description", &desc)])
            }
        };

        BlockOutput {
            lines: vec![line.into()],
        }
    }

    fn accent(&self, ctx: &BlockContext) -> Option<AccentStyle> {
        let theme = Theme::current();
        match &self.kind {
            SubagentBlockKind::Started if ctx.is_running => {
                Some(AccentStyle::static_color(theme.accent_running))
            }
            _ => None,
        }
    }

    fn bullet(&self, ctx: &BlockContext) -> Option<AccentStyle> {
        let theme = Theme::current();
        match &self.kind {
            SubagentBlockKind::Started => {
                if ctx.is_running {
                    let dim = ctx.appearance.scrollback.display.dim_accent;
                    let dimmed = blend_color(theme.bg_base, theme.accent_running, dim)
                        .unwrap_or(theme.accent_running);
                    Some(AccentStyle::animated(dimmed))
                } else {
                    // Finished — gray bullet (same as bg task "started" after completion)
                    None
                }
            }
            SubagentBlockKind::Completed { .. } => {
                Some(AccentStyle::static_color(theme.accent_success))
            }
            SubagentBlockKind::Failed { .. } | SubagentBlockKind::Cancelled { .. } => {
                Some(AccentStyle::static_color(theme.accent_error))
            }
        }
    }

    fn has_vpad(&self, _ctx: &BlockContext) -> bool {
        false
    }

    fn has_raw_mode(&self) -> bool {
        false
    }

    fn is_foldable(&self) -> bool {
        false
    }

    fn default_display_mode(&self) -> DisplayMode {
        DisplayMode::Collapsed
    }

    fn is_selectable(&self) -> bool {
        true
    }

    fn has_bullet(&self, _ctx: &BlockContext) -> bool {
        true
    }

    fn is_groupable(&self) -> bool {
        true
    }
}
