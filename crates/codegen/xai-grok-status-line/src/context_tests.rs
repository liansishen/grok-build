use serde_json::json;

use super::*;

const DIR: &str = "/home/user/project";

/// Returns the shared fixture: the JSON a command row receives on stdin, which the pager's guide test and the SDK suites also read.
/// A field renamed in the type fails this test, the guide test, and the SDK suites.
/// It carries `session_name`, which the client overlays for a command row; the agent's own notification leaves that field null.
fn wire_fixture() -> serde_json::Value {
    let mut fixture: serde_json::Value =
        serde_json::from_str(include_str!("../testdata/status_wire.json"))
            .expect("the shared fixture must be valid JSON");
    fixture
        .as_object_mut()
        .expect("the fixture is an object")
        .remove("_comment");
    fixture
}

#[test]
fn every_field_survives_a_round_trip_through_the_shared_fixture() {
    // Every field is set, rather than `..Default::default()`
    // A new field is a compile error here, and then a missing name in the fixture the SDKs read
    let ctx = StatusLineContext {
        schema_version: Some(STATUS_LINE_SCHEMA_VERSION),
        cwd: DIR.into(),
        session_id: Some("019fa651-6d59-7c83-a4f3-5a391e6901a1".into()),
        session_name: Some("add status line".into()),
        prompt_id: Some("97135ed2-71a5-4581-b959-3341bbd03e5f".into()),
        transcript_path: Some("/home/user/sessions/019fa651/updates.jsonl".into()),
        model: StatusLineModel {
            id: Some("grok-4.5".into()),
            display_name: Some("Grok 4.5".into()),
        },
        model_usage: Some(std::collections::BTreeMap::from([(
            "grok-4.5".into(),
            StatusLineModelUsage {
                input_tokens: 3000,
                output_tokens: 1000,
                reasoning_tokens: 100,
                total_tokens: 9500,
                cache_creation_input_tokens: 500,
                cache_read_input_tokens: 5000,
                model_calls: 2,
                api_duration_ms: 2300,
                cost_usd: Some(0.0123),
                cost_usd_ticks: Some(123_000_000),
                cost_is_partial: true,
            },
        )])),
        process_model_usage: Some(std::collections::BTreeMap::from([(
            "grok-4.5".into(),
            StatusLineModelUsage {
                input_tokens: 1500,
                output_tokens: 600,
                reasoning_tokens: 80,
                total_tokens: 2500,
                cache_creation_input_tokens: 100,
                cache_read_input_tokens: 300,
                model_calls: 1,
                api_duration_ms: 1200,
                cost_usd: Some(0.0045),
                cost_usd_ticks: Some(45_000_000),
                cost_is_partial: false,
            },
        )])),
        billing: Some(StatusLineBilling {
            usage_percentage: Some(42.5),
            effective_usage_percentage: Some(40.0),
            period_type: Some("USAGE_PERIOD_TYPE_MONTHLY".into()),
            period_end: Some("Mar 31, 12:00".into()),
            pay_as_you_go: Some(true),
            on_demand_cap_cents: Some(500),
            on_demand_used_cents: Some(125),
            prepaid_balance_cents: Some(2500),
        }),
        quota: Some(StatusLineQuota {
            model_id: "gpt-4.5".into(),
            accounts: vec![StatusLineQuotaAccount {
                email: "acct@example.com".into(),
                used_percentage: 24.0,
                remaining_percentage: 76.0,
                reset_at: 1_785_903_012,
                plan_type: Some("pro".into()),
            }],
        }),
        workspace: StatusLineWorkspace {
            current_dir: DIR.into(),
            repo_root: Some(DIR.into()),
            branch: Some("main".into()),
            git_worktree: Some("feature-x".into()),
            repo: Some(StatusLineRepo {
                host: "github.com".into(),
                owner: Some("owner".into()),
                name: "repo".into(),
            }),
        },
        version: "0.2.112".to_string(),
        cost: StatusLineCost {
            total_cost_usd: Some(0.0123),
            total_duration_ms: 45_000,
            total_api_duration_ms: Some(2_300),
        },
        context_window: StatusLineContextWindow {
            context_window_size: Some(500_000),
            context_tokens: Some(40_000),
            session_input_tokens: Some(52_000),
            session_output_tokens: Some(9_500),
            session_usage: Some(StatusLineSessionUsage {
                input_tokens: 10_000,
                output_tokens: 9_500,
                cache_creation_input_tokens: 2_000,
                cache_read_input_tokens: 40_000,
            }),
            used_percentage: Some(8),
            remaining_percentage: Some(92),
            auto_compact_threshold_percent: Some(80),
        },
        effort: Some(StatusLineEffort {
            level: "high".into(),
        }),
        turn: Some(StatusLineTurn {
            started_at_ms: 1_730_000_000_000,
        }),
        worktree: Some(StatusLineWorktree {
            name: Some("feature-x".into()),
            path: "/home/user/wt/feature-x".into(),
            branch: Some("feature-x".into()),
            main_worktree_root: Some(DIR.into()),
        }),
        trigger: Some(StatusLineTrigger::RefreshInterval),
    };

    assert_eq!(
        serde_json::to_value(&ctx).unwrap(),
        wire_fixture(),
        "the type and the fixture have drifted; update both SDK suites with it"
    );
    let parsed: StatusLineContext =
        serde_json::from_value(wire_fixture()).expect("the wire shape must parse back");
    assert_eq!(parsed, ctx, "a name the type writes but cannot read back");

    assert_eq!(
        serde_json::to_value(StatusLineTrigger::RefreshInterval).unwrap(),
        json!("refresh_interval")
    );
    assert_eq!(
        serde_json::to_value(StatusLineTrigger::State).unwrap(),
        json!("state")
    );
}

#[test]
fn unknown_data_is_omitted_rather_than_faked() {
    let mut bare = StatusLineContext::default();
    bare.workspace.repo = Some(StatusLineRepo {
        host: "example.com".into(),
        owner: None,
        name: "widget".into(),
    });

    assert_eq!(
        serde_json::to_value(&bare).unwrap(),
        json!({
            "schema_version": 1,
            "cwd": "", "version": "",
            "model": {},
            "workspace": {
                "current_dir": "",
                "repo": { "host": "example.com", "name": "widget" },
            },
            "cost": { "total_duration_ms": 0 },
            "context_window": {},
        }),
        "what Grok cannot source is omitted; a context window reported as 0 \
         would paint `0% ctx` over a full one"
    );
}

#[test]
fn payload_missing_newer_fields_still_deserializes() {
    let minimal: StatusLineContext = serde_json::from_str(r#"{"cwd":"/tmp"}"#).unwrap();
    assert_eq!(
        minimal.schema_version, None,
        "absent means the sender predates the field"
    );
    assert!(minimal.context_window.context_window_size.is_none());
}
