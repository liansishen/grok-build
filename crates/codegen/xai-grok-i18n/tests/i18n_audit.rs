use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tree_sitter::{Node, Parser};

#[derive(Debug)]
struct AuditConfig {
    roots: Vec<String>,
    sinks: Vec<String>,
    translation_functions: Vec<String>,
    excluded_path_fragments: Vec<String>,
    settings_files: Vec<String>,
    allow_exact: BTreeSet<String>,
    allow_prefix: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct Finding {
    path: String,
    line: usize,
    sink: String,
    literal: String,
}

impl AuditConfig {
    fn load(repo_root: &Path) -> Self {
        let path = repo_root.join("scripts/i18n-opaque.toml");
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let document: toml::Value = toml::from_str(&raw)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
        let scan = document
            .get("scan")
            .and_then(toml::Value::as_table)
            .expect("i18n-opaque.toml must contain [scan]");
        let allow = document
            .get("allow")
            .and_then(toml::Value::as_table)
            .expect("i18n-opaque.toml must contain [allow]");

        Self {
            roots: string_array(scan, "roots"),
            sinks: string_array(scan, "sinks"),
            translation_functions: string_array(scan, "translation_functions"),
            excluded_path_fragments: string_array(scan, "excluded_path_fragments"),
            settings_files: string_array(scan, "settings_files"),
            allow_exact: string_array(allow, "exact").into_iter().collect(),
            allow_prefix: string_array(allow, "prefix"),
        }
    }

    fn is_excluded(&self, path: &Path) -> bool {
        let normalized = path.to_string_lossy().replace('\\', "/");
        self.excluded_path_fragments
            .iter()
            .any(|fragment| normalized.contains(fragment))
    }

    fn is_allowed_opaque(&self, value: &str) -> bool {
        let trimmed = value.trim();
        self.allow_exact.contains(trimmed)
            || self
                .allow_prefix
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
    }
}

fn string_array(table: &toml::map::Map<String, toml::Value>, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("i18n-opaque.toml field `{key}` must be an array"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("i18n-opaque.toml field `{key}` must contain strings"))
                .to_owned()
        })
        .collect()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("xai-grok-i18n must remain three levels below the repository root")
        .to_path_buf()
}


fn parser() -> Parser {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("tree-sitter Rust language must install");
    parser
}

fn normalized(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .trim_end_matches('!')
        .to_owned()
}

fn name_matches(name: &str, configured: &str) -> bool {
    let name = normalized(name);
    let configured = normalized(configured);
    name == configured
        || name.ends_with(&format!("::{configured}"))
        || name.ends_with(&format!(".{configured}"))
}

fn call_name<'a>(node: Node<'a>, source: &'a [u8]) -> Option<String> {
    node.child_by_field_name("function")
        .and_then(|function| function.utf8_text(source).ok())
        .map(normalized)
}

fn macro_name<'a>(node: Node<'a>, source: &'a [u8]) -> Option<String> {
    node.child_by_field_name("macro")
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).next()
        })
        .and_then(|macro_node| macro_node.utf8_text(source).ok())
        .map(normalized)
}

fn matches_any_name(name: &str, configured: &[String]) -> bool {
    configured.iter().any(|candidate| name_matches(name, candidate))
}

fn preceding_named_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    let mut cursor = parent.walk();
    let mut previous = None;
    for sibling in parent.named_children(&mut cursor) {
        if sibling.start_byte() == node.start_byte() && sibling.end_byte() == node.end_byte() {
            return previous;
        }
        previous = Some(sibling);
    }
    None
}

fn has_test_attribute(item: Node<'_>, source: &[u8]) -> bool {
    let mut previous = preceding_named_sibling(item);
    while let Some(attribute) = previous {
        if attribute.kind() != "attribute_item" {
            break;
        }
        let text = attribute.utf8_text(source).unwrap_or_default();
        if text.trim_start().starts_with("#[test")
            || (text.contains("cfg") && text.contains("test"))
        {
            return true;
        }
        previous = preceding_named_sibling(attribute);
    }
    false
}

fn in_test_code(node: Node<'_>, source: &[u8]) -> bool {
    let mut current = Some(node);
    while let Some(item) = current {
        if matches!(item.kind(), "function_item" | "impl_item" | "mod_item")
            && has_test_attribute(item, source)
        {
            return true;
        }
        current = item.parent();
    }
    false
}

fn string_value(raw: &str) -> Option<String> {
    if raw.starts_with('r') {
        let quote = raw.find('"')?;
        let prefix = &raw[..quote];
        let hashes = prefix.strip_prefix('r')?.len();
        let body_start = quote + 1;
        let closing = format!("\"{}", "#".repeat(hashes));
        let body_end = raw[body_start..].rfind(&closing)? + body_start;
        return Some(raw[body_start..body_end].to_owned());
    }
    if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
        return None;
    }

    let body = &raw[1..raw.len() - 1];
    let mut output = String::with_capacity(body.len());
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        let Some(escaped) = characters.next() else {
            output.push('\\');
            break;
        };
        match escaped {
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            '0' => output.push('\0'),
            '\\' => output.push('\\'),
            '"' => output.push('"'),
            'u' => {
                if characters.next() != Some('{') {
                    output.push('u');
                    continue;
                }
                let mut digits = String::new();
                for digit in characters.by_ref() {
                    if digit == '}' {
                        break;
                    }
                    digits.push(digit);
                }
                if let Ok(codepoint) = u32::from_str_radix(&digits, 16)
                    && let Some(value) = char::from_u32(codepoint)
                {
                    output.push(value);
                }
            }
            other => output.push(other),
        }
    }
    Some(output)
}

fn has_prose_letters(value: &str) -> bool {
    let mut outside_placeholder = String::with_capacity(value.len());
    let mut placeholder_depth = 0usize;
    for character in value.chars() {
        match character {
            '{' => placeholder_depth = placeholder_depth.saturating_add(1),
            '}' if placeholder_depth > 0 => placeholder_depth -= 1,
            _ if placeholder_depth > 0 => {}
            _ => outside_placeholder.push(character),
        }
    }
    outside_placeholder
        .chars()
        .any(|character| character.is_ascii_alphabetic())
}

fn is_candidate(value: &str, config: &AuditConfig) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || !has_prose_letters(value) || config.is_allowed_opaque(value) {
        return false;
    }

    // Identifiers, paths, URLs, and model slugs are opaque when they have no prose whitespace.
    if !trimmed.chars().any(char::is_whitespace)
        && trimmed
            .chars()
            .any(|character| matches!(character, '/' | '\\' | '.' | ':' | '-' | '_' | '@' | '='))
    {
        return false;
    }
    true
}

fn literal_intersects_lines(node: Node<'_>, lines: &BTreeSet<usize>) -> bool {
    if lines.is_empty() {
        return false;
    }
    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;
    lines.range(start..=end).next().is_some()
}

fn visible_sink<'a>(node: Node<'a>, source: &'a [u8], config: &AuditConfig) -> Option<String> {
    let mut current = node.parent();
    while let Some(ancestor) = current {
        if ancestor.kind() == "call_expression"
            && let Some(name) = call_name(ancestor, source)
        {
            if matches_any_name(&name, &config.translation_functions) {
                return None;
            }
            if let Some(sink) = config
                .sinks
                .iter()
                .find(|sink| name_matches(&name, sink))
            {
                return Some(sink.clone());
            }
        }
        if ancestor.kind() == "macro_invocation"
            && let Some(name) = macro_name(ancestor, source)
        {
            if matches_any_name(&name, &config.translation_functions) {
                return None;
            }
            if let Some(sink) = config
                .sinks
                .iter()
                .find(|sink| name_matches(&name, sink))
            {
                return Some(sink.clone());
            }
        }
        current = ancestor.parent();
    }
    None
}

fn scan_node(
    node: Node<'_>,
    source: &[u8],
    path: &str,
    config: &AuditConfig,
    changed_lines: Option<&BTreeSet<usize>>,
    findings: &mut Vec<Finding>,
) {
    if matches!(node.kind(), "string_literal" | "raw_string_literal")
        && !in_test_code(node, source)
        && changed_lines.is_none_or(|lines| literal_intersects_lines(node, lines))
        && let Ok(raw) = node.utf8_text(source)
        && let Some(value) = string_value(raw)
        && is_candidate(&value, config)
        && let Some(sink) = visible_sink(node, source, config)
    {
        findings.push(Finding {
            path: path.to_owned(),
            line: node.start_position().row + 1,
            sink,
            literal: value,
        });
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan_node(child, source, path, config, changed_lines, findings);
    }
}

fn scan_source(
    path: &str,
    source: &[u8],
    config: &AuditConfig,
    changed_lines: Option<&BTreeSet<usize>>,
) -> Result<Vec<Finding>, String> {
    let mut parser = parser();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| format!("tree-sitter returned no tree for {path}"))?;
    if tree.root_node().has_error() {
        return Err(format!("Rust parser reported a syntax error in {path}"));
    }
    let mut findings = Vec::new();
    scan_node(
        tree.root_node(),
        source,
        path,
        config,
        changed_lines,
        &mut findings,
    );
    Ok(findings)
}

fn scan_missing_translation_keys(
    path: &str,
    source: &[u8],
    config: &AuditConfig,
    changed_lines: Option<&BTreeSet<usize>>,
) -> Result<Vec<Finding>, String> {
    let mut parser = parser();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| format!("tree-sitter returned no tree for {path}"))?;
    if tree.root_node().has_error() {
        return Err(format!("Rust parser reported a syntax error in {path}"));
    }

    fn visit(
        node: Node<'_>,
        source: &[u8],
        path: &str,
        config: &AuditConfig,
        changed_lines: Option<&BTreeSet<usize>>,
        findings: &mut Vec<Finding>,
    ) {
        if node.kind() == "call_expression"
            && !in_test_code(node, source)
            && changed_lines.is_none_or(|lines| literal_intersects_lines(node, lines))
            && let Some(name) = call_name(node, source)
        {
            let indexes: &[usize] = if name_matches(&name, "t_for") { &[1] } else { &[0] };
            if matches_any_name(&name, &config.translation_functions)
                && (name_matches(&name, "t")
                    || name_matches(&name, "t_for")
                    || name_matches(&name, "t_fmt")
                    || name_matches(&name, "t_or"))
                && let Some(arguments) = node.child_by_field_name("arguments")
            {
                let mut named = arguments.walk();
                let args = arguments.named_children(&mut named).collect::<Vec<_>>();
                for &index in indexes {
                    let Some(argument) = args.get(index) else {
                        continue;
                    };
                    if !matches!(argument.kind(), "string_literal" | "raw_string_literal") {
                        continue;
                    }
                    if !changed_lines.is_none_or(|lines| literal_intersects_lines(*argument, lines)) {
                        continue;
                    }
                    let Ok(raw) = argument.utf8_text(source) else {
                        continue;
                    };
                    let Some(key) = string_value(raw) else {
                        continue;
                    };
                    if !xai_grok_i18n::has_en(&key) {
                        findings.push(Finding {
                            path: path.to_owned(),
                            line: argument.start_position().row + 1,
                            sink: "missing translation key".to_owned(),
                            literal: key,
                        });
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            visit(child, source, path, config, changed_lines, findings);
        }
    }

    let mut findings = Vec::new();
    visit(tree.root_node(), source, path, config, changed_lines, &mut findings);
    Ok(findings)
}

fn field_name<'a>(node: Node<'a>, source: &'a [u8]) -> Option<String> {
    node.child_by_field_name("field")
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).last()
        })
        .and_then(|field| field.utf8_text(source).ok())
        .map(normalized)
}

fn field_receiver<'a>(node: Node<'a>, source: &'a [u8]) -> Option<String> {
    node.child_by_field_name("value")
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).next()
        })
        .and_then(|value| value.utf8_text(source).ok())
        .map(normalized)
}

fn banned_settings_field(node: Node<'_>, source: &[u8]) -> Option<&'static str> {
    if node.kind() != "field_expression" {
        return None;
    }
    let field = field_name(node, source)?;
    let receiver = field_receiver(node, source)?;
    let receiver_is_meta = matches!(
        receiver.as_str(),
        "meta" | "group_meta" | "child_meta" | "self.meta"
    ) || receiver.ends_with(".meta");
    let receiver_is_choice = receiver == "choice" || receiver.ends_with(".choice");

    if receiver_is_meta && matches!(field.as_str(), "label" | "description") {
        return Some("setting metadata field");
    }
    if receiver_is_choice && matches!(field.as_str(), "display" | "description") {
        return Some("enum choice field");
    }
    None
}

fn scan_settings_fields(path: &str, source: &[u8]) -> Result<Vec<Finding>, String> {
    let mut parser = parser();
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| format!("tree-sitter returned no tree for {path}"))?;
    if tree.root_node().has_error() {
        return Err(format!("Rust parser reported a syntax error in {path}"));
    }

    fn visit(node: Node<'_>, source: &[u8], path: &str, findings: &mut Vec<Finding>) {
        if let Some(kind) = banned_settings_field(node, source) {
            let field = field_name(node, source).unwrap_or_else(|| "field".to_owned());
            findings.push(Finding {
                path: path.to_owned(),
                line: node.start_position().row + 1,
                sink: kind.to_owned(),
                literal: field,
            });
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            visit(child, source, path, findings);
        }
    }

    let mut findings = Vec::new();
    visit(tree.root_node(), source, path, &mut findings);
    Ok(findings)
}

fn parse_new_line_range(header: &str) -> Option<(usize, usize)> {
    let plus = header.split_whitespace().find(|part| part.starts_with('+'))?;
    let mut range = plus.trim_start_matches('+').split(',');
    let start = range.next()?.parse::<usize>().ok()?;
    let count = range.next().unwrap_or("1").parse::<usize>().ok()?;
    Some((start, count))
}

fn changed_files_from_diff(diff: &str) -> BTreeMap<String, BTreeSet<usize>> {
    let mut files = BTreeMap::<String, BTreeSet<usize>>::new();
    let mut current_path: Option<String> = None;
    let mut next_new_line: Option<usize> = None;

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(path.to_owned());
            next_new_line = None;
            continue;
        }
        if line.starts_with("@@") {
            if let Some((start, count)) = parse_new_line_range(line) {
                next_new_line = (count > 0).then_some(start);
            }
            continue;
        }
        let Some(next) = next_new_line.as_mut() else {
            continue;
        };
        if line.starts_with('+') && !line.starts_with("+++") {
            if let Some(path) = &current_path {
                files.entry(path.clone()).or_default().insert(*next);
            }
            *next += 1;
        } else if line.starts_with('-') || line.starts_with('\\') {
            // Removed lines do not advance the new-file line number.
        } else {
            *next += 1;
        }
    }
    files
}

fn git_diff(repo_root: &Path, old: &str, new: &str, roots: &[String]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--no-renames", "--unified=0", old, new, "--"])
        .args(roots)
        .output()
        .map_err(|error| format!("cannot run git diff: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git diff {old} {new} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_show(repo_root: &Path, revision: &str, path: &str) -> Result<Vec<u8>, String> {
    let object = format!("{revision}:{path}");
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["show", &object])
        .output()
        .map_err(|error| format!("cannot read {object}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git show {object} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn audit_revision_diff(
    repo_root: &Path,
    old: &str,
    new: &str,
    config: &AuditConfig,
) -> Result<Vec<Finding>, String> {
    let diff = git_diff(repo_root, old, new, &config.roots)?;
    let changed = changed_files_from_diff(&diff);
    let mut findings = Vec::new();
    for (path, lines) in changed {
        if !path.ends_with(".rs") || config.is_excluded(Path::new(&path)) {
            continue;
        }
        let source = git_show(repo_root, new, &path)?;
        findings.extend(scan_source(&path, &source, config, Some(&lines))?);
        findings.extend(scan_missing_translation_keys(&path, &source, config, Some(&lines))?);
    }
    Ok(findings)
}

fn assert_no_findings(scope: &str, findings: &[Finding]) {
    if findings.is_empty() {
        return;
    }
    let details = findings
        .iter()
        .map(|finding| {
            format!(
                "{}:{} [{}] {:?}",
                finding.path, finding.line, finding.sink, finding.literal
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    panic!("{scope} found untranslated user-visible strings or fields:\n{details}");
}

#[test]
fn settings_field_fixture_detects_untranslated_field_access() {
    let source = br#"
        fn render(meta: &Meta, choice: &Choice) {
            let _ = meta.label;
            let _ = meta.description;
            let _ = choice.display;
            let _ = choice.description;
            let _ = meta.label_t();
        }
    "#;
    let findings = scan_settings_fields("settings_fixture.rs", source).expect("fixture parses");
    assert_eq!(findings.len(), 4);
    assert!(findings.iter().any(|finding| finding.literal == "label"));
    assert!(findings.iter().any(|finding| finding.literal == "description"));
    assert!(findings.iter().any(|finding| finding.literal == "display"));
}

#[test]
fn diff_fixture_tracks_only_added_lines() {
    let diff = "diff --git a/src/view.rs b/src/view.rs\n--- a/src/view.rs\n+++ b/src/view.rs\n@@ -2,2 +2,3 @@\n old\n+new\n context\n";
    let changed = changed_files_from_diff(diff);
    assert_eq!(changed.get("src/view.rs"), Some(&BTreeSet::from([3])));
}

#[test]
fn fixture_flags_direct_output_but_allows_translation_and_opaque_values() {
    let config = AuditConfig::load(&repo_root());
    let source = br#"
        fn render() {
            show_toast("Waiting for approval");
            show_toast(t("toast.waiting"));
            Span::styled("Enter", style);
            println!("https://example.com");
        }
    "#;
    let findings = scan_source("fixture.rs", source, &config, None).expect("fixture parses");
    assert_eq!(
        findings,
        vec![Finding {
            path: "fixture.rs".to_owned(),
            line: 3,
            sink: "show_toast".to_owned(),
            literal: "Waiting for approval".to_owned(),
        }]
    );
}

#[test]
fn settings_renderer_uses_translated_accessors_for_metadata_and_choices() {
    let root = repo_root();
    let config = AuditConfig::load(&root);
    for relative in &config.settings_files {
        let path = root.join(relative);
        let source = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let findings = scan_settings_fields(relative, &source).expect("settings source parses");
        assert_no_findings("settings accessor audit", &findings);
    }
}

#[test]
fn changed_user_visible_strings_use_translation_or_opaque_allowlist() {
    let base = match std::env::var("GROK_I18N_AUDIT_BASE") {
        Ok(value) if !value.trim().is_empty() && !value.chars().all(|character| character == '0') => value,
        _ => {
            eprintln!("GROK_I18N_AUDIT_BASE is not set; skipping repository diff audit");
            return;
        }
    };
    let root = repo_root();
    let config = AuditConfig::load(&root);
    let findings = audit_revision_diff(&root, &base, "HEAD", &config)
        .unwrap_or_else(|error| panic!("repository i18n audit failed: {error}"));
    assert_no_findings("repository diff audit", &findings);
}

#[test]
fn upstream_changed_user_visible_strings_use_translation_or_opaque_allowlist() {
    let old = std::env::var("GROK_I18N_UPSTREAM_OLD").ok();
    let new = std::env::var("GROK_I18N_UPSTREAM_NEW").ok();
    match (old, new) {
        (None, None) => {
            eprintln!("upstream revisions are not set; skipping upstream diff audit");
        }
        (Some(old), Some(new)) => {
            let root = repo_root();
            let config = AuditConfig::load(&root);
            let findings = audit_revision_diff(&root, &old, &new, &config)
                .unwrap_or_else(|error| panic!("upstream i18n audit failed: {error}"));
            assert_no_findings("upstream diff audit", &findings);
        }
        _ => panic!("GROK_I18N_UPSTREAM_OLD and GROK_I18N_UPSTREAM_NEW must be set together"),
    }
}
