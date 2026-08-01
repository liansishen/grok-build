# Local patches pending upstream

Track temporary fork-only fixes that should be **reverted** when upstream
(`xai-org/grok-build`) lands an equivalent fix. Search the codebase for
`LOCAL-PATCH(upstream-fork-secondary-model)` to find every touch point.

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
