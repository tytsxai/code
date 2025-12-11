//! Diagnostics engine for detecting anomalous Auto Drive behavior.
//!
//! This module provides loop detection, goal drift analysis, and token anomaly
//! detection to help identify when Auto Drive is stuck or behaving unexpectedly.

use std::collections::VecDeque;
use std::hash::Hash;
use std::hash::Hasher;
use std::time::Instant;

/// Maximum number of tool calls to track for loop detection.
const TOOL_CALL_WINDOW: usize = 10;

/// Maximum number of response patterns to track.
const RESPONSE_PATTERN_WINDOW: usize = 5;

/// Default threshold for consecutive identical tool calls.
const DEFAULT_LOOP_THRESHOLD: usize = 3;

/// Record of a tool call for diagnostics.
#[derive(Clone, Debug)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments_hash: u64,
    pub timestamp: Instant,
    pub outcome: ToolOutcome,
}

/// Outcome of a tool execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolOutcome {
    Success,
    Failure(String),
    Timeout,
}

/// Pattern extracted from a model response.
#[derive(Clone, Debug)]
pub struct ResponsePattern {
    pub content_hash: u64,
    pub timestamp: Instant,
}

/// Token usage projection for anomaly detection.
#[derive(Clone, Debug, Default)]
pub struct TokenProjection {
    pub projected_total: u64,
    pub actual_total: u64,
    pub turns_projected: usize,
}

/// Thresholds for anomaly detection.
#[derive(Clone, Debug)]
pub struct AnomalyThreshold {
    pub loop_count: usize,
    pub token_overrun_ratio: f32,
    pub repetitive_response_count: usize,
}

impl Default for AnomalyThreshold {
    fn default() -> Self {
        Self {
            loop_count: DEFAULT_LOOP_THRESHOLD,
            token_overrun_ratio: 1.5,
            repetitive_response_count: 3,
        }
    }
}

/// Diagnostic alerts emitted by the engine.
#[derive(Clone, Debug)]
pub enum DiagnosticAlert {
    /// Same tool called with identical arguments multiple times.
    LoopDetected { tool_name: String, count: usize },
    /// Current context has drifted from original goal.
    GoalDrift {
        similarity_score: f32,
        original: String,
        current: String,
    },
    /// Token usage significantly exceeds projection.
    TokenOverrun {
        projected: u64,
        actual: u64,
        ratio: f32,
    },
    /// Model responses showing repetitive patterns.
    RepetitiveResponse { pattern: String, occurrences: usize },
    /// Session running slower than threshold.
    SessionSlow {
        session_id: String,
        elapsed_ms: i64,
        task_id: Option<String>,
    },
    /// Session exceeded stuck threshold.
    SessionStuck {
        session_id: String,
        elapsed_ms: i64,
        task_id: Option<String>,
    },
    /// Session task migrated to a new worker.
    SessionMigrated {
        from_session: String,
        to_session: Option<String>,
        task_id: String,
        retry_count: i32,
    },
    /// Parallel execution configured with low concurrency.
    LowConcurrency { max_concurrent_agents: i32 },
}

/// Summary report of diagnostic findings.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticReport {
    pub alerts: Vec<DiagnosticAlert>,
    pub tool_calls_analyzed: usize,
    pub responses_analyzed: usize,
    pub token_projection: TokenProjection,
}

/// Engine for detecting anomalous Auto Drive behavior.
pub struct DiagnosticsEngine {
    tool_call_history: VecDeque<ToolCallRecord>,
    response_patterns: VecDeque<ResponsePattern>,
    token_projection: TokenProjection,
    anomaly_threshold: AnomalyThreshold,
    original_goal: Option<String>,
}

impl DiagnosticsEngine {
    /// Creates a new DiagnosticsEngine with default thresholds.
    pub fn new() -> Self {
        Self {
            tool_call_history: VecDeque::with_capacity(TOOL_CALL_WINDOW),
            response_patterns: VecDeque::with_capacity(RESPONSE_PATTERN_WINDOW),
            token_projection: TokenProjection::default(),
            anomaly_threshold: AnomalyThreshold::default(),
            original_goal: None,
        }
    }

    /// Creates a new DiagnosticsEngine with custom thresholds.
    pub fn with_thresholds(thresholds: AnomalyThreshold) -> Self {
        Self {
            anomaly_threshold: thresholds,
            ..Self::new()
        }
    }

    /// Sets the original goal for drift detection.
    pub fn set_goal(&mut self, goal: &str) {
        self.original_goal = Some(goal.to_string());
    }

    /// Sets the projected token usage for anomaly detection.
    pub fn set_projection(&mut self, projected_total: u64, turns: usize) {
        self.token_projection.projected_total = projected_total;
        self.token_projection.turns_projected = turns;
    }

    /// Sets the loop detection threshold.
    pub fn set_loop_threshold(&mut self, threshold: usize) {
        self.anomaly_threshold.loop_count = threshold;
    }

    /// Records a tool call for analysis.
    pub fn record_tool_call(&mut self, record: ToolCallRecord) {
        if self.tool_call_history.len() >= TOOL_CALL_WINDOW {
            self.tool_call_history.pop_front();
        }
        self.tool_call_history.push_back(record);
    }

    /// Records a model response for pattern analysis.
    pub fn record_response(&mut self, response: &str) {
        let hash = Self::hash_content(response);
        let pattern = ResponsePattern {
            content_hash: hash,
            timestamp: Instant::now(),
        };

        if self.response_patterns.len() >= RESPONSE_PATTERN_WINDOW {
            self.response_patterns.pop_front();
        }
        self.response_patterns.push_back(pattern);
    }

    /// Updates actual token usage.
    pub fn update_token_usage(&mut self, actual_total: u64) {
        self.token_projection.actual_total = actual_total;
    }

    /// Checks for loop behavior in tool calls.
    pub fn check_loop(&self) -> Option<DiagnosticAlert> {
        if self.tool_call_history.len() < self.anomaly_threshold.loop_count {
            return None;
        }

        // Check for consecutive identical tool calls
        let recent: Vec<_> = self
            .tool_call_history
            .iter()
            .rev()
            .take(self.anomaly_threshold.loop_count)
            .collect();

        if recent.is_empty() {
            return None;
        }

        let first = &recent[0];
        let all_same = recent
            .iter()
            .all(|r| r.tool_name == first.tool_name && r.arguments_hash == first.arguments_hash);

        if all_same {
            Some(DiagnosticAlert::LoopDetected {
                tool_name: first.tool_name.clone(),
                count: recent.len(),
            })
        } else {
            None
        }
    }

    /// Checks for goal drift.
    pub fn check_goal_drift(&self, current_context: &str) -> Option<DiagnosticAlert> {
        let original = self.original_goal.as_ref()?;

        let similarity = Self::calculate_similarity(original, current_context);

        // Alert if similarity drops below 30%
        if similarity < 0.3 {
            Some(DiagnosticAlert::GoalDrift {
                similarity_score: similarity,
                original: original.chars().take(100).collect(),
                current: current_context.chars().take(100).collect(),
            })
        } else {
            None
        }
    }

    /// Checks for token usage anomalies.
    pub fn check_token_anomaly(&self) -> Option<DiagnosticAlert> {
        if self.token_projection.projected_total == 0 {
            return None;
        }

        let ratio = self.token_projection.actual_total as f32
            / self.token_projection.projected_total as f32;

        if ratio > self.anomaly_threshold.token_overrun_ratio {
            Some(DiagnosticAlert::TokenOverrun {
                projected: self.token_projection.projected_total,
                actual: self.token_projection.actual_total,
                ratio,
            })
        } else {
            None
        }
    }

    /// Checks for repetitive response patterns.
    pub fn check_repetitive_responses(&self) -> Option<DiagnosticAlert> {
        if self.response_patterns.len() < self.anomaly_threshold.repetitive_response_count {
            return None;
        }

        // Count occurrences of each pattern
        let mut counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        for pattern in &self.response_patterns {
            *counts.entry(pattern.content_hash).or_insert(0) += 1;
        }

        // Find patterns that exceed threshold
        for (hash, count) in counts {
            if count >= self.anomaly_threshold.repetitive_response_count {
                return Some(DiagnosticAlert::RepetitiveResponse {
                    pattern: format!("hash:{hash:x}"),
                    occurrences: count,
                });
            }
        }

        None
    }

    /// Generates a comprehensive diagnostic report.
    pub fn generate_report(&self) -> DiagnosticReport {
        let mut alerts = Vec::new();

        if let Some(alert) = self.check_loop() {
            alerts.push(alert);
        }
        if let Some(alert) = self.check_token_anomaly() {
            alerts.push(alert);
        }
        if let Some(alert) = self.check_repetitive_responses() {
            alerts.push(alert);
        }

        DiagnosticReport {
            alerts,
            tool_calls_analyzed: self.tool_call_history.len(),
            responses_analyzed: self.response_patterns.len(),
            token_projection: self.token_projection.clone(),
        }
    }

    /// Resets the diagnostics engine state.
    pub fn reset(&mut self) {
        self.tool_call_history.clear();
        self.response_patterns.clear();
        self.token_projection = TokenProjection::default();
        self.original_goal = None;
    }

    fn hash_content(content: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn calculate_similarity(a: &str, b: &str) -> f32 {
        // Simple word overlap similarity
        let words_a: std::collections::HashSet<_> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<_> = b.split_whitespace().collect();

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}

impl Default for DiagnosticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_detection() {
        let mut engine = DiagnosticsEngine::new();

        // Add identical tool calls
        for _ in 0..3 {
            engine.record_tool_call(ToolCallRecord {
                tool_name: "read_file".to_string(),
                arguments_hash: 12345,
                timestamp: Instant::now(),
                outcome: ToolOutcome::Success,
            });
        }

        let alert = engine.check_loop();
        assert!(alert.is_some());

        if let Some(DiagnosticAlert::LoopDetected { tool_name, count }) = alert {
            assert_eq!(tool_name, "read_file");
            assert_eq!(count, 3);
        }
    }

    #[test]
    fn test_no_loop_with_different_calls() {
        let mut engine = DiagnosticsEngine::new();

        engine.record_tool_call(ToolCallRecord {
            tool_name: "read_file".to_string(),
            arguments_hash: 111,
            timestamp: Instant::now(),
            outcome: ToolOutcome::Success,
        });
        engine.record_tool_call(ToolCallRecord {
            tool_name: "write_file".to_string(),
            arguments_hash: 222,
            timestamp: Instant::now(),
            outcome: ToolOutcome::Success,
        });
        engine.record_tool_call(ToolCallRecord {
            tool_name: "read_file".to_string(),
            arguments_hash: 333,
            timestamp: Instant::now(),
            outcome: ToolOutcome::Success,
        });

        assert!(engine.check_loop().is_none());
    }

    #[test]
    fn test_token_anomaly_detection() {
        let mut engine = DiagnosticsEngine::new();
        engine.set_projection(1000, 10);
        engine.update_token_usage(1600); // 60% over projection

        let alert = engine.check_token_anomaly();
        assert!(alert.is_some());

        if let Some(DiagnosticAlert::TokenOverrun { ratio, .. }) = alert {
            assert!(ratio > 1.5);
        }
    }

    #[test]
    fn test_no_token_anomaly_within_threshold() {
        let mut engine = DiagnosticsEngine::new();
        engine.set_projection(1000, 10);
        engine.update_token_usage(1200); // 20% over, within threshold

        assert!(engine.check_token_anomaly().is_none());
    }

    #[test]
    fn test_goal_drift_detection() {
        let mut engine = DiagnosticsEngine::new();
        engine.set_goal("implement user authentication with JWT tokens");

        // Very similar context - no drift (high word overlap)
        let similar = "implement user authentication with JWT tokens and validation";
        assert!(engine.check_goal_drift(similar).is_none());

        // Very different context - drift detected (low word overlap)
        let different = "optimizing database queries for performance tuning";
        let alert = engine.check_goal_drift(different);
        assert!(alert.is_some());
    }

    #[test]
    fn test_repetitive_response_detection() {
        let mut engine = DiagnosticsEngine::new();

        // Add same response multiple times
        for _ in 0..3 {
            engine.record_response("I'll help you with that task.");
        }

        let alert = engine.check_repetitive_responses();
        assert!(alert.is_some());
    }

    #[test]
    fn test_generate_report() {
        let mut engine = DiagnosticsEngine::new();

        // Add some data
        for _ in 0..3 {
            engine.record_tool_call(ToolCallRecord {
                tool_name: "test".to_string(),
                arguments_hash: 999,
                timestamp: Instant::now(),
                outcome: ToolOutcome::Success,
            });
        }

        let report = engine.generate_report();
        assert_eq!(report.tool_calls_analyzed, 3);
        assert!(!report.alerts.is_empty());
    }
}
