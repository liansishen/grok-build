use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

use super::config::ProgramStatusMode;
use crate::terminal::{MultiplexerKind, TerminalContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Permission,
    Question,
    Auth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramStatus {
    Idle,
    Working,
    Done,
    Blocked(BlockKind),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Route {
    pub tmux: bool,
    pub tern_agent_compat: bool,
}

#[derive(Default)]
struct Environment {
    interactive: bool,
    marked: bool,
    tern: bool,
}

const CLEAR: &str = "\x1b]7501;state=clear\x1b\\";
const CLEAR_TMUX: &str = "\x1bPtmux;\x1b\x1b]7501;state=clear\x1b\x1b\\\x1b\\";
// Tern 0.9.21 classifies shell agents by OSC 133's command name.
const TERN_REGISTER: &str = "\x1b]133;C;cmdline=omp\x07";
const TERN_RESTORE: &str = "\x1b]133;C;cmdline=grok\x07";
const DIRECT: u8 = 1;
const TMUX: u8 = 2;
const TERN: u8 = 4;
static TEARDOWN: AtomicU8 = AtomicU8::new(0);

fn nonempty(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

fn tern_environment(program: Option<&str>, pane: bool, socket: bool) -> bool {
    program.is_some_and(|name| name.eq_ignore_ascii_case("tern")) || (pane && socket)
}

pub(super) fn route(
    mode: ProgramStatusMode,
    tern_agent_compat: bool,
    ctx: &TerminalContext,
) -> Option<Route> {
    // The pager retains its terminal writer before redirecting process stderr.
    let output_is_tty = xai_tty_utils::dup_tui_stderr().is_ok_and(|output| output.is_terminal());
    let program = std::env::var("TERM_PROGRAM").ok();
    let env = Environment {
        interactive: std::io::stdin().is_terminal() && output_is_tty,
        marked: (nonempty("HC_TERMINAL_ID") && nonempty("HC_TERMINAL_SOCKET"))
            || (nonempty("REX_SESSION") && nonempty("REX_BLOCK")),
        tern: tern_environment(
            program.as_deref(),
            nonempty("TERN_PANE"),
            nonempty("TERN_PANE_SOCKET") || nonempty("TERN_WINDOW_SOCKET"),
        ),
    };
    select_route(mode, tern_agent_compat, ctx, env)
}

fn select_route(
    mode: ProgramStatusMode,
    tern_agent_compat: bool,
    ctx: &TerminalContext,
    env: Environment,
) -> Option<Route> {
    if mode == ProgramStatusMode::Off || !env.interactive {
        return None;
    }
    if mode == ProgramStatusMode::Auto
        && ((!env.marked && !env.tern)
            || ctx.embedded_editor.is_some()
            || !matches!(
                ctx.multiplexer,
                MultiplexerKind::Undetected | MultiplexerKind::Herdr
            ))
    {
        return None;
    }
    let tmux = ctx.is_tmux_backed();
    if tmux && !super::tmux::passthrough_available(ctx) {
        return None;
    }
    Some(Route {
        tmux,
        tern_agent_compat: tern_agent_compat
            && env.tern
            && ctx.embedded_editor.is_none()
            && ctx.multiplexer == MultiplexerKind::Undetected,
    })
}

pub(super) fn update_escape(status: ProgramStatus, route: Route, first: bool) -> String {
    let mut escape = String::new();
    if first && route.tern_agent_compat {
        escape.push_str(TERN_REGISTER);
    }
    escape.push_str(&report(status, route.tmux));
    escape
}

pub(super) fn report(status: ProgramStatus, tmux: bool) -> String {
    let state = match status {
        ProgramStatus::Idle => "idle",
        ProgramStatus::Working => "working",
        ProgramStatus::Done => "done",
        ProgramStatus::Blocked(_) => "blocked",
        ProgramStatus::Error => "error",
    };
    let kind = match status {
        ProgramStatus::Blocked(BlockKind::Permission) => ":kind=permission",
        ProgramStatus::Blocked(BlockKind::Question) => ":kind=question",
        ProgramStatus::Blocked(BlockKind::Auth) => ":kind=auth",
        _ => "",
    };
    let sequence = format!("\x1b]7501;state={state}{kind}:app=grok-build\x1b\\");
    if tmux {
        super::tmux::tmux_passthrough(&sequence)
    } else {
        sequence
    }
}

pub(super) fn mark_reported(route: Route) {
    let flags =
        if route.tmux { TMUX } else { DIRECT } | if route.tern_agent_compat { TERN } else { 0 };
    TEARDOWN.fetch_or(flags, Ordering::AcqRel);
}

fn take_teardown(state: &AtomicU8) -> Option<String> {
    let flags = state.swap(0, Ordering::AcqRel);
    let mut escape = match flags & (DIRECT | TMUX) {
        DIRECT => CLEAR.to_owned(),
        TMUX => CLEAR_TMUX.to_owned(),
        _ => return None,
    };
    if flags & TERN != 0 {
        escape.push_str(TERN_RESTORE);
    }
    Some(escape)
}

pub(crate) fn take_teardown_escape() -> Option<String> {
    take_teardown(&TEARDOWN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::EmbeddedEditor;

    fn tern() -> Environment {
        Environment {
            interactive: true,
            tern: true,
            marked: false,
        }
    }

    #[test]
    fn tern_detected_by_brand_or_complete_pane_markers() {
        assert!(tern_environment(Some("tern"), false, false));
        assert!(tern_environment(Some("Tern"), false, false));
        assert!(tern_environment(None, true, true));
        for (program, pane, socket) in [
            (None, false, false),
            (None, true, false),
            (None, false, true),
            (Some(""), false, false),
            (Some("kitty"), false, false),
        ] {
            assert!(!tern_environment(program, pane, socket));
        }
    }

    #[test]
    fn auto_enables_tern_reporting_and_compatibility() {
        assert_eq!(
            select_route(
                ProgramStatusMode::Auto,
                true,
                &TerminalContext::default(),
                tern()
            ),
            Some(Route {
                tmux: false,
                tern_agent_compat: true
            }),
        );
    }

    #[test]
    fn compatibility_can_be_disabled_without_disabling_status() {
        assert_eq!(
            select_route(
                ProgramStatusMode::Auto,
                false,
                &TerminalContext::default(),
                tern()
            ),
            Some(Route {
                tmux: false,
                tern_agent_compat: false
            }),
        );
    }

    #[test]
    fn off_and_noninteractive_outputs_emit_nothing() {
        for mode in [
            ProgramStatusMode::Auto,
            ProgramStatusMode::On,
            ProgramStatusMode::Off,
        ] {
            assert_eq!(
                select_route(
                    mode,
                    true,
                    &TerminalContext::default(),
                    Environment {
                        interactive: false,
                        ..tern()
                    }
                ),
                None,
            );
        }
        assert_eq!(
            select_route(
                ProgramStatusMode::Off,
                true,
                &TerminalContext::default(),
                tern()
            ),
            None,
        );
    }

    #[test]
    fn other_terminals_keep_existing_routing() {
        let ctx = TerminalContext::default();
        assert_eq!(
            select_route(
                ProgramStatusMode::Auto,
                true,
                &ctx,
                Environment {
                    interactive: true,
                    ..Default::default()
                }
            ),
            None,
        );
        for mode in [ProgramStatusMode::Auto, ProgramStatusMode::On] {
            assert_eq!(
                select_route(
                    mode,
                    true,
                    &ctx,
                    Environment {
                        interactive: true,
                        marked: true,
                        tern: false,
                    }
                ),
                Some(Route {
                    tmux: false,
                    tern_agent_compat: false
                }),
            );
        }
    }

    #[test]
    fn inherited_tern_markers_do_not_register_inner_multiplexers() {
        for multiplexer in [
            MultiplexerKind::Tmux,
            MultiplexerKind::Screen,
            MultiplexerKind::Zellij,
            MultiplexerKind::Cmux,
            MultiplexerKind::Herdr,
        ] {
            let ctx = TerminalContext {
                multiplexer,
                tmux_version: Some("tmux 3.4".into()),
                ..Default::default()
            };
            if let Some(route) = select_route(ProgramStatusMode::Auto, true, &ctx, tern()) {
                assert_eq!(multiplexer, MultiplexerKind::Herdr);
                assert!(!route.tern_agent_compat);
            }
            let explicit = select_route(ProgramStatusMode::On, true, &ctx, tern()).unwrap();
            assert!(!explicit.tern_agent_compat);
            assert_eq!(explicit.tmux, multiplexer == MultiplexerKind::Tmux);
        }
    }

    #[test]
    fn explicit_tmux_reporting_still_requires_passthrough_support() {
        for version in [None, Some("tmux 3.2"), Some("tmux 3.3")] {
            let ctx = TerminalContext {
                multiplexer: MultiplexerKind::Tmux,
                tmux_version: version.map(str::to_owned),
                ..Default::default()
            };
            assert_eq!(
                select_route(ProgramStatusMode::On, true, &ctx, tern()).is_some(),
                version == Some("tmux 3.3"),
            );
        }
    }

    #[test]
    fn embedded_editors_never_receive_tern_registration() {
        let ctx = TerminalContext {
            embedded_editor: Some(EmbeddedEditor::Neovim),
            ..Default::default()
        };
        assert_eq!(
            select_route(ProgramStatusMode::Auto, true, &ctx, tern()),
            None
        );
        assert_eq!(
            select_route(ProgramStatusMode::On, true, &ctx, tern()),
            Some(Route {
                tmux: false,
                tern_agent_compat: false
            }),
        );
    }

    #[test]
    fn first_status_registers_tern_before_reporting() {
        let route = Route {
            tmux: false,
            tern_agent_compat: true,
        };
        assert_eq!(
            update_escape(ProgramStatus::Idle, route, true),
            format!("{TERN_REGISTER}\x1b]7501;state=idle:app=grok-build\x1b\\"),
        );
        assert_eq!(
            update_escape(ProgramStatus::Working, route, false),
            "\x1b]7501;state=working:app=grok-build\x1b\\",
        );
        let disabled = Route {
            tern_agent_compat: false,
            ..route
        };
        assert_eq!(
            update_escape(ProgramStatus::Idle, disabled, true),
            report(ProgramStatus::Idle, false),
        );
    }

    #[test]
    fn all_states_preserve_grok_identity_and_block_kinds() {
        for (status, fields) in [
            (ProgramStatus::Idle, "idle"),
            (ProgramStatus::Working, "working"),
            (ProgramStatus::Done, "done"),
            (ProgramStatus::Error, "error"),
            (
                ProgramStatus::Blocked(BlockKind::Permission),
                "blocked:kind=permission",
            ),
            (
                ProgramStatus::Blocked(BlockKind::Question),
                "blocked:kind=question",
            ),
            (ProgramStatus::Blocked(BlockKind::Auth), "blocked:kind=auth"),
        ] {
            let direct = format!("\x1b]7501;state={fields}:app=grok-build\x1b\\");
            assert_eq!(report(status, false), direct);
            assert_eq!(
                report(status, true),
                super::super::tmux::tmux_passthrough(&direct)
            );
        }
    }

    #[test]
    fn teardown_restores_grok_only_after_tern_registration() {
        for (flags, expected) in [
            (0, None),
            (DIRECT, Some(CLEAR.to_owned())),
            (TMUX, Some(CLEAR_TMUX.to_owned())),
            (DIRECT | TERN, Some(format!("{CLEAR}{TERN_RESTORE}"))),
        ] {
            let state = AtomicU8::new(flags);
            assert_eq!(take_teardown(&state), expected);
            assert_eq!(take_teardown(&state), None);
        }
    }
}
