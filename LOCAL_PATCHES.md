# Local patches pending upstream

Track temporary fork-only fixes that should be **reverted** when upstream
(`xai-org/grok-build`) lands an equivalent fix. Search the codebase for
`LOCAL-PATCH(upstream-fork-secondary-model)` to find every touch point.

The broader disposition of Fork features (including candidates that should be upstreamed or removed) is tracked in [`FORK_FEATURES.md`](FORK_FEATURES.md). Every remaining code marker must use a patch id documented here; features that are not temporary patches belong in the feature ledger rather than receiving an untracked marker.

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
