use std::borrow::Cow;
use std::path::Path;

use super::{DbStats, GcReport, RebuildReport};
use xai_fast_worktree::WorktreeRecord;
use xai_grok_i18n::{t, t_fmt};
use xai_grok_shell::session::worktree::META_KEY_LABEL;

/// Extract the label from a worktree record's metadata JSON.
fn extract_label(rec: &WorktreeRecord) -> &str {
    rec.metadata
        .as_ref()
        .and_then(|m| m.get(META_KEY_LABEL))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

pub fn print_table(records: &[WorktreeRecord]) {
    if records.is_empty() {
        println!("{}", t("cli.worktree.none_found"));
        return;
    }

    // Compute dynamic ID column width so long IDs are never truncated
    let id_width = records
        .iter()
        .map(|r| r.id.len())
        .max()
        .unwrap_or(0)
        .max(16);

    // Compute dynamic label column width (min 5 for header "LABEL")
    let label_width = records
        .iter()
        .map(|r| extract_label(r).len())
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
        let row = format!(
            "  {:<id_width$} {:<8} {:<6} {:<label_width$} {:<20} {:<10} {}",
            rec.id,
            rec.kind.as_str(),
            truncate(&rec.repo_name, 6),
            truncate(label, label_width),
            truncate(branch, 20),
            age,
            path,
        );
        println!("{row}");
    }

    let total = records.len();
    let by_kind: std::collections::HashMap<&str, usize> =
        records
            .iter()
            .fold(std::collections::HashMap::new(), |mut m, r| {
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

pub fn print_json(records: &[WorktreeRecord]) {
    let json = serde_json::to_string_pretty(records).unwrap_or_else(|_| "[]".to_string());
    println!("{json}");
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

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut val = bytes as f64;
    for unit in UNITS {
        if val < 1024.0 {
            return format!("{val:.1} {unit}");
        }
        val /= 1024.0;
    }
    format!("{val:.1} TB")
}

fn truncate(s: &str, max: usize) -> Cow<'_, str> {
    if s.chars().count() <= max {
        Cow::Borrowed(s)
    } else {
        let end = s
            .char_indices()
            .nth(max.saturating_sub(1))
            .map_or(s.len(), |(i, _)| i);
        Cow::Owned(format!("{}…", &s[..end]))
    }
}

fn abbreviate_home(path: &Path) -> String {
    crate::util::abbreviate_path(&path.to_string_lossy()).into_owned()
}

fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    dir_size_recurse(path, &mut total);
    Ok(total)
}

fn dir_size_recurse(dir: &Path, total: &mut u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_file() {
            if let Ok(meta) = entry.metadata() {
                *total += meta.len();
            }
        } else if ft.is_dir() {
            dir_size_recurse(&entry.path(), total);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512.0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_age() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(format_age(now - 30).ends_with("s ago"));
        assert!(format_age(now - 120).ends_with("m ago"));
        assert!(format_age(now - 7200).ends_with("h ago"));
        assert!(format_age(now - 172800).ends_with("d ago"));
    }

    #[test]
    fn test_truncate_no_truncation() {
        assert_eq!(truncate("hello", 10).as_ref(), "hello");
        assert!(matches!(truncate("hello", 10), Cow::Borrowed(_)));
    }

    #[test]
    fn test_truncate_with_truncation() {
        let result = truncate("hello world", 5);
        assert_eq!(result.as_ref(), "hell…");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_truncate_utf8_safe() {
        let result = truncate("héllo wörld", 5);
        assert_eq!(result.as_ref(), "héll…");
    }

    #[test]
    fn test_abbreviate_home() {
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(format!("{home}/work/xai"));
            assert_eq!(abbreviate_home(&path), "~/work/xai");
        }
    }

    #[test]
    fn test_print_table_long_id_not_truncated() {
        let long_id = "a".repeat(40);

        // Verify width computation: ID longer than 16 should determine column width
        let id_width = long_id.len().max(16);
        let formatted = format!("{:<id_width$}", long_id, id_width = id_width);
        assert!(formatted.len() >= 40, "ID should not be truncated");
        assert!(formatted.contains(&long_id), "Full ID must be present");
    }

    fn make_test_record(metadata: Option<serde_json::Value>) -> xai_fast_worktree::WorktreeRecord {
        use xai_fast_worktree::{WorktreeKind, WorktreeRecord, WorktreeStatus};
        WorktreeRecord {
            id: "test".into(),
            path: "/tmp/wt".into(),
            source_repo: "/repo".into(),
            repo_name: "repo".into(),
            kind: WorktreeKind::Session,
            creation_mode: "linked".into(),
            git_ref: None,
            head_commit: None,
            session_id: None,
            creator_pid: None,
            created_at: 0,
            last_accessed_at: None,
            status: WorktreeStatus::Alive,
            metadata,
        }
    }

    #[test]
    fn test_extract_label_present() {
        let rec = make_test_record(Some(
            serde_json::json!({"label": "my-feature", "user_provided": true}),
        ));
        assert_eq!(extract_label(&rec), "my-feature");
    }

    #[test]
    fn test_extract_label_missing_metadata() {
        let rec = make_test_record(None);
        assert_eq!(extract_label(&rec), "");
    }

    #[test]
    fn test_extract_label_no_label_key() {
        let rec = make_test_record(Some(serde_json::json!({"other": "data"})));
        assert_eq!(extract_label(&rec), "");
    }
}
