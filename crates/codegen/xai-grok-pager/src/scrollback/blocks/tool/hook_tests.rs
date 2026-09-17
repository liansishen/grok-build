use std::time::Duration;

use ratatui::text::Span;

use super::{
    HookRunEntry, HookRunStatus, ToolCallHookData, render_hooks_inline_suffix,
    render_stop_hooks_summary,
};

fn run(status: HookRunStatus) -> HookRunEntry {
    HookRunEntry {
        name: "hook".to_owned(),
        status,
        output: None,
    }
}

fn text(spans: Vec<Span<'static>>) -> String {
    let mut text = String::new();
    for span in spans {
        text.push_str(span.content.as_ref());
    }
    text
}

#[test]
fn compact_suffix_keeps_blocked_and_failure_formatting() {
    let elapsed = Duration::from_millis(1);
    let data = ToolCallHookData {
        post_hooks: vec![
            run(HookRunStatus::Blocked {
                detail: "denied".to_owned(),
                elapsed,
            }),
            run(HookRunStatus::Failed {
                error: "exit 1".to_owned(),
                elapsed,
            }),
        ],
        ..ToolCallHookData::default()
    };
    assert_eq!(
        text(render_hooks_inline_suffix(&data).expect("hook suffix")),
        "  [hooks: 1/1]"
    );
    let stop_groups = [("stop".to_owned(), data.post_hooks)];
    assert_eq!(
        text(render_stop_hooks_summary(&stop_groups).expect("stop suffix")),
        "stop  [hooks: 1/1]"
    );
}

/// The elapsed-time suffix on a blocked hook run is catalog-backed.
#[test]
fn blocked_hook_elapsed_copy_comes_from_the_catalog() {
    let runs = vec![run(HookRunStatus::Blocked {
        detail: "denied".to_owned(),
        elapsed: Duration::from_millis(250),
    })];
    let rendered = xai_grok_i18n::with_pseudo_locale(|| {
        super::render_hooks_detail(&runs, crate::scrollback::types::DisplayMode::Expanded)
            .iter()
            .map(|line| {
                line.content
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    });
    assert!(
        rendered.contains("⟦tool.hooks.elapsed_ms⟧"),
        "blocked hook elapsed suffix: {rendered}"
    );
}
