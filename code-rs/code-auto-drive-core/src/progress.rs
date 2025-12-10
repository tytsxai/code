//! Progress visualization for Auto Drive sessions.
//!
//! This module provides data structures and logic for collecting and displaying
//! progress information during Auto Drive execution.

use std::time::{Duration, Instant};

use crate::budget::{BudgetAlert, ResourceUsage};
use crate::diagnostics::DiagnosticAlert;
use crate::scheduler::AgentState;

/// Current phase of Auto Drive execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoDrivePhase {
    /// Waiting for user to enter a goal.
    AwaitingGoal,
    /// Initializing the session.
    Initializing,
    /// Actively processing turns.
    Running,
    /// Waiting for user confirmation.
    AwaitingConfirmation,
    /// Paused due to budget constraints.
    PausedBudget,
    /// Paused due to diagnostic alert.
    PausedDiagnostic,
    /// Waiting for user intervention.
    AwaitingIntervention,
    /// Creating a checkpoint.
    Checkpointing,
    /// Recovering from a checkpoint.
    Recovering,
    /// Session completed successfully.
    Completed,
    /// Session stopped by user.
    Stopped,
    /// Session failed with error.
    Failed,
}

impl Default for AutoDrivePhase {
    fn default() -> Self {
        Self::AwaitingGoal
    }
}

/// Token usage breakdown for display.
#[derive(Clone, Debug, Default)]
pub struct TokenMetrics {
    /// Tokens used for input/prompts.
    pub input_tokens: u64,
    /// Tokens used for output/responses.
    pub output_tokens: u64,
    /// Total tokens used.
    pub total_tokens: u64,
    /// Token budget if configured.
    pub budget: Option<u64>,
    /// Percentage of budget used.
    pub budget_percentage: Option<f32>,
}

/// Status of a single agent in multi-agent execution.
#[derive(Clone, Debug)]
pub struct AgentProgress {
    /// Unique identifier for the agent.
    pub agent_id: String,
    /// Current state of the agent.
    pub state: AgentState,
    /// Time spent on this agent.
    pub elapsed: Option<Duration>,
}

/// Notification about history compaction.
#[derive(Clone, Debug)]
pub struct CompactionNotification {
    /// Token count before compaction.
    pub tokens_before: u64,
    /// Token count after compaction.
    pub tokens_after: u64,
    /// Number of items removed.
    pub items_removed: usize,
    /// Timestamp of compaction.
    pub timestamp: Instant,
}

impl CompactionNotification {
    /// Creates a new compaction notification.
    pub fn new(tokens_before: u64, tokens_after: u64, items_removed: usize) -> Self {
        Self {
            tokens_before,
            tokens_after,
            items_removed,
            timestamp: Instant::now(),
        }
    }

    /// Returns the number of tokens saved.
    pub fn tokens_saved(&self) -> u64 {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    /// Returns the savings percentage.
    pub fn savings_percentage(&self) -> f32 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        (self.tokens_saved() as f32 / self.tokens_before as f32) * 100.0
    }
}

/// Complete progress view model for UI display.
#[derive(Clone, Debug)]
pub struct ProgressViewModel {
    /// Current execution phase.
    pub phase: AutoDrivePhase,
    /// Number of turns completed.
    pub turns_completed: usize,
    /// Time elapsed since session start.
    pub elapsed: Duration,
    /// Token usage metrics.
    pub token_metrics: TokenMetrics,
    /// Active agents and their status.
    pub agents: Vec<AgentProgress>,
    /// Current budget alert if any.
    pub budget_alert: Option<BudgetAlert>,
    /// Current diagnostic alert if any.
    pub diagnostic_alert: Option<DiagnosticAlert>,
    /// Recent compaction notification if any.
    pub compaction: Option<CompactionNotification>,
    /// Session goal.
    pub goal: Option<String>,
    /// Whether the session is actively running.
    pub is_active: bool,
}

impl Default for ProgressViewModel {
    fn default() -> Self {
        Self {
            phase: AutoDrivePhase::default(),
            turns_completed: 0,
            elapsed: Duration::ZERO,
            token_metrics: TokenMetrics::default(),
            agents: Vec::new(),
            budget_alert: None,
            diagnostic_alert: None,
            compaction: None,
            goal: None,
            is_active: false,
        }
    }
}

impl ProgressViewModel {
    /// Creates a new progress view model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the phase.
    pub fn set_phase(&mut self, phase: AutoDrivePhase) {
        self.is_active = matches!(
            phase,
            AutoDrivePhase::Running
                | AutoDrivePhase::Initializing
                | AutoDrivePhase::AwaitingConfirmation
                | AutoDrivePhase::Checkpointing
                | AutoDrivePhase::Recovering
        );
        self.phase = phase;
    }

    /// Updates turn count.
    pub fn set_turns(&mut self, turns: usize) {
        self.turns_completed = turns;
    }

    /// Updates elapsed time.
    pub fn set_elapsed(&mut self, elapsed: Duration) {
        self.elapsed = elapsed;
    }

    /// Updates token metrics.
    pub fn set_token_metrics(&mut self, metrics: TokenMetrics) {
        self.token_metrics = metrics;
    }

    /// Updates agent progress.
    pub fn set_agents(&mut self, agents: Vec<AgentProgress>) {
        self.agents = agents;
    }

    /// Sets a budget alert.
    pub fn set_budget_alert(&mut self, alert: Option<BudgetAlert>) {
        self.budget_alert = alert;
    }

    /// Sets a diagnostic alert.
    pub fn set_diagnostic_alert(&mut self, alert: Option<DiagnosticAlert>) {
        self.diagnostic_alert = alert;
    }

    /// Sets a compaction notification.
    pub fn set_compaction(&mut self, notification: Option<CompactionNotification>) {
        self.compaction = notification;
    }

    /// Sets the session goal.
    pub fn set_goal(&mut self, goal: Option<String>) {
        self.goal = goal;
    }

    /// Returns whether all required fields are populated.
    pub fn is_complete(&self) -> bool {
        // Phase is always set (has default)
        // turns_completed has default 0
        // elapsed has default ZERO
        // These are the minimum required fields
        true
    }

    /// Returns a human-readable status string.
    pub fn status_string(&self) -> String {
        match &self.phase {
            AutoDrivePhase::AwaitingGoal => "Awaiting goal".to_string(),
            AutoDrivePhase::Initializing => "Initializing...".to_string(),
            AutoDrivePhase::Running => {
                format!("Running (turn {})", self.turns_completed)
            }
            AutoDrivePhase::AwaitingConfirmation => "Awaiting confirmation".to_string(),
            AutoDrivePhase::PausedBudget => "Paused (budget)".to_string(),
            AutoDrivePhase::PausedDiagnostic => "Paused (diagnostic)".to_string(),
            AutoDrivePhase::AwaitingIntervention => "Awaiting intervention".to_string(),
            AutoDrivePhase::Checkpointing => "Saving checkpoint...".to_string(),
            AutoDrivePhase::Recovering => "Recovering...".to_string(),
            AutoDrivePhase::Completed => {
                format!("Completed ({} turns)", self.turns_completed)
            }
            AutoDrivePhase::Stopped => "Stopped".to_string(),
            AutoDrivePhase::Failed => "Failed".to_string(),
        }
    }
}

/// Collector for gathering progress data from various sources.
pub struct ProgressCollector {
    started_at: Option<Instant>,
    turns_completed: usize,
    token_metrics: TokenMetrics,
    agents: Vec<AgentProgress>,
    phase: AutoDrivePhase,
    goal: Option<String>,
    budget_alert: Option<BudgetAlert>,
    diagnostic_alert: Option<DiagnosticAlert>,
    compaction: Option<CompactionNotification>,
}

impl ProgressCollector {
    /// Creates a new progress collector.
    pub fn new() -> Self {
        Self {
            started_at: None,
            turns_completed: 0,
            token_metrics: TokenMetrics::default(),
            agents: Vec::new(),
            phase: AutoDrivePhase::AwaitingGoal,
            goal: None,
            budget_alert: None,
            diagnostic_alert: None,
            compaction: None,
        }
    }

    /// Starts the session timer.
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
        self.phase = AutoDrivePhase::Initializing;
    }

    /// Sets the session goal.
    pub fn set_goal(&mut self, goal: &str) {
        self.goal = Some(goal.to_string());
    }

    /// Updates the current phase.
    pub fn set_phase(&mut self, phase: AutoDrivePhase) {
        self.phase = phase;
    }

    /// Records a completed turn.
    pub fn record_turn(&mut self) {
        self.turns_completed += 1;
    }

    /// Updates token metrics from resource usage.
    pub fn update_from_usage(&mut self, usage: &ResourceUsage, budget: Option<u64>) {
        self.token_metrics.total_tokens = usage.total_tokens;
        self.token_metrics.budget = budget;
        if let Some(b) = budget {
            if b > 0 {
                self.token_metrics.budget_percentage =
                    Some((usage.total_tokens as f32 / b as f32) * 100.0);
            }
        }
    }

    /// Updates token metrics with input/output breakdown.
    pub fn update_tokens(&mut self, input: u64, output: u64) {
        self.token_metrics.input_tokens = input;
        self.token_metrics.output_tokens = output;
        self.token_metrics.total_tokens = input + output;
    }

    /// Updates agent progress.
    pub fn update_agents(&mut self, agents: Vec<AgentProgress>) {
        self.agents = agents;
    }

    /// Sets a budget alert.
    pub fn set_budget_alert(&mut self, alert: Option<BudgetAlert>) {
        let has_alert = alert.is_some();
        self.budget_alert = alert;
        if has_alert {
            self.phase = AutoDrivePhase::PausedBudget;
        }
    }

    /// Sets a diagnostic alert.
    pub fn set_diagnostic_alert(&mut self, alert: Option<DiagnosticAlert>) {
        let has_alert = alert.is_some();
        self.diagnostic_alert = alert;
        if has_alert {
            self.phase = AutoDrivePhase::PausedDiagnostic;
        }
    }

    /// Records a compaction event.
    pub fn record_compaction(&mut self, tokens_before: u64, tokens_after: u64, items_removed: usize) {
        self.compaction = Some(CompactionNotification::new(
            tokens_before,
            tokens_after,
            items_removed,
        ));
    }

    /// Clears the compaction notification.
    pub fn clear_compaction(&mut self) {
        self.compaction = None;
    }

    /// Marks the session as completed.
    pub fn complete(&mut self) {
        self.phase = AutoDrivePhase::Completed;
    }

    /// Marks the session as stopped.
    pub fn stop(&mut self) {
        self.phase = AutoDrivePhase::Stopped;
    }

    /// Marks the session as failed.
    pub fn fail(&mut self) {
        self.phase = AutoDrivePhase::Failed;
    }

    /// Resets the collector state.
    pub fn reset(&mut self) {
        self.started_at = None;
        self.turns_completed = 0;
        self.token_metrics = TokenMetrics::default();
        self.agents.clear();
        self.phase = AutoDrivePhase::AwaitingGoal;
        self.goal = None;
        self.budget_alert = None;
        self.diagnostic_alert = None;
        self.compaction = None;
    }

    /// Builds a progress view model from current state.
    pub fn build_view_model(&self) -> ProgressViewModel {
        let elapsed = self
            .started_at
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        ProgressViewModel {
            phase: self.phase.clone(),
            turns_completed: self.turns_completed,
            elapsed,
            token_metrics: self.token_metrics.clone(),
            agents: self.agents.clone(),
            budget_alert: self.budget_alert.clone(),
            diagnostic_alert: self.diagnostic_alert.clone(),
            compaction: self.compaction.clone(),
            goal: self.goal.clone(),
            is_active: matches!(
                self.phase,
                AutoDrivePhase::Running
                    | AutoDrivePhase::Initializing
                    | AutoDrivePhase::AwaitingConfirmation
                    | AutoDrivePhase::Checkpointing
                    | AutoDrivePhase::Recovering
            ),
        }
    }
}

impl Default for ProgressCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_view_model_default() {
        let model = ProgressViewModel::default();
        assert_eq!(model.phase, AutoDrivePhase::AwaitingGoal);
        assert_eq!(model.turns_completed, 0);
        assert!(!model.is_active);
        assert!(model.is_complete());
    }

    #[test]
    fn test_progress_view_model_status_string() {
        let mut model = ProgressViewModel::default();

        model.set_phase(AutoDrivePhase::Running);
        model.set_turns(5);
        assert_eq!(model.status_string(), "Running (turn 5)");

        model.set_phase(AutoDrivePhase::Completed);
        assert_eq!(model.status_string(), "Completed (5 turns)");
    }

    #[test]
    fn test_progress_collector_lifecycle() {
        let mut collector = ProgressCollector::new();

        // Initial state
        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::AwaitingGoal);
        assert!(!model.is_active);

        // Start session
        collector.start();
        collector.set_goal("Test goal");
        collector.set_phase(AutoDrivePhase::Running);

        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::Running);
        assert!(model.is_active);
        assert_eq!(model.goal, Some("Test goal".to_string()));

        // Record turns
        collector.record_turn();
        collector.record_turn();

        let model = collector.build_view_model();
        assert_eq!(model.turns_completed, 2);

        // Complete
        collector.complete();

        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::Completed);
        assert!(!model.is_active);
    }

    #[test]
    fn test_compaction_notification() {
        let notification = CompactionNotification::new(10000, 6000, 15);

        assert_eq!(notification.tokens_saved(), 4000);
        assert!((notification.savings_percentage() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_token_metrics_update() {
        let mut collector = ProgressCollector::new();

        collector.update_tokens(500, 300);

        let model = collector.build_view_model();
        assert_eq!(model.token_metrics.input_tokens, 500);
        assert_eq!(model.token_metrics.output_tokens, 300);
        assert_eq!(model.token_metrics.total_tokens, 800);
    }

    #[test]
    fn test_budget_alert_sets_phase() {
        let mut collector = ProgressCollector::new();
        collector.start();
        collector.set_phase(AutoDrivePhase::Running);

        collector.set_budget_alert(Some(BudgetAlert::TokenExceeded {
            used: 1000,
            limit: 1000,
        }));

        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::PausedBudget);
        assert!(model.budget_alert.is_some());
    }

    #[test]
    fn test_diagnostic_alert_sets_phase() {
        let mut collector = ProgressCollector::new();
        collector.start();
        collector.set_phase(AutoDrivePhase::Running);

        collector.set_diagnostic_alert(Some(DiagnosticAlert::LoopDetected {
            tool_name: "test".to_string(),
            count: 3,
        }));

        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::PausedDiagnostic);
        assert!(model.diagnostic_alert.is_some());
    }

    #[test]
    fn test_collector_reset() {
        let mut collector = ProgressCollector::new();
        collector.start();
        collector.set_goal("Test");
        collector.record_turn();
        collector.update_tokens(100, 50);

        collector.reset();

        let model = collector.build_view_model();
        assert_eq!(model.phase, AutoDrivePhase::AwaitingGoal);
        assert_eq!(model.turns_completed, 0);
        assert_eq!(model.token_metrics.total_tokens, 0);
        assert!(model.goal.is_none());
    }
}
