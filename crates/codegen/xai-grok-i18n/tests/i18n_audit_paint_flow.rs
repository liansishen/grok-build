//! Copy that leaves a file through a function return, a struct field, a const, or a helper that paints
//! its parameter — the shapes the per-file [`FlowIndex`](super::FlowIndex) cannot follow.
//!
//! That index walks syntactic ancestors and tracks `let`/buffer bindings, so it sees
//! `let line = helper(); Span::styled(line, …)`. It misses the dock, where the copy travels as:
//!
//! ```ignore
//! impl Section { fn label(self) -> &'static str { … => "Workflows" } }  // literal in a return value
//! let line = section_header(theme, width, expanded, section.label(), count); // method call, argument
//! Span::styled(label.to_string(), style)                                // callee paints its parameter
//! DockRow { kind: "Workflow".into(), … }                                // literal into a struct field
//! Span::styled(row.kind.clone(), accent)                                // that field, painted elsewhere
//! ```
//!
//! Method calls hide the callee name behind a `field_expression` (the ancestor walk only sees plain
//! `identifier`s), a call argument is not a binding, and a struct field is neither.
//!
//! This pass builds one workspace-wide graph over four node kinds:
//!
//! | Node | Meaning |
//! |------|---------|
//! | `bind:<file>:<name>` | value of a `let` / `const` / `static`, scoped to its file |
//! | `field:<name>` | value assigned to a struct-literal field |
//! | `fn:<name>` | value a function returns (bare name: a call site cannot see the receiver's type) |
//! | `param:<name>#<index>` | value passed into a function parameter |
//!
//! An edge says "this value flows into that one". Values a configured prose sink paints are the
//! roots ([`State::painted_roots`]); a literal is reported when the node its value lands in is
//! reachable from one of them by following edges backwards.
//!
//! # Scope and precision
//!
//! This pass runs in the *diff* audit, over the whole revision's tree, and reports only findings on
//! changed lines. The full scan keeps its existing baseline, so copy that predates the pass is not
//! reported; every newly added literal in one of these shapes is gated.
//!
//! Keys are as narrow as the syntax allows. `bind:` and `param:` keys carry their file, and so do
//! `fn:` keys: a call site sees only the callee's name, and a global `fn:len` would let `x.len()`
//! anywhere mark every `len` helper in the workspace as painted. `field:` keys stay global because the
//! field is written in one file and read back in another (`DockRow.kind` is the case this pass exists
//! for). Name sharing can therefore still over-report; `[allow]` covers a genuine collision.

use super::*;

/// Keys that reach the screen, collected while walking.
#[derive(Default)]
struct State {
    /// Source node → nodes its value flows into.
    edges: BTreeMap<String, BTreeSet<String>>,
    /// Values a visible prose sink renders directly.
    painted_roots: BTreeSet<String>,
    /// Candidate literals whose carrier decides whether they are findings.
    pending: Vec<PendingSite>,
}

/// A candidate literal that only the flow graph can judge.
struct PendingSite {
    path: String,
    line: usize,
    literal: String,
    /// Node key the literal's value lands in.
    carrier: String,
}

/// Workspace-wide paint flow. Feed every first-party file with [`collect_file`](Self::collect_file),
/// then resolve with [`into_findings`](Self::into_findings).
pub(crate) struct PaintFlow {
    config: AuditConfig,
    state: State,
}

/// Function context: the key call sites match and the parameter names by index.
#[derive(Clone, Default)]
struct FnContext {
    name: Option<String>,
    params: BTreeMap<String, usize>,
}

impl PaintFlow {
    pub(crate) fn new(config: AuditConfig) -> Self {
        Self {
            config,
            state: State::default(),
        }
    }

    /// Parse one file and fold its flow edges, painted roots and literal sites into the graph.
    pub(crate) fn collect_file(&mut self, path: &str, source: &[u8]) -> Result<(), String> {
        let mut parser = parser();
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| format!("tree-sitter returned no tree for {path}"))?;
        self.collect_tree(tree.root_node(), path, source);
        Ok(())
    }

    /// Fold an already-parsed file into the graph.
    pub(crate) fn collect_tree(&mut self, root: Node<'_>, path: &str, source: &[u8]) {
        walk(
            root,
            path,
            source,
            &FnContext::default(),
            &self.config,
            &mut self.state,
        );
    }

    /// Findings for the pending sites whose carrier a visible sink paints.
    pub(crate) fn into_findings(self) -> Vec<Finding> {
        let mut reverse: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (from, targets) in &self.state.edges {
            for target in targets {
                reverse.entry(target.as_str()).or_default().insert(from.as_str());
            }
        }

        let mut painted: BTreeSet<&str> = BTreeSet::new();
        let mut queue: Vec<&str> = self.state.painted_roots.iter().map(String::as_str).collect();
        while let Some(current) = queue.pop() {
            if !painted.insert(current) {
                continue;
            }
            if let Some(sources) = reverse.get(current) {
                queue.extend(sources.iter().copied());
            }
        }

        self.state
            .pending
            .into_iter()
            .filter(|site| painted.contains(site.carrier.as_str()))
            .map(|site| Finding {
                path: site.path,
                line: site.line,
                sink: "paint-flow/UI carrier".to_owned(),
                literal: site.literal,
            })
            .collect()
    }
}

fn bind_key(path: &str, name: &str) -> String {
    format!("bind:{path}:{name}")
}

fn field_key(field: &str) -> String {
    format!("field:{field}")
}

/// Function and parameter keys carry their file: a call site sees only the callee's name, so a global
/// key would let `x.len()` anywhere mark every `len` helper in the workspace as painted. Scoping to the
/// file keeps the helper-and-its-painter shape (the common one) precise and drops the cross-file guess.
fn fn_key(path: &str, name: &str) -> String {
    format!("fn:{path}:{name}")
}

fn param_key(path: &str, name: &str, index: usize) -> String {
    format!("param:{path}:{name}#{index}")
}

fn add_edges(state: &mut State, sources: BTreeSet<String>, target: &str) {
    for source in sources {
        state
            .edges
            .entry(source)
            .or_default()
            .insert(target.to_owned());
    }
}

fn walk(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    ctx: &FnContext,
    config: &AuditConfig,
    state: &mut State,
) {
    let ctx = if node.kind() == "function_item" {
        FnContext {
            name: function_name(node, source),
            params: parameter_indices(node, source),
        }
    } else {
        ctx.clone()
    };

    match node.kind() {
        // `let name = value`, `const NAME: &str = value`
        "let_declaration" | "const_item" | "static_item" => {
            if let (Some(name), Some(value)) = (
                binding_name(node, source),
                node.child_by_field_name("value"),
            ) {
                let target = bind_key(path, &name);
                let sources = dependencies(value, path, source, &ctx);
                add_edges(state, sources, &target);
            }
        }
        // `Struct { field: value }`
        "field_initializer" => {
            if let (Some(field), Some(value)) = (
                node.child_by_field_name("field"),
                node.child_by_field_name("value"),
            ) && let Ok(field) = field.utf8_text(source)
            {
                let target = field_key(field);
                let sources = dependencies(value, path, source, &ctx);
                add_edges(state, sources, &target);
            }
        }
        // `return value`
        "return_expression" => {
            if let Some(name) = ctx.name.as_deref() {
                let target = fn_key(path, name);
                for child in node.named_children(&mut node.walk()) {
                    let sources = dependencies(child, path, source, &ctx);
                    add_edges(state, sources, &target);
                }
            }
        }
        // Every argument flows into the matching parameter of the callee.
        "call_expression" => {
            if let Some((callee, arguments)) = call_parts(node, source) {
                for (index, argument) in arguments.iter().enumerate() {
                    let target = param_key(path, &callee, index);
                    let sources = dependencies(*argument, path, source, &ctx);
                    add_edges(state, sources, &target);
                }
            }
        }
        _ => {}
    }

    // Tail expression: the function's value, same as `return`.
    if node.kind() == "block"
        && node.parent().is_some_and(|parent| parent.kind() == "function_item")
        && let Some(name) = ctx.name.as_deref()
        && let Some(last) = last_named_child(node)
    {
        let target = fn_key(path, name);
        let sources = dependencies(last, path, source, &ctx);
        add_edges(state, sources, &target);
    }

    // A visible prose sink paints the argument it renders, everything passed into calls inside that
    // argument (a helper paints its parameter), and any field it reads.
    if node.kind() == "call_expression"
        && call_is_visible_sink(node, source, config, &config.prose_sinks)
        && let Some(function_name) = call_name(node, source)
        && let Some((_, arguments)) = call_parts(node, source)
        && let Some(argument) = arguments.get(sink_argument_index(&function_name))
    {
        collect_painted(*argument, path, source, &ctx, &mut state.painted_roots);
    }

    // Candidate literal: remember where its value lands so the graph can judge it later.
    if matches!(node.kind(), "string_literal" | "raw_string_literal")
        && !inside_translation_call(node, source, config)
        && !inside_comparison(node)
        && !in_test_code(node, source)
        && !inside_developer_diagnostic(node, source)
        && !inside_developer_log(node, source)
        && !inside_token_position(node, source)
        && visible_sink_in(node, source, config, &config.prose_sinks).is_none()
        && let Ok(raw) = node.utf8_text(source)
        && let Some(value) = string_value(raw)
        && is_candidate(&value, config)
        && let Some(carrier) = literal_carrier(node, path, source, &ctx)
    {
        state.pending.push(PendingSite {
            path: path.to_owned(),
            line: node.start_position().row + 1,
            literal: value,
            carrier,
        });
    }

    for child in node.named_children(&mut node.walk()) {
        walk(child, path, source, &ctx, config, state);
    }
}

fn function_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .map(str::to_owned)
}

/// Parameter names by position, `self` excluded so the index matches call arguments.
fn parameter_indices(node: Node<'_>, source: &[u8]) -> BTreeMap<String, usize> {
    let mut params = BTreeMap::new();
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return params;
    };
    let mut index = 0;
    for parameter in parameters.named_children(&mut parameters.walk()) {
        if parameter.kind() != "parameter" {
            continue;
        }
        if let Some(pattern) = parameter.child_by_field_name("pattern")
            && let Some(identifier) = first_identifier(pattern, source)
        {
            params.insert(identifier, index);
        }
        index += 1;
    }
    params
}

fn first_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "identifier"
        && let Ok(text) = node.utf8_text(source)
    {
        return Some(text.to_owned());
    }
    node.named_children(&mut node.walk())
        .find_map(|child| first_identifier(child, source))
}

fn last_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let count = node.named_child_count();
    if count == 0 {
        None
    } else {
        node.named_child((count - 1) as u32)
    }
}

/// Callee name (a free function or a method's field name) and the call's arguments.
fn call_parts<'tree>(node: Node<'tree>, source: &[u8]) -> Option<(String, Vec<Node<'tree>>)> {
    let function = node.child_by_field_name("function")?;
    let callee = match function.kind() {
        "identifier" => function.utf8_text(source).ok()?.to_owned(),
        "field_expression" => function
            .child_by_field_name("field")
            .and_then(|field| field.utf8_text(source).ok())?
            .to_owned(),
        "scoped_identifier" => function
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source).ok())?
            .to_owned(),
        _ => return None,
    };
    let arguments = node.child_by_field_name("arguments")?;
    Some((
        callee,
        arguments.named_children(&mut arguments.walk()).collect(),
    ))
}

/// Node keys the value of `node` flows into: identifiers (bindings or parameters), struct fields, and
/// callees of calls the value mentions.
fn dependencies(node: Node<'_>, path: &str, source: &[u8], ctx: &FnContext) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    collect_dependencies(node, path, source, ctx, &mut keys);
    keys
}

fn collect_dependencies(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    ctx: &FnContext,
    keys: &mut BTreeSet<String>,
) {
    // Each arm descends exactly once. Letting the shared child walk repeat an arm's descent made a
    // method or field chain cost 2^length: `a.b().c().d()` is one nested expression per link, and the
    // repository scan then stalled on a single 35 KB file.
    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(source) {
                keys.insert(reference_key(name, path, ctx));
            }
            return;
        }
        "field_expression" => {
            if let Some(field) = node.child_by_field_name("field")
                && let Ok(field) = field.utf8_text(source)
            {
                keys.insert(field_key(field));
            }
            // Read through the chain, but stop at the receiver: `bind:<receiver>` would tie this
            // expression to an unrelated local of the same name.
            if let Some(value) = node.child_by_field_name("value")
                && matches!(value.kind(), "field_expression" | "call_expression")
            {
                collect_dependencies(value, path, source, ctx, keys);
            }
            return;
        }
        // A call contributes its callee; its arguments belong to its own node, which maps them to
        // that call's parameters.
        "call_expression" => {
            if let Some((callee, _)) = call_parts(node, source) {
                keys.insert(fn_key(path, &callee));
            }
            return;
        }
        // `format!("{count}")` hides its named placeholders inside the token tree.
        "string_literal" | "raw_string_literal" => {
            if let Ok(raw) = node.utf8_text(source)
                && let Some(value) = string_value(raw)
            {
                let mut identifiers = BTreeSet::new();
                collect_identifiers(node, source, &mut identifiers);
                for identifier in identifiers {
                    if value.contains(&format!("{{{identifier}}}"))
                        || value.contains(&format!("{{{identifier}:"))
                    {
                        keys.insert(reference_key(&identifier, path, ctx));
                    }
                }
            }
            return;
        }
        _ => {}
    }
    for child in node.named_children(&mut node.walk()) {
        collect_dependencies(child, path, source, ctx, keys);
    }
}

fn reference_key(name: &str, path: &str, ctx: &FnContext) -> String {
    match ctx.params.get(name) {
        Some(index) => match ctx.name.as_deref() {
            Some(function) => param_key(path, function, *index),
            None => bind_key(path, name),
        },
        None => bind_key(path, name),
    }
}

/// Every value a painted expression reads: identifiers, struct fields, and callees.
///
/// Each node is visited once. An arm that also falls through to the shared child loop would cost
/// `2^length` on a method or field chain, which stalled the repository scan on one 35 KB file.
fn collect_painted(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    ctx: &FnContext,
    painted: &mut BTreeSet<String>,
) {
    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(source) {
                painted.insert(reference_key(name, path, ctx));
            }
        }
        "field_expression" => {
            if let Some(field) = node.child_by_field_name("field")
                && let Ok(field) = field.utf8_text(source)
            {
                painted.insert(field_key(field));
            }
        }
        "call_expression" => {
            if let Some((callee, _)) = call_parts(node, source) {
                painted.insert(fn_key(path, &callee));
            }
        }
        // `format!("{count}")` hides its named placeholders inside the token tree.
        "string_literal" | "raw_string_literal" => {
            if let Ok(raw) = node.utf8_text(source)
                && let Some(value) = string_value(raw)
            {
                let mut identifiers = BTreeSet::new();
                collect_identifiers(node, source, &mut identifiers);
                for identifier in identifiers {
                    if value.contains(&format!("{{{identifier}}}"))
                        || value.contains(&format!("{{{identifier}:"))
                    {
                        painted.insert(reference_key(&identifier, path, ctx));
                    }
                }
            }
        }
        _ => {}
    }
    for child in node.named_children(&mut node.walk()) {
        collect_painted(child, path, source, ctx, painted);
    }
}

/// The node key a literal's value lands in, for the shapes this pass resolves.
///
/// Precision order: a struct-literal field beats a surrounding return or binding, because the field is
/// what the renderer reads back.
fn literal_carrier(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    ctx: &FnContext,
) -> Option<String> {
    // Struct-literal field value.
    let mut current = node.parent();
    while let Some(item) = current {
        match item.kind() {
            "field_initializer" => {
                let field = item.child_by_field_name("field")?;
                return Some(field_key(field.utf8_text(source).ok()?));
            }
            "function_item" => break,
            _ => {}
        }
        current = item.parent();
    }

    // Binding initializer, returned value, or call argument.
    let mut current = node.parent();
    while let Some(item) = current {
        match item.kind() {
            "let_declaration" | "const_item" | "static_item" => {
                return Some(bind_key(path, &binding_name(item, source)?));
            }
            "return_expression" => return ctx.name.as_deref().map(|name| fn_key(path, name)),
            "arguments" => {
                let call = item.parent()?;
                let (callee, arguments) = call_parts(call, source)?;
                let index = arguments.iter().position(|argument| {
                    argument.start_byte() <= node.start_byte()
                        && node.end_byte() <= argument.end_byte()
                })?;
                return Some(param_key(path, &callee, index));
            }
            "function_item" => break,
            _ => {}
        }
        current = item.parent();
    }

    // Tail expression of the enclosing function.
    let mut current = node.parent();
    while let Some(item) = current {
        if item.kind() == "block"
            && item.parent().is_some_and(|parent| parent.kind() == "function_item")
            && let Some(last) = last_named_child(item)
            && last.start_byte() <= node.start_byte()
            && node.end_byte() <= last.end_byte()
        {
            return ctx.name.as_deref().map(|name| fn_key(path, name));
        }
        if item.kind() == "function_item" {
            break;
        }
        current = item.parent();
    }
    None
}
