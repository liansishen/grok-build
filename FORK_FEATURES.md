# Fork feature disposition

This ledger records why a Fork-only area is kept, reshaped, upstreamed, or removed. It is intentionally separate from `LOCAL_PATCHES.md`: `LOCAL_PATCHES.md` contains temporary patches with a concrete upstream reversion condition, while this file covers the complete Fork feature surface.

## Status meanings

- **P2 rework** — cross-cutting internationalization work; keep the current API until the POC proves a lower-conflict replacement.
- **P3 adapter** — retain the behavior, but move the Fork-specific policy behind an upstream-shaped boundary.
- **Upstream candidate** — isolate as a small patch and offer it upstream; remove the Fork copy after adoption.
- **Keep additive** — retain the feature, but do not reformat or rewrite unrelated upstream code.
- **Remove when absorbed** — re-check against `upstream/main` after every large sync and delete the duplicate implementation when equivalent behavior exists upstream.
- **Temporary patch** — an item must also have a matching `LOCAL-PATCH(<id>)` marker and a section in `LOCAL_PATCHES.md`.

## Inventory

| Area | Primary paths | Current disposition | Next action |
| --- | --- | --- | --- |
| Internationalization runtime and catalogs | `crates/codegen/xai-grok-i18n/`, `scripts/gen_i18n_*.py`, `scripts/merge_i18n_catalogs.py`, `scripts/i18n-opaque.toml` | **P2 rework** | First make catalog updates incremental and deterministic; then run the source-text lookup/boundary-wrapper POC before migrating call sites. |
| User-visible translation call sites | pager, pager-render, tools, workspace, shell, login, update and `ptyctl-cli` crates | **P2 rework** | Migrate by namespace only after the POC; retain `t`/`t_fmt`/`t_or` compatibility while duplicate and placeholder reports are clean. |
| Pager UI additions | `xai-grok-pager/src/views/`, `app/agent_view/`, `app/dispatch/` | **Keep additive** | Keep UI behavior, but isolate new rendering and settings behavior from upstream-owned core functions. |
| Status line, usage and billing | `xai-grok-pager/src/app/status_line*`, `status_blocks.rs`, `app/dispatch/billing.rs`, shell session usage/persistence | **Keep additive** | Keep additive fields narrow; add contract tests for usage and pricing; offer generic improvements upstream where possible. |
| Permission policy and grants | `xai-grok-workspace/src/permission/manager/`, `types.rs`, `grants.rs`, `gate_preflight.rs`, `prompter.rs`, `hub_permission.rs` | **P3 adapter** | Preserve the actor and decision order, but use one `PermissionRequest` boundary and typed policy/transport results. |
| Hook-forced prompts and Hub HITL | `xai-grok-workspace/src/permission/reasons.rs`, `hub_permission.rs`, `prompter.rs` | **P3 adapter** | Keep hook/policy prompts ahead of yolo, grants and auto mode; protect the behavior with security tests. |
| ACP extension metadata | `xai-grok-pager/src/acp/meta.rs`, `acp/mod.rs`, `app/acp_handler/` and `xai-acp-lib/src/message.rs` | **P3 adapter** | Normalize `x.ai/session_notification` and `x.ai/session/update` into one internal envelope without putting Fork policy in the wire layer. |
| Replay, reconnect and prompt ownership | pager `acp/tracker.rs`, `app/acp_handler/session_notification.rs`, `session_load_barrier.rs`, `prompt_ack.rs` | **P3 adapter** | Keep replay/live highwater, adoption and prompt acknowledgement semantics; centralize routing and deduplication. |
| Session load, fork and headless context | pager `app/dispatch/session/`, `app/session_startup.rs`, `headless.rs` | **P3 adapter** | Build one session request context for load, fork, reconnect and headless paths. |
| Secondary model and effort | `xai-grok-shared/src/ui_config.rs`, shell config/settings, pager fork/settings paths | **Temporary patch** | Maintain `upstream-fork-secondary-model` until upstream has equivalent model and effort semantics; follow the reversion checklist. |
| Subagent wake and persistence | `xai-grok-shell/src/agent/subagent/`, `mvp_agent/`, session notification bridge | **Upstream candidate** | Separate regression fixes from product behavior, offer regression fixes upstream, and keep any unavoidable local patch explicitly marked. |
| Human-message interject and queue editing | pager `app/queue_edit.rs`, dispatch queue/interject paths, shell message delivery | **Keep additive / upstream candidate** | Preserve the real user behavior; isolate the queue protocol adapter and upstream generic fixes where possible. |
| Sampler latency semantics | `xai-grok-sampler/src/metrics.rs`, `span_timing.rs`, tests | **Upstream candidate** | Choose the upstream TTFT/TTFB contract, remove duplicate semantics only after contract tests pass. |
| Tool descriptions, reminders and limits | `xai-grok-tools/src/types/`, `reminders/`, media limits and truncation paths | **Keep additive** | Keep dynamic tool/schema text separate from static UI translation; use incremental catalog updates and targeted tests. |
| Features already present upstream | feedback, request metrics, credit bar, subagent takeover, remote settings and theme cache candidates | **Remove when absorbed** | Compare behavior, tests and metadata with `upstream/main`; delete Fork duplicates instead of resolving both implementations forever. |
| User guide and release-facing documentation | `crates/codegen/xai-grok-pager/docs/user-guide/`, `zh-CN/`, shell `CHANGELOG.md` | **Keep additive** | Keep source/translation trees aligned; treat missing translations as an explicit report and preserve the repository changelog rules. |

## Temporary patch registry contract

1. A `LOCAL-PATCH(<id>)` marker is valid only when `<id>` appears as a section in `LOCAL_PATCHES.md`.
2. Each section must list the affected paths, behavior being protected, upstream equivalent or tracking reference, reversion condition, and behavior tests.
3. A patch that becomes part of upstream must be removed from both the code and this registry in the same cleanup change.
4. A Fork feature that is intentionally permanent should be listed in the inventory above but should not receive a temporary marker.
5. The marker audit must fail closed for an unknown id, a missing code marker, or a registry section with no matching marker.

## Review cadence

The upstream arrival time is not controlled by this repository. Re-run this ledger review whenever an upstream batch is received, before resolving conflicts. The review should produce a machine-readable disposition report, but the report itself does not need to be committed when it contains only per-sync counts.
