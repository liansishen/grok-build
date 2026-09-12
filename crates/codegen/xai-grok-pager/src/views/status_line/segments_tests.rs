use super::*;

const DIR: &str = "/home/user/project";

fn context() -> StatusLineContext {
    let mut ctx = crate::app::status_line::test_context(DIR);
    ctx.session_name = Some("status_line work".into());
    ctx.model.display_name = Some("Grok Build".into());
    ctx.cost.total_cost_usd = Some(0.3745);
    ctx.context_window.used_percentage = Some(42);
    ctx.context_window.auto_compact_threshold_percent = Some(80);
    ctx
}

fn plain(ctx: &StatusLineContext, turn_elapsed: Option<Duration>) -> String {
    compose_builtin(ctx, turn_elapsed, StatusLineItem::ALL)
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(SEGMENT_SEPARATOR)
}

#[test]
fn composes_every_segment_in_order() {
    assert_eq!(
        plain(&context(), Some(Duration::from_secs(83))),
        "project │ Grok Build │ 42% ctx │ $0.37 │ 1m23s │ status_line work"
    );
}

#[test]
fn generation_segments_show_ttft_and_live_speed() {
    let mut ctx = context();
    ctx.generation = Some(xai_grok_status_line::StatusLineGeneration {
        first_token_ms: Some(842),
        tokens_per_second: Some(132.4),
        estimated: true,
        stale: false,
    });

    assert_eq!(
        plain(&ctx, None),
        "project │ Grok Build │ 42% ctx │ $0.37 │ ttft 842ms │ 132 tok/s │ status_line work"
    );

    ctx.generation.as_mut().unwrap().stale = true;
    assert_eq!(
        plain(&ctx, None),
        "project │ Grok Build │ 42% ctx │ $0.37 │ ttft 842ms │ status_line work"
    );
}

#[test]
fn omits_segments_whose_data_is_missing_or_rounds_to_zero() {
    let mut ctx = context();
    ctx.cost.total_cost_usd = Some(0.004);
    assert_eq!(
        plain(&ctx, Some(Duration::from_millis(400))),
        "project │ Grok Build │ 42% ctx │ status_line work"
    );

    ctx.cost.total_cost_usd = Some(0.006);
    assert!(plain(&ctx, None).contains("$0.01"));

    ctx.context_window.used_percentage = None;
    assert!(compose_builtin(&ctx, None, &[StatusLineItem::Context]).is_empty());
}

#[test]
fn usage_billing_and_quota_segments_use_optional_context_data() {
    use xai_grok_status_line::{
        StatusLineBilling, StatusLineQuota, StatusLineQuotaAccount, StatusLineSessionUsage,
    };

    let mut ctx = context();
    ctx.context_window.session_usage = Some(StatusLineSessionUsage {
        input_tokens: 1_000,
        output_tokens: 500,
        cache_creation_input_tokens: 250,
        cache_read_input_tokens: 250,
    });
    ctx.billing = Some(StatusLineBilling {
        usage_percentage: Some(42.5),
        period_type: Some("USAGE_PERIOD_TYPE_WEEKLY".into()),
        ..Default::default()
    });
    ctx.quota = Some(StatusLineQuota {
        model_id: "grok-4.5".into(),
        accounts: vec![StatusLineQuotaAccount {
            email: "user@example.com".into(),
            used_percentage: 24.0,
            remaining_percentage: 76.0,
            ..Default::default()
        }],
    });

    let text = compose_builtin(
        &ctx,
        None,
        &[
            StatusLineItem::Usage,
            StatusLineItem::Billing,
            StatusLineItem::Quota,
        ],
    );
    let text = text
        .iter()
        .map(|segment| segment.text())
        .collect::<Vec<_>>()
        .join(" │ ");
    assert!(text.contains("2.0k tok"), "usage segment: {text}");
    assert!(text.contains("wk 42%"), "billing segment: {text}");
    assert!(text.contains("quota 76%"), "quota segment: {text}");
}

#[test]
fn name_past_its_budget_is_cut_by_painted_columns() {
    let mut ctx = context();
    ctx.session_name = Some("辺".repeat(SESSION_NAME_COLS));
    let cut = &compose_builtin(&ctx, None, &[StatusLineItem::SessionName])[0].text;

    let width = super::super::painted_width(cut);
    assert!(
        cut.ends_with('…'),
        "a cut name has to say it was cut: {cut}"
    );
    assert!(
        width <= SESSION_NAME_COLS,
        "{width} columns overruns the {SESSION_NAME_COLS} the segment was given"
    );
    // The cut fills the budget to within one cluster; a byte or character cut would leave more unused
    assert!(
        width + 2 > SESSION_NAME_COLS,
        "{width} columns leaves more than a cluster of the budget unused"
    );
}

#[test]
fn cost_the_session_does_not_have_omits_its_segment() {
    let mut ctx = context();
    ctx.cost.total_cost_usd = None;
    assert!(compose_builtin(&ctx, None, &[StatusLineItem::Cost]).is_empty());
}

#[test]
fn context_segment_warns_near_compaction() {
    let mut ctx = context();
    let tone =
        |ctx: &StatusLineContext| compose_builtin(ctx, None, &[StatusLineItem::Context])[0].tone;

    ctx.context_window.used_percentage = Some(90);
    assert_eq!(tone(&ctx), SegmentTone::Warn);
    ctx.context_window.used_percentage = Some(50);
    assert_eq!(tone(&ctx), SegmentTone::Dim);

    ctx.context_window.auto_compact_threshold_percent = Some(65);
    ctx.context_window.used_percentage = Some(70);
    assert_eq!(tone(&ctx), SegmentTone::Warn);
}
