#!/usr/bin/env python3
"""Merge mode-banner / prompt-flag / session chrome locale keys."""

from __future__ import annotations

import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

EN = {
    "mode.switched": "Switched to mode: {mode}",
    "mode.name.plan": "Plan",
    "mode.name.auto": "Auto",
    "mode.name.always_approve": "Always-Approve",
    "mode.name.normal": "Normal",
    "mode.flag.always_approve": "always-approve",
    "mode.flag.auto": "auto",
    "mode.flag.plan": "plan",
    "mode.flag.plan_approval": "plan approval",
    "mode.flag.commenting": "commenting",
    "mode.flag.commenting_line": "commenting L{n}",
    "mode.flag.commenting_range": "commenting L{start}-{end}",
    "prompt.placeholder.build_anything": "Build anything",
    "session.worked_for": "Worked for {duration}",
    "session.turn_completed": "Turn completed.",
    "session.turn_cancelled": "Turn cancelled by user in {duration}.",
    "session.turn_halted": (
        "Agent was unable to make progress \u2014 turn ended in {duration}."
    ),
    "session.turn_failed_in": "Turn failed in {duration}: {error}",
    "session.turn_failed": "Turn failed: {error}",
    "session.compaction_started": "Context {percentage}% full. Compacting\u2026",
    "session.compaction_completed_range": "Context compacted: {before} \u2192 {after} tokens",
    "session.compaction_completed": "Context compacted \u2192 {after} tokens",
    "session.compaction_completed_with_time": "{body} ({secs}s)",
    "session.compaction_failed": "Compaction failed.",
    "session.compaction_failed_error": "Compaction failed: {error}",
    "session.compaction_cancelled": "Compaction cancelled.",
    "session.compact_completed": "Compaction completed in {duration}.",
    "session.retry_failed": "Retry failed: {error}",
    "session.retry_encrypted_mismatch": (
        "This session's conversation history is incompatible with the "
        "current model. Please start a new session."
    ),
    "session.reauth_required": (
        "Authentication required \u2014 your session has expired or your "
        "credentials were rejected. Run /login to re-authenticate, then resend "
        "your message."
    ),
    "session.context_too_large": (
        "This conversation is too large for the model's context window. "
        "Use /new to start a new session."
    ),
    "session.model_switched": '{reason} Switched to "{model}".',
    "session.memory_saved": "Memory saved ({trigger}) \u2192 {path}  \u00b7  /memory to view",
    "session.goal_complete": "Goal complete \u2014 {duration} end-to-end.",
    "actions.ToggleYolo.label": "always-approve",
    "actions.DashboardToggleAutoApprove.label": "always-approve",
    "actions.DashboardToggleAutoApprove.description": "Toggle always-approve",
    "toast.mouse_reporting_off_scrollback": (
        "Ctrl+r to enable mouse reporting and restore TUI features"
    ),
    "toast.mouse_reporting_off_prompt": (
        "/toggle-mouse-reporting to enable mouse reporting and restore TUI features"
    ),
}

ZH = {
    "mode.switched": "已切换模式：{mode}",
    "mode.name.plan": "计划",
    "mode.name.auto": "自动",
    "mode.name.always_approve": "始终批准",
    "mode.name.normal": "普通",
    "mode.flag.always_approve": "始终批准",
    "mode.flag.auto": "自动",
    "mode.flag.plan": "计划",
    "mode.flag.plan_approval": "计划审批",
    "mode.flag.commenting": "评论中",
    "mode.flag.commenting_line": "评论 L{n}",
    "mode.flag.commenting_range": "评论 L{start}-{end}",
    "prompt.placeholder.build_anything": "随便写点什么",
    "session.worked_for": "工作了 {duration}",
    "session.turn_completed": "回合已完成。",
    "session.turn_cancelled": "用户在 {duration} 后取消了回合。",
    "session.turn_halted": "智能体无法继续推进 — 回合在 {duration} 后结束。",
    "session.turn_failed_in": "回合在 {duration} 后失败：{error}",
    "session.turn_failed": "回合失败：{error}",
    "session.compaction_started": "上下文已用 {percentage}%。正在压缩…",
    "session.compaction_completed_range": "上下文已压缩：{before} → {after} tokens",
    "session.compaction_completed": "上下文已压缩 → {after} tokens",
    "session.compaction_completed_with_time": "{body}（{secs}s）",
    "session.compaction_failed": "压缩失败。",
    "session.compaction_failed_error": "压缩失败：{error}",
    "session.compaction_cancelled": "压缩已取消。",
    "session.compact_completed": "压缩在 {duration} 内完成。",
    "session.retry_failed": "重试失败：{error}",
    "session.retry_encrypted_mismatch": (
        "此会话的对话历史与当前模型不兼容。请开始新会话。"
    ),
    "session.reauth_required": (
        "需要重新认证 — 会话已过期或凭据被拒绝。"
        "请运行 /login 重新登录，然后重新发送消息。"
    ),
    "session.context_too_large": (
        "此对话过大，超出模型上下文窗口。请使用 /new 开始新会话。"
    ),
    "session.model_switched": "{reason} 已切换到「{model}」。",
    "session.memory_saved": "记忆已保存（{trigger}） → {path}  ·  /memory 查看",
    "session.goal_complete": "目标完成 — 端到端 {duration}。",
    "actions.ToggleYolo.label": "始终批准",
    "actions.DashboardToggleAutoApprove.label": "始终批准",
    "actions.DashboardToggleAutoApprove.description": "切换始终批准",
    "toast.mouse_reporting_off_scrollback": "Ctrl+R 开启鼠标报告以恢复 TUI 功能",
    "toast.mouse_reporting_off_prompt": "/toggle-mouse-reporting 开启鼠标报告以恢复 TUI 功能",
}


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def flat_section(d: dict[str, str]) -> str:
    tables: dict[tuple[str, ...], dict[str, str]] = {}
    for k, v in d.items():
        *path, last = k.split(".")
        tables.setdefault(tuple(path), {})[last] = v
    out: list[str] = []
    for path, fields in sorted(tables.items(), key=lambda x: x[0]):
        out.append(f"[{'.'.join(path)}]")
        for fk, fv in fields.items():
            out.append(f'{fk} = "{esc(fv)}"')
        out.append("")
    return "\n".join(out)


def flatten(node: object, prefix: str = "") -> dict[str, str]:
    out: dict[str, str] = {}
    if isinstance(node, dict):
        for k, v in node.items():
            p = f"{prefix}.{k}" if prefix else str(k)
            if isinstance(v, dict):
                out.update(flatten(v, p))
            else:
                out[p] = str(v)
    return out


def merge(path: Path, extra: dict[str, str]) -> None:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    all_keys = flatten(data)
    before = len(all_keys)
    all_keys.update(extra)
    path.write_text(
        "# Locale catalog (merged)\n" + flat_section(all_keys), encoding="utf-8"
    )
    tomllib.loads(path.read_text(encoding="utf-8"))
    print(f"{path.name}: {len(all_keys)} keys (+{len(all_keys) - before})")


def main() -> None:
    merge(ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml", EN)
    merge(ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml", ZH)


if __name__ == "__main__":
    main()
