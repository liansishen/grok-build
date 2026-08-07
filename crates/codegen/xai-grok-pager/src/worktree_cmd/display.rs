use std::io::Write;
use std::path::Path;

use unicode_width::UnicodeWidthStr;
use xai_fast_worktree::WorktreeRecord;
use xai_grok_i18n::{t, t_fmt};
use xai_grok_shell::session::worktree::META_KEY_LABEL;

use super::{DbStats, GcReport, RebuildReport};
use crate::fs_size::{Volume, physical_dir_size};
use crate::util::{format_age, format_bytes, pad_to_width, truncate_to_width, unix_now};

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
        println!("{}", t("cli.worktree.none_found"));
        return;
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

    let header = format!(
        "  {:<id_width$} {:<8} {:<6} {:<label_width$} {:<20} {:<10} {}",
        t("cli.worktree.header.id"),
        t("cli.worktree.header.type"),
        t("cli.worktree.header.repo"),
        t("cli.worktree.header.label"),
        t("cli.worktree.header.branch"),
        t("cli.worktree.header.age"),
        t("cli.worktree.header.path"),
    );
    println!("{header}");
    for rec in records {
        let age = format_age(rec.created_at);
        let branch = rec
            .git_ref
            .as_deref()
            .unwrap_or_else(|| t("cli.worktree.detached"));
        let label = extract_label(rec);
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
    println!(
        "{}",
        t_fmt(
            "cli.worktree.total",
            &[
                ("count", total.to_string().as_str()),
                ("breakdown", breakdown.join(", ").as_str()),
            ],
        )
    );
}

pub fn print_json(records: &[WorktreeRecord], out: &mut impl Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(records).unwrap_or_else(|_| "[]".to_string());
    writeln!(out, "{json}")
}

pub fn print_show(rec: &WorktreeRecord) {
    print_detail(
        "cli.worktree.detail.path",
        rec.path.to_string_lossy().as_ref(),
    );
    print_detail("cli.worktree.detail.id", rec.id.as_str());
    print_detail("cli.worktree.detail.type", rec.kind.as_str());
    print_detail(
        "cli.worktree.detail.source_repo",
        rec.source_repo.to_string_lossy().as_ref(),
    );
    print_detail(
        "cli.worktree.detail.creation_mode",
        rec.creation_mode.as_str(),
    );
    if let Some(ref git_ref) = rec.git_ref {
        print_detail("cli.worktree.detail.git_ref", git_ref);
    }
    if let Some(ref commit) = rec.head_commit {
        let short = if commit.len() > 12 {
            &commit[..12]
        } else {
            commit
        };
        print_detail("cli.worktree.detail.head", short);
    }
    print_detail(
        "cli.worktree.detail.created",
        format_timestamp(rec.created_at).as_str(),
    );
    if let Some(ts) = rec.last_accessed_at {
        print_detail(
            "cli.worktree.detail.last_accessed",
            format_timestamp(ts).as_str(),
        );
    }
    if let Some(ref sid) = rec.session_id {
        print_detail("cli.worktree.detail.session_id", sid);
    }
    if let Some(pid) = rec.creator_pid {
        print_detail("cli.worktree.detail.creator_pid", pid.to_string().as_str());
    }
    print_detail("cli.worktree.detail.status", rec.status.as_str());
    let label = extract_label(rec);
    if !label.is_empty() {
        print_detail("cli.worktree.detail.label", label);
    }

    if rec.path.exists()
        && let Ok(size) = dir_size(&rec.path)
    {
        print_detail(
            "cli.worktree.detail.disk_usage",
            format_bytes(size).as_str(),
        );
    }
    Ok(())
}

fn print_detail(key: &str, value: &str) {
    println!("{}", t_fmt(key, &[("value", value)]));
}

pub fn print_stats(stats: &DbStats) {
    println!("{}", t("cli.worktree.stats.title"));
    println!("{}", t("cli.worktree.stats.divider"));
    print_detail(
        "cli.worktree.stats.total_records",
        stats.total_records.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.stats.alive",
        stats.alive_count.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.stats.dead",
        stats.dead_count.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.stats.db_size",
        format_bytes(stats.db_file_bytes).as_str(),
    );
}

pub fn print_gc(report: &GcReport) {
    println!("{}", t("cli.worktree.gc.title"));
    print_detail(
        "cli.worktree.gc.dead_removed",
        report.dead_removed.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.gc.expired_removed",
        report.expired_removed.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.gc.skipped_alive",
        report.skipped_alive.to_string().as_str(),
    );
    if report.remove_failed > 0 {
        print_detail(
            "cli.worktree.gc.remove_failed",
            report.remove_failed.to_string().as_str(),
        );
    }
    Ok(())
}

pub fn print_rebuild(report: &RebuildReport) {
    println!("{}", t("cli.worktree.rebuild.title"));
    print_detail(
        "cli.worktree.rebuild.discovered",
        report.discovered.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.rebuild.registered",
        report.registered.to_string().as_str(),
    );
    print_detail(
        "cli.worktree.rebuild.already_tracked",
        report.already_tracked.to_string().as_str(),
    );
}

fn format_age(created_at: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let delta = now.saturating_sub(created_at);
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
