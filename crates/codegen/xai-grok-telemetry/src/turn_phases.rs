use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crate::events::PromptLatency;
use crate::session_ctx::log_event;

const LIVE_TPS_WINDOW: Duration = Duration::from_secs(5);
const LIVE_TPS_STALE_AFTER: Duration = Duration::from_millis(1_500);
const LIVE_TPS_MIN_DURATION: Duration = Duration::from_secs(1);
const ESTIMATED_BYTES_PER_TOKEN: u64 = 5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GenerationMetrics {
    pub first_token_ms: Option<u64>,
    pub tokens_per_second: Option<f64>,
    pub estimated: bool,
    pub stale: bool,
}

#[derive(Clone, Copy, Debug)]
struct TextSample {
    at: Instant,
    bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnPhases {
    pub before_first_model_ms: u64,
    pub sampling_ms: u64,
    pub tool_blocking_ms: u64,
    pub compaction_ms: u64,
    pub between_sampling_overhead_ms: u64,
    pub after_last_sampling_ms: u64,
    pub turn_total_ms: u64,
    pub sampling_request_count: u32,
    pub sampling_retry_count: u32,
    pub ttft_ms: Option<u64>,
    pub ttfm_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Sampling,
    ToolBlocking,
    Compaction,
}

#[derive(Debug, Default)]
pub struct TurnPhaseProfile {
    state: parking_lot::Mutex<PhaseState>,
    pending_latency: parking_lot::Mutex<Option<PromptLatency>>,
}

impl TurnPhaseProfile {
    pub fn start(&self) {
        self.state.lock().start(Instant::now());
        *self.pending_latency.lock() = None;
    }

    pub fn arm_latency(&self, event: PromptLatency) {
        *self.pending_latency.lock() = Some(event);
    }

    pub fn emit_pending_latency(&self) -> bool {
        let Some(mut event) = self.pending_latency.lock().take() else {
            return false;
        };
        apply_phases(&mut event, &self.complete());
        log_event(event);
        true
    }

    pub fn record_sampling_request(&self) {
        self.state.lock().record_request();
    }

    pub fn begin_sampling(self: &Arc<Self>) -> TurnPhaseGuard {
        self.begin(Phase::Sampling)
    }

    pub fn begin_tool_blocking(self: &Arc<Self>) -> TurnPhaseGuard {
        self.begin(Phase::ToolBlocking)
    }

    pub fn begin_compaction(self: &Arc<Self>) -> TurnPhaseGuard {
        self.begin(Phase::Compaction)
    }

    fn begin(self: &Arc<Self>, phase: Phase) -> TurnPhaseGuard {
        let mut state = self.state.lock();
        let generation = state.generation;
        let active = state.begin_phase(phase, Instant::now());
        TurnPhaseGuard {
            profile: Arc::clone(self),
            phase,
            generation,
            active,
        }
    }

    pub fn record_sampling_retries(&self, retries: u32) {
        self.state.lock().record_retries(retries);
    }

    pub fn current_generation(&self) -> u64 {
        self.state.lock().generation
    }

    pub fn record_first_token(&self, generation: u64) {
        self.state
            .lock()
            .record_first_token(generation, Instant::now());
    }

    pub fn commit_first_token(&self) {
        self.state.lock().commit_first_token();
    }

    pub fn discard_uncommitted_first_token(&self) {
        self.state.lock().discard_uncommitted_first_token();
    }

    pub fn record_first_meaningful_output(&self, generation: u64) {
        self.state
            .lock()
            .record_first_meaningful(generation, Instant::now());
    }

    pub fn commit_first_meaningful(&self) {
        self.state.lock().commit_first_meaningful();
    }

    pub fn discard_uncommitted_first_meaningful(&self) {
        self.state.lock().discard_uncommitted_first_meaningful();
    }

    pub fn record_text_delta(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.state.lock().record_text_delta(text, Instant::now());
    }

    pub fn record_completed_response(
        &self,
        output_tokens: Option<u32>,
        time_to_first_token_ms: Option<u64>,
        time_to_last_byte_ms: u64,
    ) {
        self.state.lock().record_completed_response(
            output_tokens,
            time_to_first_token_ms,
            time_to_last_byte_ms,
        );
    }

    pub fn generation_metrics(&self) -> GenerationMetrics {
        self.state.lock().generation_metrics(Instant::now())
    }

    pub fn reset_generation_after_retry(&self) {
        self.state.lock().reset_generation_after_retry();
    }

    pub fn complete(&self) -> TurnPhases {
        self.state.lock().complete(Instant::now())
    }
}

fn apply_phases(event: &mut PromptLatency, phases: &TurnPhases) {
    event.before_first_model_ms = phases.before_first_model_ms;
    event.sampling_ms = phases.sampling_ms;
    event.tool_blocking_ms = phases.tool_blocking_ms;
    event.compaction_ms = phases.compaction_ms;
    event.between_sampling_overhead_ms = phases.between_sampling_overhead_ms;
    event.after_last_sampling_ms = phases.after_last_sampling_ms;
    event.turn_total_ms = phases.turn_total_ms;
    event.sampling_request_count = phases.sampling_request_count;
    event.sampling_retry_count = phases.sampling_retry_count;
    event.ttft_ms = phases.ttft_ms;
    event.ttfm_ms = phases.ttfm_ms;
}

#[must_use]
pub struct TurnPhaseGuard {
    profile: Arc<TurnPhaseProfile>,
    phase: Phase,
    generation: u64,
    active: bool,
}

impl Drop for TurnPhaseGuard {
    fn drop(&mut self) {
        if self.active {
            self.profile
                .state
                .lock()
                .end_phase(self.phase, self.generation, Instant::now());
        }
    }
}

#[derive(Debug, Default)]
struct PhaseState {
    generation: u64,
    started_at: Option<Instant>,
    last_transition_at: Option<Instant>,
    stack: Vec<Phase>,
    seen_sampling: bool,
    before_first_model: Duration,
    sampling: Duration,
    tool_blocking: Duration,
    compaction: Duration,
    between_sampling_overhead: Duration,
    pending_idle_after_sampling: Duration,
    sampling_request_count: u32,
    sampling_retry_count: u32,
    first_token: Option<Duration>,
    first_token_committed: bool,
    first_meaningful: Option<Duration>,
    first_meaningful_committed: bool,
    completed: Option<TurnPhases>,
    text_samples: VecDeque<TextSample>,
    last_text_at: Option<Instant>,
    settled_output_tokens: u64,
    settled_decode_ms: u64,
}

impl PhaseState {
    fn start(&mut self, now: Instant) -> u64 {
        let generation = self.generation.wrapping_add(1);
        *self = Self {
            generation,
            started_at: Some(now),
            last_transition_at: Some(now),
            ..Self::default()
        };
        generation
    }

    fn begin_phase(&mut self, phase: Phase, now: Instant) -> bool {
        if self.completed.is_some() || self.started_at.is_none() {
            return false;
        }
        self.advance(now);
        if phase == Phase::Sampling {
            if self.seen_sampling {
                self.between_sampling_overhead +=
                    std::mem::take(&mut self.pending_idle_after_sampling);
            }
            self.seen_sampling = true;
        }
        self.stack.push(phase);
        true
    }

    fn end_phase(&mut self, phase: Phase, generation: u64, now: Instant) {
        if generation != self.generation
            || self.completed.is_some()
            || self.stack.last() != Some(&phase)
        {
            return;
        }
        self.advance(now);
        self.stack.pop();
    }

    fn record_request(&mut self) {
        if self.completed.is_none() && self.started_at.is_some() {
            self.sampling_request_count = self.sampling_request_count.saturating_add(1);
        }
    }

    fn record_retries(&mut self, retries: u32) {
        if self.completed.is_none() && self.started_at.is_some() {
            self.sampling_retry_count = self.sampling_retry_count.saturating_add(retries);
        }
    }

    fn commit_first_token(&mut self) {
        if self.completed.is_none() {
            self.first_token_committed = true;
        }
    }

    fn discard_uncommitted_first_token(&mut self) {
        if self.completed.is_none() && !self.first_token_committed {
            self.first_token = None;
        }
    }

    fn record_first_token(&mut self, generation: u64, now: Instant) {
        if generation != self.generation || self.completed.is_some() || self.first_token.is_some() {
            return;
        }
        let Some(started_at) = self.started_at else {
            return;
        };
        self.first_token = Some(now.saturating_duration_since(started_at));
    }

    fn commit_first_meaningful(&mut self) {
        if self.completed.is_none() {
            self.first_meaningful_committed = true;
        }
    }

    fn discard_uncommitted_first_meaningful(&mut self) {
        if self.completed.is_none() && !self.first_meaningful_committed {
            self.first_meaningful = None;
        }
    }

    fn record_first_meaningful(&mut self, generation: u64, now: Instant) {
        if generation != self.generation
            || self.completed.is_some()
            || self.first_meaningful.is_some()
        {
            return;
        }
        let Some(started_at) = self.started_at else {
            return;
        };
        self.first_meaningful = Some(now.saturating_duration_since(started_at));
    }

    fn record_text_delta(&mut self, text: &str, now: Instant) {
        if self.completed.is_some() || self.started_at.is_none() {
            return;
        }
        let bytes = text.len() as u64;
        self.text_samples.push_back(TextSample { at: now, bytes });
        self.last_text_at = Some(now);
        self.prune_text_samples(now);
    }

    fn record_completed_response(
        &mut self,
        output_tokens: Option<u32>,
        time_to_first_token_ms: Option<u64>,
        time_to_last_byte_ms: u64,
    ) {
        let (Some(output_tokens), Some(ttft_ms)) = (output_tokens, time_to_first_token_ms) else {
            return;
        };
        let decode_ms = time_to_last_byte_ms.saturating_sub(ttft_ms);
        if decode_ms > 0 {
            self.settled_output_tokens = self
                .settled_output_tokens
                .saturating_add(u64::from(output_tokens));
            self.settled_decode_ms = self.settled_decode_ms.saturating_add(decode_ms);
        }
    }

    fn reset_generation_after_retry(&mut self) {
        if self.completed.is_none() {
            self.text_samples.clear();
            self.last_text_at = None;
            self.settled_output_tokens = 0;
            self.settled_decode_ms = 0;
        }
    }

    fn prune_text_samples(&mut self, now: Instant) {
        while self
            .text_samples
            .front()
            .is_some_and(|sample| now.saturating_duration_since(sample.at) > LIVE_TPS_WINDOW)
        {
            self.text_samples.pop_front();
        }
    }

    fn live_tps(&mut self, now: Instant) -> Option<f64> {
        self.prune_text_samples(now);
        let first = self.text_samples.front()?.at;
        let duration = now.saturating_duration_since(first);
        if duration < LIVE_TPS_MIN_DURATION {
            return None;
        }
        let bytes: u64 = self.text_samples.iter().map(|sample| sample.bytes).sum();
        (bytes > 0)
            .then_some(bytes as f64 / ESTIMATED_BYTES_PER_TOKEN as f64 / duration.as_secs_f64())
    }

    fn generation_metrics(&mut self, now: Instant) -> GenerationMetrics {
        let first_token_ms = self.first_token.map(duration_to_ms);
        if self.completed.is_some() {
            let tokens_per_second = if self.settled_decode_ms > 0 {
                Some(self.settled_output_tokens as f64 / (self.settled_decode_ms as f64 / 1000.0))
            } else {
                self.live_tps(now)
            };
            return GenerationMetrics {
                first_token_ms,
                tokens_per_second,
                estimated: self.settled_decode_ms == 0,
                stale: false,
            };
        }
        let stale = self
            .last_text_at
            .is_some_and(|at| now.saturating_duration_since(at) > LIVE_TPS_STALE_AFTER);
        GenerationMetrics {
            first_token_ms,
            tokens_per_second: (!stale).then(|| self.live_tps(now)).flatten(),
            estimated: true,
            stale,
        }
    }

    fn advance(&mut self, now: Instant) {
        let Some(previous) = self.last_transition_at.replace(now) else {
            return;
        };
        let elapsed = now.saturating_duration_since(previous);
        match self.stack.last() {
            Some(Phase::Sampling) => self.sampling += elapsed,
            Some(Phase::ToolBlocking) => self.tool_blocking += elapsed,
            Some(Phase::Compaction) => self.compaction += elapsed,
            None if self.seen_sampling => self.pending_idle_after_sampling += elapsed,
            None => self.before_first_model += elapsed,
        }
    }

    fn complete(&mut self, now: Instant) -> TurnPhases {
        if let Some(phases) = self.completed.as_ref() {
            return phases.clone();
        }
        let final_phase = self.stack.last().copied();
        self.advance(now);
        let after_last_sampling = if self.seen_sampling {
            std::mem::take(&mut self.pending_idle_after_sampling)
        } else {
            Duration::ZERO
        };

        let mut phases = TurnPhases {
            before_first_model_ms: duration_to_ms(self.before_first_model),
            sampling_ms: duration_to_ms(self.sampling),
            tool_blocking_ms: duration_to_ms(self.tool_blocking),
            compaction_ms: duration_to_ms(self.compaction),
            between_sampling_overhead_ms: duration_to_ms(self.between_sampling_overhead),
            after_last_sampling_ms: duration_to_ms(after_last_sampling),
            turn_total_ms: 0,
            sampling_request_count: self.sampling_request_count,
            sampling_retry_count: self.sampling_retry_count,
            ttft_ms: self.first_token.map(duration_to_ms),
            ttfm_ms: self.first_meaningful.map(duration_to_ms),
        };
        let total_ms = self
            .started_at
            .map(|started_at| duration_to_ms(now.saturating_duration_since(started_at)))
            .unwrap_or_default();
        phases.turn_total_ms = total_ms;
        let classified_ms = phases
            .before_first_model_ms
            .saturating_add(phases.sampling_ms)
            .saturating_add(phases.tool_blocking_ms)
            .saturating_add(phases.compaction_ms)
            .saturating_add(phases.between_sampling_overhead_ms)
            .saturating_add(phases.after_last_sampling_ms);
        let residue_ms = total_ms.saturating_sub(classified_ms);
        match final_phase {
            Some(Phase::Sampling) => phases.sampling_ms += residue_ms,
            Some(Phase::ToolBlocking) => phases.tool_blocking_ms += residue_ms,
            Some(Phase::Compaction) => phases.compaction_ms += residue_ms,
            None if self.seen_sampling => phases.after_last_sampling_ms += residue_ms,
            None => phases.before_first_model_ms += residue_ms,
        }

        self.stack.clear();
        self.completed = Some(phases.clone());
        phases
    }
}

fn duration_to_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttft_stamps_on_any_output_ttfm_on_text_only() {
        let reasoning_or_tool = TurnPhaseProfile::default();
        reasoning_or_tool.start();
        reasoning_or_tool.record_first_token(reasoning_or_tool.current_generation());
        let phases = reasoning_or_tool.complete();
        assert!(phases.ttft_ms.is_some(), "any first output stamps ttft");
        assert_eq!(
            phases.ttfm_ms, None,
            "reasoning and tool calls never stamp ttfm"
        );

        let reasoned_then_text = TurnPhaseProfile::default();
        reasoned_then_text.start();
        let generation = reasoned_then_text.current_generation();
        reasoned_then_text.record_first_token(generation);
        reasoned_then_text.record_first_token(generation);
        reasoned_then_text.record_first_meaningful_output(generation);
        let phases = reasoned_then_text.complete();
        assert!(phases.ttft_ms.is_some(), "reasoning stamps ttft");
        assert!(phases.ttfm_ms.is_some(), "text stamps ttfm");
        assert!(
            phases.ttft_ms <= phases.ttfm_ms,
            "ttft must not exceed ttfm"
        );
    }

    #[test]
    fn live_speed_uses_utf8_bytes_over_the_window() {
        let start = Instant::now();
        let mut state = PhaseState::default();
        state.start(start);
        state.record_text_delta("12345", start + Duration::from_secs(1));

        let metrics = state.generation_metrics(start + Duration::from_secs(2));
        assert_eq!(metrics.tokens_per_second, Some(1.0));
        assert!(metrics.estimated);
        assert!(!metrics.stale);
    }

    #[test]
    fn settled_speed_uses_provider_output_tokens_and_decode_time() {
        let start = Instant::now();
        let mut state = PhaseState::default();
        state.start(start);
        state.record_completed_response(Some(100), Some(200), 1_200);
        state.complete(start + Duration::from_secs(2));

        let metrics = state.generation_metrics(start + Duration::from_secs(2));
        assert_eq!(metrics.tokens_per_second, Some(100.0));
        assert!(!metrics.estimated);
        assert!(!metrics.stale);
    }
}
