//! Telemetry collector for observability.
//!
//! This module provides structured telemetry collection for Auto Drive sessions,
//! including span management and metrics tracking.

use std::time::{Duration, Instant};

use chrono::Utc;

/// Context for a telemetry span.
#[derive(Clone, Debug)]
pub struct SpanContext {
    pub span_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub started_at: Instant,
    pub attributes: Vec<(String, SpanAttribute)>,
}

/// Attribute value types for spans.
#[derive(Clone, Debug)]
pub enum SpanAttribute {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// A span representing a turn in the Auto Drive session.
pub struct TurnSpan {
    pub turn_number: u32,
    pub started_at: Instant,
    pub span_id: String,
}

/// Outcome of a turn.
#[derive(Clone, Debug)]
pub enum TurnOutcome {
    Success { tokens_used: u64 },
    Failure { error: String },
    Skipped { reason: String },
}

/// Outcome of a session.
#[derive(Clone, Debug)]
pub enum SessionOutcome {
    Completed { turns: u32, success: bool },
    Interrupted { reason: String },
    Failed { error: String },
}

/// Metrics collected during a session.
#[derive(Clone, Debug, Default)]
pub struct SessionMetrics {
    pub total_turns: u32,
    pub successful_turns: u32,
    pub failed_turns: u32,
    pub total_tokens: u64,
    pub total_duration: Duration,
    pub average_turn_duration: Duration,
    pub errors: Vec<String>,
}

/// Collector for telemetry data.
pub struct TelemetryCollector {
    session_span: Option<SpanContext>,
    turn_spans: Vec<SpanContext>,
    metrics: SessionMetrics,
    debug_enabled: bool,
    session_started_at: Option<Instant>,
}

impl TelemetryCollector {
    /// Creates a new TelemetryCollector.
    pub fn new() -> Self {
        Self {
            session_span: None,
            turn_spans: Vec::new(),
            metrics: SessionMetrics::default(),
            debug_enabled: false,
            session_started_at: None,
        }
    }

    /// Enables debug logging.
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_enabled = enabled;
        self
    }

    /// Starts tracking a session.
    pub fn start_session(&mut self, goal: &str, session_id: &str) {
        let span_id = format!("session-{session_id}");
        self.session_started_at = Some(Instant::now());

        self.session_span = Some(SpanContext {
            span_id: span_id.clone(),
            parent_id: None,
            name: "auto_drive_session".to_string(),
            started_at: Instant::now(),
            attributes: vec![
                ("goal".to_string(), SpanAttribute::String(goal.to_string())),
                (
                    "session_id".to_string(),
                    SpanAttribute::String(session_id.to_string()),
                ),
            ],
        });

        tracing::info!(
            session_id = session_id,
            goal = goal,
            "Auto Drive session started"
        );
    }

    /// Starts tracking a turn.
    pub fn start_turn(&mut self, turn_number: u32) -> TurnSpan {
        let span_id = format!(
            "turn-{}-{}",
            turn_number,
            Utc::now().timestamp_millis()
        );

        if self.debug_enabled {
            tracing::debug!(turn = turn_number, "Turn started");
        }

        TurnSpan {
            turn_number,
            started_at: Instant::now(),
            span_id,
        }
    }

    /// Ends tracking a turn.
    pub fn end_turn(&mut self, span: TurnSpan, outcome: TurnOutcome) {
        let duration = span.started_at.elapsed();

        let mut attributes = vec![
            (
                "turn_number".to_string(),
                SpanAttribute::Int(span.turn_number as i64),
            ),
            (
                "duration_ms".to_string(),
                SpanAttribute::Int(duration.as_millis() as i64),
            ),
        ];

        match &outcome {
            TurnOutcome::Success { tokens_used } => {
                attributes.push((
                    "tokens_used".to_string(),
                    SpanAttribute::Int(*tokens_used as i64),
                ));
                attributes.push(("success".to_string(), SpanAttribute::Bool(true)));
                self.metrics.successful_turns += 1;
                self.metrics.total_tokens += tokens_used;
            }
            TurnOutcome::Failure { error } => {
                attributes.push(("success".to_string(), SpanAttribute::Bool(false)));
                attributes.push(("error".to_string(), SpanAttribute::String(error.clone())));
                self.metrics.failed_turns += 1;
                self.metrics.errors.push(error.clone());
            }
            TurnOutcome::Skipped { reason } => {
                attributes.push(("skipped".to_string(), SpanAttribute::Bool(true)));
                attributes.push(("reason".to_string(), SpanAttribute::String(reason.clone())));
            }
        }

        self.metrics.total_turns += 1;

        let span_context = SpanContext {
            span_id: span.span_id,
            parent_id: self.session_span.as_ref().map(|s| s.span_id.clone()),
            name: format!("turn_{}", span.turn_number),
            started_at: span.started_at,
            attributes,
        };

        self.turn_spans.push(span_context);

        if self.debug_enabled {
            tracing::debug!(
                turn = span.turn_number,
                duration_ms = duration.as_millis(),
                outcome = ?outcome,
                "Turn completed"
            );
        }
    }

    /// Records an error.
    pub fn record_error(&mut self, error: &anyhow::Error) {
        let error_str = format!("{error:#}");
        self.metrics.errors.push(error_str.clone());

        tracing::error!(error = %error_str, "Auto Drive error");

        // Add error to session span attributes
        if let Some(span) = &mut self.session_span {
            span.attributes
                .push(("error".to_string(), SpanAttribute::String(error_str)));
        }
    }

    /// Ends tracking a session.
    pub fn end_session(&mut self, outcome: SessionOutcome) {
        if let Some(started) = self.session_started_at {
            self.metrics.total_duration = started.elapsed();
        }

        if self.metrics.total_turns > 0 {
            self.metrics.average_turn_duration =
                self.metrics.total_duration / self.metrics.total_turns;
        }

        let (success, message) = match &outcome {
            SessionOutcome::Completed { turns, success } => {
                (*success, format!("Completed after {turns} turns"))
            }
            SessionOutcome::Interrupted { reason } => (false, format!("Interrupted: {reason}")),
            SessionOutcome::Failed { error } => (false, format!("Failed: {error}")),
        };

        tracing::info!(
            success = success,
            total_turns = self.metrics.total_turns,
            total_tokens = self.metrics.total_tokens,
            duration_ms = self.metrics.total_duration.as_millis(),
            message = message,
            "Auto Drive session ended"
        );

        // Update session span
        if let Some(span) = &mut self.session_span {
            span.attributes
                .push(("success".to_string(), SpanAttribute::Bool(success)));
            span.attributes.push((
                "total_turns".to_string(),
                SpanAttribute::Int(self.metrics.total_turns as i64),
            ));
            span.attributes.push((
                "total_tokens".to_string(),
                SpanAttribute::Int(self.metrics.total_tokens as i64),
            ));
        }
    }

    /// Logs a decision in debug mode.
    pub fn log_decision(&self, decision_json: &str) {
        if self.debug_enabled {
            tracing::debug!(decision = decision_json, "Coordinator decision");
        }
    }

    /// Logs a state transition in debug mode.
    pub fn log_state_transition(&self, from: &str, to: &str) {
        if self.debug_enabled {
            tracing::debug!(from = from, to = to, "State transition");
        }
    }

    /// Exports collected metrics.
    pub fn export_metrics(&self) -> SessionMetrics {
        self.metrics.clone()
    }

    /// Returns the session span.
    pub fn session_span(&self) -> Option<&SpanContext> {
        self.session_span.as_ref()
    }

    /// Returns all turn spans.
    pub fn turn_spans(&self) -> &[SpanContext] {
        &self.turn_spans
    }

    /// Resets the collector.
    pub fn reset(&mut self) {
        self.session_span = None;
        self.turn_spans.clear();
        self.metrics = SessionMetrics::default();
        self.session_started_at = None;
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let mut collector = TelemetryCollector::new();

        collector.start_session("Test goal", "session-123");
        assert!(collector.session_span().is_some());

        let turn1 = collector.start_turn(1);
        collector.end_turn(turn1, TurnOutcome::Success { tokens_used: 100 });

        let turn2 = collector.start_turn(2);
        collector.end_turn(turn2, TurnOutcome::Success { tokens_used: 150 });

        collector.end_session(SessionOutcome::Completed {
            turns: 2,
            success: true,
        });

        let metrics = collector.export_metrics();
        assert_eq!(metrics.total_turns, 2);
        assert_eq!(metrics.successful_turns, 2);
        assert_eq!(metrics.total_tokens, 250);
    }

    #[test]
    fn test_turn_failure() {
        let mut collector = TelemetryCollector::new();

        collector.start_session("Test", "session-456");

        let turn = collector.start_turn(1);
        collector.end_turn(
            turn,
            TurnOutcome::Failure {
                error: "Test error".to_string(),
            },
        );

        let metrics = collector.export_metrics();
        assert_eq!(metrics.failed_turns, 1);
        assert_eq!(metrics.errors.len(), 1);
    }

    #[test]
    fn test_error_recording() {
        let mut collector = TelemetryCollector::new();

        collector.start_session("Test", "session-789");
        collector.record_error(&anyhow::anyhow!("Test error"));

        let metrics = collector.export_metrics();
        assert_eq!(metrics.errors.len(), 1);
    }

    #[test]
    fn test_span_coverage() {
        let mut collector = TelemetryCollector::new();

        collector.start_session("Test", "session-abc");

        for i in 1..=3 {
            let turn = collector.start_turn(i);
            collector.end_turn(turn, TurnOutcome::Success { tokens_used: 50 });
        }

        collector.end_session(SessionOutcome::Completed {
            turns: 3,
            success: true,
        });

        // Verify span coverage
        assert!(collector.session_span().is_some());
        assert_eq!(collector.turn_spans().len(), 3);

        // Each turn span should have the session as parent
        for turn_span in collector.turn_spans() {
            assert!(turn_span.parent_id.is_some());
        }
    }

    #[test]
    fn test_debug_mode() {
        let collector = TelemetryCollector::new().with_debug(true);
        assert!(collector.debug_enabled);

        // These should not panic
        collector.log_decision(r#"{"status": "continue"}"#);
        collector.log_state_transition("Idle", "Active");
    }
}
