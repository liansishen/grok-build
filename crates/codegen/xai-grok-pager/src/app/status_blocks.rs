//! Read-only system-block text for `/queue`, `/tasks`, and `/usage`.
//!
//! Plain text committed into scrollback — the primary inspection surface in
//! minimal mode (no interactive panes). Kept out of `dispatch` for easy
//! unit tests.

use crate::app::agent::BgTaskStatus;
use crate::app::agent_view::AgentView;
use crate::app::subagent::format_subagent_label;
use crate::util::{format_duration, group_thousands};
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
        xai_grok_i18n::t("status.queue_empty").to_string()
    } else {
        let header = if rows.len() == 1 {
            t_fmt("tasks.queued_header_singular", &[("count", &rows.len().to_string())])
        } else {
            t_fmt("tasks.queued_header_plural", &[("count", &rows.len().to_string())])
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
        rows.push(format!(
            "  {:<9}{workflow_label}{phase}{agents}  ({})",
            if run.is_active() {
                t("tasks.status_running").to_string()
            } else {
                run.status.replace('_', " ")
            },
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
            t("tasks.status_running")
        } else {
            info.status.as_deref().unwrap_or(t("tasks.status_done"))
        };
        let label = if desc.is_empty() {
            type_label
        } else {
            format!("{type_label} · {desc}")
        };
        rows.push(format!(
            "  {status:<9}{label}  ({})",
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
        let kind = if task.is_monitor { t("tasks.kind_monitor") } else { t("tasks.kind_task") };
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
                BgTaskStatus::Running => t("tasks.status_running"),
                BgTaskStatus::Done => t("tasks.status_done"),
                BgTaskStatus::Failed => t("tasks.status_failed"),
            }
        };
        rows.push(format!(
            "  {status:<9}{kind} · {one_line}  ({})",
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
            "  {:<9}{} · {} · {}",
            t("tasks.status_scheduled"),
            info.tag,
            info.human_schedule,
            first_nonempty_line(&info.prompt)
        ));
    }

    if rows.is_empty() {
        t("tasks.empty").to_string()
    } else {
        let header = if rows.len() == 1 {
            t_fmt("tasks.header_singular", &[("count", &rows.len().to_string())])
        } else {
            t_fmt("tasks.header_plural", &[("count", &rows.len().to_string())])
        };
        join_header_rows(header, rows)
    }
}

/// `/usage` body — per-session token and cost totals, scoped to the ledger's
/// lifetime: since session start, or since the last `/resume`.
pub(crate) fn session_usage_block_text(
    usage: &xai_grok_shell::extensions::notification::PromptUsage,
) -> String {
    let totals = &usage.totals;
    if totals.model_calls == 0 && usage.model_usage.is_empty() {
        return if usage.usage_is_incomplete {
            t("tasks.usage_none_incomplete").to_string()
        } else {
            t("tasks.usage_none").to_string()
        };
    }

    let mut rows = Vec::new();
    rows.push(t_fmt("tasks.input_tokens", &[
        ("count", &group_thousands(totals.input_tokens)),
        ("cached", &group_thousands(totals.cached_read_tokens)),
    ]));
    rows.push(t_fmt("tasks.output_tokens", &[
        ("count", &group_thousands(totals.output_tokens)),
        ("reasoning", &group_thousands(totals.reasoning_tokens)),
    ]));
    rows.push(t_fmt("tasks.total_tokens", &[
        ("count", &group_thousands(totals.total_tokens)),
    ]));
    rows.push(t_fmt("tasks.model_calls", &[
        ("count", &group_thousands(totals.model_calls)),
        ("time", &format_duration(std::time::Duration::from_millis(totals.api_duration_ms))),
    ]));
    rows.push(t_fmt("tasks.cost", &[("cost", &format_cost(totals))]));

    if usage.model_usage.len() > 1 {
        rows.push(t("tasks.by_model").to_string());
        for (model, m) in &usage.model_usage {
            rows.push(t_fmt("tasks.model_breakdown", &[
                ("model", model.as_str()),
                ("input", &group_thousands(m.input_tokens)),
                ("output", &group_thousands(m.output_tokens)),
                ("cost", &format_cost(m)),
            ]));
        }
    }

    if usage.usage_is_incomplete {
        rows.push(t("tasks.usage_incomplete_note").to_string());
    }

    join_header_rows(
        t("tasks.usage_header").to_string(),
        rows,
    )
}

/// Cost cell. Ticks are 1e10 per USD; partial sums are scrubbed to absent.
fn format_cost(m: &xai_grok_shell::extensions::notification::PromptUsageModel) -> String {
    use xai_grok_shell::extensions::notification::ticks_to_usd;
    match m.cost_usd_ticks {
        Some(ticks) => format!("${:.4}", ticks_to_usd(ticks)),
        None if m.cost_is_partial => t("tasks.cost_not_available_partial").to_string(),
        None => t("tasks.cost_not_available").to_string(),
    }
}

/// First non-empty, trimmed line of `text` (empty string if none). Collapses a
/// multi-line prompt/command to a single display line.
fn first_nonempty_line(text: &str) -> &str {
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
        let suffix = if extra == 1 {
            t_fmt("tasks.more_lines_singular", &[("extra", &extra.to_string())])
        } else {
            t_fmt("tasks.more_lines_plural", &[("extra", &extra.to_string())])
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
        assert!(text.contains("grok-build — 100 in / 10 out"), "{text}");
        assert!(text.contains("grok-4 — 50 in / 5 out"), "{text}");
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
