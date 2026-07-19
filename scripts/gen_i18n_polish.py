#!/usr/bin/env python3
"""Merge polish-pass locale keys (screenshot gap categories)."""

from __future__ import annotations

import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

EN = {
    "ui.search_placeholder": " / to search",
    "usage.weekly_limit": "Weekly limit",
    "usage.monthly_limit": "Monthly limit",
    "usage.usage": "Usage",
    "usage.next_reset": "Next reset: {time}",
    "usage.credits": "Credits: {amount}",
    "usage.auto_topup": "Auto topup: {amount}",
    "usage.max_monthly_topup": "Max monthly topup: {amount}",
    "usage.auto_topup_disabled": "Auto topup: disabled",
    "usage.payg": "Pay-as-you-go: {used} used of {cap} limit",
    "shortcuts.footer.nav": "↑/↓ nav",
    "shortcuts.footer.filter": "f filter",
    "shortcuts.footer.show_all": "f show all",
    "shortcuts.footer.expand": "e/Space/→ expand",
    "shortcuts.footer.collapse": "← collapse",
    "shortcuts.footer.details": "Enter details",
    "shortcuts.footer.search": "/ search",
    "shortcuts.footer.close": "Esc close",
    "shortcuts.footer.esc_back": "Esc back",
    "shortcuts.footer.scroll": "↑/↓ scroll",
    "shortcuts.footer.close_chord": "Ctrl+./X close",
    "settings.max_thoughts_width.label": "Max thoughts width",
    "settings.max_thoughts_width.description": (
        "Column width budget for the agent's thoughts panel (40-500, default 120)."
    ),
    "settings.scroll_mode.choice_auto": "Auto-detect",
    "settings.scroll_mode.choice_auto_desc": (
        "Detect wheel vs trackpad per gesture from event timing. Default."
    ),
    "settings.scroll_mode.choice_wheel": "Mouse wheel",
    "settings.scroll_mode.choice_wheel_desc": (
        "Always treat scrolling as wheel notches (fixed lines per tick)."
    ),
    "settings.scroll_mode.choice_trackpad": "Trackpad",
    "settings.scroll_mode.choice_trackpad_desc": (
        "Always treat scrolling as a trackpad (fractional accumulation)."
    ),
    "settings.keep_text_selection.choice_flash": "Flash after copy",
    "settings.keep_text_selection.choice_flash_desc": (
        "Brief highlight on mouse-up, then clear. Double-click toggles fold. Default."
    ),
    "settings.keep_text_selection.choice_hold": "Hold until dismissed",
    "settings.keep_text_selection.choice_hold_desc": (
        "Keep the selection visible until Esc, click, or scroll. Double-click toggles fold."
    ),
    "settings.keep_text_selection.choice_word_select": "Word select (terminal-like)",
    "settings.keep_text_selection.choice_word_select_desc": (
        "Double-click selects & copies a word, triple-click a line; selection stays until dismissed."
    ),
    "settings.default_selected_permission.choice_always_allow_all_sessions": (
        "Always allow on all sessions"
    ),
    "settings.default_selected_permission.choice_allow_once": "Allow once",
    "settings.default_selected_permission.choice_allow_command_always": (
        "Always allow this command"
    ),
    "settings.default_selected_permission.choice_reject": "Reject",
    "settings.render_mermaid.choice_auto": "Auto",
    "settings.render_mermaid.choice_on": "On",
    "settings.render_mermaid.choice_off": "Off",
    "toast.permission_mode_auto": "✓ Permission mode: Auto (classifier)",
    "toast.permission_mode_ask": "✓ Permission mode: Ask",
    "toast.permission_mode_default": "✓ Permission mode: Default",
    "toast.always_approve_on": "⚠ Always-approve ON",
    "toast.always_approve_on_plan": "⚠ Always-approve ON (plan mode still blocks edits)",
    "toast.always_approve": "Always-approve",
    "hint.queue": "queue",
    "hint.send": "send",
    "hint.expand": "expand",
    "hint.send_now": "send now",
    "hint.newline": "newline",
    "hint.lines": "lines",
    "hint.accept_suggestion": "accept suggestion",
    "hint.mode": "mode",
    "hint.delete_row": "delete row",
    "hint.edit": "edit",
    "hint.copy": "copy",
    "hint.save": "save",
    "hint.cancel": "cancel",
    "hint.select": "select",
    "hint.view": "view",
    "hint.copy_output": "copy output",
    "hint.kill": "kill",
    "hint.go": "go",
    "hint.open": "open",
    "hint.send_to_bg": "send to bg",
    "hint.nav": "nav",
    "hint.page": "page",
    "hint.prompt": "prompt",
    "hint.turn": "turn",
    "hint.reorder": "reorder",
    "hint.hide_done": "hide done",
    "hint.show_done": "show done",
    "hint.next_prev": "next/prev",
    "toast.restart_to_apply": "(restart to apply)",
    "toast.mermaid": "✓ Mermaid: {value}",
    "modal.reset_setting_prompt": "Reset '{label}' to default ({value})?",
    "modal.reset_setting_breadcrumb": "Reset '{label}'",
    "settings.hunk_tracker_mode.choice_agent_only": "Agent only",
    "settings.hunk_tracker_mode.choice_agent_only_desc": (
        "Track only files the agent edits (default)."
    ),
    "settings.hunk_tracker_mode.choice_all_dirty": "All dirty",
    "settings.hunk_tracker_mode.choice_all_dirty_desc": (
        "Track every git-dirty file, including external edits."
    ),
    "settings.hunk_tracker_mode.choice_off": "Off",
    "settings.hunk_tracker_mode.choice_off_desc": (
        "Disable hunk tracking entirely. Also disables LOC tracking."
    ),
    "settings.voice_capture_mode.choice_toggle": "Toggle",
    "settings.voice_capture_mode.choice_toggle_desc": (
        "Ctrl+Space / F8 starts dictation; press again (or Esc/Enter) to stop."
    ),
    "settings.voice_capture_mode.choice_hold": "Hold to talk",
    "settings.voice_capture_mode.choice_hold_desc": (
        "Hold Ctrl+Space / F8 to record, release to stop. Needs a Kitty-protocol terminal."
    ),
    "settings.coding_data_sharing.choice_opt_in": "Opt in",
    "settings.coding_data_sharing.choice_opt_in_desc": (
        "Allow SpaceXAI to retain coding session data for model training and product improvement."
    ),
    "settings.coding_data_sharing.choice_opt_out": "Opt out",
    "settings.coding_data_sharing.choice_opt_out_desc": (
        "Do not retain coding session data for training. Does not disable product analytics."
    ),
    "settings.theme.choice_auto": "Auto",
    "settings.theme.choice_groknight": "Grok Night",
    "settings.theme.choice_grokday": "Grok Day",
    "settings.theme.choice_tokyonight": "Tokyo Night",
    "settings.theme.choice_rosepine_moon": "Rose Pine Moon",
    "settings.theme.choice_oscura_midnight": "Oscura Midnight",
    "settings.voice_stt_language.choice_auto": "System",
    "settings.render_mermaid.choice_auto_desc": (
        "Show diagrams with a clickable row to open/copy the rendered image."
    ),
    "settings.render_mermaid.choice_on_desc": (
        "Same as auto: always show the clickable affordance row."
    ),
    "settings.render_mermaid.choice_off_desc": (
        "Always show the raw Mermaid source as a code block."
    ),
    "slash.usage.arg_show": "View credit usage",
    "slash.usage.arg_manage": "Open billing management page",
    "actions.SendPrompt.description": "Send",
    "actions.FocusPrompt.description": "Focus prompt",
    "actions.CancelTurn.description": "Cancel turn",
    "actions.CycleMode.description": "Cycle mode (Normal / Plan / Always-approve)",
    "actions.Quit.description": "Quit",
    "actions.CommandPalette.description": "Command palette",
    "actions.ShortcutsHelp.description": "Keyboard shortcuts",
    "actions.OpenSettings.description": "Open the settings modal",
    "actions.FocusScrollback.description": "Focus scrollback",
    "actions.ToggleYolo.description": "Toggle always-approve",
    "actions.ToggleMultiline.description": "Toggle multiline",
    "actions.NewSession.description": "New session",
    "actions.OpenDashboard.description": "Open the Agent Dashboard",
    "actions.ModelPicker.description": "Pick model",
    "actions.SelectNext.description": "Select next entry",
    "actions.SelectPrev.description": "Select previous entry",
    "actions.ScrollUp.description": "Scroll up one line",
    "actions.ScrollDown.description": "Scroll down one line",
    "actions.CopyBlockContent.description": "Copy content",
    "actions.ToggleFold.description": "Expand / collapse",
    "actions.Rewind.description": "Rewind to selected turn",
    "actions.ToggleTodos.description": "Toggle todo pane",
    "actions.ToggleTasks.description": "Toggle tasks pane",
    "actions.ToggleQueue.description": "Toggle prompt queue",
    "actions.OpenSessions.description": "Open sessions",
    "actions.InterjectPrompt.description": "Send now while running (cancels the current turn)",
    "actions.SendToBackground.description": "Send running task to background",
    "actions.VoiceToggle.description": "Voice dictation (Ctrl+Space / F8)",
    "shortcuts.pseudo.search_scrollback": "Search scrollback",
    "shortcuts.pseudo.paste": "Paste images (and text) from the clipboard",
}

ZH = {
    "ui.search_placeholder": " / 搜索",
    "usage.weekly_limit": "每周限额",
    "usage.monthly_limit": "每月限额",
    "usage.usage": "用量",
    "usage.next_reset": "下次重置：{time}",
    "usage.credits": "余额：{amount}",
    "usage.auto_topup": "自动充值：{amount}",
    "usage.max_monthly_topup": "每月充值上限：{amount}",
    "usage.auto_topup_disabled": "自动充值：已关闭",
    "usage.payg": "按量付费：已用 {used} / 上限 {cap}",
    "shortcuts.footer.nav": "↑/↓ 导航",
    "shortcuts.footer.filter": "f 筛选",
    "shortcuts.footer.show_all": "f 显示全部",
    "shortcuts.footer.expand": "e/Space/→ 展开",
    "shortcuts.footer.collapse": "← 折叠",
    "shortcuts.footer.details": "Enter 详情",
    "shortcuts.footer.search": "/ 搜索",
    "shortcuts.footer.close": "Esc 关闭",
    "shortcuts.footer.esc_back": "Esc 返回",
    "shortcuts.footer.scroll": "↑/↓ 滚动",
    "shortcuts.footer.close_chord": "Ctrl+./X 关闭",
    "settings.max_thoughts_width.label": "思考区最大宽度",
    "settings.max_thoughts_width.description": "智能体思考面板的列宽预算（40–500，默认 120）。",
    "settings.scroll_mode.choice_auto": "自动检测",
    "settings.scroll_mode.choice_auto_desc": "根据事件时序自动区分滚轮与触控板。默认。",
    "settings.scroll_mode.choice_wheel": "鼠标滚轮",
    "settings.scroll_mode.choice_wheel_desc": "始终按滚轮刻度滚动（每档固定行数）。",
    "settings.scroll_mode.choice_trackpad": "触控板",
    "settings.scroll_mode.choice_trackpad_desc": "始终按触控板方式滚动（分数累加）。",
    "settings.keep_text_selection.choice_flash": "复制后闪一下",
    "settings.keep_text_selection.choice_flash_desc": "鼠标松开后短暂高亮再清除。双击切换折叠。默认。",
    "settings.keep_text_selection.choice_hold": "保持到取消",
    "settings.keep_text_selection.choice_hold_desc": "选区保持到 Esc、点击或滚动。双击切换折叠。",
    "settings.keep_text_selection.choice_word_select": "词选择（类终端）",
    "settings.keep_text_selection.choice_word_select_desc": "双击选中并复制词，三击选行；选区保持到取消。",
    "settings.default_selected_permission.choice_always_allow_all_sessions": "所有会话始终允许",
    "settings.default_selected_permission.choice_allow_once": "允许一次",
    "settings.default_selected_permission.choice_allow_command_always": "始终允许此命令",
    "settings.default_selected_permission.choice_reject": "拒绝",
    "settings.render_mermaid.choice_auto": "自动",
    "settings.render_mermaid.choice_on": "开",
    "settings.render_mermaid.choice_off": "关",
    "toast.permission_mode_auto": "✓ 权限模式：自动",
    "toast.permission_mode_ask": "✓ 权限模式：询问",
    "toast.permission_mode_default": "✓ 权限模式：默认",
    "toast.always_approve_on": "⚠ 始终批准：开",
    "toast.always_approve_on_plan": "⚠ 始终批准：开（计划模式仍拦编辑）",
    "toast.always_approve": "始终批准",
    "hint.queue": "排队",
    "hint.send": "发送",
    "hint.expand": "展开",
    "hint.send_now": "立即发送",
    "hint.newline": "换行",
    "hint.lines": "行号",
    "hint.accept_suggestion": "接受建议",
    "hint.mode": "模式",
    "hint.delete_row": "删除行",
    "hint.edit": "编辑",
    "hint.copy": "复制",
    "hint.save": "保存",
    "hint.cancel": "取消",
    "hint.select": "选择",
    "hint.view": "查看",
    "hint.copy_output": "复制输出",
    "hint.kill": "终止",
    "hint.go": "跳转",
    "hint.open": "打开",
    "hint.send_to_bg": "转后台",
    "hint.nav": "导航",
    "hint.page": "翻页",
    "hint.prompt": "输入",
    "hint.turn": "回合",
    "hint.reorder": "重排",
    "hint.hide_done": "隐藏已完成",
    "hint.show_done": "显示已完成",
    "hint.next_prev": "下/上一项",
    "toast.restart_to_apply": "（需重启生效）",
    "toast.mermaid": "✓ Mermaid: {value}",
    "modal.reset_setting_prompt": "将「{label}」重置为默认（{value}）？",
    "modal.reset_setting_breadcrumb": "重置「{label}」",
    "settings.hunk_tracker_mode.choice_agent_only": "仅智能体",
    "settings.hunk_tracker_mode.choice_agent_only_desc": "仅跟踪智能体编辑的文件（默认）。",
    "settings.hunk_tracker_mode.choice_all_dirty": "全部脏文件",
    "settings.hunk_tracker_mode.choice_all_dirty_desc": "跟踪所有 git 脏文件，含外部修改。",
    "settings.hunk_tracker_mode.choice_off": "关",
    "settings.hunk_tracker_mode.choice_off_desc": "完全禁用 hunk 跟踪（以及 LOC 统计）。",
    "settings.voice_capture_mode.choice_toggle": "切换",
    "settings.voice_capture_mode.choice_toggle_desc": (
        "Ctrl+Space / F8 开始听写；再按一次（或 Esc/Enter）停止。"
    ),
    "settings.voice_capture_mode.choice_hold": "按住说话",
    "settings.voice_capture_mode.choice_hold_desc": (
        "按住 Ctrl+Space / F8 录音，松开停止。需要 Kitty 协议终端。"
    ),
    "settings.coding_data_sharing.choice_opt_in": "加入",
    "settings.coding_data_sharing.choice_opt_in_desc": (
        "允许 SpaceXAI 保留编码会话数据用于模型训练与产品改进。"
    ),
    "settings.coding_data_sharing.choice_opt_out": "退出",
    "settings.coding_data_sharing.choice_opt_out_desc": (
        "不保留编码会话数据用于训练。不影响产品分析。"
    ),
    "settings.theme.choice_auto": "自动",
    "settings.theme.choice_groknight": "Grok Night",
    "settings.theme.choice_grokday": "Grok Day",
    "settings.theme.choice_tokyonight": "Tokyo Night",
    "settings.theme.choice_rosepine_moon": "Rose Pine Moon",
    "settings.theme.choice_oscura_midnight": "Oscura Midnight",
    "settings.voice_stt_language.choice_auto": "跟随系统",
    "settings.render_mermaid.choice_auto_desc": "显示图表，并提供可点击行以打开/复制渲染图。",
    "settings.render_mermaid.choice_on_desc": "与自动相同：始终显示可点击操作行。",
    "settings.render_mermaid.choice_off_desc": "始终以代码块显示原始 Mermaid 源码。",
    "slash.usage.arg_show": "查看额度用量",
    "slash.usage.arg_manage": "打开账单管理页",
    "actions.SendPrompt.description": "发送",
    "actions.FocusPrompt.description": "聚焦输入",
    "actions.CancelTurn.description": "取消回合",
    "actions.CycleMode.description": "切换模式（普通 / 计划 / 始终批准）",
    "actions.Quit.description": "退出",
    "actions.CommandPalette.description": "命令面板",
    "actions.ShortcutsHelp.description": "键盘快捷键",
    "actions.OpenSettings.description": "打开设置",
    "actions.FocusScrollback.description": "聚焦回滚",
    "actions.ToggleYolo.description": "切换始终批准",
    "actions.ToggleMultiline.description": "切换多行输入",
    "actions.NewSession.description": "新会话",
    "actions.OpenDashboard.description": "打开智能体仪表盘",
    "actions.ModelPicker.description": "选择模型",
    "actions.SelectNext.description": "选择下一项",
    "actions.SelectPrev.description": "选择上一项",
    "actions.ScrollUp.description": "向上滚动一行",
    "actions.ScrollDown.description": "向下滚动一行",
    "actions.CopyBlockContent.description": "复制内容",
    "actions.ToggleFold.description": "展开 / 折叠",
    "actions.Rewind.description": "回退到选中回合",
    "actions.ToggleTodos.description": "切换待办面板",
    "actions.ToggleTasks.description": "切换任务面板",
    "actions.ToggleQueue.description": "切换提示队列",
    "actions.OpenSessions.description": "打开会话列表",
    "actions.InterjectPrompt.description": "运行中立即发送（取消当前回合）",
    "actions.SendToBackground.description": "将运行中任务转到后台",
    "actions.VoiceToggle.description": "语音听写（Ctrl+Space / F8）",
    "shortcuts.pseudo.search_scrollback": "搜索回滚",
    "shortcuts.pseudo.paste": "从剪贴板粘贴图片（与文本）",
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
