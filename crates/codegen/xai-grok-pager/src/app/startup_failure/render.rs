use std::fmt::Write as _;
use std::time::Duration;

use xai_grok_i18n::{t, t_fmt};
use xai_grok_telemetry::startup::{AgentKind, PhaseSnapshot, StartupPhase, format_duration};

use super::{ConnectAttempt, Context, EarlierAttempt, Reason, StartupFailure};

const WRAP_WIDTH: usize = 76;

pub(super) fn render(failure: &StartupFailure) -> String {
    let context = &failure.context;
    let mut rows = vec![
        (t("startup.failure.row.mode"), attempted_agents(context)),
        (t("startup.failure.row.version"), context.version.clone()),
    ];
    let mut report = match &failure.reason {
        Reason::TimedOut { waited, timings } => {
            let advice = advice_for(timings, context.attempt);
            rows.push((t("startup.failure.row.steps"), format_steps(timings)));
            if let Some(command) = advice.next_step.command() {
                rows.push((t("startup.failure.row.try"), command.to_owned()));
            }
            let explanation = fill_indented(&advice.explanation(), "  ", "  ");
            t_fmt(
                "startup.failure.timed_out",
                &[
                    ("waited", &whole_seconds(*waited)),
                    ("explanation", &explanation),
                ],
            )
        }
        Reason::Cancelled => t_fmt(
            "startup.failure.cancelled",
            &[("agent", agent_name(context.target))],
        ),
    };
    rows.push((
        t("startup.failure.row.log"),
        context.log_path.display().to_string(),
    ));
    let _ = write!(report, "\n\n{}", label_rows(&rows));
    report
}

struct Advice {
    doing: Option<&'static str>,
    earlier: Option<EarlierAttempt>,
    next_step: NextStep,
}

/// A wedged leader is only ever the earlier attempt: the fallback that renders
/// this message never enters `LeaderConnect` itself.
fn advice_for(timings: &PhaseSnapshot, attempt: ConnectAttempt) -> Advice {
    let step = timings.longest_step().map(step_advice);
    let earlier = attempt.earlier();
    Advice {
        doing: step.map(|(doing, _)| doing),
        earlier: earlier.filter(|earlier| earlier.shaped_the_wait()),
        next_step: if earlier.is_some_and(|earlier| earlier.wedged_leader()) {
            NextStep::RestartSharedLeader
        } else {
            step.map_or(NextStep::Retry, |(_, next_step)| next_step)
        },
    }
}

impl Advice {
    fn explanation(&self) -> String {
        let mut explanation = match self.doing {
            Some(doing) => t_fmt("startup.failure.longest_step", &[("doing", doing)]),
            None => t("startup.failure.no_step_begun").to_owned(),
        };
        if let Some(earlier) = self.earlier {
            let target = agent_name(earlier.target);
            let _ = write!(
                explanation,
                "{}",
                t_fmt(
                    "startup.failure.spent_on_earlier",
                    &[
                        ("waited", &whole_seconds(earlier.wait)),
                        ("agent", target),
                    ],
                )
            );
        }
        let _ = write!(explanation, " {}", self.next_step.text());
        explanation
    }
}

fn format_steps(timings: &PhaseSnapshot) -> String {
    let completed = timings
        .completed
        .iter()
        .map(|&(phase, elapsed)| (phase, elapsed, ""));
    let open = timings.open.map(|(phase, elapsed)| (phase, elapsed, "+"));
    let steps: Vec<String> = completed
        .chain(open)
        .map(|(phase, elapsed, still_running)| {
            format!(
                "{}={}{still_running}",
                phase.label(),
                format_duration(elapsed)
            )
        })
        .collect();
    if steps.is_empty() {
        return t("startup.failure.steps_none").to_owned();
    }
    steps.join(", ")
}

/// Values hang under their label, so a wrapped one never reads as a new field.
fn label_rows(rows: &[(&str, String)]) -> String {
    let column_width = rows
        .iter()
        .map(|(label, _)| label.len() + ":".len())
        .max()
        .unwrap_or(0);
    rows.iter()
        .map(|(label, value)| {
            let label = format!("  {:<column_width$} ", format!("{label}:"));
            let hanging = " ".repeat(label.len());
            fill_indented(value, &label, &hanging)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// A path or a command has to survive a paste, so words are never split.
fn fill_indented(text: &str, initial_indent: &str, subsequent_indent: &str) -> String {
    textwrap::fill(
        text,
        textwrap::Options::new(WRAP_WIDTH)
            .initial_indent(initial_indent)
            .subsequent_indent(subsequent_indent)
            .break_words(false)
            .wrap_algorithm(textwrap::WrapAlgorithm::FirstFit),
    )
}

#[derive(Clone, Copy)]
enum NextStep {
    Retry,
    CheckNetworkThenRetry,
    RestartSharedLeader,
}

impl NextStep {
    fn text(self) -> &'static str {
        match self {
            Self::Retry => t("startup.failure.next.retry"),
            Self::CheckNetworkThenRetry => t("startup.failure.next.check_network"),
            Self::RestartSharedLeader => t("startup.failure.next.restart_leader"),
        }
    }

    /// Kept out of the prose so wrapping can never split it.
    fn command(self) -> Option<&'static str> {
        match self {
            Self::Retry | Self::CheckNetworkThenRetry => None,
            Self::RestartSharedLeader => Some("grok leader kill"),
        }
    }
}

/// Reads as the object of "The longest step was".
fn step_advice(phase: StartupPhase) -> (&'static str, NextStep) {
    use NextStep::{CheckNetworkThenRetry as Network, RestartSharedLeader, Retry};
    match phase {
        StartupPhase::LoadConfig => (t("startup.failure.step.load_config"), Retry),
        StartupPhase::ManagedPolicy => (t("startup.failure.step.managed_policy"), Network),
        StartupPhase::Bootstrap => (t("startup.failure.step.bootstrap"), Network),
        // A disk cache read; the network fetch is the background refresh.
        StartupPhase::ModelCatalog => (t("startup.failure.step.model_catalog"), Retry),
        StartupPhase::SpawnWorker => (t("startup.failure.step.spawn_worker"), Retry),
        // A Unix socket and a local spawn, never the network.
        StartupPhase::LeaderConnect => (t("startup.failure.step.leader_connect"), RestartSharedLeader),
        StartupPhase::AcpInitialize => (t("startup.failure.step.acp_initialize"), Retry),
        StartupPhase::EagerAuth => (t("startup.failure.step.eager_auth"), Network),
        StartupPhase::AppInit => (t("startup.failure.step.app_init"), Retry),
        StartupPhase::SessionCreate => (t("startup.failure.step.session_create"), Retry),
    }
}

fn attempted_agents(context: &Context) -> String {
    let target = agent_name(context.target);
    match context.attempt {
        ConnectAttempt::First => target.to_owned(),
        ConnectAttempt::AfterFallback(earlier) => t_fmt(
            "startup.failure.mode.after_fallback",
            &[("first", agent_name(earlier.target)), ("then", target)],
        ),
    }
}

fn agent_name(agent: AgentKind) -> &'static str {
    match agent {
        AgentKind::Embedded => t("startup.failure.agent.local"),
        AgentKind::Leader => t("startup.failure.agent.leader"),
    }
}

/// Rounded: a truncated total can print smaller than the steps it sums.
pub(super) fn whole_seconds(wait: Duration) -> String {
    format!("{}s", (wait.as_millis() + 500) / 1000)
}
