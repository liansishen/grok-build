//! Runtime localization for model-facing tool descriptions and schemas.

use crate::types::tool::ToolNamespace;
use serde_json::Value;

/// Localize a built-in description template while preserving MiniJinja markers.
pub(crate) fn localized_description(raw: &str) -> String {
    localized_value("tool.description", raw)
}

pub(crate) fn localized_description_for_namespace(
    namespace: ToolNamespace,
    raw: &str,
) -> String {
    let namespace_key = catalog_key(
        &format!("tool.description.{}", namespace_slug(namespace)),
        raw,
    );
    if xai_grok_i18n::has_en(namespace_key) {
        xai_grok_i18n::t(namespace_key).to_string()
    } else {
        localized_description(raw)
    }
}

fn namespace_slug(namespace: ToolNamespace) -> &'static str {
    match namespace {
        ToolNamespace::GrokBuild => "grok_build",
        ToolNamespace::GrokBuildConcise => "grok_build_concise",
        ToolNamespace::GrokBuildHashline => "grok_build_hashline",
        ToolNamespace::Codex => "codex",
        ToolNamespace::OpenCode => "opencode",
        ToolNamespace::MCP => "mcp",
    }
}

/// Localize every generated JSON Schema description in place.
///
/// Dynamic MCP schemas are excluded by the registry caller. Missing catalog
/// entries keep the source description so custom and user-provided text is safe.
pub(crate) fn localize_schema_descriptions(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            if let Some(Value::String(description)) = map.get("description") {
                let localized = localized_value("tool.schema", description);
                if localized != *description {
                    map.insert("description".to_string(), Value::String(localized));
                }
            }
            for value in map.values_mut() {
                localize_schema_descriptions(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                localize_schema_descriptions(item);
            }
        }
        _ => {}
    }
}

fn localized_value(namespace: &str, raw: &str) -> String {
    let key = catalog_key(namespace, raw);
    if xai_grok_i18n::has_en(key) {
        xai_grok_i18n::t(key).to_string()
    } else {
        raw.to_string()
    }
}

fn catalog_key(namespace: &str, raw: &str) -> &'static str {
    let first_line = raw
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(raw)
        .trim();
    let mut slug = String::new();
    let mut separator = false;
    for ch in first_line.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            separator = false;
        } else if !separator {
            slug.push('_');
            separator = true;
        }
    }
    let slug = slug.trim_matches('_');
    let slug = if slug.is_empty() { "text" } else { slug };
    xai_grok_i18n::intern_key(&format!("{namespace}.{slug}"))
}
