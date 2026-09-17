//! Scrollback entries for the subagent lifecycle.
//!
//! Similar to BgTaskBlock: always collapsed, an animated bullet while running, a colored bullet when done.
//! Enter / Ctrl-F opens the subagent view.
//!
//! Two modes:
//! - **Blocking** (sync): one `Started` block; it blinks while running and turns green/red when done. Text: `Subagent "description"`
//! - **Background** (async): the `Started` block stays forever (turns gray) and a separate `Completed`/`Failed` block is added when done.
//!   Started text: `Subagent started: "description"`
//!   Completed text: `Subagent completed in 43s: "description"`

use std::time::Duration;

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::app::subagent::format_subagent_meta;
use crate::appearance::AppearanceConfig;
use crate::render::line_utils::truncate_str;
use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockOutput, DisplayMode};
use crate::theme::Theme;
use crate::util::format_duration;

/// What kind of subagent lifecycle event this block represents.
#[derive(Debug, Clone)]
pub enum SubagentBlockKind {
    /// Subagent is running (or was running; `finish_running` stops animation).
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

/// Always collapsed and not foldable; groupable and selectable.
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
    /// Live activity label from the child session's turn tracker. The user sees interactive progress without opening
    /// the subagent view.
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
        // When selected, lift only the bold "Subagent" label to `text_primary` so it reads as undimmed
        // This mirrors `read.rs` and `search.rs`, which bump only the label and leave the rest at `muted`
        // The detail text (verb, description, meta) stays muted in every state
        let bold = if ctx.is_selected {
            theme.primary().add_modifier(Modifier::BOLD)
        } else {
            theme.muted().add_modifier(Modifier::BOLD)
        };
        let muted = theme.muted();
        let w = ctx.width as usize;

        let line = match (&self.kind, self.is_background) {
            (SubagentBlockKind::Started, bg) => {
                let verb = if bg {
                    xai_grok_i18n::t("scrollback.subagent.verb_started")
                } else {
                    xai_grok_i18n::t("scrollback.subagent.verb_running")
                };
                let activity_suffix: String = self
                    .activity_label
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|a| format!(" \u{00b7} {a}"))
                    .unwrap_or_default();
                let meta = format_subagent_meta(
                    self.persona.as_deref(),
                    self.role.as_deref(),
                    self.model.as_deref(),
                );
                // The label and the verb are catalog copy, so their width decides the budget
                // ("Subagent " + "running: " is 18 cells in English).
                let overhead = xai_grok_i18n::t("scrollback.subagent.label").width()
                    + verb.width()
                    + meta.width()
                    + activity_suffix.width();
                let desc = quoted_desc(&self.description, w.saturating_sub(overhead));
                let mut spans = vec![
                    Span::styled(xai_grok_i18n::t("scrollback.subagent.label"), bold),
                    Span::styled(verb, muted),
                    Span::styled(desc, muted),
                ];
                if !activity_suffix.is_empty() {
                    spans.push(Span::styled(activity_suffix, muted));
                }
                spans.push(Span::styled(meta, muted));
                Line::from(spans)
            }
            // Completed: Subagent completed in Xs: "description"
            (SubagentBlockKind::Completed { elapsed }, _) => {
                let time_str = format_duration(*elapsed);
                // "Subagent completed in Xs: " = 26 + time_str.len()
                let prefix_len = 26 + time_str.len();
                let desc = quoted_desc(&self.description, w.saturating_sub(prefix_len));
                Line::from(vec![
                    Span::styled(xai_grok_i18n::t("scrollback.subagent.label"), bold),
                    Span::styled(
                        xai_grok_i18n::t_fmt(
                            "scrollback.subagent.completed_in",
                            &[("time", &time_str)],
                        ),
                        muted,
                    ),
                    Span::styled(desc, muted),
                ])
            }
            // Failed: Subagent failed in Xs: "description"
            (SubagentBlockKind::Failed { elapsed, error }, _) => {
                let time_str = format_duration(*elapsed);
                let detail = error
                    .as_deref()
                    .map(|e| format!(" ({e})"))
                    .unwrap_or_default();
                let prefix_len = 21 + time_str.len() + detail.len();
                let desc = quoted_desc(&self.description, w.saturating_sub(prefix_len));
                Line::from(vec![
                    Span::styled(xai_grok_i18n::t("scrollback.subagent.label"), bold),
                    Span::styled(
                        xai_grok_i18n::t_fmt(
                            "scrollback.subagent.failed_in",
                            &[("time", &time_str), ("detail", &detail)],
                        ),
                        muted,
                    ),
                    Span::styled(desc, muted),
                ])
            }
            // Cancelled: Subagent cancelled in Xs: "description"
            (SubagentBlockKind::Cancelled { elapsed }, _) => {
                let time_str = format_duration(*elapsed);
                // "Subagent cancelled in Xs: " = 26 + time_str.len()
                let prefix_len = 26 + time_str.len();
                let desc = quoted_desc(&self.description, w.saturating_sub(prefix_len));
                Line::from(vec![
                    Span::styled(xai_grok_i18n::t("scrollback.subagent.label"), bold),
                    Span::styled(
                        xai_grok_i18n::t_fmt(
                            "scrollback.subagent.cancelled_in",
                            &[("time", &time_str)],
                        ),
                        muted,
                    ),
                    Span::styled(desc, muted),
                ])
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
                    Some(AccentStyle::animated_running(ctx, &theme))
                } else {
                    // Finished: gray bullet (same as bg task "started" after completion)
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

    fn has_vpad_for(&self, _appearance: &AppearanceConfig) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::AppearanceConfig;
    use crate::scrollback::types::{BlockContext, DisplayMode};
    use std::time::Duration;

    fn ctx() -> BlockContext {
        BlockContext {
            mode: DisplayMode::Collapsed,
            is_running: false,
            width: 100,
            raw: false,
            max_lines: None,
            appearance: AppearanceConfig::default(),
            is_selected: false,
            cwd: None,
        }
    }

    fn plain(block: &SubagentBlock) -> String {
        block
            .output(&ctx())
            .lines
            .iter()
            .map(|line| crate::scrollback::types::line_plain_text(&line.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Every subagent row's label and tense phrase is catalog-backed.
    #[test]
    fn subagent_row_copy_comes_from_the_catalog() {
        let started = SubagentBlock::started(
            "scan the tree",
            "child-1",
            "explore",
            None,
            None,
            None,
            false,
        );
        let completed =
            SubagentBlock::completed("scan the tree", "child-1", Duration::from_secs(3));
        let failed = SubagentBlock::failed(
            "scan the tree",
            "child-1",
            Duration::from_secs(3),
            Some("boom".into()),
        );
        let cancelled =
            SubagentBlock::cancelled("scan the tree", "child-1", Duration::from_secs(3));

        let (started_text, completed_text, failed_text, cancelled_text) =
            xai_grok_i18n::with_pseudo_locale(|| {
                (
                    plain(&started),
                    plain(&completed),
                    plain(&failed),
                    plain(&cancelled),
                )
            });

        assert!(
            started_text.contains("⟦scrollback.subagent.label⟧"),
            "started row label: {started_text}"
        );
        assert!(
            completed_text.contains("⟦scrollback.subagent.completed_in⟧"),
            "completed row phrase: {completed_text}"
        );
        assert!(
            failed_text.contains("⟦scrollback.subagent.failed_in⟧"),
            "failed row phrase: {failed_text}"
        );
        assert!(
            cancelled_text.contains("⟦scrollback.subagent.cancelled_in⟧"),
            "cancelled row phrase: {cancelled_text}"
        );
        assert!(
            completed_text.contains("⟦scrollback.subagent.label⟧"),
            "every row shares the label: {completed_text}"
        );
    }
}
