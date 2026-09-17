use super::*;
use crate::scrollback::types::line_plain_text;

fn context(mode: DisplayMode) -> BlockContext {
    BlockContext {
        width: 80,
        mode,
        is_running: false,
        raw: false,
        max_lines: None,
        appearance: Default::default(),
        is_selected: false,
        cwd: None,
    }
}

fn rendered(block: &OtherToolCallBlock, mode: DisplayMode) -> String {
    block
        .output(&context(mode))
        .lines
        .iter()
        .map(|line| line_plain_text(&line.content))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn failed_call_expands_to_its_error() {
    let block = OtherToolCallBlock::new("computer_use__computer_apps", "")
        .with_error("the helper did not answer within 15s");
    assert!(block.is_foldable(), "a failure has a body to show");
    let expanded = rendered(&block, DisplayMode::Expanded);
    assert!(
        expanded.contains("the helper did not answer within 15s"),
        "{expanded}"
    );
    assert!(!rendered(&block, DisplayMode::Collapsed).contains("did not answer"));
}

#[test]
fn success_without_output_stays_unfoldable() {
    assert!(!OtherToolCallBlock::new("tool", "done").is_foldable());
}

/// The empty-answer marker in an AskUserQuestion body is catalog-backed.
#[test]
fn empty_answer_copy_comes_from_the_catalog() {
    let block = OtherToolCallBlock::new("AskUserQuestion", "")
        .with_output("Questions asked\n- \"Which color?\"\n  (No answer provided)");
    let text = xai_grok_i18n::with_pseudo_locale(|| rendered(&block, DisplayMode::Expanded));
    assert!(
        text.contains("⟦tool.question.no_answer⟧"),
        "empty-answer marker: {text}"
    );
}
