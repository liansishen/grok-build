//! `/terminal-setup` — diagnose terminal, color/theme, and clipboard setup.
//!
//! Runs the same diagnostics engine used for startup warnings and formats
//! the results as a user-readable message. This gives users an on-demand
//! way to check their environment and see fix instructions.

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use crate::terminal::TerminalName;

pub struct TerminalSetupCommand;

impl SlashCommand for TerminalSetupCommand {
    fn name(&self) -> &str {
        "terminal-setup"
    }

    fn aliases(&self) -> &[&str] {
        &["terminal-check", "terminal-info"]
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.terminal-setup.description")
    }

    fn usage(&self) -> &str {
        "/terminal-setup"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        let ctx = crate::terminal::terminal_context();
        let query = crate::diagnostics::LiveTmuxQuery;
        let is_control_mode = crate::terminal::detect_tmux_control_mode(ctx);
        let mut warnings = crate::diagnostics::collect_startup_warnings(
            ctx,
            &query,
            is_control_mode,
            _ctx.screen_mode.is_fullscreen(),
        );
        // Live-environment check, kept out of `collect_startup_warnings` so
        // its tests stay hermetic (same pattern as the WezTerm warning below).
        warnings.extend(crate::diagnostics::diagnose_wayland_data_control_live());
        // WezTerm without the Kitty keyboard protocol: surface the fix
        // alongside the other issues. By the time the user runs
        // /terminal-setup the async XTVERSION reply has landed, so this
        // also catches WezTerm over SSH (env brand Unknown, self-report
        // "WezTerm <version>").
        let wezterm_warning = crate::diagnostics::wezterm_kitty_keyboard_warning(
            ctx,
            crate::app::kitty_flags_pushed(),
            crate::terminal::xtversion::detected(),
        );
        let wezterm_kkp_off = wezterm_warning.is_some();
        warnings.extend(wezterm_warning);
        // Color not in collect_startup_warnings (noisy on limited terminals).
        let color_level = crate::theme::color_support::get();
        warnings.extend(crate::diagnostics::color_support_warning(
            color_level,
            ctx.brand,
            ctx.is_tmux_backed(),
            &ctx.tmux_config_path(),
        ));
        // SSH wrap recommendation — rendered as its own section below, NOT an
        // issue row: nothing is misconfigured, so it must not put "N issue(s)"
        // on every healthy SSH session. On-demand diagnostics also ignore the
        // `[ui.contextual_hints].ssh_wrap` tip opt-out: that gate (both its
        // user and remote tiers) governs the unprompted session-load tip,
        // while here the user explicitly asked for setup guidance, and an
        // environment report that omits a known improvement would be
        // incomplete.
        let ssh_wrap_recommendation = crate::diagnostics::ssh_wrap_hint(
            ctx.is_ssh,
            crate::clipboard::osc52_sink_active(),
            ctx.is_official_vscode_remote,
        );
        let route = crate::clipboard::clipboard_route();
        let is_ssh = xai_grok_shell::util::clipboard::is_remote_session();
        let container_no_display =
            xai_grok_shell::util::clipboard::is_containerized_without_display();

        let mut out = String::new();

        // -- Environment --
        out.push_str(xai_grok_i18n::t_or(
            "slash.terminal-setup.environment",
            "Environment\n",
        ));
        out.push_str(
            &xai_grok_i18n::t_or(
                "slash.terminal-setup.row_terminal",
                "  terminal     {value}\n",
            )
            .replace("{value}", &ctx.brand.to_string()),
        );
        if let Some(v) = crate::terminal::xtversion::detected() {
            out.push_str(
                &xai_grok_i18n::t_or(
                    "slash.terminal-setup.row_xtversion",
                    "  xtversion    {value}\n",
                )
                .replace("{value}", &v),
            );
        }
        out.push_str(
            &xai_grok_i18n::t_or(
                "slash.terminal-setup.row_multiplexer",
                "  multiplexer  {value}\n",
            )
            .replace("{value}", &ctx.multiplexer.to_string()),
        );
        if let Some(ref byobu) = ctx.byobu {
            out.push_str(
                &xai_grok_i18n::t_or("slash.terminal-setup.row_byobu", "  byobu        {value}\n")
                    .replace("{value}", &byobu.to_string()),
            );
        }
        let ssh_value = if is_ssh {
            xai_grok_i18n::t_or("slash.terminal-setup.yes", "yes")
        } else {
            xai_grok_i18n::t_or("slash.terminal-setup.no", "no")
        };
        out.push_str(
            &xai_grok_i18n::t_or("slash.terminal-setup.row_ssh", "  ssh          {value}\n")
                .replace("{value}", ssh_value),
        );
        out.push_str(&crate::diagnostics::format_color_env_line(color_level));
        out.push_str(&crate::diagnostics::format_themes_env_line(color_level));

        let kb = ctx.keyboard_capabilities();
        if kb.modifier_delivery.benefits_from_rescue() || kb.enter_needs_rescue() {
            let rescue = if cfg!(target_os = "macos") {
                xai_grok_i18n::t_or("slash.terminal-setup.os_rescue_active", "OS rescue active")
            } else {
                xai_grok_i18n::t_or(
                    "slash.terminal-setup.os_rescue_unavailable",
                    "OS rescue unavailable on this platform",
                )
            };
            out.push_str(
                &xai_grok_i18n::t_or(
                    "slash.terminal-setup.row_keyboard",
                    "  keyboard     {delivery} ({rescue})\n",
                )
                .replace("{delivery}", kb.modifier_delivery.label())
                .replace("{rescue}", rescue),
            );
        }

        // Some terminals can't distinguish Shift+Enter from bare Enter at
        // the byte level because the Kitty keyboard protocol isn't
        // negotiated (VTE < 0.82, or VS Code's xterm.js which mis-encodes
        // shifted keys). Point users at Alt+Enter, which is reliably
        // delivered as ESC+CR. Suppressed when the WezTerm warning fired:
        // stock WezTerm binds Alt+Enter to ToggleFullScreen, so advertising
        // it would contradict that warning's `\`+Enter guidance.
        if ctx.shift_enter_unavailable() && !wezterm_kkp_off {
            let detail = if ctx.vte_version.is_some() || ctx.brand == TerminalName::Vte {
                match ctx.vte_version.as_deref() {
                    Some(v) => xai_grok_i18n::t_or(
                        "slash.terminal-setup.newline_vte_version",
                        "VTE {version}; need >= 8200 for Shift+Enter",
                    )
                    .replace("{version}", v),
                    None => xai_grok_i18n::t_or(
                        "slash.terminal-setup.newline_legacy_vte",
                        "legacy VTE; need VTE >= 0.82 for Shift+Enter",
                    )
                    .to_owned(),
                }
            } else if matches!(
                ctx.brand,
                TerminalName::VsCode
                    | TerminalName::Cursor
                    | TerminalName::Windsurf
                    | TerminalName::Zed
            ) {
                xai_grok_i18n::t_or(
                    "slash.terminal-setup.newline_xtermjs",
                    "{terminal}: xterm.js can't distinguish Shift+Enter",
                )
                .replace("{terminal}", &ctx.brand.to_string())
            } else {
                xai_grok_i18n::t_or(
                    "slash.terminal-setup.newline_no_kitty",
                    "no Kitty keyboard protocol; Shift+Enter == Enter",
                )
                .to_owned()
            };
            out.push_str(
                &xai_grok_i18n::t_or(
                    "slash.terminal-setup.row_newline",
                    "  newline      Alt+Enter ({detail})\n",
                )
                .replace("{detail}", &detail),
            );
        }

        // -- Clipboard --
        let display_server = crate::host::DisplayServer::current();
        let is_wayland = display_server == crate::host::DisplayServer::Wayland;
        let clipboard_diagnostics = crate::diagnostics::format_clipboard_diagnostics(
            crate::diagnostics::ClipboardDiagnosticsInput {
                route_native: route.native,
                route_tmux: route.tmux_buffer,
                route_osc52: route.osc52,
                native_tool: xai_grok_shell::util::clipboard::native_tool_name(),
                brand: ctx.brand,
                host_os: crate::host::HostOs::current(),
                display_server,
                is_ssh,
                container_no_display,
                osc52_sink: crate::clipboard::osc52_sink_active(),
                wayland_data_control: is_wayland
                    && xai_grok_shell::util::clipboard::wayland_data_control_supported(),
                wl_copy_available: is_wayland
                    && xai_grok_shell::util::clipboard::native_tool_name() == "wl-copy",
            },
        );
        out.push('\n');
        out.push_str(&clipboard_diagnostics.text);

        // -- Diagnostics --
        if warnings.is_empty() && !clipboard_diagnostics.has_issue {
            out.push_str(xai_grok_i18n::t_or(
                "slash.terminal-setup.no_issues",
                "\nNo issues found.\n",
            ));
        } else if !warnings.is_empty() {
            out.push_str(
                &xai_grok_i18n::t_or(
                    "slash.terminal-setup.additional_issues",
                    "\n{count} additional issue(s)\n",
                )
                .replace("{count}", &warnings.len().to_string()),
            );
            for w in &warnings {
                out.push_str(&format!("\n  [!] {}\n", w.message));
                match (w.fix.as_deref(), w.config_path.as_deref()) {
                    (Some(fix), Some(path)) => {
                        out.push_str(
                            &xai_grok_i18n::t_or(
                                "slash.terminal-setup.fix_place",
                                "      Fix: place `{fix}` in {path}\n",
                            )
                            .replace("{fix}", fix)
                            .replace("{path}", path),
                        );
                    }
                    (Some(fix), None) => {
                        out.push_str(
                            &xai_grok_i18n::t_or(
                                "slash.terminal-setup.fix_run",
                                "      Fix: run `{fix}`\n",
                            )
                            .replace("{fix}", fix),
                        );
                    }
                    _ => {}
                }
                if let Some(note) = w.note.as_deref() {
                    out.push_str(
                        &xai_grok_i18n::t_or("slash.terminal-setup.note", "      Note: {note}\n")
                            .replace("{note}", note),
                    );
                }
            }
        }

        // -- Recommendation --
        if let Some(rec) = ssh_wrap_recommendation {
            out.push_str(
                &xai_grok_i18n::t_or(
                    "slash.terminal-setup.recommendation",
                    "\nRecommendation\n\n  {message}\n",
                )
                .replace("{message}", &rec.message),
            );
            if let Some(fix) = rec.fix.as_deref() {
                out.push_str(
                    &xai_grok_i18n::t_or("slash.terminal-setup.run", "      Run: `{fix}`\n")
                        .replace("{fix}", fix),
                );
            }
            if let Some(note) = rec.note.as_deref() {
                out.push_str(
                    &xai_grok_i18n::t_or("slash.terminal-setup.note", "      Note: {note}\n")
                        .replace("{note}", note),
                );
            }
        }

        CommandResult::Message(out)
    }
}
