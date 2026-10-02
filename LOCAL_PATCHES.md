# Local patches pending upstream

Track temporary fork-only fixes that should be **reverted** when upstream
(`xai-org/grok-build`) lands an equivalent fix. Search the codebase for
`LOCAL-PATCH(upstream-fork-secondary-model)` to find every touch point.

The broader disposition of Fork features (including candidates that should be upstreamed or removed) is tracked in [`FORK_FEATURES.md`](FORK_FEATURES.md). Every remaining code marker must use a patch id documented here; features that are not temporary patches belong in the feature ledger rather than receiving an untracked marker.

## `upstream-link-rendering` (2026-09-27)

### Problems (present on upstream; not introduced by this fork)

1. **Markers inside a link or image alt text are printed.** `Tag::Link`/`Tag::Image` push `link_text` over the
   whole link interior, so the `**` bytes of a nested emphasis carry the hidden `strong_outer` *and* the visible
   `link_text` at once. The pretty-mode skip decision (`all_hidden`: every covering style must be hidden) then
   fails and the markers are drawn:

   ```text
   [**Modernity-TomsStorage-0.2.2.zip**](build/resourcepacks/Modernity-TomsStorage-0.2.2.zip)
   → **Modernity-TomsStorage-0.2.2.zip** (build/resourcepacks/Modernity-TomsStorage-0.2.2.zip)
   ```

   Nested emphasis, strikethrough, and inline code leak the same way; table cells take a different path and were
   already correct.

2. **An image keeps its `!`.** `![alt](url)` renders as `!alt (url)`; only the `[` was removed.

3. **Reference-style links keep their source form.** `[text][ref]`, `[text][]`, and `[text]` render literally,
   because the destination is not inside the tag range and the inline splitter (which looks for `](`) cannot find
   it.

### Local fix (revert when upstream ships)

| Area | Change |
|------|--------|
| `buffers.rs` | `MarkdownBuffers::link_container_highlights: Vec<usize>` records container highlight indices |
| `parse.rs` | `push_link_container_highlight` records the index; used for the `link_text` interior, the degenerate whole-range fallback, and the reference-form text |
| `parse.rs` | the opener transform drops `![` for images, not only `[` |
| `parse.rs` | `try_push_reference_link` rewrites `[text][label]` / `[text][]` / `[text]` to `text (url)`: opener removed, label replaced, `link_text` container over the text, `link_url` over the rewritten tail |
| `render.rs` | both skip decisions (`render_ansi` and `render_ratatui`) drop container highlights before `all_hidden`, so a hidden marker stays hidden while the container still reaches the drawn style through `merge_styles` |
| tests | `hyperlinks::hyperlink_tests::markers_inside_link_text_are_hidden`, `link_text_inside_emphasis_keeps_target_and_styling`, `reference_link_text_is_the_clickable_span`, `render::tests::test_pretty_image_bracket_removed`, `test_pretty_reference_links_rewritten`, `test_pretty_reference_link_hides_inner_markers`, pager `markers_inside_link_text_render_hidden` |

Marking by *index* is required: comparing style values misfires because themes legitimately give several elements
the same style (in the crate's `test_style` the link styles equal the plain text style), and dropping those
highlights collapses code blocks and plain text.

The rewritten tail also has to carry a highlight: text after the last render event is emitted by the trailing
path, which applies force transforms only, so an unstyled tail would keep its raw source.

### Revert condition

Revert when upstream hides syntax markers inside a link/image container, drops the image `!`, and renders
reference-style links like inline ones (for example by applying link styling per content event instead of over
the whole interior, and by resolving every `LinkType` to the same `text (url)` shape).

### Notes (local)

- Container styles still contribute to the drawn style; only the skip decision ignores them.
- Raw mode is unchanged: it shows the source, so markers, the `!`, and the reference label stay visible there.
- A reference *definition* line (`[ref]: https://e.com`) is still drawn as source text; only the link itself is
  rewritten.
- Link titles and urls remain literal text, where markers are not syntax.

## `upstream-pulldown-unreleased` (2026-09-27)

### Problem (present on upstream; not introduced by this fork)

Two parser behaviors we need exist only on pulldown-cmark's `main`, not in any published release
(crates.io tops out at `0.13.4`, 2026-05-20; this repo locked `0.13.0`):

| Behavior | Upstream |
|---|---|
| Emphasis next to CJK punctuation (`**验证结果：**3 项…`, `**テスト。**テスト`, `これは**「重要」**です`) | `Options::ENABLE_CJK_FRIENDLY_EMPHASIS`, PR #1059, merged 2026-07-30 (`f978fb0`) |
| A closing `$` may not be followed by a digit (currency vs inline math) | PR #1098, merged 2026-05-22 (`3d67feabb`) |

Without the first, a bold label whose CJK colon sits inside the markers and whose sentence continues
right after them prints its `**` literally, because CommonMark's right-flanking rule rejects the
closer (commonmark/commonmark-spec#650).

### Local fix (revert when upstream ships)

| Area | Change |
|------|--------|
| workspace `Cargo.toml` | `[workspace.dependencies] pulldown-cmark` pinned to `f978fb051404b483b344813d4b0235414b5e4f8f` (the PR #1059 merge), which also carries PR #1098. A direct git dependency is required rather than `[patch.crates-io]`: `prost-build` (a build-dependency) takes pulldown-cmark with `default-features = false`, resolver v2 resolves that feature set separately, and the rev gates `std` behind a feature, so a patch that also hits the build-dependency graph fails with pulldown's own `compile_error!`. |
| `crates/codegen/xai-grok-markdown-core/src/lib.rs` | `parser_options()` inserts `Options::ENABLE_CJK_FRIENDLY_EMPHASIS` |
| tests | render-level `cjk_emphasis_tests` (colon, period, and bracket shapes; the ASCII shape stays literal), streaming parity, and pager `cjk_punctuation_before_closer_renders_bold` |

### Revert checklist

1. In the workspace root replace the `[workspace.dependencies]` pulldown-cmark git entry with `pulldown-cmark = "0.13"` (and keep the `LOCAL-PATCH` comment removal in the same change).
2. Keep `Options::ENABLE_CJK_FRIENDLY_EMPHASIS` in `parser_options()`; only the pin goes away.
3. Delete this section.
4. Confirm the chosen release carries both behaviors: `**验证结果：**3 项…` renders bold, and `**$0.06**、…**$0.05**` renders as two bold amounts.

### Notes (local)

- The pinned rev also activates upstream's math digit rule (PR #1098), which is what `LOCAL-PATCH(upstream-dollar-currency-math)` approximated; that patch is tracked separately and can be dropped once the currency tests pass without it.
- Two pulldown-cmark copies coexist in the graph (registry `0.13.x` for `prost-build`, the pinned rev for our crates). No types cross between them: no own crate uses `pulldown-cmark-to-cmark`, and prost-build is a build-dependency with no data flow into our code.

## `upstream-dollar-currency-math` (2026-09-27)

### Problem (present on upstream; not introduced by this fork)

pulldown-cmark's `$` math only checks whitespace adjacency, so it lacks the "a closing `$`
may not be followed by a digit" clause that Pandoc, GitHub, and markdown-it-katex implement.
Two currency amounts on one line therefore pair into a single inline-math span, and every byte
between them — including markdown emphasis markers — is drawn verbatim as math content:

```text
三次合计费用为启用组 **$0.06861016**、关闭组 **$0.05646648**；
```

The span swallows the closing marker of the first `**` pair and the opening marker of the
second, so the message prints `**` literally (the `$` are consumed by the math span instead).

### Local fix (revert when upstream ships)

| Area | Change |
|------|--------|
| `LatexDelimiterNormalizer::prev_byte` | new field: the last consumed source byte, so the rule can see the byte before a held-back `$` |
| `LatexDelimiterNormalizer::inline_math_open` | new field: true while an unescaped single `$` earlier in the paragraph can still open a span |
| single `$` arm in `State::Normal` | escape the `$` as `\$` when it would close a pending span and the next byte is a digit |
| newline arm | a blank line clears `inline_math_open` (pulldown cannot pair a `$` across a paragraph break) |
| fence-open arm | a code fence clears `inline_math_open` |
| `classify_backslash` | `\$` is emitted as a two-byte literal so the currency rule cannot re-escape it |
| tests | normalizer currency / idempotency / byte-split fixtures, render-level `currency_amounts_keep_bold_markers_hidden`, streaming equivalence, pager `currency_amounts_render_without_literal_markers` |

### Upstream equivalent

pulldown-cmark implements only the whitespace half of the Pandoc rule set; the digit clause
belongs in its math delimiter scan (`src/firstpass.rs`, the `b'$'` arm).

### Revert condition

Revert once upstream rejects a closing `$` that is immediately followed by a digit.

### Semantic note (local)

- The escape is inserted before parsing, so the normalized source (`MarkdownContent::text()`,
  which raw-mode copy uses) contains `\$` — valid markdown for a literal `$` — while the
  rendered line shows no backslash.
- Only a `$` that pulldown would use as a *closer* is escaped, so `$1 + x = 2$` and `有$3$个`
  keep rendering as math, and `$x$2` degrades to literal text exactly as it does in Pandoc.

## `upstream-fork-secondary-model` (2026-08-01)

### Problems (present on upstream; not introduced by this fork)

1. **Selected secondary model not applied on fork**  
   Settings UI + config persistence for `[ui].fork_secondary_model` existed,
   but `/fork` and headless fork never passed `newModelId`. Child always kept
   the parent/source model.

2. **Cannot meaningfully select Grok as secondary model**  
   - Empty clear path wrote `default_model()` (`grok-4.5`) to disk.  
   - `current_value_for` folded `== default_model()` to empty → UI showed
     `(no override)`.  
   - Selecting Grok was indistinguishable from “cleared”.

### Local fix (revert when upstream ships)

| Area | Change |
|------|--------|
| `UiConfig::fork_secondary_model` default | `""` (no override), not `default_model()` |
| `set_fork_secondary_model` (shell write) | empty stays empty (no rewrite to default) |
| `current_value_for` | no baseline fold; map stored id → display name via catalog |
| `clear_fork_secondary_model` | mirror `""` |
| `fork_session_params` / `Effect::ForkSession` / headless | pass `newModelId` when configured |
| `resolve_model_name` | also match model ids (defensive) |

### Revert checklist

1. Search `LOCAL-PATCH(upstream-fork-secondary-model)` and remove/restore
   each hunk against upstream.
2. Delete this section (or the whole file if empty).
3. Confirm upstream: selecting Grok sticks in settings **and** forked
   sessions use that model when set.
4. If upstream adds a first-party secondary-effort setting, drop
   `fork_secondary_reasoning_effort` and related wiring.

### Semantic note (local)

- **Empty model** = no override → child keeps source session model.  
- **Any non-empty model id** (including `grok-4.5`) = explicit pin for the
  forked session.
- **Empty effort** = no override → parent/model default effort.  
- **Non-empty effort** = explicit pin; menu is built from the **effective
  secondary model** (override if set, else current session model).  
- Effort also defaults task subagents when role/persona/spawn omit effort.
