//! Enhanced Auto Drive integration layer.
//!
//! This module provides the integration layer that connects the new enhancement
//! components (checkpoint, diagnostics, budget, scheduler, audit, telemetry)
//! with the existing AutoCoordinator and AutoDriveController.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::AutoRunPhase;
use crate::AutoTurnAgentsTiming;
use crate::audit::AuditLogger;
use crate::audit::AuditOperation;
use crate::audit::AuditOutcome;
use crate::backlog::BacklogManager;
use crate::budget::BudgetAlert;
use crate::budget::BudgetConfig;
use crate::budget::BudgetController;
use crate::checkpoint::AutoDriveCheckpoint;
use crate::checkpoint::CheckpointManager;
use crate::checkpoint::TokenUsage;
use crate::compaction::CompactionConfig;
use crate::compaction::CompactionEngine;
use crate::compaction::ItemClassification;
use crate::diagnostics::DiagnosticAlert;
use crate::diagnostics::DiagnosticsEngine;
use crate::diagnostics::ToolCallRecord;
use crate::intervention::InterventionAction;
use crate::intervention::InterventionHandler;
use crate::intervention::InterventionReason;
use crate::parallel_execution::ParallelModel;
use crate::parallel_execution::ParallelRole;
use crate::parallel_execution::execute_parallel_roles;
use crate::parallel_execution::merge_parallel_results;
use crate::progress::AutoDrivePhase;
use crate::progress::CompactionNotification;
use crate::progress::ProgressCollector;
use crate::progress::ProgressViewModel;
use crate::progress_log::ProgressEntry;
use crate::progress_log::ProgressLogger;
use crate::progress_log::ProgressType;
use crate::role_channel::RoleChannelHub;
use crate::role_channel::RoleMessage;
use crate::role_channel::RoleReceiver;
use crate::scheduler::AgentId;
use crate::scheduler::AgentResult;
use crate::scheduler::AgentScheduler;
use crate::scheduler::AgentTask;
use crate::selective_tests::TestCommandResult;
use crate::selective_tests::TestPlan;
use crate::selective_tests::plan_from_diff;
use crate::selective_tests::verification_result_for_feature;
use crate::session_pool::PoolError;
use crate::session_pool::PoolTask;
use crate::session_pool::SessionPool;
use crate::session_pool::TaskResult;
use crate::task_pipeline::PipelineError;
use crate::task_pipeline::StageAction;
use crate::task_pipeline::TaskPipeline;
use crate::telemetry::SessionOutcome;
use crate::telemetry::TelemetryCollector;
use crate::telemetry::TurnOutcome;
use anyhow::anyhow;
use code_core::config_types::AutoDriveSettings;
use std::sync::Arc;
use std::time::Instant;

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
            max_concurrent_agents: 8,
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
        let budget =
            if token_budget.is_some() || turn_limit.is_some() || duration_limit_seconds.is_some() {
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
    CheckpointSaved { session_id: String, turns: usize },
    /// Checkpoint was restored.
    CheckpointRestored { session_id: String, turns: usize },
    /// Diagnostic alert detected.
    DiagnosticAlert(DiagnosticAlert),
    /// Budget alert triggered.
    BudgetAlert(BudgetAlert),
    /// Agent progress update.
    AgentProgress { agent_id: String, completed: bool },
    /// Intervention required.
    InterventionRequired(InterventionReason),
    /// History was compacted.
    HistoryCompacted(CompactionNotification),
}

fn parallel_role_from_name(name: &str) -> Option<ParallelRole> {
    match name {
        "Coordinator" => Some(ParallelRole::Coordinator),
        "Architect" => Some(ParallelRole::Architect),
        "Tester" => Some(ParallelRole::Tester),
        "Debugger" => Some(ParallelRole::Debugger),
        "Reviewer" => Some(ParallelRole::Reviewer),
        _ if name.starts_with("Executor-") => name
            .split('-')
            .nth(1)
            .and_then(|n| n.parse::<u8>().ok())
            .map(ParallelRole::Executor),
        _ => None,
    }
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
    backlog: Option<BacklogManager>,
    progress_logger: Option<ProgressLogger>,
    pipeline: TaskPipeline,
    role_channel: RoleChannelHub,
    role_receivers: HashMap<String, RoleReceiver>,

    session_id: Option<String>,
    goal: Option<String>,
    pipeline_task_id: Option<String>,
    turns_since_checkpoint: u32,
    pub session_pool: Option<Arc<SessionPool>>,
    pending_events: Vec<EnhancedEvent>,
}

impl EnhancedCoordinator {
    fn log_progress(
        &self,
        kind: ProgressType,
        status: &str,
        tests: &str,
        summary: &str,
        note: &str,
    ) -> anyhow::Result<()> {
        if let Some(logger) = &self.progress_logger {
            logger.append(ProgressEntry::new(kind, status, tests, summary, note))?;
        }
        Ok(())
    }

    /// Creates a new enhanced coordinator with the given configuration.
    pub fn new(config: EnhancedConfig, auto_settings: &AutoDriveSettings) -> Self {
        let checkpoint_manager = if config.checkpoint_enabled {
            config
                .checkpoint_dir
                .as_ref()
                .map(|path| CheckpointManager::new(path.clone()))
        } else {
            None
        };

        let mut diagnostics = DiagnosticsEngine::new();
        if config.diagnostics_enabled {
            diagnostics.set_loop_threshold(config.loop_threshold);
        }

        let audit = if config.audit_enabled {
            if let Some(path) = &config.audit_path {
                Some(AuditLogger::new("session").with_log_path(path.clone()))
            } else {
                Some(AuditLogger::new("session"))
            }
        } else {
            None
        };

        let mut budget = BudgetController::new();
        if let Some(budget_config) = &config.budget {
            budget.configure(budget_config.clone());
        }

        let backlog = BacklogManager::load("ai/feature_list.json").ok();
        let progress_logger = Some(ProgressLogger::new(PathBuf::from("ai/progress.log")));

        let session_pool = if auto_settings.parallel_instances > 1 {
            use crate::session_pool::SessionPool;
            use crate::session_pool::SessionPoolConfig;
            let mut pool_config = SessionPoolConfig::default();
            pool_config.max_sessions = auto_settings.high_throughput.max_sessions;
            pool_config.min_sessions = auto_settings.high_throughput.min_sessions;
            pool_config.scale_up_threshold = auto_settings.high_throughput.scale_up_threshold;
            pool_config.scale_down_threshold = auto_settings.high_throughput.scale_down_threshold;
            pool_config.backpressure_threshold =
                pool_config.max_sessions * auto_settings.high_throughput.backpressure_multiplier;
            Some(Arc::new(SessionPool::new(pool_config)))
        } else {
            None
        };

        let mut role_channel = RoleChannelHub::new(32);
        let mut role_receivers = HashMap::new();
        let role_names: Vec<String> =
            ParallelRole::roles_for_count(auto_settings.parallel_instances.max(1))
                .into_iter()
                .map(|role| role.name())
                .collect();
        for role in role_names {
            let rx = role_channel.register(role.clone());
            role_receivers.insert(role, rx);
        }

        let mut pending_events = Vec::new();
        if config.max_concurrent_agents < 8 && auto_settings.parallel_instances > 1 {
            pending_events.push(EnhancedEvent::DiagnosticAlert(
                DiagnosticAlert::LowConcurrency {
                    max_concurrent_agents: config.max_concurrent_agents as i32,
                },
            ));
        }

        Self {
            scheduler: AgentScheduler::new(config.max_concurrent_agents),
            config: config.clone(),
            checkpoint_manager,
            diagnostics,
            budget,
            audit,
            telemetry: TelemetryCollector::new(),
            compaction: CompactionEngine::with_config(config.compaction),
            intervention: InterventionHandler::new(),
            progress: ProgressCollector::new(),
            backlog,
            progress_logger,
            pipeline: TaskPipeline::new(),
            role_channel,
            role_receivers,
            session_id: None,
            goal: None,
            pipeline_task_id: None,
            turns_since_checkpoint: 0,
            session_pool,
            pending_events,
        }
    }

    /// Starts a new session.
    pub fn start_session(&mut self, goal: &str, session_id: &str) {
        self.session_id = Some(session_id.to_string());
        self.goal = Some(goal.to_string());
        self.turns_since_checkpoint = 0;
        self.pipeline_task_id = Some(self.pipeline.create_from_goal(goal));
        if let Some(id) = &self.pipeline_task_id {
            let _ = self.pipeline.advance(id);
        }

        self.diagnostics.set_goal(goal);
        self.budget.start();
        self.progress.start();
        self.progress.set_goal(goal);
        self.progress.set_phase(AutoDrivePhase::Running);
        let _ = self.backlog.as_ref().map(|b| b.features().len());
        let _ = self.log_progress(ProgressType::Step, "running", "", "Session start", goal);
        if let Some(backlog) = &self.backlog {
            let _ = self.log_progress(
                ProgressType::Change,
                "info",
                "",
                "Backlog loaded",
                &format!("features={}", backlog.features().len()),
            );
        }

        if let Some(pool) = &self.session_pool {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.warmup().await;
            });
        }

        if self.config.telemetry_enabled {
            self.telemetry.start_session(goal, session_id);
        }

        if let Some(manager) = &mut self.checkpoint_manager
            && let Ok(checkpoint) = manager.create(goal, session_id)
        {
            tracing::debug!(session_id, "Created initial checkpoint");
            let _ = checkpoint; // Initial checkpoint created
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
            self.pending_events
                .push(EnhancedEvent::BudgetAlert(alert.clone()));
            self.progress.set_budget_alert(Some(alert.clone()));

            if self.budget.should_pause() {
                self.intervention.request_for_budget(&alert);
                self.pending_events
                    .push(EnhancedEvent::InterventionRequired(
                        InterventionReason::BudgetDecision {
                            alert: crate::intervention::BudgetAlertKind::from(&alert),
                        },
                    ));
            }
        }

        // Check diagnostics
        let report = self.diagnostics.generate_report();
        for alert in report.alerts {
            self.pending_events
                .push(EnhancedEvent::DiagnosticAlert(alert.clone()));
            self.progress.set_diagnostic_alert(Some(alert.clone()));
            self.intervention.request_for_diagnostic(&alert);
        }

        // Telemetry
        if self.config.telemetry_enabled {
            let turn_num = self.progress.build_view_model().turns_completed as u32;
            let span = self.telemetry.start_turn(turn_num);
            self.telemetry
                .end_turn(span, TurnOutcome::Success { tokens_used });
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
                    crate::diagnostics::ToolOutcome::Timeout => {
                        AuditOutcome::Failure("timeout".to_string())
                    }
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
        self.scheduler
            .report_completion_with_order(id, result, order);
        self.pending_events.push(EnhancedEvent::AgentProgress {
            agent_id: format!("{}", id.0),
            completed: true,
        });
    }

    /// Collects all agent results.
    pub fn collect_agent_results(&mut self) -> Vec<AgentResult> {
        self.scheduler.collect_results()
    }

    /// Returns agent tasks for the active pipeline stage.
    pub fn current_stage_tasks(&mut self) -> Result<Vec<AgentTask>, PipelineError> {
        let task_id = self
            .pipeline_task_id
            .clone()
            .ok_or_else(|| PipelineError::TaskNotFound {
                task_id: "active".to_string(),
            })?;
        self.pipeline.get_stage_tasks(&task_id)
    }

    /// Records a role completion for the current pipeline task.
    pub fn handle_stage_role_complete(
        &mut self,
        role: &str,
        result: &str,
        success: bool,
    ) -> Result<StageAction, PipelineError> {
        let task_id = self
            .pipeline_task_id
            .clone()
            .ok_or_else(|| PipelineError::TaskNotFound {
                task_id: "active".to_string(),
            })?;
        self.pipeline
            .handle_role_complete(&task_id, role, result, success)
    }

    /// Drains pipeline tasks that have reached a terminal state.
    pub fn drain_terminal_pipeline(&mut self) -> Vec<crate::task_pipeline::PipelineTask> {
        self.pipeline.drain_terminal()
    }

    /// Executes the active pipeline stage using the session pool and parallel executor.
    pub async fn run_active_stage(
        &mut self,
        client: Arc<dyn ParallelModel>,
        model: &str,
    ) -> anyhow::Result<StageAction> {
        let task_id = self
            .pipeline_task_id
            .clone()
            .ok_or_else(|| anyhow!("no active pipeline task"))?;
        let pipeline_task = self
            .pipeline
            .get(&task_id)
            .ok_or_else(|| anyhow!("pipeline task {task_id} not found"))?;
        let stage = pipeline_task.stage;
        let goal = self.goal.clone().unwrap_or_default();
        let base_prompt = format!("[{stage:?}] {goal}");
        let roles: Vec<ParallelRole> = stage
            .active_roles()
            .iter()
            .filter_map(|name| parallel_role_from_name(name))
            .collect();
        let max_agents = self.config.max_concurrent_agents as i32;
        let start = Instant::now();
        let _ = self.log_progress(
            ProgressType::Step,
            "running",
            "",
            format!("Dispatch stage {stage:?}").as_str(),
            &base_prompt,
        );

        let session_id = if let Some(pool) = self.session_pool.clone() {
            let pool_task = PoolTask::new(task_id.clone(), base_prompt.clone());
            pool.submit(pool_task)
                .await
                .map_err(|err| anyhow!(err.to_string()))?;
            let mut session_id = pool.session_for_task(&task_id).await;
            if session_id.is_none() {
                let _ = pool
                    .dispatch_from_queue()
                    .await
                    .map_err(|err| anyhow!(err.to_string()))?;
                session_id = pool.session_for_task(&task_id).await;
            }
            session_id.ok_or_else(|| anyhow!(PoolError::NoAvailableSessions.to_string()))?
        } else {
            "direct".to_string()
        };

        let results =
            execute_parallel_roles(client, roles, &base_prompt, model, max_agents).await?;

        for result in &results {
            let message = RoleMessage::WorkComplete {
                role: result.role.name(),
                task_id: Some(task_id.clone()),
                success: result.success,
                result: result.response.clone(),
            };
            let _ = self.role_channel.broadcast(message).await;
        }

        let merged = merge_parallel_results(results.clone());
        let mut last_action = StageAction::Wait;
        let mut advance_target = None;

        for result in results {
            let action = self.handle_stage_role_complete(
                &result.role.name(),
                &result.response,
                result.success,
            )?;
            if let StageAction::Fail { role, error } = &action {
                let _ = self
                    .role_channel
                    .broadcast(RoleMessage::ErrorOccurred {
                        role: role.clone(),
                        task_id: Some(task_id.clone()),
                        error: error.clone(),
                    })
                    .await;
            }
            if let StageAction::Advance(next) = action {
                advance_target = Some(next);
            }
            last_action = action;
            if matches!(last_action, StageAction::Fail { .. }) {
                break;
            }
        }

        if let Some(pool) = &self.session_pool {
            let task_result = TaskResult {
                task_id: task_id.clone(),
                session_id: session_id.clone(),
                success: !matches!(last_action, StageAction::Fail { .. }),
                content: merged,
                tokens_used: 0,
                duration: start.elapsed(),
            };
            let _ = pool.complete_session(&session_id, task_result).await;
            pool.auto_scale().await;
        }

        if !matches!(last_action, StageAction::Fail { .. })
            && let Some(next) = advance_target
        {
            let _ = self.log_progress(
                ProgressType::Change,
                "advanced",
                "",
                format!("Stage {stage:?} -> {next:?}").as_str(),
                "",
            );
            let _ = self
                .role_channel
                .broadcast(RoleMessage::StageAdvance {
                    task_id: Some(task_id),
                    from_stage: Some(format!("{stage:?}")),
                    stage: format!("{next:?}"),
                })
                .await;
        }

        self.poll_pool_events().await;

        Ok(last_action)
    }

    /// Collects pool health and backpressure events into pending events and audit log.
    pub async fn poll_pool_events(&mut self) {
        if let Some(pool) = &self.session_pool {
            if let Some(alert) = pool.take_backpressure_alert().await {
                self.pending_events
                    .push(EnhancedEvent::BudgetAlert(alert.clone()));
                if let Some(audit) = &mut self.audit {
                    audit.log(
                        AuditOperation::BudgetWarning {
                            alert: format!("{alert:?}"),
                        },
                        AuditOutcome::Success,
                    );
                }
            }

            let report = pool.health_check().await;

            for (session_id, elapsed_ms, task_id) in report.slow_sessions {
                self.pending_events.push(EnhancedEvent::DiagnosticAlert(
                    DiagnosticAlert::SessionSlow {
                        session_id,
                        elapsed_ms,
                        task_id,
                    },
                ));
            }

            for (session_id, elapsed_ms, task_id) in report.stuck_sessions {
                self.pending_events.push(EnhancedEvent::DiagnosticAlert(
                    DiagnosticAlert::SessionStuck {
                        session_id,
                        elapsed_ms,
                        task_id,
                    },
                ));
            }

            for migration in report.migrations {
                self.pending_events.push(EnhancedEvent::DiagnosticAlert(
                    DiagnosticAlert::SessionMigrated {
                        from_session: migration.from_session.clone(),
                        to_session: migration.to_session.clone(),
                        task_id: migration.task_id.clone(),
                        retry_count: migration.retry_count,
                    },
                ));
                if let Some(audit) = &mut self.audit {
                    audit.log(
                        AuditOperation::SessionMigration {
                            from_session: migration.from_session,
                            to_session: migration.to_session,
                            task_id: migration.task_id,
                            retry_count: migration.retry_count,
                        },
                        AuditOutcome::Success,
                    );
                }
            }
        }
    }

    /// Checks if compaction should be triggered.
    pub fn should_compact(&self, current_tokens: u64, context_limit: u64) -> bool {
        self.compaction
            .should_compact(current_tokens, context_limit)
    }

    /// Performs compaction on history items.
    pub fn compact_history(
        &mut self,
        items: &[ItemClassification],
    ) -> crate::compaction::CompactionResult {
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
        self.pending_events
            .push(EnhancedEvent::HistoryCompacted(notification));

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

    /// 生成受影响特性的测试计划（基于 git diff 输出）。
    pub fn plan_tests_from_diff(&self, diff_output: &str) -> Option<TestPlan> {
        let backlog = self.backlog.as_ref()?;
        Some(plan_from_diff(backlog, diff_output))
    }

    /// 记录特性验证结果并写入外部记忆/进度日志。
    pub fn record_verification(
        &mut self,
        feature_id: &str,
        verified: bool,
        tests_run: Vec<String>,
        summary: &str,
        reason: Option<&str>,
    ) -> anyhow::Result<()> {
        let result = crate::backlog::VerificationResult::new(verified, tests_run, summary)
            .with_reason(reason.unwrap_or(""));
        self.record_verification_result(feature_id, result)?;
        Ok(())
    }

    /// 使用标准化验证结果写回 backlog 与进度日志。
    pub fn record_verification_result(
        &mut self,
        feature_id: &str,
        result: crate::backlog::VerificationResult,
    ) -> anyhow::Result<crate::backlog::VerificationResult> {
        if let Some(backlog) = &mut self.backlog {
            backlog.update_verification(feature_id, result.clone())?;
        }
        let status = if result.verified {
            "verified"
        } else {
            "failed"
        };
        let tests = if result.tests_run.is_empty() {
            "".to_string()
        } else {
            result.tests_run.join(",")
        };
        let note = result.reason.clone().unwrap_or_default();
        let _ = self.log_progress(ProgressType::Verify, status, &tests, &result.summary, &note);
        Ok(result)
    }

    /// 基于测试计划和执行结果计算验证结论并写回外部记忆/日志。
    pub fn evaluate_and_record_verification(
        &mut self,
        feature: &crate::backlog::Feature,
        plan: &TestPlan,
        executed: &[TestCommandResult],
        summary: &str,
    ) -> anyhow::Result<crate::backlog::VerificationResult> {
        let result = verification_result_for_feature(feature, plan, executed, summary.to_string());
        self.record_verification_result(&feature.id, result.clone())?;
        Ok(result)
    }

    /// Ends the session.
    pub fn end_session(&mut self, success: bool) {
        self.progress.set_phase(if success {
            AutoDrivePhase::Completed
        } else {
            AutoDrivePhase::Stopped
        });

        let status_text = if success { "success" } else { "stopped" };
        let _ = self.log_progress(ProgressType::Change, status_text, "", "Session ended", "");

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

        if let Ok(mut checkpoint) = manager.create(goal, session_id)
            && manager
                .update(
                    &mut checkpoint,
                    vec![],
                    model.turns_completed,
                    token_usage,
                    &AutoRunPhase::Active,
                )
                .is_ok()
        {
            self.turns_since_checkpoint = 0;
            self.pending_events.push(EnhancedEvent::CheckpointSaved {
                session_id: session_id.clone(),
                turns: model.turns_completed,
            });
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
        self.pipeline = TaskPipeline::new();
        self.pipeline_task_id = None;
        self.role_channel = RoleChannelHub::new(32);
        self.role_receivers.clear();

        if let Some(audit) = &mut self.audit {
            audit.clear();
        }
    }
}

impl Default for EnhancedCoordinator {
    fn default() -> Self {
        Self::new(EnhancedConfig::default(), &AutoDriveSettings::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parallel_execution::ParallelResponseStream;
    use crate::task_pipeline::StageAction;
    use code_core::Prompt;
    use code_core::ResponseEvent;
    use futures::stream;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Mutex;
    use std::time::Instant;

    struct FakeModel {
        responses: Mutex<Vec<String>>,
    }

    impl FakeModel {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl ParallelModel for FakeModel {
        fn stream_prompt(
            &self,
            _prompt: Prompt,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<ParallelResponseStream>> + Send + '_>>
        {
            let text = {
                let mut guard = self.responses.lock().unwrap();
                guard.pop().unwrap_or_else(|| "ok".to_string())
            };
            Box::pin(async move {
                let events = vec![
                    Ok(ResponseEvent::OutputTextDelta {
                        delta: text,
                        item_id: None,
                        sequence_number: None,
                        output_index: None,
                    }),
                    Ok(ResponseEvent::Completed {
                        response_id: "resp".to_string(),
                        token_usage: None,
                    }),
                ];
                Ok(Box::pin(stream::iter(events)) as ParallelResponseStream)
            })
        }
    }

    #[test]
    fn test_enhanced_coordinator_lifecycle() {
        let mut coord = EnhancedCoordinator::new(
            EnhancedConfig {
                diagnostics_enabled: true,
                ..Default::default()
            },
            &AutoDriveSettings::default(),
        );

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
    fn pipeline_tasks_available_after_start() {
        let mut coord = EnhancedCoordinator::default();
        coord.start_session("Goal", "session-1");

        let tasks = coord.current_stage_tasks().unwrap();
        assert!(!tasks.is_empty());

        let action = coord
            .handle_stage_role_complete("Coordinator", "ok", true)
            .unwrap();
        assert!(matches!(
            action,
            StageAction::Wait | StageAction::Advance(_)
        ));
    }

    #[tokio::test]
    async fn run_active_stage_dispatches_and_advances_pipeline() {
        let mut settings = AutoDriveSettings::default();
        settings.parallel_instances = 8;
        let mut coord = EnhancedCoordinator::new(EnhancedConfig::default(), &settings);
        coord.start_session("Goal", "session-1");

        let pool = coord.session_pool.clone().expect("session pool created");
        pool.warmup().await;

        let client: Arc<dyn ParallelModel> = Arc::new(FakeModel::new(vec![
            "Architect plan".to_string(),
            "Coordinator plan".to_string(),
        ]));

        let action = coord
            .run_active_stage(client, "gpt-5.1")
            .await
            .expect("stage executed");

        assert!(matches!(
            action,
            StageAction::Advance(crate::task_pipeline::PipelineStage::Implementing)
        ));

        let result = pool.next_result().await.unwrap();
        assert!(result.success);
        assert!(!result.content.is_empty());

        if let Some(rx) = coord.role_receivers.get_mut("Coordinator")
            && let Some(message) = rx.recv().await
        {
            match message {
                RoleMessage::WorkComplete { role, .. } => assert_eq!(role, "Coordinator"),
                other => panic!("unexpected message: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn poll_pool_events_emits_stuck_alert() {
        let mut settings = AutoDriveSettings::default();
        settings.parallel_instances = 2;
        let mut coord = EnhancedCoordinator::new(EnhancedConfig::default(), &settings);
        coord.start_session("Goal", "session-stuck");

        let pool = coord.session_pool.clone().expect("session pool created");
        pool.warmup().await;
        pool.inject_session_state(
            crate::session_pool::SessionState::Running,
            crate::session_pool::PoolTask::new("t1", "work"),
            std::time::Instant::now() - std::time::Duration::from_secs(400),
        )
        .await;

        coord.poll_pool_events().await;
        let events = coord.take_events();
        assert!(events.iter().any(|e| {
            matches!(
                e,
                EnhancedEvent::DiagnosticAlert(
                    crate::diagnostics::DiagnosticAlert::SessionStuck { .. }
                )
            )
        }));
    }

    #[tokio::test]
    async fn poll_pool_events_emits_slow_alert() {
        let mut settings = AutoDriveSettings::default();
        settings.parallel_instances = 2;
        let mut coord = EnhancedCoordinator::new(EnhancedConfig::default(), &settings);
        coord.start_session("Goal", "session-slow");

        let pool = coord.session_pool.clone().expect("session pool created");
        pool.warmup().await;
        pool.inject_session_state(
            crate::session_pool::SessionState::Running,
            crate::session_pool::PoolTask::new("t2", "work"),
            std::time::Instant::now()
                - std::time::Duration::from_millis(pool.slow_threshold().as_millis() as u64 + 50),
        )
        .await;

        coord.poll_pool_events().await;
        let events = coord.take_events();
        assert!(events.iter().any(|e| {
            matches!(
                e,
                EnhancedEvent::DiagnosticAlert(
                    crate::diagnostics::DiagnosticAlert::SessionSlow { .. }
                )
            )
        }));
    }

    #[test]
    fn plan_tests_from_diff_uses_backlog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("feature_list.json");
        let mgr = BacklogManager::from_features(
            path,
            vec![crate::backlog::Feature {
                id: "F-10".to_string(),
                description: "core change".to_string(),
                module: "core/src".to_string(),
                test_requirements: crate::backlog::TestRequirements {
                    unit: vec!["cargo test -p code-core".to_string()],
                    e2e: vec![],
                },
                ..Default::default()
            }],
        );
        mgr.save().unwrap();

        let mut coord = EnhancedCoordinator::default();
        coord.backlog = Some(mgr);

        let plan = coord
            .plan_tests_from_diff("core/src/lib.rs\n")
            .expect("plan");
        assert!(plan.quick.iter().any(|c| c.contains("code-core")));
    }

    #[test]
    fn evaluate_and_record_verification_updates_backlog_and_progress() {
        let dir = tempfile::tempdir().unwrap();
        let backlog_path = dir.path().join("feature_list.json");
        let progress_path = dir.path().join("progress.log");

        let feature = crate::backlog::Feature {
            id: "F-verify".to_string(),
            description: "verify strict".to_string(),
            tdd_mode: crate::backlog::TddMode::Strict,
            test_requirements: crate::backlog::TestRequirements {
                unit: vec!["cargo test -p code-auto-drive-core".to_string()],
                e2e: vec![],
            },
            ..Default::default()
        };

        let mgr = BacklogManager::from_features(backlog_path.clone(), vec![feature.clone()]);
        mgr.save().unwrap();

        let mut coord = EnhancedCoordinator::default();
        coord.backlog = Some(BacklogManager::load(backlog_path.clone()).unwrap());
        coord.progress_logger = Some(ProgressLogger::new(progress_path.clone()));

        let plan = crate::selective_tests::generate_quick_plan(&[feature.clone()]);
        let executed = vec![crate::selective_tests::TestCommandResult::success(
            "cargo test -p code-auto-drive-core",
        )];

        let result = coord
            .evaluate_and_record_verification(&feature, &plan, &executed, "ok")
            .unwrap();
        assert!(result.verified);

        let saved = BacklogManager::load(backlog_path).unwrap();
        assert!(
            saved.features().iter().any(|f| f
                .verification
                .as_ref()
                .map(|v| v.verified)
                .unwrap_or(false))
        );

        let progress = std::fs::read_to_string(progress_path).unwrap();
        assert!(progress.contains("VERIFY"));
    }

    #[test]
    fn emits_low_concurrency_alert_on_init() {
        let mut config = EnhancedConfig::default();
        config.max_concurrent_agents = 4;
        let mut settings = AutoDriveSettings::default();
        settings.parallel_instances = 2;
        let mut coord = EnhancedCoordinator::new(config, &settings);
        let events = coord.take_events();
        assert!(events.iter().any(|e| {
            matches!(
                e,
                EnhancedEvent::DiagnosticAlert(DiagnosticAlert::LowConcurrency { .. })
            )
        }));
    }

    #[test]
    fn does_not_emit_low_concurrency_when_serial() {
        let mut config = EnhancedConfig::default();
        config.max_concurrent_agents = 4;
        let coord = EnhancedCoordinator::new(config, &AutoDriveSettings::default());
        let events = coord.pending_events;
        assert!(!events.iter().any(|e| {
            matches!(
                e,
                EnhancedEvent::DiagnosticAlert(DiagnosticAlert::LowConcurrency { .. })
            )
        }));
    }

    #[tokio::test]
    async fn poll_pool_events_collects_backpressure() {
        let mut settings = AutoDriveSettings::default();
        settings.parallel_instances = 2;
        settings.high_throughput.max_sessions = 1;
        settings.high_throughput.min_sessions = 0;
        settings.high_throughput.backpressure_multiplier = 1;

        let mut coord = EnhancedCoordinator::new(EnhancedConfig::default(), &settings);
        coord.start_session("Goal", "session-backpressure");

        let pool = coord.session_pool.clone().expect("pool");
        pool.warmup().await;
        pool.inject_session_state(
            crate::session_pool::SessionState::Running,
            crate::session_pool::PoolTask::new("busy", "work"),
            Instant::now(),
        )
        .await;

        pool.submit(crate::session_pool::PoolTask::new("queued1", "task"))
            .await
            .unwrap();
        let err = pool
            .submit(crate::session_pool::PoolTask::new("queued2", "task"))
            .await
            .unwrap_err();
        assert!(matches!(err, PoolError::BackpressureFull { .. }));

        coord.poll_pool_events().await;
        let events = coord.take_events();
        assert!(events.iter().any(|e| {
            matches!(
                e,
                EnhancedEvent::BudgetAlert(BudgetAlert::BackpressureExceeded { .. })
            )
        }));
    }

    #[test]
    fn test_budget_alert_triggers_intervention() {
        let mut coord = EnhancedCoordinator::new(
            EnhancedConfig {
                budget: Some(BudgetConfig {
                    token_budget: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &AutoDriveSettings::default(),
        );

        coord.start_session("Test", "session");
        coord.record_turn(100); // Exceeds budget

        assert!(coord.intervention_pending());

        let events = coord.take_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EnhancedEvent::BudgetAlert(_)))
        );
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
