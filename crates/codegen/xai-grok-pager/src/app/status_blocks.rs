//! Read-only system-block text for `/queue`, `/tasks`, and `/usage`.
//!
//! Plain text committed into scrollback — the primary inspection surface in
//! minimal mode (no interactive panes). Kept out of `dispatch` for easy
//! unit tests.

use crate::app::agent::BgTaskStatus;
use crate::app::agent_view::AgentView;
use crate::app::subagent::format_subagent_label;
use crate::util::{format_duration, group_thousands};
use unicode_width::UnicodeWidthStr;
use xai_grok_i18n::{t, t_fmt};

/// `/queue` body — a read-only list of the queued prompts.
///
/// Server-authoritative shared-queue rows (the in-flight prompt excluded) come
/// first in broadcast order, then the local drip-feed queue — matching
/// [`crate::views::queue_pane::QueuePane::sync_from_merged`]'s ordering.
pub(crate) fn queue_block_text(agent: &AgentView) -> String {
    let running_id = agent.session.current_prompt_id.as_deref();

    let mut rows: Vec<String> = Vec::new();
    let mut pos = 1usize;
    for wire in &agent.shared_queue {
        if running_id == Some(wire.id.as_str()) {
            continue;
        }
        rows.push(format_queue_row(pos, &wire.text));
        pos += 1;
    }
    for prompt in &agent.session.pending_prompts {
        rows.push(format_queue_row(pos, &prompt.text));
        pos += 1;
    }

    if rows.is_empty() {
        t("status.queue_empty").to_string()
    } else {
        let count = rows.len().to_string();
        let header = if rows.len() == 1 {
            t_fmt("tasks.queued_header_singular", &[("count", &count)])
        } else {
            t_fmt("tasks.queued_header_plural", &[("count", &count)])
        };
        join_header_rows(header, rows)
    }
}

///
/// [`crate::views::tasks_pane::TasksPane`] without its styled rows.
pub(crate) fn tasks_block_text(agent: &AgentView) -> String {
    let mut rows: Vec<String> = Vec::new();

    let mut workflows: Vec<_> = agent.workflow_runs.iter().collect();
    workflows.sort_by(|a, b| {
        b.is_active()
            .cmp(&a.is_active())
            .then(b.received_at.cmp(&a.received_at))
            .then(a.run_id.cmp(&b.run_id))
    });
    for run in workflows {
        let active = run.active_agent_count();
        let agents = match active {
            0 => String::new(),
            1 => t("tasks.agents_one").to_string(),
            n => t_fmt("tasks.agents_many", &[("n", &n.to_string())]),
        };
        let phase = run
            .current_phase
            .as_deref()
            .map(str::trim)
            .filter(|phase| !phase.is_empty())
            .map(|phase| format!(" · {phase}"))
            .unwrap_or_default();
        let workflow_label = t_fmt("tasks.workflow", &[("name", run.name.as_str())]);
        let status = if run.is_active() {
            t("tasks.workflow.running").to_string()
        } else {
            let status_key = match run.status.as_str() {
                "budget_limited" => Some("tasks.workflow.status_budget_limited"),
                "cancelled" | "canceled" => Some("tasks.workflow.status_cancelled"),
                "complete" | "completed" | "done" => Some("tasks.workflow.status_complete"),
                "failed" => Some("tasks.workflow.status_failed"),
                "interrupted" => Some("tasks.workflow.status_interrupted"),
                _ => None,
            };
            status_key
                .map(t)
                .map(str::to_string)
                .unwrap_or_else(|| run.status.replace('_', " "))
        };
        rows.push(format!(
            "  {}{workflow_label}{phase}{agents}  ({})",
            pad_right(status.as_str(), 9),
            format_duration(std::time::Duration::from_millis(run.live_elapsed_ms()))
        ));
    }

    // ── Subagents ──
    let mut subs: Vec<_> = agent
        .subagent_sessions
        .values()
        .filter(|s| s.workflow_run_id.is_none())
        .collect();
    subs.sort_by(|a, b| {
        b.is_running()
            .cmp(&a.is_running())
            .then(b.started_at.cmp(&a.started_at))
            .then(a.child_session_id.cmp(&b.child_session_id))
    });
    for info in subs {
        let (type_label, desc) = format_subagent_label(info);
        let status = if info.pending_kill {
            t("tasks.status_stopping")
        } else if info.is_running() {
            t("tasks.workflow.running")
        } else {
            match info.status.as_deref() {
                Some("done" | "complete" | "completed") | None => t("tasks.status_done"),
                Some("failed") => t("tasks.status_failed"),
                Some("stopping") => t("tasks.status_stopping"),
                Some(status) => status,
            }
        };
        let label = if desc.is_empty() {
            type_label
        } else {
            format!("{type_label} · {desc}")
        };
        rows.push(format!(
            "  {}{label}  ({})",
            pad_right(status, 9),
            format_duration(info.display_elapsed())
        ));
    }

    // ── Background tasks / monitors ──
    let mut tasks: Vec<_> = agent.session.bg_tasks.values().collect();
    tasks.sort_by(|a, b| {
        let (ar, br) = (
            a.status == BgTaskStatus::Running,
            b.status == BgTaskStatus::Running,
        );
        br.cmp(&ar)
            .then(b.start_time.cmp(&a.start_time))
            .then(a.task_id.cmp(&b.task_id))
    });
    for task in tasks {
        let kind = if task.is_monitor {
            t("tasks.kind_monitor")
        } else {
            t("tasks.kind_task")
        };
        let one_line = task
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| first_nonempty_line(&task.command));
        let status = if task.pending_kill {
            t("tasks.status_stopping")
        } else {
            match task.status {
                BgTaskStatus::Running => t("tasks.workflow.running"),
                BgTaskStatus::Done => t("tasks.status_done"),
                BgTaskStatus::Failed => t("tasks.status_failed"),
            }
        };
        rows.push(format!(
            "  {}{kind} · {one_line}  ({})",
            pad_right(status, 9),
            format_duration(task.elapsed())
        ));
    }

    // ── Scheduled (/loop) tasks ──
    let mut sched: Vec<_> = agent.session.scheduled_tasks.values().collect();
    sched.sort_by(|a, b| {
        a.tag
            .cmp(&b.tag)
            .then(a.human_schedule.cmp(&b.human_schedule))
            .then(a.task_id.cmp(&b.task_id))
    });
    for info in sched {
        rows.push(format!(
            "  {}{} · {} · {}",
            pad_right(t("tasks.status_scheduled"), 9),
            info.tag,
            info.human_schedule,
            first_nonempty_line(&info.prompt)
        ));
    }

    if rows.is_empty() {
        t("tasks.empty").to_string()
    } else {
        let count = rows.len().to_string();
        let header = if rows.len() == 1 {
            t_fmt("tasks.header_singular", &[("count", &count)])
        } else {
            t_fmt("tasks.header_plural", &[("count", &count)])
        };
        join_header_rows(header, rows)
    }
}

/// `/usage` body — per-session token and cost totals, scoped to the ledger's
/// lifetime: since session start, or since the last `/resume`.
pub(crate) fn session_usage_block_text(
    usage: &xai_grok_shell::extensions::notification::PromptUsage,
) -> String {
    let t = &usage.totals;
    if t.model_calls == 0 && usage.model_usage.is_empty() {
        return xai_grok_i18n::t(if usage.usage_is_incomplete {
            "usage.session.empty_incomplete"
        } else {
            "usage.session.empty"
        })
        .to_string();
    }

    let input = group_thousands(t.input_tokens);
    let cached = group_thousands(t.cached_read_tokens);
    let output = group_thousands(t.output_tokens);
    let reasoning = group_thousands(t.reasoning_tokens);
    let total = group_thousands(t.total_tokens);
    let calls = group_thousands(t.model_calls);
    let duration = format_duration(std::time::Duration::from_millis(t.api_duration_ms));
    let cost = format_cost(t);

    let mut rows = vec![
        xai_grok_i18n::t_fmt(
            "usage.session.input_tokens",
            &[("tokens", input.as_str()), ("cached", cached.as_str())],
        ),
        xai_grok_i18n::t_fmt(
            "usage.session.output_tokens",
            &[
                ("tokens", output.as_str()),
                ("reasoning", reasoning.as_str()),
            ],
        ),
        xai_grok_i18n::t_fmt("usage.session.total_tokens", &[("tokens", total.as_str())]),
        xai_grok_i18n::t_fmt(
            "usage.session.api_and_calls",
            &[("calls", calls.as_str()), ("duration", duration.as_str())],
        ),
        xai_grok_i18n::t_fmt("usage.session.cost", &[("cost", cost.as_str())]),
    ];

    if usage.model_usage.len() > 1 {
        rows.push(xai_grok_i18n::t("usage.session.by_model").to_string());
        for (model, m) in &usage.model_usage {
            let input = group_thousands(m.input_tokens);
            let output = group_thousands(m.output_tokens);
            let cost = format_cost(m);
            rows.push(xai_grok_i18n::t_fmt(
                "usage.session.model_row",
                &[
                    ("model", model.as_str()),
                    ("input", input.as_str()),
                    ("output", output.as_str()),
                    ("cost", cost.as_str()),
                ],
            ));
        }
    }

    if usage.usage_is_incomplete {
        rows.push(xai_grok_i18n::t("usage.session.incomplete_note").to_string());
    }

    join_header_rows(
        xai_grok_i18n::t("usage.session.header").to_string(),
        rows,
    )
}

/// Cost cell. Ticks are 1e10 per USD; partial sums are scrubbed to absent.
fn format_cost(m: &xai_grok_shell::extensions::notification::PromptUsageModel) -> String {
    use xai_grok_shell::extensions::notification::ticks_to_usd;
    match m.cost_usd_ticks {
        Some(ticks) => format!("${:.4}", ticks_to_usd(ticks)),
        None if m.cost_is_partial => {
            xai_grok_i18n::t("usage.session.cost_partial").to_string()
        }
        None => xai_grok_i18n::t("usage.session.cost_unavailable").to_string(),
    }
}

/// Compact prompt-line label: `12.3k tok`, `12.3k tok(95.0%)`, or with cost.
///
/// Returns `None` when there is nothing useful to show (no model calls yet).
/// Cost is omitted when the ledger has no complete cost stamp (matches
/// `[ui].show_session_usage_bar`: tokens always, price only when known).
/// Cache rate is `cached_read_tokens / input_tokens` (one decimal place).
pub(crate) fn session_usage_bar_label(
    usage: &xai_grok_shell::extensions::notification::PromptUsage,
) -> Option<String> {
    let t = &usage.totals;
    if t.model_calls == 0 && usage.model_usage.is_empty() && t.total_tokens == 0 {
        return None;
    }
    let compact_tokens = format_compact_tokens(t.total_tokens);
    let tokens = t_fmt(
        "usage.session.compact_tokens",
        &[("tokens", compact_tokens.as_str())],
    );
    let tokens = match cache_hit_rate_percent(t.cached_read_tokens, t.input_tokens) {
        Some(pct) => format!("{tokens}({pct:.1}%)"),
        None => tokens,
    };
    match t.cost_usd_ticks {
        Some(ticks) if !t.cost_is_partial && !usage.usage_is_incomplete => {
            use xai_grok_shell::extensions::notification::ticks_to_usd;
            Some(format!("{tokens} · ${:.4}", ticks_to_usd(ticks)))
        }
        _ => Some(tokens),
    }
}

/// Input-cache hit rate as a percentage, or `None` when there is no input.
pub(crate) fn cache_hit_rate_percent(cached_read_tokens: u64, input_tokens: u64) -> Option<f64> {
    if input_tokens == 0 {
        return None;
    }
    Some((cached_read_tokens as f64) * 100.0 / (input_tokens as f64))
}

fn format_compact_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.2}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn pad_right(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(UnicodeWidthStr::width(value)))
    )
}

/// First non-empty, trimmed line of `text` (empty string if none). Collapses a
/// multi-line prompt/command to a single display line.
pub(crate) fn first_nonempty_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}

/// Format one `/queue` row as `  #N  <first non-empty line>` with a
/// `(+K more lines)` suffix for multi-line prompts.
fn format_queue_row(pos: usize, text: &str) -> String {
    let first_line = first_nonempty_line(text);
    let extra = text.lines().count().saturating_sub(1);
    if extra > 0 {
        let singular = extra == 1;
        let extra = extra.to_string();
        let suffix = if singular {
            t_fmt("tasks.more_lines_singular", &[("extra", &extra)])
        } else {
            t_fmt("tasks.more_lines_plural", &[("extra", &extra)])
        };
        format!("  #{pos}  {first_line}  {suffix}")
    } else {
        format!("  #{pos}  {first_line}")
    }
}

/// Join a header line above its rows into a single block string.
fn join_header_rows(header: String, rows: Vec<String>) -> String {
    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use xai_grok_shell::extensions::notification::{PromptUsage, PromptUsageModel};

    fn model_row(input: u64, output: u64, ticks: Option<i64>) -> PromptUsageModel {
        PromptUsageModel {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            cached_read_tokens: 0,
            cache_creation_tokens: 0,
            reasoning_tokens: 0,
            model_calls: 1,
            api_duration_ms: 1_000,
            cost_usd_ticks: ticks,
            cost_is_partial: false,
            cost_missing_calls: 0,
        }
    }

    #[test]
    fn session_usage_block_empty_ledger() {
        let usage = PromptUsage::default();
        assert_eq!(
            session_usage_block_text(&usage),
            "Session usage: no model calls yet in this session."
        );

        // Empty but incomplete must not read as a clean zero.
        let incomplete = PromptUsage {
            usage_is_incomplete: true,
            ..Default::default()
        };
        assert!(session_usage_block_text(&incomplete).contains("incomplete"));
    }

    #[test]
    fn session_usage_bar_label_tokens_only_without_cost() {
        let usage = PromptUsage {
            totals: model_row(12_500, 500, None),
            ..Default::default()
        };
        // 12_500 + 500 = 13_000 → 13.0k
        assert_eq!(
            session_usage_bar_label(&usage).as_deref(),
            Some("13.0k tok")
        );
    }

    #[test]
    fn session_usage_bar_label_includes_cache_rate_one_decimal() {
        let mut totals = model_row(1_000_000, 100_000, None);
        totals.cached_read_tokens = 950_000;
        let usage = PromptUsage {
            totals,
            ..Default::default()
        };
        let label = session_usage_bar_label(&usage).unwrap();
        assert!(
            label.starts_with("1.1M tok(95.0%)"),
            "expected cache rate on total tokens, got {label}"
        );
    }

    #[test]
    fn session_usage_bar_label_includes_cost_when_present() {
        let usage = PromptUsage {
            totals: model_row(1_000, 100, Some(12_345_000_000)),
            ..Default::default()
        };
        let label = session_usage_bar_label(&usage).unwrap();
        assert!(label.contains("tok"));
        assert!(label.contains("$1.2345"), "got {label}");
    }

    #[test]
    fn session_usage_block_formats_tokens_and_cost() {
        let mut totals = model_row(1_234_567, 45_678, Some(12_345_000_000));
        totals.cached_read_tokens = 1_000_000;
        totals.reasoning_tokens = 12_000;
        totals.model_calls = 42;
        totals.api_duration_ms = 192_000;
        let usage = PromptUsage {
            totals,
            ..Default::default()
        };
        let text = session_usage_block_text(&usage);
        // Snapshot pins content and column alignment together; single-model
        // sessions must skip the redundant by-model breakdown.
        insta::assert_snapshot!("session_usage_block_full", text);
    }

    #[test]
    fn session_usage_block_lists_models_when_multiple() {
        let mut usage = PromptUsage {
            totals: model_row(150, 15, None),
            ..Default::default()
        };
        usage
            .model_usage
            .insert("grok-build".into(), model_row(100, 10, None));
        usage
            .model_usage
            .insert("grok-4".into(), model_row(50, 5, None));
        let text = session_usage_block_text(&usage);
        assert!(text.contains("By model:"), "{text}");
        assert!(text.contains("grok-build: 100 in / 10 out"), "{text}");
        assert!(text.contains("grok-4: 50 in / 5 out"), "{text}");
    }

    #[test]
    fn session_usage_block_absent_cost_is_unknown_not_free() {
        let usage = PromptUsage {
            totals: model_row(100, 10, None),
            ..Default::default()
        };
        let text = session_usage_block_text(&usage);
        insta::assert_snapshot!("session_usage_block_absent_cost", text);
        // Unknown cost must never read as free.
        assert!(!text.contains("$0"), "{text}");
    }

    #[test]
    fn session_usage_block_flags_partial_and_incomplete() {
        let mut totals = model_row(100, 10, None);
        totals.cost_is_partial = true;
        let usage = PromptUsage {
            totals,
            usage_is_incomplete: true,
            ..Default::default()
        };
        let text = session_usage_block_text(&usage);
        assert!(text.contains("not reported for some calls"), "{text}");
        assert!(text.contains("usage is incomplete"), "{text}");
    }

    #[test]
    fn group_thousands_groups_digits() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(1_000), "1,000");
        assert_eq!(group_thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn first_nonempty_line_skips_blank_leading_lines() {
        assert_eq!(first_nonempty_line("\n  \n  hello \nworld"), "hello");
        assert_eq!(first_nonempty_line("   "), "");
        assert_eq!(first_nonempty_line(""), "");
        assert_eq!(first_nonempty_line("only"), "only");
    }

    #[test]
    fn format_queue_row_single_line() {
        assert_eq!(format_queue_row(1, "fix the bug"), "  #1  fix the bug");
    }

    #[test]
    fn format_queue_row_multiline_reports_extra_lines() {
        assert_eq!(
            format_queue_row(2, "first\nsecond"),
            "  #2  first  (+1 more line)"
        );
        assert_eq!(
            format_queue_row(3, "first\nsecond\nthird"),
            "  #3  first  (+2 more lines)"
        );
    }
}
