#!/usr/bin/env python3
"""Generate Phase 1 en.toml / zh-CN.toml catalogs from source strings."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def flat_kv_section(d: dict[str, str]) -> str:
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


def parse_settings() -> list[tuple[str, str, str]]:
    text = (ROOT / "crates/codegen/xai-grok-pager/src/settings/defs.rs").read_text(
        encoding="utf-8"
    )
    blocks = re.split(r"SettingMeta\s*\{", text)[1:]
    settings: list[tuple[str, str, str]] = []
    for b in blocks:
        key_m = re.search(r'key:\s*"([^"]+)"', b)
        label_m = re.search(r'label:\s*"((?:\\.|[^"])*)"', b)
        desc_m = re.search(
            r'description:\s*((?:"(?:\\.|[^"])*"\s*\\?\s*)+)', b
        )
        if not key_m or not label_m:
            continue
        key = key_m.group(1)
        label = label_m.group(1).replace('\\"', '"')
        desc = ""
        if desc_m:
            parts = re.findall(r'"((?:\\.|[^"])*)"', desc_m.group(1))
            desc = re.sub(r"\s+", " ", "".join(parts)).strip().replace('\\"', '"')
        settings.append((key, label, desc))
    return settings


def parse_slash() -> dict[str, str]:
    """Parse slash descriptions from current tree or git revision with English literals."""
    import subprocess

    slash_dir = ROOT / "crates/codegen/xai-grok-pager/src/slash/commands"
    out: dict[str, str] = {}

    def extract(content: str) -> None:
        names = re.findall(r'fn name\(&self\)[^{]*\{\s*"([^"]+)"', content)
        descs = re.findall(
            r'fn description\(&self\)[^{]*\{\s*"((?:\\.|[^"])*)"', content
        )
        for n, d in zip(names, descs):
            out[f"slash.{n}.description"] = d.replace('\\"', '"')

    for f in slash_dir.glob("*.rs"):
        extract(f.read_text(encoding="utf-8"))
    if out:
        return out
    # Descriptions already use t(); recover English from commit before i18n slash patch.
    for rev in ("1baf29b", "HEAD~5"):
        try:
            listing = subprocess.check_output(
                [
                    "git",
                    "ls-tree",
                    "-r",
                    "--name-only",
                    rev,
                    "crates/codegen/xai-grok-pager/src/slash/commands",
                ],
                cwd=ROOT,
                text=True,
            )
        except subprocess.CalledProcessError:
            continue
        for rel in listing.splitlines():
            if not rel.endswith(".rs"):
                continue
            try:
                content = subprocess.check_output(
                    ["git", "show", f"{rev}:{rel}"], cwd=ROOT, text=True
                )
            except subprocess.CalledProcessError:
                continue
            extract(content)
        if out:
            break
    return out


def parse_actions() -> dict[str, str]:
    act = (
        ROOT / "crates/codegen/xai-grok-pager/src/actions/defaults.rs"
    ).read_text(encoding="utf-8")
    out: dict[str, str] = {}
    for b in re.split(r"ActionDef\s*\{", act)[1:]:
        id_m = re.search(r"id:\s*ActionId::(\w+)", b)
        lab_m = re.search(r'label:\s*"((?:\\.|[^"])*)"', b)
        des_m = re.search(r'description:\s*"((?:\\.|[^"])*)"', b)
        if not id_m or not lab_m:
            continue
        aid = id_m.group(1)
        out[f"actions.{aid}.label"] = lab_m.group(1)
        if des_m:
            out[f"actions.{aid}.description"] = des_m.group(1)
    return out


ZH_SETTINGS = {
    "compact_mode": (
        "紧凑模式",
        "减少消息周围的留白以提高信息密度。终端高度不超过 20 行时会自动启用。",
    ),
    "screen_mode": (
        "默认界面模式",
        "下次启动 plain grok 时的模式：全屏（未设置时的默认）或极简。写入 config.toml 的 [ui] screen_mode。需重启。本会话可用 /minimal 或 /fullscreen 切换。",
    ),
    "show_timestamps": ("显示时间戳", "在用户消息和智能体回复旁显示时钟时间。"),
    "show_timeline": ("时间线侧栏", "用每回合刻度条替代滚动条：悬停预览回合，点击跳转。"),
    "page_flip_on_send": (
        "发送时将提示滚到顶部",
        "发送提示时将其滚到屏幕顶部，使回复从新页开始（默认）。关闭后发送时不改变滚动位置。",
    ),
    "simple_mode": ("禁用 vim 输入模式", "在输入框使用普通 readline 风格输入，而非 vim 键。实验性。"),
    "vim_mode": ("Vim 回滚导航", "使用 vim 键（h/j/k/l、gg/G、/）浏览回滚区。不影响输入框。"),
    "language": (
        "界面语言",
        "Grok Build 界面显示语言。选择「跟随系统」时，在检测到中文或英文系统语言后自动切换。",
    ),
    "theme": ("主题", "分页器界面的配色主题。"),
    "auto_dark_theme": ("自动深色主题", "系统为深色模式时使用的主题（仅在 theme=auto 时生效）。"),
    "auto_light_theme": ("自动浅色主题", "系统为浅色模式时使用的主题（仅在 theme=auto 时生效）。"),
    "render_mermaid": (
        "渲染 Mermaid 图",
        "如何显示 mermaid 代码块：auto/on 显示可点击行以打开渲染图；off 显示原始源码。",
    ),
    "permission_mode": (
        "权限模式",
        "Default 使用智能体内置行为；Ask 在每次工具操作前询问；Auto 用 LLM 分类器审批风险操作；Always approve 自动授予全部权限。",
    ),
    "remember_tool_approvals": (
        "记住工具批准",
        "在权限提示中显示「始终允许」选项，避免对同一命令/工具反复确认。适用于 ask 与 auto；Always-approve 仍会跳过全部提示。需重启。",
    ),
    "multiline_mode": ("多行输入", "开启后 Enter 换行、Shift+Enter 发送。每次会话重置。"),
    "default_model": ("默认模型", "新会话使用的模型。更改也会切换当前会话。选择「(不覆盖)」可清除。"),
    "show_thinking_blocks": ("显示思考块", "流式输出时在回滚中显示智能体的思考/推理块。"),
    "prompt_suggestions": (
        "提示建议",
        "每回合结束后预测你可能的下一条提示，并以幽灵文本显示（Tab 接受）。每回合会调用小模型。",
    ),
    "respect_manual_folds": ("尊重手动折叠", "流式输出时保留手动折叠的块，展开块时停止自动滚动。实验性。"),
    "group_tool_verbs": (
        "合并工具调用",
        "将连续的读取/搜索/列表工具调用与子智能体行折叠为一行摘要；完成的思考也会并入组。",
    ),
    "collapsed_edit_blocks": (
        "折叠编辑块",
        "以单行 +N/-M 差异统计显示编辑，并将同一文件的连续编辑合并；展开行可查看差异。",
    ),
    "display_refresh_auto_cadence": (
        "匹配显示器刷新率",
        "在高刷新率显示器上，TUI 会以更快节奏流式输出/滚动。关闭则保持约 60 Hz。需重启。",
    ),
    "scroll_speed": ("滚动速度", "鼠标滚轮与触控板滚动速度倍率（1–100）。越高越快。"),
    "scroll_mode": ("滚动输入", "在自动检测误判设备时，强制按滚轮或触控板行为滚动。"),
    "scroll_lines": ("滚动行数", "滚轮与触控板每次滚动的行数（1–10）。未设置前沿用各终端自带配置。"),
    "invert_scroll": ("反转滚动", "反转垂直滚动方向（自然滚动）。"),
    "keep_text_selection": (
        "文本选择",
        "应用内选区保留多久，以及双击是折叠还是选中并复制单词。终端原生选择请按住 Shift 拖拽。",
    ),
    "coding_data_sharing": (
        "编码数据共享",
        "控制 SpaceXAI 是否可保留并用于训练编码会话数据。不影响产品分析。",
    ),
    "default_selected_permission": ("默认选中的权限项", "权限提示中光标预选中的行。"),
    "toolset.ask_user_question.timeout_enabled": (
        "提问超时",
        "开启后，ask_user_question 工具会在设定时间后超时，而不是无限阻塞。",
    ),
    "plan_mode": ("计划模式", "开启后，智能体在运行工具或修改文件前会先汇总计划。"),
    "show_tips": ("显示提示", "启动时显示每日提示横幅。需重启。"),
    "contextual_hints": ("显示情境提示", "工作时显示简短的情境快捷键提示；可分别开关每一项。"),
    "auto_update": ("自动更新", "启动时自动下载并安装分页器更新。需重启。"),
    "hunk_tracker_mode": (
        "Hunk 跟踪",
        "智能体跟踪哪些文件变更。Off 完全禁用跟踪（以及 LOC 统计）。需重启。",
    ),
    "voice_capture_mode": ("语音采集", "语音快捷键（Ctrl+Space / F8）行为：切换或按住说话。"),
    "voice_stt_language": (
        "语音语言",
        "语音听写的识别语言（Grok STT）。默认英语；「跟随系统」在受支持时使用系统语言。",
    ),
    "contextual_hints.undo": ("撤销", "清空提示后提醒可用 Ctrl+Z 恢复。"),
    "contextual_hints.plan_mode": ("计划模式", "当提示像是规划请求时，建议使用计划模式（Shift+Tab）。"),
    "contextual_hints.image_input": ("图片输入", "剪贴板有图片且模型支持时，提示可粘贴图片。"),
    "contextual_hints.send_now": (
        "立即发送",
        "回合中排队后续消息后，提醒在空提示上按 Enter 可立即发送队首项。",
    ),
    "contextual_hints.small_screen": ("小屏幕", "终端行数较少时建议使用 /compact-mode。"),
    "contextual_hints.word_select": ("词选择", "双击对话文本后，提醒可在设置中启用词选择。"),
    "contextual_hints.ssh_wrap": ("SSH 包装", "经 SSH 加载会话时，建议本地运行 grok wrap ssh。"),
    "fork_secondary_model": ("分叉次级模型", "分叉时次级智能体使用的模型。选择「(不覆盖)」可清除。"),
    "max_thoughts_width": ("思考区最大宽度", "智能体思考面板的列宽预算（40–500，默认 120）。"),
}

SLASH_ZH = {
    "always-approve": "切换始终批准模式（跳过全部权限提示）",
    "announcements": "显示或隐藏公告",
    "auto": "切换自动模式（分类器批准安全工具）",
    "btw": "提出旁路问题而不打断当前回合",
    "cd": "更改新智能体的工作目录",
    "compact": "压缩对话历史",
    "compact-mode": "切换紧凑界面（更少留白、更多内容）",
    "config-agents": "管理智能体定义",
    "context": "查看上下文用量",
    "copy": "将最近回复复制到剪贴板（/copy N 表示第 N 近）",
    "dashboard": "打开智能体仪表盘 — 全屏查看所有运行中的会话",
    "debug": "切换调试叠加层",
    "docs": "打开使用指南或在线 Build 文档",
    "effort": "设置当前模型的推理强度",
    "quit": "退出应用",
    "expand": "完整展开打印最近折叠的块（极简模式）",
    "export": "将当前对话导出到文件或剪贴板",
    "feedback": "发送关于当前会话的反馈",
    "find": "搜索对话回滚",
    "fork": "将当前会话分支为对等智能体",
    "help": "浏览命令与键盘快捷键",
    "history": "搜索提示历史",
    "home": "返回欢迎页",
    "import-claude": "打开 Claude 设置导入对话框",
    "jump": "跳转到对话中的某个回合",
    "login": "登录或重新认证账号",
    "logout": "登出并返回登录页",
    "loop": "按间隔循环运行提示",
    "mcps": "显示 MCP 服务器状态",
    "model": "切换活动模型",
    "multiline": "切换多行输入模式（交换 Enter 与 Shift+Enter）",
    "new": "开始新会话",
    "personas": "管理角色（创建、编辑、删除）",
    "plan": "进入计划模式",
    "hooks": "查看 hooks",
    "plugins": "查看插件",
    "marketplace": "查看市场",
    "skills": "查看技能",
    "privacy": "显示或切换隐私与数据保留状态",
    "queue": "列出当前回合后排队的提示",
    "recap": "总结目前为止的会话",
    "release-notes": "查看当前版本的发行说明",
    "remember": "保存一条记忆笔记",
    "rename": "重命名当前会话",
    "resume": "恢复先前会话",
    "rewind": "回退到先前回合",
    "session-info": "显示会话信息",
    "settings": "打开设置面板",
    "share": "通过 URL 分享此会话",
    "tasks": "列出后台任务、子智能体与定时任务",
    "terminal-setup": "检查终端、颜色与剪贴板设置",
    "theme": "切换配色主题",
    "timeline": "切换时间线侧栏",
    "timestamps": "切换消息时间戳",
    "toggle-mouse-reporting": "切换终端鼠标报告（原生拖选复制）",
    "transcript": "在分页器（$PAGER）中查看完整对话记录",
    "usage": "查看用量或管理账单",
    "view-plan": "查看当前计划",
    "vim-mode": "切换 vim 风格回滚快捷键",
}

ACTION_ZH_LABEL = {
    "SendPrompt": "发送",
    "Quit": "退出",
    "NewSession": "新会话",
    "CancelTurn": "取消",
    "OpenSettings": "设置",
    "CommandPalette": "命令",
    "ShortcutsHelp": "快捷键",
    "ModelPicker": "模型",
    "ToggleMultiline": "多行",
    "CycleMode": "模式",
    "ToggleYolo": "始终批准",
    "FocusPrompt": "提示",
    "FocusScrollback": "回滚",
    "ToggleTodos": "待办",
    "ToggleTasks": "任务",
    "ToggleQueue": "队列",
    "OpenDashboard": "仪表盘",
    "SelectNext": "导航",
    "SelectPrev": "导航",
    "ScrollUp": "上滚",
    "ScrollDown": "下滚",
    "CopyBlockContent": "复制",
    "ToggleFold": "折叠",
    "Rewind": "回退",
    "OpenSessions": "会话",
    "VoiceToggle": "麦克风",
    "EnableVoiceMode": "语音模式",
    "InterjectPrompt": "立即发送",
    "SendToBackground": "转后台",
    "DashboardExit": "退出",
    "DashboardStop": "停止",
    "OpenExtensions": "扩展",
    "PageUp": "上页",
    "PageDown": "下页",
    "GotoTop": "顶/底",
    "GotoBottom": "底部",
}


def main() -> None:
    settings = parse_settings()
    slash_en = parse_slash()
    actions_en = parse_actions()

    enums_en = {
        "settings.permission_mode.choice_default": "Default",
        "settings.permission_mode.choice_default_desc": (
            "Use the agent's default permission behavior (currently equivalent to Ask)."
        ),
        "settings.permission_mode.choice_ask": "Ask",
        "settings.permission_mode.choice_ask_desc": "Prompt for permission before tool actions.",
        "settings.permission_mode.choice_auto": "Auto",
        "settings.permission_mode.choice_auto_desc": (
            "LLM classifier approves safe tools; dangerous actions may still prompt or deny."
        ),
        "settings.permission_mode.choice_always_approve": "Always approve",
        "settings.permission_mode.choice_always_approve_desc": (
            "Auto-approve every tool action. Skips ALL permission prompts."
        ),
        "settings.plan_mode.choice_off": "Off",
        "settings.plan_mode.choice_off_desc": "Agent runs tools and edits files directly (default).",
        "settings.plan_mode.choice_on": "On",
        "settings.plan_mode.choice_on_desc": (
            "Agent summarises a plan and asks for approval before running tools."
        ),
        "settings.screen_mode.choice_fullscreen": "Fullscreen",
        "settings.screen_mode.choice_fullscreen_desc": (
            "Open plain grok in the standard fullscreen TUI. Default when unset."
        ),
        "settings.screen_mode.choice_minimal": "Minimal",
        "settings.screen_mode.choice_minimal_desc": (
            "Open plain grok in scrollback-native (minimal) mode."
        ),
        "settings.theme.choice_auto_desc": "Follow system dark/light appearance.",
        "settings.theme.choice_groknight_desc": "Neutral dark with magenta accent.",
        "settings.theme.choice_grokday_desc": "Light theme for bright environments.",
        "settings.theme.choice_tokyonight_desc": "Dark + blue-tinted; needs truecolor.",
        "settings.theme.choice_rosepine_moon_desc": "Muted dark with mauve accents; needs truecolor.",
        "settings.theme.choice_oscura_midnight_desc": "Deep dark with warm accents; needs truecolor.",
        "settings.modal.title": "Settings",
        "settings.modal.value_on": "on",
        "settings.modal.value_off": "off",
        "settings.modal.restart_pill": "restart",
        "settings.modal.no_matches_for": "No matches for",
        "settings.modal.footer.nav_jk": "↑/↓/j/k nav",
        "settings.modal.footer.space_toggle": "Space toggle",
        "settings.modal.footer.enter_edit": "Enter edit",
        "settings.modal.footer.slash_search": "/ search",
        "settings.modal.footer.reset": "d reset",
        "settings.modal.footer.close": "F2/Esc close",
        "settings.dynamic_enum.no_override": "(no override)",
        "settings.dynamic_enum.no_override_desc": "Inherit the default model (no per-user override).",
    }
    enums_zh = {
        "settings.permission_mode.choice_default": "默认",
        "settings.permission_mode.choice_default_desc": "使用智能体默认权限行为（目前等同于询问）。",
        "settings.permission_mode.choice_ask": "询问",
        "settings.permission_mode.choice_ask_desc": "在工具操作前请求许可。",
        "settings.permission_mode.choice_auto": "自动",
        "settings.permission_mode.choice_auto_desc": "LLM 分类器批准安全工具；危险操作仍可能询问或拒绝。",
        "settings.permission_mode.choice_always_approve": "始终批准",
        "settings.permission_mode.choice_always_approve_desc": "自动批准所有工具操作。跳过全部权限提示。",
        "settings.plan_mode.choice_off": "关",
        "settings.plan_mode.choice_off_desc": "智能体直接运行工具并编辑文件（默认）。",
        "settings.plan_mode.choice_on": "开",
        "settings.plan_mode.choice_on_desc": "智能体先汇总计划并在运行工具前请求批准。",
        "settings.screen_mode.choice_fullscreen": "全屏",
        "settings.screen_mode.choice_fullscreen_desc": "以标准全屏 TUI 打开 plain grok。未设置时的默认。",
        "settings.screen_mode.choice_minimal": "极简",
        "settings.screen_mode.choice_minimal_desc": "以回滚原生（极简）模式打开 plain grok。",
        "settings.theme.choice_auto_desc": "跟随系统深色/浅色外观。",
        "settings.theme.choice_groknight_desc": "中性深色，品红强调色。",
        "settings.theme.choice_grokday_desc": "适合明亮环境的浅色主题。",
        "settings.theme.choice_tokyonight_desc": "深色偏蓝；需要真彩色。",
        "settings.theme.choice_rosepine_moon_desc": "柔和深色，淡紫强调；需要真彩色。",
        "settings.theme.choice_oscura_midnight_desc": "深邃暗色，暖色强调；需要真彩色。",
        "settings.modal.title": "设置",
        "settings.modal.value_on": "开",
        "settings.modal.value_off": "关",
        "settings.modal.restart_pill": "需重启",
        "settings.modal.no_matches_for": "无匹配",
        "settings.modal.footer.nav_jk": "↑/↓/j/k 导航",
        "settings.modal.footer.space_toggle": "Space 切换",
        "settings.modal.footer.enter_edit": "Enter 编辑",
        "settings.modal.footer.slash_search": "/ 搜索",
        "settings.modal.footer.reset": "d 重置",
        "settings.modal.footer.close": "F2/Esc 关闭",
        "settings.dynamic_enum.no_override": "(不覆盖)",
        "settings.dynamic_enum.no_override_desc": "继承默认模型（无每用户覆盖）。",
    }

    perm_en = {
        "permission.default.always_allow_all_sessions": "Always allow on all sessions",
        "permission.default.allow_once": "Allow once",
        "permission.default.allow_command_always": "Always allow this command",
        "permission.default.reject": "Reject",
        "permission.hint.choose_scope": "Use ← → to choose permission scope",
        "permission.reject_once.placeholder": "No, reject (type to add feedback)",
    }
    perm_zh = {
        "permission.default.always_allow_all_sessions": "所有会话始终允许",
        "permission.default.allow_once": "允许一次",
        "permission.default.allow_command_always": "始终允许此命令",
        "permission.default.reject": "拒绝",
        "permission.hint.choose_scope": "使用 ← → 选择权限范围",
        "permission.reject_once.placeholder": "不，拒绝（可输入反馈）",
    }

    welcome_en = {
        "welcome.quit": "quit",
        "welcome.quit_menu": "Quit",
        "welcome.resume_session": "Resume session",
        "welcome.new_session": "New session",
        "welcome.settings": "Settings",
        "welcome.changelog": "Changelog",
        "welcome.switch_account": "Switch account",
        "welcome.logout": "Logout",
        "welcome.upgrade_subscription": "Upgrade Subscription",
        "welcome.import_claude_settings": "Import Claude settings",
        "welcome.new_worktree": "New worktree",
        "welcome.trust.yes": "Yes, proceed",
        "welcome.trust.no": "No, quit",
        "welcome.trust.question": "Do you trust the contents of this directory?",
        "welcome.trust.warning_line1": "Grok Build may run or modify contents in this directory,",
        "welcome.trust.warning_line2": "posing security risks.",
        "welcome.type_message": "Type a message...",
        "welcome.hero_subtitle": "Thanks for trying Grok Build, give feedback with /feedback!",
        "welcome.logged_in_api_key": "Logged in with API key",
        "welcome.zdr_unavailable": "Grok Build is not yet available for this account.",
        "welcome.supergrok_required": "SuperGrok subscription required",
        "welcome.refresh": "[Refresh]",
        "welcome.tier_free": "Free",
        "welcome.beta": "Beta",
        "welcome.login_with": "Login with {label}",
        "welcome.picker.back": "back",
        "welcome.picker.select": "select",
        "welcome.picker.navigate": "navigate",
        "welcome.picker.filter": "filter",
        "welcome.picker.worktree": "worktree",
        "auth.browser_open_header": "A browser window will open for authentication.",
        "auth.device_header": "Approve in your browser to finish signing in.",
        "auth.device_code_caption": "Make sure your browser shows this code.",
        "auth.copied": "copied!",
        "auth.copy_failed": "copy failed",
        "auth.waiting_login": "Waiting for login to complete...",
        "auth.waiting_approval": "Waiting for approval...",
        "auth.waiting_url": "Waiting for auth URL...",
        "auth.connecting": "Connecting...",
        "auth.paste_token_placeholder": "Paste your token here...",
        "auth.go_back": "go back",
        "auth.submit": "submit",
        "auth.show_full_url": "Copying not working? Click here to show full URL.",
        "auth.select_url_hint": "Select the URL below with your mouse and copy manually.",
    }
    welcome_zh = {
        "welcome.quit": "退出",
        "welcome.quit_menu": "退出",
        "welcome.resume_session": "恢复会话",
        "welcome.new_session": "新会话",
        "welcome.settings": "设置",
        "welcome.changelog": "更新日志",
        "welcome.switch_account": "切换账号",
        "welcome.logout": "登出",
        "welcome.upgrade_subscription": "升级订阅",
        "welcome.import_claude_settings": "导入 Claude 设置",
        "welcome.new_worktree": "新建工作树",
        "welcome.trust.yes": "是，继续",
        "welcome.trust.no": "否，退出",
        "welcome.trust.question": "是否信任此目录中的内容？",
        "welcome.trust.warning_line1": "Grok Build 可能在此目录中运行命令或修改内容，",
        "welcome.trust.warning_line2": "存在安全风险。",
        "welcome.type_message": "输入消息...",
        "welcome.hero_subtitle": "感谢试用 Grok Build，可用 /feedback 反馈！",
        "welcome.logged_in_api_key": "已使用 API 密钥登录",
        "welcome.zdr_unavailable": "此账号尚无法使用 Grok Build。",
        "welcome.supergrok_required": "需要 SuperGrok 订阅",
        "welcome.refresh": "[刷新]",
        "welcome.tier_free": "免费",
        "welcome.beta": "Beta",
        "welcome.login_with": "使用 {label} 登录",
        "welcome.picker.back": "返回",
        "welcome.picker.select": "选择",
        "welcome.picker.navigate": "导航",
        "welcome.picker.filter": "筛选",
        "welcome.picker.worktree": "工作树",
        "auth.browser_open_header": "将打开浏览器窗口进行身份验证。",
        "auth.device_header": "在浏览器中批准以完成登录。",
        "auth.device_code_caption": "请确认浏览器显示此代码。",
        "auth.copied": "已复制！",
        "auth.copy_failed": "复制失败",
        "auth.waiting_login": "等待登录完成...",
        "auth.waiting_approval": "等待批准...",
        "auth.waiting_url": "等待认证链接...",
        "auth.connecting": "连接中...",
        "auth.paste_token_placeholder": "在此粘贴令牌...",
        "auth.go_back": "返回",
        "auth.submit": "提交",
        "auth.show_full_url": "无法复制？点击此处显示完整 URL。",
        "auth.select_url_hint": "请用鼠标选中下方 URL 并手动复制。",
    }

    turn_en = {
        "turn.activity.cancelling": "Cancelling…",
        "turn.activity.verifying": "Verifying…",
        "turn.activity.thinking": "Thinking…",
        "turn.activity.responding": "Responding…",
        "turn.activity.compacting": "Compacting…",
        "turn.activity.running": "Running…",
        "turn.activity.waiting": "Waiting…",
        "turn.activity.starting_session": "Starting session…",
        "turn.button.stop": "[stop]",
        "turn.button.stop_spaced": " [stop]",
        "turn.wait.model": "Waiting for response…",
        "turn.wait.subagent": "Waiting on subagent…",
        "turn.wait.task_output": "Waiting on task output…",
        "turn.wait.tasks_complete": "Waiting on tasks…",
        "turn.wait.sleep": "Sleeping…",
        "turn.watchers.watching": "watching",
        "turn.tool.prefix.run": "Run ",
        "turn.tool.prefix.search": "Search ",
        "turn.tool.prefix.fetch": "Fetch ",
    }
    turn_zh = {
        "turn.activity.cancelling": "正在取消…",
        "turn.activity.verifying": "正在验证…",
        "turn.activity.thinking": "思考中…",
        "turn.activity.responding": "回复中…",
        "turn.activity.compacting": "压缩中…",
        "turn.activity.running": "运行中…",
        "turn.activity.waiting": "等待中…",
        "turn.activity.starting_session": "正在启动会话…",
        "turn.button.stop": "[停止]",
        "turn.button.stop_spaced": " [停止]",
        "turn.wait.model": "等待响应…",
        "turn.wait.subagent": "等待子智能体…",
        "turn.wait.task_output": "等待任务输出…",
        "turn.wait.tasks_complete": "等待任务…",
        "turn.wait.sleep": "休眠中…",
        "turn.watchers.watching": "监视中",
        "turn.tool.prefix.run": "运行 ",
        "turn.tool.prefix.search": "搜索 ",
        "turn.tool.prefix.fetch": "获取 ",
    }

    tips_en = {
        "tips.undo.prefix": "Input cleared · ",
        "tips.undo.suffix": " to undo",
        "tips.clipboard_image.prefix": "Image in clipboard · ",
        "tips.clipboard_image.suffix": " to paste",
        "tips.plan_nudge": "Planning? Check out plan mode via ",
        "tips.send_now.prefix": "Queued · ",
        "tips.send_now.enter": "Enter",
        "tips.send_now.suffix": " to send now",
        "tips.small_screen": "Tight on space? Try /compact-mode",
    }
    tips_zh = {
        "tips.undo.prefix": "输入已清空 · ",
        "tips.undo.suffix": " 撤销",
        "tips.clipboard_image.prefix": "剪贴板有图片 · ",
        "tips.clipboard_image.suffix": " 粘贴",
        "tips.plan_nudge": "在做规划？试试计划模式 ",
        "tips.send_now.prefix": "已排队 · ",
        "tips.send_now.enter": "Enter",
        "tips.send_now.suffix": " 立即发送",
        "tips.small_screen": "空间紧张？试试 /compact-mode",
    }

    # --- EN ---
    en: list[str] = [
        "# English is the source of truth for message keys.",
        '# Nested tables flatten to dotted keys (e.g. [welcome] quit → "welcome.quit").',
        "",
        "[settings.category]",
        'appearance = "Appearance"',
        'mouse = "Mouse"',
        'editor = "Editor & Input"',
        'agent = "Agent & Approval"',
        'privacy = "Privacy"',
        'models = "Models"',
        'session = "Session"',
        'advanced = "Advanced"',
        "",
        "[toast]",
        'language_set = "UI language: {name}"',
        "",
    ]
    setting_keys = {k for k, _, _ in settings}
    for key, label, desc in settings:
        en.append(f"[settings.{key}]")
        en.append(f'label = "{esc(label)}"')
        if desc:
            en.append(f'description = "{esc(desc)}"')
        if key == "language":
            en.append('choice_auto = "System"')
            en.append('choice_auto_desc = "Follow the operating system language."')
            en.append('choice_en = "English"')
            en.append('choice_en_desc = "English interface."')
            en.append('choice_zh_cn = "简体中文"')
            en.append('choice_zh_cn_desc = "Simplified Chinese interface."')
        prefix = f"settings.{key}."
        for ek, ev in sorted(enums_en.items()):
            if ek.startswith(prefix):
                en.append(f'{ek[len(prefix):]} = "{esc(ev)}"')
        en.append("")

    all_en: dict[str, str] = {}
    for ek, ev in enums_en.items():
        parts = ek.split(".")
        if len(parts) >= 3 and parts[0] == "settings" and parts[1] in setting_keys:
            continue
        all_en[ek] = ev
    all_en.update(perm_en)
    all_en.update(welcome_en)
    all_en.update(turn_en)
    all_en.update(tips_en)
    all_en.update(slash_en)
    all_en.update(actions_en)
    en.append(flat_kv_section(all_en))

    # --- ZH ---
    zh: list[str] = [
        "# Simplified Chinese. Missing keys fall back to en.toml at runtime.",
        "",
        "[settings.category]",
        'appearance = "外观"',
        'mouse = "鼠标"',
        'editor = "编辑与输入"',
        'agent = "智能体与批准"',
        'privacy = "隐私"',
        'models = "模型"',
        'session = "会话"',
        'advanced = "高级"',
        "",
        "[toast]",
        'language_set = "界面语言：{name}"',
        "",
    ]
    for key, label, desc in settings:
        zl, zd = ZH_SETTINGS.get(key, (label, desc))
        zh.append(f"[settings.{key}]")
        zh.append(f'label = "{esc(zl)}"')
        if zd:
            zh.append(f'description = "{esc(zd)}"')
        if key == "language":
            zh.append('choice_auto = "跟随系统"')
            zh.append('choice_auto_desc = "跟随操作系统语言。"')
            zh.append('choice_en = "English"')
            zh.append('choice_en_desc = "英文界面。"')
            zh.append('choice_zh_cn = "简体中文"')
            zh.append('choice_zh_cn_desc = "简体中文界面。"')
        prefix = f"settings.{key}."
        for ek, ev in sorted(enums_zh.items()):
            if ek.startswith(prefix):
                zh.append(f'{ek[len(prefix):]} = "{esc(ev)}"')
        zh.append("")

    all_zh: dict[str, str] = {}
    for ek, ev in enums_zh.items():
        parts = ek.split(".")
        if len(parts) >= 3 and parts[0] == "settings" and parts[1] in setting_keys:
            continue
        all_zh[ek] = ev
    all_zh.update(perm_zh)
    all_zh.update(welcome_zh)
    all_zh.update(turn_zh)
    all_zh.update(tips_zh)
    for k, v in slash_en.items():
        name = k.split(".")[1]
        all_zh[k] = SLASH_ZH.get(name, v)
    for k, v in actions_en.items():
        parts = k.split(".")
        if len(parts) == 3 and parts[2] == "label":
            all_zh[k] = ACTION_ZH_LABEL.get(parts[1], v)
        elif len(parts) == 3:
            all_zh[k] = v
    zh.append(flat_kv_section(all_zh))

    en_path = ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml"
    zh_path = ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml"
    en_path.write_text("\n".join(en) + "\n", encoding="utf-8")
    zh_path.write_text("\n".join(zh) + "\n", encoding="utf-8")
    print(f"wrote {en_path}")
    print(f"wrote {zh_path}")
    print(f"settings={len(settings)} slash={len(slash_en)} actions_fields={len(actions_en)}")


if __name__ == "__main__":
    main()
