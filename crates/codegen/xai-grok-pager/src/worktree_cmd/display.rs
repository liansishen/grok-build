use std::io::Write;
use std::path::Path;

use unicode_width::UnicodeWidthStr;
use xai_fast_worktree::WorktreeRecord;
use xai_grok_i18n::{t, t_fmt};

use super::{DbStats, GcReport, RebuildReport};
use crate::fs_size::{Volume, physical_dir_size};
use crate::util::{format_bytes, pad_to_width, truncate_to_width, unix_now};

const REPO_WIDTH: usize = 6;
const BRANCH_WIDTH: usize = 20;
const AGE_WIDTH: usize = 10;

/// Truncate-then-pad to exactly `width` display columns; headers and data
/// share it so the two stay aligned.
fn cell(s: &str, width: usize) -> String {
    pad_to_width(&truncate_to_width(s, width), width)
}

pub fn print_table(records: &[WorktreeRecord], out: &mut impl Write) -> std::io::Result<()> {
    if records.is_empty() {
        writeln!(out, "{}", t("cli.worktree.none_found"))?;
        return Ok(());
    }

    let id_width = records
        .iter()
        .map(|r| UnicodeWidthStr::width(r.id.as_str()))
        .max()
        .unwrap_or(0)
        .max(16);

    let label_width = records
        .iter()
        .map(|r| r.label().map_or(0, UnicodeWidthStr::width))
        .max()
        .unwrap_or(0)
        .clamp(5, 24);

    let type_header = t("cli.worktree.header.type");
    // Derived, not fixed: `cell` truncates rather than shifting, and
    // `subagent` already fills 8 columns.
    let type_width = records
        .iter()
        .map(|r| UnicodeWidthStr::width(r.kind.as_str()))
        .fold(UnicodeWidthStr::width(type_header), usize::max);

    writeln!(
        out,
        "  {} {} {} {} {} {:<AGE_WIDTH$} {}",
        pad_to_width(t("cli.worktree.header.id"), id_width),
        cell(type_header, type_width),
        cell(t("cli.worktree.header.repo"), REPO_WIDTH),
        cell(t("cli.worktree.header.label"), label_width),
        cell(t("cli.worktree.header.branch"), BRANCH_WIDTH),
        t("cli.worktree.header.age"),
        t("cli.worktree.header.path"),
    )?;
    let now = unix_now();
    for rec in records {
        let age = format_age_i18n(rec.created_at, now);
        let branch = rec
            .git_ref
            .as_deref()
            .unwrap_or_else(|| t("cli.worktree.detached"));
        let label = rec.label().unwrap_or("");
        let path = abbreviate_home(&rec.path);
        // AGE is ASCII, so format-width padding is width-true; every other
        // cell pads by display width.
        writeln!(
            out,
            "  {} {} {} {} {} {:<AGE_WIDTH$} {}",
            pad_to_width(&rec.id, id_width),
            cell(rec.kind.as_str(), type_width),
            cell(&rec.repo_name, REPO_WIDTH),
            cell(label, label_width),
            cell(branch, BRANCH_WIDTH),
            age,
            path,
        )?;
    }

    let total = records.len();
    let by_kind: std::collections::BTreeMap<&str, usize> =
        records
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut m, r| {
                *m.entry(r.kind.as_str()).or_default() += 1;
                m
            });
    let breakdown: Vec<String> = by_kind.iter().map(|(k, v)| format!("{v} {k}")).collect();
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.total",
            &[
                ("count", total.to_string().as_str()),
                ("breakdown", breakdown.join(", ").as_str()),
            ],
        )
    )
}

pub fn print_json(records: &[WorktreeRecord], out: &mut impl Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(records).unwrap_or_else(|_| "[]".to_string());
    writeln!(out, "{json}")
}

pub fn print_show(rec: &WorktreeRecord, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.path",
            &[("value", rec.path.to_string_lossy().as_ref())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt("cli.worktree.detail.id", &[("value", rec.id.as_str())])
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.type",
            &[("value", rec.kind.as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.source_repo",
            &[("value", rec.source_repo.to_string_lossy().as_ref())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.creation_mode",
            &[("value", rec.creation_mode.as_str())]
        )
    )?;
    if let Some(ref git_ref) = rec.git_ref {
        writeln!(
            out,
            "{}",
            t_fmt("cli.worktree.detail.git_ref", &[("value", git_ref)])
        )?;
    }
    if let Some(ref commit) = rec.head_commit {
        let short = if commit.len() > 12 {
            &commit[..12]
        } else {
            commit
        };
        writeln!(
            out,
            "{}",
            t_fmt("cli.worktree.detail.head", &[("value", short)])
        )?;
    }
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.created",
            &[("value", format_timestamp(rec.created_at).as_str())]
        )
    )?;
    if let Some(ts) = rec.last_accessed_at {
        writeln!(
            out,
            "{}",
            t_fmt(
                "cli.worktree.detail.last_accessed",
                &[("value", format_timestamp(ts).as_str())]
            )
        )?;
    }
    if let Some(ref sid) = rec.session_id {
        writeln!(
            out,
            "{}",
            t_fmt("cli.worktree.detail.session_id", &[("value", sid)])
        )?;
    }
    if let Some(pid) = rec.creator_pid {
        let pid_s = pid.to_string();
        writeln!(
            out,
            "{}",
            t_fmt(
                "cli.worktree.detail.creator_pid",
                &[("value", pid_s.as_str())]
            )
        )?;
    }
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.detail.status",
            &[("value", rec.status.as_str())]
        )
    )?;
    if let Some(label) = rec.label() {
        writeln!(
            out,
            "{}",
            t_fmt("cli.worktree.detail.label", &[("value", label)])
        )?;
    }

    if rec.path.exists() {
        // Anchored to the worktree's own volume: one tree, not a share of
        // some other total.
        let size = physical_dir_size(&rec.path, Volume::of(&rec.path));
        let bytes = size.measure.bytes().unwrap_or_default();
        let mut value = format_bytes(bytes);
        let skipped = size.issues.skipped();
        if skipped > 0 {
            let key = if skipped == 1 {
                "cli.worktree.detail.disk_skipped_one"
            } else {
                "cli.worktree.detail.disk_skipped_many"
            };
            value = format!(
                "{value} {}",
                t_fmt(key, &[("count", skipped.to_string().as_str())])
            );
        }
        writeln!(
            out,
            "{}",
            t_fmt(
                "cli.worktree.detail.disk_usage",
                &[("value", value.as_str())]
            )
        )?;
    }
    Ok(())
}

pub fn print_stats(stats: &DbStats, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(out, "{}", t("cli.worktree.stats.title"))?;
    writeln!(out, "{}", t("cli.worktree.stats.divider"))?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.stats.total_records",
            &[("value", stats.total_records.to_string().as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.stats.alive",
            &[("value", stats.alive_count.to_string().as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.stats.dead",
            &[("value", stats.dead_count.to_string().as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.stats.db_size",
            &[("value", format_bytes(stats.db_file_bytes).as_str())]
        )
    )
}

/// Used by `mod` without a `Write` handle (stdout only).
pub fn print_gc(report: &GcReport) {
    println!("{}", t("cli.worktree.gc.title"));
    println!(
        "{}",
        t_fmt(
            "cli.worktree.gc.dead_removed",
            &[("value", report.dead_removed.to_string().as_str())]
        )
    );
    println!(
        "{}",
        t_fmt(
            "cli.worktree.gc.expired_removed",
            &[("value", report.expired_removed.to_string().as_str())]
        )
    );
    println!(
        "{}",
        t_fmt(
            "cli.worktree.gc.skipped_alive",
            &[("value", report.skipped_alive.to_string().as_str())]
        )
    );
    if report.remove_failed > 0 {
        println!(
            "{}",
            t_fmt(
                "cli.worktree.gc.remove_failed",
                &[("value", report.remove_failed.to_string().as_str())]
            )
        );
    }
}

pub fn print_rebuild(report: &RebuildReport, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(out, "{}", t("cli.worktree.rebuild.title"))?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.rebuild.discovered",
            &[("value", report.discovered.to_string().as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.rebuild.registered",
            &[("value", report.registered.to_string().as_str())]
        )
    )?;
    writeln!(
        out,
        "{}",
        t_fmt(
            "cli.worktree.rebuild.already_tracked",
            &[("value", report.already_tracked.to_string().as_str())]
        )
    )
}

fn format_age_i18n(created_at: i64, now: i64) -> String {
    let delta = now.saturating_sub(created_at).max(0);
    let (key, value) = if delta < 60 {
        ("cli.worktree.age.seconds", delta)
    } else if delta < 3600 {
        ("cli.worktree.age.minutes", delta / 60)
    } else if delta < 86400 {
        ("cli.worktree.age.hours", delta / 3600)
    } else {
        ("cli.worktree.age.days", delta / 86400)
    };
    t_fmt(key, &[("count", value.to_string().as_str())])
}

fn format_timestamp(ts: i64) -> String {
    let dt = chrono::DateTime::from_timestamp(ts, 0);
    match dt {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => ts.to_string(),
    }
}

fn abbreviate_home(path: &Path) -> String {
    crate::util::abbreviate_path(&path.to_string_lossy()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_table_never_truncates_long_ids() {
        let long_id = "a".repeat(40);
        let mut out = Vec::new();
        print_table(&[make_record(&long_id, "lbl")], &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(&long_id), "full ID must be present: {text}");
    }

    fn make_record(id: &str, label: &str) -> WorktreeRecord {
        crate::test_util::make_worktree_record(
            id,
            std::path::Path::new(&format!("/tmp/wt-{id}")),
            label,
        )
    }

    #[test]
    fn print_table_pads_cjk_labels_by_display_width() {
        let records = vec![
            make_record("wt-cjk", "组件更新"),
            make_record("wt-ascii", "plain-label"),
        ];
        let mut out = Vec::new();
        print_table(&records, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("组件更新"));
        crate::test_util::assert_path_column_aligned(&text, "/tmp/wt-");
    }
}
