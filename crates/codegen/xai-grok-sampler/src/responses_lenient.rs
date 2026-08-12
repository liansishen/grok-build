//! Lenient Responses API JSON sanitization.
//!
//! Proxies and alternate providers often emit Responses-shaped SSE that is
//! *almost* OpenAI-compatible: an extra vendor field, a missing
//! `sequence_number`, a non-string `metadata` value, or an unknown `output`
//! item type. `async_openai`'s typed `Response` / `ResponseStreamEvent`
//! deserializers reject those frames hard, which aborts the whole turn.
//!
//! This module rewrites the JSON **before** typed deserialize so common
//! drift is absorbed:
//! - missing scalar defaults are filled
//! - tools / output items that still cannot parse are dropped
//! - string-only maps coerce non-string values
//! - unknown stream event `type`s are reported as skippable

use serde_json::{Map, Value};
use xai_grok_sampling_types::rs;

/// Known `type` values for [`rs::ResponseStreamEvent`].
///
/// Keep in sync with `async_openai` `ResponseStreamEvent` renames. Unknown
/// types are skipped (not hard errors) so servers can add events without
/// breaking the CLI.
const KNOWN_STREAM_EVENT_TYPES: &[&str] = &[
    "response.created",
    "response.in_progress",
    "response.completed",
    "response.failed",
    "response.incomplete",
    "response.output_item.added",
    "response.output_item.done",
    "response.content_part.added",
    "response.content_part.done",
    "response.output_text.delta",
    "response.output_text.done",
    "response.refusal.delta",
    "response.refusal.done",
    "response.function_call_arguments.delta",
    "response.function_call_arguments.done",
    "response.file_search_call.in_progress",
    "response.file_search_call.searching",
    "response.file_search_call.completed",
    "response.web_search_call.in_progress",
    "response.web_search_call.searching",
    "response.web_search_call.completed",
    "response.reasoning_summary_part.added",
    "response.reasoning_summary_part.done",
    "response.reasoning_summary_text.delta",
    "response.reasoning_summary_text.done",
    "response.reasoning_text.delta",
    "response.reasoning_text.done",
    "response.image_generation_call.completed",
    "response.image_generation_call.generating",
    "response.image_generation_call.in_progress",
    "response.image_generation_call.partial_image",
    "response.mcp_call_arguments.delta",
    "response.mcp_call_arguments.done",
    "response.mcp_call.completed",
    "response.mcp_call.failed",
    "response.mcp_call.in_progress",
    "response.mcp_list_tools.completed",
    "response.mcp_list_tools.failed",
    "response.mcp_list_tools.in_progress",
    "response.code_interpreter_call.in_progress",
    "response.code_interpreter_call.interpreting",
    "response.code_interpreter_call.completed",
    "response.code_interpreter_call_code.delta",
    "response.code_interpreter_call_code.done",
    "response.output_text.annotation.added",
    "response.queued",
    "response.custom_tool_call_input.delta",
    "response.custom_tool_call_input.done",
    "error",
];

/// Outcome of a lenient stream-event parse.
#[derive(Debug)]
pub(crate) enum LenientStreamEvent {
    /// Typed event ready for the Responses stream transform.
    Event(rs::ResponseStreamEvent),
    /// Unknown or unsalvageable frame; caller should skip without failing.
    Skip { reason: String },
}

fn is_known_stream_event_type(ty: &str) -> bool {
    KNOWN_STREAM_EVENT_TYPES.contains(&ty)
}

fn ensure_u64(obj: &mut Map<String, Value>, key: &str, default: u64) {
    match obj.get(key) {
        Some(Value::Number(n)) if n.as_u64().is_some() || n.as_i64().is_some_and(|i| i >= 0) => {}
        Some(Value::String(s)) => {
            if let Ok(n) = s.parse::<u64>() {
                obj.insert(key.to_owned(), Value::Number(n.into()));
            } else {
                obj.insert(key.to_owned(), Value::Number(default.into()));
            }
        }
        Some(Value::Null) | None => {
            obj.insert(key.to_owned(), Value::Number(default.into()));
        }
        Some(_) => {
            obj.insert(key.to_owned(), Value::Number(default.into()));
        }
    }
}

fn ensure_u32(obj: &mut Map<String, Value>, key: &str, default: u32) {
    match obj.get(key) {
        Some(Value::Number(n)) if n.as_u64().is_some() || n.as_i64().is_some_and(|i| i >= 0) => {}
        Some(Value::String(s)) => {
            if let Ok(n) = s.parse::<u32>() {
                obj.insert(key.to_owned(), Value::Number(n.into()));
            } else {
                obj.insert(key.to_owned(), Value::Number(default.into()));
            }
        }
        Some(Value::Null) | None => {
            obj.insert(key.to_owned(), Value::Number(default.into()));
        }
        Some(_) => {
            obj.insert(key.to_owned(), Value::Number(default.into()));
        }
    }
}

fn ensure_string(obj: &mut Map<String, Value>, key: &str, default: &str) {
    match obj.get(key) {
        Some(Value::String(_)) => {}
        Some(Value::Number(n)) => {
            obj.insert(key.to_owned(), Value::String(n.to_string()));
        }
        Some(Value::Bool(b)) => {
            obj.insert(key.to_owned(), Value::String(b.to_string()));
        }
        Some(Value::Null) | None => {
            obj.insert(key.to_owned(), Value::String(default.to_owned()));
        }
        Some(other) => {
            // Last resort: JSON-encode non-scalars so required String fields
            // still deserialize rather than abort the stream.
            obj.insert(
                key.to_owned(),
                Value::String(other.to_string().trim_matches('"').to_owned()),
            );
        }
    }
}

fn ensure_array(obj: &mut Map<String, Value>, key: &str) {
    match obj.get(key) {
        Some(Value::Array(_)) => {}
        Some(Value::Null) | None => {
            obj.insert(key.to_owned(), Value::Array(Vec::new()));
        }
        Some(_) => {
            obj.insert(key.to_owned(), Value::Array(Vec::new()));
        }
    }
}

/// Coerce `metadata` to `HashMap<String, String>`-compatible shape.
fn coerce_string_map(obj: &mut Map<String, Value>, key: &str) {
    let Some(raw) = obj.get(key).cloned() else {
        return;
    };
    let Value::Object(map) = raw else {
        // Wrong type entirely — drop so Option path accepts absence.
        obj.remove(key);
        return;
    };
    let mut out = Map::new();
    for (k, v) in map {
        match v {
            Value::String(s) => {
                out.insert(k, Value::String(s));
            }
            Value::Number(n) => {
                out.insert(k, Value::String(n.to_string()));
            }
            Value::Bool(b) => {
                out.insert(k, Value::String(b.to_string()));
            }
            Value::Null => {}
            other => {
                out.insert(k, Value::String(other.to_string()));
            }
        }
    }
    obj.insert(key.to_owned(), Value::Object(out));
}

fn filter_tools_array(tools: &mut Vec<Value>) {
    tools.retain(|t| serde_json::from_value::<rs::Tool>(t.clone()).is_ok());
}

/// Best-effort fix for a single `output` item; returns false when it must drop.
fn sanitize_output_item(item: &mut Value) -> bool {
    if serde_json::from_value::<rs::OutputItem>(item.clone()).is_ok() {
        return true;
    }
    let Some(obj) = item.as_object_mut() else {
        return false;
    };
    let ty = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    match ty.as_str() {
        "message" => {
            ensure_string(obj, "id", "msg_unknown");
            ensure_string(obj, "role", "assistant");
            ensure_string(obj, "status", "completed");
            ensure_array(obj, "content");
            // Drop content parts that cannot parse.
            if let Some(Value::Array(parts)) = obj.get_mut("content") {
                parts.retain(|p| {
                    serde_json::from_value::<rs::OutputMessageContent>(p.clone()).is_ok()
                });
            }
        }
        "function_call" => {
            ensure_string(obj, "arguments", "{}");
            ensure_string(obj, "call_id", "call_unknown");
            ensure_string(obj, "name", "unknown");
        }
        "reasoning" => {
            ensure_string(obj, "id", "rs_unknown");
            ensure_array(obj, "summary");
            if let Some(Value::Array(parts)) = obj.get_mut("summary") {
                parts.retain(|p| serde_json::from_value::<rs::SummaryPart>(p.clone()).is_ok());
            }
        }
        // Unknown / exotic types: drop rather than fail the whole frame.
        "" => return false,
        _ => {
            // Try one more time after coercing common optional string ids.
            if obj.contains_key("id") {
                ensure_string(obj, "id", "item_unknown");
            }
        }
    }
    serde_json::from_value::<rs::OutputItem>(item.clone()).is_ok()
}

fn sanitize_usage_object(usage: &mut Value) {
    let Some(obj) = usage.as_object_mut() else {
        return;
    };
    ensure_u32(obj, "input_tokens", 0);
    ensure_u32(obj, "output_tokens", 0);
    ensure_u32(obj, "total_tokens", 0);
    match obj.get_mut("input_tokens_details") {
        Some(Value::Object(d)) => {
            ensure_u32(d, "cached_tokens", 0);
        }
        Some(_) | None => {
            obj.insert(
                "input_tokens_details".to_owned(),
                serde_json::json!({ "cached_tokens": 0 }),
            );
        }
    }
    match obj.get_mut("output_tokens_details") {
        Some(Value::Object(d)) => {
            ensure_u32(d, "reasoning_tokens", 0);
        }
        Some(_) | None => {
            obj.insert(
                "output_tokens_details".to_owned(),
                serde_json::json!({ "reasoning_tokens": 0 }),
            );
        }
    }
}

/// Sanitize a Responses API `response` object in place.
pub(crate) fn sanitize_response_object(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    ensure_u64(obj, "created_at", 0);
    ensure_string(obj, "id", "resp_unknown");
    ensure_string(obj, "model", "unknown");
    ensure_string(obj, "object", "response");
    ensure_array(obj, "output");
    coerce_string_map(obj, "metadata");

    if let Some(usage) = obj.get_mut("usage") {
        if usage.is_null() {
            obj.remove("usage");
        } else {
            sanitize_usage_object(usage);
        }
    }

    if let Some(Value::Array(tools)) = obj.get_mut("tools") {
        filter_tools_array(tools);
    }

    if let Some(Value::Array(output)) = obj.get_mut("output") {
        let mut kept = Vec::with_capacity(output.len());
        for mut item in output.drain(..) {
            if sanitize_output_item(&mut item) {
                kept.push(item);
            } else {
                tracing::debug!(
                    target: "xai_grok_sampler::responses_lenient",
                    item = %item,
                    "dropping unparseable Responses output item"
                );
            }
        }
        *output = kept;
    }
}

fn sanitize_stream_event_fields(obj: &mut Map<String, Value>, event_type: &str) {
    ensure_u64(obj, "sequence_number", 0);

    // Fill index defaults for event families that require them.
    let needs_output_index = event_type.contains("output_item")
        || event_type.contains("output_text")
        || event_type.contains("function_call")
        || event_type.contains("content_part")
        || event_type.contains("refusal")
        || event_type.contains("reasoning")
        || event_type.contains("file_search")
        || event_type.contains("web_search")
        || event_type.contains("image_generation")
        || event_type.contains("mcp_")
        || event_type.contains("code_interpreter")
        || event_type.contains("custom_tool");
    if needs_output_index {
        ensure_u32(obj, "output_index", 0);
    }

    let needs_content_index = event_type.contains("output_text")
        || event_type.contains("content_part")
        || event_type.contains("refusal")
        || event_type.contains("reasoning_text")
        || event_type.contains("annotation");
    if needs_content_index {
        ensure_u32(obj, "content_index", 0);
    }

    if event_type.contains("reasoning_summary") {
        ensure_u32(obj, "summary_index", 0);
    }

    if event_type.contains("annotation") {
        ensure_u32(obj, "annotation_index", 0);
    }

    if event_type.contains("partial_image") {
        ensure_u32(obj, "partial_image_index", 0);
        ensure_string(obj, "partial_image_b64", "");
    }

    // Most item-scoped events require `item_id`; `output_item.*` carries `item`.
    if needs_output_index && !event_type.contains("output_item") {
        ensure_string(obj, "item_id", "item_unknown");
    } else if obj.contains_key("item_id") {
        ensure_string(obj, "item_id", "item_unknown");
    }

    if event_type.ends_with(".delta") {
        ensure_string(obj, "delta", "");
    }
    if event_type == "response.output_text.done"
        || event_type == "response.reasoning_summary_text.done"
        || event_type == "response.reasoning_text.done"
    {
        ensure_string(obj, "text", "");
    }
    if event_type == "response.refusal.done" {
        ensure_string(obj, "refusal", "");
    }
    if event_type == "response.function_call_arguments.done"
        || event_type == "response.mcp_call_arguments.done"
    {
        ensure_string(obj, "arguments", "");
    }
    if event_type == "response.code_interpreter_call_code.done" {
        ensure_string(obj, "code", "");
    }
    if event_type == "response.custom_tool_call_input.done" {
        ensure_string(obj, "input", "");
    }
    if event_type == "error" {
        ensure_string(obj, "message", "unknown error");
    }

    if let Some(resp) = obj.get_mut("response") {
        sanitize_response_object(resp);
    }

    if let Some(item) = obj.get_mut("item") {
        if !sanitize_output_item(item) {
            // Replace with a minimal empty message so required `item` still
            // deserializes; content is empty so downstream can ignore it.
            *item = serde_json::json!({
                "type": "message",
                "id": "msg_dropped",
                "role": "assistant",
                "status": "completed",
                "content": []
            });
        }
    }

    // content part / summary part: if unparseable, use empty text part.
    if let Some(part) = obj.get_mut("part") {
        let ok_output = serde_json::from_value::<rs::OutputContent>(part.clone()).is_ok();
        let ok_summary = serde_json::from_value::<rs::SummaryPart>(part.clone()).is_ok();
        if !ok_output && !ok_summary {
            if event_type.contains("reasoning_summary") {
                *part = serde_json::json!({
                    "type": "summary_text",
                    "text": ""
                });
            } else {
                *part = serde_json::json!({
                    "type": "output_text",
                    "text": "",
                    "annotations": []
                });
            }
        }
    }

    if event_type.contains("annotation") && !obj.contains_key("annotation") {
        obj.insert("annotation".to_owned(), Value::Object(Map::new()));
    }
}

/// Sanitize a top-level stream-event JSON object in place.
pub(crate) fn sanitize_stream_event_value(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let event_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    if event_type.is_empty() {
        return;
    }
    sanitize_stream_event_fields(obj, &event_type);
}

/// Parse a Responses SSE data payload with progressive leniency.
pub(crate) fn parse_response_stream_event(data: &str) -> Result<LenientStreamEvent, serde_json::Error> {
    // Fast path: exact OpenAI shape.
    if let Ok(event) = serde_json::from_str::<rs::ResponseStreamEvent>(data) {
        return Ok(LenientStreamEvent::Event(event));
    }

    let mut value: Value = serde_json::from_str(data)?;

    let event_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    if !event_type.is_empty() && !is_known_stream_event_type(&event_type) {
        return Ok(LenientStreamEvent::Skip {
            reason: format!("unknown Responses stream event type '{event_type}'"),
        });
    }

    // Existing tool strip + broader sanitization.
    if let Some(tools) = value
        .pointer_mut("/response/tools")
        .and_then(|v| v.as_array_mut())
    {
        filter_tools_array(tools);
    }
    sanitize_stream_event_value(&mut value);

    match serde_json::from_value::<rs::ResponseStreamEvent>(value.clone()) {
        Ok(event) => Ok(LenientStreamEvent::Event(event)),
        Err(err) => {
            // Still unsalvageable after rewrite — skip rather than kill the
            // stream, unless we have no type at all (then surface the error).
            if event_type.is_empty() {
                Err(err)
            } else {
                Ok(LenientStreamEvent::Skip {
                    reason: format!(
                        "unparseable Responses event '{event_type}' after sanitize: {err}"
                    ),
                })
            }
        }
    }
}

/// Parse a non-streaming `rs::Response` body with the same sanitization.
pub(crate) fn parse_response_object(bytes: &[u8]) -> Result<rs::Response, serde_json::Error> {
    if let Ok(response) = serde_json::from_slice::<rs::Response>(bytes) {
        return Ok(response);
    }
    let mut value: Value = serde_json::from_slice(bytes)?;
    sanitize_response_object(&mut value);
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_sequence_number_is_filled() {
        let data = r#"{"type":"response.output_text.delta","item_id":"i","output_index":0,"content_index":0,"delta":"hi"}"#;
        match parse_response_stream_event(data).expect("parse") {
            LenientStreamEvent::Event(rs::ResponseStreamEvent::ResponseOutputTextDelta(e)) => {
                assert_eq!(e.sequence_number, 0);
                assert_eq!(e.delta, "hi");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn extra_vendor_field_on_delta_is_ignored() {
        let data = r#"{"type":"response.output_text.delta","sequence_number":1,"item_id":"i","output_index":0,"content_index":0,"delta":"x","vendor_extra":{"a":1}}"#;
        match parse_response_stream_event(data).expect("parse") {
            LenientStreamEvent::Event(rs::ResponseStreamEvent::ResponseOutputTextDelta(e)) => {
                assert_eq!(e.delta, "x");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn unknown_event_type_is_skipped() {
        let data = r#"{"type":"response.vendor.custom","sequence_number":1,"foo":true}"#;
        match parse_response_stream_event(data).expect("parse") {
            LenientStreamEvent::Skip { reason } => {
                assert!(reason.contains("unknown"), "{reason}");
            }
            other => panic!("expected skip, got {other:?}"),
        }
    }

    #[test]
    fn response_missing_required_scalars_gets_defaults() {
        let data = r#"{"output":[]}"#;
        let resp = parse_response_object(data.as_bytes()).expect("parse");
        assert_eq!(resp.object, "response");
        assert_eq!(resp.id, "resp_unknown");
        assert!(resp.output.is_empty());
    }

    #[test]
    fn unparseable_output_item_is_dropped() {
        let data = r#"{
            "id":"r1","object":"response","created_at":1,"model":"m",
            "output":[
                {"type":"totally_unknown_item","foo":1},
                {"type":"function_call","call_id":"c1","name":"n","arguments":"{}"}
            ]
        }"#;
        let resp = parse_response_object(data.as_bytes()).expect("parse");
        assert_eq!(resp.output.len(), 1);
        assert!(matches!(
            resp.output[0],
            rs::OutputItem::FunctionCall(_)
        ));
    }

    #[test]
    fn metadata_non_string_values_are_coerced() {
        let data = r#"{
            "id":"r1","object":"response","created_at":1,"model":"m",
            "output":[],
            "metadata":{"n":1,"ok":"yes","flag":true}
        }"#;
        let resp = parse_response_object(data.as_bytes()).expect("parse");
        let meta = resp.metadata.expect("metadata");
        assert_eq!(meta.get("n").map(String::as_str), Some("1"));
        assert_eq!(meta.get("ok").map(String::as_str), Some("yes"));
        assert_eq!(meta.get("flag").map(String::as_str), Some("true"));
    }

    #[test]
    fn completed_event_with_extra_usage_fields_still_parses() {
        let data = r#"{
            "type":"response.completed",
            "sequence_number":9,
            "response":{
                "id":"r1","object":"response","created_at":1,"model":"m",
                "output":[],
                "usage":{
                    "input_tokens":1,"output_tokens":2,"total_tokens":3,
                    "vendor_cost":{"usd":0.01}
                }
            }
        }"#;
        match parse_response_stream_event(data).expect("parse") {
            LenientStreamEvent::Event(rs::ResponseStreamEvent::ResponseCompleted(e)) => {
                assert_eq!(e.sequence_number, 9);
                assert_eq!(e.response.id, "r1");
                let usage = e.response.usage.expect("usage");
                assert_eq!(usage.input_tokens, 1);
                assert_eq!(usage.input_tokens_details.cached_tokens, 0);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}
