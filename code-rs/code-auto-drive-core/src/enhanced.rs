//! Enhanced Auto Drive integration layer.
//!
//! This module provides the integration layer that connects the new enhancement
//! components (checkpoint, diagnostics, budget, scheduler, audit, telemetry)
//! with the existing AutoCoordinator and AutoDriveController.

use std::path::PathBuf;

use crate::audit::{AuditLogger, AuditOperation, AuditOutcome};
use crate::budget::{BudgetAlert, BudgetConfig, BudgetController};
use crate::checkpoint::{AutoDriveCheckpoint, CheckpointManager, TokenUsage};
use crate::compaction::{CompactionConfig, CompactionEngine, ItemClassification};
use crate::diagnostics::{DiagnosticAlert, DiagnosticsEngine, ToolCallRecord};
use crate::intervention::{InterventionAction, InterventionHandler, InterventionReason};
use crate::progress::{AutoDrivePhase, CompactionNotification, ProgressCollector, ProgressViewModel};
use crate::scheduler::{AgentId, AgentResult, AgentScheduler, AgentTask};
use crate::telemetry::{SessionOutcome, TelemetryCollector, TurnOutcome};
use crate::AutoRunPhase;
use crate::AutoTurnAgentsTiming;

/// Configuration for enhanced Auto Drive features.
#[derive(Clone, Debug)]
pub struct EnhancedConfig {
    /// Enable checkpoint persistence.
    pub checkpoint_enabled: bool,
    /// Directory for checkpoint files.
    pub checkpoint_dir: Option<PathBuf>,
    /// Checkpoint save interval in turns.
    pub checkpoint_interval: u32,

    /// Enable diagnostics engine.
    pub diagnostics_enabled: bool,
    /// Loop detection threshold.
    pub loop_threshold: usize,

    /// Budget configuration.
    pub budget: Option<BudgetConfig>,

    /// Maximum concurrent agents.
    pub max_concurrent_agents: usize,

    /// Enable audit logging.
    pub audit_enabled: bool,
    /// Audit log path.
    pub audit_path: Option<PathBuf>,

    /// Enable telemetry.
    pub telemetry_enabled: bool,

    /// Compaction configuration.
    pub compaction: CompactionConfig,
}

impl Default for EnhancedConfig {
    fn default() -> Self {
        Self {
            checkpoint_enabled: false,
            checkpoint_dir: None,
            checkpoint_interval: 5,
            diagnostics_enabled: true,
            loop_threshold: 3,
            budget: None,
            max_concurrent_agents: 4,
            audit_enabled: false,
            audit_path: None,
            telemetry_enabled: false,
            compaction: CompactionConfig::default(),
        }
    }
}

impl EnhancedConfig {
    /// Creates an `EnhancedConfig` from `AutoDriveSettings`.
    ///
    /// This allows the enhanced features to be configured via the main
    /// `config.toml` file under the `[auto_drive]` section.
    pub fn from_settings(
        checkpoint_enabled: bool,
        checkpoint_dir: Option<PathBuf>,
        checkpoint_interval: u32,
        diagnostics_enabled: bool,
        loop_threshold: u32,
        token_budget: Option<u64>,
        turn_limit: Option<u32>,
        duration_limit_seconds: Option<u64>,
        max_concurrent_agents: usize,
        audit_enabled: bool,
        audit_path: Option<PathBuf>,
        telemetry_enabled: bool,
    ) -> Self {
        let budget = if token_budget.is_some() || turn_limit.is_some() || duration_limit_seconds.is_some() {
            Some(BudgetConfig {
                token_budget,
                turn_limit,
                duration_limit: duration_limit_seconds.map(std::time::Duration::from_secs),
            })
        } else {
            None
        };

        Self {
            checkpoint_enabled,
            checkpoint_dir,
            checkpoint_interval,
            diagnostics_enabled,
            loop_threshold: loop_threshold as usize,
            budget,
            max_concurrent_agents,
            audit_enabled,
            audit_path,
            telemetry_enabled,
            compaction: CompactionConfig::default(),
        }
    }
}

/// Events emitted by the enhanced coordinator.
#[derive(Clone, Debug)]
pub enum EnhancedEvent {
    /// Checkpoint was saved.
    CheckpointSaved {
        session_id: String,
        turns: usize,
    },
    /// Checkpoint was restored.
    CheckpointRestored {
        session_id: String,
        turns: usize,
    },
    /// Diagnostic alert detected.
    DiagnosticAlert(DiagnosticAlert),
    /// Budget alert triggered.
    BudgetAlert(BudgetAlert),
    /// Agent progress update.
    AgentProgress {
        agent_id: String,
        completed: bool,
    },
    /// Intervention required.
    InterventionRequired(InterventionReason),
    /// History was compacted.
    HistoryCompacted(CompactionNotification),
}

/// Enhanced coordinator that wraps the core components.
pub struct EnhancedCoordinator {
    config: EnhancedConfig,
    checkpoint_manager: Option<CheckpointManager>,
    diagnostics: DiagnosticsEngine,
    budget: BudgetController,
    scheduler: AgentScheduler,
    audit: Option<AuditLogger>,
    telemetry: TelemetryCollector,
    compaction: CompactionEngine,
    intervention: InterventionHandler,
    progress: ProgressCollector,

    session_id: Option<String>,
    goal: Option<String>,
    turns_since_checkpoint: u32,
    pending_events: Vec<EnhancedEvent>,
}

impl EnhancedCoordinator {
    /// Creates a new enhanced coordinator with the given configuration.
    pub fn new(config: EnhancedConfig) -> Self {
        let checkpoint_manager = if config.checkpoint_enabled {
            config.checkpoint_dir.as_ref().map(|dir| CheckpointManager::new(dir.clone()))
        } else {
            None
        };

        let audit = if config.audit_enabled {
            Some(AuditLogger::new("session"))
        } else {
            None
        };

        let mut diagnostics = DiagnosticsEngine::new();
        if config.loop_threshold > 0 {
            diagnostics = DiagnosticsEngine::with_thresholds(
                crate::diagnostics::AnomalyThreshold {
                    loop_count: config.loop_threshold,
                    ..Default::default()
                }
            );
        }

        let mut budget = BudgetController::new();
        if let Some(budget_config) = &config.budget {
            budget.configure(budget_config.clone());
        }

        Self {
            checkpoint_manager,
            diagnostics,
            budget,
            scheduler: AgentScheduler::new(config.max_concurrent_agents),
            audit,
            telemetry: TelemetryCollector::new(),
            compaction: CompactionEngine::with_config(config.compaction.clone()),
            intervention: InterventionHandler::new(),
            progress: ProgressCollector::new(),
            config,
            session_id: None,
            goal: None,
            turns_since_checkpoint: 0,
            pending_events: Vec::new(),
        }
    }

    /// Starts a new session.
    pub fn start_session(&mut self, goal: &str, session_id: &str) {
        self.session_id = Some(session_id.to_string());
        self.goal = Some(goal.to_string());
        self.turns_since_checkpoint = 0;

        self.diagnostics.set_goal(goal);
        self.budget.start();
        self.progress.start();
        self.progress.set_goal(goal);
        self.progress.set_phase(AutoDrivePhase::Running);

        if self.config.telemetry_enabled {
            self.telemetry.start_session(goal, session_id);
        }

        if let Some(manager) = &mut self.checkpoint_manager {
            if let Ok(checkpoint) = manager.create(goal, session_id) {
                tracing::debug!(session_id, "Created initial checkpoint");
                let _ = checkpoint; // Initial checkpoint created
            }
        }

        if let Some(audit) = &mut self.audit {
            audit.log(
                AuditOperation::CheckpointSave {
                    checkpoint_id: session_id.to_string(),
                },
                AuditOutcome::Success,
            );
        }
    }

    /// Records a completed turn.
    pub fn record_turn(&mut self, tokens_used: u64) {
        self.budget.record_usage(tokens_used, true);
        self.progress.record_turn();
        self.turns_since_checkpoint += 1;

        // Check for checkpoint
        if self.config.checkpoint_enabled
            && self.turns_since_checkpoint >= self.config.checkpoint_interval
        {
            self.save_checkpoint();
        }

        // Check budget
        if let Some(alert) = self.budget.check_budget() {
            self.pending_events.push(EnhancedEvent::BudgetAlert(alert.clone()));
            self.progress.set_budget_alert(Some(alert.clone()));

            if self.budget.should_pause() {
                self.intervention.request_for_budget(&alert);
                self.pending_events.push(EnhancedEvent::InterventionRequired(
                    InterventionReason::BudgetDecision {
                        alert: crate::intervention::BudgetAlertKind::from(&alert),
                    },
                ));
            }
        }

        // Check diagnostics
        let report = self.diagnostics.generate_report();
        for alert in report.alerts {
            self.pending_events.push(EnhancedEvent::DiagnosticAlert(alert.clone()));
            self.progress.set_diagnostic_alert(Some(alert.clone()));
            self.intervention.request_for_diagnostic(&alert);
        }

        // Telemetry
        if self.config.telemetry_enabled {
            let turn_num = self.progress.build_view_model().turns_completed as u32;
            let span = self.telemetry.start_turn(turn_num);
            self.telemetry.end_turn(span, TurnOutcome::Success { tokens_used });
        }
    }

    /// Records a tool call for diagnostics.
    pub fn record_tool_call(&mut self, record: ToolCallRecord) {
        self.diagnostics.record_tool_call(record.clone());

        if let Some(audit) = &mut self.audit {
            audit.log(
                AuditOperation::ToolExecution {
                    tool: record.tool_name,
                    args_hash: record.arguments_hash,
                },
                match record.outcome {
                    crate::diagnostics::ToolOutcome::Success => AuditOutcome::Success,
                    crate::diagnostics::ToolOutcome::Failure(msg) => AuditOutcome::Failure(msg),
                    crate::diagnostics::ToolOutcome::Timeout => AuditOutcome::Failure("timeout".to_string()),
                },
            );
        }
    }

    /// Schedules agent tasks.
    pub fn schedule_agents(&mut self, tasks: Vec<AgentTask>, timing: AutoTurnAgentsTiming) {
        self.scheduler.schedule(tasks, timing);
    }

    /// Gets the next runnable agent task.
    pub fn next_agent(&mut self) -> Option<AgentTask> {
        self.scheduler.next_runnable()
    }

    /// Reports agent completion.
    pub fn report_agent_completion(&mut self, id: AgentId, result: String, order: Option<usize>) {
        self.scheduler.report_completion_with_order(id, result, order);
        self.pending_events.push(EnhancedEvent::AgentProgress {
            agent_id: format!("{}", id.0),
            completed: true,
        });
    }

    /// Collects all agent results.
    pub fn collect_agent_results(&mut self) -> Vec<AgentResult> {
        self.scheduler.collect_results()
    }

    /// Checks if compaction should be triggered.
    pub fn should_compact(&self, current_tokens: u64, context_limit: u64) -> bool {
        self.compaction.should_compact(current_tokens, context_limit)
    }

    /// Performs compaction on history items.
    pub fn compact_history(&mut self, items: &[ItemClassification]) -> crate::compaction::CompactionResult {
        let result = self.compaction.compact(items);

        let notification = CompactionNotification::new(
            result.tokens_before,
            result.tokens_after,
            result.remove_indices.len(),
        );
        self.progress.record_compaction(
            result.tokens_before,
            result.tokens_after,
            result.remove_indices.len(),
        );
        self.pending_events.push(EnhancedEvent::HistoryCompacted(notification));

        result
    }

    /// Handles an intervention action.
    pub fn handle_intervention(&mut self, action: InterventionAction) {
        self.intervention.resolve(action);
    }

    /// Takes the resolved intervention action.
    pub fn take_intervention_action(&mut self) -> Option<InterventionAction> {
        self.intervention.take_action()
    }

    /// Returns whether an intervention is pending.
    pub fn intervention_pending(&self) -> bool {
        self.intervention.is_awaiting_input()
    }

    /// Gets the current progress view model.
    pub fn progress(&self) -> ProgressViewModel {
        self.progress.build_view_model()
    }

    /// Takes pending events.
    pub fn take_events(&mut self) -> Vec<EnhancedEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Ends the session.
    pub fn end_session(&mut self, success: bool) {
        self.progress.set_phase(if success {
            AutoDrivePhase::Completed
        } else {
            AutoDrivePhase::Stopped
        });

        if self.config.telemetry_enabled {
            let model = self.progress.build_view_model();
            self.telemetry.end_session(SessionOutcome::Completed {
                turns: model.turns_completed as u32,
                success,
            });
        }

        // Final checkpoint
        if self.config.checkpoint_enabled {
            self.save_checkpoint();
        }
    }

    /// Attempts to restore from a checkpoint.
    pub fn restore_session(&mut self, session_id: &str) -> Option<AutoDriveCheckpoint> {
        let manager = self.checkpoint_manager.as_ref()?;
        let checkpoint = manager.restore(session_id).ok()??;

        self.session_id = Some(checkpoint.session_id.clone());
        self.goal = Some(checkpoint.goal.clone());
        self.diagnostics.set_goal(&checkpoint.goal);

        self.pending_events.push(EnhancedEvent::CheckpointRestored {
            session_id: checkpoint.session_id.clone(),
            turns: checkpoint.turns_completed,
        });

        Some(checkpoint)
    }

    /// Lists recoverable sessions.
    pub fn list_recoverable_sessions(&self) -> Vec<crate::checkpoint::CheckpointSummary> {
        self.checkpoint_manager
            .as_ref()
            .and_then(|m| m.list_recoverable().ok())
            .unwrap_or_default()
    }

    fn save_checkpoint(&mut self) {
        let Some(manager) = &mut self.checkpoint_manager else {
            return;
        };
        let Some(session_id) = &self.session_id else {
            return;
        };
        let Some(goal) = &self.goal else {
            return;
        };

        let model = self.progress.build_view_model();
        let token_usage = TokenUsage {
            input_tokens: model.token_metrics.input_tokens,
            output_tokens: model.token_metrics.output_tokens,
            total_tokens: model.token_metrics.total_tokens,
        };

        if let Ok(mut checkpoint) = manager.create(goal, session_id) {
            if manager.update(
                &mut checkpoint,
                vec![],
                model.turns_completed,
                token_usage,
                &AutoRunPhase::Active,
            ).is_ok() {
                self.turns_since_checkpoint = 0;
                self.pending_events.push(EnhancedEvent::CheckpointSaved {
                    session_id: session_id.clone(),
                    turns: model.turns_completed,
                });
            }
        }
    }

    /// Resets the coordinator state.
    pub fn reset(&mut self) {
        self.session_id = None;
        self.goal = None;
        self.turns_since_checkpoint = 0;
        self.pending_events.clear();

        self.diagnostics.reset();
        self.budget.reset();
        self.scheduler.reset();
        self.intervention.clear();
        self.progress.reset();

        if let Some(audit) = &mut self.audit {
            audit.clear();
        }
    }
}

impl Default for EnhancedCoordinator {
    fn default() -> Self {
        Self::new(EnhancedConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_coordinator_lifecycle() {
        let mut coord = EnhancedCoordinator::new(EnhancedConfig {
            diagnostics_enabled: true,
            ..Default::default()
        });

        coord.start_session("Test goal", "test-session");

        let progress = coord.progress();
        assert_eq!(progress.phase, AutoDrivePhase::Running);
        assert!(progress.is_active);

        coord.record_turn(100);
        coord.record_turn(150);

        let progress = coord.progress();
        assert_eq!(progress.turns_completed, 2);

        coord.end_session(true);

        let progress = coord.progress();
        assert_eq!(progress.phase, AutoDrivePhase::Completed);
    }

    #[test]
    fn test_budget_alert_triggers_intervention() {
        let mut coord = EnhancedCoordinator::new(EnhancedConfig {
            budget: Some(BudgetConfig {
                token_budget: Some(100),
                ..Default::default()
            }),
            ..Default::default()
        });

        coord.start_session("Test", "session");
        coord.record_turn(100); // Exceeds budget

        assert!(coord.intervention_pending());

        let events = coord.take_events();
        assert!(events.iter().any(|e| matches!(e, EnhancedEvent::BudgetAlert(_))));
    }

    #[test]
    fn test_agent_scheduling() {
        let mut coord = EnhancedCoordinator::default();

        let tasks = vec![
            AgentTask {
                id: AgentId(1),
                prompt: "Task 1".to_string(),
                context: None,
                write_access: false,
                models: None,
                dispatch_order: 0,
            },
            AgentTask {
                id: AgentId(2),
                prompt: "Task 2".to_string(),
                context: None,
                write_access: false,
                models: None,
                dispatch_order: 1,
            },
        ];

        coord.schedule_agents(tasks, AutoTurnAgentsTiming::Parallel);

        let task1 = coord.next_agent();
        assert!(task1.is_some());

        let task2 = coord.next_agent();
        assert!(task2.is_some());

        coord.report_agent_completion(AgentId(1), "Result 1".to_string(), Some(0));
        coord.report_agent_completion(AgentId(2), "Result 2".to_string(), Some(1));

        let results = coord.collect_agent_results();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_reset() {
        let mut coord = EnhancedCoordinator::default();

        coord.start_session("Test", "session");
        coord.record_turn(100);

        coord.reset();

        let progress = coord.progress();
        assert_eq!(progress.phase, AutoDrivePhase::AwaitingGoal);
        assert_eq!(progress.turns_completed, 0);
    }
}
