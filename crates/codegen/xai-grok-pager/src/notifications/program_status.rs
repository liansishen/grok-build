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

const CLEAR: &str = "\x1b]7501;state=clear\x1b\\";
const CLEAR_TMUX: &str = "\x1bPtmux;\x1b\x1b]7501;state=clear\x1b\x1b\\\x1b\\";
const DIRECT: u8 = 1;
const TMUX: u8 = 2;
static TEARDOWN: AtomicU8 = AtomicU8::new(0);

fn nonempty(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

pub(super) fn route(mode: ProgramStatusMode, ctx: &TerminalContext) -> Option<bool> {
    // The pager retains its terminal writer before redirecting process stderr.
    let output_is_tty = xai_tty_utils::dup_tui_stderr().is_ok_and(|output| output.is_terminal());
    if mode == ProgramStatusMode::Off
        || !std::io::stdin().is_terminal()
        || !output_is_tty
    {
        return None;
    }
    if mode == ProgramStatusMode::Auto {
        let marked = (nonempty("HC_TERMINAL_ID") && nonempty("HC_TERMINAL_SOCKET"))
            || (nonempty("REX_SESSION") && nonempty("REX_BLOCK"));
        if !marked
            || ctx.embedded_editor.is_some()
            || !matches!(ctx.multiplexer, MultiplexerKind::Undetected | MultiplexerKind::Herdr)
        {
            return None;
        }
    }
    if ctx.is_tmux_backed() {
        return super::tmux::passthrough_available(ctx).then_some(true);
    }
    Some(false)
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
    if tmux { super::tmux::tmux_passthrough(&sequence) } else { sequence }
}

pub(super) fn mark_reported(tmux: bool) {
    TEARDOWN.store(if tmux { TMUX } else { DIRECT }, Ordering::Release);
}

pub(crate) fn take_teardown_escape() -> Option<String> {
    match TEARDOWN.swap(0, Ordering::AcqRel) {
        DIRECT => Some(CLEAR.to_owned()),
        TMUX => Some(CLEAR_TMUX.to_owned()),
        _ => None,
    }
}
